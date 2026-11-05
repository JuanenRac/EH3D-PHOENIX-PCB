// SPDX-License-Identifier: GPL-3.0-or-later
//! Filters of microstrip: the stepped-impedance low-pass filter and the coupled-line band-pass
//! filter with open ends.

use eh3d_phoenix_rf::filter::{prototype, Approximation, Position, Prototype};

use crate::codes::{RfgenError, NOT_REALISABLE, PARAMETER_INVALID};
use crate::env::Environment;
use crate::formulas::{
    bandpass_inverters, coupled_range, degrees_to_m, even_odd_from_inverter, lowpass_sections,
    open_end_extension_m, solve_coupled,
};
use crate::layout::{even, nm_from_m, Layout};
use crate::params::Params;
use crate::report::Report;
use crate::settings::RfgenSettings;

use super::{describe_line, h_strip, open_end_problems, NetNames, OPEN_END_RANGE, OPEN_END_SOURCE};

/// The family, the order and the ripple of the prototype that a filter is made from.
fn prototype_of(
    params: &Params,
    settings: &RfgenSettings,
    env: &Environment,
    default_family: &str,
    default_order: usize,
) -> Result<Prototype, RfgenError> {
    let family = params.choice("family", &["butterworth", "chebyshev"], default_family)?;
    let order = params.whole("order", default_order, 1)?;
    if order > settings.max_filter_order {
        return Err(RfgenError::new(
            PARAMETER_INVALID,
            format!(
                "order: {order} is above max_filter_order = {}",
                settings.max_filter_order
            ),
        ));
    }
    let (approximation, ripple) = if family == "chebyshev" {
        (
            Approximation::Chebyshev,
            Some(params.positive("ripple_db", 0.5)?),
        )
    } else {
        if params.number("ripple_db")?.is_some() {
            return Err(RfgenError::new(
                PARAMETER_INVALID,
                "a Butterworth filter has no ripple",
            ));
        }
        (Approximation::Butterworth, None)
    };
    prototype(approximation, order, ripple, Position::Shunt, &env.rf)
        .map_err(|e| RfgenError::new(PARAMETER_INVALID, e.to_string()))
}

/// A stepped-impedance low-pass filter: wide lines for the capacitors and narrow lines for the
/// inductors, between leads of the line impedance. The cutoff is the frequency of the design.
pub(super) fn lowpass(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let proto = prototype_of(params, settings, env, "butterworth", 5)?;
    let sections = lowpass_sections(
        &proto,
        z0,
        settings.lowpass_z_high_ohm,
        settings.lowpass_z_low_ohm,
    );
    let feed = env.line_for_z0(z0)?;
    let high = env.line_for_z0(settings.lowpass_z_high_ohm)?;
    let low = env.line_for_z0(settings.lowpass_z_low_ohm)?;
    let lead = settings.lead_length_nm;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -lead, 0, 0, feed.width_nm);
    let mut x: i64 = 0;
    let mut problems = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        let line = if section.capacitor { &low } else { &high };
        let length = nm_from_m(degrees_to_m(section.degrees, line.lambda_g_m))?;
        if length < 1 {
            return Err(RfgenError::new(
                NOT_REALISABLE,
                format!("section {} is shorter than one nanometre", index + 1),
            ));
        }
        h_strip(layout, net, x, x + length, 0, line.width_nm);
        x += length;
        let name = format!("section {}", index + 1);
        report.dim(&format!("{name} g"), section.g, "");
        report.dim(&format!("{name} impedance"), section.impedance_ohm, "ohm");
        report.dim_mm(&format!("{name} width"), line.width_m());
        report.dim_mm(&format!("{name} length"), length as f64 * 1.0e-9);
        report.dim(&format!("{name} electrical length"), section.degrees, "deg");
        if section.degrees > 45.0 {
            problems.push(format!(
                "section {} is {:.1} degrees long: the line is no longer close to a lumped element",
                index + 1,
                section.degrees
            ));
        }
    }
    h_strip(layout, net, x, x + lead, 0, feed.width_nm);
    layout.port("P1", (-lead, 0), 180, feed.z0_ohm);
    layout.port("P2", (x + lead, 0), 0, feed.z0_ohm);
    describe_line(report, "feed line", &feed);
    report.dim("cutoff", env.freq_ghz, "GHz");
    report.dim("order", proto.order as f64, "");
    report.dim_mm("filter length", x as f64 * 1.0e-9);
    report.formula(
        "low-pass prototype values",
        "Matthaei, Young and Jones; Pozar, Microwave Engineering, section 8.3, from eh3d-phoenix-rf",
        "orders 1 to max_filter_order, ripple of the RF settings",
        &[],
    );
    report.formula(
        "stepped-impedance sections beta l = g Zl / Z0 and g Z0 / Zh",
        "Pozar, Microwave Engineering, sections 8.5 and 8.6",
        "exact at the cutoff only; sections up to 45 degrees (rule of thumb)",
        &problems,
    );
    Ok(())
}

