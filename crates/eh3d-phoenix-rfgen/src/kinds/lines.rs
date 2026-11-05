// SPDX-License-Identifier: GPL-3.0-or-later
//! Lines, bends, curves and tapers.

use crate::codes::{RfgenError, PARAMETER_INVALID};
use crate::env::Environment;
use crate::formulas::{degrees_to_m, miter_fraction, miter_range};
use crate::layout::{even, nm_from_f64, nm_from_m, Layout};
use crate::params::Params;
use crate::report::Report;
use crate::shapes::{miter_leg_nm, mitered_bend};

use super::{describe_line, h_strip, v_strip, NetNames};

fn bad(what: &str) -> RfgenError {
    RfgenError::new(PARAMETER_INVALID, what.to_string())
}

/// A length given in millimetres, in nanometres.
fn mm_to_nm(mm: f64) -> Result<i64, RfgenError> {
    nm_from_m(mm * 1.0e-3)
}

/// A microstrip line of an electrical length or a physical length.
pub(super) fn line(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let line = env.line_for_z0(z0)?;
    let degrees = params.number("degrees")?;
    let length_mm = params.number("length_mm")?;
    let length_m = match (degrees, length_mm) {
        (Some(_), Some(_)) => return Err(bad("give either degrees or length_mm, not both")),
        (Some(d), None) if d > 0.0 => degrees_to_m(d, line.lambda_g_m),
        (None, Some(mm)) if mm > 0.0 => mm * 1.0e-3,
        (None, None) => degrees_to_m(settings.default_line_degrees, line.lambda_g_m),
        _ => return Err(bad("the length must be above zero")),
    };
    let length = nm_from_m(length_m)?;
    if length < 1 {
        return Err(bad("the length is less than one nanometre"));
    }
    let net = layout.net(&nets.signal);
    h_strip(layout, net, 0, length, 0, line.width_nm);
    layout.port("P1", (0, 0), 180, line.z0_ohm);
    layout.port("P2", (length, 0), 0, line.z0_ohm);
    describe_line(report, "line", &line);
    report.dim_mm("length", length as f64 * 1.0e-9);
    report.dim(
        "electrical length",
        length as f64 * 1.0e-9 / line.lambda_g_m * 360.0,
        "deg",
    );
    report.formula(
        "guided wavelength",
        "lambda_g = c / (f sqrt(eps_eff)) with eps_eff of the quasi-static microstrip model",
        "substrate height below 0.05 of the guided wavelength (no dispersion)",
        &[],
    );
    Ok(())
}

/// The turn of a bend or a curve: true for a left turn.
fn turn_left(params: &Params) -> Result<bool, RfgenError> {
    Ok(params.choice("turn", &["left", "right"], "left")? == "left")
}

/// The length of the arms of a bend or a curve.
fn arm_nm(env: &Environment, params: &Params) -> Result<i64, RfgenError> {
    match params.number("arm_mm")? {
        Some(mm) if mm > 0.0 => mm_to_nm(mm),
        Some(_) => Err(bad("arm_mm must be above zero")),
        None => Ok(env.settings.lead_length_nm),
    }
}

/// A right-angle bend with the miter of Douville and James.
pub(super) fn bend(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let line = env.line_for_z0(z0)?;
    let width = line.width_nm;
    let arm = arm_nm(env, params)?;
    let left = turn_left(params)?;
    let w_over_h = line.width_m() / env.substrate.h_m;
    let fraction = miter_fraction(w_over_h);
    report.formula(
        "optimum miter M = 52 + 65 exp(-1.35 W/h) per cent",
        "Douville and James, IEEE Trans. MTT, 1978",
        "W/h >= 0.25, er <= 25",
        &miter_range(w_over_h, env.substrate.er),
    );
    let leg = miter_leg_nm(width, fraction)?;
    let minimum = settings.miter_min_arm_over_width * width as f64;
    if (arm as f64) < minimum {
        return Err(bad(&format!(
            "the arms of {arm} nm are shorter than {} widths ({minimum:.0} nm)",
            settings.miter_min_arm_over_width
        )));
    }
    let points = mitered_bend((0, 0), (1, 0), left, width, arm, arm, leg)?;
    let net = layout.net(&nets.signal);
    layout.poly_with_width(net, points, width);
    layout.port("P1", (-arm, 0), 180, line.z0_ohm);
    if left {
        layout.port("P2", (0, arm), 90, line.z0_ohm);
    } else {
        layout.port("P2", (0, -arm), 270, line.z0_ohm);
    }
    describe_line(report, "line", &line);
    report.dim("miter", fraction * 100.0, "% of the diagonal");
    report.dim_mm("cut along each outer edge", leg as f64 * 1.0e-9);
    report.dim_mm("arm", arm as f64 * 1.0e-9);
    Ok(())
}

