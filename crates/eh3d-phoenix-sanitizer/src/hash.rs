// SPDX-License-Identifier: GPL-3.0-or-later
//! The canonical geometric hash of a footprint or a symbol.
//!
//! The hash does not change with the order of the drawing items, with the names of the item, its
//! pads and its pins, or with differences smaller than the rounding grid of the settings. When the
//! settings allow it, it also does not change when the item is turned by 90 degrees, mirrored in X
//! or shifted: the hash is the smallest of the hashes of the allowed transforms, and the transform
//! that gave it is reported so that a replacement that would turn a part can be flagged.
//!
//! A second hash, the named one, adds the names of the pads and the pins and the electrical
//! attributes of the pins. Two duplicates with the same geometric hash and different named hashes
//! number their pads differently: replacing one by the other would rewire the board.

use sha2::{Digest, Sha256};

use crate::model::Prim;
use crate::settings::SanitizerSettings;

/// One of the eight symmetries of the square: a mirror in X followed by quarter turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transform {
    /// Quarter turns counter-clockwise, 0 to 3.
    pub quarter: u8,
    /// Mirrored in X before the turn.
    pub mirror: bool,
}

const FULL_TURN_UDEG: i64 = 360_000_000;
const QUARTER_UDEG: i64 = 90_000_000;
const HALF_TURN_UDEG: i64 = 180_000_000;

impl Transform {
    /// No change.
    pub const IDENTITY: Transform = Transform {
        quarter: 0,
        mirror: false,
    };

    /// The transforms the settings allow, the identity first and in a fixed order.
    pub fn allowed(allow_rotation: bool, allow_mirror: bool) -> Vec<Transform> {
        let mut out = Vec::new();
        for mirror in [false, true] {
            if mirror && !allow_mirror {
                continue;
            }
            for quarter in 0u8..4 {
                if quarter > 0 && !allow_rotation {
                    continue;
                }
                out.push(Transform { quarter, mirror });
            }
        }
        out
    }

    /// A small number that tells the transform apart, 0 for the identity.
    pub fn id(self) -> u8 {
        self.quarter + if self.mirror { 4 } else { 0 }
    }

    /// True for the identity.
    pub fn is_identity(self) -> bool {
        self == Transform::IDENTITY
    }

    /// Moves a point.
    pub fn point(self, x: i64, y: i64) -> (i64, i64) {
        let x = if self.mirror { -x } else { x };
        match self.quarter % 4 {
            0 => (x, y),
            1 => (-y, x),
            2 => (-x, -y),
            _ => (y, -x),
        }
    }

    /// Moves the direction of an object: an angle in micro-degrees, kept in one turn.
    pub fn direction(self, angle: i64) -> i64 {
        let a = if self.mirror {
            HALF_TURN_UDEG - angle
        } else {
            angle
        };
        (a + i64::from(self.quarter) * QUARTER_UDEG).rem_euclid(FULL_TURN_UDEG)
    }

    /// Moves the orientation of an object that is turned about its own centre and has no corners
    /// that carry the turn (a pad): a mirror in X reverses the angle, then the quarter turns add.
    pub fn orientation(self, angle: i64) -> i64 {
        let a = if self.mirror { -angle } else { angle };
        (a + i64::from(self.quarter) * QUARTER_UDEG).rem_euclid(FULL_TURN_UDEG)
    }

    /// Moves the angle of a rectangle: its corners carry the quarter turns, so only a mirror in X
    /// changes the angle.
    pub fn rect_angle(self, angle: i64) -> i64 {
        let a = if self.mirror { -angle } else { angle };
        a.rem_euclid(FULL_TURN_UDEG)
    }
}

/// Rounds half away from zero to a multiple of `step`; an odd function, so that a turn or a
/// mirror gives the same result before and after the rounding.
pub fn snap(value: i64, step: i64) -> i64 {
    let step = step.max(1);
    let half = step / 2;
    if value >= 0 {
        (value + half) / step * step
    } else {
        -((-value + half) / step * step)
    }
}

fn snap_angle(angle: i64, step: i64) -> i64 {
    snap(angle, step).rem_euclid(FULL_TURN_UDEG)
}

