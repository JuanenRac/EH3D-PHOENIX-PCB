// SPDX-License-Identifier: GPL-3.0-or-later
//! The geometry of a structure in a local frame of integer nanometres, and its conversion into
//! the reversible operations of a board.
//!
//! A generator draws in a frame where the signal runs along the x axis; the shapes are polygons
//! of copper (the exact outline of a line, a stub, a patch), arcs of a track, vias, and parts with
//! pads. Nothing is rounded here except in [`nm_from_m`], the one place where a length in metres
//! becomes nanometres: it rounds to the nearest nanometre, halves away from zero. The placement
//! turns the frame by a multiple of 90 degrees (exact in integers) and moves it. The conversion
//! makes one [`Operation::Batch`], which the board applies all or nothing and whose inverse
//! restores the board exactly.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, LayerKind, Operation, Pad, PadRef, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, Point, Udeg, COORD_LIMIT_NM};

use crate::codes::{
    RfgenError, COORDINATE_RANGE, ID_UNAVAILABLE, LAYER_NOT_FOUND, LIMIT_EXCEEDED,
    PARAMETER_INVALID,
};
use crate::settings::RfgenSettings;

/// A point of the local frame, `(x, y)` in nanometres.
pub type P = (i64, i64);

/// Converts a length in metres to nanometres, rounding to the nearest nanometre with halves away
/// from zero. A number that is not finite, or that does not fit the coordinate range of the model,
/// is an error.
pub fn nm_from_m(metres: f64) -> Result<i64, RfgenError> {
    nm_from_f64(metres * 1.0e9)
}

/// Converts a length in nanometres given as a number to an integer, rounding to the nearest
/// nanometre with halves away from zero.
pub fn nm_from_f64(nanometres: f64) -> Result<i64, RfgenError> {
    let rounded = nanometres.round();
    if !rounded.is_finite() || rounded.abs() >= COORD_LIMIT_NM as f64 {
        return Err(RfgenError::new(
            COORDINATE_RANGE,
            format!("{nanometres} nm does not fit the range of the model"),
        ));
    }
    Ok(rounded as i64)
}

/// A positive width made even by adding one nanometre when it is odd, so that a line is centred
/// exactly on its axis.
pub fn even(width_nm: i64) -> i64 {
    width_nm + (width_nm & 1)
}

/// The corners of a rectangle, counter-clockwise from the lower left, whatever the order of the
/// arguments.
pub fn rect_points(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<P> {
    let (xa, xb) = (x0.min(x1), x0.max(x1));
    let (ya, yb) = (y0.min(y1), y0.max(y1));
    vec![(xa, ya), (xb, ya), (xb, yb), (xa, yb)]
}

/// Which copper layer a shape is on: the one of the structure or its reference plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// The layer the microstrip is on.
    Signal,
    /// The reference plane under the substrate.
    Plane,
}

/// A polygon of copper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poly {
    /// The index of its net in [`Layout::nets`].
    pub net: usize,
    /// Its layer.
    pub level: Level,
    /// The corners, counter-clockwise, without repeating the first.
    pub points: Vec<P>,
}

/// A track made of one arc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcPath {
    /// The index of its net.
    pub net: usize,
    /// The width of the copper.
    pub width: i64,
    /// Where the arc starts.
    pub start: P,
    /// Where it ends.
    pub end: P,
    /// The signed sweep in micro-degrees; positive is counter-clockwise.
    pub sweep_udeg: i64,
}

/// A via from the layer of the structure to its reference plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViaSpec {
    /// The index of its net.
    pub net: usize,
    /// Where it is.
    pub at: P,
    /// The drill.
    pub drill: i64,
    /// The pad diameter.
    pub diameter: i64,
}

/// A pad of a part, relative to the origin of the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadSpec {
    /// The number printed on the part.
    pub number: String,
    /// The size along x.
    pub width: i64,
    /// The size along y.
    pub height: i64,
    /// The centre, relative to the origin of the part.
    pub at: P,
    /// The index of its net.
    pub net: usize,
}

