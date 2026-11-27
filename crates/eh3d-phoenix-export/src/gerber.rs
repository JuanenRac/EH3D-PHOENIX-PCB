//! A layer of the board as a Gerber X2 file.
//!
//! The drawing items of the layer are collected in a fixed order (zones, tracks, vias, pads, each
//! by id), the apertures are numbered from 10 in the order of their sorted shapes, and the file is
//! written by the deterministic writer of EH3D-CORE-LIBS: the same board and options always give
//! the same bytes.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_parser_gerber::{
    Aperture, Attribute, Command, CoordinateFormat, Dialect, GerberFile, ImageParams, Operation,
    Point as GPoint, Polarity, QuadrantMode, Units, ZeroOmission,
};
use eh3d_phoenix_board::connectivity::{pad_position, point_in_polygon};
use eh3d_phoenix_board::{
    arc_chords, ArcSettings, Board, Component, Id, LayerKind, Pad, PadRef, PadShape, Segment, Via,
    Zone,
};
use eh3d_phoenix_model::{segments_intersect, Point, Udeg};
use eh3d_writer_gerber::{GerberOptions, GerberOutput, WriteError};

use crate::codes;
use crate::islands;
use crate::poly;
use crate::profile::{ObjectAttribute, X2Level, X2Profile};
use crate::relief::{self, relief_gaps, ReliefShape};
use crate::x2::{self, ApFunction, FieldError, Fields, FileInput, ObjectSet, NO_NET};

/// The coordinate format of the files unless a profile says otherwise: 4 digits before and 6 after
/// the point, in millimetres.
pub const FORMAT: CoordinateFormat = CoordinateFormat {
    zero_omission: ZeroOmission::Leading,
    integer_digits: 4,
    decimal_digits: 6,
};

/// The smallest size, in nanometres, an aperture takes when a mask expansion or a paste shrink would
/// make it vanish, so that no pad is lost from a file.
pub const MIN_APERTURE_NM: i64 = 1_000;

/// The value of the `.GenerationSoftware` attribute: vendor, product and the version of this crate.
pub const GENERATION_SOFTWARE: [&str; 3] = ["EH3D", "PHOENIX-PCB", env!("CARGO_PKG_VERSION")];

