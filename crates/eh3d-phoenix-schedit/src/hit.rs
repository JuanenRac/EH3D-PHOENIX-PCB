// SPDX-License-Identifier: GPL-3.0-or-later
//! What lies under a point, what a rectangle takes in, and where a point snaps to. All distances
//! are squared integer nanometres in `i128`, so nothing rounds and nothing overflows.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_model::{Point, Rect};
use eh3d_phoenix_schematic::{Id, Item, Schematic};

/// The squared distance from `p` to the piece `a`-`b`.
pub fn distance_squared_to_piece(p: Point, a: Point, b: Point) -> i128 {
    let (px, py) = (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
    let (ax, ay) = (i128::from(a.x_nm.get()), i128::from(a.y_nm.get()));
    let (bx, by) = (i128::from(b.x_nm.get()), i128::from(b.y_nm.get()));
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 == 0 {
        return p.distance_squared(a);
    }
    let along = (px - ax) * dx + (py - ay) * dy;
    if along <= 0 {
        return p.distance_squared(a);
    }
    if along >= len2 {
        return p.distance_squared(b);
    }
    let cross = (px - ax) * dy - (py - ay) * dx;
    cross * cross / len2
}

/// The point of the grid nearest to `p`; `p` itself when the result would leave the range.
pub fn snap_to_grid(p: Point, grid_nm: i64) -> Point {
    if grid_nm <= 0 {
        return p;
    }
    let round = |v: i64| -> Option<i64> {
        let shifted = v.checked_add(grid_nm / 2)?;
        shifted.div_euclid(grid_nm).checked_mul(grid_nm)
    };
    match (round(p.x_nm.get()), round(p.y_nm.get())) {
        (Some(x), Some(y)) => Point::from_nm(x, y).unwrap_or(p),
        _ => p,
    }
}

fn better(best: &mut Option<(i128, Item)>, d: i128, item: Item) {
    let current = *best;
    if current.is_none_or(|(d0, item0)| (d, item) < (d0, item0)) {
        *best = Some((d, item));
    }
}

/// The nearest pin or wire point on a sheet within `tol_nm` of `at`: where a wire should end.
pub fn join_point_near(s: &Schematic, sheet: Id, at: Point, tol_nm: i64) -> Option<Point> {
    let tol2 = i128::from(tol_nm) * i128::from(tol_nm);
    let mut best: Option<(i128, (i64, i64), Point)> = None;
    let mut consider = |p: Point| {
        let d = at.distance_squared(p);
        let k = (p.x_nm.get(), p.y_nm.get());
        if d <= tol2 && best.is_none_or(|(d0, k0, _)| (d, k) < (d0, k0)) {
            best = Some((d, k, p));
        }
    };
    for (_, pin_sheet, p) in s.pin_positions() {
        if pin_sheet == sheet {
            consider(p);
        }
    }
    for wire in s.wires.values().filter(|w| w.sheet == sheet) {
        for p in &wire.path {
            consider(*p);
        }
    }
    best.map(|(_, _, p)| p)
}

/// Where a click on `at` lands: on a pin or wire point within the tolerance, otherwise on the grid
/// (when snapping is on), otherwise where it is.
pub fn snap_target(
    s: &Schematic,
    sheet: Id,
    at: Point,
    grid_nm: i64,
    snap: bool,
    tol_nm: i64,
) -> Point {
    if let Some(p) = join_point_near(s, sheet, at, tol_nm) {
        return p;
    }
    if snap {
        snap_to_grid(at, grid_nm)
    } else {
        at
    }
}

/// The boxes of the placed parts of a sheet: the smallest box around the origin and the pins.
fn part_boxes(s: &Schematic, sheet: Id) -> BTreeMap<Id, (i64, i64, i64, i64)> {
    let mut boxes: BTreeMap<Id, (i64, i64, i64, i64)> = BTreeMap::new();
    for (id, part) in &s.parts {
        if let Some(place) = part.placement.filter(|p| p.sheet == sheet) {
            let (x, y) = (place.at.x_nm.get(), place.at.y_nm.get());
            boxes.insert(*id, (x, y, x, y));
        }
    }
    for (pin, pin_sheet, p) in s.pin_positions() {
        if pin_sheet != sheet {
            continue;
        }
        if let Some(b) = boxes.get_mut(&pin.part) {
            let (x, y) = (p.x_nm.get(), p.y_nm.get());
            *b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    boxes
}

/// The item under `at` on a sheet. Net labels and power ports come first (within `tol_nm`), then
/// parts (the box around the pins, `pad_nm` larger on each side; the smallest box wins), then wires
/// (within `tol_nm`; the nearest wins). Ties go to the smaller id.
pub fn hit(s: &Schematic, sheet: Id, at: Point, tol_nm: i64, pad_nm: i64) -> Option<Item> {
    let tol2 = i128::from(tol_nm) * i128::from(tol_nm);
    let mut best: Option<(i128, Item)> = None;
    for (id, label) in s.labels.iter().filter(|(_, l)| l.sheet == sheet) {
        let d = at.distance_squared(label.at);
        if d <= tol2 {
            better(&mut best, d, Item::Label(*id));
        }
    }
    for (id, port) in s.power_ports.iter().filter(|(_, p)| p.sheet == sheet) {
        let d = at.distance_squared(port.at);
        if d <= tol2 {
            better(&mut best, d, Item::PowerPort(*id));
        }
    }
    if let Some((_, item)) = best {
        return Some(item);
    }
    let (x, y) = (at.x_nm.get(), at.y_nm.get());
    for (id, (x0, y0, x1, y1)) in part_boxes(s, sheet) {
        let (lo_x, hi_x) = (x0.saturating_sub(pad_nm), x1.saturating_add(pad_nm));
        let (lo_y, hi_y) = (y0.saturating_sub(pad_nm), y1.saturating_add(pad_nm));
        if (lo_x..=hi_x).contains(&x) && (lo_y..=hi_y).contains(&y) {
            let area =
                (i128::from(hi_x) - i128::from(lo_x)) * (i128::from(hi_y) - i128::from(lo_y));
            better(&mut best, area, Item::Part(id));
        }
    }
    if let Some((_, item)) = best {
        return Some(item);
    }
    for (id, wire) in s.wires.iter().filter(|(_, w)| w.sheet == sheet) {
        let nearest = wire
            .path
            .windows(2)
            .map(|w| distance_squared_to_piece(at, w[0], w[1]))
            .min();
        if let Some(d) = nearest.filter(|d| *d <= tol2) {
            better(&mut best, d, Item::Wire(*id));
        }
    }
    best.map(|(_, item)| item)
}

/// The items of a sheet that the rectangle between two corners takes in: a part whose origin is in
/// it, a wire all of whose points are in it, and the labels and power ports in it.
pub fn in_rect(s: &Schematic, sheet: Id, a: Point, b: Point) -> BTreeSet<Item> {
    let rect = Rect::from_corners(a, b);
    let mut out = BTreeSet::new();
    for (id, part) in &s.parts {
        if part
            .placement
            .is_some_and(|p| p.sheet == sheet && rect.contains(p.at))
        {
            out.insert(Item::Part(*id));
        }
    }
    for (id, wire) in &s.wires {
        if wire.sheet == sheet
            && !wire.path.is_empty()
            && wire.path.iter().all(|p| rect.contains(*p))
        {
            out.insert(Item::Wire(*id));
        }
    }
    for (id, label) in &s.labels {
        if label.sheet == sheet && rect.contains(label.at) {
            out.insert(Item::Label(*id));
        }
    }
    for (id, port) in &s.power_ports {
        if port.sheet == sheet && rect.contains(port.at) {
            out.insert(Item::PowerPort(*id));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::fixture;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn a_point_snaps_to_the_nearest_grid_point_on_both_sides_of_zero() {
        assert_eq!(
            snap_to_grid(pt(1_300_000, -1_300_000), 2_540_000),
            pt(2_540_000, -2_540_000)
        );
        assert_eq!(snap_to_grid(pt(1_200_000, 0), 2_540_000), pt(0, 0));
        assert_eq!(snap_to_grid(pt(7, 9), 0), pt(7, 9));
    }

    #[test]
    fn the_distance_to_a_piece_is_exact() {
        let (a, b) = (pt(0, 0), pt(10, 0));
        assert_eq!(distance_squared_to_piece(pt(5, 3), a, b), 9);
        assert_eq!(distance_squared_to_piece(pt(-3, 4), a, b), 25);
        assert_eq!(distance_squared_to_piece(pt(13, 4), a, b), 25);
        assert_eq!(distance_squared_to_piece(pt(2, 2), a, a), 8);
    }

    #[test]
    fn a_click_lands_on_a_pin_before_the_grid() {
        let f = fixture();
        let near_pin = pt(22_700_000, 20_100_000);
        assert_eq!(
            snap_target(&f.schematic, f.sheet, near_pin, 2_540_000, true, 1_270_000),
            pt(22_540_000, 20_000_000)
        );
        assert_eq!(
            snap_target(
                &f.schematic,
                f.sheet,
                pt(50_300_000, 31_000_000),
                2_540_000,
                true,
                1_270_000
            ),
            pt(50_800_000, 30_480_000)
        );
        assert_eq!(
            snap_target(
                &f.schematic,
                f.sheet,
                pt(50_300_000, 31_000_000),
                2_540_000,
                false,
                1_270_000
            ),
            pt(50_300_000, 31_000_000)
        );
    }

    #[test]
    fn the_item_under_a_point_is_found_in_the_order_labels_parts_wires() {
        let f = fixture();
        let s = &f.schematic;
        assert_eq!(
            hit(s, f.sheet, pt(30_000_000, 20_000_000), 1_270_000, 1_270_000),
            Some(Item::Wire(f.wire))
        );
        assert_eq!(
            hit(s, f.sheet, pt(30_000_000, 21_000_000), 1_270_000, 1_270_000),
            Some(Item::Wire(f.wire))
        );
        assert_eq!(
            hit(s, f.sheet, pt(20_000_000, 20_000_000), 1_270_000, 1_270_000),
            Some(Item::Part(f.r1))
        );
        // beside a pin the box of the part wins over the wire
        assert_eq!(
            hit(s, f.sheet, pt(22_600_000, 20_000_000), 1_270_000, 1_270_000),
            Some(Item::Part(f.r1))
        );
        assert_eq!(
            hit(
                s,
                f.sheet,
                pt(100_000_000, 100_000_000),
                1_270_000,
                1_270_000
            ),
            None
        );
        assert_eq!(
            hit(
                s,
                id_other(f.sheet),
                pt(30_000_000, 20_000_000),
                1_270_000,
                1_270_000
            ),
            None
        );
    }

    fn id_other(sheet: Id) -> Id {
        // an id that is not the sheet
        let bytes = *sheet.as_bytes();
        let mut other = bytes;
        other[15] ^= 0x01;
        Id::from_bytes(other).unwrap()
    }

    #[test]
    fn a_rectangle_takes_in_the_parts_and_the_wires_that_lie_inside_it() {
        let f = fixture();
        let s = &f.schematic;
        let small = in_rect(
            s,
            f.sheet,
            pt(15_000_000, 15_000_000),
            pt(25_000_000, 25_000_000),
        );
        assert_eq!(small, BTreeSet::from([Item::Part(f.r1)]));
        let wide = in_rect(
            s,
            f.sheet,
            pt(45_000_000, 25_000_000),
            pt(15_000_000, 15_000_000),
        );
        assert_eq!(
            wide,
            BTreeSet::from([Item::Part(f.r1), Item::Part(f.r2), Item::Wire(f.wire)])
        );
    }
}
