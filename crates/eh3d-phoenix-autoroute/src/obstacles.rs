// SPDX-License-Identifier: GPL-3.0-or-later
//! The obstacle map: the copper a new track has to keep away from, one list per copper layer and
//! one for the drill holes. The copper that was on the board is in a spatial index that is built
//! once; the copper that the run itself puts down is in a map of squares that can take a route
//! away again (rip-up).
//!
//! Every obstacle is a few straight edges widened by a half width (a round pad or a via is an edge
//! of one point), and, for a rectangular pad or a zone, a filled polygon. Distances are compared
//! exactly (see `geom`), so the answer "clear" is always proven and a doubt counts as blocked.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_spatial::{Index, SpatialConfig};

use crate::codes::{AutorouteError, GRID_TOO_LARGE};
use crate::geom::{half_up, inside_polygon, isqrt_ceil, pair, seg_seg_ge, P};

/// One obstacle.
#[derive(Debug, Clone)]
pub(crate) struct Obs {
    /// The net of the copper; none for the edge of the board and for a pad of no net.
    pub net: Option<Id>,
    /// The clearance this copper asks for.
    pub gap: i64,
    /// The half width the edges are widened by.
    pub half: i64,
    /// True for a pad, which a via of its own net keeps away from too.
    pub pad: bool,
    /// True for a drill hole, which every net keeps away from.
    pub hole: bool,
    /// The edges.
    pub edges: Vec<(P, P)>,
    /// The corners of a filled polygon; empty when the obstacle has no inside.
    pub fill: Vec<P>,
}

impl Obs {
    /// A round obstacle at one point.
    pub(crate) fn round(net: Option<Id>, c: P, half: i64, gap: i64) -> Obs {
        Obs {
            net,
            gap,
            half,
            pad: false,
            hole: false,
            edges: vec![(c, c)],
            fill: Vec::new(),
        }
    }

    /// A piece of track.
    pub(crate) fn piece(net: Id, a: P, b: P, half: i64, gap: i64) -> Obs {
        Obs {
            net: Some(net),
            gap,
            half,
            pad: false,
            hole: false,
            edges: vec![(a, b)],
            fill: Vec::new(),
        }
    }

    /// The box that holds the edges, widened by the half width: `[x0, y0, x1, y1]`.
    pub(crate) fn bbox(&self) -> [i64; 4] {
        let mut b = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
        for (p, q) in &self.edges {
            b[0] = b[0].min(p.0.min(q.0));
            b[1] = b[1].min(p.1.min(q.1));
            b[2] = b[2].max(p.0.max(q.0));
            b[3] = b[3].max(p.1.max(q.1));
        }
        [
            b[0].saturating_sub(self.half),
            b[1].saturating_sub(self.half),
            b[2].saturating_add(self.half),
            b[3].saturating_add(self.half),
        ]
    }

    /// True when a track `ab` of half width `q_half` that asks for the clearance `q_gap` would be
    /// too close. With `ignore_gap` only the copper itself is kept away from.
    pub(crate) fn blocks(&self, a: P, b: P, q_half: i64, q_gap: i64, ignore_gap: bool) -> bool {
        let gap = if ignore_gap { 0 } else { q_gap.max(self.gap) };
        let reach = i128::from(q_half) + i128::from(gap) + i128::from(self.half);
        for (c, d) in &self.edges {
            if !seg_seg_ge(a, b, *c, *d, reach) {
                return true;
            }
        }
        !self.fill.is_empty() && (inside_polygon(a, &self.fill) || inside_polygon(b, &self.fill))
    }
}

/// The obstacles of one layer that were on the board, in an index.
pub(crate) struct StaticLayer {
    obs: Vec<Obs>,
    index: Index,
    max_gap: i64,
    max_half: i64,
}

impl StaticLayer {
    pub(crate) fn new(obs: Vec<Obs>) -> Result<StaticLayer, AutorouteError> {
        let boxes: Vec<[i128; 4]> = obs
            .iter()
            .map(|o| {
                let b = o.bbox();
                [
                    i128::from(b[0]),
                    i128::from(b[1]),
                    i128::from(b[2]),
                    i128::from(b[3]),
                ]
            })
            .collect();
        let index = Index::grid(&boxes, &SpatialConfig::default())
            .map_err(|e| AutorouteError::new(GRID_TOO_LARGE, e.to_string()))?;
        let max_gap = obs.iter().map(|o| o.gap).max().unwrap_or(0);
        let max_half = obs.iter().map(|o| o.half).max().unwrap_or(0);
        Ok(StaticLayer {
            obs,
            index,
            max_gap,
            max_half,
        })
    }

