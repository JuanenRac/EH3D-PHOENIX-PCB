// SPDX-License-Identifier: GPL-3.0-or-later
//! Stubs, and the matching structures made of lines and stubs: the single-stub match from the RF
//! tools, the quarter-wave transformer and the L network with distributed elements.

use eh3d_phoenix_rf::matching::{StubKind, Termination, Topology};
use eh3d_phoenix_rf::Complex;

use crate::codes::{RfgenError, NOT_REALISABLE, PARAMETER_INVALID, WARN_APPROXIMATE};
use crate::env::Environment;
use crate::formulas::{degrees_to_m, open_end_extension_m};
use crate::layout::{nm_from_m, Layout};
use crate::params::Params;
use crate::report::Report;

use super::{
    describe_line, h_strip, open_end_problems, v_strip, NetNames, OPEN_END_RANGE, OPEN_END_SOURCE,
};

fn bad(what: &str) -> RfgenError {
    RfgenError::new(PARAMETER_INVALID, what.to_string())
}

fn not_realisable(what: String) -> RfgenError {
    RfgenError::new(NOT_REALISABLE, what)
}

/// The length of a stub in metres from its electrical length in degrees. An open stub is shorter
/// by the extension of its fringing field. The result must stay above zero.
fn stub_length_m(
    env: &Environment,
    report: &mut Report,
    degrees: f64,
    lambda_g_m: f64,
    width_m: f64,
    eps_eff: f64,
    short: bool,
) -> Result<f64, RfgenError> {
    let mut length_m = degrees_to_m(degrees, lambda_g_m);
    if !short {
        let extension_m = open_end_extension_m(width_m, env.substrate.h_m, eps_eff);
        report.dim_mm("open-end extension", extension_m);
        report.formula(
            "open-end extension of the stub",
            OPEN_END_SOURCE,
            OPEN_END_RANGE,
            &open_end_problems(env, width_m),
        );
        length_m -= extension_m;
    }
    if length_m <= 0.0 {
        return Err(not_realisable(
            "the stub is shorter than the fringing field at its open end".to_string(),
        ));
    }
    Ok(length_m)
}

/// Draws the stub going up from the centre line of the main line at `x`: a strip, and for a
/// short-circuited stub a via at its end under a pad that the copper covers.
fn draw_stub(
    env: &Environment,
    layout: &mut Layout,
    net: usize,
    x: i64,
    length: i64,
    width: i64,
    short: bool,
) {
    let settings = &env.settings;
    if short {
        v_strip(
            layout,
            net,
            x,
            0,
            length + settings.via_diameter_nm / 2,
            width,
        );
        layout.via(
            net,
            (x, length),
            settings.via_drill_nm,
            settings.via_diameter_nm,
        );
    } else {
        v_strip(layout, net, x, 0, length, width);
    }
}

/// A through line with a shunt stub in the middle.
pub(super) fn stub(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
    short: bool,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let zs = params.positive("zs", z0)?;
    let degrees = params.positive("degrees", 90.0)?;
    let main_degrees = params.positive("main_degrees", settings.stub_main_degrees)?;
    let main = env.line_for_z0(z0)?;
    let stub = env.line_for_z0(zs)?;
    let length_m = stub_length_m(
        env,
        report,
        degrees,
        stub.lambda_g_m,
        stub.width_m(),
        stub.eps_eff,
        short,
    )?;
    let stub_len = nm_from_m(length_m)?;
    let main_len = nm_from_m(degrees_to_m(main_degrees, main.lambda_g_m))?;
    if main_len <= 2 * stub.width_nm {
        return Err(not_realisable(
            "the main line is too short to hold the stub".to_string(),
        ));
    }
    let net = layout.net(&nets.signal);
    h_strip(layout, net, 0, main_len, 0, main.width_nm);
    draw_stub(
        env,
        layout,
        net,
        main_len / 2,
        stub_len,
        stub.width_nm,
        short,
    );
    layout.port("P1", (0, 0), 180, main.z0_ohm);
    layout.port("P2", (main_len, 0), 0, main.z0_ohm);
    describe_line(report, "main line", &main);
    describe_line(report, "stub", &stub);
    report.dim_mm("main length", main_len as f64 * 1.0e-9);
    report.dim("stub electrical length", degrees, "deg");
    report.dim_mm(
        if short {
            "stub length to the via centre"
        } else {
            "stub length"
        },
        stub_len as f64 * 1.0e-9,
    );
    report.formula(
        "input impedance of an open or short-circuited lossless line",
        "Pozar, Microwave Engineering, section 2.3 (open: -j Z0 cot, short: j Z0 tan)",
        "lossless, the T junction and the via inductance are not modelled",
        &[],
    );
    if short {
        report.warn(
            WARN_APPROXIMATE,
            "the via joins the stub to the reference plane: the signal net is grounded at DC",
        );
    }
    Ok(())
}