/// The format of the files with `decimals` digits after the point (1 to 6).
pub fn format_with_decimals(decimals: u32) -> CoordinateFormat {
    CoordinateFormat {
        zero_omission: ZeroOmission::Leading,
        integer_digits: 4,
        decimal_digits: decimals,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ApKey {
    Circle(i64),
    Rect(i64, i64),
    Obround(i64, i64),
}

/// An aperture: its shape and, at the levels of X2 that name it, its function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Ap {
    key: ApKey,
    function: Option<ApFunction>,
}

enum Draw {
    Flash(Ap, Point),
    Line(Ap, Point, Point),
    Arc(Ap, Point, Point, (f64, f64), bool),
    /// A filled polygon, with the function of the pad it is (none for a zone or a clearing).
    Region(Vec<Point>, Option<ApFunction>),
    /// What follows is drawn with this polarity until the next change.
    Polarity(Polarity),
}

/// How many straight pieces make the quarter of a circle at the corner of a rounded pad.
pub(crate) const CORNER_STEPS: i64 = 6;

/// True when the export writes the pad as a region and not as a flash: a pad with rounded
/// corners, and a rectangle or an oblong pad whose angle on the board is not a right angle (an
/// aperture of those shapes cannot be turned).
pub fn pad_is_region(pad: &Pad, component: &Component) -> bool {
    !matches!(pad.shape, PadShape::Circle { .. })
        && (pad.corner_radius_nm().is_some()
            || component.pad_angle_udeg(pad.rotation) % 90_000_000 != 0)
}

/// The outline of a pad that is written as a region: every side grown by half of `delta`, turned
/// to `angle` about `at`.
fn pad_ring(at: Point, pad: &Pad, angle: i64, delta: i64) -> Vec<Point> {
    let grow = |v: i64| v.saturating_add(delta).max(MIN_APERTURE_NM);
    let shape = match pad.shape {
        PadShape::Circle { diameter } => ReliefShape::Circle(grow(diameter.get())),
        PadShape::Oblong { width, height } => {
            ReliefShape::Obround(grow(width.get()), grow(height.get()))
        }
        PadShape::Rect { width, height } => {
            let (w, h) = (grow(width.get()), grow(height.get()));
            match pad.corner_radius_nm() {
                Some(radius) => ReliefShape::Rounded(w, h, (radius + delta / 2).max(0)),
                None => ReliefShape::Rect(w, h),
            }
        }
    };
    // at a right angle a rounded pad is cut in whole nanometres, as it always was
    if let (ReliefShape::Rounded(w, h, radius), 0) = (shape, angle % 90_000_000) {
        let (w, h) = if (angle / 90_000_000) % 2 == 1 {
            (h, w)
        } else {
            (w, h)
        };
        return rounded_rectangle(at, w, h, radius);
    }
    poly::tidy(
        relief::outline(shape, 0.0)
            .into_iter()
            .filter_map(|(x, y)| poly::place(at, x, y, angle))
            .collect(),
    )
}

/// The polygon that holds the keep-away of `clearance` around a pad turned by an angle that is
/// not a right angle.
fn turned_keep_away(at: Point, shape: PadShape, angle: i64, clearance: i64) -> Vec<Point> {
    let grow = clearance.saturating_mul(2);
    let (form, room) = match shape {
        PadShape::Circle { diameter } => {
            let d = diameter.get().saturating_add(grow);
            (ReliefShape::Circle(d), poly::chord_room(d / 2, 32))
        }
        PadShape::Rect { width, height } => (
            ReliefShape::Rect(
                width.get().saturating_add(grow),
                height.get().saturating_add(grow),
            ),
            0,
        ),
        PadShape::Oblong { width, height } => {
            let (w, h) = (
                width.get().saturating_add(grow),
                height.get().saturating_add(grow),
            );
            (
                ReliefShape::Obround(w, h),
                poly::chord_room(w.min(h) / 2, 32),
            )
        }
    };
    poly::tidy(
        relief::outline(form, room as f64)
            .into_iter()
            .filter_map(|(x, y)| poly::place(at, x, y, angle))
            .collect(),
    )
}

/// The corners of a rectangle of `width` by `height` about `at` with corners of `radius`, counter
/// clockwise, the corner arcs cut into `CORNER_STEPS` pieces each. The radius is at most half of
/// the shorter side.
pub(crate) fn rounded_rectangle(at: Point, width: i64, height: i64, radius: i64) -> Vec<Point> {
    let radius = radius.clamp(0, width.min(height) / 2);
    let (cx, cy) = (at.x_nm.get(), at.y_nm.get());
    let (hx, hy) = (width / 2 - radius, height / 2 - radius);
    let mut out: Vec<Point> = Vec::new();
    // the centres of the four corner arcs and the angle where each starts, counter clockwise from
    // the lower right corner
    let corners = [
        (hx, -hy, -90.0_f64),
        (hx, hy, 0.0),
        (-hx, hy, 90.0),
        (-hx, -hy, 180.0),
    ];
    for (dx, dy, start) in corners {
        for step in 0..=CORNER_STEPS {
            let angle = (start + 90.0 * step as f64 / CORNER_STEPS as f64).to_radians();
            let x = cx + dx + (radius as f64 * angle.cos()).round() as i64;
            let y = cy + dy + (radius as f64 * angle.sin()).round() as i64;
            if let Ok(p) = Point::from_nm(x, y) {
                if out.last() != Some(&p) {
                    out.push(p);
                }
            }
        }
    }
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// A drawn object with the object attributes it carries.
type Drawn = (Draw, ObjectSet);

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

fn gp(p: Point) -> GPoint {
    GPoint {
        x: mm(p.x_nm.get()),
        y: mm(p.y_nm.get()),
    }
}

/// How many edges of the outline of the board are written as arcs: the arcs that can be used and
/// have a centre.
pub(crate) fn outline_arc_count(board: &Board) -> usize {
    let n = board.outline.len();
    board
        .usable_outline_arcs()
        .iter()
        .filter(|(edge, sweep)| {
            arc_centre(
                board.outline[**edge],
                board.outline[(**edge + 1) % n],
                sweep.get(),
            )
            .is_some()
        })
        .count()
}

/// The offsets `(I, J)` from the start of an arc to its centre, in millimetres, and whether it runs
/// clockwise. `None` when the arc has no computable centre.
pub(crate) fn arc_centre(start: Point, end: Point, sweep_udeg: i64) -> Option<((f64, f64), bool)> {
    let (sx, sy) = (mm(start.x_nm.get()), mm(start.y_nm.get()));
    let (ex, ey) = (mm(end.x_nm.get()), mm(end.y_nm.get()));
    let (cx, cy) = (ex - sx, ey - sy);
    let d = (cx * cx + cy * cy).sqrt();
    let theta = (sweep_udeg.abs() as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
    let half = theta / 2.0;
    let s = half.sin();
    if d == 0.0 || s.abs() < 1e-9 || !d.is_finite() {
        return None;
    }
    let r = d / (2.0 * s);
    let h = r * half.cos();
    let (nx, ny) = (-cy / d, cx / d);
    let sign = if sweep_udeg >= 0 { 1.0 } else { -1.0 };
    let (mx, my) = (sx + cx / 2.0, sy + cy / 2.0);
    let (ox, oy) = (mx + sign * nx * h, my + sign * ny * h);
    Some(((ox - sx, oy - sy), sweep_udeg < 0))
}

fn pad_key(shape: PadShape, swapped: bool) -> ApKey {
    match shape {
        PadShape::Circle { diameter } => ApKey::Circle(diameter.get()),
        PadShape::Rect { width, height } => {
            let (w, h) = (width.get(), height.get());
            if swapped {
                ApKey::Rect(h, w)
            } else {
                ApKey::Rect(w, h)
            }
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (width.get(), height.get());
            if swapped {
                ApKey::Obround(h, w)
            } else {
                ApKey::Obround(w, h)
            }
        }
    }
}

fn push_pieces(
    draws: &mut Vec<Drawn>,
    ap: Ap,
    set: &ObjectSet,
    path: &[Segment],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for s in path {
        match s {
            Segment::Line { start, end } => draws.push((Draw::Line(ap, *start, *end), set.clone())),
            Segment::Arc { start, end, sweep } => match arc_centre(*start, *end, sweep.get()) {
                Some((offset, clockwise)) => {
                    draws.push((Draw::Arc(ap, *start, *end, offset, clockwise), set.clone()))
                }
                None => diagnostics.push(codes::diagnostic(
                    codes::ARC_SKIPPED,
                    Severity::Warning,
                    "an arc of a track or of the silkscreen",
                )),
            },
        }
    }
}

/// Grows (or, when negative, shrinks) every dimension of an aperture by `delta` nanometres.
fn adjust(key: ApKey, delta: i64) -> ApKey {
    let f = |v: i64| v.saturating_add(delta).max(MIN_APERTURE_NM);
    match key {
        ApKey::Circle(d) => ApKey::Circle(f(d)),
        ApKey::Rect(w, h) => ApKey::Rect(f(w), f(h)),
        ApKey::Obround(w, h) => ApKey::Obround(f(w), f(h)),
    }
}

/// True when the reference starts with one of the prefixes and goes on with anything but a letter.
fn is_test_reference(refdes: &str, prefixes: &[String]) -> bool {
    let up = refdes.to_ascii_uppercase();
    prefixes.iter().any(|prefix| {
        up.strip_prefix(&prefix.to_ascii_uppercase())
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphabetic()))
    })
}

/// The function of the aperture of a pad. A pad the designer marked as a test point is a test pad
/// whatever the words of the profile say.
fn pad_function(
    pad: &Pad,
    footprint_name: &str,
    refdes: &str,
    marked: bool,
    x2: &X2Profile,
) -> ApFunction {
    let name = footprint_name.to_lowercase();
    if x2
        .fiducial_names
        .iter()
        .any(|word| name.contains(&word.to_lowercase()))
    {
        ApFunction::fiducial(x2.fiducial_scope)
    } else if x2
        .heatsink_pad_numbers
        .iter()
        .any(|number| number.eq_ignore_ascii_case(&pad.number))
    {
        ApFunction::HeatsinkPad
    } else if marked
        || is_test_reference(refdes, &x2.testpoint_prefixes)
        || x2
            .testpoint_names
            .iter()
            .any(|word| name.contains(&word.to_lowercase()))
    {
        ApFunction::TestPad
    } else if pad.drill.is_some() {
        ApFunction::ComponentPad
    } else {
        ApFunction::SmdPad
    }
}

/// The thermal relief that the pads of the net of a pour get.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZoneThermal {
    /// 0 leaves the pads solid; 4 clears four gaps and leaves four spokes along the axes; 2
    /// clears two gaps (above and below) and leaves a spoke on each side.
    pub spokes: u32,
    /// Whether the pads that are not round (rectangles and oblongs at a right angle) get the
    /// relief too. Without it only the round pads do.
    pub non_round: bool,
    /// The width of each gap, in nanometres.
    pub gap_nm: i64,
    /// The width of each spoke, in nanometres.
    pub spoke_width_nm: i64,
}