/// A drawing item after the rounding, the transform and the shift.
fn prepare(prim: &Prim, t: Transform, s: &SanitizerSettings) -> Prim {
    let n = s.round_nm;
    let a = s.angle_round_udeg;
    let p = |x: i64, y: i64| t.point(snap(x, n), snap(y, n));
    match prim {
        Prim::Pad {
            name,
            thru,
            x,
            y,
            w,
            h,
            drill,
            shape,
            layer,
            rot,
        } => {
            let (x, y) = p(*x, *y);
            Prim::Pad {
                name: name.clone(),
                thru: *thru,
                x,
                y,
                w: snap(*w, n),
                h: snap(*h, n),
                drill: snap(*drill, n),
                shape: shape.clone(),
                layer: *layer,
                rot: t.orientation(snap_angle(*rot, a)),
            }
        }
        Prim::Wire {
            layer,
            x1,
            y1,
            x2,
            y2,
            width,
            curve,
        } => {
            let (x1, y1) = p(*x1, *y1);
            let (x2, y2) = p(*x2, *y2);
            let curve = snap(*curve, a);
            Prim::Wire {
                layer: *layer,
                x1,
                y1,
                x2,
                y2,
                width: snap(*width, n),
                curve: if t.mirror { -curve } else { curve },
            }
        }
        Prim::Rect {
            layer,
            x1,
            y1,
            x2,
            y2,
            rot,
        } => {
            let (x1, y1) = p(*x1, *y1);
            let (x2, y2) = p(*x2, *y2);
            Prim::Rect {
                layer: *layer,
                x1,
                y1,
                x2,
                y2,
                rot: t.rect_angle(snap_angle(*rot, a)),
            }
        }
        Prim::Circle {
            layer,
            x,
            y,
            r,
            width,
        } => {
            let (x, y) = p(*x, *y);
            Prim::Circle {
                layer: *layer,
                x,
                y,
                r: snap(*r, n),
                width: snap(*width, n),
            }
        }
        Prim::Poly {
            layer,
            width,
            verts,
        } => Prim::Poly {
            layer: *layer,
            width: snap(*width, n),
            verts: verts
                .iter()
                .map(|(x, y, c)| {
                    let (x, y) = p(*x, *y);
                    let c = snap(*c, a);
                    (x, y, if t.mirror { -c } else { c })
                })
                .collect(),
        },
        Prim::Hole { x, y, drill } => {
            let (x, y) = p(*x, *y);
            Prim::Hole {
                x,
                y,
                drill: snap(*drill, n),
            }
        }
        Prim::Text {
            layer,
            x,
            y,
            size,
            rot,
            content,
        } => {
            let (x, y) = p(*x, *y);
            Prim::Text {
                layer: *layer,
                x,
                y,
                size: snap(*size, n),
                rot: t.direction(snap_angle(*rot, a)),
                content: content.clone(),
            }
        }
        Prim::Pin {
            name,
            x,
            y,
            rot,
            length,
            direction,
            function,
            visible,
            swaplevel,
        } => {
            let (x, y) = p(*x, *y);
            Prim::Pin {
                name: name.clone(),
                x,
                y,
                rot: t.direction(snap_angle(*rot, a)),
                length: length.clone(),
                direction: direction.clone(),
                function: function.clone(),
                visible: visible.clone(),
                swaplevel: swaplevel.clone(),
            }
        }
    }
}

