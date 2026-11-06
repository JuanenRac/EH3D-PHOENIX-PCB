// SPDX-License-Identifier: GPL-3.0-or-later
//! What is measured on a board: counts for the expectations, and the facts (counts, positions and
//! names) that a round trip between formats compares with a tolerance in nanometres.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, Side};
use serde_json::Value;

/// The id of the object number `n` of a generated or read board: always the same for a number,
/// different for different numbers.
pub fn id_of(n: u64) -> Id {
    Id::from_parts(1_700_000_000_000, (n % 4096) as u16, n)
        .expect("a fixed clock value is inside the range of the ids")
}

/// A source of ids for the readers: `id_of(first)`, `id_of(first + 1)`, and so on.
pub fn id_source(first: u64) -> impl FnMut() -> Id {
    let mut n = first;
    move || {
        let id = id_of(n);
        n += 1;
        id
    }
}

/// The pieces of the tracks of a board, all of them.
fn pieces(board: &Board) -> usize {
    board.tracks.values().map(|t| t.path.len()).sum()
}

/// The pads of the components as placed (a footprint used twice counts twice).
fn placed_pads(board: &Board) -> usize {
    board
        .components
        .values()
        .map(|c| {
            board
                .footprints
                .get(&c.footprint)
                .map_or(0, |f| f.pads.len())
        })
        .sum()
}

fn copper_layers(board: &Board) -> usize {
    board
        .layers
        .values()
        .filter(|l| l.kind == LayerKind::Copper)
        .count()
}

/// The counts of a board as measures of an outcome.
pub fn board_measures(board: &Board) -> BTreeMap<String, Value> {
    let mut m: BTreeMap<String, Value> = BTreeMap::new();
    let mut put = |key: &str, value: usize| {
        m.insert(key.to_owned(), Value::from(value));
    };
    put("layers", board.layers.len());
    put("copper_layers", copper_layers(board));
    put("footprints", board.footprints.len());
    put(
        "footprint_pads",
        board.footprints.values().map(|f| f.pads.len()).sum(),
    );
    put("components", board.components.len());
    put(
        "bottom_components",
        board
            .components
            .values()
            .filter(|c| c.side == Side::Bottom)
            .count(),
    );
    put("placed_pads", placed_pads(board));
    put("nets", board.nets.len());
    put("pad_nets", board.pad_nets.len());
    put("tracks", board.tracks.len());
    put("track_pieces", pieces(board));
    put("vias", board.vias.len());
    put("zones", board.zones.len());
    put("silkscreen", board.silkscreen.len());
    put("outline_points", board.outline.len());
    put("outline_cutouts", board.outline_cutouts.len());
    put(
        "zone_cutouts",
        board.zones.values().map(|z| z.cutouts.len()).sum::<usize>(),
    );
    m
}

/// The facts of a board that a round trip compares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// Counts by name.
    pub counts: BTreeMap<&'static str, usize>,
    /// Positions in nanometres by name, sorted.
    pub points: BTreeMap<&'static str, Vec<(i64, i64)>>,
    /// Names by kind.
    pub names: BTreeMap<&'static str, BTreeSet<String>>,
}

/// What a round trip can compare, with the kind of each: counts, then positions, then names.
pub const FACT_NAMES: &[&str] = &[
    "components",
    "placed_pads",
    "nets",
    "pad_nets",
    "track_pieces",
    "vias",
    "zones",
    "outline_count",
    "copper_layers",
    "pad_points",
    "via_points",
    "track_points",
    "outline_points",
    "component_points",
    "refdes_values",
    "net_names",
    "pad_net_names",
];

fn xy(p: eh3d_phoenix_model::Point) -> (i64, i64) {
    (p.x_nm.get(), p.y_nm.get())
}

