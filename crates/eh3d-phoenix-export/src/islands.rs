//! Small islands of a copper pour that the export leaves out.
//!
//! The export has no fill of its own: it writes a zone as the region of its outline, or, for the
//! zones that the board editor has poured, one zone for each convex polygon of the fill (`docs/ZONES.md`).
//! Those polygons of one net on one layer that touch along a stretch of border make one island. An
//! island whose area (the outlines less the cutouts) is smaller than `gerber.zone_island_min_area_nm2`
//! is left out of the file together with the clearing that belongs to it. A minimum of 0 leaves
//! every zone in.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id, Zone};
use eh3d_phoenix_model::Point;

/// What was left out of one layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DroppedIslands {
    /// The zones that are not written.
    pub zones: BTreeSet<Id>,
    /// The islands that those zones made.
    pub islands: usize,
}

/// Twice the area of a polygon, in square nanometres and with its sign.
fn double_area(polygon: &[Point]) -> i128 {
    let n = polygon.len();
    let mut sum: i128 = 0;
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        sum += a.x_nm.get() as i128 * b.y_nm.get() as i128
            - b.x_nm.get() as i128 * a.y_nm.get() as i128;
    }
    sum
}

/// Twice the area of the copper of a zone: its outline less its cutouts.
fn double_copper_area(zone: &Zone) -> i128 {
    let holes: i128 = zone
        .cutouts
        .iter()
        .filter(|c| c.len() >= 3)
        .map(|c| double_area(c).abs())
        .sum();
    double_area(&zone.outline).abs() - holes
}

fn bounds(zone: &Zone) -> (i64, i64, i64, i64) {
    let xs = zone.outline.iter().map(|p| p.x_nm.get());
    let ys = zone.outline.iter().map(|p| p.y_nm.get());
    (
        xs.clone().min().unwrap_or(0),
        ys.clone().min().unwrap_or(0),
        xs.max().unwrap_or(0),
        ys.max().unwrap_or(0),
    )
}

/// Whether two segments lie on one line and overlap along a stretch of positive length.
fn share_border(a0: Point, a1: Point, b0: Point, b1: Point) -> bool {
    let p = |q: Point| (q.x_nm.get() as i128, q.y_nm.get() as i128);
    let (ax, ay) = p(a0);
    let (dx, dy) = (p(a1).0 - ax, p(a1).1 - ay);
    if dx == 0 && dy == 0 {
        return false;
    }
    let side = |q: Point| {
        let (qx, qy) = p(q);
        dx * (qy - ay) - dy * (qx - ax)
    };
    if side(b0) != 0 || side(b1) != 0 {
        return false;
    }
    let along = |q: Point| {
        let (qx, qy) = p(q);
        dx * (qx - ax) + dy * (qy - ay)
    };
    let (t0, t1) = (along(b0), along(b1));
    let (lo, hi) = (t0.min(t1).max(0), t0.max(t1).min(dx * dx + dy * dy));
    hi > lo
}

