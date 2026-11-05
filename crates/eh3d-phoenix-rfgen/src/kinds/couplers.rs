// SPDX-License-Identifier: GPL-3.0-or-later
//! The Wilkinson divider and the branch-line coupler.

use crate::codes::{RfgenError, NOT_REALISABLE, WARN_APPROXIMATE};
use crate::env::Environment;
use crate::formulas::{
    branch_line as branch_line_impedances, miter_fraction, miter_range,
    wilkinson as wilkinson_impedances,
};
use crate::layout::{even, nm_from_f64, nm_from_m, Layout};
use crate::params::Params;
use crate::report::Report;
use crate::shapes::{miter_leg_nm, mitered_bend};

use super::{describe_line, h_strip, v_strip, NetNames};

/// A Wilkinson power divider: an input lead, two quarter-wave arms of `sqrt(2) Z0` that leave the
/// junction in opposite directions and bend (with a mitered corner) to run side by side, and two
/// output leads with a gap between them where the resistor of `2 Z0` goes.
pub(super) fn wilkinson(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let (z_arm, resistor) = wilkinson_impedances(z0);
    let feed = env.line_for_z0(z0)?;
    let arm = env.line_for_z0(z_arm)?;
    let quarter = nm_from_m(arm.lambda_g_m / 4.0)?;
    let wanted = nm_from_f64(settings.wilkinson_pitch_over_width * arm.width_nm as f64)?;
    let pitch = even(
        wanted
            .max(feed.width_nm + settings.wilkinson_resistor_gap_nm)
            .max(arm.width_nm + settings.wilkinson_resistor_gap_nm),
    );
    let half = pitch / 2;
    let straight = quarter - half;
    if straight <= 2 * arm.width_nm {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            format!("the quarter wave of {quarter} nm is too short for arms {pitch} nm apart"),
        ));
    }
    let w_over_h = arm.width_m() / env.substrate.h_m;
    report.formula(
        "optimum miter M = 52 + 65 exp(-1.35 W/h) per cent",
        "Douville and James, IEEE Trans. MTT, 1978",
        "W/h >= 0.25, er <= 25",
        &miter_range(w_over_h, env.substrate.er),
    );
    let leg = miter_leg_nm(arm.width_nm, miter_fraction(w_over_h))?;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -settings.lead_length_nm, 0, 0, feed.width_nm);
    let upper = mitered_bend((0, half), (0, 1), false, arm.width_nm, half, straight, leg)?;
    let lower = mitered_bend((0, -half), (0, -1), true, arm.width_nm, half, straight, leg)?;
    layout.poly_with_width(net, upper, arm.width_nm);
    layout.poly_with_width(net, lower, arm.width_nm);
    let lead = settings.lead_length_nm;
    h_strip(layout, net, straight, straight + lead, half, feed.width_nm);
    h_strip(layout, net, straight, straight + lead, -half, feed.width_nm);
    layout.port("P1", (-lead, 0), 180, feed.z0_ohm);
    layout.port("P2", (straight + lead, half), 0, feed.z0_ohm);
    layout.port("P3", (straight + lead, -half), 0, feed.z0_ohm);
    describe_line(report, "feed line", &feed);
    describe_line(report, "arm", &arm);
    report.dim("arm impedance wanted", z_arm, "ohm");
    report.dim("isolation resistor", resistor, "ohm");
    report.dim_mm("arm quarter wave", quarter as f64 * 1.0e-9);
    report.dim_mm("arm spacing, centre to centre", pitch as f64 * 1.0e-9);
    report.dim_mm(
        "gap between the output leads",
        (pitch - feed.width_nm) as f64 * 1.0e-9,
    );
    report.formula(
        "Wilkinson divider: arms sqrt(2) Z0 a quarter wave long, resistor 2 Z0",
        "Pozar, Microwave Engineering, section 7.3",
        "narrow band around the design frequency; equal split",
        &[],
    );
    report.warn(
        WARN_APPROXIMATE,
        "the isolation resistor is not placed: fit a resistor across the gap between the output leads",
    );
    report.warn(
        WARN_APPROXIMATE,
        "the arms run side by side and couple; the junction and the corners are not compensated",
    );
    Ok(())
}

