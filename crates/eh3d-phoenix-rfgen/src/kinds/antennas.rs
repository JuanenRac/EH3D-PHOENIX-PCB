// SPDX-License-Identifier: GPL-3.0-or-later
//! The rectangular patch antenna with an inset feed and the printed dipole.

use crate::codes::{RfgenError, NOT_REALISABLE, WARN_APPROXIMATE};
use crate::env::Environment;
use crate::formulas::{
    dipole_arm_m, patch as patch_dimensions, patch_edge_resistance_ohm, patch_inset_m, patch_range,
};
use crate::layout::{even, nm_from_m, Layout};
use crate::params::Params;
use crate::report::Report;

use super::{describe_line, h_strip, v_strip, NetNames};

/// A rectangular patch by the transmission-line model, fed by a line of the given impedance that
/// enters the patch through a notch deep enough to bring the input resistance down to it.
pub(super) fn patch(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let h = env.substrate.h_m;
    let freq = env.freq_hz();
    let dims = patch_dimensions(freq, env.substrate.er, h)?;
    report.formula(
        "patch width, effective permittivity, edge extension and length",
        "Balanis, Antenna Theory, section 14.2 (transmission-line model)",
        "h / lambda0 < 0.1 and W/h >= 1",
        &patch_range(freq, h, dims.width_m),
    );
    let edge = patch_edge_resistance_ohm(
        freq,
        h,
        dims.width_m,
        dims.length_m,
        settings.patch_integral_steps,
    )?;
    let inset_m = patch_inset_m(dims.length_m, edge, z0)?;
    report.formula(
        "edge resistance 1 / (2 (G1 + G12)) and inset Rin(y0) = Rin(0) cos^2(pi y0 / L)",
        "Balanis, Antenna Theory, section 14.2",
        "h << lambda0; the mutual conductance is integrated numerically",
        &[],
    );
    let feed = env.line_for_z0(z0)?;
    let width = even(nm_from_m(dims.width_m)?);
    let length = nm_from_m(dims.length_m)?;
    let inset = nm_from_m(inset_m)?;
    let notch_half = feed.width_nm / 2 + settings.patch_notch_gap_nm;
    if width / 2 <= notch_half + feed.width_nm || inset >= length {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            "the feed line and its notch do not fit in the patch",
        ));
    }
    let lead = settings.lead_length_nm;
    let net = layout.net(&nets.signal);
    let half = width / 2;
    let outline = if inset < 1 {
        vec![(0, -half), (length, -half), (length, half), (0, half)]
    } else {
        vec![
            (0, -half),
            (length, -half),
            (length, half),
            (0, half),
            (0, notch_half),
            (inset, notch_half),
            (inset, -notch_half),
            (0, -notch_half),
        ]
    };
    layout.poly_with_width(net, outline, width.min(length));
    h_strip(
        layout,
        net,
        -lead,
        (inset + feed.width_nm).min(length),
        0,
        feed.width_nm,
    );
    layout.port("FEED", (-lead, 0), 180, feed.z0_ohm);
    describe_line(report, "feed line", &feed);
    report.dim_mm("patch width", dims.width_m);
    report.dim_mm("patch length", dims.length_m);
    report.dim("patch eps_eff", dims.eps_eff, "");
    report.dim_mm("edge extension", dims.delta_l_m);
    report.dim("edge resistance", edge, "ohm");
    report.dim_mm("inset depth", inset_m);
    report.dim_mm("notch width", 2.0 * notch_half as f64 * 1.0e-9);
    report.dim_mm("minimum ground plane width", dims.width_m + 6.0 * h);
    report.dim_mm("minimum ground plane length", dims.length_m + 6.0 * h);
    report.warn(
        WARN_APPROXIMATE,
        "the ground plane should be at least 6 h larger than the patch in both directions (Balanis); \
         the notch and the feed junction are not modelled",
    );
    Ok(())
}

