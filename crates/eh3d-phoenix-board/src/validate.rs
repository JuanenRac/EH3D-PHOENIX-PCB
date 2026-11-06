//! The consistency checks of a board and its diagnostic codes `PHOENIX-BOARD-<NNN>`.
//!
//! A reader never panics and never guesses: whatever is wrong becomes a diagnostic with a stable
//! code. All problems are collected, in a fixed order, so the same board always gives the same list.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{segments_intersect, Nm, Point};

use crate::model::{Board, Id, LayerKind, Segment};

/// A reference to a part that does not exist.
pub const DANGLING: &str = "PHOENIX-BOARD-001";
/// A track whose pieces do not join.
pub const PATH_NOT_CONNECTED: &str = "PHOENIX-BOARD-002";
/// A straight piece of track that is not at a multiple of 45 degrees.
pub const OFF_GRID_ANGLE: &str = "PHOENIX-BOARD-003";
/// A size that is zero or negative where it must be positive.
pub const BAD_SIZE: &str = "PHOENIX-BOARD-004";
/// A pad given to more than one net, or the list of pad nets out of order.
pub const PAD_NET_CONFLICT: &str = "PHOENIX-BOARD-005";
/// An outline that is not a closed polygon.
pub const BAD_OUTLINE: &str = "PHOENIX-BOARD-006";
/// A layer of the wrong kind or a via that starts and ends on one layer.
pub const BAD_LAYER: &str = "PHOENIX-BOARD-007";
/// Two components with the same reference designator.
pub const DUPLICATE_REFDES: &str = "PHOENIX-BOARD-008";
/// An arc that has no shape.
pub const BAD_ARC: &str = "PHOENIX-BOARD-009";

/// Every code of this crate with its title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (DANGLING, "Reference to a part that does not exist"),
    (PATH_NOT_CONNECTED, "Track pieces do not join"),
    (
        OFF_GRID_ANGLE,
        "Straight track piece off the 45-degree grid",
    ),
    (BAD_SIZE, "Size is zero or negative"),
    (
        PAD_NET_CONFLICT,
        "Pad assigned twice or pad nets out of order",
    ),
    (BAD_OUTLINE, "Outline is not a closed polygon"),
    (BAD_LAYER, "Layer of the wrong kind"),
    (DUPLICATE_REFDES, "Duplicate reference designator"),
    (BAD_ARC, "Arc has no shape"),
];

fn make(code: &'static str, severity: Severity, detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, severity, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Geometric)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

fn polygon_problem(points: &[Point]) -> Option<&'static str> {
    if points.len() < 3 {
        return Some("fewer than three points");
    }
    let n = points.len();
    for i in 0..n {
        if points[i] == points[(i + 1) % n] {
            return Some("two consecutive points coincide");
        }
    }
    for i in 0..n {
        for j in (i + 1)..n {
            // edges that share a point are neighbours
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            if segments_intersect(
                points[i],
                points[(i + 1) % n],
                points[j],
                points[(j + 1) % n],
            ) {
                return Some("the outline crosses itself");
            }
        }
    }
    None
}

fn positive(value: Nm) -> bool {
    value.get() > 0
}

impl Board {
    /// Every problem of the board, errors and warnings, in a fixed order.
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let mut error =
            |code: &'static str, detail: String| out.push(make(code, Severity::Error, detail));
        let mut warnings: Vec<Diagnostic> = Vec::new();