/// The four gaps of the thermal relief of a round pad at `at`, as closed polygons. Each is the
/// part of the ring between the edge of the pad and `gap` beyond it that lies between two of the
/// four spokes, which run along the axes and are `spoke` wide; the arcs are sampled in six steps.
/// Nothing when the gap or the spoke is not positive or the spoke is as wide as the pad.
pub fn thermal_gaps(at: Point, diameter: i64, gap: i64, spoke: i64) -> Vec<Vec<Point>> {
    let r_in = diameter as f64 / 2.0;
    let r_out = r_in + gap as f64;
    let half = spoke as f64 / 2.0;
    if gap <= 0 || spoke <= 0 || half >= r_in {
        return Vec::new();
    }
    const STEPS: usize = 6;
    // where the edge of a spoke meets a circle of the radius r, as an angle from the axis
    let start = |r: f64| (half / r).asin();
    let arc = |r: f64, j: usize| {
        let a = start(r);
        let theta = a + (std::f64::consts::FRAC_PI_2 - 2.0 * a) * j as f64 / STEPS as f64;
        (r * theta.cos(), r * theta.sin())
    };
    let mut quadrant: Vec<(f64, f64)> = (0..=STEPS).map(|j| arc(r_in, j)).collect();
    quadrant.extend((0..=STEPS).rev().map(|j| arc(r_out, j)));
    let (ax, ay) = (at.x_nm.get(), at.y_nm.get());
    let mut out: Vec<Vec<Point>> = Vec::new();
    for turns in 0..4 {
        let ring: Option<Vec<Point>> = quadrant
            .iter()
            .map(|&(x, y)| {
                let (x, y) = match turns {
                    0 => (x, y),
                    1 => (-y, x),
                    2 => (-x, -y),
                    _ => (y, -x),
                };
                Point::from_nm(ax + x.round() as i64, ay + y.round() as i64).ok()
            })
            .collect();
        if let Some(ring) = ring {
            out.push(ring);
        }
    }
    out
}

/// The distance in nanometres from a point to the straight piece from `a` to `b`.
fn point_piece_distance(p: Point, a: Point, b: Point) -> f64 {
    let xy = |q: Point| (q.x_nm.get() as f64, q.y_nm.get() as f64);
    let ((px, py), (ax, ay), (bx, by)) = (xy(p), xy(a), xy(b));
    let (dx, dy) = (bx - ax, by - ay);
    let length2 = dx * dx + dy * dy;
    let t = if length2 > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / length2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (px - ax - t * dx).hypot(py - ay - t * dy)
}

/// Whether an object that reaches `radius` about the straight piece from `a` to `b` (a point when
/// both are the same) takes copper away from a pour with this outline: an end inside it, the piece
/// across its border, or nearer to the border than the radius. It never says no when it does.
fn takes_from(outline: &[Point], a: Point, b: Point, radius: i64) -> bool {
    if point_in_polygon(a, outline) || point_in_polygon(b, outline) {
        return true;
    }
    // a nanometre more, for the rounding of the distance
    let reach = radius as f64 + 1.0;
    let n = outline.len();
    (0..n).any(|i| {
        let (p, q) = (outline[i], outline[(i + 1) % n]);
        segments_intersect(a, b, p, q)
            || point_piece_distance(a, p, q) <= reach
            || point_piece_distance(b, p, q) <= reach
            || point_piece_distance(p, a, b) <= reach
            || point_piece_distance(q, a, b) <= reach
    })
}

/// How far a flash of this aperture reaches from its centre: the radius of a circle around it.
fn flash_reach(key: ApKey) -> i64 {
    match key {
        ApKey::Circle(d) => d / 2 + d % 2,
        ApKey::Rect(w, h) => ((w as f64).hypot(h as f64) / 2.0).ceil() as i64,
        ApKey::Obround(w, h) => w.max(h) / 2 + w.max(h) % 2,
    }
}

/// The width of the room that the hole of a pad that is not plated takes: its drill, or, for an
/// oblong pad, the length of the slot that may be milled for it.
fn hole_width(shape: PadShape, drill: i64) -> i64 {
    let PadShape::Oblong { width, height } = shape else {
        return drill;
    };
    let (long, short) = (width.get().max(height.get()), width.get().min(height.get()));
    if short <= 0 {
        return drill;
    }
    let slot = i128::from(long) * i128::from(drill) / i128::from(short);
    i64::try_from(slot).unwrap_or(i64::MAX).max(drill)
}

