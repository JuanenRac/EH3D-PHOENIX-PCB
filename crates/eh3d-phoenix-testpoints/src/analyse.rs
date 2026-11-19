// SPDX-License-Identifier: GPL-3.0-or-later
//! The test points of a board and the nets they reach.
//!
//! A **test point** is a pad that the board marks or a pad of a part that the settings recognise: a part whose reference starts
//! with one of `testpoint_prefixes` (and goes on with a digit, a mark or nothing, so `TP12` and
//! `TP_GND` are test points and `TPS5` is not) or whose footprint name holds one of
//! `testpoint_footprint_words`. It is **probed** when it is on the side the settings name.
//!
//! A **net is covered** when it has at least `required_per_net` probed test points. Nets with
//! fewer than `min_pads_for_test` pads and the nets named in `ignore_nets` are not asked. A net
//! whose test points are all on the other side is reported apart from one with none.
//!
//! Each test point is also judged: its smallest side against `min_pad_nm`, its distance to the
//! edge of the board against `min_edge_nm`, and its distance to the other test points on the same
//! side against `min_spacing_nm`. A test point on no net is reported.
//!
//! A pad the designer marked as a test point (`Board::test_points`) is one too, whatever its part is
//! called; `use_marks` and `use_names` of the settings switch each way of recognising them on or off.
//!
//! The report is the same every time: the test points come in the order of their reference and
//! pad, the nets in the order of their names, and every number is an integer.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, Pad, PadRef, PadShape, Side};
use eh3d_phoenix_model::Point;
use serde::Serialize;

use crate::codes::{
    Finding, Severity, TestPointError, LIMIT_EXCEEDED, NEAR_EDGE, NET_NO_TESTPOINT, NET_WRONG_SIDE,
    NONE_RECOGNISED, NO_MASK_OPENING, NO_NET, PAD_SMALL, PART_TOO_CLOSE, TOO_CLOSE,
};
use crate::settings::{ProbeSide, TestPointSettings};

/// The name of the format of the JSON report.
pub const FORMAT: &str = "phoenix-testpoints/1";

/// A pad that is a test point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TestPoint {
    /// The reference of its part.
    pub refdes: String,
    /// The number of the pad.
    pub pad: String,
    /// The name of its net, when it has one.
    pub net: Option<String>,
    /// The side of the board it is on: `top` or `bottom`.
    pub side: &'static str,
    /// Where its centre is, in nanometres.
    pub x_nm: i64,
    /// Where its centre is, in nanometres.
    pub y_nm: i64,
    /// Its smallest side, in nanometres.
    pub size_nm: i64,
    /// True when it is on the side that is probed.
    pub probed: bool,
}

/// What the test points do for one net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetCoverage {
    /// The name of the net.
    pub net: String,
    /// The pads on the net.
    pub pads: usize,
    /// Its test points, as `reference:pad`.
    pub test_points: Vec<String>,
    /// How many of them are on the probed side.
    pub probed: usize,
    /// True when `probed` reaches the number the settings ask for.
    pub covered: bool,
}

/// The test points of a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TestPointReport {
    /// The name of the format.
    pub format: &'static str,
    /// The name of the board.
    pub name: String,
    /// The side that is probed: `both`, `top` or `bottom`.
    pub probe_side: &'static str,
    /// The test points.
    pub test_points: Vec<TestPoint>,
    /// The nets that were asked to have a test point.
    pub nets: Vec<NetCoverage>,
    /// How many nets were asked.
    pub nets_asked: usize,
    /// How many of them are covered.
    pub nets_covered: usize,
    /// The covered share of the nets asked, in hundredths of a percent (100 % when none was asked).
    pub coverage_x100: i64,
    /// What is wrong, errors first when listed.
    pub findings: Vec<Finding>,
}

