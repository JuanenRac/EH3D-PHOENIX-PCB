//! A seeded generator of valid boards and operations for the tests.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Operation, Pad, PadNet, PadRef,
    PadShape, Segment, Side, SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, Nm, Point, StepClock, Udeg};
use std::collections::BTreeMap;

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
}

pub struct Gen {
    pub rng: Rng,
    ids: IdGenerator<StepClock, DeterministicRandom>,
}

impl Gen {
    pub fn new(seed: u64) -> Gen {
        Gen {
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            ids: IdGenerator::new(
                StepClock::new(1_700_000_000_000, 1),
                DeterministicRandom::new(seed),
            ),
        }
    }

    pub fn id(&mut self) -> Id {
        self.ids.generate().unwrap()
    }

    pub fn nm(&mut self, lo: i64, hi: i64) -> Nm {
        Nm::new(self.rng.range(lo, hi)).unwrap()
    }

    pub fn point(&mut self) -> Point {
        Point {
            x_nm: self.nm(-50_000_000, 50_000_000),
            y_nm: self.nm(-50_000_000, 50_000_000),
        }
    }

    /// A track path of 45-degree lines, each piece starting where the one before ends.
    pub fn path(&mut self) -> Vec<Segment> {
        let mut at = self.point();
        let mut out = Vec::new();
        for _ in 0..self.rng.range(1, 5) {
            let step = self.rng.range(1, 20) * 100_000;
            let (dx, dy) = [
                (1, 0),
                (0, 1),
                (1, 1),
                (1, -1),
                (-1, 0),
                (0, -1),
                (-1, -1),
                (-1, 1),
            ][self.rng.below(8) as usize];
            let end = Point {
                x_nm: Nm::new(at.x_nm.get() + dx * step).unwrap(),
                y_nm: Nm::new(at.y_nm.get() + dy * step).unwrap(),
            };
            out.push(Segment::Line { start: at, end });
            at = end;
        }
        out
    }