    fn near(&self, a: P, b: P, q_half: i64, q_gap: i64) -> Vec<usize> {
        let m =
            i128::from(q_half) + i128::from(q_gap.max(self.max_gap)) + i128::from(self.max_half);
        self.index.candidates(&[
            i128::from(a.0.min(b.0)) - m,
            i128::from(a.1.min(b.1)) - m,
            i128::from(a.0.max(b.0)) + m,
            i128::from(a.1.max(b.1)) + m,
        ])
    }
}

/// The obstacles of one layer that the run puts down, in squares.
pub(crate) struct DynLayer {
    items: Vec<Option<Obs>>,
    buckets: BTreeMap<(i64, i64), Vec<usize>>,
    cell: i64,
    max_gap: i64,
    max_half: i64,
}

impl DynLayer {
    pub(crate) fn new(cell: i64) -> DynLayer {
        DynLayer {
            items: Vec::new(),
            buckets: BTreeMap::new(),
            cell: cell.max(1),
            max_gap: 0,
            max_half: 0,
        }
    }

    fn cells(&self, b: [i64; 4]) -> (i64, i64, i64, i64) {
        (
            b[0].div_euclid(self.cell),
            b[1].div_euclid(self.cell),
            b[2].div_euclid(self.cell),
            b[3].div_euclid(self.cell),
        )
    }

    /// Puts an obstacle down and returns its number.
    pub(crate) fn add(&mut self, obs: Obs) -> usize {
        let id = self.items.len();
        let (x0, y0, x1, y1) = self.cells(obs.bbox());
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                self.buckets.entry((cx, cy)).or_default().push(id);
            }
        }
        self.max_gap = self.max_gap.max(obs.gap);
        self.max_half = self.max_half.max(obs.half);
        self.items.push(Some(obs));
        id
    }

    /// Takes an obstacle away.
    pub(crate) fn remove(&mut self, id: usize) {
        let Some(obs) = self.items.get_mut(id).and_then(Option::take) else {
            return;
        };
        let (x0, y0, x1, y1) = self.cells(obs.bbox());
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                if let Some(list) = self.buckets.get_mut(&(cx, cy)) {
                    list.retain(|i| *i != id);
                }
            }
        }
    }

    fn near(&self, a: P, b: P, q_half: i64, q_gap: i64) -> Vec<usize> {
        let m = q_half
            .saturating_add(q_gap.max(self.max_gap))
            .saturating_add(self.max_half);
        let (x0, y0, x1, y1) = self.cells([
            a.0.min(b.0).saturating_sub(m),
            a.1.min(b.1).saturating_sub(m),
            a.0.max(b.0).saturating_add(m),
            a.1.max(b.1).saturating_add(m),
        ]);
        let mut found: Vec<usize> = Vec::new();
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                if let Some(list) = self.buckets.get(&(cx, cy)) {
                    found.extend_from_slice(list);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }
}

/// What the answer to "may a track go here" is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    /// Nothing is in the way.
    Free,
    /// Only copper that this run put down is in the way, which rip-up may take away.
    Soft,
    /// Something that cannot be moved is in the way.
    Blocked,
}

/// The net being routed: its half width and its clearance.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Spec {
    pub net: Id,
    pub half: i64,
    pub gap: i64,
}

/// The size of a via and the spacing of its hole.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViaSpec {
    pub half: i64,
    pub drill_half: i64,
    pub drill_gap: i64,
    pub in_pad: bool,
}

/// All the obstacles of a run.
pub(crate) struct World {
    outline: Vec<P>,
    stat: Vec<StaticLayer>,
    dynamic: Vec<DynLayer>,
}

