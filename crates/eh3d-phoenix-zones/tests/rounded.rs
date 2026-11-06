// SPDX-License-Identifier: GPL-3.0-or-later
//! Rounded corners of the fill and of the thermal relief, on boards written as JSON so the test
//! does not depend on how the board model is built.

use eh3d_phoenix_board::Board;
use eh3d_phoenix_zones::codes::SETTINGS_INVALID;
use eh3d_phoenix_zones::{
    check_fill, fill_board, has_errors, FillReport, ZoneFile, ZoneRules, ZoneSettings,
};

const MM: i64 = 1_000_000;

const LAYER: &str = "01890a5d-ac96-7ab2-80e2-4536629c1301";
const NET: &str = "01890a5d-ac96-7ab2-80e2-4536629c1302";
const ZONE: &str = "01890a5d-ac96-7ab2-80e2-4536629c130a";

/// A board of 20 mm by 20 mm with one copper layer and one net, a zone of 18 mm by 18 mm and,
/// when asked, a round pad of 1 mm in the middle that belongs to the net.
fn board_json(with_pad: bool) -> String {
    let (footprints, components, pad_nets) = if with_pad {
        (
            r#"{"01890a5d-ac96-7ab2-80e2-4536629c1330": {"name": "round", "pads": {
                 "01890a5d-ac96-7ab2-80e2-4536629c1331": {
                   "number": "1", "shape": {"shape": "circle", "diameter": 1000000},
                   "at": {"x_nm": 0, "y_nm": 0}, "rotation": 0,
                   "layers": ["01890a5d-ac96-7ab2-80e2-4536629c1301"]}}}}"#,
            r#"{"01890a5d-ac96-7ab2-80e2-4536629c1332": {
                 "refdes": "C1", "value": "1u",
                 "footprint": "01890a5d-ac96-7ab2-80e2-4536629c1330",
                 "at": {"x_nm": 10000000, "y_nm": 10000000}, "rotation": 0, "side": "top"}}"#,
            r#"[{"component": "01890a5d-ac96-7ab2-80e2-4536629c1332",
                 "pad": "01890a5d-ac96-7ab2-80e2-4536629c1331",
                 "net": "01890a5d-ac96-7ab2-80e2-4536629c1302"}]"#,
        )
    } else {
        ("{}", "{}", "[]")
    };
    format!(
        r#"{{
  "name": "rounded",
  "outline": [{{"x_nm": 0, "y_nm": 0}}, {{"x_nm": 20000000, "y_nm": 0}},
              {{"x_nm": 20000000, "y_nm": 20000000}}, {{"x_nm": 0, "y_nm": 20000000}}],
  "layers": {{"{LAYER}": {{"name": "F.Cu", "kind": "copper"}}}},
  "nets": {{"{NET}": {{"name": "GND"}}}},
  "footprints": {footprints},
  "components": {components},
  "pad_nets": {pad_nets},
  "tracks": {{}},
  "vias": {{}},
  "zones": {{"{ZONE}": {{
    "net": "{NET}", "layer": "{LAYER}", "clearance": 200000,
    "outline": [{{"x_nm": 1000000, "y_nm": 1000000}}, {{"x_nm": 19000000, "y_nm": 1000000}},
                {{"x_nm": 19000000, "y_nm": 19000000}}, {{"x_nm": 1000000, "y_nm": 19000000}}]}}}}
}}"#
    )
}

fn board(with_pad: bool) -> Board {
    Board::from_bytes(board_json(with_pad).as_bytes()).unwrap()
}

fn rules() -> ZoneRules {
    ZoneRules {
        min_clearance_nm: 200_000,
        min_edge_clearance_nm: 0,
        class_clearance_nm: std::collections::BTreeMap::new(),
    }
}

fn settings(radius: i64) -> ZoneSettings {
    ZoneSettings {
        keep_unconnected_islands: true,
        min_island_area_nm2: 0,
        corner_radius_nm: radius,
        ..ZoneSettings::default()
    }
}

