// SPDX-License-Identifier: GPL-3.0-or-later
//! What the estimates need to know about each net of a board: its copper length, the layer and
//! the width most of it runs on and its delay. The length and the delay come from the lengths
//! crate (with the stack-up when there is one); the straight pieces of the tracks are listed for
//! the search of parallel runs.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id, Segment};
use eh3d_phoenix_lengths::{measure_nets, LengthSettings};
use eh3d_phoenix_stackup::StackupJob;

use crate::codes::{SiError, LIMIT_EXCEEDED};
use crate::env::Env;
use crate::settings::SiSettings;

/// A point as a pair of nanometres.
pub(crate) type P = (i64, i64);

/// A straight piece of a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    pub(crate) net: Id,
    pub(crate) layer: Id,
    pub(crate) width: i64,
    pub(crate) a: P,
    pub(crate) b: P,
}

/// The straight pieces of all the tracks, in the order of the tracks, and the number of arcs that
/// were left out.
pub(crate) fn straight_pieces(board: &Board) -> (Vec<Piece>, usize) {
    let mut pieces = Vec::new();
    let mut arcs = 0;
    for track in board.tracks.values() {
        for segment in &track.path {
            match segment {
                Segment::Line { start, end } => pieces.push(Piece {
                    net: track.net,
                    layer: track.layer,
                    width: track.width.get(),
                    a: (start.x_nm.get(), start.y_nm.get()),
                    b: (end.x_nm.get(), end.y_nm.get()),
                }),
                Segment::Arc { .. } => arcs += 1,
            }
        }
    }
    (pieces, arcs)
}

/// The name of a layer.
pub(crate) fn layer_name(board: &Board, layer: Id) -> String {
    board
        .layers
        .get(&layer)
        .map_or_else(|| layer.to_string(), |l| l.name.clone())
}

/// The name of a net.
pub(crate) fn net_name(board: &Board, net: Id) -> String {
    board
        .nets
        .get(&net)
        .map_or_else(|| net.to_string(), |n| n.name.clone())
}

/// One net with copper.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NetData {
    pub(crate) name: String,
    /// The copper length of tracks and vias, in nanometres.
    pub(crate) length_nm: i64,
    /// The delay of the net, in picoseconds.
    pub(crate) td_ps: f64,
    /// The layer most of the tracks run on.
    pub(crate) layer: String,
    /// The width most of the tracks on that layer have, in nanometres.
    pub(crate) width_nm: i64,
}

/// The length of a segment in nanometres, an arc by its chord.
fn chord(segment: &Segment) -> f64 {
    let (a, b) = (segment.start(), segment.end());
    let dx = (a.x_nm.get() - b.x_nm.get()) as f64;
    let dy = (a.y_nm.get() - b.y_nm.get()) as f64;
    dx.hypot(dy)
}