/// The facts of a board.
pub fn facts(board: &Board) -> Facts {
    let mut f = Facts::default();
    f.counts.insert("components", board.components.len());
    f.counts.insert("placed_pads", placed_pads(board));
    f.counts.insert("nets", board.nets.len());
    f.counts.insert("pad_nets", board.pad_nets.len());
    f.counts.insert("track_pieces", pieces(board));
    f.counts.insert("vias", board.vias.len());
    f.counts.insert("zones", board.zones.len());
    f.counts.insert("outline_count", board.outline.len());
    f.counts.insert("copper_layers", copper_layers(board));

    let mut pads = Vec::new();
    for (cid, c) in &board.components {
        if let Some(fp) = board.footprints.get(&c.footprint) {
            for pid in fp.pads.keys() {
                let pad = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                if let Some(p) = pad_position(board, pad) {
                    pads.push(xy(p));
                }
            }
        }
    }
    pads.sort_unstable();
    f.points.insert("pad_points", pads);
    let mut vias: Vec<(i64, i64)> = board.vias.values().map(|v| xy(v.at)).collect();
    vias.sort_unstable();
    f.points.insert("via_points", vias);
    let mut ends: Vec<(i64, i64)> = Vec::new();
    for t in board.tracks.values() {
        for s in &t.path {
            ends.push(xy(s.start()));
            ends.push(xy(s.end()));
        }
    }
    ends.sort_unstable();
    f.points.insert("track_points", ends);
    let mut outline: Vec<(i64, i64)> = board.outline.iter().map(|p| xy(*p)).collect();
    outline.sort_unstable();
    f.points.insert("outline_points", outline);
    let mut at: Vec<(i64, i64)> = board.components.values().map(|c| xy(c.at)).collect();
    at.sort_unstable();
    f.points.insert("component_points", at);

    f.names.insert(
        "refdes_values",
        board
            .components
            .values()
            .map(|c| format!("{}={}", c.refdes, c.value))
            .collect(),
    );
    f.names.insert(
        "net_names",
        board.nets.values().map(|n| n.name.clone()).collect(),
    );
    let mut named = BTreeSet::new();
    for pn in &board.pad_nets {
        let component = board.components.get(&pn.component);
        let pad = component
            .and_then(|c| board.footprints.get(&c.footprint))
            .and_then(|fp| fp.pads.get(&pn.pad));
        let net = board.nets.get(&pn.net);
        if let (Some(c), Some(p), Some(n)) = (component, pad, net) {
            named.insert(format!("{}.{}={}", c.refdes, p.number, n.name));
        }
    }
    f.names.insert("pad_net_names", named);
    f
}

