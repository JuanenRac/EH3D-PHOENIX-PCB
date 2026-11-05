// SPDX-License-Identifier: GPL-3.0-or-later
//! What lies under a point of the board, and what a rectangle takes in. Exact integer arithmetic;
//! an arc is taken as its chord.

use std::collections::BTreeSet;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape};
use eh3d_phoenix_model::{Point, Rect};

/// Something on the board that a click can pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Picked {
    /// A component.
    Component(Id),
    /// A track.
    Track(Id),
    /// A via.
    Via(Id),
}

impl Picked {
    /// True when the board still has the thing.
    pub fn exists(&self, board: &Board) -> bool {
        match self {
            Picked::Component(id) => board.components.contains_key(id),
            Picked::Track(id) => board.tracks.contains_key(id),
            Picked::Via(id) => board.vias.contains_key(id),
        }
    }

    /// The net of a track or a via.
    pub fn net(&self, board: &Board) -> Option<Id> {
        match self {
            Picked::Component(_) => None,
            Picked::Track(id) => board.tracks.get(id).map(|t| t.net),
            Picked::Via(id) => board.vias.get(id).map(|v| v.net),
        }
    }
}

/// The squared distance from a point to the closed segment `a-b`, in square nanometres.
pub fn distance_squared(p: Point, a: Point, b: Point) -> i128 {
    let (px, py) = (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
    let (ax, ay) = (i128::from(a.x_nm.get()), i128::from(a.y_nm.get()));
    let (bx, by) = (i128::from(b.x_nm.get()), i128::from(b.y_nm.get()));
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let to_a = (px - ax) * (px - ax) + (py - ay) * (py - ay);
    if len2 == 0 {
        return to_a;
    }
    let along = (px - ax) * dx + (py - ay) * dy;
    if along <= 0 {
        return to_a;
    }
    if along >= len2 {
        return (px - bx) * (px - bx) + (py - by) * (py - by);
    }
    let cross = (px - ax) * dy - (py - ay) * dx;
    cross * cross / len2
}

/// The box of a component: its pads (and the space around them), in nanometres as
/// `(min x, min y, max x, max y)`.
pub fn component_box(board: &Board, id: Id, padding_nm: i64) -> Option<(i64, i64, i64, i64)> {
    let component = board.components.get(&id)?;
    let mut bounds: Option<(i64, i64, i64, i64)> = None;
    if let Some(footprint) = board.footprints.get(&component.footprint) {
        for pad in footprint.pads.keys() {
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: id,
                    pad: *pad,
                },
            ) else {
                continue;
            };
            let (x, y) = (at.x_nm.get(), at.y_nm.get());
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    let (x0, y0, x1, y1) = bounds.unwrap_or((
        component.at.x_nm.get(),
        component.at.y_nm.get(),
        component.at.x_nm.get(),
        component.at.y_nm.get(),
    ));
    Some((
        x0 - padding_nm,
        y0 - padding_nm,
        x1 + padding_nm,
        y1 + padding_nm,
    ))
}

fn inside(p: Point, bounds: (i64, i64, i64, i64)) -> bool {
    let (x, y) = (p.x_nm.get(), p.y_nm.get());
    (bounds.0..=bounds.2).contains(&x) && (bounds.1..=bounds.3).contains(&y)
}

/// The thing under a point: a via, else a track, else a component. A via or a track is picked when
/// the point is within its radius plus the tolerance; the nearest one wins. A component is picked
/// when the point is in its box; the smallest box wins.
pub fn pick(board: &Board, at: Point, tolerance_nm: i64, padding_nm: i64) -> Option<Picked> {
    let reach = |radius: i64| {
        let r = i128::from(radius.max(0) + tolerance_nm.max(0));
        r * r
    };
    let mut best: Option<(i128, Id)> = None;
    for (id, via) in &board.vias {
        let d = distance_squared(at, via.at, via.at);
        if d <= reach(via.diameter.get() / 2) && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, *id));
        }
    }
    if let Some((_, id)) = best {
        return Some(Picked::Via(id));
    }
    for (id, track) in &board.tracks {
        let half = track.width.get() / 2;
        for segment in &track.path {
            let d = distance_squared(at, segment.start(), segment.end());
            if d <= reach(half) && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, *id));
            }
        }
    }
    if let Some((_, id)) = best {
        return Some(Picked::Track(id));
    }
    let mut smallest: Option<(i128, Id)> = None;
    for id in board.components.keys() {
        let Some(bounds) = component_box(board, *id, padding_nm) else {
            continue;
        };
        if !inside(at, bounds) {
            continue;
        }
        let area = i128::from(bounds.2 - bounds.0) * i128::from(bounds.3 - bounds.1);
        if smallest.is_none_or(|(a, _)| area < a) {
            smallest = Some((area, *id));
        }
    }
    smallest.map(|(_, id)| Picked::Component(id))
}