/// A right-angle bend as an arc of a track, with a straight arm of copper at each end.
pub(super) fn curve(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let line = env.line_for_z0(z0)?;
    let width = line.width_nm;
    let arm = arm_nm(env, params)?;
    let left = turn_left(params)?;
    let radius = match params.number("radius_mm")? {
        Some(mm) if mm > 0.0 => mm_to_nm(mm)?,
        Some(_) => return Err(bad("radius_mm must be above zero")),
        None => nm_from_f64(settings.curve_radius_over_width * width as f64)?,
    };
    if radius <= width / 2 {
        return Err(bad(
            "the radius must be larger than half the width of the line",
        ));
    }
    let wanted = settings.curve_radius_over_width * width as f64;
    let mut problems = Vec::new();
    if ((radius + 1) as f64) < wanted {
        problems.push(format!(
            "the radius {radius} nm is below {} widths ({wanted:.0} nm)",
            settings.curve_radius_over_width
        ));
    }
    report.formula(
        "curved bend of a radius of a few widths",
        "rule of thumb of the microstrip literature, no single source",
        "R >= curve_radius_over_width * W",
        &problems,
    );
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -arm, 0, 0, width);
    let (end_y, sweep, far_y) = if left {
        (radius, 90_000_000, radius + arm)
    } else {
        (-radius, -90_000_000, -radius - arm)
    };
    layout.arc(net, width, (0, 0), (radius, end_y), sweep);
    v_strip(layout, net, radius, end_y, far_y, width);
    layout.port("P1", (-arm, 0), 180, line.z0_ohm);
    layout.port(
        "P2",
        (radius, far_y),
        if left { 90 } else { 270 },
        line.z0_ohm,
    );
    describe_line(report, "line", &line);
    report.dim_mm("radius", radius as f64 * 1.0e-9);
    report.dim_mm(
        "arc length",
        radius as f64 * 1.0e-9 * std::f64::consts::FRAC_PI_2,
    );
    report.dim_mm("arm", arm as f64 * 1.0e-9);
    Ok(())
}

