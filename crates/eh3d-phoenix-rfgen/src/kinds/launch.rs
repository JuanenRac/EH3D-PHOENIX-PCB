// SPDX-License-Identifier: GPL-3.0-or-later
//! The edge launch of an SMA connector: the footprint of the connector with its signal pad and two
//! ground pads, the line that leaves it, the ground strips along the line and the fence of vias
//! that stitches them to the reference plane.

use crate::codes::{RfgenError, NOT_REALISABLE, PARAMETER_INVALID, WARN_APPROXIMATE};
use crate::env::Environment;
use crate::formulas::fence_spacing_m;
use crate::layout::{even, nm_from_m, rect_points, Layout, PadSpec, PartSpec};
use crate::params::Params;
use crate::report::Report;

use super::{describe_line, h_strip, NetNames};

/// The launch of an edge-mounted SMA connector. The connector sits at `x = 0` and its pads run
/// along the line; the line is `length_mm` long. The vias of the fence are a fraction
/// (`fence_spacing_wavelengths`, 1/20 by default) of the guided wavelength apart.
pub(super) fn sma(
    env: &Environment,
    params: &Params,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) -> Result<(), RfgenError> {
    let settings = &env.settings;
    let z0 = params.positive("z0", settings.default_z0_ohm)?;
    let length_mm = params.positive("length_mm", 10.0)?;
    let line = env.line_for_z0(z0)?;
    let width = line.width_nm;
    let length = nm_from_m(length_mm * 1.0e-3)?;
    let pad_length = even(settings.sma_pad_length_nm);
    let signal_width = even(settings.sma_signal_pad_width_nm);
    let ground_width = even(settings.sma_ground_pad_width_nm);
    let signal = layout.net(&nets.signal);
    let ground = layout.net(&nets.ground);

    // the ground copper starts a gap away from the widest of the pad and the line
    let inner = width.max(signal_width) / 2 + settings.sma_ground_gap_nm;
    let ring = settings.via_diameter_nm + 2 * settings.fence_gap_nm;
    let outer = inner + ring.max(ground_width);
    let offset = inner + ground_width / 2;
    layout.parts.push(PartSpec {
        prefix: "J".to_owned(),
        value: "SMA edge launch".to_owned(),
        footprint: "SMA-EDGE".to_owned(),
        at: (0, 0),
        pads: vec![
            PadSpec {
                number: "1".to_owned(),
                width: pad_length,
                height: signal_width,
                at: (pad_length / 2, 0),
                net: signal,
            },
            PadSpec {
                number: "2".to_owned(),
                width: pad_length,
                height: ground_width,
                at: (pad_length / 2, offset),
                net: ground,
            },
            PadSpec {
                number: "3".to_owned(),
                width: pad_length,
                height: ground_width,
                at: (pad_length / 2, -offset),
                net: ground,
            },
        ],
    });
    let end = pad_length + length;
    h_strip(layout, signal, pad_length, end, 0, width);
    layout.poly_with_width(ground, rect_points(0, inner, end, outer), outer - inner);
    layout.poly_with_width(ground, rect_points(0, -outer, end, -inner), outer - inner);

    // the fence
    let spacing = nm_from_m(fence_spacing_m(
        line.lambda_g_m,
        settings.fence_spacing_wavelengths,
    ))?;
    let diameter = settings.via_diameter_nm;
    if spacing <= diameter {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            format!(
                "the spacing of {spacing} nm leaves no room between vias of {diameter} nm: \
                 use smaller vias or a larger fence_spacing_wavelengths"
            ),
        ));
    }
    let first = pad_length + diameter;
    let last = end - diameter;
    if last < first {
        return Err(RfgenError::new(
            PARAMETER_INVALID,
            "length_mm is too short for a via beyond the pads of the connector",
        ));
    }
    let centre = inner + ring / 2;
    let mut count: usize = 0;
    let mut x = first;
    while x <= last {
        layout.via(ground, (x, centre), settings.via_drill_nm, diameter);
        layout.via(ground, (x, -centre), settings.via_drill_nm, diameter);
        count += 1;
        x += spacing;
        if count > settings.max_items {
            return Err(RfgenError::new(
                PARAMETER_INVALID,
                "the fence has more vias than max_items",
            ));
        }
    }
    layout.port("CONNECTOR", (0, 0), 180, line.z0_ohm);
    layout.port("LINE", (end, 0), 0, line.z0_ohm);
    describe_line(report, "line", &line);
    report.dim_mm("line length", length as f64 * 1.0e-9);
    report.dim_mm("via spacing", spacing as f64 * 1.0e-9);
    report.dim("vias per side", count as f64, "");
    report.dim_mm("via centres from the line axis", centre as f64 * 1.0e-9);
    report.dim_mm("ground strip width", (outer - inner) as f64 * 1.0e-9);
    report.formula(
        "via fence spaced at a fraction of the guided wavelength (1/20 by default)",
        "common design practice, no single source",
        "fence_spacing_wavelengths in the settings",
        &[],
    );
    report.warn(
        WARN_APPROXIMATE,
        "the pads are generic sizes from the settings, not the land pattern of a connector data sheet: check them against the connector",
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

    fn env_with(settings: RfgenSettings, freq_ghz: f64) -> Environment {
        Environment::new(
            &two_layer(),
            &materials(),
            None,
            freq_ghz,
            settings,
            StackupSettings::default(),
        )
        .unwrap()
    }

    fn run(
        params: &Params,
        settings: RfgenSettings,
        freq_ghz: f64,
    ) -> Result<(Layout, Report), RfgenError> {
        build(
            Kind::Sma,
            &env_with(settings, freq_ghz),
            params,
            &NetNames::default(),
        )
    }

    #[test]
    fn the_launch_has_a_connector_a_line_two_strips_and_a_fence() {
        let (layout, report) = run(&Params::new(), RfgenSettings::default(), 2.4).unwrap();
        assert_eq!(layout.parts.len(), 1);
        assert_eq!(layout.parts[0].pads.len(), 3);
        // the line and the two ground strips
        assert_eq!(layout.polys.len(), 3);
        let per_side = report.value("vias per side").unwrap() as usize;
        assert!(per_side >= 2, "{per_side}");
        assert_eq!(layout.vias.len(), 2 * per_side);
        assert_eq!(layout.nets, ["RF", "GND"]);
        assert!(layout.vias.iter().all(|v| v.net == 1));
        assert_eq!(layout.ports.len(), 2);
    }

    #[test]
    fn the_fence_is_a_twentieth_of_the_guided_wavelength_apart() {
        let (layout, report) = run(&Params::new(), RfgenSettings::default(), 2.4).unwrap();
        let lambda = report.value("line guided wavelength").unwrap();
        let spacing = report.value("via spacing").unwrap();
        assert!(
            (spacing - lambda / 20.0).abs() < 1.0e-5,
            "{spacing} {lambda}"
        );
        let upper: Vec<i64> = layout
            .vias
            .iter()
            .filter(|v| v.at.1 > 0)
            .map(|v| v.at.0)
            .collect();
        for pair in upper.windows(2) {
            assert!(((pair[1] - pair[0]) as f64 / 1.0e6 - spacing).abs() < 1.0e-5);
            assert!(pair[1] > pair[0]);
        }
        // the two sides are mirror images
        let lower: Vec<i64> = layout
            .vias
            .iter()
            .filter(|v| v.at.1 < 0)
            .map(|v| v.at.0)
            .collect();
        assert_eq!(upper, lower);
    }

    #[test]
    fn the_distance_of_the_fence_is_configurable() {
        let settings = RfgenSettings {
            fence_spacing_wavelengths: 0.1,
            ..RfgenSettings::default()
        };
        let (_, wide) = run(&Params::new(), settings, 2.4).unwrap();
        let (_, narrow) = run(&Params::new(), RfgenSettings::default(), 2.4).unwrap();
        let ratio = wide.value("via spacing").unwrap() / narrow.value("via spacing").unwrap();
        assert!((ratio - 2.0).abs() < 1.0e-3, "{ratio}");
    }

    #[test]
    fn the_ground_copper_keeps_away_from_the_signal() {
        let (layout, _) = run(&Params::new(), RfgenSettings::default(), 2.4).unwrap();
        let line = &layout.polys[0].points;
        let upper = &layout.polys[1].points;
        // the line is net 0, the strips net 1
        assert_eq!(layout.polys[0].net, 0);
        assert_eq!(layout.polys[1].net, 1);
        let line_top = line[2].1;
        let strip_bottom = upper[0].1;
        assert!(
            strip_bottom - line_top >= 400_000,
            "{line_top} {strip_bottom}"
        );
        // the ground pads start where the strip starts, so that they join it
        let pad = &layout.parts[0].pads[1];
        assert_eq!(pad.at.1 - pad.height / 2, strip_bottom);
    }

    #[test]
    fn a_fence_that_cannot_fit_is_refused() {
        // 30 GHz: a twentieth of the guided wavelength is smaller than the via
        let e = run(&Params::new(), RfgenSettings::default(), 30.0).unwrap_err();
        assert_eq!(e.code, NOT_REALISABLE);
        let short = Params::new().with("length_mm", "1");
        let e = run(&short, RfgenSettings::default(), 2.4).unwrap_err();
        assert_eq!(e.code, PARAMETER_INVALID);
    }
}