/// The objects that clear the copper of a zone: its own cutouts, the gaps of the thermal relief
/// of the round pads of its own net, when the profile asks for them, and a keep-away of its
/// clearance around the tracks, vias and pads of other nets that lie on the layer and around the
/// holes that are not plated, which go through every layer. An object is drawn when its keep-away
/// takes copper from this pour, wherever its ends are. All are drawn
/// with clear polarity, between a change to clear and a change back to dark, so that what is
/// drawn after them (the copper of the board) stays. Nothing when there is nothing to clear. The
/// aperture of a keep-away has the function of the object it keeps away from.
fn zone_clearing(
    board: &Board,
    zone: &Zone,
    layer: Id,
    x2: &X2Profile,
    thermal: &ZoneThermal,
    earlier: &[&Zone],
) -> Vec<Drawn> {
    let named = x2.level.has_aperture_attributes();
    let ap = |key: ApKey, function: ApFunction| Ap {
        key,
        function: named.then_some(function),
    };
    let none = ObjectSet::default;
    let mut clear: Vec<Drawn> = Vec::new();
    for cutout in zone.cutouts.iter().filter(|c| c.len() >= 3) {
        clear.push((Draw::Region(cutout.clone(), None), none()));
    }
    let c = zone.clearance.get();
    if c > 0 && zone.outline.len() >= 3 {
        let grow = c.saturating_mul(2);
        let xs = zone.outline.iter().map(|p| p.x_nm.get());
        let ys = zone.outline.iter().map(|p| p.y_nm.get());
        let (x0, x1) = (xs.clone().min().unwrap_or(0), xs.max().unwrap_or(0));
        let (y0, y1) = (ys.clone().min().unwrap_or(0), ys.max().unwrap_or(0));
        // whether the keep-away of a piece, `radius` about it, takes copper from the pour: first
        // its box against the box of the zone, then the outline itself
        let takes = |a: Point, b: Point, radius: i64| {
            let (ax, ay) = (a.x_nm.get(), a.y_nm.get());
            let (bx, by) = (b.x_nm.get(), b.y_nm.get());
            ax.max(bx).saturating_add(radius) >= x0
                && ax.min(bx).saturating_sub(radius) <= x1
                && ay.max(by).saturating_add(radius) >= y0
                && ay.min(by).saturating_sub(radius) <= y1
                && takes_from(&zone.outline, a, b, radius)
        };
        for track in board
            .tracks
            .values()
            .filter(|t| t.layer == layer && t.net != zone.net)
        {
            let width = track.width.get().saturating_add(grow);
            let key = ap(ApKey::Circle(width), ApFunction::Conductor);
            let radius = flash_reach(ApKey::Circle(width));
            for piece in &track.path {
                match piece {
                    Segment::Line { start, end } => {
                        if takes(*start, *end, radius) {
                            clear.push((Draw::Line(key, *start, *end), none()));
                        }
                    }
                    Segment::Arc { start, end, sweep } => {
                        let Some((offset, clockwise)) = arc_centre(*start, *end, sweep.get())
                        else {
                            continue;
                        };
                        // the arc is judged as its chords, which are within `doubt` of it; an arc
                        // that cannot be cut is cleared wherever the zone is
                        let near = arc_chords(*start, *end, *sweep, &ArcSettings::current())
                            .is_none_or(|(points, doubt)| {
                                points
                                    .windows(2)
                                    .any(|w| takes(w[0], w[1], radius.saturating_add(doubt)))
                            });
                        if near {
                            clear.push((Draw::Arc(key, *start, *end, offset, clockwise), none()));
                        }
                    }
                }
            }
        }
        for via in board
            .vias
            .values()
            .filter(|v| v.net != zone.net && via_on_layer(board, v, layer))
        {
            let key = ApKey::Circle(via.diameter.get().saturating_add(grow));
            if takes(via.at, via.at, flash_reach(key)) {
                clear.push((Draw::Flash(ap(key, ApFunction::ViaPad), via.at), none()));
            }
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
                // a hole that is not plated is drilled through every layer, whatever the layers
                // of its pad are, and no copper may reach its wall
                let hole = pad.drill.filter(|_| pad.is_npth());
                if hole.is_none()
                    && (!pad.layers.contains(&layer) || board.net_of(pad_ref) == Some(zone.net))
                {
                    continue;
                }
                let Some(at) = pad_position(board, pad_ref) else {
                    continue;
                };
                let total = component.pad_angle_udeg(pad.rotation);
                let swapped = (total / 90_000_000) % 2 == 1 && total % 90_000_000 == 0;
                let turned = hole.is_none()
                    && total % 90_000_000 != 0
                    && !matches!(pad.shape, PadShape::Circle { .. });
                let key = match hole {
                    Some(drill) => {
                        ApKey::Circle(hole_width(pad.shape, drill.get()).saturating_add(grow))
                    }
                    None => adjust(pad_key(pad.shape, swapped), grow),
                };
                if !takes(at, at, flash_reach(key)) {
                    continue;
                }
                // the keep-away of a hole is the aperture of no copper object: as it is only drawn
                // clear, it is written with the function of a conductor, as that of a track is
                let function = match hole {
                    Some(_) => ApFunction::Conductor,
                    None => pad_function(
                        pad,
                        &footprint.name,
                        &component.refdes,
                        board.is_test_point(pad_ref),
                        x2,
                    ),
                };
                if turned {
                    let ring = turned_keep_away(at, pad.shape, total, c);
                    clear.push((Draw::Region(ring, None), none()));
                } else {
                    clear.push((Draw::Flash(ap(key, function), at), none()));
                }
            }
        }
    }
    if matches!(thermal.spokes, 2 | 4) {
        for (cid, component) in &board.components {
            let Some(footprint) = board.footprints.get(&component.footprint) else {
                continue;
            };
            for (pid, pad) in &footprint.pads {
                if !pad.layers.contains(&layer) || pad.is_npth() {
                    continue;
                }
                let pad_ref = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                if board.net_of(pad_ref) != Some(zone.net) {
                    continue;
                }
                let total = component.pad_angle_udeg(pad.rotation);
                // the gaps of a pad that is not at a right angle are made along its own axes and
                // turned with it
                let tilted =
                    total % 90_000_000 != 0 && !matches!(pad.shape, PadShape::Circle { .. });
                let shape = match pad.shape {
                    PadShape::Circle { diameter } => ReliefShape::Circle(diameter.get()),
                    PadShape::Rect { .. } | PadShape::Oblong { .. } if !thermal.non_round => {
                        continue;
                    }
                    PadShape::Rect { .. } | PadShape::Oblong { .. } => {
                        match pad_key(pad.shape, !tilted && (total / 90_000_000) % 2 == 1) {
                            // a pad with rounded corners keeps them in the ring of the relief
                            ApKey::Rect(w, h) => match pad.corner_radius_nm() {
                                Some(radius) => ReliefShape::Rounded(w, h, radius),
                                None => ReliefShape::Rect(w, h),
                            },
                            ApKey::Obround(w, h) => ReliefShape::Obround(w, h),
                            ApKey::Circle(d) => ReliefShape::Circle(d),
                        }
                    }
                };
                let Some(at) = pad_position(board, pad_ref) else {
                    continue;
                };
                if !point_in_polygon(at, &zone.outline) {
                    continue;
                }
                let about = if tilted {
                    Point::from_nm(0, 0).unwrap_or(at)
                } else {
                    at
                };
                let gaps = match (shape, thermal.spokes) {
                    (ReliefShape::Circle(d), 4) => {
                        thermal_gaps(about, d, thermal.gap_nm, thermal.spoke_width_nm)
                    }
                    (other, spokes) => {
                        relief_gaps(other, about, thermal.gap_nm, thermal.spoke_width_nm, spokes)
                    }
                };
                for gap in gaps {
                    let gap = if tilted {
                        gap.iter()
                            .filter_map(|p| {
                                let (x, y) = (p.x_nm.get() as f64, p.y_nm.get() as f64);
                                poly::place(at, x, y, total)
                            })
                            .collect()
                    } else {
                        gap
                    };
                    clear.push((Draw::Region(gap, None), none()));
                }
            }
        }
    }
    let clear = keep_inside(clear, zone, earlier);
    if clear.is_empty() {
        return clear;
    }
    let mut out: Vec<Drawn> = Vec::with_capacity(clear.len() + 2);
    out.push((Draw::Polarity(Polarity::Clear), none()));
    out.extend(clear);
    out.push((Draw::Polarity(Polarity::Dark), none()));
    out
}

/// The filled polygons that a clearing draws.
fn clearing_shapes(draw: &Draw) -> Vec<Vec<Point>> {
    let convex = |at: Point, form: ReliefShape, room: i64| -> Vec<Point> {
        relief::outline(form, room as f64)
            .into_iter()
            .filter_map(|(x, y)| poly::place(at, x, y, 0))
            .collect()
    };
    match draw {
        Draw::Flash(ap, at) => match ap.key {
            ApKey::Circle(d) => vec![poly::stadium(*at, *at, d)],
            ApKey::Rect(w, h) => vec![convex(*at, ReliefShape::Rect(w, h), 0)],
            ApKey::Obround(w, h) => vec![convex(
                *at,
                ReliefShape::Obround(w, h),
                poly::chord_room(w.min(h) / 2, 32),
            )],
        },
        Draw::Line(ap, from, to) => vec![poly::stadium(*from, *to, flash_reach(ap.key) * 2)],
        Draw::Arc(ap, from, to, (i, j), clockwise) => {
            // the sweep again from the centre, to cut the arc into its chords
            let (cx, cy) = (
                from.x_nm.get() as f64 + i * 1_000_000.0,
                from.y_nm.get() as f64 + j * 1_000_000.0,
            );
            let angle = |p: &Point| (p.y_nm.get() as f64 - cy).atan2(p.x_nm.get() as f64 - cx);
            let mut sweep = (angle(to) - angle(from)).to_degrees().rem_euclid(360.0);
            if *clockwise {
                sweep -= 360.0;
            }
            let width = flash_reach(ap.key) * 2;
            Udeg::new((sweep * 1_000_000.0).round() as i64)
                .ok()
                .and_then(|sweep| arc_chords(*from, *to, sweep, &ArcSettings::current()))
                .map(|(points, doubt)| {
                    let wide = width.saturating_add(doubt.saturating_mul(2));
                    points
                        .windows(2)
                        .map(|w| poly::stadium(w[0], w[1], wide))
                        .collect()
                })
                .unwrap_or_default()
        }
        Draw::Region(points, _) => vec![points.clone()],
        Draw::Polarity(_) => Vec::new(),
    }
}

