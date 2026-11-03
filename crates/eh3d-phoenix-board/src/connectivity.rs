//! Which copper of each net is joined to which: the islands of a net.
//!
//! A net is whole when all its pads, tracks, vias and zones form one island. Two items are joined
//! when they touch on a layer both are on:
//!
//! * a track piece touches a pad, a via or another track where an end point of one lies within the
//!   other (inside the pad shape, inside the pad of the via, within half the width of the other
//!   track);
//! * a via joins the two layers it starts and ends on;
//! * a zone touches whatever of its net lies inside its outline on its layer.
//!
//! Exact integer arithmetic is used throughout. What the check cannot judge (arcs are judged by
//! their end points only) is reported as information, never as a failure. A net with several
//! islands is not an error while a board is being designed: it is the list of connections still
//! to route, reported as information with code `PHOENIX-BOARD-010`.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{Point, Udeg};

use crate::model::{Board, Id, PadRef, PadShape, Segment, Side};

/// A net is split into several islands.
pub const NET_SPLIT: &str = "PHOENIX-BOARD-010";
/// An arc was judged by its end points only.
pub const ARC_BY_ENDS: &str = "PHOENIX-BOARD-011";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (NET_SPLIT, "Net is split into islands"),
    (ARC_BY_ENDS, "Arc judged by its end points only"),
];

/// A part of copper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Item {
    /// A pad of a placed component.
    Pad(PadRef),
    /// A track.
    Track(Id),
    /// A via.
    Via(Id),
    /// A zone.
    Zone(Id),
}

/// The items of a net that are joined together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Island {
    /// The items, in order.
    pub items: Vec<Item>,
}

/// The islands of one net.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetConnectivity {
    /// The net.
    pub net: Id,
    /// Its islands, ordered by their first item.
    pub islands: Vec<Island>,
}

impl NetConnectivity {
    /// How many connections are still missing (islands minus one; zero for an empty net).
    pub fn missing_connections(&self) -> usize {
        self.islands.len().saturating_sub(1)
    }
}

/// The result of the check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectivityReport {
    /// Every net that has copper, ordered by id.
    pub nets: Vec<NetConnectivity>,
    /// Information about split nets and about what could not be judged.
    pub diagnostics: Vec<Diagnostic>,
}

impl ConnectivityReport {
    /// True when every net is one island.
    pub fn is_fully_connected(&self) -> bool {
        self.nets.iter().all(|n| n.islands.len() <= 1)
    }
}

/// Where the centre of a pad is on the board, in nanometres.
///
/// A component on the bottom side is mirrored about the vertical axis before it is rotated. Right
/// angles are exact; other angles use floating point once and round to the nearest nanometre.
pub fn pad_position(board: &Board, pad: PadRef) -> Option<Point> {
    let component = board.components.get(&pad.component)?;
    let footprint = board.footprints.get(&component.footprint)?;
    let p = footprint.pads.get(&pad.pad)?;
    let mut x = p.at.x_nm.get();
    let y = p.at.y_nm.get();
    if component.side == Side::Bottom {
        x = -x;
    }
    let (rx, ry) = rotate(x, y, component.rotation);
    Point::from_nm(
        component.at.x_nm.get().checked_add(rx)?,
        component.at.y_nm.get().checked_add(ry)?,
    )
    .ok()
}