/// Moves every point of the prepared items by `(dx, dy)`.
fn shift(prim: &mut Prim, dx: i64, dy: i64) {
    match prim {
        Prim::Pad { x, y, .. }
        | Prim::Circle { x, y, .. }
        | Prim::Hole { x, y, .. }
        | Prim::Text { x, y, .. }
        | Prim::Pin { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
        Prim::Wire { x1, y1, x2, y2, .. } | Prim::Rect { x1, y1, x2, y2, .. } => {
            *x1 += dx;
            *x2 += dx;
            *y1 += dy;
            *y2 += dy;
        }
        Prim::Poly { verts, .. } => {
            for v in verts.iter_mut() {
                v.0 += dx;
                v.1 += dy;
            }
        }
    }
}

/// The corner the shape is measured from when the origin is ignored: the smallest x and the
/// smallest y of the pads, the pins and the holes, or of every point when there are none.
fn origin_of(prims: &[Prim]) -> (i64, i64) {
    let mut anchors: Vec<(i64, i64)> = Vec::new();
    let mut points: Vec<(i64, i64)> = Vec::new();
    for p in prims {
        match p {
            Prim::Pad { x, y, .. } | Prim::Pin { x, y, .. } | Prim::Hole { x, y, .. } => {
                anchors.push((*x, *y));
            }
            Prim::Circle { x, y, .. } | Prim::Text { x, y, .. } => points.push((*x, *y)),
            Prim::Wire { x1, y1, x2, y2, .. } | Prim::Rect { x1, y1, x2, y2, .. } => {
                points.push((*x1, *y1));
                points.push((*x2, *y2));
            }
            Prim::Poly { verts, .. } => points.extend(verts.iter().map(|v| (v.0, v.1))),
        }
    }
    let source = if anchors.is_empty() {
        &points
    } else {
        &anchors
    };
    let min_x = source.iter().map(|p| p.0).min().unwrap_or(0);
    let min_y = source.iter().map(|p| p.1).min().unwrap_or(0);
    (min_x, min_y)
}

/// The prepared items of an item under a transform, shifted when the settings ignore the origin.
pub fn prepared(prims: &[Prim], t: Transform, s: &SanitizerSettings) -> Vec<Prim> {
    let mut out: Vec<Prim> = prims.iter().map(|p| prepare(p, t, s)).collect();
    if s.ignore_origin {
        let (dx, dy) = origin_of(&out);
        for p in out.iter_mut() {
            shift(p, -dx, -dy);
        }
    }
    out
}

/// A line with its curve, the ends in a fixed order.
fn edge(x1: i64, y1: i64, x2: i64, y2: i64, curve: i64) -> (i64, i64, i64, i64, i64) {
    if (x1, y1) <= (x2, y2) {
        (x1, y1, x2, y2, curve)
    } else {
        (x2, y2, x1, y1, -curve)
    }
}

/// The text of one prepared item in its canonical form.
fn encode(prim: &Prim, named: bool) -> String {
    match prim {
        Prim::Pad {
            name,
            thru,
            x,
            y,
            w,
            h,
            drill,
            shape,
            layer,
            rot,
        } => {
            let (mut w, mut h, mut a) = (*w, *h, *rot);
            match shape.as_str() {
                "round" => a = 0,
                "square" | "octagon" => a %= QUARTER_UDEG,
                // no symmetry
                "offset" => {}
                _ => {
                    a %= HALF_TURN_UDEG;
                    if shape.starts_with("smd") {
                        if a == QUARTER_UDEG {
                            std::mem::swap(&mut w, &mut h);
                            a = 0;
                        }
                        if w == h {
                            a %= QUARTER_UDEG;
                        }
                    }
                }
            }
            let base = format!(
                "P|{}|{x}|{y}|{w}|{h}|{drill}|{shape}|{layer}|{a}",
                u8::from(*thru)
            );
            if named {
                format!("{base}|{name}")
            } else {
                base
            }
        }
        Prim::Wire {
            layer,
            x1,
            y1,
            x2,
            y2,
            width,
            curve,
        } => {
            let (a, b, c, d, k) = edge(*x1, *y1, *x2, *y2, *curve);
            format!("W|{layer}|{a}|{b}|{c}|{d}|{width}|{k}")
        }
        Prim::Rect {
            layer,
            x1,
            y1,
            x2,
            y2,
            rot,
        } => {
            let (mut w, mut h) = ((x2 - x1).abs(), (y2 - y1).abs());
            let mut a = rot % HALF_TURN_UDEG;
            if a == QUARTER_UDEG {
                std::mem::swap(&mut w, &mut h);
                a = 0;
            }
            format!("R|{layer}|{}|{}|{w}|{h}|{a}", x1 + x2, y1 + y2)
        }
        Prim::Circle {
            layer,
            x,
            y,
            r,
            width,
        } => format!("C|{layer}|{x}|{y}|{r}|{width}"),
        Prim::Poly {
            layer,
            width,
            verts,
        } => {
            let mut edges: Vec<String> = Vec::with_capacity(verts.len());
            for (i, v) in verts.iter().enumerate() {
                let next = &verts[(i + 1) % verts.len()];
                let (a, b, c, d, k) = edge(v.0, v.1, next.0, next.1, v.2);
                edges.push(format!("{a},{b},{c},{d},{k}"));
            }
            edges.sort();
            format!("G|{layer}|{width}|{}", edges.join(";"))
        }
        Prim::Hole { x, y, drill } => format!("H|{x}|{y}|{drill}"),
        Prim::Text {
            layer,
            x,
            y,
            size,
            rot,
            content,
        } => format!("T|{layer}|{x}|{y}|{size}|{rot}|{content}"),
        Prim::Pin {
            name,
            x,
            y,
            rot,
            length,
            direction,
            function,
            visible,
            swaplevel,
        } => {
            let base = format!("N|{x}|{y}|{rot}|{length}|{function}");
            if named {
                format!("{base}|{name}|{direction}|{visible}|{swaplevel}")
            } else {
                base
            }
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest(kind: &str, mut lines: Vec<String>) -> String {
    lines.sort();
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    for line in &lines {
        hasher.update(b"\n");
        hasher.update(line.as_bytes());
    }
    hex(&hasher.finalize())
}

/// The hashes of an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonHash {
    /// The geometric hash: no names, no electrical attributes.
    pub geometry: String,
    /// The hash that also holds the names of the pads and pins and the electrical attributes of
    /// the pins, taken under the same transform.
    pub named: String,
    /// The transform that gave the smallest geometric hash; the identity when it is one of them.
    pub transform: Transform,
}

/// The canonical hashes of the drawing items of a footprint (`kind` = `package`) or a symbol
/// (`kind` = `symbol`).
pub fn canonical(prims: &[Prim], kind: &str, s: &SanitizerSettings) -> CanonHash {
    // the list always holds the identity, which comes first
    let mut geometry = String::new();
    let mut transform = Transform::IDENTITY;
    for (i, t) in Transform::allowed(s.allow_rotation, s.allow_mirror)
        .into_iter()
        .enumerate()
    {
        let items = prepared(prims, t, s);
        let h = digest(kind, items.iter().map(|p| encode(p, false)).collect());
        if i == 0 || h < geometry {
            geometry = h;
            transform = t;
        }
    }
    let items = prepared(prims, transform, s);
    let named = digest(kind, items.iter().map(|p| encode(p, true)).collect());
    CanonHash {
        geometry,
        named,
        transform,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smd(name: &str, x: i64, y: i64, w: i64, h: i64, rot: i64) -> Prim {
        Prim::Pad {
            name: name.into(),
            thru: false,
            x,
            y,
            w,
            h,
            drill: 0,
            shape: "smd0".into(),
            layer: 1,
            rot,
        }
    }

    fn wire(x1: i64, y1: i64, x2: i64, y2: i64, curve: i64) -> Prim {
        Prim::Wire {
            layer: 21,
            x1,
            y1,
            x2,
            y2,
            width: 100_000,
            curve,
        }
    }

    fn base() -> Vec<Prim> {
        vec![
            smd("1", -900_000, 0, 1_200_000, 1_400_000, 0),
            smd("2", 900_000, 0, 1_200_000, 1_400_000, 0),
            wire(-1_500_000, 800_000, 1_500_000, 800_000, 0),
            wire(-1_500_000, -800_000, 1_500_000, -800_000, 0),
        ]
    }

    fn settings() -> SanitizerSettings {
        SanitizerSettings::default()
    }

    fn geometry(prims: &[Prim], s: &SanitizerSettings) -> String {
        canonical(prims, "package", s).geometry
    }

    #[test]
    fn the_hash_ignores_the_order_of_the_items_and_the_names() {
        let s = settings();
        let a = base();
        let mut b = base();
        b.reverse();
        for p in b.iter_mut() {
            if let Prim::Pad { name, .. } = p {
                *name = format!("pad-{name}");
            }
        }
        assert_eq!(geometry(&a, &s), geometry(&b, &s));
        assert_eq!(geometry(&a, &s).len(), 64);
        assert_ne!(
            canonical(&a, "package", &s).named,
            canonical(&b, "package", &s).named
        );
    }

    #[test]
    fn small_differences_vanish_in_the_rounding_and_large_ones_do_not() {
        let s = settings();
        let mut near = base();
        near[0] = smd(
            "1",
            -900_000 + 1_000,
            1_500,
            1_200_000 - 1_000,
            1_400_000,
            0,
        );
        assert_eq!(geometry(&base(), &s), geometry(&near, &s));
        let mut far = base();
        far[0] = smd("1", -900_000 + 20_000, 0, 1_200_000, 1_400_000, 0);
        assert_ne!(geometry(&base(), &s), geometry(&far, &s));
        // a coarser grid in the settings merges them
        let coarse = SanitizerSettings {
            round_nm: 50_000,
            ..settings()
        };
        assert_eq!(geometry(&base(), &coarse), geometry(&far, &coarse));
    }

    #[test]
    fn a_wire_read_backwards_and_a_pad_turned_by_half_a_turn_are_the_same() {
        let s = settings();
        let mut b = base();
        b[2] = wire(1_500_000, 800_000, -1_500_000, 800_000, 0);
        b[0] = smd("1", -900_000, 0, 1_200_000, 1_400_000, 180_000_000);
        assert_eq!(geometry(&base(), &s), geometry(&b, &s));
        // an arc read backwards flips the sign of its angle
        let arc = [wire(0, 0, 1_000_000, 0, 90_000_000)];
        let back = [wire(1_000_000, 0, 0, 0, -90_000_000)];
        assert_eq!(geometry(&arc, &s), geometry(&back, &s));
        let other = [wire(1_000_000, 0, 0, 0, 90_000_000)];
        assert_ne!(geometry(&arc, &s), geometry(&other, &s));
    }

    #[test]
    fn a_pad_turned_by_a_quarter_is_the_pad_with_its_sides_swapped() {
        let s = settings();
        let a = [smd("1", 0, 0, 1_000_000, 2_000_000, 90_000_000)];
        let b = [smd("1", 0, 0, 2_000_000, 1_000_000, 0)];
        assert_eq!(geometry(&a, &s), geometry(&b, &s));
        let r1 = [Prim::Rect {
            layer: 21,
            x1: -1_000_000,
            y1: -500_000,
            x2: 1_000_000,
            y2: 500_000,
            rot: 90_000_000,
        }];
        let r2 = [Prim::Rect {
            layer: 21,
            x1: -500_000,
            y1: -1_000_000,
            x2: 500_000,
            y2: 1_000_000,
            rot: 0,
        }];
        assert_eq!(geometry(&r1, &s), geometry(&r2, &s));
    }

    #[test]
    fn a_polygon_does_not_depend_on_where_it_starts_or_which_way_it_runs() {
        let s = settings();
        let v = |list: &[(i64, i64)]| Prim::Poly {
            layer: 39,
            width: 0,
            verts: list.iter().map(|(x, y)| (*x, *y, 0)).collect(),
        };
        let a = [v(&[(0, 0), (1_000_000, 0), (1_000_000, 1_000_000)])];
        let b = [v(&[(1_000_000, 0), (1_000_000, 1_000_000), (0, 0)])];
        let c = [v(&[(0, 0), (1_000_000, 1_000_000), (1_000_000, 0)])];
        assert_eq!(geometry(&a, &s), geometry(&b, &s));
        assert_eq!(geometry(&a, &s), geometry(&c, &s));
    }

    #[test]
    fn turned_and_mirrored_copies_match_only_when_the_settings_allow_it() {
        let item = vec![
            smd("1", -1_000_000, 0, 600_000, 900_000, 0),
            smd("2", 1_000_000, 0, 600_000, 900_000, 0),
            smd("3", 1_000_000, 2_000_000, 600_000, 900_000, 0),
            wire(-1_500_000, -1_000_000, 1_500_000, -1_000_000, 0),
        ];
        let rotated: Vec<Prim> = prepared(
            &item,
            Transform {
                quarter: 1,
                mirror: false,
            },
            &settings(),
        );
        let mirrored: Vec<Prim> = prepared(
            &item,
            Transform {
                quarter: 0,
                mirror: true,
            },
            &settings(),
        );
        let off = settings();
        assert_ne!(geometry(&item, &off), geometry(&rotated, &off));
        assert_ne!(geometry(&item, &off), geometry(&mirrored, &off));
        let rot_only = SanitizerSettings {
            allow_rotation: true,
            ..settings()
        };
        assert_eq!(geometry(&item, &rot_only), geometry(&rotated, &rot_only));
        assert_ne!(geometry(&item, &rot_only), geometry(&mirrored, &rot_only));
        let both = SanitizerSettings {
            allow_rotation: true,
            allow_mirror: true,
            ..settings()
        };
        assert_eq!(geometry(&item, &both), geometry(&rotated, &both));
        assert_eq!(geometry(&item, &both), geometry(&mirrored, &both));
        // the transform that gave the hash tells which of them was turned
        let from_item = canonical(&item, "package", &both).transform;
        let from_rotated = canonical(&rotated, "package", &both).transform;
        let from_mirrored = canonical(&mirrored, "package", &both).transform;
        assert_ne!(from_item, from_rotated);
        assert_ne!(from_item, from_mirrored);
        assert_eq!(
            from_item,
            canonical(&item.clone(), "package", &both).transform
        );
        // with the transforms off the only one is the identity
        assert!(canonical(&item, "package", &off).transform.is_identity());
    }

    #[test]
    fn the_origin_counts_unless_the_settings_ignore_it() {
        let item = base();
        let moved: Vec<Prim> = item
            .iter()
            .cloned()
            .map(|mut p| {
                shift(&mut p, 3_000_000, -2_000_000);
                p
            })
            .collect();
        let s = settings();
        assert_ne!(geometry(&item, &s), geometry(&moved, &s));
        let free = SanitizerSettings {
            ignore_origin: true,
            ..settings()
        };
        assert_eq!(geometry(&item, &free), geometry(&moved, &free));
    }

    #[test]
    fn swapped_pad_names_keep_the_geometry_and_change_the_named_hash() {
        let s = settings();
        let a = base();
        let mut swapped = base();
        for (i, p) in swapped.iter_mut().enumerate() {
            if let Prim::Pad { name, .. } = p {
                *name = if i == 0 { "2".into() } else { "1".into() };
            }
        }
        let ha = canonical(&a, "package", &s);
        let hs = canonical(&swapped, "package", &s);
        assert_eq!(ha.geometry, hs.geometry);
        assert_ne!(ha.named, hs.named);
        // the kind is part of the hash
        assert_ne!(ha.geometry, canonical(&a, "symbol", &s).geometry);
    }

    #[test]
    fn symbols_hash_their_pins_without_names_and_with_them() {
        let pin = |name: &str, x: i64, rot: i64, dir: &str| Prim::Pin {
            name: name.into(),
            x,
            y: 0,
            rot,
            length: "short".into(),
            direction: dir.into(),
            function: "none".into(),
            visible: "both".into(),
            swaplevel: "0".into(),
        };
        let s = settings();
        let a = [
            pin("IN", 0, 0, "in"),
            pin("OUT", 7_620_000, 180_000_000, "out"),
        ];
        let b = [pin("A", 0, 0, "io"), pin("B", 7_620_000, 180_000_000, "io")];
        let (ha, hb) = (canonical(&a, "symbol", &s), canonical(&b, "symbol", &s));
        assert_eq!(ha.geometry, hb.geometry);
        assert_ne!(ha.named, hb.named);
    }

    #[test]
    fn the_snap_is_odd_and_the_transforms_are_the_eight_symmetries() {
        for v in [-7, -5, -4, -1, 0, 1, 4, 5, 7, 12_500, -12_500] {
            assert_eq!(snap(-v, 10), -snap(v, 10));
        }
        assert_eq!(snap(5, 10), 10);
        assert_eq!(snap(-5, 10), -10);
        assert_eq!(snap(4, 10), 0);
        assert_eq!(snap(17, 0), 17);
        let all = Transform::allowed(true, true);
        assert_eq!(all.len(), 8);
        assert!(all[0].is_identity());
        let ids: Vec<u8> = all.iter().map(|t| t.id()).collect();
        assert_eq!(ids, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(Transform::allowed(false, false), vec![Transform::IDENTITY]);
        // four quarter turns come back
        let t = Transform {
            quarter: 1,
            mirror: false,
        };
        let mut p = (3, 5);
        for _ in 0..4 {
            p = t.point(p.0, p.1);
        }
        assert_eq!(p, (3, 5));
    }

    #[test]
    fn an_empty_item_has_a_hash() {
        let s = settings();
        assert_eq!(geometry(&[], &s).len(), 64);
    }
}