/// One section of a coupled-line filter.
struct Section {
    width: i64,
    gap: i64,
    length: i64,
}

/// A coupled-line band-pass filter of `N` resonators, each half a wavelength long and open at both
/// ends, in the staggered layout: `N + 1` coupled sections a quarter wave long, with the width and
/// the gap found from the even and odd impedances of the inverters.
pub(super) fn bandpass(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let percent = params.positive("bandwidth_percent", settings.bandpass_bandwidth_percent)?;
    if percent >= 100.0 {
        return Err(RfgenError::new(
            PARAMETER_INVALID,
            "bandwidth_percent must be below 100",
        ));
    }
    let proto = prototype_of(params, settings, env, "chebyshev", 3)?;
    let n = proto.order;
    let delta = percent / 100.0;
    let inverters = bandpass_inverters(&proto.g, proto.g_load, delta);
    let h = env.substrate.h_m;
    let er = env.substrate.er;
    let half_gap = settings.bandpass_end_gap_nm / 2;
    let mut sections: Vec<Section> = Vec::with_capacity(n + 1);
    for (index, zj) in inverters.iter().enumerate() {
        let (ze, zo) = even_odd_from_inverter(z0, *zj);
        let found = solve_coupled(
            ze,
            zo,
            er,
            settings.solver_tolerance_ohm,
            settings.solver_max_iterations,
        )?;
        report.formula(
            "even and odd mode impedances of coupled microstrips",
            "Hammerstad and Jensen 1980 as collected by Garg and Bahl 1979 (prototype)",
            "0.1 <= W/h <= 10, 0.1 <= s/h <= 3, 1 <= er <= 18; zero thickness",
            &coupled_range(found.u, found.g, er),
        );
        let eps_mean = (found.pair.eps_even + found.pair.eps_odd) / 2.0;
        let lambda_g = env.lambda0_m() / eps_mean.sqrt();
        let width_m = found.u * h;
        let extension_m = open_end_extension_m(width_m, h, eps_mean);
        let length_m = lambda_g / 4.0 - extension_m;
        let section = Section {
            width: even(nm_from_m(width_m)?),
            gap: nm_from_m(found.g * h)?,
            length: nm_from_m(length_m)?,
        };
        if section.length <= 2 * half_gap || section.gap < 1 || section.width < 2 {
            return Err(RfgenError::new(
                NOT_REALISABLE,
                format!("section {} has no usable size", index + 1),
            ));
        }
        let name = format!("section {}", index + 1);
        report.dim(&format!("{name} Z0 J"), *zj, "");
        report.dim(&format!("{name} even impedance"), ze, "ohm");
        report.dim(&format!("{name} odd impedance"), zo, "ohm");
        report.dim(&format!("{name} W/h"), found.u, "");
        report.dim(&format!("{name} s/h"), found.g, "");
        report.dim_mm(&format!("{name} width"), section.width as f64 * 1.0e-9);
        report.dim_mm(&format!("{name} gap"), section.gap as f64 * 1.0e-9);
        report.dim_mm(&format!("{name} length"), section.length as f64 * 1.0e-9);
        report.dim_mm(&format!("{name} open-end extension"), extension_m);
        sections.push(section);
    }
    let feed = env.line_for_z0(z0)?;
    let lead = settings.lead_length_nm;

    // the junctions between the sections along x, and the height of each resonator
    let mut cum: Vec<i64> = vec![0];
    for section in &sections {
        let last = cum.last().copied().unwrap_or(0);
        cum.push(last + section.length);
    }
    let mut y: Vec<i64> = vec![0];
    for k in 1..=n + 1 {
        let previous = y.last().copied().unwrap_or(0);
        let section = &sections[k - 1];
        let pitch = section.width + section.gap;
        y.push(if k % 2 == 1 {
            previous + pitch
        } else {
            previous - pitch
        });
    }

    // the input feed and its coupled strip
    let input = layout.net(&nets.signal);
    h_strip(layout, input, -lead, 0, 0, feed.width_nm);
    h_strip(layout, input, 0, cum[1] - half_gap, y[0], sections[0].width);
    // the resonators, each end kept half the end gap from the junction of its section
    for k in 1..=n {
        let net = layout.net(&format!("{}_R{k}", nets.signal));
        h_strip(
            layout,
            net,
            cum[k - 1] + half_gap,
            cum[k],
            y[k],
            sections[k - 1].width,
        );
        h_strip(
            layout,
            net,
            cum[k],
            cum[k + 1] - half_gap,
            y[k],
            sections[k].width,
        );
    }
    // the output strip and feed
    let output = layout.net(&format!("{}_OUT", nets.signal));
    h_strip(
        layout,
        output,
        cum[n] + half_gap,
        cum[n + 1],
        y[n + 1],
        sections[n].width,
    );
    h_strip(
        layout,
        output,
        cum[n + 1],
        cum[n + 1] + lead,
        y[n + 1],
        feed.width_nm,
    );
    layout.port("P1", (-lead, 0), 180, feed.z0_ohm);
    layout.port("P2", (cum[n + 1] + lead, y[n + 1]), 0, feed.z0_ohm);

    describe_line(report, "feed line", &feed);
    report.dim("centre frequency", env.freq_ghz, "GHz");
    report.dim("bandwidth", percent, "%");
    report.dim("resonators", n as f64, "");
    report.dim_mm("filter length", cum[n + 1] as f64 * 1.0e-9);
    report.dim_mm("end gap kept", (2 * half_gap) as f64 * 1.0e-9);
    let mut narrow = Vec::new();
    if percent > 20.0 {
        narrow.push(format!(
            "a bandwidth of {percent:.1} per cent is wide for the narrow-band design equations"
        ));
    }
    report.formula(
        "impedance inverters Z0 J and even/odd impedances Z0e,o = Z0 (1 +- Z0 J + (Z0 J)^2)",
        "Pozar, Microwave Engineering, section 8.7; Hong and Lancaster, Microstrip Filters for RF/Microwave Applications, chapter 5",
        "narrow band (warning above 20 per cent)",
        &narrow,
    );
    report.formula(
        "low-pass prototype values",
        "Matthaei, Young and Jones; Pozar, Microwave Engineering, section 8.3, from eh3d-phoenix-rf",
        "orders 1 to max_filter_order, ripple of the RF settings",
        &[],
    );
    report.formula(
        "open-end extension of the resonators",
        OPEN_END_SOURCE,
        OPEN_END_RANGE,
        &open_end_problems(env, sections[0].width as f64 * 1.0e-9),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{build, Kind};
    use super::*;
    use crate::env::fixtures::{materials, two_layer};
    use eh3d_phoenix_stackup::StackupSettings;

    fn env(freq_ghz: f64) -> Environment {
        Environment::new(
            &two_layer(),
            &materials(),
            None,
            freq_ghz,
            RfgenSettings::default(),
            StackupSettings::default(),
        )
        .unwrap()
    }

    fn run(kind: Kind, params: &Params, freq_ghz: f64) -> Result<(Layout, Report), RfgenError> {
        build(kind, &env(freq_ghz), params, &NetNames::default())
    }

    #[test]
    fn the_book_low_pass_filter_has_the_published_electrical_lengths() {
        // Pozar example 8.5: maximally flat, N = 6, Zl = 20, Zh = 120, cutoff 2.5 GHz
        let p = Params::new().with("order", "6");
        let (layout, report) = run(Kind::Lowpass, &p, 2.5).unwrap();
        // the leads and six sections
        assert_eq!(layout.polys.len(), 8);
        let degrees: Vec<f64> = (1..=6)
            .map(|k| {
                report
                    .value(&format!("section {k} electrical length"))
                    .unwrap()
            })
            .collect();
        for (got, want) in degrees
            .iter()
            .zip([11.86, 33.76, 44.27, 46.12, 32.41, 12.36])
        {
            assert!((got - want).abs() < 0.05, "{degrees:?}");
        }
        // wide lines for the capacitors, narrow lines for the inductors
        let first = report.value("section 1 width").unwrap();
        let second = report.value("section 2 width").unwrap();
        assert!(first > 3.0 * second, "{first} {second}");
        assert_eq!(layout.ports.len(), 2);
    }

    #[test]
    fn a_low_pass_filter_is_longer_at_a_lower_cutoff() {
        let p = Params::new().with("order", "3");
        let high = run(Kind::Lowpass, &p, 2.0).unwrap().1;
        let low = run(Kind::Lowpass, &p, 1.0).unwrap().1;
        assert!(low.value("filter length").unwrap() > 1.8 * high.value("filter length").unwrap());
    }

    #[test]
    fn a_chebyshev_low_pass_filter_needs_a_ripple_and_a_butterworth_refuses_one() {
        let cheb = Params::new().with("family", "chebyshev").with("order", "3");
        assert!(run(Kind::Lowpass, &cheb, 2.0).is_ok());
        let ripple = Params::new().with("ripple_db", "0.5");
        assert_eq!(
            run(Kind::Lowpass, &ripple, 2.0).unwrap_err().code,
            PARAMETER_INVALID
        );
        let big = Params::new().with("order", "10");
        assert_eq!(
            run(Kind::Lowpass, &big, 2.0).unwrap_err().code,
            PARAMETER_INVALID
        );
        let unknown = Params::new().with("family", "elliptic");
        assert!(run(Kind::Lowpass, &unknown, 2.0).is_err());
    }

    #[test]
    fn the_book_band_pass_filter_has_the_published_impedances() {
        // Pozar example: 0.5 dB ripple, N = 3, 10 per cent, 50 ohm
        let (layout, report) = run(Kind::Bandpass, &Params::new(), 2.4).unwrap();
        let ze1 = report.value("section 1 even impedance").unwrap();
        let zo1 = report.value("section 1 odd impedance").unwrap();
        assert!((ze1 - 70.61).abs() < 0.05, "{ze1}");
        assert!((zo1 - 39.24).abs() < 0.05, "{zo1}");
        let ze2 = report.value("section 2 even impedance").unwrap();
        let zo2 = report.value("section 2 odd impedance").unwrap();
        assert!((ze2 - 56.64).abs() < 0.05 && (zo2 - 44.77).abs() < 0.05);
        // sections 1 and 4 are the same, sections 2 and 3 are the same
        let w1 = report.value("section 1 width").unwrap();
        let w4 = report.value("section 4 width").unwrap();
        assert!((w1 - w4).abs() < 1.0e-3);
        let g1 = report.value("section 1 gap").unwrap();
        let g2 = report.value("section 2 gap").unwrap();
        assert!(g1 < g2, "the tighter coupling has the narrower gap");
        // input, three resonators and output are five nets
        assert_eq!(layout.nets.len(), 5);
        assert_eq!(layout.nets[0], "RF");
        assert_eq!(layout.nets[1], "RF_R1");
        assert_eq!(layout.nets[4], "RF_OUT");
        // 2 + 2 * 3 + 2 strips
        assert_eq!(layout.polys.len(), 10);
        assert_eq!(layout.ports.len(), 2);
    }

    #[test]
    fn the_sections_are_about_a_quarter_wave_long() {
        let (_, report) = run(Kind::Bandpass, &Params::new(), 2.4).unwrap();
        let length = report.value("section 2 length").unwrap();
        // free-space quarter wave 31.2 mm, divided by sqrt(eps) of about 1.7, less the extension
        assert!((14.0..20.0).contains(&length), "{length}");
    }

    #[test]
    fn the_strips_of_different_nets_never_touch() {
        let (layout, _) = run(Kind::Bandpass, &Params::new(), 2.4).unwrap();
        for (i, a) in layout.polys.iter().enumerate() {
            for b in &layout.polys[i + 1..] {
                if a.net == b.net {
                    continue;
                }
                let (ax0, ax1) = (a.points[0].0, a.points[2].0);
                let (ay0, ay1) = (a.points[0].1, a.points[2].1);
                let (bx0, bx1) = (b.points[0].0, b.points[2].0);
                let (by0, by1) = (b.points[0].1, b.points[2].1);
                let separated = ax1 < bx0 || bx1 < ax0 || ay1 < by0 || by1 < ay0;
                assert!(separated, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn a_wide_band_warns_and_an_impossible_one_is_refused() {
        let wide = Params::new().with("bandwidth_percent", "30");
        if let Ok((_, report)) = run(Kind::Bandpass, &wide, 2.4) {
            assert!(report.formulas.iter().any(|f| !f.in_range));
        }
        let silly = Params::new().with("bandwidth_percent", "150");
        assert_eq!(
            run(Kind::Bandpass, &silly, 2.4).unwrap_err().code,
            PARAMETER_INVALID
        );
        let order = Params::new().with("order", "0");
        assert!(run(Kind::Bandpass, &order, 2.4).is_err());
    }

    #[test]
    fn a_single_resonator_has_two_sections() {
        let p = Params::new().with("order", "1");
        let (layout, report) = run(Kind::Bandpass, &p, 2.4).unwrap();
        assert_eq!(layout.nets.len(), 3);
        assert!(report.value("section 2 width").is_some());
        assert!(report.value("section 3 width").is_none());
    }
}