/// A linear transition between the widths of two impedances.
pub(super) fn taper(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z_a = params.positive("z0", settings.default_z0_ohm)?;
    let z_b = params.required("z0_b")?;
    let a = env.line_for_z0(z_a)?;
    let b = env.line_for_z0(z_b)?;
    let degrees = params.positive("degrees", settings.taper_length_degrees)?;
    let mean_lambda = (a.lambda_g_m + b.lambda_g_m) / 2.0;
    let length = nm_from_m(degrees_to_m(degrees, mean_lambda))?;
    if length < 1 {
        return Err(bad("the taper is shorter than one nanometre"));
    }
    let lead = settings.lead_length_nm;
    let net = layout.net(&nets.signal);
    h_strip(layout, net, -lead, 0, 0, a.width_nm);
    let (ha, hb) = (a.width_nm / 2, b.width_nm / 2);
    layout.poly_with_width(
        net,
        vec![(0, -ha), (length, -hb), (length, hb), (0, ha)],
        even(a.width_nm.min(b.width_nm)),
    );
    h_strip(layout, net, length, length + lead, 0, b.width_nm);
    layout.port("P1", (-lead, 0), 180, a.z0_ohm);
    layout.port("P2", (length + lead, 0), 0, b.z0_ohm);
    describe_line(report, "line A", &a);
    describe_line(report, "line B", &b);
    report.dim_mm("taper length", length as f64 * 1.0e-9);
    let mut problems = Vec::new();
    if degrees < 90.0 {
        problems.push(format!(
            "a taper of {degrees:.1} degrees is shorter than a quarter wavelength and reflects more"
        ));
    }
    report.formula(
        "linear taper between two widths",
        "tapered lines, Pozar, Microwave Engineering; length from the mean guided wavelength",
        "at least a quarter wavelength (rule of thumb)",
        &problems,
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
    fn a_quarter_wave_line_at_one_gigahertz_is_about_forty_one_millimetres() {
        let (layout, report) = run(Kind::Line, &Params::new(), 1.0).unwrap();
        assert_eq!(layout.polys.len(), 1);
        let length = report.value("length").unwrap();
        assert!((39.0..43.0).contains(&length), "{length}");
        let degrees = report.value("electrical length").unwrap();
        assert!((degrees - 90.0).abs() < 0.001, "{degrees}");
        let width = report.value("line width").unwrap();
        assert!((2.8..3.2).contains(&width), "{width}");
        assert_eq!(layout.ports.len(), 2);
        assert!((layout.ports[1].at.0 as f64 / 1.0e6 - length).abs() < 1.0e-9);
    }

    #[test]
    fn the_length_can_be_given_in_millimetres_but_not_twice() {
        let (layout, report) =
            run(Kind::Line, &Params::new().with("length_mm", "12.5"), 1.0).unwrap();
        assert_eq!(layout.ports[1].at.0, 12_500_000);
        assert!((report.value("length").unwrap() - 12.5).abs() < 1.0e-9);
        let both = Params::new().with("length_mm", "12").with("degrees", "45");
        assert_eq!(
            run(Kind::Line, &both, 1.0).unwrap_err().code,
            PARAMETER_INVALID
        );
        let wrong = Params::new().with("degrees", "-5");
        assert!(run(Kind::Line, &wrong, 1.0).is_err());
        let unknown = Params::new().with("lenght_mm", "5");
        assert!(run(Kind::Line, &unknown, 1.0).is_err());
    }

    #[test]
    fn a_higher_impedance_is_a_narrower_line() {
        let wide = run(Kind::Line, &Params::new().with("z0", "30"), 1.0)
            .unwrap()
            .1;
        let narrow = run(Kind::Line, &Params::new().with("z0", "90"), 1.0)
            .unwrap()
            .1;
        assert!(wide.value("line width").unwrap() > narrow.value("line width").unwrap());
    }

    #[test]
    fn the_miter_of_a_bend_follows_the_published_fraction() {
        let (layout, report) = run(Kind::Bend, &Params::new(), 1.0).unwrap();
        assert_eq!(layout.polys.len(), 1);
        assert_eq!(layout.polys[0].points.len(), 7);
        // W/h = 3.0 / 1.6 = 1.88: M = 52 + 65 exp(-1.35 * 1.88) = 57.2 per cent
        let miter = report.value("miter").unwrap();
        assert!((miter - 57.2).abs() < 0.5, "{miter}");
        assert_eq!(layout.ports[1].angle_deg, 90);
        let (right, _) = run(Kind::Bend, &Params::new().with("turn", "right"), 1.0).unwrap();
        assert_eq!(right.ports[1].angle_deg, 270);
        assert_eq!(right.ports[1].at.1, -layout.ports[1].at.1);
    }

    #[test]
    fn short_arms_or_a_bad_turn_are_refused() {
        let short = Params::new().with("arm_mm", "2");
        assert_eq!(
            run(Kind::Bend, &short, 1.0).unwrap_err().code,
            PARAMETER_INVALID
        );
        let turn = Params::new().with("turn", "up");
        assert!(run(Kind::Bend, &turn, 1.0).is_err());
        assert!(run(Kind::Bend, &Params::new().with("arm_mm", "0"), 1.0).is_err());
        // an arm just long enough is accepted
        let ok = Params::new().with("arm_mm", "7");
        assert!(run(Kind::Bend, &ok, 1.0).is_ok());
    }

    #[test]
    fn a_curve_is_an_arc_between_two_arms() {
        let (layout, report) = run(Kind::Curve, &Params::new(), 1.0).unwrap();
        assert_eq!(layout.polys.len(), 2);
        assert_eq!(layout.arcs.len(), 1);
        let arc = &layout.arcs[0];
        assert_eq!(arc.sweep_udeg, 90_000_000);
        let radius = report.value("radius").unwrap();
        // three widths
        assert!((radius - 9.0).abs() < 0.2, "{radius}");
        assert_eq!(arc.end.0, arc.end.1);
        assert!(report.formulas.iter().all(|f| f.in_range));
        let (right, _) = run(Kind::Curve, &Params::new().with("turn", "right"), 1.0).unwrap();
        assert_eq!(right.arcs[0].sweep_udeg, -90_000_000);
        assert_eq!(right.arcs[0].end.1, -right.arcs[0].end.0);
    }

    #[test]
    fn a_small_radius_warns_and_a_tiny_one_is_refused() {
        let (_, report) = run(Kind::Curve, &Params::new().with("radius_mm", "4"), 1.0).unwrap();
        assert!(report.formulas.iter().any(|f| !f.in_range));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.detail.contains("below 3 widths")));
        let e = run(Kind::Curve, &Params::new().with("radius_mm", "1"), 1.0).unwrap_err();
        assert_eq!(e.code, PARAMETER_INVALID);
    }

    #[test]
    fn a_taper_joins_the_two_widths() {
        let (layout, report) = run(
            Kind::Taper,
            &Params::new().with("z0", "50").with("z0_b", "100"),
            1.0,
        )
        .unwrap();
        assert_eq!(layout.polys.len(), 3);
        let a = report.value("line A width").unwrap();
        let b = report.value("line B width").unwrap();
        assert!(a > b);
        let trapezoid = &layout.polys[1].points;
        assert_eq!(trapezoid.len(), 4);
        assert_eq!(trapezoid[0].1, -trapezoid[3].1);
        assert_eq!(trapezoid[1].1, -trapezoid[2].1);
        assert!(trapezoid[3].1 > trapezoid[2].1);
        // twenty degrees is short for a taper: the range note says so
        assert!(report.formulas.iter().any(|f| !f.in_range));
        let missing = run(Kind::Taper, &Params::new(), 1.0).unwrap_err();
        assert_eq!(missing.code, PARAMETER_INVALID);
    }

    #[test]
    fn an_impedance_no_line_has_is_a_synthesis_error() {
        let e = run(Kind::Line, &Params::new().with("z0", "2"), 1.0).unwrap_err();
        assert_eq!(e.code, crate::codes::SYNTHESIS_FAILED);
    }
}