/// The nets that have copper, ordered by name, with their length, delay and the layer and width
/// they mostly run on.
pub(crate) fn collect(
    board: &Board,
    settings: &SiSettings,
    job: Option<&StackupJob>,
    env: &mut Env,
) -> Result<Vec<NetData>, SiError> {
    if board.tracks.len() + board.vias.len() > settings.max_items {
        return Err(SiError::new(
            LIMIT_EXCEEDED,
            "the board has more tracks and vias than max_items",
        ));
    }
    let (lengths, _) = measure_nets(board, &LengthSettings::default(), job)
        .map_err(|e| SiError::new(LIMIT_EXCEEDED, e.detail))?;
    let mut runs: BTreeMap<(Id, Id, i64), f64> = BTreeMap::new();
    for track in board.tracks.values() {
        let sum: f64 = track.path.iter().map(chord).sum();
        *runs
            .entry((track.net, track.layer, track.width.get()))
            .or_insert(0.0) += sum;
    }
    let mut mostly: BTreeMap<Id, (Id, i64, f64)> = BTreeMap::new();
    for ((net, layer, width), length) in &runs {
        let entry = mostly.entry(*net).or_insert((*layer, *width, *length));
        if *length > entry.2 {
            *entry = (*layer, *width, *length);
        }
    }
    let mut out = Vec::new();
    for item in &lengths {
        let Some((layer, width, _)) = mostly.get(&item.net) else {
            continue;
        };
        if item.total_nm <= 0 {
            continue;
        }
        let layer = layer_name(board, *layer);
        let td_ps = if job.is_some() {
            item.delay_fs as f64 / 1000.0
        } else {
            let line = env.line(&layer, *width, 1.0);
            item.total_nm as f64 / 1.0e6 * line.delay_ps_per_mm
        };
        out.push(NetData {
            name: item.name.clone(),
            length_nm: item.total_nm,
            td_ps,
            layer,
            width_nm: *width,
        });
    }
    out.sort_by_key(|n| n.name.clone());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Layer, LayerKind, Net, Track};
    use eh3d_phoenix_model::{Nm, Point};

    const MM: i64 = 1_000_000;

    fn id(n: u8) -> Id {
        format!("01890a5d-ac96-7ab2-80e2-4536629c13{n:02x}")
            .parse()
            .unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn board() -> Board {
        let mut b = Board::default();
        for (n, name) in [(1, "F.Cu"), (2, "B.Cu")] {
            b.layers.insert(
                id(n),
                Layer {
                    name: name.to_owned(),
                    kind: LayerKind::Copper,
                    side: None,
                },
            );
        }
        for (n, name) in [(3, "B"), (4, "A")] {
            b.nets.insert(
                id(n),
                Net {
                    name: name.to_owned(),
                },
            );
        }
        let track = |net: u8, layer: u8, width: i64, x0: i64, x1: i64| Track {
            net: id(net),
            layer: id(layer),
            width: Nm::new(width).unwrap(),
            path: vec![Segment::Line {
                start: pt(x0, 0),
                end: pt(x1, 0),
            }],
        };
        b.tracks.insert(id(10), track(3, 1, 200_000, 0, 10 * MM));
        b.tracks.insert(id(11), track(4, 1, 200_000, 0, 5 * MM));
        b.tracks.insert(id(12), track(4, 2, 300_000, 0, 8 * MM));
        b
    }

    #[test]
    fn the_nets_are_ordered_by_name_with_their_main_layer_and_width() {
        let b = board();
        let settings = SiSettings::default();
        let mut env = Env::new(None, &settings);
        let nets = collect(&b, &settings, None, &mut env).unwrap();
        assert_eq!(nets.len(), 2);
        assert_eq!(nets[0].name, "A");
        assert_eq!(nets[0].length_nm, 13 * MM);
        // 8 mm on the bottom layer against 5 mm on the top one
        assert_eq!(nets[0].layer, "B.Cu");
        assert_eq!(nets[0].width_nm, 300_000);
        assert_eq!(nets[1].name, "B");
        assert_eq!(nets[1].layer, "F.Cu");
        // sqrt(3) / c = 5.7773 ps/mm over 10 mm
        assert!((nets[1].td_ps - 57.773).abs() < 0.01, "{}", nets[1].td_ps);
    }

    #[test]
    fn the_straight_pieces_are_listed_and_arcs_are_counted() {
        let mut b = board();
        b.tracks.insert(
            id(13),
            Track {
                net: id(3),
                layer: id(1),
                width: Nm::new(200_000).unwrap(),
                path: vec![Segment::Arc {
                    start: pt(0, 0),
                    end: pt(MM, MM),
                    sweep: eh3d_phoenix_model::Udeg::new(90_000_000).unwrap(),
                }],
            },
        );
        let (pieces, arcs) = straight_pieces(&b);
        assert_eq!(pieces.len(), 3);
        assert_eq!(arcs, 1);
        assert_eq!(layer_name(&b, id(1)), "F.Cu");
        assert_eq!(net_name(&b, id(3)), "B");
    }

    #[test]
    fn too_many_items_are_refused() {
        let b = board();
        let settings = SiSettings {
            max_items: 2,
            ..SiSettings::default()
        };
        let mut env = Env::new(None, &settings);
        assert_eq!(
            collect(&b, &settings, None, &mut env).unwrap_err().code,
            LIMIT_EXCEEDED
        );
    }
}