/// The single shunt stub that matches a complex load, from the RF tools.
pub(super) fn stub_match(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let re = params.required("load_re")?;
    let im = params.number("load_im")?.unwrap_or(0.0);
    let short = params.choice("termination", &["open", "short"], "open")? == "short";
    let index = params.whole("solution", 1, 1)?;
    let termination = if short {
        Termination::Short
    } else {
        Termination::Open
    };
    let found = eh3d_phoenix_rf::matching::stub_match(
        Complex::new(re, im),
        z0,
        StubKind::Shunt,
        termination,
        &env.rf,
    )
    .map_err(|e| not_realisable(e.to_string()))?;
    let chosen = found.get(index - 1).ok_or_else(|| {
        bad(&format!(
            "there are {} solutions and the solution {index} was asked for",
            found.len()
        ))
    })?;
    let line = env.line_for_z0(z0)?;
    let distance_m = degrees_to_m(chosen.distance_deg, line.lambda_g_m);
    let length_m = stub_length_m(
        env,
        report,
        chosen.stub_deg,
        line.lambda_g_m,
        line.width_m(),
        line.eps_eff,
        short,
    )?;
    let distance = nm_from_m(distance_m)?;
    let stub_len = nm_from_m(length_m)?;
    let lead = settings.lead_length_nm;
    let x_load = lead + distance;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, 0, x_load, 0, line.width_nm);
    draw_stub(env, layout, net, lead, stub_len, line.width_nm, short);
    layout.port("SOURCE", (0, 0), 180, line.z0_ohm);
    layout.port("LOAD", (x_load, 0), 0, line.z0_ohm);
    describe_line(report, "line", &line);
    report.dim("solutions", found.len() as f64, "");
    report.dim("solution used", index as f64, "");
    report.dim("distance load to stub", chosen.distance_deg, "deg");
    report.dim("stub electrical length", chosen.stub_deg, "deg");
    report.dim_mm("distance load to stub, physical", distance as f64 * 1.0e-9);
    report.dim_mm("stub length", stub_len as f64 * 1.0e-9);
    report.formula(
        "single-stub tuning of a complex load",
        "Pozar, Microwave Engineering, section 5.2, computed by eh3d-phoenix-rf",
        "lossless line, stub of the impedance of the line, a shunt stub",
        &[],
    );
    if short {
        report.warn(
            WARN_APPROXIMATE,
            "the via joins the stub to the reference plane: the signal net is grounded at DC",
        );
    }
    Ok(())
}