/// Whether two outlines share a stretch of border.
fn touch(a: &Zone, b: &Zone) -> bool {
    let (na, nb) = (a.outline.len(), b.outline.len());
    for i in 0..na {
        let (a0, a1) = (a.outline[i], a.outline[(i + 1) % na]);
        for j in 0..nb {
            if share_border(a0, a1, b.outline[j], b.outline[(j + 1) % nb]) {
                return true;
            }
        }
    }
    false
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The zones of `layer` that make an island smaller than `min_area_nm2` square nanometres. Nothing
/// when the minimum is 0 or less.
pub fn dropped_zones(board: &Board, layer: Id, min_area_nm2: i64) -> DroppedIslands {
    let mut dropped = DroppedIslands::default();
    if min_area_nm2 <= 0 {
        return dropped;
    }
    let mut by_net: BTreeMap<Id, Vec<(Id, &Zone)>> = BTreeMap::new();
    for (id, zone) in &board.zones {
        if zone.layer == layer && zone.outline.len() >= 3 {
            by_net.entry(zone.net).or_default().push((*id, zone));
        }
    }
    let limit = 2 * min_area_nm2 as i128;
    for group in by_net.values() {
        let n = group.len();
        let boxes: Vec<(i64, i64, i64, i64)> = group.iter().map(|(_, z)| bounds(z)).collect();
        // a sweep over the left edges, so that the far apart polygons of a large fill are not compared
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| boxes[i].0);
        let mut parent: Vec<usize> = (0..n).collect();
        for (k, &i) in order.iter().enumerate() {
            for &j in &order[k + 1..] {
                if boxes[j].0 > boxes[i].2 {
                    break;
                }
                let apart = boxes[j].1 > boxes[i].3 || boxes[i].1 > boxes[j].3;
                if !apart && touch(group[i].1, group[j].1) {
                    let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                    parent[ri] = rj;
                }
            }
        }
        let mut area: BTreeMap<usize, i128> = BTreeMap::new();
        for (i, (_, zone)) in group.iter().enumerate() {
            let root = find(&mut parent, i);
            *area.entry(root).or_default() += double_copper_area(zone);
        }
        for (root, twice) in &area {
            if *twice < limit {
                dropped.islands += 1;
                for (i, (zone_id, _)) in group.iter().enumerate() {
                    if find(&mut parent, i) == *root {
                        dropped.zones.insert(*zone_id);
                    }
                }
            }
        }
    }
    dropped
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::Zone;
    use eh3d_phoenix_model::{Nm, ObjectId};

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn square(x: i64, y: i64, side: i64) -> Vec<Point> {
        vec![
            pt(x, y),
            pt(x + side, y),
            pt(x + side, y + side),
            pt(x, y + side),
        ]
    }

    fn zone(net: u16, outline: Vec<Point>) -> Zone {
        Zone {
            net: id(net),
            layer: id(1),
            outline,
            clearance: Nm::new(0).unwrap(),
            cutouts: Vec::new(),
        }
    }

    fn board_of(zones: Vec<Zone>) -> Board {
        let mut b = Board::default();
        for (i, z) in zones.into_iter().enumerate() {
            b.zones.insert(id(100 + i as u16), z);
        }
        b
    }

    const MM: i64 = 1_000_000;

    #[test]
    fn a_minimum_of_zero_leaves_every_zone_in() {
        let b = board_of(vec![zone(5, square(0, 0, MM))]);
        assert!(dropped_zones(&b, id(1), 0).zones.is_empty());
    }

    #[test]
    fn a_zone_smaller_than_the_minimum_is_dropped_and_a_larger_one_stays() {
        // 1 mm by 1 mm = 1 mm2 (1e12 nm2) and 3 mm by 3 mm = 9 mm2, far apart
        let b = board_of(vec![
            zone(5, square(0, 0, MM)),
            zone(5, square(20 * MM, 0, 3 * MM)),
        ]);
        let layer = id(1);
        let out = dropped_zones(&b, layer, 2_000_000_000_000);
        assert_eq!(out.islands, 1);
        assert_eq!(out.zones.len(), 1);
        assert!(out.zones.contains(&id(100)));
        // exactly the area is not smaller than the minimum
        assert!(dropped_zones(&b, layer, 1_000_000_000_000).zones.is_empty());
        // one more square nanometre of minimum drops it
        assert_eq!(
            dropped_zones(&b, layer, 1_000_000_000_001).zones.len(),
            1
        );
    }

    #[test]
    fn pieces_that_share_a_border_make_one_island_that_counts_by_its_whole_area() {
        // two squares of 1 mm2 side by side make 2 mm2; a third, apart, is its own island
        let b = board_of(vec![
            zone(5, square(0, 0, MM)),
            zone(5, square(MM, 0, MM)),
            zone(5, square(0, 10 * MM, MM)),
        ]);
        let layer = id(1);
        let out = dropped_zones(&b, layer, 1_500_000_000_000);
        assert_eq!(out.islands, 1);
        assert_eq!(
            out.zones.iter().copied().collect::<Vec<_>>(),
            vec![id(102)]
        );
        // a bigger minimum takes the pair too
        let out = dropped_zones(&b, layer, 2_500_000_000_000);
        assert_eq!((out.islands, out.zones.len()), (2, 3));
    }

    #[test]
    fn pieces_that_meet_at_a_corner_or_belong_to_other_nets_do_not_join() {
        let b = board_of(vec![
            zone(5, square(0, 0, MM)),
            zone(5, square(MM, MM, MM)),
            zone(6, square(MM, 0, MM)),
        ]);
        let out = dropped_zones(&b, id(1), 1_500_000_000_000);
        assert_eq!((out.islands, out.zones.len()), (3, 3));
    }

    #[test]
    fn a_stretch_of_border_that_covers_part_of_a_side_still_joins() {
        // a square of 2 mm and a square of 1 mm against the middle of its right side
        let b = board_of(vec![
            zone(5, square(0, 0, 2 * MM)),
            zone(5, square(2 * MM, MM / 2, MM)),
        ]);
        // 4 + 1 = 5 mm2 together, so a minimum of 4.5 mm2 keeps both
        let out = dropped_zones(&b, id(1), 4_500_000_000_000);
        assert!(out.zones.is_empty());
    }

    #[test]
    fn the_cutouts_are_taken_off_the_area() {
        let mut z = zone(5, square(0, 0, 4 * MM));
        // 16 mm2 less a hole of 2 mm by 2 mm = 12 mm2
        z.cutouts.push(square(MM, MM, 2 * MM));
        let b = board_of(vec![z]);
        let layer = id(1);
        assert!(dropped_zones(&b, layer, 12_000_000_000_000)
            .zones
            .is_empty());
        assert_eq!(
            dropped_zones(&b, layer, 12_000_000_000_001).zones.len(),
            1
        );
    }

    #[test]
    fn zones_of_other_layers_are_not_looked_at() {
        let b = board_of(vec![zone(5, square(0, 0, MM))]);
        assert!(dropped_zones(&b, id(2), 9_000_000_000_000)
            .zones
            .is_empty());
    }
}