/// A part with pads on the layer of the structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartSpec {
    /// The start of the reference designator (`J` becomes `J1`, or the next free number).
    pub prefix: String,
    /// The value.
    pub value: String,
    /// The name of the footprint.
    pub footprint: String,
    /// Where the origin of the part is.
    pub at: P,
    /// The pads.
    pub pads: Vec<PadSpec>,
}

/// A port of the structure, where it joins the rest of the board.
#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    /// The name (`P1`).
    pub name: String,
    /// Where it is, in the local frame.
    pub at: P,
    /// The direction in which the signal leaves the structure, in degrees.
    pub angle_deg: i32,
    /// The impedance of the line at the port in ohm.
    pub z0_ohm: f64,
}

/// Everything a generator draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    /// The names of the nets, in order of first use.
    pub nets: Vec<String>,
    /// The polygons of copper.
    pub polys: Vec<Poly>,
    /// The tracks that are arcs.
    pub arcs: Vec<ArcPath>,
    /// The vias.
    pub vias: Vec<ViaSpec>,
    /// The parts.
    pub parts: Vec<PartSpec>,
    /// The ports.
    pub ports: Vec<Port>,
    /// The narrowest copper shape drawn, in nanometres; zero until something is drawn.
    pub narrowest_nm: i64,
}

impl Layout {
    /// An empty layout.
    pub fn new() -> Layout {
        Layout::default()
    }

    /// The index of a net, added when it is new.
    pub fn net(&mut self, name: &str) -> usize {
        if let Some(index) = self.nets.iter().position(|n| n == name) {
            return index;
        }
        self.nets.push(name.to_owned());
        self.nets.len() - 1
    }

    fn note_width(&mut self, width: i64) {
        if width > 0 && (self.narrowest_nm == 0 || width < self.narrowest_nm) {
            self.narrowest_nm = width;
        }
    }

    /// Adds a polygon on the layer of the structure.
    pub fn poly(&mut self, net: usize, points: Vec<P>) {
        self.polys.push(Poly {
            net,
            level: Level::Signal,
            points,
        });
    }

    /// Adds a rectangle on the layer of the structure and notes its smaller side.
    pub fn rect(&mut self, net: usize, x0: i64, y0: i64, x1: i64, y1: i64) {
        self.note_width((x1 - x0).abs().min((y1 - y0).abs()));
        self.poly(net, rect_points(x0, y0, x1, y1));
    }

    /// Adds a polygon and notes a width for it (the width of the line it stands for).
    pub fn poly_with_width(&mut self, net: usize, points: Vec<P>, width: i64) {
        self.note_width(width);
        self.poly(net, points);
    }

    /// Adds a rectangle on the reference plane.
    pub fn plane_rect(&mut self, net: usize, x0: i64, y0: i64, x1: i64, y1: i64) {
        self.polys.push(Poly {
            net,
            level: Level::Plane,
            points: rect_points(x0, y0, x1, y1),
        });
    }

    /// Adds a via.
    pub fn via(&mut self, net: usize, at: P, drill: i64, diameter: i64) {
        self.vias.push(ViaSpec {
            net,
            at,
            drill,
            diameter,
        });
    }

    /// Adds an arc track.
    pub fn arc(&mut self, net: usize, width: i64, start: P, end: P, sweep_udeg: i64) {
        self.note_width(width);
        self.arcs.push(ArcPath {
            net,
            width,
            start,
            end,
            sweep_udeg,
        });
    }

    /// Adds a port.
    pub fn port(&mut self, name: &str, at: P, angle_deg: i32, z0_ohm: f64) {
        self.ports.push(Port {
            name: name.to_owned(),
            at,
            angle_deg,
            z0_ohm,
        });
    }

    /// The number of shapes, parts and vias.
    pub fn items(&self) -> usize {
        self.polys.len() + self.arcs.len() + self.vias.len() + self.parts.len()
    }