/// A branch-line coupler as a rectangular ring of four quarter-wave sections, with a lead at each
/// corner. The two horizontal sections are `Z0 / sqrt(2)` and the two vertical ones `Z0`.
pub(super) fn branch_line(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let (z_series, z_shunt) = branch_line_impedances(z0);
    let series = env.line_for_z0(z_series)?;
    let shunt = env.line_for_z0(z_shunt)?;
    let length_h = even(nm_from_m(series.lambda_g_m / 4.0)?);
    let length_v = even(nm_from_m(shunt.lambda_g_m / 4.0)?);
    let (hx, hy) = (length_h / 2, length_v / 2);
    let lead = settings.lead_length_nm;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -hx, hx, hy, series.width_nm);
    h_strip(layout, net, -hx, hx, -hy, series.width_nm);
    v_strip(layout, net, -hx, -hy, hy, shunt.width_nm);
    v_strip(layout, net, hx, -hy, hy, shunt.width_nm);
    h_strip(layout, net, -hx - lead, -hx, hy, shunt.width_nm);
    h_strip(layout, net, -hx - lead, -hx, -hy, shunt.width_nm);
    h_strip(layout, net, hx, hx + lead, hy, shunt.width_nm);
    h_strip(layout, net, hx, hx + lead, -hy, shunt.width_nm);
    layout.port("P1", (-hx - lead, hy), 180, shunt.z0_ohm);
    layout.port("P2", (hx + lead, hy), 0, shunt.z0_ohm);
    layout.port("P3", (hx + lead, -hy), 0, shunt.z0_ohm);
    layout.port("P4", (-hx - lead, -hy), 180, shunt.z0_ohm);
    describe_line(report, "series section", &series);
    describe_line(report, "shunt section", &shunt);
    report.dim("series impedance wanted", z_series, "ohm");
    report.dim_mm("series section length", length_h as f64 * 1.0e-9);
    report.dim_mm("shunt section length", length_v as f64 * 1.0e-9);
    report.formula(
        "branch-line coupler: sections Z0/sqrt(2) and Z0, each a quarter wave",
        "Pozar, Microwave Engineering, section 7.4",
        "narrow band around the design frequency",
        &[],
    );
    report.warn(
        WARN_APPROXIMATE,
        "the T junctions at the corners are not compensated; the ports are numbered clockwise from the upper left",
    );
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
    fn the_wilkinson_arms_are_a_quarter_wave_at_the_square_root_of_two_times_the_impedance() {
        let (layout, report) = run(Kind::Wilkinson, &Params::new(), 1.0).unwrap();
        assert!((report.value("arm impedance wanted").unwrap() - 70.710_678).abs() < 1.0e-5);
        assert!((report.value("isolation resistor").unwrap() - 100.0).abs() < 1.0e-9);
        assert!((report.value("arm impedance").unwrap() - 70.71).abs() < 0.1);
        let quarter = report.value("arm guided wavelength").unwrap() / 4.0;
        assert!((report.value("arm quarter wave").unwrap() - quarter).abs() < 1.0e-5);
        // the input lead, two arms and two output leads
        assert_eq!(layout.polys.len(), 5);
        assert_eq!(layout.ports.len(), 3);
        assert_eq!(layout.ports[1].at.1, -layout.ports[2].at.1);
        assert_eq!(layout.nets.len(), 1);
    }

    #[test]
    fn the_centre_line_of_an_arm_is_a_quarter_wave_long() {
        let (layout, report) = run(Kind::Wilkinson, &Params::new(), 1.0).unwrap();
        // the vertical run to the corner plus the horizontal run is the quarter wave
        let pitch = report.value("arm spacing, centre to centre").unwrap();
        let quarter = report.value("arm quarter wave").unwrap();
        let end = layout.ports[1].at.0 as f64 / 1.0e6 - 3.0; // minus the output lead
        assert!(
            ((pitch / 2.0 + end) - quarter).abs() < 1.0e-5,
            "{pitch} {end} {quarter}"
        );
    }

    #[test]
    fn the_output_leads_leave_room_for_the_resistor() {
        let (_, report) = run(Kind::Wilkinson, &Params::new(), 1.0).unwrap();
        let gap = report.value("gap between the output leads").unwrap();
        assert!(gap >= 1.5 - 1.0e-9, "{gap}");
        assert!(report
            .warnings
            .iter()
            .any(|w| w.detail.contains("isolation resistor is not placed")));
    }

    #[test]
    fn a_wilkinson_at_ten_gigahertz_is_too_small_for_its_own_pitch() {
        let e = run(Kind::Wilkinson, &Params::new(), 10.0).unwrap_err();
        assert_eq!(e.code, NOT_REALISABLE);
    }

    #[test]
    fn the_branch_line_is_a_ring_of_four_sections_with_four_leads() {
        let (layout, report) = run(Kind::BranchLine, &Params::new(), 1.0).unwrap();
        assert_eq!(layout.polys.len(), 8);
        assert_eq!(layout.ports.len(), 4);
        assert!((report.value("series impedance wanted").unwrap() - 35.355_339).abs() < 1.0e-5);
        let series = report.value("series section width").unwrap();
        let shunt = report.value("shunt section width").unwrap();
        assert!(series > shunt, "the lower impedance is the wider line");
        let lh = report.value("series section length").unwrap();
        let lv = report.value("shunt section length").unwrap();
        let quarter_h = report.value("series section guided wavelength").unwrap() / 4.0;
        let quarter_v = report.value("shunt section guided wavelength").unwrap() / 4.0;
        assert!((lh - quarter_h).abs() < 1.0e-5 && (lv - quarter_v).abs() < 1.0e-5);
        // the ports sit on a rectangle
        let p: Vec<(i64, i64)> = layout.ports.iter().map(|q| q.at).collect();
        assert_eq!(p[0].1, p[1].1);
        assert_eq!(p[2].1, p[3].1);
        assert_eq!(p[0].0, p[3].0);
        assert_eq!(p[1].0, p[2].0);
        assert_eq!(p[0].0, -p[1].0);
    }

    #[test]
    fn a_coupler_takes_only_the_impedance() {
        let bad = Params::new().with("order", "3");
        assert!(run(Kind::BranchLine, &bad, 1.0).is_err());
        assert!(run(Kind::Wilkinson, &bad, 1.0).is_err());
    }
}
