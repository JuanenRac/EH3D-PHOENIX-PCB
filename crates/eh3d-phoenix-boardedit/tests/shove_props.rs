// SPDX-License-Identifier: GPL-3.0-or-later
//! Properties of the push on boards made with a fixed seed: whatever it does, it never adds a rule
//! error, never cuts a connection, keeps the clearance of the net classes, moves only tracks and
//! vias, is one step that undo takes back to the same bytes, and a push that is refused changes
//! nothing.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via,
};
use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp, Shape};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::{diff, findings, RuleContext};
use eh3d_phoenix_rules::{NetClass, NetClasses, RuleValues, Severity};

const NETS: usize = 12;
const ROUTE_NET: usize = 50;
const CLASS_CLEARANCE: i64 = 400_000;

struct Dice(u64);

impl Dice {
    fn next(&mut self, n: i64) -> i64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) % n as u64) as i64
    }

    /// A value of `low` to `high` in tenths of a millimetre, in nanometres.
    fn tenths(&mut self, low: i64, high: i64) -> i64 {
        (low + self.next(high - low + 1)) * 100_000
    }
}

fn id(n: usize) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n as u64).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

const TOP: usize = 1;
const BOTTOM: usize = 2;
const FOOTPRINT: usize = 3;
const PAD: usize = 4;

fn net_id(n: usize) -> Id {
    id(100 + n)
}

fn rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: nm(200_000),
            min_track_width: nm(150_000),
            min_drill_spacing: nm(200_000),
            min_edge_clearance: nm(200_000),
            ..RuleValues::default()
        },
        classes: NetClasses::new(vec![NetClass {
            name: "HV".to_owned(),
            nets: vec!["N0".to_owned(), "N1".to_owned()],
            min_track_width: Nm::ZERO,
            min_clearance: nm(CLASS_CLEARANCE),
            min_via_diameter: Nm::ZERO,
            min_via_drill: Nm::ZERO,
        }])
        .unwrap(),
    }
}