    /// The box around every point of the signal layer shapes, as lower left and upper right.
    pub fn bbox(&self) -> Option<(P, P)> {
        let mut found: Option<(P, P)> = None;
        let mut grow = |p: P| {
            found = Some(match found {
                None => (p, p),
                Some((lo, hi)) => (
                    (lo.0.min(p.0), lo.1.min(p.1)),
                    (hi.0.max(p.0), hi.1.max(p.1)),
                ),
            });
        };
        for poly in &self.polys {
            for p in &poly.points {
                grow(*p);
            }
        }
        for arc in &self.arcs {
            grow(arc.start);
            grow(arc.end);
        }
        for via in &self.vias {
            grow(via.at);
        }
        for part in &self.parts {
            for pad in &part.pads {
                grow((part.at.0 + pad.at.0, part.at.1 + pad.at.1));
            }
        }
        found
    }
}

/// Where a structure is put on the board: the place of the origin of its frame and the rotation
/// of the frame, a multiple of 90 degrees counter-clockwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Placement {
    /// The x coordinate of the origin in nanometres.
    pub x_nm: i64,
    /// The y coordinate of the origin in nanometres.
    pub y_nm: i64,
    /// The rotation in degrees; a multiple of 90.
    pub rotation_deg: i32,
}

impl Placement {
    /// A placement, refused when the rotation is not a multiple of 90 degrees.
    pub fn new(x_nm: i64, y_nm: i64, rotation_deg: i32) -> Result<Placement, RfgenError> {
        if rotation_deg % 90 != 0 {
            return Err(RfgenError::new(
                PARAMETER_INVALID,
                format!("the rotation {rotation_deg} is not a multiple of 90 degrees"),
            ));
        }
        Ok(Placement {
            x_nm,
            y_nm,
            rotation_deg,
        })
    }

    fn quarter_turns(&self) -> i32 {
        self.rotation_deg.rem_euclid(360) / 90
    }

    /// The point of the board that a point of the local frame becomes.
    pub fn apply(&self, p: P) -> Result<Point, RfgenError> {
        let (x, y) = match self.quarter_turns() {
            0 => (p.0, p.1),
            1 => (-p.1, p.0),
            2 => (-p.0, -p.1),
            _ => (p.1, -p.0),
        };
        let (x, y) = (
            x.checked_add(self.x_nm).ok_or_else(range_error)?,
            y.checked_add(self.y_nm).ok_or_else(range_error)?,
        );
        Point::from_nm(x, y).map_err(|e| RfgenError::new(COORDINATE_RANGE, e.to_string()))
    }

    /// The direction of a port of the local frame on the board, in degrees from 0 to 359.
    pub fn angle(&self, angle_deg: i32) -> i32 {
        (angle_deg + self.rotation_deg).rem_euclid(360)
    }

    /// The rotation as an angle of the model.
    pub fn udeg(&self) -> Result<Udeg, RfgenError> {
        Udeg::new(i64::from(self.rotation_deg.rem_euclid(360)) * 1_000_000)
            .map_err(|e| RfgenError::new(COORDINATE_RANGE, e.to_string()))
    }
}

fn range_error() -> RfgenError {
    RfgenError::new(
        COORDINATE_RANGE,
        "a coordinate overflows after the placement",
    )
}

/// A source of ids that never gives one the board already uses.
pub struct IdSource<'a> {
    next: &'a mut dyn FnMut() -> Id,
    used: BTreeSet<Id>,
    attempts: usize,
}

