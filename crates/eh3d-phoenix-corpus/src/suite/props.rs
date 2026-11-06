// SPDX-License-Identifier: GPL-3.0-or-later
//! Properties of the readers, the writers and the operations, tried on small boards that a
//! generator of our own makes from a seed (xorshift, no dependency): undoing every change gives
//! the same bytes, a rejected change changes nothing, writing, reading and writing again is
//! idempotent, and the rule check does not change after a round trip.
//!
//! The same seed always gives the same board, so a failure names the seed and can be repeated.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::rules::RuleSet;
use eh3d_phoenix_board::{
    Board, Component, Footprint, History, Id, Layer, LayerKind, Net, Operation, Pad, PadNet,
    PadRef, PadShape, Segment, Side, Track, Via,
};
use eh3d_phoenix_eagle_board::{read_board, signature, write_board};
use eh3d_phoenix_kicad::{read_pcb, write_pcb, KicadSettings, WriteSettings};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_route::dsn;
use eh3d_phoenix_route::live::RuleContext;
use eh3d_phoenix_route::settings::RouteSettings;

use super::facts::{compare, facts, id_of, id_source};
use super::rng::XorShift;
use super::settings::CorpusSettings;

/// The properties, by name.
pub const PROPERTIES: &[&str] = &[
    "undo",
    "rejection",
    "json",
    "eagle",
    "kicad",
    "drc-eagle",
    "drc-kicad",
    "dsn",
];

/// What EAGLE and KiCad both keep of a board: the facts that a round trip must not change.
const PRESERVED: &[&str] = &[
    "components",
    "placed_pads",
    "nets",
    "pad_nets",
    "track_pieces",
    "vias",
    "outline_count",
    "pad_points",
    "via_points",
    "track_points",
    "outline_points",
    "refdes_values",
    "net_names",
    "pad_net_names",
];

/// The rules that the property of the rule check uses: item by item, so that they depend only on
/// what the formats keep.
const RULES: &str =
    "min_track_width = 0.28\nmin_via_diameter = 0.7\nmin_drill = 0.4\nmin_annular_ring = 0.1\n";

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).expect("a point inside the range")
}

fn nm(v: i64) -> Nm {
    Nm::new(v).expect("a length inside the range")
}

fn turn(quarters: u64) -> Udeg {
    Udeg::new(quarters as i64 * 90_000_000).expect("an angle inside the range")
}

fn ghost() -> Id {
    id_of(9_000_000)
}

fn pick<K: Copy + Ord, V>(map: &BTreeMap<K, V>, rng: &mut XorShift) -> Option<K> {
    if map.is_empty() {
        None
    } else {
        map.keys().nth(rng.index(map.len())).copied()
    }
}

