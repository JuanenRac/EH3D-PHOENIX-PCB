// SPDX-License-Identifier: GPL-3.0-or-later
//! A board reduced to the plain facts that the comparison looks at: the layers and nets by name,
//! the parts by reference, the pins by `REF.PAD` with their net, the copper of each net on each
//! layer, the vias, the zones and the outline. Nothing here depends on the ids of the objects, so
//! two boards made apart from the same design can be compared.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Segment, Side};
use eh3d_phoenix_model::Point;

use crate::codes::{DiffError, LIMIT_EXCEEDED};
use crate::settings::DiffSettings;

/// A part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartFact {
    /// The reference.
    pub refdes: String,
    /// The value.
    pub value: String,
    /// The name of the footprint.
    pub footprint: String,
    /// The place across, in nanometres.
    pub x_nm: i64,
    /// The place up, in nanometres.
    pub y_nm: i64,
    /// The turn, in millionths of a degree, from 0 up to a full turn (not included).
    pub rotation_udeg: i64,
    /// Whether the part is on the bottom side.
    pub bottom: bool,
}

/// The tracks of one net on one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CopperFact {
    /// How many tracks.
    pub tracks: usize,
    /// Their length in nanometres; an arc counts as its arc.
    pub length_nm: i64,
}

/// The zones of one net on one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZoneFact {
    /// How many zones.
    pub count: usize,
    /// Twice the area, in square nanometres, with the cutouts taken away.
    pub double_area: i128,
}

/// The outline and the openings in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutlineFact {
    /// The corners of the outline.
    pub corners: usize,
    /// The openings.
    pub cutouts: usize,
    /// The width of the box of the outline, in nanometres.
    pub width_nm: i64,
    /// The height of the box of the outline, in nanometres.
    pub height_nm: i64,
    /// Twice the area of the outline, in square nanometres, with the cutouts taken away.
    pub double_area: i128,
}

/// What the comparison needs of a board.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    /// The name of the board.
    pub name: String,
    /// The names of the layers.
    pub layers: BTreeSet<String>,
    /// The names of the nets.
    pub nets: BTreeSet<String>,
    /// The parts, by reference.
    pub parts: BTreeMap<String, PartFact>,
    /// The references that more than one part has.
    pub duplicates: BTreeSet<String>,
    /// The net of each pin, by `REF.PAD`.
    pub pins: BTreeMap<String, String>,
    /// The tracks, by net and layer.
    pub copper: BTreeMap<(String, String), CopperFact>,
    /// The vias, by net and place, with how many are there.
    pub vias: BTreeMap<(String, i64, i64), usize>,
    /// The zones, by net and layer.
    pub zones: BTreeMap<(String, String), ZoneFact>,
    /// The outline.
    pub outline: OutlineFact,
}

fn shoelace(points: &[Point]) -> i128 {
    let n = points.len();
    if n < 3 {
        return 0;
    }
    let mut sum: i128 = 0;
    for (i, a) in points.iter().enumerate() {
        let b = &points[(i + 1) % n];
        sum += i128::from(a.x_nm.get()) * i128::from(b.y_nm.get())
            - i128::from(b.x_nm.get()) * i128::from(a.y_nm.get());
    }
    sum.abs()
}

fn polygon_with_cutouts(outline: &[Point], cutouts: &[Vec<Point>]) -> i128 {
    let holes: i128 = cutouts.iter().map(|c| shoelace(c)).sum();
    (shoelace(outline) - holes).max(0)
}

/// The length of a piece of a track in nanometres: a line by its two ends, an arc by its radius
/// and sweep (a chord with no sweep is a line). Both are rounded to a whole nanometre.
pub fn piece_length_nm(segment: &Segment) -> i64 {
    let (a, b) = (segment.start(), segment.end());
    let dx = i128::from(b.x_nm.get()) - i128::from(a.x_nm.get());
    let dy = i128::from(b.y_nm.get()) - i128::from(a.y_nm.get());
    let chord = ((dx * dx + dy * dy) as f64).sqrt();
    let length = match segment {
        Segment::Line { .. } => chord,
        Segment::Arc { sweep, .. } => {
            let theta = (sweep.get().unsigned_abs() as f64 / 1_000_000.0).to_radians();
            let half_sin = (theta / 2.0).sin();
            if half_sin.abs() < 1e-9 {
                chord
            } else {
                chord / (2.0 * half_sin) * theta
            }
        }
    };
    length.round() as i64
}