impl TestPointReport {
    /// True when a finding is an error.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// A test point with the id of its net, for the analysis only.
struct Raw {
    point: TestPoint,
    /// The part the test point is a pad of.
    component: Id,
    net_id: Option<Id>,
    /// Whether the pad lists a solder mask layer that opens it on its side.
    mask_open: bool,
}

fn side_word(side: Side) -> &'static str {
    match side {
        Side::Top => "top",
        Side::Bottom => "bottom",
    }
}

fn is_probed(side: Side, probe: ProbeSide) -> bool {
    matches!(
        (probe, side),
        (ProbeSide::Both, _) | (ProbeSide::Top, Side::Top) | (ProbeSide::Bottom, Side::Bottom)
    )
}

/// True when the reference starts with a prefix and the next character is not a letter.
fn has_prefix(refdes: &str, prefixes: &[String]) -> bool {
    let up = refdes.to_ascii_uppercase();
    prefixes.iter().any(|p| {
        let p = p.to_ascii_uppercase();
        !p.is_empty()
            && up
                .strip_prefix(&p)
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphabetic()))
    })
}

fn has_word(footprint: &str, words: &[String]) -> bool {
    let low = footprint.to_ascii_lowercase();
    words
        .iter()
        .any(|w| !w.is_empty() && low.contains(&w.to_ascii_lowercase()))
}

/// True when a layer of the pad is a solder mask layer of the side of the part (or of no side);
/// a through-hole pad shows on both sides, so a mask layer of either one will do.
fn has_mask_opening(board: &Board, pad: &Pad, side: Side) -> bool {
    pad.layers.iter().any(|id| {
        board.layers.get(id).is_some_and(|layer| {
            layer.kind == LayerKind::SolderMask
                && (pad.drill.is_some() || layer.side.is_none() || layer.side == Some(side))
        })
    })
}

fn smallest_side(shape: &PadShape) -> i64 {
    match *shape {
        PadShape::Circle { diameter } => diameter.get(),
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            width.get().min(height.get())
        }
    }
}

fn point_segment_distance(p: (i64, i64), a: (i64, i64), b: (i64, i64)) -> f64 {
    let (px, py) = (p.0 as f64, p.1 as f64);
    let (ax, ay) = (a.0 as f64, a.1 as f64);
    let (bx, by) = (b.0 as f64, b.1 as f64);
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// The distance from a point to the nearest edge of the outline or of a cutout of it, in
/// nanometres; `None` when the board has no outline.
fn edge_distance(board: &Board, p: Point) -> Option<f64> {
    let mut best: Option<f64> = None;
    let polygons = std::iter::once(&board.outline).chain(board.outline_cutouts.iter());
    for polygon in polygons {
        if polygon.len() < 3 {
            continue;
        }
        for (i, a) in polygon.iter().enumerate() {
            let b = &polygon[(i + 1) % polygon.len()];
            let d = point_segment_distance(
                (p.x_nm.get(), p.y_nm.get()),
                (a.x_nm.get(), a.y_nm.get()),
                (b.x_nm.get(), b.y_nm.get()),
            );
            best = Some(best.map_or(d, |current| current.min(d)));
        }
    }
    best
}

fn millimetres(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
}

fn find_test_points(board: &Board, settings: &TestPointSettings) -> Vec<Raw> {
    let mut found = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        let by_name = settings.use_names
            && (has_prefix(&component.refdes, &settings.prefixes)
                || has_word(&footprint.name, &settings.footprint_words));
        if !by_name && !(settings.use_marks && !board.test_points.is_empty()) {
            continue;
        }
        for (pad_id, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            if !by_name && !board.is_test_point(pad_ref) {
                continue;
            }
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let net_id = board.net_of(pad_ref);
            found.push(Raw {
                component: *component_id,
                point: TestPoint {
                    refdes: component.refdes.clone(),
                    pad: pad.number.clone(),
                    net: net_id
                        .and_then(|n| board.nets.get(&n))
                        .map(|n| n.name.clone()),
                    side: side_word(component.side),
                    x_nm: at.x_nm.get(),
                    y_nm: at.y_nm.get(),
                    size_nm: smallest_side(&pad.shape),
                    probed: is_probed(component.side, settings.probe_side),
                },
                net_id,
                mask_open: has_mask_opening(board, pad, component.side),
            });
        }
    }
    found.sort_by(|a, b| {
        (a.point.refdes.as_str(), a.point.pad.as_str())
            .cmp(&(b.point.refdes.as_str(), b.point.pad.as_str()))
    });
    found
}