/// Clear polarity takes away everything drawn before it, other pours too. A clearing of `zone`
/// that leaves the zone and reaches a pour drawn earlier on the layer is therefore written as the
/// regions of its part inside `zone`; any other clearing is left as it is.
fn keep_inside(clear: Vec<Drawn>, zone: &Zone, earlier: &[&Zone]) -> Vec<Drawn> {
    if earlier.is_empty() {
        return clear;
    }
    let mut out: Vec<Drawn> = Vec::with_capacity(clear.len());
    for (draw, set) in clear {
        let shapes = clearing_shapes(&draw);
        let spills = shapes.iter().any(|shape| {
            !poly::within(shape, &zone.outline)
                && earlier
                    .iter()
                    .any(|other| poly::meet(shape, &other.outline))
        });
        if !spills {
            out.push((draw, set));
            continue;
        }
        for shape in &shapes {
            for part in poly::intersection(shape, &zone.outline) {
                out.push((Draw::Region(part, None), set.clone()));
            }
        }
    }
    out
}

/// The pours of a layer in the order they are drawn: by id, but a pour that lies inside the
/// outline of a larger one (in one of its cutouts) after it, so that the cutout of the larger pour
/// does not take the smaller one away.
fn drawing_order(zones: Vec<&Zone>) -> Vec<&Zone> {
    let boxes: Vec<(i64, i64, i64, i64)> = zones.iter().map(|z| poly::bounds(&z.outline)).collect();
    let areas: Vec<i128> = zones
        .iter()
        .map(|z| poly::double_area(&z.outline).abs())
        .collect();
    let mut order: Vec<(usize, usize)> = (0..zones.len())
        .map(|i| {
            let depth = (0..zones.len())
                .filter(|&j| {
                    areas[j] > areas[i]
                        && boxes[j].0 <= boxes[i].0
                        && boxes[j].1 <= boxes[i].1
                        && boxes[j].2 >= boxes[i].2
                        && boxes[j].3 >= boxes[i].3
                        && zones[i]
                            .outline
                            .iter()
                            .all(|p| point_in_polygon(*p, &zones[j].outline))
                })
                .count();
            (depth, i)
        })
        .collect();
    order.sort_unstable();
    order.into_iter().map(|(_, i)| zones[i]).collect()
}

/// The strips of width `2 * clearance` along the board outline and its openings, drawn clear, so
/// that no pour reaches within `clearance` of the edge.
fn edge_pull_back(board: &Board, clearance: i64, x2: &X2Profile) -> Vec<Drawn> {
    if clearance <= 0 || board.outline.len() < 3 {
        return Vec::new();
    }
    let ap = Ap {
        key: ApKey::Circle(clearance.saturating_mul(2)),
        function: x2
            .level
            .has_aperture_attributes()
            .then_some(ApFunction::Conductor),
    };
    let mut out: Vec<Drawn> = vec![(Draw::Polarity(Polarity::Clear), ObjectSet::default())];
    out.extend(outline_draws(board, ap));
    out.push((Draw::Polarity(Polarity::Dark), ObjectSet::default()));
    out
}

/// The outline and then every opening inside it, each a closed contour drawn with `line`.
fn outline_draws(board: &Board, line: Ap) -> Vec<Drawn> {
    let mut draws: Vec<Drawn> = Vec::new();
    for (r, ring) in std::iter::once(&board.outline)
        .chain(board.outline_cutouts.iter())
        .enumerate()
    {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        // only the outline itself has arcs; each is written as a circular interpolation
        let arcs = if r == 0 {
            board.usable_outline_arcs()
        } else {
            BTreeMap::new()
        };
        for i in 0..n {
            let (from, to) = (ring[i], ring[(i + 1) % n]);
            let arc = arcs
                .get(&i)
                .and_then(|sweep| arc_centre(from, to, sweep.get()));
            let draw = match arc {
                Some((offset, clockwise)) => Draw::Arc(line, from, to, offset, clockwise),
                None => Draw::Line(line, from, to),
            };
            draws.push((draw, ObjectSet::default()));
        }
    }
    draws
}

