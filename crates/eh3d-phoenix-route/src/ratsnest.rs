// SPDX-License-Identifier: GPL-3.0-or-later
//! The ratsnest: the connections that are still to be routed.
//!
//! The islands of each net come from the connectivity of the board (which copper touches which).
//! Within a net the islands are joined by a minimum spanning tree over the distance between the
//! closest points of two islands (the centre of a pad, a via, the vertices of a track or of a zone
//! outline), built with Prim's algorithm. Every tie is broken by the lowest index, so the same
//! board always gives the same lines, in the order of the nets.

use eh3d_phoenix_board::connectivity::{pad_position, Item};
use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::Point;

use crate::codes::{RouteError, LIMIT_EXCEEDED};
use crate::geom::{isqrt, pair};
use crate::settings::RouteSettings;

/// One connection still to be routed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatLine {
    /// The net.
    pub net: Id,
    /// The copper at one end.
    pub from: Item,
    /// Where that end is.
    pub from_at: Point,
    /// The copper at the other end.
    pub to: Item,
    /// Where that end is.
    pub to_at: Point,
    /// The distance between the two ends in nanometres, rounded down.
    pub length_nm: i64,
}

fn anchors_of(board: &Board, item: Item) -> Vec<Point> {
    match item {
        Item::Pad(pad) => pad_position(board, pad).into_iter().collect(),
        Item::Via(id) => board.vias.get(&id).map(|v| v.at).into_iter().collect(),
        Item::Track(id) => {
            let mut points = Vec::new();
            if let Some(track) = board.tracks.get(&id) {
                for (i, segment) in track.path.iter().enumerate() {
                    if i == 0 {
                        points.push(segment.start());
                    }
                    points.push(segment.end());
                }
            }
            points
        }
        Item::Zone(id) => board
            .zones
            .get(&id)
            .map(|z| z.outline.clone())
            .unwrap_or_default(),
    }
}

/// The text that names an item for a person: `R1.2` for a pad, `track <id>`, `via <id>` or
/// `zone <id>`.
pub fn item_label(board: &Board, item: Item) -> String {
    match item {
        Item::Pad(pad) => {
            let named = board.components.get(&pad.component).and_then(|c| {
                let number = board
                    .footprints
                    .get(&c.footprint)?
                    .pads
                    .get(&pad.pad)?
                    .number
                    .clone();
                Some(format!("{}.{}", c.refdes, number))
            });
            named.unwrap_or_else(|| format!("pad {}/{}", pad.component, pad.pad))
        }
        Item::Via(id) => format!("via {id}"),
        Item::Track(id) => format!("track {id}"),
        Item::Zone(id) => format!("zone {id}"),
    }
}

fn distance_squared(a: Point, b: Point) -> i128 {
    a.distance_squared(b)
}

/// The closest pair of anchors of two islands: the squared distance and the indices.
fn closest(
    a: &[(Item, Point)],
    b: &[(Item, Point)],
    pairs: &mut u64,
) -> Option<(i128, usize, usize)> {
    let mut best: Option<(i128, usize, usize)> = None;
    for (i, (_, p)) in a.iter().enumerate() {
        for (j, (_, q)) in b.iter().enumerate() {
            let d = distance_squared(*p, *q);
            let better = match best {
                None => true,
                Some((current, _, _)) => d < current,
            };
            if better {
                best = Some((d, i, j));
            }
        }
    }
    *pairs = pairs.saturating_add((a.len() as u64).saturating_mul(b.len() as u64));
    best
}

/// The connections still to be routed on the board, net by net.
pub fn ratsnest(board: &Board, settings: &RouteSettings) -> Result<Vec<RatLine>, RouteError> {
    let report = board.connectivity();
    let mut lines: Vec<RatLine> = Vec::new();
    let mut pairs = 0_u64;
    for net in &report.nets {
        if net.islands.len() < 2 {
            continue;
        }
        let islands: Vec<Vec<(Item, Point)>> = net
            .islands
            .iter()
            .map(|island| {
                let mut list = Vec::new();
                for item in &island.items {
                    for point in anchors_of(board, *item) {
                        list.push((*item, point));
                    }
                }
                list
            })
            .collect();
        let count = islands.len();
        let mut in_tree = vec![false; count];
        // for every island outside the tree: the squared distance, the island of the tree and the
        // two anchors of the closest pair
        let mut best: Vec<Option<(i128, usize, usize, usize)>> = vec![None; count];
        in_tree[0] = true;
        let mut last = 0_usize;
        for _ in 1..count {
            for other in 0..count {
                if in_tree[other] {
                    continue;
                }
                if let Some((d, i, j)) = closest(&islands[last], &islands[other], &mut pairs) {
                    let better = match best[other] {
                        None => true,
                        Some((current, _, _, _)) => d < current,
                    };
                    if better {
                        best[other] = Some((d, last, i, j));
                    }
                }
            }
            if pairs > settings.ratsnest_max_pairs {
                return Err(RouteError::new(
                    LIMIT_EXCEEDED,
                    "the ratsnest compared more pairs of points than ratsnest_max_pairs",
                ));
            }
            let mut pick: Option<(i128, usize)> = None;
            for other in 0..count {
                if in_tree[other] {
                    continue;
                }
                if let Some((d, _, _, _)) = best[other] {
                    let better = match pick {
                        None => true,
                        Some((current, _)) => d < current,
                    };
                    if better {
                        pick = Some((d, other));
                    }
                }
            }
            let Some((d, next)) = pick else {
                break;
            };
            let Some((_, from_island, i, j)) = best[next] else {
                break;
            };
            let (from, from_at) = islands[from_island][i];
            let (to, to_at) = islands[next][j];
            lines.push(RatLine {
                net: net.net,
                from,
                from_at,
                to,
                to_at,
                length_nm: i64::try_from(isqrt(d)).unwrap_or(i64::MAX),
            });
            in_tree[next] = true;
            last = next;
        }
    }
    Ok(lines)
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// The lines as text, one per line: the net, the two ends and the length.
pub fn to_text(board: &Board, lines: &[RatLine]) -> String {
    let mut out = format!("connections to route: {}\n", lines.len());
    for line in lines {
        let net = board
            .nets
            .get(&line.net)
            .map_or_else(|| line.net.to_string(), |n| n.name.clone());
        out.push_str(&format!(
            "  net {net}: {} -> {} ({} nm)\n",
            item_label(board, line.from),
            item_label(board, line.to),
            line.length_nm
        ));
    }
    out
}

/// The lines as JSON, in the same order.
pub fn to_json(board: &Board, lines: &[RatLine]) -> String {
    let mut out = String::from("{\"lines\":[");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let net = board
            .nets
            .get(&line.net)
            .map_or_else(|| line.net.to_string(), |n| n.name.clone());
        let (fa, ta) = (pair(line.from_at), pair(line.to_at));
        out.push_str(&format!(
            "{{\"net\":\"{}\",\"from\":\"{}\",\"to\":\"{}\",\"from_nm\":[{},{}],\"to_nm\":[{},{}],\"length_nm\":{}}}",
            escape(&net),
            escape(&item_label(board, line.from)),
            escape(&item_label(board, line.to)),
            fa.0,
            fa.1,
            ta.0,
            ta.1,
            line.length_nm
        ));
    }
    out.push_str("]}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_in_json_is_escaped() {
        assert_eq!(escape("a\"b\\c\n"), "a\\\"b\\\\c\\u000a");
        assert_eq!(escape("plain"), "plain");
    }
}