/// A quarter-wave transformer between two real impedances.
pub(super) fn quarter_wave(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z1 = params.positive("z1", settings.default_z0_ohm)?;
    let z2 = params.required("z2")?;
    if (z1 - z2).abs() <= 1.0e-9 * z1.max(z2) {
        return Err(not_realisable(
            "two equal impedances need no transformer".to_string(),
        ));
    }
    let zt = eh3d_phoenix_rf::matching::quarter_wave_impedance(z1, z2, &env.rf)
        .map_err(|e| not_realisable(e.to_string()))?;
    let first = env.line_for_z0(z1)?;
    let second = env.line_for_z0(z2)?;
    let section = env.line_for_z0(zt)?;
    let length = nm_from_m(section.lambda_g_m / 4.0)?;
    let lead = settings.lead_length_nm;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -lead, 0, 0, first.width_nm);
    h_strip(layout, net, 0, length, 0, section.width_nm);
    h_strip(layout, net, length, length + lead, 0, second.width_nm);
    layout.port("P1", (-lead, 0), 180, first.z0_ohm);
    layout.port("P2", (length + lead, 0), 0, second.z0_ohm);
    describe_line(report, "line 1", &first);
    describe_line(report, "line 2", &second);
    describe_line(report, "transformer", &section);
    report.dim("transformer impedance wanted", zt, "ohm");
    report.dim_mm("transformer length", length as f64 * 1.0e-9);
    report.formula(
        "quarter-wave transformer Zt = sqrt(Z1 Z2)",
        "Pozar, Microwave Engineering, section 5.4, computed by eh3d-phoenix-rf",
        "narrow band around the design frequency; the steps are not compensated",
        &[],
    );
    Ok(())
}

/// A realisation of one L network with distributed elements.
struct Realised {
    series_degrees: f64,
    stub_degrees: f64,
    open: bool,
    series_first: bool,
}