impl World {
    pub(crate) fn new(outline: Vec<P>, stat: Vec<StaticLayer>, cell: i64) -> World {
        let dynamic = (0..stat.len()).map(|_| DynLayer::new(cell)).collect();
        World {
            outline,
            stat,
            dynamic,
        }
    }

    /// The number of copper layers; the list of the holes comes after them.
    pub(crate) fn copper_layers(&self) -> usize {
        self.stat.len().saturating_sub(1)
    }

    /// Puts an obstacle of the run down on a layer (the holes are the layer after the copper).
    pub(crate) fn put(&mut self, layer: usize, obs: Obs) -> usize {
        self.dynamic[layer].add(obs)
    }

    /// Takes an obstacle of the run away.
    pub(crate) fn take(&mut self, layer: usize, id: usize) {
        self.dynamic[layer].remove(id);
    }

    fn inside(&self, a: P, b: P) -> bool {
        inside_polygon(a, &self.outline) && inside_polygon(b, &self.outline)
    }

    /// May a track `ab` of this net go on this layer?
    pub(crate) fn edge(
        &self,
        layer: usize,
        a: P,
        b: P,
        spec: &Spec,
        soft: bool,
        own_pads_block: bool,
    ) -> Edge {
        if layer < self.copper_layers() && !self.inside(a, b) {
            return Edge::Blocked;
        }
        let stat = &self.stat[layer];
        for i in stat.near(a, b, spec.half, spec.gap) {
            let obs = &stat.obs[i];
            let same = !obs.hole && obs.net == Some(spec.net);
            if same && !(own_pads_block && obs.pad) {
                continue;
            }
            if obs.blocks(a, b, spec.half, spec.gap, same) {
                return Edge::Blocked;
            }
        }
        let dynamic = &self.dynamic[layer];
        let mut result = Edge::Free;
        for i in dynamic.near(a, b, spec.half, spec.gap) {
            let Some(obs) = &dynamic.items[i] else {
                continue;
            };
            if !obs.hole && obs.net == Some(spec.net) {
                continue;
            }
            if obs.blocks(a, b, spec.half, spec.gap, false) {
                if !soft {
                    return Edge::Blocked;
                }
                result = Edge::Soft;
            }
        }
        result
    }

    /// May a via of this net go at `p`, joining the layers `lo` to `hi` of the stack?
    pub(crate) fn via(
        &self,
        p: P,
        lo: usize,
        hi: usize,
        via: &ViaSpec,
        spec: &Spec,
        soft: bool,
    ) -> Edge {
        let mut worst = Edge::Free;
        let on_copper = Spec {
            net: spec.net,
            half: via.half,
            gap: spec.gap,
        };
        for layer in lo..=hi {
            match self.edge(layer, p, p, &on_copper, soft, !via.in_pad) {
                Edge::Blocked => return Edge::Blocked,
                Edge::Soft => worst = Edge::Soft,
                Edge::Free => {}
            }
        }
        let hole = Spec {
            net: spec.net,
            half: via.drill_half,
            gap: via.drill_gap,
        };
        match self.edge(self.copper_layers(), p, p, &hole, soft, false) {
            Edge::Blocked => Edge::Blocked,
            Edge::Soft => Edge::Soft,
            Edge::Free => worst,
        }
    }

    /// The nets of the run that are in the way of a track `ab` of this net on this layer.
    pub(crate) fn blockers(&self, layer: usize, a: P, b: P, spec: &Spec, out: &mut BTreeSet<Id>) {
        let dynamic = &self.dynamic[layer];
        for i in dynamic.near(a, b, spec.half, spec.gap) {
            let Some(obs) = &dynamic.items[i] else {
                continue;
            };
            let Some(other) = obs.net else {
                continue;
            };
            if other != spec.net && obs.blocks(a, b, spec.half, spec.gap, false) {
                out.insert(other);
            }
        }
    }
}

/// How the obstacles are made.
pub(crate) struct BuildParams<'a> {
    /// The clearance of copper that asks for no more.
    pub base_gap: i64,
    /// The distance the edge of the board is kept away from.
    pub edge_gap: i64,
    /// Whether the zones of other nets are obstacles.
    pub zones_block: bool,
    /// The clearance each net asks for.
    pub net_gap: &'a BTreeMap<Id, i64>,
    /// The spacing between the walls of two holes.
    pub drill_gap: i64,
}