/// What lies wholly inside the rectangle between two corners.
pub fn in_rectangle(board: &Board, a: Point, b: Point, padding_nm: i64) -> BTreeSet<Picked> {
    let rect = Rect::from_corners(a, b);
    let mut found = BTreeSet::new();
    for id in board.components.keys() {
        if let Some((x0, y0, x1, y1)) = component_box(board, *id, padding_nm) {
            let corners = [Point::from_nm(x0, y0), Point::from_nm(x1, y1)];
            if corners
                .iter()
                .all(|c| c.as_ref().is_ok_and(|p| rect.contains(*p)))
            {
                found.insert(Picked::Component(*id));
            }
        }
    }
    for (id, track) in &board.tracks {
        let all = !track.path.is_empty()
            && track
                .path
                .iter()
                .all(|s| rect.contains(s.start()) && rect.contains(s.end()));
        if all {
            found.insert(Picked::Track(*id));
        }
    }
    for (id, via) in &board.vias {
        if rect.contains(via.at) {
            found.insert(Picked::Via(*id));
        }
    }
    found
}

/// The pad under a point: the pad whose centre is nearest, if the point is within the half size
/// of the pad plus the tolerance.
pub fn pad_near(board: &Board, at: Point, tolerance_nm: i64) -> Option<PadRef> {
    let mut best: Option<(i128, PadRef)> = None;
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(centre) = pad_position(board, pad_ref) else {
                continue;
            };
            let half = match pad.shape {
                PadShape::Circle { diameter } => diameter.get() / 2,
                PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
                    width.get().max(height.get()) / 2
                }
            };
            let r = i128::from(half.max(0) + tolerance_nm.max(0));
            let d = distance_squared(at, centre, centre);
            if d <= r * r && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, pad_ref));
            }
        }
    }
    best.map(|(_, pad)| pad)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::fixture;

    #[test]
    fn the_distance_to_a_segment_is_exact() {
        let p = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
        // beside the middle, before the start, after the end, and a segment that is a point
        assert_eq!(distance_squared(p(5, 3), p(0, 0), p(10, 0)), 9);
        assert_eq!(distance_squared(p(-3, 4), p(0, 0), p(10, 0)), 25);
        assert_eq!(distance_squared(p(13, 4), p(0, 0), p(10, 0)), 25);
        assert_eq!(distance_squared(p(3, 4), p(0, 0), p(0, 0)), 25);
    }

    #[test]
    fn a_track_a_via_and_a_component_are_picked_in_that_order_of_priority() {
        let mut f = fixture();
        // the track runs from (11.5, 10) to (28.5, 10): a point on it
        let on_track = f.pt(20, 10);
        assert_eq!(
            pick(&f.board, on_track, 100_000, 500_000),
            Some(Picked::Track(f.track))
        );
        // a point on the first resistor, away from the track
        let on_part = f.pt(9, 10);
        assert_eq!(
            pick(&f.board, on_part, 100_000, 500_000),
            Some(Picked::Component(f.r1))
        );
        assert_eq!(pick(&f.board, f.pt(50, 30), 100_000, 500_000), None);
        // a via on the track wins over it
        let via_id = crate::testutil::id(50);
        f.board.vias.insert(
            via_id,
            eh3d_phoenix_board::Via {
                net: f.sig,
                at: on_track,
                drill: eh3d_phoenix_model::Nm::new(300_000).unwrap(),
                diameter: eh3d_phoenix_model::Nm::new(600_000).unwrap(),
                from_layer: f.top,
                to_layer: f.bottom,
            },
        );
        assert_eq!(
            pick(&f.board, on_track, 100_000, 500_000),
            Some(Picked::Via(via_id))
        );
        assert_eq!(Picked::Via(via_id).net(&f.board), Some(f.sig));
        assert!(Picked::Via(via_id).exists(&f.board));
        assert!(!Picked::Via(crate::testutil::id(99)).exists(&f.board));
    }

    #[test]
    fn a_rectangle_takes_in_what_is_wholly_inside() {
        let f = fixture();
        let small = in_rectangle(&f.board, f.pt(5, 5), f.pt(15, 15), 500_000);
        assert!(small.contains(&Picked::Component(f.r1)));
        assert!(!small.contains(&Picked::Track(f.track)));
        let wide = in_rectangle(&f.board, f.pt(55, 35), f.pt(1, 1), 500_000);
        assert_eq!(wide.len(), 3);
    }

    #[test]
    fn a_pad_is_found_by_its_centre_within_its_size() {
        let f = fixture();
        let found = pad_near(&f.board, f.pt(11, 10), 100_000).unwrap();
        assert_eq!(found, f.pad(f.r1, f.pad2));
        assert_eq!(pad_near(&f.board, f.pt(20, 20), 100_000), None);
    }
}