/// A printed half-wave dipole: two collinear strips with a gap at the centre and a balanced pair of
/// feed strips going down from the inner ends. The arms are on two nets, the signal net and the
/// signal net with `_B`.
pub(super) fn dipole(
    env: &Environment,
    _params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let arm_m = dipole_arm_m(env.freq_hz(), env.substrate.er, settings.dipole_shortening);
    let arm = nm_from_m(arm_m)?;
    let strip = even(settings.dipole_strip_width_nm);
    let gap = even(settings.dipole_feed_gap_nm);
    let feed_length = settings.dipole_feed_length_nm;
    if arm <= strip {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            "the arm of the dipole is not longer than its strip is wide",
        ));
    }
    let net_a = layout.net(&nets.signal);
    let net_b = layout.net(&format!("{}_B", nets.signal));
    h_strip(layout, net_a, gap / 2, gap / 2 + arm, 0, strip);
    h_strip(layout, net_b, -gap / 2 - arm, -gap / 2, 0, strip);
    v_strip(
        layout,
        net_a,
        gap / 2 + strip / 2,
        -feed_length,
        strip / 2,
        strip,
    );
    v_strip(
        layout,
        net_b,
        -gap / 2 - strip / 2,
        -feed_length,
        strip / 2,
        strip,
    );
    layout.port("FEED_A", (gap / 2 + strip / 2, -feed_length), 270, 0.0);
    layout.port("FEED_B", (-gap / 2 - strip / 2, -feed_length), 270, 0.0);
    report.dim_mm("arm length", arm_m);
    report.dim_mm("total length", 2.0 * arm_m + gap as f64 * 1.0e-9);
    report.dim_mm("strip width", strip as f64 * 1.0e-9);
    report.dim_mm("feed gap", gap as f64 * 1.0e-9);
    report.dim("shortening", settings.dipole_shortening, "");
    report.formula(
        "half-wave dipole on a dielectric: arm = shortening * lambda0 / (4 sqrt((er + 1) / 2))",
        "rule of thumb for a conductor on a dielectric surface, no single source",
        "strip much narrower than the arm; no ground plane near the antenna",
        &[],
    );
    report.warn(
        WARN_APPROXIMATE,
        "a reference plane under the dipole detunes it: keep the plane out from under the antenna",
    );
    report.warn(
        WARN_APPROXIMATE,
        "the feed is balanced and has no balun; its impedance is not calculated",
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
    fn the_patch_at_two_point_four_gigahertz_on_fr4_is_about_thirty_eight_by_twenty_nine() {
        let (layout, report) = run(Kind::Patch, &Params::new(), 2.4).unwrap();
        let width = report.value("patch width").unwrap();
        let length = report.value("patch length").unwrap();
        assert!((width - 38.01).abs() < 0.05, "{width}");
        assert!((length - 29.42).abs() < 0.05, "{length}");
        assert!((report.value("edge resistance").unwrap() - 160.2).abs() < 1.0);
        assert!((report.value("inset depth").unwrap() - 9.16).abs() < 0.1);
        // the notch makes the outline eight points; the feed is the second shape
        assert_eq!(layout.polys.len(), 2);
        assert_eq!(layout.polys[0].points.len(), 8);
        assert_eq!(layout.ports.len(), 1);
        assert!(report.formulas.iter().all(|f| f.in_range));
    }

    #[test]
    fn the_patch_outline_is_counter_clockwise_and_the_notch_is_centred() {
        let (layout, _) = run(Kind::Patch, &Params::new(), 2.4).unwrap();
        let points = &layout.polys[0].points;
        assert!(crate::shapes::twice_area(points) > 0);
        assert_eq!(points[4].1, -points[7].1);
        assert_eq!(points[5].1, -points[6].1);
        assert_eq!(points[5].0, points[6].0);
        assert_eq!(points[0].1, -points[3].1);
    }

    #[test]
    fn a_feed_of_higher_impedance_than_the_edge_is_refused() {
        // the edge resistance is about 160 ohm: an inset cannot raise it
        let p = Params::new().with("z0", "200");
        let e = run(Kind::Patch, &p, 2.4).unwrap_err();
        assert!(
            e.code == NOT_REALISABLE || e.code == crate::codes::SYNTHESIS_FAILED,
            "{e}"
        );
    }

    #[test]
    fn a_patch_on_a_thick_substrate_at_a_high_frequency_warns_about_the_range() {
        // h / lambda0 = 1.6 mm * 20 GHz / c = 0.107
        match run(Kind::Patch, &Params::new(), 20.0) {
            Ok((_, report)) => assert!(report.formulas.iter().any(|f| !f.in_range)),
            Err(e) => assert_eq!(e.code, NOT_REALISABLE),
        }
    }

    #[test]
    fn a_dipole_has_two_arms_on_two_nets_and_a_balanced_feed() {
        let (layout, report) = run(Kind::Dipole, &Params::new(), 2.4).unwrap();
        assert_eq!(layout.nets, vec!["RF".to_string(), "RF_B".to_string()]);
        assert_eq!(layout.polys.len(), 4);
        assert_eq!(layout.ports.len(), 2);
        // 0.95 * 124.9 mm / (4 * sqrt(2.7)) = 18.05 mm
        let arm = report.value("arm length").unwrap();
        assert!((arm - 18.05).abs() < 0.05, "{arm}");
        let total = report.value("total length").unwrap();
        assert!((total - (2.0 * arm + 1.0)).abs() < 1.0e-6);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.detail.contains("reference plane under the dipole")));
    }

    #[test]
    fn the_dipole_arms_are_mirror_images() {
        let (layout, _) = run(Kind::Dipole, &Params::new(), 2.4).unwrap();
        let a = &layout.polys[0].points;
        let b = &layout.polys[1].points;
        assert_eq!(a[0].0, -b[1].0);
        assert_eq!(a[1].0, -b[0].0);
        assert_eq!(layout.ports[0].at.0, -layout.ports[1].at.0);
    }

    #[test]
    fn a_dipole_takes_no_parameters() {
        assert!(run(Kind::Dipole, &Params::new().with("z0", "50"), 2.4).is_err());
    }
}