fn gap_of(params: &BuildParams<'_>, net: Option<Id>) -> i64 {
    net.and_then(|n| params.net_gap.get(&n).copied())
        .unwrap_or(params.base_gap)
        .max(params.base_gap)
}

fn closed(points: &[P]) -> Vec<P> {
    let mut out = points.to_vec();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

fn polygon_edges(points: &[P]) -> Vec<(P, P)> {
    let n = points.len();
    (0..n).map(|i| (points[i], points[(i + 1) % n])).collect()
}

/// The obstacle of a pad: a circle, or the rectangle itself when the pad is turned by a right
/// angle, or the circle around it for any other angle.
fn pad_obstacle(shape: PadShape, total_udeg: i64, c: P, net: Option<Id>, gap: i64) -> Obs {
    let (w, h) = match shape {
        PadShape::Circle { diameter } => {
            let mut obs = Obs::round(net, c, half_up(diameter.get()), gap);
            obs.pad = true;
            return obs;
        }
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            (width.get(), height.get())
        }
    };
    let turned = total_udeg.rem_euclid(360_000_000);
    if turned % 90_000_000 != 0 {
        let (w2, h2) = (i128::from(w), i128::from(h));
        let r = i64::try_from(half_up_i128(isqrt_ceil(w2 * w2 + h2 * h2))).unwrap_or(i64::MAX);
        let mut obs = Obs::round(net, c, r, gap);
        obs.pad = true;
        return obs;
    }
    let (w, h) = if (turned / 90_000_000) % 2 == 1 {
        (h, w)
    } else {
        (w, h)
    };
    let (hx, hy) = (half_up(w), half_up(h));
    let corners = vec![
        (c.0 - hx, c.1 - hy),
        (c.0 + hx, c.1 - hy),
        (c.0 + hx, c.1 + hy),
        (c.0 - hx, c.1 + hy),
    ];
    Obs {
        net,
        gap,
        half: 0,
        pad: true,
        hole: false,
        edges: polygon_edges(&corners),
        fill: corners,
    }
}

fn half_up_i128(v: i128) -> i128 {
    v / 2 + v % 2
}

/// The obstacle of a piece of an arc: the disk that has the chord as its diameter holds an arc of
/// up to half a turn; a longer arc is held by a disk twice the radius of its circle, worked out
/// once with floating point.
fn arc_obstacle(net: Id, start: P, end: P, sweep_udeg: i64, half: i64, gap: i64) -> Obs {
    let mid = (
        start.0 + (end.0 - start.0) / 2,
        start.1 + (end.1 - start.1) / 2,
    );
    let (dx, dy) = (
        i128::from(end.0) - i128::from(start.0),
        i128::from(end.1) - i128::from(start.1),
    );
    let chord = isqrt_ceil(dx * dx + dy * dy);
    let mut radius = half_up_i128(chord) + 1;
    if sweep_udeg.abs() > 180_000_000 {
        let theta = (sweep_udeg.abs() as f64) / 1.0e6_f64 * std::f64::consts::PI / 180.0;
        let s = (theta / 2.0).sin().abs().max(1.0e-3);
        let big = (chord as f64) / (2.0 * s);
        radius = radius.max((2.0 * big).ceil() as i128 + 1);
    }
    let radius = i64::try_from(radius).unwrap_or(i64::MAX / 4);
    Obs::round(Some(net), mid, radius.saturating_add(half), gap)
}