fn rotate(x: i64, y: i64, angle: Udeg) -> (i64, i64) {
    let a = angle.normalised().get();
    match a {
        0 => (x, y),
        90_000_000 => (-y, x),
        180_000_000 => (-x, -y),
        270_000_000 => (y, -x),
        _ => {
            let rad = (a as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
            let (s, c) = rad.sin_cos();
            let fx = x as f64;
            let fy = y as f64;
            (
                (fx * c - fy * s).round() as i64,
                (fx * s + fy * c).round() as i64,
            )
        }
    }
}

/// True when `p` lies inside the shape of `pad`, with the pad centred at `centre`.
fn pad_contains(board: &Board, pad: PadRef, centre: Point, p: Point) -> bool {
    let Some(component) = board.components.get(&pad.component) else {
        return false;
    };
    let Some(footprint) = board.footprints.get(&component.footprint) else {
        return false;
    };
    let Some(shape) = footprint.pads.get(&pad.pad) else {
        return false;
    };
    let total = (shape.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
    let dx = (p.x_nm.get() - centre.x_nm.get()) as i128;
    let dy = (p.y_nm.get() - centre.y_nm.get()) as i128;
    match shape.shape {
        PadShape::Circle { diameter } => {
            let r = diameter.get() as i128;
            // (2 d)^2 <= diameter^2 * ... : compare 4 * dist^2 with diameter^2
            4 * (dx * dx + dy * dy) <= r * r
        }
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            let (w, h) = (width.get() as i128, height.get() as i128);
            match total {
                0 | 180_000_000 => 2 * dx.abs() <= w && 2 * dy.abs() <= h,
                90_000_000 | 270_000_000 => 2 * dx.abs() <= h && 2 * dy.abs() <= w,
                _ => {
                    // Any other angle: the circle that fits inside the shape.
                    let m = w.min(h);
                    4 * (dx * dx + dy * dy) <= m * m
                }
            }
        }
    }
}

/// True when `p` is within `r` (a radius in nanometres, may be half a width) of the segment ab.
fn near_segment(p: Point, a: Point, b: Point, r2x4: i128) -> bool {
    near_segment_exact(p, a, b, r2x4, false).unwrap_or(false)
}

/// Is the distance from `p` to the segment ab at most (`strict` false) or less than (`strict` true)
/// the radius whose square times four is `r2x4`? `None` when the numbers do not fit in 128 bits, so
/// the answer is not known.
pub(crate) fn near_segment_exact(
    p: Point,
    a: Point,
    b: Point,
    r2x4: i128,
    strict: bool,
) -> Option<bool> {
    let cmp = |lhs: i128, rhs: i128| if strict { lhs < rhs } else { lhs <= rhs };
    let (px, py) = (p.x_nm.get() as i128, p.y_nm.get() as i128);
    let (ax, ay) = (a.x_nm.get() as i128, a.y_nm.get() as i128);
    let (bx, by) = (b.x_nm.get() as i128, b.y_nm.get() as i128);
    let (abx, aby) = (bx - ax, by - ay);
    let len2 = abx * abx + aby * aby;
    let t = (px - ax) * abx + (py - ay) * aby;
    if len2 == 0 || t <= 0 {
        let (dx, dy) = (px - ax, py - ay);
        return Some(cmp(4 * (dx * dx + dy * dy), r2x4));
    }
    if t >= len2 {
        let (dx, dy) = (px - bx, py - by);
        return Some(cmp(4 * (dx * dx + dy * dy), r2x4));
    }
    let cross = (px - ax) * aby - (py - ay) * abx;
    // dist^2 = cross^2 / len2, so 4 cross^2 compared with r2x4 * len2
    let lhs = cross.checked_mul(cross)?.checked_mul(4)?;
    let rhs = r2x4.checked_mul(len2)?;
    Some(cmp(lhs, rhs))
}

pub(crate) fn point_in_polygon(p: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let (px, py) = (p.x_nm.get() as i128, p.y_nm.get() as i128);
    let mut inside = false;
    let n = polygon.len();
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        let (ax, ay) = (a.x_nm.get() as i128, a.y_nm.get() as i128);
        let (bx, by) = (b.x_nm.get() as i128, b.y_nm.get() as i128);
        if (ay > py) != (by > py) {
            // x of the edge at height py, compared without division
            let lhs = (px - ax) * (by - ay);
            let rhs = (bx - ax) * (py - ay);
            let right_of_point = if by > ay { lhs < rhs } else { lhs > rhs };
            if right_of_point {
                inside = !inside;
            }
        }
    }
    inside
}

fn track_points(segments: &[Segment]) -> Vec<Point> {
    let mut out = Vec::new();
    for (i, s) in segments.iter().enumerate() {
        if i == 0 {
            out.push(s.start());
        }
        out.push(s.end());
    }
    out
}

fn straight_pieces(segments: &[Segment]) -> Vec<(Point, Point)> {
    segments.iter().map(|s| (s.start(), s.end())).collect()
}

struct Union {
    parent: Vec<usize>,
}

impl Union {
    fn new(n: usize) -> Self {
        Union {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }
    fn join(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            // the smaller index stays the root, so the result does not depend on the order
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[hi] = lo;
        }
    }
}