/// A small board made from a seed: two copper layers and an outline, one or three nets, two
/// footprints (a chip with two surface pads and a header with two drilled pads), two to
/// `max_components` components turned by right angles, nets on most pads, up to three tracks on
/// rows of their own, up to two vias. Only the nets that something uses are kept.
pub fn random_board(seed: u64, max_components: u32) -> Board {
    let mut rng = XorShift::new(seed);
    let (top, bottom, edge) = (id_of(1), id_of(2), id_of(3));
    let mut b = Board {
        name: format!("Random {seed}"),
        outline: vec![
            pt(0, 0),
            pt(60_000_000, 0),
            pt(60_000_000, 40_000_000),
            pt(0, 40_000_000),
        ],
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    for (id, name, kind) in [
        (top, "Top", LayerKind::Copper),
        (bottom, "Bottom", LayerKind::Copper),
        (edge, "Outline", LayerKind::Outline),
    ] {
        b.layers.insert(
            id,
            Layer {
                name: name.into(),
                kind,
                side: None,
            },
        );
    }
    let mut nets: Vec<Id> = Vec::new();
    for i in 0..(1 + rng.below(3)) {
        let id = id_of(100 + i);
        b.nets.insert(
            id,
            Net {
                name: format!("N{}", i + 1),
            },
        );
        nets.push(id);
    }
    let (fp_chip, fp_header) = (id_of(10), id_of(11));
    let mut chip = BTreeMap::new();
    for (n, number, x) in [(12, "1", -800_000), (13, "2", 800_000)] {
        chip.insert(
            id_of(n),
            Pad {
                number: number.into(),
                shape: PadShape::Rect {
                    width: nm(900_000),
                    height: nm(1_000_000),
                },
                at: pt(x, 0),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![top],
            },
        );
    }
    b.footprints.insert(
        fp_chip,
        Footprint {
            name: "CHIP".into(),
            pads: chip,
        },
    );
    let mut header = BTreeMap::new();
    for (n, number, x) in [(14, "1", 0), (15, "2", 2_540_000)] {
        header.insert(
            id_of(n),
            Pad {
                number: number.into(),
                shape: PadShape::Circle {
                    diameter: nm(1_600_000),
                },
                at: pt(x, 0),
                rotation: Udeg::ZERO,
                drill: Some(nm(800_000)),
                layers: vec![top, bottom],
            },
        );
    }
    b.footprints.insert(
        fp_header,
        Footprint {
            name: "HEADER".into(),
            pads: header,
        },
    );
    let spare = u64::from(max_components.saturating_sub(1)).max(1);
    let count = 2 + rng.below(spare) as usize;
    for i in 0..count {
        let (footprint, prefix) = if rng.chance(1, 2) {
            (fp_chip, "R")
        } else {
            (fp_header, "J")
        };
        let cid = id_of(200 + i as u64);
        let x = 6_000_000 + (i as i64 % 5) * 10_000_000;
        let y = 12_000_000 + (i as i64 / 5) * 12_000_000;
        b.components.insert(
            cid,
            Component {
                refdes: format!("{prefix}{}", i + 1),
                value: format!("{}k", 1 + rng.below(99)),
                symbol: None,
                footprint,
                at: pt(x, y),
                rotation: turn(rng.below(4)),
                side: Side::Top,
            },
        );
        let pads: Vec<Id> = b.footprints[&footprint].pads.keys().copied().collect();
        for pad in pads {
            if rng.chance(2, 3) {
                b.pad_nets.push(PadNet {
                    component: cid,
                    pad,
                    net: nets[rng.index(nets.len())],
                });
            }
        }
    }
    if b.pad_nets.is_empty() {
        // a net is always used, so that no net is a name that only the model holds
        if let Some((cid, c)) = b.components.iter().next() {
            if let Some(pad) = b.footprints[&c.footprint].pads.keys().next() {
                b.pad_nets.push(PadNet {
                    component: *cid,
                    pad: *pad,
                    net: nets[0],
                });
            }
        }
    }
    b.pad_nets.sort_by_key(|p| p.pad_ref());
    for j in 0..rng.below(4) {
        let y = 2_000_000 + j as i64 * 3_000_000;
        let x1 = 2_000_000 + rng.below(10) as i64 * 1_000_000;
        let x2 = x1 + (2 + rng.below(8) as i64) * 1_000_000;
        b.tracks.insert(
            id_of(300 + j),
            Track {
                net: nets[rng.index(nets.len())],
                layer: if rng.chance(1, 2) { top } else { bottom },
                width: nm(if rng.chance(1, 2) { 250_000 } else { 300_000 }),
                path: vec![Segment::Line {
                    start: pt(x1, y),
                    end: pt(x2, y),
                }],
            },
        );
    }
    for j in 0..rng.below(3) {
        b.vias.insert(
            id_of(400 + j),
            Via {
                net: nets[rng.index(nets.len())],
                at: pt(
                    3_000_000 + rng.below(50) as i64 * 1_000_000,
                    30_000_000 + rng.below(8) as i64 * 1_000_000,
                ),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
    }
    let used: BTreeSet<Id> = b
        .pad_nets
        .iter()
        .map(|p| p.net)
        .chain(b.tracks.values().map(|t| t.net))
        .chain(b.vias.values().map(|v| v.net))
        .collect();
    b.nets.retain(|id, _| used.contains(id));
    b
}

fn bytes_of(board: &Board) -> Result<Vec<u8>, String> {
    board.to_bytes().map_err(|e| format!("{e:?}"))
}

/// A change to the board that may or may not apply; the ones that cannot apply are part of the
/// point.
fn random_operation(board: &Board, rng: &mut XorShift, step: u32) -> Operation {
    let n = u64::from(step);
    let component = pick(&board.components, rng).unwrap_or_else(ghost);
    match rng.below(11) {
        0 => Operation::AddNet {
            id: id_of(1000 + n),
            name: format!("X{step}"),
        },
        1 => Operation::RenameNet {
            id: pick(&board.nets, rng).unwrap_or_else(ghost),
            name: format!("Y{step}"),
        },
        2 => Operation::RemoveNet {
            id: pick(&board.nets, rng).unwrap_or_else(ghost),
        },
        3 => Operation::MoveComponent {
            id: component,
            at: pt(
                2_000_000 + rng.below(56) as i64 * 1_000_000,
                2_000_000 + rng.below(36) as i64 * 1_000_000,
            ),
            rotation: turn(rng.below(4)),
        },
        4 => {
            let pads: Vec<Id> = board
                .components
                .get(&component)
                .and_then(|c| board.footprints.get(&c.footprint))
                .map(|f| f.pads.keys().copied().collect())
                .unwrap_or_default();
            let pad = if pads.is_empty() {
                ghost()
            } else {
                pads[rng.index(pads.len())]
            };
            let net = if rng.chance(1, 3) {
                None
            } else {
                pick(&board.nets, rng)
            };
            Operation::SetPadNet {
                pad: PadRef { component, pad },
                net,
            }
        }
        5 => {
            let y = 1_000_000 + rng.below(30) as i64 * 1_000_000;
            Operation::AddTrack {
                id: id_of(2000 + n),
                track: Track {
                    net: pick(&board.nets, rng).unwrap_or_else(ghost),
                    layer: id_of(1),
                    width: nm(250_000),
                    path: vec![Segment::Line {
                        start: pt(1_000_000, y),
                        end: pt(5_000_000, y),
                    }],
                },
            }
        }
        6 => Operation::RemoveTrack {
            id: pick(&board.tracks, rng).unwrap_or_else(ghost),
        },
        7 => Operation::AddVia {
            id: id_of(3000 + n),
            via: Via {
                net: pick(&board.nets, rng).unwrap_or_else(ghost),
                at: pt(
                    2_000_000 + rng.below(56) as i64 * 1_000_000,
                    2_000_000 + rng.below(36) as i64 * 1_000_000,
                ),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: id_of(1),
                to_layer: id_of(2),
            },
        },
        8 => Operation::RemoveVia {
            id: pick(&board.vias, rng).unwrap_or_else(ghost),
        },
        9 => Operation::RemoveComponent { id: component },
        _ => {
            let (refdes, footprint) = board.components.get(&component).map_or_else(
                || ("X".to_owned(), ghost()),
                |c| (c.refdes.clone(), c.footprint),
            );
            Operation::SetComponentFields {
                id: component,
                refdes,
                value: format!("{step}u"),
                footprint,
            }
        }
    }
}

/// Every change that is applied can be undone to the same bytes, and done again to the bytes it
/// had; a change that is refused leaves the bytes as they were.
fn undo(seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    let mut board = random_board(seed, settings.property_max_components);
    let start = bytes_of(&board)?;
    let mut rng = XorShift::new(seed ^ 0x5DEE_CE66_D1CE_4E5B);
    let mut history = History::new();
    let mut applied = 0usize;
    for step in 0..settings.property_max_steps {
        let op = random_operation(&board, &mut rng, step);
        let before = bytes_of(&board)?;
        match history.execute(&mut board, op) {
            Ok(()) => applied += 1,
            Err(_) => {
                if bytes_of(&board)? != before {
                    return Err(format!("a refused change altered the board at step {step}"));
                }
            }
        }
    }
    let done = bytes_of(&board)?;
    let mut undone = 0usize;
    while history.undo(&mut board) {
        undone += 1;
    }
    if undone != applied {
        return Err(format!(
            "{applied} changes were applied and {undone} could be undone"
        ));
    }
    if bytes_of(&board)? != start {
        return Err("undoing every change did not give the bytes of the start".to_owned());
    }
    while history.redo(&mut board) {}
    if bytes_of(&board)? != done {
        return Err("doing every change again did not give the bytes it had".to_owned());
    }
    Ok(())
}

/// Changes that cannot apply leave the board as it was, one by one and as a batch, and the
/// history does not record them.
fn rejection(seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    let mut board = random_board(seed, settings.property_max_components);
    let any_net = *board.nets.keys().next().ok_or("the board has no net")?;
    let any_component = *board
        .components
        .keys()
        .next()
        .ok_or("the board has no component")?;
    let used_footprint = board.components[&any_component].footprint;
    let any_pad = *board.footprints[&used_footprint]
        .pads
        .keys()
        .next()
        .ok_or("the footprint has no pad")?;
    let good_track = Track {
        net: any_net,
        layer: id_of(1),
        width: nm(250_000),
        path: vec![Segment::Line {
            start: pt(1_000_000, 1_000_000),
            end: pt(2_000_000, 1_000_000),
        }],
    };
    let refused = [
        (
            "a net with an id that exists",
            Operation::AddNet {
                id: any_net,
                name: "again".into(),
            },
        ),
        (
            "a net that is not there removed",
            Operation::RemoveNet { id: ghost() },
        ),
        (
            "a net that is not there renamed",
            Operation::RenameNet {
                id: ghost(),
                name: "x".into(),
            },
        ),
        (
            "a component that is not there moved",
            Operation::MoveComponent {
                id: ghost(),
                at: pt(0, 0),
                rotation: Udeg::ZERO,
            },
        ),
        (
            "a component with a footprint that is not there",
            Operation::AddComponent {
                id: id_of(8_000_000),
                component: Component {
                    refdes: "Z1".into(),
                    value: "1".into(),
                    symbol: None,
                    footprint: ghost(),
                    at: pt(0, 0),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            },
        ),
        (
            "a track on a net that is not there",
            Operation::AddTrack {
                id: id_of(8_000_001),
                track: Track {
                    net: ghost(),
                    ..good_track.clone()
                },
            },
        ),
        (
            "a track on a layer that is not there",
            Operation::AddTrack {
                id: id_of(8_000_002),
                track: Track {
                    layer: ghost(),
                    ..good_track.clone()
                },
            },
        ),
        (
            "a via between layers that are not there",
            Operation::AddVia {
                id: id_of(8_000_003),
                via: Via {
                    net: any_net,
                    at: pt(1_000_000, 1_000_000),
                    drill: nm(300_000),
                    diameter: nm(600_000),
                    from_layer: ghost(),
                    to_layer: id_of(2),
                },
            },
        ),
        (
            "a pad of a component that is not there",
            Operation::SetPadNet {
                pad: PadRef {
                    component: ghost(),
                    pad: any_pad,
                },
                net: Some(any_net),
            },
        ),
        (
            "a footprint in use removed",
            Operation::RemoveFootprint { id: used_footprint },
        ),
        (
            "a batch whose second change cannot apply",
            Operation::Batch {
                ops: vec![
                    Operation::AddNet {
                        id: id_of(8_000_004),
                        name: "half".into(),
                    },
                    Operation::RemoveNet { id: ghost() },
                ],
            },
        ),
    ];
    let mut history = History::new();
    for (what, op) in refused {
        let before = bytes_of(&board)?;
        if board.apply(&op).is_ok() {
            return Err(format!("{what}: the change was applied"));
        }
        if bytes_of(&board)? != before {
            return Err(format!("{what}: the board changed"));
        }
        if history.execute(&mut board, op).is_ok() || !history.done().is_empty() {
            return Err(format!("{what}: the history recorded a refused change"));
        }
    }
    Ok(())
}

/// Reading the canonical bytes gives the same board and writing it gives the same bytes.
fn json(seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    let board = random_board(seed, settings.property_max_components);
    let first = bytes_of(&board)?;
    let back = Board::from_bytes(&first).map_err(|e| e.to_string())?;
    if back != board {
        return Err("the board read from its bytes is another board".to_owned());
    }
    if bytes_of(&back)? != first {
        return Err("writing the board read back gives other bytes".to_owned());
    }
    Ok(())
}

fn eagle_trip(board: &Board) -> Result<Board, String> {
    let written = write_board(board);
    let mut ids = id_source(7000);
    read_board(&written.text, &mut ids)
        .map(|r| r.board)
        .map_err(|d| d.message)
}

fn kicad_trip(board: &Board) -> Result<Board, String> {
    let out = write_pcb(board, &WriteSettings::default());
    if out.report.has_problem() {
        return Err(
            "the KiCad writer reports that its file does not read back the same".to_owned(),
        );
    }
    let file = out.files.first().ok_or("the KiCad writer gave no file")?;
    let mut ids = id_source(7000);
    read_pcb(file.text.as_bytes(), &KicadSettings::default(), &mut ids)
        .map(|r| r.board)
        .map_err(|d| d.message)
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// Writing in EAGLE, reading and writing again gives the same file, the signature of the board
/// does not change and the facts that EAGLE keeps are the same.
fn eagle(seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    let board = random_board(seed, settings.property_max_components);
    let first = write_board(&board);
    let back = eagle_trip(&board)?;
    if write_board(&back).text != first.text {
        return Err("writing the board read back gives another file".to_owned());
    }
    if signature(&board) != signature(&back) {
        return Err("the signature of the board changed".to_owned());
    }
    let lines = compare(&facts(&board), &facts(&back), 0, &names(PRESERVED));
    if lines.is_empty() {
        Ok(())
    } else {
        Err(lines.join("; "))
    }
}

/// Writing in KiCad and reading back keeps the facts that KiCad keeps, and doing it twice gives
/// the same board as doing it once.
fn kicad(seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    let board = random_board(seed, settings.property_max_components);
    let once = kicad_trip(&board)?;
    let twice = kicad_trip(&once)?;
    let tolerance = settings.round_trip_tolerance_nm;
    let first = compare(&facts(&board), &facts(&once), tolerance, &names(PRESERVED));
    if !first.is_empty() {
        return Err(format!("after one trip: {}", first.join("; ")));
    }
    let second = compare(&facts(&once), &facts(&twice), tolerance, &names(PRESERVED));
    if !second.is_empty() {
        return Err(format!("after two trips: {}", second.join("; ")));
    }
    Ok(())
}

fn drc_counts(board: &Board) -> BTreeMap<String, usize> {
    let (rules, _) = RuleSet::parse(RULES);
    let report = board.check_rules(&rules);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for d in report.confirmed.iter().chain(report.inconclusive.iter()) {
        let code = d.code.as_deref().unwrap_or("none").to_owned();
        *counts.entry(code).or_insert(0) += 1;
    }
    counts
}

fn drc(
    seed: u64,
    settings: &CorpusSettings,
    trip: fn(&Board) -> Result<Board, String>,
) -> Result<(), String> {
    let board = random_board(seed, settings.property_max_components);
    let back = trip(&board)?;
    let (before, after) = (drc_counts(&board), drc_counts(&back));
    if before == after {
        Ok(())
    } else {
        Err(format!(
            "the rule check gave {before:?} before and {after:?} after"
        ))
    }
}

/// The autorouter file is the same every time and counts what the board holds.
fn dsn_file(seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    let board = random_board(seed, settings.property_max_components);
    let (ctx, route) = (RuleContext::default(), RouteSettings::default());
    let fail = |e: eh3d_phoenix_route::codes::RouteError| format!("{}: {}", e.code, e.detail);
    let a = dsn::export(&board, &ctx, &route).map_err(fail)?;
    let b = dsn::export(&board, &ctx, &route).map_err(fail)?;
    if a.text != b.text {
        return Err("the file is not the same twice".to_owned());
    }
    if a.tracks != board.tracks.len()
        || a.vias != board.vias.len()
        || a.components != board.components.len()
        || a.layers != 2
    {
        return Err(format!(
            "the file counts {} tracks, {} vias, {} components and {} layers",
            a.tracks, a.vias, a.components, a.layers
        ));
    }
    Ok(())
}

/// Tries one property with one seed.
pub fn check(name: &str, seed: u64, settings: &CorpusSettings) -> Result<(), String> {
    match name {
        "undo" => undo(seed, settings),
        "rejection" => rejection(seed, settings),
        "json" => json(seed, settings),
        "eagle" => eagle(seed, settings),
        "kicad" => kicad(seed, settings),
        "drc-eagle" => drc(seed, settings, eagle_trip),
        "drc-kicad" => drc(seed, settings, kicad_trip),
        "dsn" => dsn_file(seed, settings),
        other => Err(format!("{other} is not a property")),
    }
}

/// The result of one property over the seeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyResult {
    /// The name of the property.
    pub name: &'static str,
    /// The seeds that were tried.
    pub seeds: u32,
    /// The first seed that failed and why.
    pub failure: Option<(u64, String)>,
}

/// Tries every property (or the one named) with the seeds of the settings. A property stops at
/// its first failing seed.
pub fn run_all(settings: &CorpusSettings, only: Option<&str>) -> Vec<PropertyResult> {
    let mut out = Vec::new();
    for &name in PROPERTIES {
        if only.is_some_and(|o| o != name) {
            continue;
        }
        let mut result = PropertyResult {
            name,
            seeds: 0,
            failure: None,
        };
        for k in 0..settings.property_seeds {
            let seed = settings.property_first_seed + u64::from(k);
            result.seeds += 1;
            if let Err(why) = check(name, seed, settings) {
                result.failure = Some((seed, why));
                break;
            }
        }
        out.push(result);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_makes_the_same_board_and_it_is_a_good_board() {
        let mut different = false;
        let mut previous: Option<Board> = None;
        for seed in 1..=40 {
            let a = random_board(seed, 5);
            assert_eq!(a, random_board(seed, 5), "seed {seed}");
            assert!(a.is_consistent(), "seed {seed}: {:?}", a.validate());
            assert!((2..=5).contains(&a.components.len()), "seed {seed}");
            assert!(!a.nets.is_empty() && a.nets.len() <= 3);
            if let Some(p) = &previous {
                different |= *p != a;
            }
            previous = Some(a);
        }
        assert!(different);
    }

    #[test]
    fn the_generator_uses_every_kind_of_part_over_the_seeds() {
        let boards: Vec<Board> = (1..=40).map(|s| random_board(s, 5)).collect();
        assert!(boards.iter().any(|b| !b.tracks.is_empty()));
        assert!(boards.iter().any(|b| b.tracks.is_empty()));
        assert!(boards.iter().any(|b| !b.vias.is_empty()));
        assert!(boards.iter().any(|b| !b.pad_nets.is_empty()));
        assert!(boards
            .iter()
            .any(|b| b.footprints.len() == 2 && b.components.len() >= 4));
    }

    #[test]
    fn the_smallest_size_still_gives_two_components() {
        for seed in 1..=10 {
            assert_eq!(random_board(seed, 2).components.len(), 2);
        }
    }

    #[test]
    fn every_property_holds_for_the_seeds_of_the_settings() {
        let settings = CorpusSettings::default();
        let results = run_all(&settings, None);
        assert_eq!(results.len(), PROPERTIES.len());
        for r in &results {
            assert_eq!(r.failure, None, "{}", r.name);
            assert_eq!(r.seeds, settings.property_seeds, "{}", r.name);
        }
    }

    #[test]
    fn a_property_can_be_chosen_by_name_and_an_unknown_one_fails() {
        let settings = CorpusSettings {
            property_seeds: 3,
            ..CorpusSettings::default()
        };
        let one = run_all(&settings, Some("json"));
        assert_eq!(one.len(), 1);
        assert_eq!((one[0].name, one[0].seeds), ("json", 3));
        assert!(run_all(&settings, Some("nothing")).is_empty());
        assert!(check("nothing", 1, &settings).is_err());
    }

    #[test]
    fn the_rule_check_sees_the_narrow_tracks_and_the_small_vias_of_a_board() {
        let with_all = (1..=40)
            .map(|s| random_board(s, 5))
            .find(|b| !b.vias.is_empty() && b.tracks.values().any(|t| t.width.get() == 250_000))
            .expect("some seed makes a via and a narrow track");
        let counts = drc_counts(&with_all);
        assert!(
            counts.get("PHOENIX-RULE-002").copied().unwrap_or(0) >= 1,
            "{counts:?}"
        );
        assert!(
            counts.get("PHOENIX-RULE-003").copied().unwrap_or(0) >= 2,
            "{counts:?}"
        );
    }

    #[test]
    fn the_rejection_and_the_undo_work_on_the_board_of_a_seed() {
        let settings = CorpusSettings::default();
        assert_eq!(check("rejection", 99, &settings), Ok(()));
        assert_eq!(check("undo", 99, &settings), Ok(()));
    }
}