impl<'a> IdSource<'a> {
    /// A source over a generator of ids; the ids of the board are skipped. At most `attempts`
    /// ids are tried for each id that is needed.
    pub fn new(board: &Board, next: &'a mut dyn FnMut() -> Id, attempts: usize) -> IdSource<'a> {
        let mut used = BTreeSet::new();
        used.extend(board.layers.keys().copied());
        used.extend(board.nets.keys().copied());
        used.extend(board.components.keys().copied());
        used.extend(board.tracks.keys().copied());
        used.extend(board.vias.keys().copied());
        used.extend(board.zones.keys().copied());
        used.extend(board.silkscreen.keys().copied());
        for (id, footprint) in &board.footprints {
            used.insert(*id);
            used.extend(footprint.pads.keys().copied());
        }
        IdSource {
            next,
            used,
            attempts,
        }
    }

    /// A new id.
    pub fn fresh(&mut self) -> Result<Id, RfgenError> {
        for _ in 0..self.attempts {
            let id = (self.next)();
            if self.used.insert(id) {
                return Ok(id);
            }
        }
        Err(RfgenError::new(
            ID_UNAVAILABLE,
            format!("{} ids were tried and all were in use", self.attempts),
        ))
    }
}

/// The copper layer of a board with a name, compared without regard to case.
pub fn resolve_layer(board: &Board, name: &str) -> Result<Id, RfgenError> {
    board
        .layers
        .iter()
        .find(|(_, layer)| layer.kind == LayerKind::Copper && layer.name.eq_ignore_ascii_case(name))
        .map(|(id, _)| *id)
        .ok_or_else(|| RfgenError::new(LAYER_NOT_FOUND, format!("the copper layer {name}")))
}

/// A reference designator that no component of the board has: `prefix` and the first free number.
fn free_refdes(board: &Board, prefix: &str, taken: &BTreeSet<String>) -> String {
    let mut n: u64 = 1;
    loop {
        let candidate = format!("{prefix}{n}");
        if !taken.contains(&candidate) && !board.components.values().any(|c| c.refdes == candidate)
        {
            return candidate;
        }
        n += 1;
    }
}

/// The operation that puts a layout on a board.
///
/// `signal` and `plane` are the names of the copper layers of the structure and of its reference
/// plane. The nets are found by name or added. The operation is one batch in the order nets,
/// footprints, components, pad nets, zones, tracks and vias.
pub fn to_operation(
    layout: &Layout,
    board: &Board,
    signal: &str,
    plane: &str,
    placement: &Placement,
    settings: &RfgenSettings,
    ids: &mut IdSource<'_>,
) -> Result<Operation, RfgenError> {
    if layout.items() > settings.max_items {
        return Err(RfgenError::new(
            LIMIT_EXCEEDED,
            format!(
                "{} items are more than max_items = {}",
                layout.items(),
                settings.max_items
            ),
        ));
    }
    let signal_layer = resolve_layer(board, signal)?;
    let needs_plane =
        layout.polys.iter().any(|p| p.level == Level::Plane) || !layout.vias.is_empty();
    let plane_layer = if needs_plane {
        resolve_layer(board, plane)?
    } else {
        signal_layer
    };
    let layer_of = |level: Level| match level {
        Level::Signal => signal_layer,
        Level::Plane => plane_layer,
    };

    let mut ops: Vec<Operation> = Vec::new();
    let mut net_ids: Vec<Id> = Vec::with_capacity(layout.nets.len());
    for name in &layout.nets {
        let existing = board
            .nets
            .iter()
            .find(|(_, net)| net.name == *name)
            .map(|(id, _)| *id);
        match existing {
            Some(id) => net_ids.push(id),
            None => {
                let id = ids.fresh()?;
                ops.push(Operation::AddNet {
                    id,
                    name: name.clone(),
                });
                net_ids.push(id);
            }
        }
    }
    let net_id = |index: usize| -> Result<Id, RfgenError> {
        net_ids.get(index).copied().ok_or_else(|| {
            RfgenError::new(
                PARAMETER_INVALID,
                format!("the net number {index} is missing"),
            )
        })
    };

    let mut pad_ops: Vec<Operation> = Vec::new();
    let mut refdes_taken: BTreeSet<String> = BTreeSet::new();
    // the id of each footprint made in this run and the id of each of its pads by number
    let mut footprints: BTreeMap<String, (Id, BTreeMap<String, Id>)> = BTreeMap::new();
    for part in &layout.parts {
        if !footprints.contains_key(&part.footprint) {
            let id = ids.fresh()?;
            let mut pads = BTreeMap::new();
            let mut by_number = BTreeMap::new();
            for pad in &part.pads {
                let shape = PadShape::Rect {
                    width: Nm::new(pad.width).map_err(model_error)?,
                    height: Nm::new(pad.height).map_err(model_error)?,
                };
                let pad_id = ids.fresh()?;
                by_number.insert(pad.number.clone(), pad_id);
                pads.insert(
                    pad_id,
                    Pad {
                        number: pad.number.clone(),
                        shape,
                        at: Point::from_nm(pad.at.0, pad.at.1).map_err(model_error)?,
                        rotation: Udeg::ZERO,
                        drill: None,
                        layers: vec![signal_layer],
                    },
                );
            }
            ops.push(Operation::AddFootprint {
                id,
                footprint: Footprint {
                    name: part.footprint.clone(),
                    pads,
                },
            });
            footprints.insert(part.footprint.clone(), (id, by_number));
        }
        let Some((footprint_id, by_number)) = footprints.get(&part.footprint) else {
            continue;
        };
        let component_id = ids.fresh()?;
        let refdes = free_refdes(board, &part.prefix, &refdes_taken);
        refdes_taken.insert(refdes.clone());
        ops.push(Operation::AddComponent {
            id: component_id,
            component: Component {
                refdes,
                value: part.value.clone(),
                symbol: None,
                footprint: *footprint_id,
                at: placement.apply(part.at)?,
                rotation: placement.udeg()?,
                side: Side::Top,
            },
        });
        for pad in &part.pads {
            if let Some(pad_id) = by_number.get(&pad.number) {
                pad_ops.push(Operation::SetPadNet {
                    pad: PadRef {
                        component: component_id,
                        pad: *pad_id,
                    },
                    net: Some(net_id(pad.net)?),
                });
            }
        }
    }
    ops.append(&mut pad_ops);

    let clearance = Nm::new(settings.zone_clearance_nm).map_err(model_error)?;
    for poly in &layout.polys {
        let mut outline = Vec::with_capacity(poly.points.len());
        for p in &poly.points {
            outline.push(placement.apply(*p)?);
        }
        ops.push(Operation::AddZone {
            id: ids.fresh()?,
            zone: Zone {
                net: net_id(poly.net)?,
                layer: layer_of(poly.level),
                outline,
                clearance,
            },
        });
    }
    for arc in &layout.arcs {
        ops.push(Operation::AddTrack {
            id: ids.fresh()?,
            track: Track {
                net: net_id(arc.net)?,
                layer: signal_layer,
                width: Nm::new(arc.width).map_err(model_error)?,
                path: vec![Segment::Arc {
                    start: placement.apply(arc.start)?,
                    end: placement.apply(arc.end)?,
                    sweep: Udeg::new(arc.sweep_udeg).map_err(model_error)?,
                }],
            },
        });
    }
    for via in &layout.vias {
        ops.push(Operation::AddVia {
            id: ids.fresh()?,
            via: Via {
                net: net_id(via.net)?,
                at: placement.apply(via.at)?,
                drill: Nm::new(via.drill).map_err(model_error)?,
                diameter: Nm::new(via.diameter).map_err(model_error)?,
                from_layer: signal_layer,
                to_layer: plane_layer,
            },
        });
    }
    Ok(Operation::Batch { ops })
}

fn model_error(e: eh3d_phoenix_model::ModelError) -> RfgenError {
    RfgenError::new(COORDINATE_RANGE, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Layer, Net};
    use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, ObjectId, StepClock};