fn collect(
    board: &Board,
    spec: &LayerFileSpec,
    outline_width: i64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Drawn> {
    let (layer, kind) = (spec.layer, spec.kind);
    let x2 = &spec.x2;
    let named = x2.level.has_aperture_attributes();
    let ap = |key: ApKey, function: ApFunction| Ap {
        key,
        function: named.then_some(function),
    };
    let plain = |key: ApKey| Ap {
        key,
        function: None,
    };
    let delta = match kind {
        LayerKind::SolderMask => spec.mask_expansion.saturating_mul(2),
        LayerKind::Paste => spec.paste_shrink.saturating_mul(-2),
        _ => 0,
    };
    let is_copper = kind == LayerKind::Copper;
    let net_name = |id: Id| -> String {
        board
            .nets
            .get(&id)
            .map(|n| n.name.as_str())
            .filter(|n| !n.is_empty())
            .unwrap_or(NO_NET)
            .to_owned()
    };
    let net_set = |id: Id| ObjectSet {
        net: (is_copper && x2.writes(ObjectAttribute::Net)).then(|| net_name(id)),
        ..ObjectSet::default()
    };
    let mut draws: Vec<Drawn> = Vec::new();
    if is_copper {
        let mut zones = 0usize;
        let small = islands::dropped_zones(board, layer, spec.zone_island_min_area_nm2);
        let written: Vec<&Zone> = board
            .zones
            .iter()
            .filter(|(id, z)| {
                z.layer == layer && z.outline.len() >= 3 && !small.zones.contains(*id)
            })
            .map(|(_, z)| z)
            .collect();
        let mut earlier: Vec<&Zone> = Vec::new();
        for z in drawing_order(written) {
            draws.push((Draw::Region(z.outline.clone(), None), net_set(z.net)));
            zones += 1;
            // the openings of the pour and the keep-away of other copper are cleared out of it,
            // before the copper of the board is drawn on top
            draws.extend(zone_clearing(
                board,
                z,
                layer,
                x2,
                &spec.zone_thermal,
                &earlier,
            ));
            earlier.push(z);
        }
        if zones > 0 {
            draws.extend(edge_pull_back(board, spec.zone_edge_clearance, x2));
        }
        if zones > 0 {
            diagnostics.push(codes::diagnostic(
                codes::ZONE_NOT_POURED,
                Severity::Info,
                &format!("{zones} zone(s) on one layer"),
            ));
        }
        if small.islands > 0 {
            diagnostics.push(codes::diagnostic(
                codes::ISLAND_LEFT_OUT,
                Severity::Info,
                &format!(
                    "{} island(s) of {} zone(s) on one layer",
                    small.islands,
                    small.zones.len()
                ),
            ));
        }
        for t in board.tracks.values().filter(|t| t.layer == layer) {
            push_pieces(
                &mut draws,
                ap(ApKey::Circle(t.width.get()), ApFunction::Conductor),
                &net_set(t.net),
                &t.path,
                diagnostics,
            );
        }
        for v in board
            .vias
            .values()
            .filter(|v| via_on_layer(board, v, layer))
        {
            draws.push((
                Draw::Flash(
                    ap(ApKey::Circle(v.diameter.get()), ApFunction::ViaPad),
                    v.at,
                ),
                net_set(v.net),
            ));
        }
    }
    if kind == LayerKind::Silkscreen {
        let (mut no_glyph, mut undrawn) = (0usize, 0usize);
        for item in board.silkscreen.values().filter(|i| i.layer() == layer) {
            match item.drawn() {
                Ok(d) => {
                    no_glyph += d.missing_glyphs;
                    push_pieces(
                        &mut draws,
                        plain(ApKey::Circle(d.width.get())),
                        &ObjectSet::default(),
                        &d.pieces,
                        diagnostics,
                    );
                }
                Err(_) => undrawn += 1,
            }
        }
        if no_glyph > 0 {
            diagnostics.push(codes::diagnostic(
                codes::SILK_NO_GLYPH,
                Severity::Warning,
                &format!("{no_glyph} character(s) on one layer"),
            ));
        }
        if undrawn > 0 {
            diagnostics.push(codes::diagnostic(
                codes::SILK_UNDRAWN,
                Severity::Warning,
                &format!("{undrawn} item(s) on one layer"),
            ));
        }
    }
    if kind == LayerKind::Outline {
        // the profile is the outline and then every opening inside it, each a closed contour
        let line = ap(ApKey::Circle(outline_width), ApFunction::Profile);
        draws.extend(outline_draws(board, line));
    }
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            if !pad.layers.contains(&layer) {
                continue;
            }
            if (is_copper || kind == LayerKind::Paste) && pad.is_npth() {
                // a hole that is not plated has no copper and takes no paste
                continue;
            }
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            let total = component.pad_angle_udeg(pad.rotation);
            let swapped = (total / 90_000_000) % 2 == 1 && total % 90_000_000 == 0;
            let refdes = (!component.refdes.is_empty()).then(|| component.refdes.clone());
            let set = ObjectSet {
                net: (is_copper && x2.writes(ObjectAttribute::Net)).then(|| {
                    board
                        .net_of(pad_ref)
                        .map_or_else(|| NO_NET.to_owned(), net_name)
                }),
                pin: refdes
                    .clone()
                    .filter(|_| x2.writes(ObjectAttribute::Pin) && !pad.number.is_empty())
                    .map(|r| (r, pad.number.clone())),
                component: refdes.filter(|_| x2.writes(ObjectAttribute::Component)),
            };
            let function = pad_function(
                pad,
                &footprint.name,
                &component.refdes,
                board.is_test_point(pad_ref),
                x2,
            );
            if pad_is_region(pad, component) {
                let ring = pad_ring(at, pad, total, delta);
                if ring.len() >= 3 {
                    draws.push((Draw::Region(ring, named.then_some(function)), set));
                    continue;
                }
            }
            draws.push((
                Draw::Flash(ap(adjust(pad_key(pad.shape, swapped), delta), function), at),
                set,
            ));
        }
    }
    draws
}

fn aperture_of(key: ApKey) -> Aperture {
    match key {
        ApKey::Circle(d) => Aperture::Circle {
            diameter: mm(d),
            hole: None,
        },
        ApKey::Rect(w, h) => Aperture::Rectangle {
            x: mm(w),
            y: mm(h),
            hole: None,
        },
        ApKey::Obround(w, h) => Aperture::Obround {
            x: mm(w),
            y: mm(h),
            hole: None,
        },
    }
}

/// What a layer needs to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerFileSpec {
    /// The layer.
    pub layer: Id,
    /// Its kind.
    pub kind: LayerKind,
    /// The `.FileFunction` value, when it is known.
    pub file_function: Option<Vec<String>>,
    /// The `.FilePolarity` value (`Positive` or `Negative`), when the kind of layer has one.
    pub polarity: Option<&'static str>,
    /// The coordinate format of the file.
    pub format: CoordinateFormat,
    /// How far, per side, a solder mask opening is larger than its pad, in nanometres.
    pub mask_expansion: i64,
    /// How far, per side, a paste opening is smaller than its pad, in nanometres.
    pub paste_shrink: i64,
    /// The thermal relief of the pads of the net of a pour.
    pub zone_thermal: ZoneThermal,
    /// How far the pours stay from the board edge and its openings, in nanometres; 0 writes them
    /// as drawn.
    pub zone_edge_clearance: i64,
    /// The smallest island of a pour that is written, in square nanometres; 0 writes every zone.
    pub zone_island_min_area_nm2: i64,
    /// The Gerber X2 attributes to write.
    pub x2: X2Profile,
}

/// Why a layer file could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerError {
    /// The writer of the format refused the file.
    Write(WriteError),
    /// The writer of the drill format refused the file.
    Drill(eh3d_writer_excellon::WriteError),
    /// A text cannot go into an attribute field.
    Field(FieldError),
    /// The attribute commands could not be put into the file (a defect of this export).
    Attributes,
}

impl std::fmt::Display for LayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerError::Write(e) => write!(f, "{e}"),
            LayerError::Drill(e) => write!(f, "{e}"),
            LayerError::Field(e) => write!(f, "{e}"),
            LayerError::Attributes => f.write_str("the attribute commands do not fit the file"),
        }
    }
}

impl std::error::Error for LayerError {}

impl From<WriteError> for LayerError {
    fn from(e: WriteError) -> Self {
        LayerError::Write(e)
    }
}

impl From<eh3d_writer_excellon::WriteError> for LayerError {
    fn from(e: eh3d_writer_excellon::WriteError) -> Self {
        LayerError::Drill(e)
    }
}

impl From<FieldError> for LayerError {
    fn from(e: FieldError) -> Self {
        LayerError::Field(e)
    }
}