/// An L network with a series line and a shunt stub, made from the first solution of the RF
/// tools that a line and a stub can stand for.
pub(super) fn l_match(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let re = params.required("load_re")?;
    let im = params.number("load_im")?.unwrap_or(0.0);
    let found = eh3d_phoenix_rf::matching::l_match(
        Complex::new(z0, 0.0),
        Complex::new(re, im),
        env.freq_hz(),
        &env.rf,
    )
    .map_err(|e| not_realisable(e.to_string()))?;
    let series_ohm = settings.lmatch_series_line_ohm;
    let mut chosen: Option<Realised> = None;
    for network in &found {
        let x = network.series_reactance_ohm;
        let b = network.shunt_susceptance_s;
        // a line stands for an inductance only: X = Zs sin(theta)
        if !(0.0..=series_ohm).contains(&x) {
            continue;
        }
        let series_degrees = (x / series_ohm).asin().to_degrees();
        if series_degrees > settings.lmatch_max_series_degrees {
            continue;
        }
        let open = b >= 0.0;
        let stub_degrees = if open {
            (b * z0).atan().to_degrees()
        } else {
            (1.0 / (-b * z0)).atan().to_degrees()
        };
        chosen = Some(Realised {
            series_degrees,
            stub_degrees,
            open,
            series_first: network.topology == Topology::SeriesAtSource,
        });
        break;
    }
    let Some(plan) = chosen else {
        return Err(not_realisable(format!(
            "none of the {} solutions has a series inductance that a line of {series_ohm} ohm \
             and at most {} degrees can stand for",
            found.len(),
            settings.lmatch_max_series_degrees
        )));
    };
    if plan.series_degrees < 1.0e-9 && plan.stub_degrees < 1.0e-9 {
        return Err(not_realisable(
            "the load is already matched: the network has no elements".to_string(),
        ));
    }
    let line = env.line_for_z0(z0)?;
    let series_line = env.line_for_z0(series_ohm)?;
    let series_len = nm_from_m(degrees_to_m(plan.series_degrees, series_line.lambda_g_m))?;
    let lead = settings.lead_length_nm;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -lead, 0, 0, line.width_nm);
    if series_len > 0 {
        h_strip(layout, net, 0, series_len, 0, series_line.width_nm);
    }
    h_strip(layout, net, series_len, series_len + lead, 0, line.width_nm);
    let node = if plan.series_first { series_len } else { 0 };
    if plan.stub_degrees > 1.0e-9 {
        let length_m = stub_length_m(
            env,
            report,
            plan.stub_degrees,
            line.lambda_g_m,
            line.width_m(),
            line.eps_eff,
            !plan.open,
        )?;
        let stub_len = nm_from_m(length_m)?;
        draw_stub(env, layout, net, node, stub_len, line.width_nm, !plan.open);
        report.dim_mm("stub length", stub_len as f64 * 1.0e-9);
    }
    layout.port("P1", (-lead, 0), 180, line.z0_ohm);
    layout.port("P2", (series_len + lead, 0), 0, line.z0_ohm);
    describe_line(report, "line", &line);
    describe_line(report, "series line", &series_line);
    report.dim("networks found", found.len() as f64, "");
    report.dim("series line electrical length", plan.series_degrees, "deg");
    report.dim("stub electrical length", plan.stub_degrees, "deg");
    report.dim_mm("series line length", series_len as f64 * 1.0e-9);
    report.formula(
        "L network of a load",
        "Pozar, Microwave Engineering, section 5.1, computed by eh3d-phoenix-rf",
        "lumped elements at the design frequency",
        &[],
    );
    report.formula(
        "series inductance as a short line, X = Zs sin(theta)",
        "approximation of a short high-impedance line (Pozar, section 8.6)",
        "theta at most lmatch_max_series_degrees and Zs well above the line impedance",
        &[],
    );
    if !plan.open {
        report.warn(
            WARN_APPROXIMATE,
            "the via joins the stub to the reference plane: the signal net is grounded at DC",
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{build, Kind};
    use super::*;
    use crate::env::fixtures::{materials, two_layer};
    use crate::settings::RfgenSettings;
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
    fn an_open_stub_of_a_quarter_wave_is_shorter_by_the_fringing_extension() {
        let (layout, report) = run(Kind::StubOpen, &Params::new(), 1.0).unwrap();
        assert_eq!(layout.polys.len(), 2);
        assert!(layout.vias.is_empty());
        let stub = report.value("stub length").unwrap();
        let quarter = report.value("stub guided wavelength").unwrap() / 4.0;
        let extension = report.value("open-end extension").unwrap();
        assert!(
            (stub - (quarter - extension)).abs() < 1.0e-5,
            "{stub} {quarter} {extension}"
        );
        // the extension is a few tenths of the height of the substrate
        assert!((0.3..1.2).contains(&extension), "{extension}");
        assert!((report.value("stub electrical length").unwrap() - 90.0).abs() < 1.0e-9);
    }

    #[test]
    fn the_extension_is_the_published_function_of_the_stub() {
        let e = env(1.0);
        let stub = e.line_for_z0(50.0).unwrap();
        let expected = open_end_extension_m(stub.width_m(), e.substrate.h_m, stub.eps_eff) * 1.0e3;
        let (_, report) = run(Kind::StubOpen, &Params::new(), 1.0).unwrap();
        assert!((report.value("open-end extension").unwrap() - expected).abs() < 1.0e-9);
    }

    #[test]
    fn a_short_stub_ends_in_a_via_at_the_electrical_length() {
        let (layout, report) = run(Kind::StubShort, &Params::new(), 1.0).unwrap();
        assert_eq!(layout.vias.len(), 1);
        let via = &layout.vias[0];
        let length = report.value("stub length to the via centre").unwrap();
        let quarter = report.value("stub guided wavelength").unwrap() / 4.0;
        assert!((length - quarter).abs() < 1.0e-5);
        assert!((via.at.1 as f64 / 1.0e6 - length).abs() < 1.0e-5);
        // the copper covers the pad
        let top = layout.polys[1].points.iter().map(|p| p.1).max().unwrap();
        assert!(top >= via.at.1 + via.diameter / 2);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.detail.contains("grounded at DC")));
    }

    #[test]
    fn a_stub_of_another_impedance_has_its_own_width() {
        let p = Params::new().with("zs", "100");
        let (_, report) = run(Kind::StubOpen, &p, 1.0).unwrap();
        assert!(report.value("stub width").unwrap() < report.value("main line width").unwrap());
    }

    #[test]
    fn a_stub_too_short_for_the_fringing_field_is_refused() {
        let p = Params::new().with("degrees", "1");
        assert_eq!(
            run(Kind::StubOpen, &p, 1.0).unwrap_err().code,
            NOT_REALISABLE
        );
        // the short-circuited one has no fringing field to take off
        assert!(run(Kind::StubShort, &p, 1.0).is_ok());
        let short_main = Params::new().with("main_degrees", "1");
        assert_eq!(
            run(Kind::StubOpen, &short_main, 1.0).unwrap_err().code,
            NOT_REALISABLE
        );
    }

    #[test]
    fn the_stub_that_matches_a_resistive_load_is_made_from_the_rf_tools() {
        // 100 ohm load on a 50 ohm line
        let p = Params::new().with("load_re", "100");
        let (layout, report) = run(Kind::StubMatch, &p, 1.0).unwrap();
        assert_eq!(report.value("solutions"), Some(2.0));
        assert_eq!(layout.ports.len(), 2);
        // the first solution of the RF tools for 2 z0: distance 0.1 lambda... about 35 degrees
        let distance = report.value("distance load to stub").unwrap();
        assert!((0.0..180.0).contains(&distance), "{distance}");
        let second = Params::new().with("load_re", "100").with("solution", "2");
        let (_, other) = run(Kind::StubMatch, &second, 1.0).unwrap();
        assert!(
            (other.value("distance load to stub").unwrap() - distance).abs() > 1.0,
            "the two solutions differ"
        );
        let third = Params::new().with("load_re", "100").with("solution", "3");
        assert_eq!(
            run(Kind::StubMatch, &third, 1.0).unwrap_err().code,
            PARAMETER_INVALID
        );
        assert!(run(Kind::StubMatch, &Params::new(), 1.0).is_err());
    }

    #[test]
    fn the_stub_match_of_a_short_stub_has_a_via() {
        let p = Params::new()
            .with("load_re", "30")
            .with("load_im", "20")
            .with("termination", "short");
        let (layout, _) = run(Kind::StubMatch, &p, 1.0).unwrap();
        assert_eq!(layout.vias.len(), 1);
    }

    #[test]
    fn a_quarter_wave_transformer_from_fifty_to_one_fifty_is_a_section_of_the_geometric_mean() {
        let p = Params::new().with("z2", "150");
        let (layout, report) = run(Kind::QuarterWave, &p, 1.0).unwrap();
        assert_eq!(layout.polys.len(), 3);
        let wanted = report.value("transformer impedance wanted").unwrap();
        assert!((wanted - 86.602_540).abs() < 1.0e-5, "{wanted}");
        assert!((report.value("transformer impedance").unwrap() - wanted).abs() < 0.05);
        let quarter = report.value("transformer guided wavelength").unwrap() / 4.0;
        assert!((report.value("transformer length").unwrap() - quarter).abs() < 1.0e-5);
        let widths = [
            report.value("line 1 width").unwrap(),
            report.value("transformer width").unwrap(),
            report.value("line 2 width").unwrap(),
        ];
        assert!(widths[0] > widths[1] && widths[1] > widths[2], "{widths:?}");
    }

    #[test]
    fn equal_impedances_need_no_transformer() {
        let p = Params::new().with("z2", "50");
        assert_eq!(
            run(Kind::QuarterWave, &p, 1.0).unwrap_err().code,
            NOT_REALISABLE
        );
        assert!(run(Kind::QuarterWave, &Params::new(), 1.0).is_err());
    }

    #[test]
    fn an_l_network_for_a_load_is_made_of_a_line_and_a_stub() {
        // a load of 100 + j50 ohm on a 50 ohm line at 1 GHz
        let p = Params::new().with("load_re", "100").with("load_im", "50");
        match run(Kind::LMatch, &p, 1.0) {
            Ok((layout, report)) => {
                assert!(layout.polys.len() >= 2);
                assert!(report.value("networks found").unwrap() >= 1.0);
                let degrees = report.value("series line electrical length").unwrap();
                assert!(degrees <= 45.0 + 1.0e-9);
                assert_eq!(layout.ports.len(), 2);
            }
            Err(e) => assert_eq!(e.code, NOT_REALISABLE),
        }
    }

    #[test]
    fn a_matched_load_or_a_missing_load_is_an_error() {
        let matched = Params::new().with("load_re", "50");
        assert!(run(Kind::LMatch, &matched, 1.0).is_err());
        assert!(run(Kind::LMatch, &Params::new(), 1.0).is_err());
    }
}