/// The test points that are too close to each other, as pairs of positions in the sorted list.
fn close_pairs(points: &[TestPoint], spacing_nm: i64) -> Vec<(usize, usize)> {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by_key(|&i| (points[i].x_nm, i));
    let limit = i128::from(spacing_nm) * i128::from(spacing_nm);
    let mut pairs = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            let dx = i128::from(points[j].x_nm) - i128::from(points[i].x_nm);
            if dx >= i128::from(spacing_nm) {
                break;
            }
            if points[i].side != points[j].side {
                continue;
            }
            let dy = i128::from(points[j].y_nm) - i128::from(points[i].y_nm);
            if dx * dx + dy * dy < limit {
                pairs.push((i.min(j), i.max(j)));
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

/// The centres of the pads of the parts that hold no test point, with the part and the pad, from
/// the least x.
fn neighbour_pads(board: &Board, test_parts: &BTreeSet<Id>) -> Vec<(i64, i64, Id, Id)> {
    let mut list = Vec::new();
    for (component_id, component) in &board.components {
        if test_parts.contains(component_id) {
            continue;
        }
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for pad_id in footprint.pads.keys() {
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            if let Some(at) = pad_position(board, pad_ref) {
                list.push((at.x_nm.get(), at.y_nm.get(), *component_id, *pad_id));
            }
        }
    }
    list.sort_by_key(|n| (n.0, n.1));
    list
}

/// For one test point, the parts on its side that have a pad closer than `limit_nm`, each with its
/// nearest pad: the reference, the number of the pad and the distance, by reference.
fn parts_near(
    board: &Board,
    point: &TestPoint,
    neighbours: &[(i64, i64, Id, Id)],
    limit_nm: i64,
) -> Vec<(String, String, i64)> {
    let low = point.x_nm.saturating_sub(limit_nm);
    let start = neighbours.partition_point(|n| n.0 < low);
    let limit2 = i128::from(limit_nm) * i128::from(limit_nm);
    let mut nearest: BTreeMap<Id, (i128, Id)> = BTreeMap::new();
    for &(x, y, component_id, pad_id) in &neighbours[start..] {
        if x > point.x_nm.saturating_add(limit_nm) {
            break;
        }
        let Some(component) = board.components.get(&component_id) else {
            continue;
        };
        if side_word(component.side) != point.side {
            continue;
        }
        let dx = i128::from(x) - i128::from(point.x_nm);
        let dy = i128::from(y) - i128::from(point.y_nm);
        let d2 = dx * dx + dy * dy;
        if d2 >= limit2 {
            continue;
        }
        let entry = nearest.entry(component_id).or_insert((d2, pad_id));
        if (d2, pad_id) < *entry {
            *entry = (d2, pad_id);
        }
    }
    let mut found: Vec<(String, String, i64)> = nearest
        .into_iter()
        .filter_map(|(component_id, (d2, pad_id))| {
            let component = board.components.get(&component_id)?;
            let pad = board
                .footprints
                .get(&component.footprint)?
                .pads
                .get(&pad_id)?;
            Some((
                component.refdes.clone(),
                pad.number.clone(),
                (d2 as f64).sqrt().round() as i64,
            ))
        })
        .collect();
    found.sort();
    found
}

/// The test points of a board and the nets they reach.
pub fn analyse(
    board: &Board,
    settings: &TestPointSettings,
) -> Result<TestPointReport, TestPointError> {
    let items = board
        .components
        .len()
        .saturating_add(board.pad_nets.len())
        .saturating_add(board.nets.len());
    if items > settings.max_items {
        return Err(TestPointError::new(
            LIMIT_EXCEEDED,
            format!("{items} objects, the limit is {}", settings.max_items),
        ));
    }
    let raws = find_test_points(board, settings);
    let mut findings = Vec::new();

    // the test points one by one
    for raw in &raws {
        let p = &raw.point;
        let name = format!("{}:{}", p.refdes, p.pad);
        if p.net.is_none() {
            findings.push(Finding::new(NO_NET, Severity::Warning, Some(&name), ""));
        }
        if settings.require_mask_opening && !raw.mask_open {
            findings.push(Finding::new(
                NO_MASK_OPENING,
                Severity::Warning,
                Some(&name),
                format!(
                    "no solder mask layer of the {} side among the layers of the pad",
                    p.side
                ),
            ));
        }
        if settings.min_pad_nm > 0 && p.size_nm < settings.min_pad_nm {
            findings.push(Finding::new(
                PAD_SMALL,
                Severity::Warning,
                Some(&name),
                format!(
                    "{} mm, the smallest is {} mm",
                    millimetres(p.size_nm),
                    millimetres(settings.min_pad_nm)
                ),
            ));
        }
        if settings.min_edge_nm > 0 {
            if let Ok(at) = Point::from_nm(p.x_nm, p.y_nm) {
                if let Some(d) = edge_distance(board, at) {
                    if d < settings.min_edge_nm as f64 {
                        findings.push(Finding::new(
                            NEAR_EDGE,
                            Severity::Warning,
                            Some(&name),
                            format!(
                                "{} mm from the edge, the least is {} mm",
                                millimetres(d.round() as i64),
                                millimetres(settings.min_edge_nm)
                            ),
                        ));
                    }
                }
            }
        }
    }
    if settings.min_spacing_nm > 0 {
        let points: Vec<TestPoint> = raws.iter().map(|r| r.point.clone()).collect();
        for (i, j) in close_pairs(&points, settings.min_spacing_nm) {
            let (a, b) = (&points[i], &points[j]);
            let dx = i128::from(a.x_nm) - i128::from(b.x_nm);
            let dy = i128::from(a.y_nm) - i128::from(b.y_nm);
            let distance = ((dx * dx + dy * dy) as f64).sqrt().round() as i64;
            findings.push(Finding::new(
                TOO_CLOSE,
                Severity::Warning,
                Some(&format!("{}:{} / {}:{}", a.refdes, a.pad, b.refdes, b.pad)),
                format!(
                    "{} mm apart, the least is {} mm",
                    millimetres(distance),
                    millimetres(settings.min_spacing_nm)
                ),
            ));
        }
    }
    if settings.min_part_clearance_nm > 0 && !raws.is_empty() {
        let test_parts: BTreeSet<Id> = raws.iter().map(|r| r.component).collect();
        let neighbours = neighbour_pads(board, &test_parts);
        for raw in &raws {
            let p = &raw.point;
            let name = format!("{}:{}", p.refdes, p.pad);
            for (refdes, pad, distance) in
                parts_near(board, p, &neighbours, settings.min_part_clearance_nm)
            {
                findings.push(Finding::new(
                    PART_TOO_CLOSE,
                    Severity::Warning,
                    Some(&name),
                    format!(
                        "{refdes}.{pad} is {} mm from the test point, the least is {} mm",
                        millimetres(distance),
                        millimetres(settings.min_part_clearance_nm)
                    ),
                ));
            }
        }
    }
    if raws.is_empty() {
        findings.push(Finding::new(
            NONE_RECOGNISED,
            Severity::Warning,
            None,
            "check testpoint_prefixes and testpoint_footprint_words",
        ));
    }

    // the nets
    let mut pads_of: BTreeMap<Id, usize> = BTreeMap::new();
    for pad_net in &board.pad_nets {
        *pads_of.entry(pad_net.net).or_insert(0) += 1;
    }
    let mut nets: Vec<NetCoverage> = Vec::new();
    for (net_id, net) in &board.nets {
        let count = pads_of.get(net_id).copied().unwrap_or(0);
        if count < settings.min_pads_for_test || settings.ignores(&net.name) {
            continue;
        }
        let mine: Vec<&Raw> = raws.iter().filter(|r| r.net_id == Some(*net_id)).collect();
        let probed = mine.iter().filter(|r| r.point.probed).count();
        let covered = probed >= settings.required_per_net;
        if !covered && !raws.is_empty() {
            let (code, detail) = if probed == 0 && !mine.is_empty() {
                (
                    NET_WRONG_SIDE,
                    format!(
                        "{} test points, none on the {} side",
                        mine.len(),
                        settings.probe_side.as_str()
                    ),
                )
            } else {
                (
                    NET_NO_TESTPOINT,
                    format!("{probed} of {} probed", settings.required_per_net),
                )
            };
            findings.push(Finding::new(
                code,
                settings.uncovered_severity,
                Some(&net.name),
                detail,
            ));
        }
        nets.push(NetCoverage {
            net: net.name.clone(),
            pads: count,
            test_points: mine
                .iter()
                .map(|r| format!("{}:{}", r.point.refdes, r.point.pad))
                .collect(),
            probed,
            covered,
        });
    }
    nets.sort_by(|a, b| a.net.cmp(&b.net));
    let nets_asked = nets.len();
    let nets_covered = nets.iter().filter(|n| n.covered).count();
    let coverage_x100 = if nets_asked == 0 {
        10_000
    } else {
        i64::try_from(nets_covered as u128 * 10_000 / nets_asked as u128).unwrap_or(10_000)
    };

    Ok(TestPointReport {
        format: FORMAT,
        name: board.name.clone(),
        probe_side: settings.probe_side.as_str(),
        test_points: raws.into_iter().map(|r| r.point).collect(),
        nets,
        nets_asked,
        nets_covered,
        coverage_x100,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_is_a_test_point_when_a_prefix_is_followed_by_a_digit_or_a_mark() {
        let prefixes = vec!["TP".to_owned()];
        assert!(has_prefix("TP1", &prefixes));
        assert!(has_prefix("tp12", &prefixes));
        assert!(has_prefix("TP_GND", &prefixes));
        assert!(has_prefix("TP", &prefixes));
        assert!(!has_prefix("TPS5", &prefixes));
        assert!(!has_prefix("R1", &prefixes));
        assert!(!has_prefix("TP1", &[String::new()]));
    }

    #[test]
    fn a_footprint_is_a_test_point_when_it_holds_a_word_in_any_case() {
        let words = vec!["testpoint".to_owned()];
        assert!(has_word("TestPoint_Pad_D1.5mm", &words));
        assert!(!has_word("R_0603", &words));
        assert!(!has_word("anything", &[String::new()]));
    }

    #[test]
    fn the_distance_to_a_segment_is_to_its_nearest_point() {
        assert_eq!(point_segment_distance((5, 3), (0, 0), (10, 0)), 3.0);
        assert_eq!(point_segment_distance((13, 4), (0, 0), (10, 0)), 5.0);
        assert_eq!(point_segment_distance((3, 4), (0, 0), (0, 0)), 5.0);
    }

    #[test]
    fn close_pairs_are_found_on_the_same_side_only() {
        let at = |x: i64, y: i64, side: &'static str| TestPoint {
            refdes: "TP".into(),
            pad: "1".into(),
            net: None,
            side,
            x_nm: x,
            y_nm: y,
            size_nm: 0,
            probed: true,
        };
        let points = vec![
            at(0, 0, "top"),
            at(1000, 0, "top"),
            at(500, 0, "bottom"),
            at(5000, 0, "top"),
        ];
        assert_eq!(close_pairs(&points, 2000), vec![(0, 1)]);
        assert!(close_pairs(&points, 400).is_empty());
        // just under the spacing counts, equal to it does not
        assert_eq!(close_pairs(&points, 1001), vec![(0, 1)]);
        assert!(close_pairs(&points, 1000).is_empty());
    }
}
