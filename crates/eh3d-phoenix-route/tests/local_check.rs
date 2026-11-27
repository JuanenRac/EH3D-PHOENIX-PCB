// SPDX-License-Identifier: GPL-3.0-or-later
//! The check of the zone that an edit touches gives the same findings as the check of the whole
//! board: on boards and edits made with a fixed seed, with every rule on.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::{findings, findings_after, findings_near, RuleContext};
use eh3d_phoenix_rules::{codes, Finding, NetClass, NetClasses, RuleValues};

const NETS: usize = 14;
const TOP: usize = 1;
const BOTTOM: usize = 2;
const SILK: usize = 3;
const MASK: usize = 4;
const FOOTPRINTS: usize = 3;
const STEP: i64 = 100_000;
const WIDE: i64 = 300;
const HIGH: i64 = 200;

struct Dice(u64);

impl Dice {
    fn next(&mut self, n: i64) -> i64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) % n as u64) as i64
    }

    fn pick<V: Copy>(&mut self, list: &[V]) -> V {
        list[self.next(list.len() as i64) as usize]
    }

    /// A place of the board, on the grid.
    fn place(&mut self) -> Point {
        pt(self.next(WIDE) * STEP, self.next(HIGH) * STEP)
    }

    fn near(&mut self, p: Point, tenths: i64) -> Point {
        pt(
            p.x_nm.get() + (self.next(2 * tenths + 1) - tenths) * STEP,
            p.y_nm.get() + (self.next(2 * tenths + 1) - tenths) * STEP,
        )
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

fn net_id(n: usize) -> Id {
    id(100 + n)
}

fn rules() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_clearance: nm(250_000),
            min_track_width: nm(200_000),
            min_drill: nm(300_000),
            min_via_diameter: nm(500_000),
            min_annular_ring: nm(120_000),
            min_drill_spacing: nm(300_000),
            min_edge_clearance: nm(300_000),
            min_mask_bridge: nm(150_000),
            connect_tolerance: nm(10_000),
            silkscreen_over_pads: true,
            silkscreen_over_mask_openings: true,
        },
        classes: NetClasses::new(vec![NetClass {
            name: "P".to_owned(),
            nets: vec!["N0".to_owned(), "N1".to_owned()],
            min_track_width: nm(300_000),
            min_clearance: Nm::ZERO,
            min_via_diameter: nm(700_000),
            min_via_drill: Nm::ZERO,
        }])
        .unwrap(),
    }
}