        if !self.outline.is_empty() {
            if let Some(why) = polygon_problem(&self.outline) {
                error(BAD_OUTLINE, format!("the board: {why}"));
            }
        }
        for (zone_id, zone) in &self.zones {
            for (i, cutout) in zone.cutouts.iter().enumerate() {
                if let Some(why) = polygon_problem(cutout) {
                    error(
                        BAD_OUTLINE,
                        format!("cutout {} of zone {zone_id}: {why}", i + 1),
                    );
                }
            }
        }
        for (i, cutout) in self.outline_cutouts.iter().enumerate() {
            if let Some(why) = polygon_problem(cutout) {
                error(BAD_OUTLINE, format!("cutout {}: {why}", i + 1));
            }
        }
        let layer_check =
            |id: &Id, what: &str, error: &mut dyn FnMut(&'static str, String)| match self
                .layers
                .get(id)
            {
                None => error(
                    DANGLING,
                    format!("{what} names a layer that does not exist"),
                ),
                Some(l) if l.kind != LayerKind::Copper => {
                    error(
                        BAD_LAYER,
                        format!("{what} is on a layer that is not copper"),
                    );
                }
                Some(_) => {}
            };

        for (id, fp) in &self.footprints {
            for (pad_id, pad) in &fp.pads {
                for layer in &pad.layers {
                    if !self.layers.contains_key(layer) {
                        error(
                            DANGLING,
                            format!("pad {pad_id} of footprint {id} names a missing layer"),
                        );
                    }
                }
                if let Some(d) = pad.drill {
                    if !positive(d) {
                        error(BAD_SIZE, format!("drill of pad {pad_id}"));
                    }
                }
                let sizes = match pad.shape {
                    crate::model::PadShape::Circle { diameter } => vec![diameter],
                    crate::model::PadShape::Rect { width, height }
                    | crate::model::PadShape::Oblong { width, height } => vec![width, height],
                };
                if sizes.iter().any(|s| !positive(*s)) {
                    error(BAD_SIZE, format!("shape of pad {pad_id}"));
                }
                if pad.roundness > crate::model::ROUNDNESS_MAX {
                    error(BAD_SIZE, format!("roundness of pad {pad_id}"));
                }
                if !pad.plated && pad.drill.is_none() {
                    error(
                        BAD_SIZE,
                        format!("pad {pad_id} is not plated and has no drill"),
                    );
                }
            }
        }

        let mut refdes = std::collections::BTreeMap::new();
        for (id, c) in &self.components {
            if !self.footprints.contains_key(&c.footprint) {
                error(
                    DANGLING,
                    format!("component {id} names a missing footprint"),
                );
            }
            if c.refdes.is_empty() {
                error(
                    BAD_SIZE,
                    format!("component {id} has an empty reference designator"),
                );
            } else if let Some(first) = refdes.insert(c.refdes.clone(), *id) {
                error(DUPLICATE_REFDES, format!("components {first} and {id}"));
            }
        }

        let mut previous = None;
        for pn in &self.pad_nets {
            let key = pn.pad_ref();
            if previous.is_some_and(|p| p >= key) {
                error(
                    PAD_NET_CONFLICT,
                    format!("pad {} of component {}", pn.pad, pn.component),
                );
            }
            previous = Some(key);
            match self.components.get(&pn.component) {
                None => error(
                    DANGLING,
                    format!(
                        "a pad assignment names a missing component {}",
                        pn.component
                    ),
                ),
                Some(c) => {
                    let has = self
                        .footprints
                        .get(&c.footprint)
                        .is_some_and(|f| f.pads.contains_key(&pn.pad));
                    if !has {
                        error(
                            DANGLING,
                            format!("component {} has no pad {}", pn.component, pn.pad),
                        );
                    }
                }
            }
            if !self.nets.contains_key(&pn.net) {
                error(
                    DANGLING,
                    format!("a pad assignment names a missing net {}", pn.net),
                );
            }
        }

        for (id, t) in &self.tracks {
            if !self.nets.contains_key(&t.net) {
                error(DANGLING, format!("track {id} names a missing net"));
            }
            layer_check(&t.layer, &format!("track {id}"), &mut error);
            if !positive(t.width) {
                error(BAD_SIZE, format!("width of track {id}"));
            }
            if t.path.is_empty() {
                error(PATH_NOT_CONNECTED, format!("track {id} has no pieces"));
            }
            for (i, piece) in t.path.iter().enumerate() {
                if i > 0 && t.path[i - 1].end() != piece.start() {
                    error(PATH_NOT_CONNECTED, format!("track {id} at piece {i}"));
                }
                match piece {
                    Segment::Line { start, end } => {
                        let dx = end.x_nm.get() - start.x_nm.get();
                        let dy = end.y_nm.get() - start.y_nm.get();
                        if dx == 0 && dy == 0 {
                            error(BAD_SIZE, format!("track {id} has a piece of length zero"));
                        } else if !(dx == 0 || dy == 0 || dx.abs() == dy.abs()) {
                            warnings.push(make(
                                OFF_GRID_ANGLE,
                                Severity::Warning,
                                format!("track {id} at piece {i}"),
                            ));
                        }
                    }
                    Segment::Arc { .. } => {
                        if let Some(Err(e)) = piece.to_arc() {
                            error(BAD_ARC, format!("track {id} at piece {i}: {e}"));
                        }
                    }
                }
            }
        }

        for (id, v) in &self.vias {
            if !self.nets.contains_key(&v.net) {
                error(DANGLING, format!("via {id} names a missing net"));
            }
            layer_check(&v.from_layer, &format!("via {id}"), &mut error);
            layer_check(&v.to_layer, &format!("via {id}"), &mut error);
            if v.from_layer == v.to_layer {
                error(BAD_LAYER, format!("via {id} starts and ends on one layer"));
            }
            if !positive(v.drill) || v.diameter.get() <= v.drill.get() {
                error(BAD_SIZE, format!("drill and diameter of via {id}"));
            }
        }

        for (id, z) in &self.zones {
            if !self.nets.contains_key(&z.net) {
                error(DANGLING, format!("zone {id} names a missing net"));
            }
            layer_check(&z.layer, &format!("zone {id}"), &mut error);
            if let Some(why) = polygon_problem(&z.outline) {
                error(BAD_OUTLINE, format!("zone {id}: {why}"));
            }
            if z.clearance.get() < 0 {
                error(BAD_SIZE, format!("clearance of zone {id}"));
            }
        }
        out.extend(warnings);
        out.extend(self.stack_problems());
        out.extend(self.silk_problems());
        out
    }

    /// True when [`Board::validate`] finds no error (warnings do not count).
    pub fn is_consistent(&self) -> bool {
        self.validate()
            .iter()
            .all(|d| d.severity != Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_documented() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            assert!(seen.insert(*code));
            assert!(
                code.starts_with("PHOENIX-BOARD-") && code.len() == 17,
                "{code}"
            );
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