    fn board() -> Board {
        let mut b = Board::default();
        let mut generator = IdGenerator::new(
            StepClock::new(1_700_000_000_000, 1),
            DeterministicRandom::new(7),
        );
        let top = generator.generate().unwrap();
        let bottom = generator.generate().unwrap();
        for (id, name) in [(top, "top"), (bottom, "bottom")] {
            b.layers.insert(
                id,
                Layer {
                    name: name.into(),
                    kind: LayerKind::Copper,
                    side: None,
                },
            );
        }
        b
    }

    fn source() -> impl FnMut() -> Id {
        let mut generator = IdGenerator::new(
            StepClock::new(1_700_000_000_000, 1),
            DeterministicRandom::new(7),
        );
        move || generator.generate().unwrap()
    }

    #[test]
    fn lengths_are_rounded_to_the_nearest_nanometre_halves_away_from_zero() {
        assert_eq!(nm_from_f64(0.4).unwrap(), 0);
        assert_eq!(nm_from_f64(0.5).unwrap(), 1);
        assert_eq!(nm_from_f64(-0.5).unwrap(), -1);
        assert_eq!(nm_from_f64(1234.49).unwrap(), 1234);
        assert_eq!(nm_from_m(0.001_234_568).unwrap(), 1_234_568);
        assert!(nm_from_f64(f64::NAN).is_err());
        assert!(nm_from_f64(f64::INFINITY).is_err());
        assert!(nm_from_f64(2.0e9).is_err());
        assert!(nm_from_m(2.0).is_err());
        assert_eq!(nm_from_f64(1.0e9).unwrap(), 1_000_000_000);
    }