struct Entry {
    item: Item,
    net: Id,
}

impl Board {
    /// The islands of every net that has copper.
    pub fn connectivity(&self) -> ConnectivityReport {
        let mut entries: Vec<Entry> = Vec::new();
        for pn in &self.pad_nets {
            entries.push(Entry {
                item: Item::Pad(pn.pad_ref()),
                net: pn.net,
            });
        }
        for (id, t) in &self.tracks {
            entries.push(Entry {
                item: Item::Track(*id),
                net: t.net,
            });
        }
        for (id, v) in &self.vias {
            entries.push(Entry {
                item: Item::Via(*id),
                net: v.net,
            });
        }
        for (id, z) in &self.zones {
            entries.push(Entry {
                item: Item::Zone(*id),
                net: z.net,
            });
        }
        entries.sort_by_key(|e| (e.net, e.item));
        let mut uf = Union::new(entries.len());
        let mut saw_arc = false;
        for i in 0..entries.len() {
            if let Item::Track(id) = entries[i].item {
                if self.tracks[&id]
                    .path
                    .iter()
                    .any(|s| matches!(s, Segment::Arc { .. }))
                {
                    saw_arc = true;
                }
            }
            for j in (i + 1)..entries.len() {
                if entries[i].net != entries[j].net {
                    break;
                }
                if self.touch(entries[i].item, entries[j].item) {
                    uf.join(i, j);
                }
            }
        }
        let mut by_net: BTreeMap<Id, BTreeMap<usize, Vec<Item>>> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            let root = uf.find(i);
            by_net
                .entry(entry.net)
                .or_default()
                .entry(root)
                .or_default()
                .push(entry.item);
        }
        let mut nets = Vec::new();
        let mut diagnostics = Vec::new();
        for (net, groups) in by_net {
            let islands: Vec<Island> = groups
                .into_values()
                .map(|mut items| {
                    items.sort();
                    Island { items }
                })
                .collect();
            if islands.len() > 1 {
                let name = self.nets.get(&net).map_or("?", |n| n.name.as_str());
                diagnostics.push(
                    Diagnostic::new(
                        0,
                        Severity::Info,
                        format!(
                            "{}: net {name} has {} islands ({} connections to route)",
                            title(NET_SPLIT),
                            islands.len(),
                            islands.len() - 1
                        ),
                    )
                    .with_code(NET_SPLIT)
                    .with_category(Category::Geometric)
                    .with_recoverability(Recoverability::Recoverable),
                );
            }
            nets.push(NetConnectivity { net, islands });
        }
        if saw_arc {
            diagnostics.push(
                Diagnostic::new(0, Severity::Info, title(ARC_BY_ENDS).to_owned())
                    .with_code(ARC_BY_ENDS)
                    .with_category(Category::Geometric)
                    .with_recoverability(Recoverability::Recoverable),
            );
        }
        ConnectivityReport { nets, diagnostics }
    }

    fn pad_layers(&self, pad: PadRef) -> Option<&[Id]> {
        let c = self.components.get(&pad.component)?;
        let f = self.footprints.get(&c.footprint)?;
        Some(&f.pads.get(&pad.pad)?.layers)
    }

    fn touch(&self, a: Item, b: Item) -> bool {
        use Item::*;
        match (a, b) {
            (Pad(p), Track(t)) | (Track(t), Pad(p)) => self.pad_track(p, t),
            (Pad(p), Via(v)) | (Via(v), Pad(p)) => self.pad_via(p, v),
            (Pad(p), Zone(z)) | (Zone(z), Pad(p)) => self.pad_zone(p, z),
            (Track(x), Track(y)) => self.track_track(x, y),
            (Track(t), Via(v)) | (Via(v), Track(t)) => self.track_via(t, v),
            (Track(t), Zone(z)) | (Zone(z), Track(t)) => self.track_zone(t, z),
            (Via(x), Via(y)) => self.via_via(x, y),
            (Via(v), Zone(z)) | (Zone(z), Via(v)) => self.via_zone(v, z),
            (Zone(x), Zone(y)) => self.zone_zone(x, y),
            (Pad(_), Pad(_)) => false,
        }
    }

    fn pad_track(&self, pad: PadRef, track: Id) -> bool {
        let (Some(layers), Some(centre), Some(t)) = (
            self.pad_layers(pad),
            pad_position(self, pad),
            self.tracks.get(&track),
        ) else {
            return false;
        };
        layers.contains(&t.layer)
            && track_points(&t.path)
                .into_iter()
                .any(|p| pad_contains(self, pad, centre, p))
    }

    fn pad_via(&self, pad: PadRef, via: Id) -> bool {
        let (Some(layers), Some(centre), Some(v)) = (
            self.pad_layers(pad),
            pad_position(self, pad),
            self.vias.get(&via),
        ) else {
            return false;
        };
        (layers.contains(&v.from_layer) || layers.contains(&v.to_layer))
            && pad_contains(self, pad, centre, v.at)
    }

    fn pad_zone(&self, pad: PadRef, zone: Id) -> bool {
        let (Some(layers), Some(centre), Some(z)) = (
            self.pad_layers(pad),
            pad_position(self, pad),
            self.zones.get(&zone),
        ) else {
            return false;
        };
        layers.contains(&z.layer) && point_in_polygon(centre, &z.outline)
    }

    fn track_track(&self, x: Id, y: Id) -> bool {
        let (Some(a), Some(b)) = (self.tracks.get(&x), self.tracks.get(&y)) else {
            return false;
        };
        if a.layer != b.layer {
            return false;
        }
        // 4 r^2 with r = half of the wider track: (max width)^2
        let w = a.width.get().max(b.width.get()) as i128;
        let r2x4 = w * w;
        let ends_a = track_points(&a.path);
        let ends_b = track_points(&b.path);
        let pieces_a = straight_pieces(&a.path);
        let pieces_b = straight_pieces(&b.path);
        ends_a
            .iter()
            .any(|p| pieces_b.iter().any(|(s, e)| near_segment(*p, *s, *e, r2x4)))
            || ends_b
                .iter()
                .any(|p| pieces_a.iter().any(|(s, e)| near_segment(*p, *s, *e, r2x4)))
    }

    fn track_via(&self, track: Id, via: Id) -> bool {
        let (Some(t), Some(v)) = (self.tracks.get(&track), self.vias.get(&via)) else {
            return false;
        };
        if t.layer != v.from_layer && t.layer != v.to_layer {
            return false;
        }
        let d = v.diameter.get() as i128;
        let r2x4 = d * d;
        straight_pieces(&t.path)
            .iter()
            .any(|(s, e)| near_segment(v.at, *s, *e, r2x4))
    }

    fn track_zone(&self, track: Id, zone: Id) -> bool {
        let (Some(t), Some(z)) = (self.tracks.get(&track), self.zones.get(&zone)) else {
            return false;
        };
        t.layer == z.layer
            && track_points(&t.path)
                .into_iter()
                .any(|p| point_in_polygon(p, &z.outline))
    }

    fn via_via(&self, x: Id, y: Id) -> bool {
        let (Some(a), Some(b)) = (self.vias.get(&x), self.vias.get(&y)) else {
            return false;
        };
        let shares = [a.from_layer, a.to_layer]
            .iter()
            .any(|l| *l == b.from_layer || *l == b.to_layer);
        let w = a.diameter.get().max(b.diameter.get()) as i128;
        shares && near_segment(a.at, b.at, b.at, w * w)
    }

    fn via_zone(&self, via: Id, zone: Id) -> bool {
        let (Some(v), Some(z)) = (self.vias.get(&via), self.zones.get(&zone)) else {
            return false;
        };
        (z.layer == v.from_layer || z.layer == v.to_layer) && point_in_polygon(v.at, &z.outline)
    }

    fn zone_zone(&self, x: Id, y: Id) -> bool {
        let (Some(a), Some(b)) = (self.zones.get(&x), self.zones.get(&y)) else {
            return false;
        };
        a.layer == b.layer
            && (a.outline.iter().any(|p| point_in_polygon(*p, &b.outline))
                || b.outline.iter().any(|p| point_in_polygon(*p, &a.outline)))
    }

    /// The layers a net has copper on (tracks, zones and the ends of vias), as a set of ids.
    pub fn layers_of_net(&self, net: Id) -> BTreeSet<Id> {
        let mut out = BTreeSet::new();
        for t in self.tracks.values().filter(|t| t.net == net) {
            out.insert(t.layer);
        }
        for z in self.zones.values().filter(|z| z.net == net) {
            out.insert(z.layer);
        }
        for v in self.vias.values().filter(|v| v.net == net) {
            out.insert(v.from_layer);
            out.insert(v.to_layer);
        }
        out
    }
}