fn cutout_file() -> ZoneFile {
    // a hole of 4 mm by 4 mm in the middle of the zone
    ZoneFile::from_json(&format!(
        r#"{{"format": "phoenix-zones/1", "zones": [{{"zone": "{ZONE}", "cutouts":
           [[[8000000, 8000000], [12000000, 8000000], [12000000, 12000000], [8000000, 12000000]]]}}]}}"#
    ))
    .unwrap()
}

fn fill(board: &Board, radius: i64, file: Option<&ZoneFile>) -> FillReport {
    fill_board(board, &rules(), &settings(radius), file).unwrap()
}

fn area(report: &FillReport) -> i64 {
    report.fill.zones.iter().map(|z| z.area_nm2).sum()
}

fn vertices(report: &FillReport) -> usize {
    report
        .fill
        .zones
        .iter()
        .flat_map(|z| z.polygons.iter())
        .map(Vec::len)
        .sum()
}

fn inside(polygon: &[[i64; 2]], x: i64, y: i64) -> bool {
    let n = polygon.len();
    (0..n).all(|i| {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        let cross = i128::from(b[0] - a[0]) * i128::from(y - a[1])
            - i128::from(b[1] - a[1]) * i128::from(x - a[0]);
        cross >= 0
    })
}

fn covered(report: &FillReport, x: i64, y: i64) -> bool {
    report
        .fill
        .zones
        .iter()
        .flat_map(|z| z.polygons.iter())
        .any(|p| inside(p, x, y))
}

/// Whether a point is inside a convex polygon or at most `slack` nanometres outside it.
fn near_inside(polygon: &[[i64; 2]], x: i64, y: i64, slack: f64) -> bool {
    let n = polygon.len();
    (0..n).all(|i| {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        let (dx, dy) = ((b[0] - a[0]) as f64, (b[1] - a[1]) as f64);
        let cross = dx * (y - a[1]) as f64 - dy * (x - a[0]) as f64;
        cross / dx.hypot(dy) >= -slack
    })
}

/// What a right-angle corner loses to an arc of radius `r`: `r^2 (1 - pi / 4)`.
fn corner_area(r: i64) -> f64 {
    let r = r as f64;
    r * r * (1.0 - std::f64::consts::FRAC_PI_4)
}

#[test]
fn the_settings_read_the_radius_and_the_tolerance_and_refuse_what_is_out_of_range() {
    let s = ZoneSettings::parse("corner_radius_nm = 500000\ncorner_tolerance_nm = 250\n").unwrap();
    assert_eq!(s.corner_radius_nm, 500_000);
    assert_eq!(s.corner_tolerance_nm, 250);
    let default = ZoneSettings::default();
    assert_eq!(default.corner_radius_nm, 0);
    assert_eq!(default.corner_tolerance_nm, 1000);
    for text in [
        "corner_radius_nm = -1",
        "corner_radius_nm = 5000000000",
        "corner_radius_nm = wide",
        "corner_tolerance_nm = 0",
        "corner_tolerance_nm = 2000000",
    ] {
        let error = ZoneSettings::parse(text).unwrap_err();
        assert_eq!(error.code, SETTINGS_INVALID, "{text}");
    }
}

#[test]
fn a_radius_of_zero_changes_nothing() {
    let board = board(false);
    let sharp = fill(&board, 0, None);
    assert_eq!(sharp.fill.zones[0].polygons.len(), 1);
    assert_eq!(sharp.fill.zones[0].polygons[0].len(), 4);
    assert_eq!(sharp.fill.zones[0].area_nm2, 324_000_000_000_000);
}