fn empty_board() -> Board {
    let mut board = Board::default();
    for (layer, name, kind, side) in [
        (TOP, "Top", LayerKind::Copper, Some(Side::Top)),
        (BOTTOM, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
        (SILK, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
        (MASK, "Mask", LayerKind::SolderMask, Some(Side::Top)),
    ] {
        board.layers.insert(
            id(layer),
            Layer {
                name: name.to_owned(),
                kind,
                side,
            },
        );
    }
    board.copper_stack = vec![id(TOP), id(BOTTOM)];
    board.outline = vec![
        pt(0, 0),
        pt(WIDE * STEP, 0),
        pt(WIDE * STEP, HIGH * STEP),
        pt(0, HIGH * STEP),
    ];
    for n in 0..NETS {
        board.nets.insert(
            net_id(n),
            Net {
                name: format!("N{n}"),
            },
        );
    }
    // a footprint of two flat pads, one of two drilled pads and one of a single round pad
    let shapes = [
        (
            PadShape::Rect {
                width: nm(600_000),
                height: nm(500_000),
            },
            None,
            vec![(0, 0), (800_000, 0)],
        ),
        (
            PadShape::Circle {
                diameter: nm(900_000),
            },
            Some(500_000),
            vec![(0, 0), (1_300_000, 0)],
        ),
        (
            PadShape::Oblong {
                width: nm(400_000),
                height: nm(900_000),
            },
            None,
            vec![(0, 0)],
        ),
    ];
    for (n, (shape, drill, places)) in shapes.into_iter().enumerate() {
        let pads: BTreeMap<Id, Pad> = places
            .iter()
            .enumerate()
            .map(|(k, (x, y))| {
                (
                    id(20 + 4 * n + k),
                    Pad {
                        number: (k + 1).to_string(),
                        shape,
                        at: pt(*x, *y),
                        rotation: Udeg::ZERO,
                        drill: drill.map(nm),
                        layers: vec![id(TOP), id(MASK)],
                        roundness: 0,
                        plated: true,
                    },
                )
            })
            .collect();
        board.footprints.insert(
            id(10 + n),
            Footprint {
                name: format!("F{n}"),
                pads,
            },
        );
    }
    board
}

struct Maker {
    dice: Dice,
    next: usize,
}

impl Maker {
    fn fresh(&mut self) -> Id {
        self.next += 1;
        id(self.next)
    }

    fn component(&mut self, board: &mut Board) {
        let component = self.fresh();
        let footprint = id(10 + self.dice.next(FOOTPRINTS as i64) as usize);
        board.components.insert(
            component,
            Component {
                refdes: format!("U{}", self.next),
                value: "x".to_owned(),
                symbol: None,
                footprint,
                at: self.dice.place(),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        let pads: Vec<Id> = board.footprints[&footprint].pads.keys().copied().collect();
        for pad in pads {
            // one pad in eight has no net
            if self.dice.next(8) > 0 {
                board.pad_nets.push(PadNet {
                    component,
                    pad,
                    net: net_id(self.dice.next(NETS as i64) as usize),
                });
            }
        }
        board.pad_nets.sort_by_key(PadNet::pad_ref);
    }

    fn track(&mut self, board: &mut Board) {
        let track = self.fresh();
        let a = self.dice.place();
        let b = self.dice.near(a, 30);
        let mut path = vec![Segment::Line { start: a, end: b }];
        if self.dice.next(2) == 0 {
            path.push(Segment::Line {
                start: b,
                end: self.dice.near(b, 20),
            });
        }
        board.tracks.insert(
            track,
            Track {
                net: net_id(self.dice.next(NETS as i64) as usize),
                layer: id(self.dice.pick(&[TOP, BOTTOM])),
                width: nm(self.dice.pick(&[150_000, 250_000, 300_000])),
                path,
            },
        );
    }

    fn via(&mut self, board: &mut Board) {
        let via = self.fresh();
        board.vias.insert(
            via,
            Via {
                net: net_id(self.dice.next(NETS as i64) as usize),
                at: self.dice.place(),
                drill: nm(self.dice.pick(&[250_000, 300_000, 400_000])),
                diameter: nm(self.dice.pick(&[450_000, 600_000, 800_000])),
                from_layer: id(TOP),
                to_layer: id(BOTTOM),
            },
        );
    }

    fn silk(&mut self, board: &mut Board) {
        let item = self.fresh();
        let a = self.dice.place();
        let b = self.dice.near(a, 15);
        board.silkscreen.insert(
            item,
            SilkItem::Stroke {
                layer: id(SILK),
                width: nm(150_000),
                path: vec![Segment::Line { start: a, end: b }],
            },
        );
    }

    fn zone(&mut self, board: &mut Board) {
        let zone = self.fresh();
        let a = self.dice.place();
        let (x, y) = (a.x_nm.get(), a.y_nm.get());
        let size = (10 + self.dice.next(30)) * STEP;
        board.zones.insert(
            zone,
            Zone {
                net: net_id(self.dice.next(NETS as i64) as usize),
                layer: id(BOTTOM),
                outline: vec![a, pt(x + size, y), pt(x + size, y + size), pt(x, y + size)],
                clearance: nm(self.dice.pick(&[200_000, 300_000])),
                cutouts: Vec::new(),
            },
        );
    }

    /// One edit of the kinds an editor makes.
    fn edit(&mut self, board: &mut Board) {
        let key = |dice: &mut Dice, keys: Vec<Id>| (!keys.is_empty()).then(|| dice.pick(&keys));
        match self.dice.next(14) {
            0 => self.track(board),
            1 => self.via(board),
            2 => self.component(board),
            3 => self.silk(board),
            4 => {
                if let Some(t) = key(&mut self.dice, board.tracks.keys().copied().collect()) {
                    board.tracks.remove(&t);
                }
            }
            5 | 6 => {
                if let Some(t) = key(&mut self.dice, board.tracks.keys().copied().collect()) {
                    let (dx, dy) = (
                        (self.dice.next(11) - 5) * STEP,
                        (self.dice.next(11) - 5) * STEP,
                    );
                    let moved = |p: Point| pt(p.x_nm.get() + dx, p.y_nm.get() + dy);
                    let track = board.tracks.get_mut(&t).unwrap();
                    for segment in &mut track.path {
                        *segment = Segment::Line {
                            start: moved(segment.start()),
                            end: moved(segment.end()),
                        };
                    }
                }
            }
            7 => {
                if let Some(v) = key(&mut self.dice, board.vias.keys().copied().collect()) {
                    let at = self.dice.near(board.vias[&v].at, 8);
                    board.vias.get_mut(&v).unwrap().at = at;
                }
            }
            8 => {
                if let Some(v) = key(&mut self.dice, board.vias.keys().copied().collect()) {
                    board.vias.remove(&v);
                }
            }
            9 | 10 => {
                if let Some(c) = key(&mut self.dice, board.components.keys().copied().collect()) {
                    let at = self.dice.near(board.components[&c].at, 12);
                    board.components.get_mut(&c).unwrap().at = at;
                }
            }
            11 => {
                if let Some(c) = key(&mut self.dice, board.components.keys().copied().collect()) {
                    board.components.remove(&c);
                    board.pad_nets.retain(|p| p.component != c);
                }
            }
            12 => {
                if let Some(t) = key(&mut self.dice, board.tracks.keys().copied().collect()) {
                    let net = net_id(self.dice.next(NETS as i64) as usize);
                    let track = board.tracks.get_mut(&t).unwrap();
                    track.net = net;
                    track.width = nm(self.dice.pick(&[150_000, 250_000, 300_000]));
                }
                if !board.pad_nets.is_empty() {
                    let n = self.dice.next(board.pad_nets.len() as i64) as usize;
                    board.pad_nets[n].net = net_id(self.dice.next(NETS as i64) as usize);
                }
            }
            _ => {
                if self.dice.next(2) == 0 || board.zones.is_empty() {
                    self.zone(board);
                } else if let Some(z) = key(&mut self.dice, board.zones.keys().copied().collect()) {
                    board.zones.remove(&z);
                }
            }
        }
    }
}

fn shorted(list: &[Finding]) -> bool {
    list.iter().any(|f| f.code == codes::COPPER_SHORT)
}

/// A board with many broken rules and no short.
fn random_board(maker: &mut Maker, ctx: &RuleContext) -> Board {
    let mut board = empty_board();
    for n in 0..260 {
        let was = board.clone();
        match n % 5 {
            0 => maker.component(&mut board),
            1 | 2 => maker.track(&mut board),
            3 => maker.via(&mut board),
            _ => maker.silk(&mut board),
        }
        if shorted(&findings(&board, ctx)) {
            board = was;
        }
    }
    board
}

#[test]
fn the_zone_check_gives_the_findings_of_the_whole_board() {
    let ctx = rules();
    let (mut near, mut whole, mut with_changes) = (0, 0, 0);
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    for seed in 0..8 {
        let mut maker = Maker {
            dice: Dice(31 + seed),
            next: 1_000,
        };
        let mut board = random_board(&mut maker, &ctx);
        let mut known = findings(&board, &ctx);
        for step in 0..120 {
            let mut next = board.clone();
            // one edit, or a few together
            for _ in 0..1 + maker.dice.next(6) / 4 {
                maker.edit(&mut next);
            }
            let full = findings(&next, &ctx);
            let what = format!("seed {seed} step {step}");
            match findings_near(&board, &known, &next, &ctx) {
                Some(found) => {
                    near += 1;
                    assert_eq!(found, full, "{what}");
                    if full != known {
                        with_changes += 1;
                        for f in full.iter().filter(|f| !known.contains(f)) {
                            *kinds.entry(f.code).or_default() += 1;
                        }
                    }
                }
                None => whole += 1,
            }
            assert_eq!(findings_after(&board, &known, &next, &ctx), full, "{what}");
            if !shorted(&full) {
                board = next;
                known = full;
            }
        }
    }
    println!("ZONE_CHECK near={near} whole={whole} with_changes={with_changes} added={kinds:?}");
    // most edits are judged by their zone, and many of those change what the rules say
    assert!(near > 500, "{near} near, {whole} whole");
    assert!(with_changes > 150, "{with_changes}");
    assert!(kinds.len() >= 6, "{kinds:?}");
}

#[test]
fn what_cannot_be_judged_by_the_zone_asks_for_the_whole_board() {
    let ctx = rules();
    let mut board = empty_board();
    let track = |net: usize, x: i64, y: i64| Track {
        net: net_id(net),
        layer: id(TOP),
        width: nm(300_000),
        path: vec![Segment::Line {
            start: pt(x, y),
            end: pt(x + 2_000_000, y),
        }],
    };
    board.tracks.insert(id(900), track(2, 5_000_000, 5_000_000));
    let known = findings(&board, &ctx);

    // nothing changed: the same findings
    assert_eq!(
        findings_near(&board, &known, &board, &ctx),
        Some(known.clone())
    );

    // another outline
    let mut outline = board.clone();
    outline.outline[2] = pt(WIDE * STEP + STEP, HIGH * STEP);
    assert_eq!(findings_near(&board, &known, &outline, &ctx), None);

    // a short that was already there
    let mut short = board.clone();
    short.tracks.insert(id(901), track(3, 6_000_000, 5_000_000));
    let with_short = findings(&short, &ctx);
    assert!(shorted(&with_short));
    assert_eq!(findings_near(&short, &with_short, &board, &ctx), None);
    // adding it is judged by the zone: the two tracks touch
    assert_eq!(
        findings_near(&board, &known, &short, &ctx),
        Some(with_short)
    );

    // a track that ends on a pad with no net could join nets through it
    let mut loose = board.clone();
    loose.components.insert(
        id(902),
        Component {
            refdes: "U1".to_owned(),
            value: "x".to_owned(),
            symbol: None,
            footprint: id(12),
            at: pt(10_000_000, 10_000_000),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    let with_pad = findings(&loose, &ctx);
    let mut joined = loose.clone();
    joined
        .tracks
        .insert(id(903), track(4, 8_000_000, 10_000_000));
    assert_eq!(findings_near(&loose, &with_pad, &joined, &ctx), None);
    assert_eq!(
        findings_after(&loose, &with_pad, &joined, &ctx),
        findings(&joined, &ctx)
    );
}