    #[test]
    fn widths_are_made_even_and_rectangles_have_a_fixed_order() {
        assert_eq!(even(3_000_000), 3_000_000);
        assert_eq!(even(3_000_001), 3_000_002);
        assert_eq!(
            rect_points(5, 7, 1, 3),
            vec![(1, 3), (5, 3), (5, 7), (1, 7)]
        );
    }

    #[test]
    fn the_placement_turns_in_whole_quarters_and_moves() {
        let p = Placement::new(1000, 2000, 90).unwrap();
        let q = p.apply((10, 0)).unwrap();
        assert_eq!((q.x_nm.get(), q.y_nm.get()), (1000, 2010));
        let p = Placement::new(0, 0, 180).unwrap();
        let q = p.apply((10, 4)).unwrap();
        assert_eq!((q.x_nm.get(), q.y_nm.get()), (-10, -4));
        let p = Placement::new(0, 0, -90).unwrap();
        let q = p.apply((10, 4)).unwrap();
        assert_eq!((q.x_nm.get(), q.y_nm.get()), (4, -10));
        assert_eq!(p.angle(0), 270);
        assert_eq!(p.udeg().unwrap().get(), 270_000_000);
        assert!(Placement::new(0, 0, 45).is_err());
        let far = Placement::new(1 << 30, 0, 0).unwrap();
        assert!(far.apply((0, 0)).is_err());
    }

    #[test]
    fn the_layout_keeps_the_nets_the_widths_and_the_box() {
        let mut l = Layout::new();
        let a = l.net("RF");
        let b = l.net("GND");
        assert_eq!((a, b, l.net("RF")), (0, 1, 0));
        assert_eq!(l.bbox(), None);
        l.rect(a, 0, -500, 4000, 500);
        l.rect(a, 4000, -200, 6000, 200);
        l.via(b, (100, 900), 300, 600);
        assert_eq!(l.narrowest_nm, 400);
        assert_eq!(l.bbox(), Some(((0, -500), (6000, 900))));
        assert_eq!(l.items(), 3);
    }

    #[test]
    fn the_operation_adds_nets_zones_tracks_and_vias_and_undoes_exactly() {
        let mut b = board();
        b.nets.insert(
            ObjectId::from_parts(1_700_000_000_000, 999, 999).unwrap(),
            Net {
                name: "RF".to_string(),
            },
        );
        let before = b.clone();
        let mut l = Layout::new();
        let rf = l.net("RF");
        let gnd = l.net("GND");
        l.rect(rf, 0, -750_000, 10_000_000, 750_000);
        l.arc(rf, 1_500_000, (0, 0), (3_000_000, 3_000_000), 90_000_000);
        l.via(gnd, (5_000_000, 3_000_000), 300_000, 600_000);
        let mut next = source();
        let mut ids = IdSource::new(&b, &mut next, 1000);
        let op = to_operation(
            &l,
            &b,
            "TOP",
            "bottom",
            &Placement::default(),
            &RfgenSettings::default(),
            &mut ids,
        )
        .unwrap();
        let inverse = b.apply(&op).unwrap();
        assert_eq!(b.zones.len(), 1);
        assert_eq!(b.tracks.len(), 1);
        assert_eq!(b.vias.len(), 1);
        // RF existed, GND was added
        assert_eq!(b.nets.len(), 2);
        assert!(b.is_consistent(), "{:?}", b.validate());
        b.apply(&inverse).unwrap();
        assert_eq!(b, before);
    }