    pub fn board(&mut self, parts: usize) -> Board {
        let mut b = Board {
            name: format!("board {}", self.rng.below(1000)),
            outline: vec![
                Point::from_nm(-60_000_000, -40_000_000).unwrap(),
                Point::from_nm(60_000_000, -40_000_000).unwrap(),
                Point::from_nm(60_000_000, 40_000_000).unwrap(),
                Point::from_nm(-60_000_000, 40_000_000).unwrap(),
            ],
            ..Board::default()
        };
        let top = self.id();
        let bottom = self.id();
        let silk = self.id();
        b.layers.insert(
            top,
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.layers.insert(
            bottom,
            Layer {
                name: "Bottom".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.layers.insert(
            silk,
            Layer {
                name: "Silk".into(),
                kind: LayerKind::Silkscreen,
                side: Some(Side::Top),
            },
        );
        let nets: Vec<Id> = (0..parts.max(1))
            .map(|i| {
                let id = self.id();
                b.nets.insert(
                    id,
                    Net {
                        name: format!("N{i}"),
                    },
                );
                id
            })
            .collect();
        let mut footprints = Vec::new();
        for i in 0..(parts / 3 + 1) {
            let fp = self.id();
            let mut pads = BTreeMap::new();
            for n in 0..self.rng.range(1, 4) {
                let shape = match self.rng.below(3) {
                    0 => PadShape::Circle {
                        diameter: self.nm(500_000, 2_000_000),
                    },
                    1 => PadShape::Rect {
                        width: self.nm(500_000, 2_000_000),
                        height: self.nm(500_000, 2_000_000),
                    },
                    _ => PadShape::Oblong {
                        width: self.nm(500_000, 2_000_000),
                        height: self.nm(500_000, 2_000_000),
                    },
                };
                let drill = if self.rng.below(2) == 0 {
                    Some(self.nm(300_000, 400_000))
                } else {
                    None
                };
                pads.insert(
                    self.id(),
                    Pad {
                        number: (n + 1).to_string(),
                        shape,
                        at: self.point(),
                        rotation: Udeg::new(self.rng.range(0, 359) * 1_000_000).unwrap(),
                        drill,
                        layers: vec![top, bottom],
                        roundness: 0,
                        plated: true,
                    },
                );
            }
            b.footprints.insert(
                fp,
                Footprint {
                    name: format!("FP{i}"),
                    pads,
                },
            );
            footprints.push(fp);
        }
        for i in 0..parts {
            let id = self.id();
            let fp = footprints[self.rng.below(footprints.len() as u64) as usize];
            b.components.insert(
                id,
                Component {
                    refdes: format!("U{i}"),
                    value: format!("{}k", self.rng.below(100)),
                    symbol: if self.rng.below(2) == 0 {
                        Some(self.id())
                    } else {
                        None
                    },
                    footprint: fp,
                    at: self.point(),
                    rotation: Udeg::new(self.rng.range(0, 359) * 1_000_000).unwrap(),
                    side: if self.rng.below(2) == 0 {
                        Side::Top
                    } else {
                        Side::Bottom
                    },
                },
            );
            for pad in b.footprints[&fp].pads.keys().copied().collect::<Vec<_>>() {
                if self.rng.below(3) != 0 {
                    let net = nets[self.rng.below(nets.len() as u64) as usize];
                    b.pad_nets.push(PadNet {
                        component: id,
                        pad,
                        net,
                    });
                }
            }
        }
        b.pad_nets.sort_by_key(|p| p.pad_ref());
        for _ in 0..parts {
            let net = nets[self.rng.below(nets.len() as u64) as usize];
            let layer = if self.rng.below(2) == 0 { top } else { bottom };
            let id = self.id();
            b.tracks.insert(
                id,
                Track {
                    net,
                    layer,
                    width: self.nm(100_000, 500_000),
                    path: self.path(),
                },
            );
        }
        for _ in 0..parts / 2 {
            let net = nets[self.rng.below(nets.len() as u64) as usize];
            let id = self.id();
            b.vias.insert(
                id,
                Via {
                    net,
                    at: self.point(),
                    drill: self.nm(200_000, 400_000),
                    diameter: self.nm(500_000, 900_000),
                    from_layer: top,
                    to_layer: bottom,
                },
            );
        }
        let net = nets[0];
        let id = self.id();
        b.zones.insert(
            id,
            Zone {
                net,
                layer: top,
                outline: vec![
                    Point::from_nm(-1_000_000, -1_000_000).unwrap(),
                    Point::from_nm(1_000_000, -1_000_000).unwrap(),
                    Point::from_nm(1_000_000, 1_000_000).unwrap(),
                ],
                clearance: self.nm(100_000, 300_000),
                cutouts: Vec::new(),
            },
        );
        for i in 0..parts / 2 + 1 {
            let id = self.id();
            let item = if i % 2 == 0 {
                const WORDS: [&str; 4] = ["R1", "C12 10uF", "GND", "J3-5V"];
                SilkItem::Text {
                    layer: silk,
                    at: self.point(),
                    text: WORDS[self.rng.below(4) as usize].to_owned(),
                    height: self.nm(600_000, 1_500_000),
                    stroke: self.nm(100_000, 200_000),
                    rotation: Udeg::new(self.rng.range(0, 3) * 90_000_000).unwrap(),
                    mirrored: self.rng.below(2) == 0,
                }
            } else {
                SilkItem::Stroke {
                    layer: silk,
                    width: self.nm(100_000, 250_000),
                    path: self.path(),
                }
            };
            b.silkscreen.insert(id, item);
        }
        b
    }

    /// A random operation that is valid for `board` most of the time (some are meant to fail).
    pub fn operation(&mut self, board: &Board) -> Operation {
        let nets: Vec<Id> = board.nets.keys().copied().collect();
        let comps: Vec<Id> = board.components.keys().copied().collect();
        let tracks: Vec<Id> = board.tracks.keys().copied().collect();
        let vias: Vec<Id> = board.vias.keys().copied().collect();
        let layers: Vec<Id> = board
            .layers
            .iter()
            .filter(|(_, l)| l.kind == LayerKind::Copper)
            .map(|(i, _)| *i)
            .collect();
        let pick = |rng: &mut Rng, v: &[Id]| v[rng.below(v.len() as u64) as usize];
        match self.rng.below(11) {
            0 => Operation::AddNet {
                id: self.id(),
                name: format!("X{}", self.rng.below(99)),
            },
            1 if !nets.is_empty() => Operation::RenameNet {
                id: pick(&mut self.rng, &nets),
                name: format!("R{}", self.rng.below(99)),
            },
            2 if !nets.is_empty() => Operation::RemoveNet {
                id: pick(&mut self.rng, &nets),
            },
            3 if !comps.is_empty() => Operation::MoveComponent {
                id: pick(&mut self.rng, &comps),
                at: self.point(),
                rotation: Udeg::new(self.rng.range(0, 359) * 1_000_000).unwrap(),
            },
            4 if !comps.is_empty() => Operation::RemoveComponent {
                id: pick(&mut self.rng, &comps),
            },
            5 if !comps.is_empty() && !nets.is_empty() => {
                let c = pick(&mut self.rng, &comps);
                let fp = &board.footprints[&board.components[&c].footprint];
                let pads: Vec<Id> = fp.pads.keys().copied().collect();
                let net = if self.rng.below(4) == 0 {
                    None
                } else {
                    Some(pick(&mut self.rng, &nets))
                };
                Operation::SetPadNet {
                    pad: PadRef {
                        component: c,
                        pad: pick(&mut self.rng, &pads),
                    },
                    net,
                }
            }
            6 if !nets.is_empty() => Operation::AddTrack {
                id: self.id(),
                track: Track {
                    net: pick(&mut self.rng, &nets),
                    layer: pick(&mut self.rng, &layers),
                    width: self.nm(100_000, 400_000),
                    path: self.path(),
                },
            },
            7 if !tracks.is_empty() => Operation::RemoveTrack {
                id: pick(&mut self.rng, &tracks),
            },
            8 if !nets.is_empty() => Operation::AddVia {
                id: self.id(),
                via: Via {
                    net: pick(&mut self.rng, &nets),
                    at: self.point(),
                    drill: self.nm(200_000, 300_000),
                    diameter: self.nm(500_000, 800_000),
                    from_layer: layers[0],
                    to_layer: layers[1],
                },
            },
            9 if !vias.is_empty() => Operation::RemoveVia {
                id: pick(&mut self.rng, &vias),
            },
            _ => Operation::Batch {
                ops: vec![Operation::AddNet {
                    id: self.id(),
                    name: "B".into(),
                }],
            },
        }
    }
}
