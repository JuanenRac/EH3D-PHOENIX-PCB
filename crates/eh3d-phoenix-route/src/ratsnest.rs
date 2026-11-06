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
use eh3d_phoenix_rules::IndexMode;
use eh3d_phoenix_spatial::{inflate, Bx, Index, SpatialConfig};

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

/// An island with more anchors than this gets a spatial index; a smaller one is compared point
/// by point, which is as fast.
const SMALL_ISLAND: usize = 24;

/// The work of a ratsnest, counted and not timed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RatsnestWork {
    /// The nets with two islands or more.
    pub nets: usize,
    /// The islands of those nets.
    pub islands: usize,
    /// The anchor points of those islands.
    pub anchors: usize,
    /// The distances between two points that were measured.
    pub comparisons: u64,
}

/// The anchors of an island and, when it has many, the index that finds the nearest one.
struct Anchors {
    list: Vec<(Item, Point)>,
    index: Option<Index>,
}

fn point_box(p: Point) -> Bx {
    let (x, y) = (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
    [x, y, x, y]
}

impl Anchors {
    fn new(list: Vec<(Item, Point)>, mode: IndexMode) -> Anchors {
        let index = if mode == IndexMode::Grid && list.len() > SMALL_ISLAND {
            let boxes: Vec<Bx> = list.iter().map(|(_, p)| point_box(*p)).collect();
            // a cell as wide as the points are apart if they were spread along their longest side
            let (mut lo, mut hi) = ([i128::MAX; 2], [i128::MIN; 2]);
            for b in &boxes {
                lo = [lo[0].min(b[0]), lo[1].min(b[1])];
                hi = [hi[0].max(b[0]), hi[1].max(b[1])];
            }
            let extent = (hi[0] - lo[0]).max(hi[1] - lo[1]);
            let cell = (extent / list.len() as i128).max(1);
            let config = SpatialConfig {
                cell_nm: i64::try_from(cell).unwrap_or(i64::MAX),
                ..SpatialConfig::default()
            };
            Index::grid(&boxes, &config).ok()
        } else {
            None
        };
        Anchors { list, index }
    }
}

/// The anchor of `list` nearest to `from`, as the squared distance and the place in the list; of
/// several at the same distance, the one that comes first. The square around the point grows
/// until the nearest anchor found is inside it, so no anchor nearer can have been missed.
fn nearest(
    index: &Index,
    list: &[(Item, Point)],
    from: Point,
    evals: &mut u64,
) -> Option<(i128, usize)> {
    let query = point_box(from);
    let mut radius = index.cell_nm().max(1);
    loop {
        let found = index.candidates(&inflate(&query, radius));
        *evals = evals.saturating_add(found.len() as u64);
        let mut best: Option<(i128, usize)> = None;
        for &j in &found {
            let d = distance_squared(from, list[j].1);
            if best.is_none_or(|(current, _)| d < current) {
                best = Some((d, j));
            }
        }
        let everything = found.len() == index.count();
        match best {
            Some((d, j)) if everything || d <= radius.saturating_mul(radius) => {
                return Some((d, j));
            }
            None if everything => return None,
            _ => radius = radius.saturating_mul(2),
        }
    }
}

/// The closest pair of anchors of two islands, the second with an index: the same pair as
/// [`closest`] gives.
fn closest_indexed(
    a: &[(Item, Point)],
    b: &Anchors,
    index: &Index,
    evals: &mut u64,
) -> Option<(i128, usize, usize)> {
    let mut best: Option<(i128, usize, usize)> = None;
    for (i, (_, p)) in a.iter().enumerate() {
        if let Some((d, j)) = nearest(index, &b.list, *p, evals) {
            if best.is_none_or(|(current, _, _)| d < current) {
                best = Some((d, i, j));
            }
        }
    }
    best
}

/// The connections still to be routed on the board, net by net.
pub fn ratsnest(board: &Board, settings: &RouteSettings) -> Result<Vec<RatLine>, RouteError> {
    ratsnest_with(board, settings, IndexMode::Grid).map(|(lines, _)| lines)
}

/// The connections still to be routed, with the chosen way of finding the closest anchors of two
/// islands (the lines are the same either way) and the work it took.
pub fn ratsnest_with(
    board: &Board,
    settings: &RouteSettings,
    mode: IndexMode,
) -> Result<(Vec<RatLine>, RatsnestWork), RouteError> {
    let report = board.connectivity();
    let mut lines: Vec<RatLine> = Vec::new();
    let mut pairs = 0_u64;
    let mut work = RatsnestWork::default();
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
        work.nets += 1;
        work.islands += count;
        work.anchors += islands.iter().map(Vec::len).sum::<usize>();
        let islands: Vec<Anchors> = islands
            .into_iter()
            .map(|list| Anchors::new(list, mode))
            .collect();
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
                let found = match &islands[other].index {
                    Some(index) => {
                        let size = (islands[last].list.len() as u64)
                            .saturating_mul(islands[other].list.len() as u64);
                        pairs = pairs.saturating_add(size);
                        closest_indexed(
                            &islands[last].list,
                            &islands[other],
                            index,
                            &mut work.comparisons,
                        )
                    }
                    None => {
                        let before = pairs;
                        let found = closest(&islands[last].list, &islands[other].list, &mut pairs);
                        work.comparisons = work.comparisons.saturating_add(pairs - before);
                        found
                    }
                };
                if let Some((d, i, j)) = found {
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
            let (from, from_at) = islands[from_island].list[i];
            let (to, to_at) = islands[next].list[j];
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
    Ok((lines, work))
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