/// Writes one layer. Returns the file and the diagnostics.
pub fn write_layer(
    board: &Board,
    spec: &LayerFileSpec,
    outline_width: i64,
) -> Result<(GerberOutput, Vec<Diagnostic>), LayerError> {
    let mut diagnostics = Vec::new();
    let draws = collect(board, spec, outline_width, &mut diagnostics);
    let mut keys: Vec<Ap> = draws
        .iter()
        .filter_map(|(d, _)| match d {
            Draw::Flash(k, _) | Draw::Line(k, _, _) | Draw::Arc(k, _, _, _, _) => Some(*k),
            Draw::Region(_, _) | Draw::Polarity(_) => None,
        })
        .collect();
    keys.sort();
    keys.dedup();
    let numbers: BTreeMap<Ap, u32> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, 10 + i as u32))
        .collect();
    let mut file = GerberFile {
        units: Some(Units::Millimetres),
        format: Some(spec.format),
        apertures: numbers
            .iter()
            .map(|(k, n)| (*n, aperture_of(k.key)))
            .collect(),
        macros: BTreeMap::new(),
        attributes: Vec::new(),
        commands: Vec::new(),
        ended: true,
        image: ImageParams::default(),
        dialect: Dialect::default(),
        diagnostics: Vec::new(),
    };
    let mut fields = Fields::new(spec.x2.strict);
    let input = FileInput {
        x2: &spec.x2,
        board_name: &board.name,
        function: spec.file_function.clone(),
        polarity: spec.polarity,
    };
    for (name, values) in x2::file_attributes(&input, &mut fields)? {
        file.attributes.push(Attribute {
            kind: "TF".to_owned(),
            name: name.to_owned(),
            values,
        });
    }
    let objects = spec.x2.level == X2Level::Full;
    let mut changes: Vec<String> = Vec::new();
    let mut in_force = ObjectSet::default();
    let mut current: Option<u32> = None;
    let push = |file: &mut GerberFile, op: Operation| {
        file.commands.push(Command { line: 0, op });
    };
    for (d, set) in &draws {
        let mut text = String::new();
        if objects {
            text = x2::object_changes(&in_force, set, &mut fields)?;
            if !text.is_empty() {
                in_force = set.clone();
            }
        }
        // a pad written as a region carries its function as the attribute in force for it
        if let Draw::Region(_, Some(f)) = d {
            text.push_str(&format!("%TA.AperFunction,{}*%\n", f.values().join(",")));
        }
        if !text.is_empty() {
            push(&mut file, x2::marker());
            changes.push(text);
        }
        if let Draw::Flash(k, _) | Draw::Line(k, _, _) | Draw::Arc(k, _, _, _, _) = d {
            let n = numbers[k];
            if current != Some(n) {
                push(&mut file, Operation::SelectAperture(n));
                current = Some(n);
            }
        }
        match d {
            Draw::Flash(_, at) => push(&mut file, Operation::Flash { at: gp(*at) }),
            Draw::Line(_, from, to) => push(
                &mut file,
                Operation::Line {
                    from: gp(*from),
                    to: gp(*to),
                },
            ),
            Draw::Arc(_, from, to, offset, clockwise) => push(
                &mut file,
                Operation::Arc {
                    from: gp(*from),
                    to: gp(*to),
                    center_offset: *offset,
                    clockwise: *clockwise,
                    quadrant: QuadrantMode::Multi,
                },
            ),
            Draw::Region(points, function) => {
                push(&mut file, Operation::RegionStart);
                push(&mut file, Operation::Move { to: gp(points[0]) });
                for w in points.windows(2) {
                    push(
                        &mut file,
                        Operation::Line {
                            from: gp(w[0]),
                            to: gp(w[1]),
                        },
                    );
                }
                push(
                    &mut file,
                    Operation::Line {
                        from: gp(points[points.len() - 1]),
                        to: gp(points[0]),
                    },
                );
                push(&mut file, Operation::RegionEnd);
                if function.is_some() {
                    push(&mut file, x2::marker());
                    changes.push("%TD.AperFunction*%\n".to_owned());
                }
            }
            Draw::Polarity(p) => push(&mut file, Operation::Polarity(*p)),
        }
    }
    if objects {
        let text = x2::object_changes(&in_force, &ObjectSet::default(), &mut fields)?;
        if !text.is_empty() {
            push(&mut file, x2::marker());
            changes.push(text);
        }
    }
    let mut output = eh3d_writer_gerber::write(&file, &GerberOptions::default())?;
    if !changes.is_empty() {
        output.text = x2::replace_markers(&output.text, &changes).ok_or(LayerError::Attributes)?;
    }
    if spec.x2.level.has_aperture_attributes() {
        let functions: BTreeMap<u32, ApFunction> = numbers
            .iter()
            .filter_map(|(k, n)| k.function.map(|f| (*n, f)))
            .collect();
        output.text = x2::insert_aperture_attributes(&output.text, &functions);
    }
    if fields.escaped() > 0 {
        diagnostics.push(codes::diagnostic(
            codes::ATTRIBUTE_ESCAPED,
            Severity::Info,
            &format!("{} text(s) in one file", fields.escaped()),
        ));
    }
    Ok((output, diagnostics))
}