/// Reduces a board to its facts. The parts and nets the settings leave out are not in them.
pub fn snapshot(board: &Board, settings: &DiffSettings) -> Result<Snapshot, DiffError> {
    let items = board.components.len()
        + board.pad_nets.len()
        + board.tracks.len()
        + board.vias.len()
        + board.zones.len();
    if items > settings.max_items {
        return Err(DiffError::new(
            LIMIT_EXCEEDED,
            format!(
                "{} holds {items} objects, more than max_items = {}",
                board.name, settings.max_items
            ),
        ));
    }
    let mut snap = Snapshot {
        name: board.name.clone(),
        ..Snapshot::default()
    };
    snap.layers = board.layers.values().map(|l| l.name.clone()).collect();
    let net_name = |id| {
        board
            .nets
            .get(id)
            .map_or_else(|| "?".to_owned(), |n| n.name.clone())
    };
    let layer_name = |id| {
        board
            .layers
            .get(id)
            .map_or_else(|| "?".to_owned(), |l| l.name.clone())
    };
    snap.nets = board
        .nets
        .values()
        .map(|n| n.name.clone())
        .filter(|n| !settings.ignores_net(n))
        .collect();
    // the references that are in the board, by id of the part, for the pins
    let mut ref_of = BTreeMap::new();
    for (id, c) in &board.components {
        ref_of.insert(*id, c.refdes.clone());
        if settings.ignores_part(&c.refdes) {
            continue;
        }
        let Some(footprint) = board.footprints.get(&c.footprint) else {
            continue;
        };
        let fact = PartFact {
            refdes: c.refdes.clone(),
            value: c.value.clone(),
            footprint: footprint.name.clone(),
            x_nm: c.at.x_nm.get(),
            y_nm: c.at.y_nm.get(),
            rotation_udeg: c.rotation.normalised().get(),
            bottom: c.side == Side::Bottom,
        };
        if snap.parts.contains_key(&c.refdes) {
            snap.duplicates.insert(c.refdes.clone());
        } else {
            snap.parts.insert(c.refdes.clone(), fact);
        }
    }
    for pn in &board.pad_nets {
        let Some(refdes) = ref_of.get(&pn.component) else {
            continue;
        };
        let net = net_name(&pn.net);
        if settings.ignores_part(refdes) || settings.ignores_net(&net) {
            continue;
        }
        let number = board
            .components
            .get(&pn.component)
            .and_then(|c| board.footprints.get(&c.footprint))
            .and_then(|f| f.pads.get(&pn.pad))
            .map(|p| p.number.clone());
        if let Some(number) = number {
            snap.pins.insert(format!("{refdes}.{number}"), net);
        }
    }
    for track in board.tracks.values() {
        let net = net_name(&track.net);
        if settings.ignores_net(&net) {
            continue;
        }
        let entry = snap
            .copper
            .entry((net, layer_name(&track.layer)))
            .or_default();
        entry.tracks += 1;
        entry.length_nm += track.path.iter().map(piece_length_nm).sum::<i64>();
    }
    for via in board.vias.values() {
        let net = net_name(&via.net);
        if settings.ignores_net(&net) {
            continue;
        }
        *snap
            .vias
            .entry((net, via.at.x_nm.get(), via.at.y_nm.get()))
            .or_insert(0) += 1;
    }
    for zone in board.zones.values() {
        let net = net_name(&zone.net);
        if settings.ignores_net(&net) {
            continue;
        }
        let entry = snap
            .zones
            .entry((net, layer_name(&zone.layer)))
            .or_default();
        entry.count += 1;
        entry.double_area += polygon_with_cutouts(&zone.outline, &zone.cutouts);
    }
    let xs = board.outline.iter().map(|p| p.x_nm.get());
    let ys = board.outline.iter().map(|p| p.y_nm.get());
    let span = |values: Vec<i64>| match (values.iter().min(), values.iter().max()) {
        (Some(low), Some(high)) => high - low,
        _ => 0,
    };
    snap.outline = OutlineFact {
        corners: board.outline.len(),
        cutouts: board.outline_cutouts.len(),
        width_nm: span(xs.collect()),
        height_nm: span(ys.collect()),
        double_area: polygon_with_cutouts(&board.outline, &board.outline_cutouts),
    };
    Ok(snap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::{Point, Udeg};

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
        Segment::Line {
            start: pt(a.0, a.1),
            end: pt(b.0, b.1),
        }
    }

    #[test]
    fn a_line_is_as_long_as_its_ends_are_apart() {
        assert_eq!(piece_length_nm(&line((0, 0), (3_000, 4_000))), 5_000);
        assert_eq!(piece_length_nm(&line((5, 5), (5, 5))), 0);
        let far = 1_073_741_823;
        assert_eq!(piece_length_nm(&line((-far, 0), (far, 0))), 2 * far);
    }

    #[test]
    fn an_arc_is_as_long_as_its_radius_and_sweep_say() {
        // a half circle of diameter 2 mm: pi mm
        let half = Segment::Arc {
            start: pt(0, 0),
            end: pt(2_000_000, 0),
            sweep: Udeg::new(180_000_000).unwrap(),
        };
        assert_eq!(piece_length_nm(&half), 3_141_593);
        // the sweep in the other direction is as long
        let back = Segment::Arc {
            start: pt(0, 0),
            end: pt(2_000_000, 0),
            sweep: Udeg::new(-180_000_000).unwrap(),
        };
        assert_eq!(piece_length_nm(&back), 3_141_593);
        // a quarter circle of radius 1 mm: its chord is the square root of 2 mm
        let quarter = Segment::Arc {
            start: pt(1_000_000, 0),
            end: pt(0, 1_000_000),
            sweep: Udeg::new(90_000_000).unwrap(),
        };
        assert_eq!(piece_length_nm(&quarter), 1_570_796);
        // no sweep: the chord
        let flat = Segment::Arc {
            start: pt(0, 0),
            end: pt(3_000, 4_000),
            sweep: Udeg::ZERO,
        };
        assert_eq!(piece_length_nm(&flat), 5_000);
    }

    #[test]
    fn the_area_of_a_polygon_is_twice_the_shoelace_and_the_cutouts_are_taken_away() {
        let square = [pt(0, 0), pt(4_000, 0), pt(4_000, 4_000), pt(0, 4_000)];
        assert_eq!(shoelace(&square), 32_000_000);
        // the other way round gives the same
        let back = [pt(0, 4_000), pt(4_000, 4_000), pt(4_000, 0), pt(0, 0)];
        assert_eq!(shoelace(&back), 32_000_000);
        let hole = vec![
            pt(1_000, 1_000),
            pt(2_000, 1_000),
            pt(2_000, 2_000),
            pt(1_000, 2_000),
        ];
        assert_eq!(
            polygon_with_cutouts(&square, std::slice::from_ref(&hole)),
            32_000_000 - 2_000_000
        );
        // fewer than three corners have no area, and a cutout cannot make it negative
        assert_eq!(shoelace(&square[..2]), 0);
        assert_eq!(polygon_with_cutouts(&hole, &[square.to_vec()]), 0);
    }

    #[test]
    fn the_largest_polygon_of_the_range_does_not_overflow() {
        let far = 1_073_741_823;
        let big = [pt(-far, -far), pt(far, -far), pt(far, far), pt(-far, far)];
        let side = i128::from(2 * far);
        assert_eq!(shoelace(&big), 2 * side * side);
    }
}