/// Compares two sets of facts on the named categories. A position counts as the same when both
/// coordinates are within `tolerance_nm`. Each difference is a line of text; none means the
/// facts agree.
pub fn compare(a: &Facts, b: &Facts, tolerance_nm: i64, categories: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for name in categories {
        let name = name.as_str();
        if let (Some(x), Some(y)) = (a.counts.get(name), b.counts.get(name)) {
            if x != y {
                out.push(format!("{name}: {x} before, {y} after"));
            }
        } else if let (Some(x), Some(y)) = (a.points.get(name), b.points.get(name)) {
            if x.len() != y.len() {
                out.push(format!("{name}: {} before, {} after", x.len(), y.len()));
                continue;
            }
            for (p, q) in x.iter().zip(y.iter()) {
                if (p.0 - q.0).abs() > tolerance_nm || (p.1 - q.1).abs() > tolerance_nm {
                    out.push(format!(
                        "{name}: ({}, {}) before, ({}, {}) after",
                        p.0, p.1, q.0, q.1
                    ));
                    break;
                }
            }
        } else if let (Some(x), Some(y)) = (a.names.get(name), b.names.get(name)) {
            for lost in x.difference(y) {
                out.push(format!("{name}: {lost:?} is not there after"));
            }
            for new in y.difference(x) {
                out.push(format!("{name}: {new:?} is new after"));
            }
        } else {
            out.push(format!("{name}: not a fact that can be compared"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Component, Footprint, Layer, Net, Pad, PadNet, PadShape, Side};
    use eh3d_phoenix_model::{Nm, Point, Udeg};

    fn p(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// Two layers, one footprint with two pads, two components that use it, one net on three pads.
    fn small() -> Board {
        let (top, bottom, net, fp, p1, p2) =
            (id_of(1), id_of(2), id_of(3), id_of(4), id_of(5), id_of(6));
        let mut b = Board::default();
        for (id, name) in [(top, "Top"), (bottom, "Bottom")] {
            b.layers.insert(
                id,
                Layer {
                    name: name.into(),
                    kind: LayerKind::Copper,
                    side: None,
                },
            );
        }
        b.nets.insert(net, Net { name: "GND".into() });
        let mut pads = BTreeMap::new();
        for (pid, number, x) in [(p1, "1", -1_000_000), (p2, "2", 1_000_000)] {
            pads.insert(
                pid,
                Pad {
                    number: number.into(),
                    shape: PadShape::Circle {
                        diameter: Nm::new(1_000_000).unwrap(),
                    },
                    at: p(x, 0),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![top],
                    roundness: 0,
                    plated: true,
                },
            );
        }
        b.footprints.insert(
            fp,
            Footprint {
                name: "F".into(),
                pads,
            },
        );
        for (n, refdes, x) in [(7, "R1", 10_000_000), (8, "R2", 20_000_000)] {
            b.components.insert(
                id_of(n),
                Component {
                    refdes: refdes.into(),
                    value: "1k".into(),
                    symbol: None,
                    footprint: fp,
                    at: p(x, 5_000_000),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            );
        }
        for (c, pad) in [(id_of(7), p1), (id_of(7), p2), (id_of(8), p1)] {
            b.pad_nets.push(PadNet {
                component: c,
                pad,
                net,
            });
        }
        b.pad_nets.sort_by_key(|x| x.pad_ref());
        b
    }

    #[test]
    fn the_ids_are_the_same_for_a_number_and_different_for_two_numbers() {
        assert_eq!(id_of(9), id_of(9));
        assert_ne!(id_of(9), id_of(10));
        assert_ne!(id_of(1), id_of(4097));
        let mut next = id_source(100);
        assert_eq!(next(), id_of(100));
        assert_eq!(next(), id_of(101));
    }

    #[test]
    fn the_measures_count_what_the_board_holds() {
        let b = small();
        assert!(b.is_consistent());
        let m = board_measures(&b);
        assert_eq!(m["layers"], Value::from(2));
        assert_eq!(m["copper_layers"], Value::from(2));
        assert_eq!(m["footprints"], Value::from(1));
        assert_eq!(m["footprint_pads"], Value::from(2));
        assert_eq!(m["components"], Value::from(2));
        assert_eq!(m["bottom_components"], Value::from(0));
        assert_eq!(m["placed_pads"], Value::from(4));
        assert_eq!(m["nets"], Value::from(1));
        assert_eq!(m["pad_nets"], Value::from(3));
        assert_eq!(m["tracks"], Value::from(0));
        assert_eq!(m["outline_points"], Value::from(0));
        assert_eq!(m["outline_cutouts"], Value::from(0));
        assert_eq!(m["zone_cutouts"], Value::from(0));
    }

    #[test]
    fn the_facts_hold_the_places_of_the_pads_and_the_names() {
        let f = facts(&small());
        assert_eq!(f.counts["placed_pads"], 4);
        assert_eq!(
            f.points["pad_points"],
            vec![
                (9_000_000, 5_000_000),
                (11_000_000, 5_000_000),
                (19_000_000, 5_000_000),
                (21_000_000, 5_000_000)
            ]
        );
        assert_eq!(
            f.names["pad_net_names"].iter().cloned().collect::<Vec<_>>(),
            vec!["R1.1=GND", "R1.2=GND", "R2.1=GND"]
        );
        assert!(f.names["refdes_values"].contains("R2=1k"));
        assert!(f.names["net_names"].contains("GND"));
        for name in FACT_NAMES {
            let known = f.counts.contains_key(name)
                || f.points.contains_key(name)
                || f.names.contains_key(name);
            assert!(known, "{name}");
        }
    }

    #[test]
    fn a_shift_inside_the_tolerance_is_the_same_and_outside_it_is_not() {
        let a = facts(&small());
        let mut moved = small();
        let first = *moved.components.keys().next().unwrap();
        moved.components.get_mut(&first).unwrap().at = p(10_000_005, 5_000_000);
        let b = facts(&moved);
        let all: Vec<String> = FACT_NAMES.iter().map(|s| (*s).to_owned()).collect();
        assert!(compare(&a, &b, 10, &all).is_empty());
        let outside = compare(&a, &b, 4, &all);
        assert!(
            outside.iter().any(|l| l.starts_with("pad_points:")),
            "{outside:?}"
        );
        assert!(outside.iter().any(|l| l.starts_with("component_points:")));
    }

    #[test]
    fn a_missing_part_and_a_renamed_net_are_found() {
        let a = facts(&small());
        let mut other = small();
        let gone = *other.components.keys().next().unwrap();
        other.components.remove(&gone);
        other.pad_nets.retain(|x| x.component != gone);
        for net in other.nets.values_mut() {
            net.name = "VCC".into();
        }
        let b = facts(&other);
        let lines = compare(
            &a,
            &b,
            0,
            &[
                "components".to_owned(),
                "net_names".to_owned(),
                "pad_net_names".to_owned(),
                "nonsense".to_owned(),
            ],
        );
        assert!(
            lines.contains(&"components: 2 before, 1 after".to_owned()),
            "{lines:?}"
        );
        assert!(lines
            .iter()
            .any(|l| l.contains("\"GND\" is not there after")));
        assert!(lines.iter().any(|l| l.contains("\"VCC\" is new after")));
        assert!(lines.contains(&"nonsense: not a fact that can be compared".to_owned()));
    }
}