fn title(code: &str) -> &'static str {
    REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Component, Footprint, Layer, LayerKind, Net, Pad, PadNet, Track, Via, Zone,
    };
    use eh3d_phoenix_model::{Nm, ObjectId};

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    /// Two layers, one net, a footprint with two pads 10 mm apart, one component at the origin.
    fn base() -> (Board, Id, Id, Id, PadRef, PadRef) {
        let (top, bottom, net, fp, comp) = (id(1), id(2), id(3), id(4), id(5));
        let (pad_a, pad_b) = (id(6), id(7));
        let mut b = Board::default();
        for (l, name) in [(top, "Top"), (bottom, "Bottom")] {
            b.layers.insert(
                l,
                Layer {
                    name: name.into(),
                    kind: LayerKind::Copper,
                    side: None,
                },
            );
        }
        b.nets.insert(net, Net { name: "N".into() });
        let mut pads = BTreeMap::new();
        for (pid, x, n) in [(pad_a, 0, "1"), (pad_b, 10_000_000, "2")] {
            pads.insert(
                pid,
                Pad {
                    number: n.into(),
                    shape: PadShape::Circle {
                        diameter: nm(1_600_000),
                    },
                    at: pt(x, 0),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![top],
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
        b.components.insert(
            comp,
            Component {
                refdes: "R1".into(),
                value: "1k".into(),
                symbol: None,
                footprint: fp,
                at: pt(0, 0),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        let (ra, rb) = (
            PadRef {
                component: comp,
                pad: pad_a,
            },
            PadRef {
                component: comp,
                pad: pad_b,
            },
        );
        b.pad_nets = vec![
            PadNet {
                component: comp,
                pad: pad_a,
                net,
            },
            PadNet {
                component: comp,
                pad: pad_b,
                net,
            },
        ];
        b.pad_nets.sort_by_key(|p| p.pad_ref());
        (b, top, bottom, net, ra, rb)
    }

    fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
        Segment::Line {
            start: pt(a.0, a.1),
            end: pt(b.0, b.1),
        }
    }

    #[test]
    fn two_unrouted_pads_are_two_islands_and_one_missing_connection() {
        let (b, _, _, net, _, _) = base();
        let r = b.connectivity();
        assert_eq!(r.nets.len(), 1);
        assert_eq!(r.nets[0].net, net);
        assert_eq!(r.nets[0].islands.len(), 2);
        assert_eq!(r.nets[0].missing_connections(), 1);
        assert!(!r.is_fully_connected());
        assert_eq!(r.diagnostics[0].code.as_deref(), Some(NET_SPLIT));
        assert_eq!(r.diagnostics[0].severity, Severity::Info);
    }

    #[test]
    fn a_track_from_pad_to_pad_joins_them_and_a_gap_does_not() {
        let (mut b, top, _, net, _, _) = base();
        let t = id(20);
        b.tracks.insert(
            t,
            Track {
                net,
                layer: top,
                width: nm(250_000),
                path: vec![
                    line((0, 0), (5_000_000, 0)),
                    line((5_000_000, 0), (10_000_000, 0)),
                ],
            },
        );
        let r = b.connectivity();
        assert!(r.is_fully_connected(), "{:?}", r.nets);
        assert_eq!(r.nets[0].islands[0].items.len(), 3);
        // move the end of the track out of the second pad
        if let Some(track) = b.tracks.get_mut(&t) {
            track.path[1] = line((5_000_000, 0), (8_000_000, 0));
        }
        assert!(!b.connectivity().is_fully_connected());
    }

    #[test]
    fn a_track_on_the_wrong_layer_does_not_touch_the_pad() {
        let (mut b, _, bottom, net, _, _) = base();
        b.tracks.insert(
            id(20),
            Track {
                net,
                layer: bottom,
                width: nm(250_000),
                path: vec![line((0, 0), (10_000_000, 0))],
            },
        );
        let r = b.connectivity();
        assert!(!r.is_fully_connected());
    }

    #[test]
    fn a_via_joins_tracks_on_two_layers() {
        let (mut b, top, bottom, net, _, _) = base();
        b.tracks.insert(
            id(20),
            Track {
                net,
                layer: top,
                width: nm(250_000),
                path: vec![line((0, 0), (5_000_000, 0))],
            },
        );
        b.tracks.insert(
            id(21),
            Track {
                net,
                layer: bottom,
                width: nm(250_000),
                path: vec![line((5_000_000, 0), (10_000_000, 0))],
            },
        );
        b.vias.insert(
            id(22),
            Via {
                net,
                at: pt(5_000_000, 0),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
        // the second pad is on the top layer only, so the bottom track does not reach it
        let r = b.connectivity();
        assert_eq!(r.nets[0].islands.len(), 2);
        // a second via at the end of the bottom track and a top track to the pad finish the job
        b.vias.insert(
            id(23),
            Via {
                net,
                at: pt(10_000_000, 0),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
        assert!(b.connectivity().is_fully_connected());
    }

    #[test]
    fn a_zone_joins_what_lies_inside_it_on_its_layer() {
        let (mut b, top, _, net, _, _) = base();
        b.zones.insert(
            id(30),
            Zone {
                net,
                layer: top,
                outline: vec![
                    pt(-2_000_000, -2_000_000),
                    pt(12_000_000, -2_000_000),
                    pt(12_000_000, 2_000_000),
                    pt(-2_000_000, 2_000_000),
                ],
                clearance: nm(200_000),
            },
        );
        assert!(b.connectivity().is_fully_connected());
    }

    #[test]
    fn nets_do_not_mix() {
        let (mut b, top, _, _, _, _) = base();
        let other = id(40);
        b.nets.insert(other, Net { name: "O".into() });
        b.tracks.insert(
            id(20),
            Track {
                net: other,
                layer: top,
                width: nm(250_000),
                path: vec![line((0, 0), (10_000_000, 0))],
            },
        );
        let r = b.connectivity();
        assert_eq!(r.nets.len(), 2);
        // the track of another net passes over both pads and joins nothing of the first net
        let first = r.nets.iter().find(|n| n.islands.len() == 2);
        assert!(first.is_some());
    }

    #[test]
    fn a_rotated_and_mirrored_component_moves_its_pads() {
        let (mut b, _, _, _, ra, rb) = base();
        let comp = ra.component;
        assert_eq!(pad_position(&b, rb), Some(pt(10_000_000, 0)));
        b.components.get_mut(&comp).unwrap().rotation = Udeg::new(90_000_000).unwrap();
        assert_eq!(pad_position(&b, rb), Some(pt(0, 10_000_000)));
        let c = b.components.get_mut(&comp).unwrap();
        c.rotation = Udeg::ZERO;
        c.side = Side::Bottom;
        assert_eq!(pad_position(&b, rb), Some(pt(-10_000_000, 0)));
        let c = b.components.get_mut(&comp).unwrap();
        c.side = Side::Top;
        c.rotation = Udeg::new(45_000_000).unwrap();
        let p = pad_position(&b, rb).unwrap();
        assert_eq!((p.x_nm.get(), p.y_nm.get()), (7_071_068, 7_071_068));
    }

    #[test]
    fn arcs_are_reported_as_judged_by_their_end_points() {
        let (mut b, top, _, net, _, _) = base();
        b.tracks.insert(
            id(20),
            Track {
                net,
                layer: top,
                width: nm(250_000),
                path: vec![Segment::Arc {
                    start: pt(0, 0),
                    end: pt(10_000_000, 0),
                    sweep: Udeg::new(90_000_000).unwrap(),
                }],
            },
        );
        let r = b.connectivity();
        assert!(r.is_fully_connected());
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some(ARC_BY_ENDS)));
    }

    #[test]
    fn the_result_does_not_depend_on_anything_but_the_board() {
        let (mut b, top, _, net, _, _) = base();
        b.tracks.insert(
            id(20),
            Track {
                net,
                layer: top,
                width: nm(250_000),
                path: vec![line((0, 0), (10_000_000, 0))],
            },
        );
        assert_eq!(b.connectivity(), b.clone().connectivity());
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