#[test]
fn a_zone_alone_loses_its_four_corners_to_arcs_of_the_radius() {
    let board = board(false);
    let rounded = fill(&board, MM, None);
    assert_eq!(rounded.fill.zones[0].polygons.len(), 1);
    // four corners of 33 points each, a sagitta of at most one micrometre
    assert_eq!(rounded.fill.zones[0].polygons[0].len(), 4 * 33);
    let removed = (324_000_000_000_000 - area(&rounded)) as f64;
    let exact = 4.0 * corner_area(MM);
    // the chords lie inside the arcs, so a little more than the circle takes is taken
    assert!(
        removed > exact && removed < exact * 1.006,
        "{removed} {exact}"
    );
    // the middle of each side is whole, the corner itself is gone
    assert!(covered(&rounded, 10 * MM, MM + 1000));
    assert!(!covered(&rounded, MM + 50_000, MM + 50_000));
    assert!(covered(&rounded, MM + 400_000, MM + 400_000));
}

#[test]
fn the_rounded_fill_stays_inside_the_sharp_one_and_breaks_no_rule() {
    let board = board(false);
    let file = cutout_file();
    let sharp = fill(&board, 0, Some(&file));
    let rounded = fill(&board, MM, Some(&file));
    // the four corners of the zone are rounded; the corners of the hole face the hole and are not
    let removed = (area(&sharp) - area(&rounded)) as f64;
    let exact = 4.0 * corner_area(MM);
    assert!(
        removed > exact && removed < exact * 1.006,
        "{removed} {exact}"
    );
    assert!(vertices(&rounded) > vertices(&sharp));
    // every point of the rounded fill is a point of the sharp fill, to the rounding of a nanometre
    for polygon in rounded.fill.zones.iter().flat_map(|z| z.polygons.iter()) {
        for v in polygon {
            let in_sharp = sharp
                .fill
                .zones
                .iter()
                .flat_map(|z| z.polygons.iter())
                .any(|p| near_inside(p, v[0], v[1], 2.0));
            assert!(in_sharp, "{v:?}");
        }
    }
    // the hole is as wide as it was: its corners are not touched
    assert!(!covered(&rounded, 10 * MM, 10 * MM));
    assert!(covered(&rounded, 8 * MM - 1000, 8 * MM - 1000));
    let findings = check_fill(&board, &rules(), &settings(MM), &rounded.fill).unwrap();
    assert!(!has_errors(&findings), "{findings:?}");
}

#[test]
fn the_same_board_gives_the_same_bytes_with_rounded_corners() {
    let board = board(false);
    let file = cutout_file();
    let first = fill(&board, MM, Some(&file));
    let second = fill(&board, MM, Some(&file));
    assert_eq!(first, second);
}

#[test]
fn a_thermal_relief_gets_a_fillet_at_the_root_of_its_spokes() {
    let board = board(true);
    let strict = |radius: i64| ZoneSettings {
        corner_radius_nm: radius,
        ..ZoneSettings::default()
    };
    let sharp = fill_board(&board, &rules(), &strict(0), None).unwrap();
    let rounded = fill_board(&board, &rules(), &strict(200_000), None).unwrap();
    // rounding the corners that are exposed takes a little copper off the edges of the spokes and of the gaps: the area changes by well under 1 mm2
    let gain = area(&rounded) - area(&sharp);
    assert!(gain.abs() < 1_000_000_000_000, "{gain}");
    // the gap on the diagonal is still a gap, and the spoke along an axis is still whole
    assert!(!covered(&rounded, 10 * MM + 400_000, 10 * MM + 400_000));
    assert!(covered(&rounded, 10 * MM + 600_000, 10 * MM));
    assert!(covered(&rounded, 10 * MM, 10 * MM - 600_000));
    assert!(covered(&rounded, 10 * MM + 900_000, 10 * MM + 900_000));
    // the rounded relief is judged against the board as the sharp one is
    let findings = check_fill(&board, &rules(), &strict(200_000), &rounded.fill).unwrap();
    assert!(!has_errors(&findings), "{findings:?}");
}