/// True when the via has a pad on the copper layer: the layer is one of its two ends, or lies
/// between them in the stack-up of the board (a through via has a pad on every inner layer and a
/// blind or buried via on the layers it spans). A board without a stack-up, or a via whose ends
/// are not in it, gets a pad on its two ends only.
pub fn via_on_layer(board: &Board, via: &Via, layer: Id) -> bool {
    if via.from_layer == layer || via.to_layer == layer {
        return true;
    }
    let position = |id: Id| board.copper_stack.iter().position(|l| *l == id);
    match (
        position(via.from_layer),
        position(via.to_layer),
        position(layer),
    ) {
        (Some(a), Some(b), Some(l)) => l > a.min(b) && l < a.max(b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::Point;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    #[test]
    fn a_quarter_circle_has_its_centre_where_it_should() {
        // from (1,0) to (0,1) counter-clockwise by 90 degrees about the origin (in mm)
        let ((i, j), cw) = arc_centre(pt(1_000_000, 0), pt(0, 1_000_000), 90_000_000).unwrap();
        assert!(!cw);
        assert!((i + 1.0).abs() < 1e-9 && j.abs() < 1e-9, "{i} {j}");
        // the same points clockwise: the centre is on the other side of the chord
        let ((i, j), cw) = arc_centre(pt(1_000_000, 0), pt(0, 1_000_000), -90_000_000).unwrap();
        assert!(cw);
        assert!((i - 0.0).abs() < 1e-9 && (j - 1.0).abs() < 1e-9, "{i} {j}");
        // a half circle has its centre on the chord
        let ((i, j), _) = arc_centre(pt(0, 0), pt(2_000_000, 0), 180_000_000).unwrap();
        assert!((i - 1.0).abs() < 1e-9 && j.abs() < 1e-9);
        // no sweep and no length give no centre
        assert!(arc_centre(pt(0, 0), pt(1, 0), 0).is_none());
        assert!(arc_centre(pt(5, 5), pt(5, 5), 90_000_000).is_none());
    }

    #[test]
    fn a_via_has_a_pad_on_every_layer_it_spans_and_on_no_other() {
        use eh3d_phoenix_model::{Nm, ObjectId};
        let layer = |n: u16| ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap();
        // four copper layers, top to bottom: 1 (top), 2, 3, 4 (bottom)
        let mut board = Board {
            copper_stack: (1..=4).map(layer).collect(),
            ..Board::default()
        };
        let via = |from: u16, to: u16| Via {
            net: layer(10),
            at: pt(0, 0),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: layer(from),
            to_layer: layer(to),
        };
        let spans = |board: &Board, v: &Via| -> Vec<bool> {
            (1..=4).map(|n| via_on_layer(board, v, layer(n))).collect()
        };
        // a through via: a pad on all four layers
        assert_eq!(spans(&board, &via(1, 4)), [true, true, true, true]);
        // the ends in the other order give the same answer
        assert_eq!(spans(&board, &via(4, 1)), [true, true, true, true]);
        // a blind via from the top to the second inner layer: 1, 2 and 3 but not 4
        assert_eq!(spans(&board, &via(1, 3)), [true, true, true, false]);
        // a buried via between the two inner layers: 2 and 3 only
        assert_eq!(spans(&board, &via(2, 3)), [false, true, true, false]);
        // a via on a layer that the stack-up does not list keeps the old rule: its two ends
        assert!(via_on_layer(&board, &via(1, 9), layer(9)));
        assert!(!via_on_layer(&board, &via(1, 9), layer(2)));
        // a board without a stack-up: the two ends only
        board.copper_stack.clear();
        assert_eq!(spans(&board, &via(1, 4)), [true, false, false, true]);
    }

    #[test]
    fn apertures_are_keyed_in_a_fixed_order() {
        let mut keys = vec![
            ApKey::Obround(1, 2),
            ApKey::Circle(9),
            ApKey::Rect(3, 3),
            ApKey::Circle(2),
        ];
        keys.sort();
        assert_eq!(
            keys,
            vec![
                ApKey::Circle(2),
                ApKey::Circle(9),
                ApKey::Rect(3, 3),
                ApKey::Obround(1, 2)
            ]
        );
    }

    #[test]
    fn a_rounded_rectangle_has_its_corners_cut_into_pieces() {
        // 2 by 1 mm with the largest radius, 0.5 mm: both ends are half circles. Each quarter has
        // 7 points; the point where two quarters meet at the end of a half circle is the same and
        // is kept once, so 7 + 6 + 7 + 6 = 26
        let ring = rounded_rectangle(pt(0, 0), 2_000_000, 1_000_000, 500_000);
        assert_eq!(ring.len(), 26);
        assert!(ring.contains(&pt(1_000_000, 0)));
        assert!(ring.contains(&pt(-1_000_000, 0)));
        assert!(ring.contains(&pt(500_000, -500_000)));
        assert!(ring.contains(&pt(-500_000, 500_000)));
        let xs: Vec<i64> = ring.iter().map(|p| p.x_nm.get()).collect();
        let ys: Vec<i64> = ring.iter().map(|p| p.y_nm.get()).collect();
        assert_eq!(
            (xs.iter().min(), xs.iter().max()),
            (Some(&-1_000_000), Some(&1_000_000))
        );
        assert_eq!(
            (ys.iter().min(), ys.iter().max()),
            (Some(&-500_000), Some(&500_000))
        );
        // a radius larger than half the shorter side is cut to it
        assert_eq!(
            ring,
            rounded_rectangle(pt(0, 0), 2_000_000, 1_000_000, 9_000_000)
        );
        // the box moves with the centre
        let moved = rounded_rectangle(pt(10_000_000, 5_000_000), 2_000_000, 1_000_000, 200_000);
        assert_eq!(moved.iter().map(|p| p.x_nm.get()).min(), Some(9_000_000));
        assert_eq!(moved.iter().map(|p| p.y_nm.get()).max(), Some(5_500_000));
    }

    #[test]
    fn only_a_rounded_rectangle_at_a_right_angle_is_a_region() {
        let pad = Pad {
            number: "1".into(),
            shape: PadShape::Rect {
                width: eh3d_phoenix_model::Nm::new(2_000_000).unwrap(),
                height: eh3d_phoenix_model::Nm::new(1_000_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: Vec::new(),
            roundness: 500,
            plated: true,
        };
        let at = |degrees: i64| Component {
            refdes: "R1".into(),
            value: String::new(),
            symbol: None,
            footprint: eh3d_phoenix_model::ObjectId::from_parts(1, 1, 1).unwrap(),
            at: pt(0, 0),
            rotation: Udeg::new(degrees * 1_000_000).unwrap(),
            side: eh3d_phoenix_board::Side::Top,
        };
        assert!(pad_is_region(&pad, &at(0)));
        assert!(pad_is_region(&pad, &at(90)));
        assert!(pad_is_region(&pad, &at(45)));
        let square = Pad {
            roundness: 0,
            ..pad.clone()
        };
        assert!(!pad_is_region(&square, &at(0)));
        assert!(pad_is_region(&square, &at(45)));
        let round = Pad {
            shape: PadShape::Circle {
                diameter: eh3d_phoenix_model::Nm::new(1_000_000).unwrap(),
            },
            ..pad.clone()
        };
        assert!(!pad_is_region(&round, &at(45)));
    }

    #[test]
    fn the_gaps_of_a_thermal_relief_are_four_pieces_of_the_ring_between_the_spokes() {
        let at = Point::from_nm(10_000_000, 5_000_000).unwrap();
        // a pad of 1 mm, a gap of 0.25 mm and spokes of 0.25 mm: the ring goes from the radius
        // 0.5 to the radius 0.75 mm and a spoke covers 0.125 mm on each side of its axis
        let gaps = thermal_gaps(at, 1_000_000, 250_000, 250_000);
        assert_eq!(gaps.len(), 4);
        for ring in &gaps {
            assert_eq!(ring.len(), 14);
            for p in ring {
                let dx = (p.x_nm.get() - 10_000_000) as f64;
                let dy = (p.y_nm.get() - 5_000_000) as f64;
                let r = dx.hypot(dy);
                assert!((499_999.0..=750_001.0).contains(&r), "{r}");
                assert!(dx.abs() >= 124_999.0 || dy.abs() >= 124_999.0);
            }
        }
        // the first piece starts on the inner circle where it meets the edge of the east spoke:
        // x = sqrt(500000^2 - 125000^2) = 484123 (rounded), y = 125000
        assert_eq!(
            (gaps[0][0].x_nm.get(), gaps[0][0].y_nm.get()),
            (10_484_123, 5_125_000)
        );
        // and ends on the outer circle on the same edge: x = sqrt(750000^2 - 125000^2) = 739510
        assert_eq!(
            (gaps[0][13].x_nm.get(), gaps[0][13].y_nm.get()),
            (10_739_510, 5_125_000)
        );
        // the other pieces are the first one turned by right angles about the pad
        assert_eq!(
            (gaps[1][0].x_nm.get(), gaps[1][0].y_nm.get()),
            (9_875_000, 5_484_123)
        );
        assert_eq!(
            (gaps[2][0].x_nm.get(), gaps[2][0].y_nm.get()),
            (9_515_877, 4_875_000)
        );
        // no gap, no spoke or a spoke as wide as the pad: nothing to clear
        assert!(thermal_gaps(at, 1_000_000, 0, 250_000).is_empty());
        assert!(thermal_gaps(at, 1_000_000, 250_000, 0).is_empty());
        assert!(thermal_gaps(at, 1_000_000, 250_000, 1_000_000).is_empty());
    }
}