    #[test]
    fn a_part_gets_a_footprint_pads_with_nets_and_a_free_reference() {
        let mut b = board();
        let mut l = Layout::new();
        let rf = l.net("RF");
        let gnd = l.net("GND");
        l.parts.push(PartSpec {
            prefix: "J".into(),
            value: "SMA".into(),
            footprint: "SMA-EDGE".into(),
            at: (0, 0),
            pads: vec![
                PadSpec {
                    number: "1".into(),
                    width: 2_000_000,
                    height: 1_500_000,
                    at: (1_000_000, 0),
                    net: rf,
                },
                PadSpec {
                    number: "2".into(),
                    width: 2_000_000,
                    height: 2_000_000,
                    at: (1_000_000, 2_000_000),
                    net: gnd,
                },
            ],
        });
        let mut next = source();
        let mut ids = IdSource::new(&b, &mut next, 1000);
        let placement = Placement::new(10_000_000, 0, 90).unwrap();
        let op = to_operation(
            &l,
            &b,
            "top",
            "bottom",
            &placement,
            &RfgenSettings::default(),
            &mut ids,
        )
        .unwrap();
        b.apply(&op).unwrap();
        assert_eq!(b.footprints.len(), 1);
        assert_eq!(b.components.len(), 1);
        assert_eq!(b.pad_nets.len(), 2);
        let component = b.components.values().next().unwrap();
        assert_eq!(component.refdes, "J1");
        assert_eq!(component.rotation.get(), 90_000_000);
        assert!(b.is_consistent(), "{:?}", b.validate());
    }

    #[test]
    fn a_layer_that_is_missing_or_too_many_items_are_refused() {
        let b = board();
        let mut l = Layout::new();
        let rf = l.net("RF");
        l.rect(rf, 0, 0, 10, 10);
        let mut next = source();
        let mut ids = IdSource::new(&b, &mut next, 1000);
        let missing = to_operation(
            &l,
            &b,
            "nowhere",
            "bottom",
            &Placement::default(),
            &RfgenSettings::default(),
            &mut ids,
        )
        .unwrap_err();
        assert_eq!(missing.code, LAYER_NOT_FOUND);
        let settings = RfgenSettings {
            max_items: 0,
            ..RfgenSettings::default()
        };
        let too_many = to_operation(
            &l,
            &b,
            "top",
            "bottom",
            &Placement::default(),
            &settings,
            &mut ids,
        )
        .unwrap_err();
        assert_eq!(too_many.code, LIMIT_EXCEEDED);
    }

    #[test]
    fn ids_the_board_already_uses_are_skipped_and_a_dry_source_is_an_error() {
        let b = board();
        let first = b.layers.keys().next().copied().unwrap();
        // a source that gives a used id twice and then a new one
        let mut sequence = vec![first, first];
        let mut generator = source();
        let mut next = move || {
            if let Some(id) = sequence.pop() {
                id
            } else {
                // the board layers use the first two ids of this generator
                generator()
            }
        };
        let mut ids = IdSource::new(&b, &mut next, 10);
        let got = ids.fresh().unwrap();
        assert!(!b.layers.contains_key(&got));
        let mut stuck = move || first;
        let mut dry = IdSource::new(&b, &mut stuck, 5);
        assert_eq!(dry.fresh().unwrap_err().code, ID_UNAVAILABLE);
    }
}