/// The obstacles of a board: one list per copper layer in the order of `copper`, then the holes.
/// `copper` is the stack from top to bottom.
pub(crate) fn build(
    board: &Board,
    copper: &[Id],
    params: &BuildParams<'_>,
    cell: i64,
) -> Result<World, AutorouteError> {
    let layers = copper.len();
    let mut lists: Vec<Vec<Obs>> = (0..=layers).map(|_| Vec::new()).collect();
    let index_of = |layer: Id| copper.iter().position(|l| *l == layer);
    let outline: Vec<P> = closed(&board.outline.iter().map(|p| pair(*p)).collect::<Vec<P>>());
    for list in lists.iter_mut().take(layers) {
        list.push(Obs {
            net: None,
            gap: params.edge_gap,
            half: 0,
            pad: false,
            hole: false,
            edges: polygon_edges(&outline),
            fill: Vec::new(),
        });
    }
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let c = pair(at);
            let net = board.net_of(pad_ref);
            let gap = gap_of(params, net);
            let total = pad.rotation.get() + component.rotation.get();
            let obs = pad_obstacle(pad.shape, total, c, net, gap);
            let on: Vec<usize> = if pad.drill.is_some() {
                (0..layers).collect()
            } else {
                pad.layers.iter().filter_map(|l| index_of(*l)).collect()
            };
            for layer in on {
                lists[layer].push(obs.clone());
            }
            if let Some(drill) = pad.drill {
                let mut hole = Obs::round(None, c, half_up(drill.get()), params.drill_gap);
                hole.hole = true;
                lists[layers].push(hole);
            }
        }
    }
    for track in board.tracks.values() {
        let Some(layer) = index_of(track.layer) else {
            continue;
        };
        let half = half_up(track.width.get());
        let gap = gap_of(params, Some(track.net));
        for segment in &track.path {
            let obs = match segment {
                Segment::Line { start, end } => {
                    Obs::piece(track.net, pair(*start), pair(*end), half, gap)
                }
                Segment::Arc { start, end, sweep } => {
                    arc_obstacle(track.net, pair(*start), pair(*end), sweep.get(), half, gap)
                }
            };
            lists[layer].push(obs);
        }
    }
    for via in board.vias.values() {
        let lo = index_of(via.from_layer);
        let hi = index_of(via.to_layer);
        let (lo, hi) = match (lo, hi) {
            (Some(a), Some(b)) => (a.min(b), a.max(b)),
            _ => (0, layers.saturating_sub(1)),
        };
        let c = pair(via.at);
        let gap = gap_of(params, Some(via.net));
        for list in lists.iter_mut().take(hi + 1).skip(lo) {
            list.push(Obs::round(
                Some(via.net),
                c,
                half_up(via.diameter.get()),
                gap,
            ));
        }
        let mut hole = Obs::round(Some(via.net), c, half_up(via.drill.get()), params.drill_gap);
        hole.hole = true;
        lists[layers].push(hole);
    }
    if params.zones_block {
        for zone in board.zones.values() {
            let Some(layer) = index_of(zone.layer) else {
                continue;
            };
            let points = closed(
                &zone
                    .outline
                    .iter()
                    .map(|p: &Point| pair(*p))
                    .collect::<Vec<P>>(),
            );
            if points.len() < 3 {
                continue;
            }
            let gap = zone.clearance.get().max(gap_of(params, Some(zone.net)));
            lists[layer].push(Obs {
                net: Some(zone.net),
                gap,
                half: 0,
                pad: false,
                hole: false,
                edges: polygon_edges(&points),
                fill: points,
            });
        }
    }
    let mut stat = Vec::with_capacity(layers + 1);
    for list in lists {
        stat.push(StaticLayer::new(list)?);
    }
    Ok(World::new(outline, stat, cell))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u16) -> Id {
        eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    #[test]
    fn a_round_obstacle_keeps_a_track_away_by_its_radius_the_clearance_and_the_half_width() {
        // radius 300, clearance 200, track half width 125: the centre line stays 625 away
        let obs = Obs::round(Some(id(1)), (0, 0), 300, 200);
        assert!(obs.blocks((624, -5000), (624, 5000), 125, 200, false));
        assert!(!obs.blocks((625, -5000), (625, 5000), 125, 200, false));
        // a larger clearance of the track wins over the one of the obstacle
        assert!(obs.blocks((700, -5000), (700, 5000), 125, 300, false));
        // with the clearance ignored only the copper counts
        assert!(!obs.blocks((425, -5000), (425, 5000), 125, 200, true));
        assert!(obs.blocks((424, -5000), (424, 5000), 125, 200, true));
    }

    #[test]
    fn a_filled_polygon_blocks_a_track_inside_it() {
        let square = vec![(0, 0), (1000, 0), (1000, 1000), (0, 1000)];
        let obs = Obs {
            net: Some(id(1)),
            gap: 10,
            half: 0,
            pad: false,
            hole: false,
            edges: polygon_edges(&square),
            fill: square,
        };
        assert!(obs.blocks((400, 400), (600, 600), 5, 10, false));
        assert!(!obs.blocks((2000, 400), (2000, 600), 5, 10, false));
        assert_eq!(obs.bbox(), [0, 0, 1000, 1000]);
    }

    #[test]
    fn a_pad_turned_by_a_right_angle_is_a_rectangle_and_by_another_angle_a_circle() {
        let shape = PadShape::Rect {
            width: eh3d_phoenix_model::Nm::new(1000).unwrap(),
            height: eh3d_phoenix_model::Nm::new(400).unwrap(),
        };
        let flat = pad_obstacle(shape, 0, (0, 0), None, 0);
        assert_eq!(flat.bbox(), [-500, -200, 500, 200]);
        let upright = pad_obstacle(shape, 90_000_000, (0, 0), None, 0);
        assert_eq!(upright.bbox(), [-200, -500, 200, 500]);
        let turned = pad_obstacle(shape, 30_000_000, (0, 0), None, 0);
        assert!(turned.fill.is_empty());
        assert!(turned.half >= 539);
        assert!(flat.pad && upright.pad && turned.pad);
    }

    #[test]
    fn the_layer_of_the_run_takes_a_piece_away_and_the_square_forgets_it() {
        let mut layer = DynLayer::new(1000);
        let a = layer.add(Obs::piece(id(1), (0, 0), (5000, 0), 100, 100));
        let b = layer.add(Obs::piece(id(2), (0, 3000), (5000, 3000), 100, 100));
        assert_eq!(layer.near((2000, 50), (2000, 60), 100, 100), vec![a]);
        layer.remove(a);
        assert!(layer.near((2000, 50), (2000, 60), 100, 100).is_empty());
        assert_eq!(layer.near((2000, 3050), (2000, 3060), 100, 100), vec![b]);
        layer.remove(a);
    }

    #[test]
    fn the_answer_tells_what_is_fixed_from_what_the_run_put_down() {
        let outline = vec![(0, 0), (100_000, 0), (100_000, 100_000), (0, 100_000)];
        let fixed =
            StaticLayer::new(vec![Obs::round(Some(id(9)), (50_000, 50_000), 1000, 100)]).unwrap();
        let holes = StaticLayer::new(Vec::new()).unwrap();
        let mut world = World::new(outline, vec![fixed, holes], 10_000);
        let spec = Spec {
            net: id(1),
            half: 100,
            gap: 100,
        };
        // the fixed round pad is in the way of a track through it
        let across = |w: &World, soft: bool| {
            w.edge(0, (40_000, 50_000), (60_000, 50_000), &spec, soft, false)
        };
        assert_eq!(across(&world, false), Edge::Blocked);
        assert_eq!(across(&world, true), Edge::Blocked);
        // a track that keeps away is free, one outside the outline is blocked
        assert_eq!(
            world.edge(0, (40_000, 10_000), (60_000, 10_000), &spec, false, false),
            Edge::Free
        );
        assert_eq!(
            world.edge(0, (-10, 10_000), (60_000, 10_000), &spec, false, false),
            Edge::Blocked
        );
        // copper of the run is soft when rip-up looks for it, blocked otherwise
        let put = world.put(
            0,
            Obs::piece(id(2), (50_000, 10_000), (50_000, 20_000), 100, 100),
        );
        let free_line = |w: &World, soft: bool| {
            w.edge(0, (40_000, 15_000), (60_000, 15_000), &spec, soft, false)
        };
        assert_eq!(free_line(&world, false), Edge::Blocked);
        assert_eq!(free_line(&world, true), Edge::Soft);
        let mut nets = BTreeSet::new();
        world.blockers(0, (40_000, 15_000), (60_000, 15_000), &spec, &mut nets);
        assert_eq!(nets.into_iter().collect::<Vec<_>>(), vec![id(2)]);
        // the same net does not block itself
        let same = Spec { net: id(2), ..spec };
        assert_eq!(
            world.edge(0, (40_000, 15_000), (60_000, 15_000), &same, false, false),
            Edge::Free
        );
        world.take(0, put);
        assert_eq!(free_line(&world, false), Edge::Free);
    }
}