fn empty_board() -> Board {
    let mut board = Board::default();
    for (layer, name, side) in [(TOP, "Top", Side::Top), (BOTTOM, "Bottom", Side::Bottom)] {
        board.layers.insert(
            id(layer),
            Layer {
                name: name.to_owned(),
                kind: LayerKind::Copper,
                side: Some(side),
            },
        );
    }
    board.copper_stack = vec![id(TOP), id(BOTTOM)];
    board.outline = vec![
        pt(0, 0),
        pt(40_000_000, 0),
        pt(40_000_000, 30_000_000),
        pt(0, 30_000_000),
    ];
    for n in 0..NETS {
        board.nets.insert(
            net_id(n),
            Net {
                name: format!("N{n}"),
            },
        );
    }
    board.nets.insert(
        net_id(ROUTE_NET),
        Net {
            name: "R".to_owned(),
        },
    );
    let mut pads = BTreeMap::new();
    pads.insert(
        id(PAD),
        Pad {
            number: "1".to_owned(),
            shape: PadShape::Rect {
                width: nm(600_000),
                height: nm(600_000),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![id(TOP)],
            roundness: 0,
            plated: true,
        },
    );
    board.footprints.insert(
        id(FOOTPRINT),
        Footprint {
            name: "P".to_owned(),
            pads,
        },
    );
    board
}

struct Maker {
    board: Board,
    ctx: RuleContext,
    next: usize,
}

impl Maker {
    fn fresh(&mut self) -> Id {
        self.next += 1;
        id(self.next)
    }

    /// Keeps the change when it adds no rule error and the class clearance holds.
    fn add(&mut self, change: impl FnOnce(&mut Maker)) {
        let was = self.board.clone();
        change(self);
        let bad = findings(&self.board, &self.ctx)
            .iter()
            .any(|f| f.severity == Severity::Error)
            || !class_holds(&was, &self.board);
        if bad {
            self.board = was;
        }
    }

    fn pad(&mut self, net: usize, at: Point) {
        let component = self.fresh();
        self.board.components.insert(
            component,
            Component {
                refdes: format!("P{}", self.next),
                value: "x".to_owned(),
                symbol: None,
                footprint: id(FOOTPRINT),
                at,
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        self.board.pad_nets.push(PadNet {
            component,
            pad: id(PAD),
            net: net_id(net),
        });
        self.board.pad_nets.sort_by_key(PadNet::pad_ref);
    }

    fn track(&mut self, net: usize, layer: usize, points: &[Point]) {
        let track = self.fresh();
        self.board.tracks.insert(
            track,
            Track {
                net: net_id(net),
                layer: id(layer),
                width: nm(250_000),
                path: points
                    .windows(2)
                    .map(|w| Segment::Line {
                        start: w[0],
                        end: w[1],
                    })
                    .collect(),
            },
        );
    }

    fn via(&mut self, net: usize, at: Point) {
        let via = self.fresh();
        self.board.vias.insert(
            via,
            Via {
                net: net_id(net),
                at,
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: id(TOP),
                to_layer: id(BOTTOM),
            },
        );
    }
}

fn row_y(k: i64) -> i64 {
    3_000_000 + k * 1_400_000
}

/// Rows of tracks between pads, some of them ending on a via with a track below, and loose vias
/// and pads between the rows.
fn random_board(seed: u64) -> Board {
    let mut dice = Dice(seed);
    let mut maker = Maker {
        board: empty_board(),
        ctx: rules(),
        next: 1000,
    };
    for k in 0..17 {
        let net = (k as usize) % NETS;
        let y = row_y(k);
        let (x0, x1) = (dice.tenths(20, 90), dice.tenths(250, 370));
        let with_via = dice.next(2) == 0;
        maker.add(|m| {
            m.pad(net, pt(x0, y));
            if with_via {
                let turn = x1 - 3_000_000;
                m.via(net, pt(turn, y));
                m.track(net, TOP, &[pt(x0, y), pt(turn, y)]);
                m.track(net, BOTTOM, &[pt(turn, y), pt(x1, y)]);
            } else {
                m.pad(net, pt(x1, y));
                m.track(net, TOP, &[pt(x0, y), pt(x1, y)]);
            }
        });
    }
    for _ in 0..50 {
        let net = dice.next(NETS as i64) as usize;
        let at = pt(
            dice.tenths(30, 370),
            row_y(dice.next(16)) + dice.tenths(6, 8),
        );
        if dice.next(3) == 0 {
            maker.add(|m| m.pad(net, at));
        } else {
            maker.add(|m| m.via(net, at));
        }
    }
    maker.board
}

fn random_push(dice: &mut Dice) -> EditOp {
    let y = row_y(dice.next(16)) + dice.tenths(2, 7);
    let x0 = dice.tenths(20, 300);
    let x1 = x0 + dice.tenths(20, 90);
    let mut points = vec![pt(x0, y), pt(x1, y)];
    if dice.next(3) == 0 {
        points.push(pt(x1 + 700_000, y + 700_000));
    }
    EditOp::RouteShove {
        net: net_id(ROUTE_NET),
        layer: id(TOP),
        points,
        shape: Some(Shape::Diagonal),
        width_nm: None,
        horizontal_first: None,
    }
}

type Copper = (Id, Id, f64, (f64, f64), (f64, f64));

/// Every piece of copper as `(net, layer, half width, end, end)`; pads as discs of half a side.
fn copper(board: &Board) -> Vec<Copper> {
    let v = |p: Point| (p.x_nm.get() as f64, p.y_nm.get() as f64);
    let mut out = Vec::new();
    for track in board.tracks.values() {
        for s in &track.path {
            let half = track.width.get() as f64 / 2.0;
            out.push((track.net, track.layer, half, v(s.start()), v(s.end())));
        }
    }
    for via in board.vias.values() {
        for layer in [via.from_layer, via.to_layer] {
            let half = via.diameter.get() as f64 / 2.0;
            out.push((via.net, layer, half, v(via.at), v(via.at)));
        }
    }
    for (component, part) in &board.components {
        let net = board
            .pad_nets
            .iter()
            .find(|p| p.component == *component)
            .map(|p| p.net);
        if let Some(net) = net {
            out.push((net, id(TOP), 300_000.0, v(part.at), v(part.at)));
        }
    }
    out
}

fn point_to(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

fn apart(x: &Copper, y: &Copper) -> f64 {
    let side = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let (a, b, c, d) = (x.3, x.4, y.3, y.4);
    if side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0 {
        return 0.0;
    }
    point_to(a, c, d)
        .min(point_to(b, c, d))
        .min(point_to(c, a, b))
        .min(point_to(d, a, b))
}

/// Copper of the class HV that was not there before is not nearer to another net than the class
/// asks.
fn class_holds(before: &Board, after: &Board) -> bool {
    let old = copper(before);
    let all = copper(after);
    let hv = |net: Id| net == net_id(0) || net == net_id(1);
    all.iter().filter(|c| !old.contains(c)).all(|new| {
        all.iter().all(|other| {
            new.0 == other.0
                || new.1 != other.1
                || !(hv(new.0) || hv(other.0))
                || apart(new, other) + 2.0 >= new.2 + other.2 + CLASS_CLEARANCE as f64
        })
    })
}

fn assert_keeps_the_board(before: &Board, after: &Board, what: &str) {
    let ctx = rules();
    let changes = diff(&findings(before, &ctx), &findings(after, &ctx));
    assert!(!changes.adds_error(), "{what}: {:?}", changes.added);
    assert!(class_holds(before, after), "{what}: class clearance");
    let now = after.connectivity();
    for net in &before.connectivity().nets {
        let found = now.nets.iter().find(|n| n.net == net.net);
        for island in &net.islands {
            assert!(
                found.is_some_and(|f| f
                    .islands
                    .iter()
                    .any(|i| island.items.iter().all(|item| i.items.contains(item)))),
                "{what}: net {:?} was cut",
                net.net
            );
        }
    }
    assert_eq!(before.components, after.components, "{what}");
    assert_eq!(before.pad_nets, after.pad_nets, "{what}");
    assert_eq!(before.outline, after.outline, "{what}");
    assert_eq!(before.vias.len(), after.vias.len(), "{what}");
    for (id, via) in &before.vias {
        let now = &after.vias[id];
        assert_eq!(
            (
                via.net,
                via.drill,
                via.diameter,
                via.from_layer,
                via.to_layer
            ),
            (
                now.net,
                now.drill,
                now.diameter,
                now.from_layer,
                now.to_layer
            ),
            "{what}"
        );
    }
    for (id, track) in &before.tracks {
        let now = &after.tracks[id];
        assert_eq!(
            (track.net, track.layer, track.width),
            (now.net, now.layer, now.width),
            "{what}"
        );
        assert!(
            now.path.windows(2).all(|w| w[0].end() == w[1].start()),
            "{what}: track with a gap"
        );
    }
}

fn editor(board: Board) -> BoardEdit {
    BoardEdit::new(board, rules(), BoardEditSettings::default()).unwrap()
}

/// Whether an end of the new track is within the clearance of a pad of another net: a pad never
/// moves, so no push can do it.
fn ends_at_a_pad(board: &Board, op: &EditOp) -> bool {
    let EditOp::RouteShove { points, .. } = op else {
        return false;
    };
    [points.first(), points.last()]
        .into_iter()
        .flatten()
        .any(|end| {
            board.components.values().any(|pad| {
                let dx = (pad.at.x_nm.get() - end.x_nm.get()).abs() - 300_000;
                let dy = (pad.at.y_nm.get() - end.y_nm.get()).abs() - 300_000;
                (dx.max(0) as f64).hypot(dy.max(0) as f64) < 325_000.0
            })
        })
}

#[test]
fn the_findings_that_the_editor_keeps_give_the_same_push_as_a_new_check() {
    let board = random_board(1003);
    let with = editor(board.clone());
    let without = BoardEdit::new(
        board,
        rules(),
        BoardEditSettings {
            live_drc: false,
            ..BoardEditSettings::default()
        },
    )
    .unwrap();
    let mut dice = Dice(5);
    for n in 0..20 {
        let op = random_push(&mut dice);
        assert_eq!(with.preview_shove(&op), without.preview_shove(&op), "{n}");
    }
}

#[test]
fn a_push_never_adds_an_error_or_cuts_a_net_and_undo_gives_the_board_back() {
    let (mut done, mut with_vias, mut tried, mut parallel) = (0, 0, 0, 0);
    let mut at_a_pad = 0;
    let mut refused: BTreeMap<&'static str, usize> = BTreeMap::new();
    for seed in 0..12 {
        let board = random_board(1000 + seed);
        let bytes = board.to_bytes().unwrap();
        let mut dice = Dice(77 + seed);
        let base = editor(board.clone());
        // the same pushes with no walk and no via moved, to say how many the walk adds
        let plain = BoardEdit::new(
            board.clone(),
            rules(),
            BoardEditSettings {
                shove_vias: false,
                shove_walk_around: false,
                ..BoardEditSettings::default()
            },
        )
        .unwrap();
        for n in 0..25 {
            let op = random_push(&mut dice);
            let what = format!("seed {seed} push {n} {op:?}");
            let mut e = base.clone();
            tried += 1;
            parallel += usize::from(plain.preview_shove(&op).is_ok());
            let preview = e.preview_shove(&op);
            match e.apply(&op) {
                Ok(out) => {
                    done += 1;
                    let preview = preview.expect(&what);
                    with_vias += usize::from(!preview.vias.is_empty());
                    assert!(!out.drc.adds_error(), "{what}");
                    assert_keeps_the_board(&board, e.board(), &what);
                    assert_eq!(e.board().tracks.len(), board.tracks.len() + 1, "{what}");
                    for moved in &preview.moves {
                        assert_eq!(e.board().tracks[&moved.track].path, moved.after, "{what}");
                    }
                    for moved in &preview.vias {
                        assert_eq!(e.board().vias[&moved.via].at, moved.after, "{what}");
                    }
                    assert_eq!(e.steps(), 1, "{what}");
                    assert!(e.undo(), "{what}");
                    assert_eq!(e.board().to_bytes().unwrap(), bytes, "{what}");
                }
                Err(error) => {
                    assert!(preview.is_err(), "{what}: {error:?}");
                    *refused.entry(error.code).or_default() += 1;
                    at_a_pad += usize::from(ends_at_a_pad(&board, &op));
                    assert_eq!(e.board().to_bytes().unwrap(), bytes, "{what}");
                }
            }
        }
    }
    println!("PUSH_PROPS tried={tried} done={done} parallel_only={parallel} with_vias={with_vias} refused={refused:?} of them with an end at a pad of another net={at_a_pad}");
    assert!(done * 3 > tried, "{done} of {tried}: {refused:?}");
    assert!(with_vias > 0);
}

#[test]
fn pushes_one_after_another_keep_the_board_and_undo_all_the_way_back() {
    let mut total = 0;
    for seed in 0..6 {
        let board = random_board(5000 + seed);
        let bytes = board.to_bytes().unwrap();
        let mut dice = Dice(9 + seed);
        let mut e = editor(board);
        let mut done = 0;
        for n in 0..30 {
            let before = e.board().clone();
            let op = random_push(&mut dice);
            if e.apply(&op).is_ok() {
                done += 1;
                assert_keeps_the_board(&before, e.board(), &format!("seed {seed} push {n}"));
            }
        }
        total += done;
        for _ in 0..done {
            assert!(e.undo());
        }
        assert_eq!(e.board().to_bytes().unwrap(), bytes);
    }
    assert!(total > 20, "{total}");
}

#[test]
fn the_check_of_the_zone_changes_nothing_of_a_push() {
    let whole = BoardEditSettings {
        local_checks: false,
        ..BoardEditSettings::default()
    };
    let (mut done, mut near) = (0, 0);
    for seed in 0..6 {
        let board = random_board(5000 + seed);
        let mut dice = Dice(9 + seed);
        let mut one = editor(board.clone());
        let mut other = BoardEdit::new(board, rules(), whole.clone()).unwrap();
        for n in 0..30 {
            let op = random_push(&mut dice);
            let what = format!("seed {seed} push {n} {op:?}");
            assert_eq!(one.preview_shove(&op), other.preview_shove(&op), "{what}");
            let (a, b) = (one.apply(&op), other.apply(&op));
            assert_eq!(a.is_ok(), b.is_ok(), "{what}: {a:?} {b:?}");
            if let (Ok(a), Ok(b)) = (a, b) {
                done += 1;
                assert_eq!(a.drc, b.drc, "{what}");
            }
            assert_eq!(one.findings(), other.findings(), "{what}");
            assert_eq!(one.findings(), findings(one.board(), &rules()), "{what}");
        }
        near += one.zone_checks();
        assert_eq!(other.zone_checks(), 0);
    }
    assert!(done > 20, "{done}");
    assert!(near > 20, "{near}");
}
