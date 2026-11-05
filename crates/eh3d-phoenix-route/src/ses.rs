// SPDX-License-Identifier: GPL-3.0-or-later
//! The import of a Specctra session (SES), the file in which an external autorouter returns its
//! routes, as a plan that can be checked before anything is changed.
//!
//! The session is read, every wire and via is matched with the nets, layers and via sizes of the
//! board, and the new copper is added to a copy of the board. The plan is all or nothing: if one
//! name does not match, if the board refuses an operation or if the rules find an error that the
//! board did not have before, no part of the session is used. The board that was read is never
//! changed; the caller applies [`SesPlan::operation`] to a board of its own, or writes the result
//! to a new file.
//!
//! A wire or via of the session that is already on the board (the fixed copper that a router
//! writes back) is counted and left out; wires are paths of straight pieces (a session with any
//! other shape is refused) and a via is read from its padstack name, which is the name that the
//! export wrote.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id, Operation, Segment, Track, Via};
use eh3d_phoenix_model::{Nm, Point, ScaledDecimal};

use crate::codes::{
    RouteError, NOT_CONNECTED, OPERATION_FAILED, OUT_OF_RANGE, RULES_REFUSE, SES_MISMATCH,
    SES_UNREADABLE,
};
use crate::live::{diff, findings, LiveDiff, RuleContext};
use crate::ratsnest::ratsnest;
use crate::router::{copper_layers, fresh_id};
use crate::settings::RouteSettings;
use crate::sexpr::{parse, Node};

/// The most characters of a number in a session.
const MAX_NUMBER_CHARS: usize = 40;

/// The routes of a session, ready to be applied to the board they were read against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SesPlan {
    /// Adds the new tracks and vias, as one step.
    pub operation: Operation,
    /// The new tracks, by id.
    pub tracks: Vec<(Id, Track)>,
    /// The new vias, by id.
    pub vias: Vec<(Id, Via)>,
    /// Wires and vias of the session that the board already had and that were left out.
    pub already_there: usize,
    /// The length of the new copper tracks, in nanometres, rounded down for each piece.
    pub length_nm: i64,
    /// The connections still to route before the plan.
    pub unrouted_before: usize,
    /// The connections still to route after the plan.
    pub unrouted_after: usize,
    /// What the rule check finds after the plan that it did not before, and the other way round.
    pub diff: LiveDiff,
    /// Per net, in the order of the names: the name, the new tracks, the new vias and the length.
    pub nets: Vec<(String, usize, usize, i64)>,
}

impl SesPlan {
    /// Applies the plan to a board. The board is not changed when this fails.
    pub fn apply(&self, board: &mut Board) -> Result<(), RouteError> {
        let mut copy = board.clone();
        copy.apply(&self.operation)
            .map_err(|e| RouteError::new(OPERATION_FAILED, e.to_string()))?;
        *board = copy;
        Ok(())
    }

    /// Refuses a plan that leaves connections unrouted.
    pub fn require_complete(&self) -> Result<(), RouteError> {
        if self.unrouted_after > 0 {
            return Err(RouteError::new(
                NOT_CONNECTED,
                format!(
                    "{} connections are still to route after the session",
                    self.unrouted_after
                ),
            ));
        }
        Ok(())
    }

    /// The report as plain text, always the same for the same plan.
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "session routes: {} tracks, {} vias, {} nm of copper\n",
            self.tracks.len(),
            self.vias.len(),
            self.length_nm
        );
        out.push_str(&format!(
            "already on the board and left out: {}\n",
            self.already_there
        ));
        out.push_str(&format!(
            "connections to route: {} before, {} after\n",
            self.unrouted_before, self.unrouted_after
        ));
        out.push_str(&format!(
            "rule findings: {} added, {} removed\n",
            self.diff.added.len(),
            self.diff.removed.len()
        ));
        for (name, tracks, vias, length) in &self.nets {
            out.push_str(&format!(
                "  net {name}: {tracks} tracks, {vias} vias, {length} nm\n"
            ));
        }
        out
    }

    /// The report as JSON, always the same for the same plan.
    pub fn to_json(&self) -> String {
        let mut nets = Vec::new();
        for (name, tracks, vias, length) in &self.nets {
            nets.push(format!(
                "{{\"net\":\"{}\",\"tracks\":{tracks},\"vias\":{vias},\"length_nm\":{length}}}",
                json_text(name)
            ));
        }
        format!(
            "{{\"tracks\":{},\"vias\":{},\"length_nm\":{},\"already_there\":{},\"unrouted_before\":{},\"unrouted_after\":{},\"findings_added\":{},\"findings_removed\":{},\"nets\":[{}]}}",
            self.tracks.len(),
            self.vias.len(),
            self.length_nm,
            self.already_there,
            self.unrouted_before,
            self.unrouted_after,
            self.diff.added.len(),
            self.diff.removed.len(),
            nets.join(",")
        )
    }
}

fn json_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn unreadable(what: impl Into<String>) -> RouteError {
    RouteError::new(SES_UNREADABLE, what)
}

fn mismatch(what: impl Into<String>) -> RouteError {
    RouteError::new(SES_MISMATCH, what)
}

/// The unit of the coordinates of the session: `per` units to the length unit `unit_nm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Resolution {
    unit_nm: i128,
    per: i128,
}

impl Resolution {
    fn parse(node: &Node) -> Result<Resolution, RouteError> {
        let [_, unit, value] = node.items() else {
            return Err(unreadable("the resolution is not (resolution UNIT VALUE)"));
        };
        let unit_nm: i128 = match unit.atom() {
            Some("inch") => 25_400_000,
            Some("mil") => 25_400,
            Some("cm") => 10_000_000,
            Some("mm") => 1_000_000,
            Some("um") => 1_000,
            _ => return Err(unreadable("the unit of the resolution is not known")),
        };
        let per: i128 = value
            .atom()
            .and_then(|t| t.parse::<i128>().ok())
            .filter(|v| (1..=1_000_000_000).contains(v))
            .ok_or_else(|| unreadable("the value of the resolution is not a positive integer"))?;
        Ok(Resolution { unit_nm, per })
    }

    /// A coordinate or length of the session in nanometres, rounded to the nearest (halves away
    /// from zero).
    fn nm(&self, text: &str) -> Result<i64, RouteError> {
        if text.len() > MAX_NUMBER_CHARS {
            return Err(unreadable("a number is too long"));
        }
        let scaled = ScaledDecimal::parse(text)
            .map_err(|e| unreadable(format!("{text:?} is not a number: {e}")))?
            .scaled;
        let num = scaled
            .checked_mul(self.unit_nm)
            .ok_or_else(|| RouteError::new(OUT_OF_RANGE, format!("{text} is too large")))?;
        let den = self.per * 1_000_000;
        let magnitude = (num.abs() + den / 2) / den;
        let value = if num < 0 { -magnitude } else { magnitude };
        Nm::from_i128(value)
            .map(Nm::get)
            .map_err(|e| RouteError::new(OUT_OF_RANGE, format!("{text}: {e}")))
    }
}

/// The size of a via from the name of its padstack: `Via_<diameter>_<drill>_nm`.
fn via_from_name(name: &str) -> Option<(i64, i64)> {
    let body = name.strip_prefix("Via_")?.strip_suffix("_nm")?;
    let (diameter, drill) = body.split_once('_')?;
    let (diameter, drill): (i64, i64) = (diameter.parse().ok()?, drill.parse().ok()?);
    (diameter > 0 && drill > 0).then_some((diameter, drill))
}

/// One piece of new copper, in the order in which ids are given to it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Candidate {
    net: String,
    /// 0 for a track, 1 for a via.
    kind: u8,
    layer: usize,
    points: Vec<(i64, i64)>,
    width: i64,
    diameter: i64,
}

/// The vertices of a path with the repeated ones taken out, in the direction that sorts first.
fn normalised(points: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() != Some(p) {
            out.push(*p);
        }
    }
    let mut back = out.clone();
    back.reverse();
    if back < out {
        back
    } else {
        out
    }
}

fn point(p: (i64, i64)) -> Result<Point, RouteError> {
    Point::from_nm(p.0, p.1).map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))
}

fn length_of(points: &[(i64, i64)]) -> i64 {
    points
        .windows(2)
        .map(|w| {
            let dx = i128::from(w[1].0) - i128::from(w[0].0);
            let dy = i128::from(w[1].1) - i128::from(w[0].1);
            i64::try_from(((dx * dx + dy * dy) as u128).isqrt()).unwrap_or(i64::MAX)
        })
        .fold(0_i64, i64::saturating_add)
}

/// Reads a session and makes the plan that adds its routes to `board`.
///
/// `ctx` and `settings` are the ones the DSN was exported with. `new_id` gives the ids of the new
/// tracks and vias; ids in use are skipped.
pub fn import(
    board: &Board,
    ctx: &RuleContext,
    settings: &RouteSettings,
    text: &str,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<SesPlan, RouteError> {
    let root = parse(text)?;
    if root.head() != Some("session") {
        return Err(unreadable("the text is not a (session ...)"));
    }
    let routes = root
        .list("routes")
        .ok_or_else(|| unreadable("the session has no (routes ...)"))?;
    let resolution = Resolution::parse(
        routes
            .list("resolution")
            .ok_or_else(|| unreadable("the routes have no (resolution ...)"))?,
    )?;

    let network = routes
        .list("network_out")
        .ok_or_else(|| unreadable("the routes have no (network_out ...)"))?;

    let copper = copper_layers(board);
    if copper.is_empty() {
        return Err(mismatch("the board has no copper layer"));
    }
    let mut layer_of: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, id) in copper.iter().enumerate() {
        if let Some(layer) = board.layers.get(id) {
            layer_of.insert(layer.name.as_str(), index);
        }
    }
    let mut net_of: BTreeMap<&str, Id> = BTreeMap::new();
    for (id, net) in &board.nets {
        net_of.insert(net.name.as_str(), *id);
    }

    // What the board already has, so that fixed copper written back is not added twice.
    let mut existing: BTreeSet<Candidate> = BTreeSet::new();
    for track in board.tracks.values() {
        let Some(net) = board.nets.get(&track.net) else {
            continue;
        };
        let Some(layer) = copper.iter().position(|l| *l == track.layer) else {
            continue;
        };
        let mut points = Vec::new();
        let mut lines_only = true;
        for (i, segment) in track.path.iter().enumerate() {
            match segment {
                Segment::Line { start, end } => {
                    if i == 0 {
                        points.push((start.x_nm.get(), start.y_nm.get()));
                    }
                    points.push((end.x_nm.get(), end.y_nm.get()));
                }
                Segment::Arc { .. } => lines_only = false,
            }
        }
        if lines_only {
            existing.insert(Candidate {
                net: net.name.clone(),
                kind: 0,
                layer,
                points: normalised(&points),
                width: track.width.get(),
                diameter: 0,
            });
        }
    }
    for via in board.vias.values() {
        let Some(net) = board.nets.get(&via.net) else {
            continue;
        };
        existing.insert(Candidate {
            net: net.name.clone(),
            kind: 1,
            layer: 0,
            points: vec![(via.at.x_nm.get(), via.at.y_nm.get())],
            width: via.drill.get(),
            diameter: via.diameter.get(),
        });
    }

    let mut wanted: BTreeSet<Candidate> = BTreeSet::new();
    let mut already_there = 0;
    for net in network.lists("net") {
        let name = net
            .items()
            .get(1)
            .and_then(Node::atom)
            .ok_or_else(|| unreadable("a net of the session has no name"))?;
        if !net_of.contains_key(name) {
            return Err(mismatch(format!(
                "the session has the net {name:?}, which the board does not"
            )));
        }
        for wire in net.lists("wire") {
            let shapes: Vec<&Node> = wire
                .items()
                .iter()
                .skip(1)
                .filter(|n| {
                    matches!(
                        n.head(),
                        Some(
                            "path"
                                | "polygon"
                                | "qarc"
                                | "rect"
                                | "circle"
                                | "polyline_path"
                                | "polygon_aperture"
                        )
                    )
                })
                .collect();
            let [shape] = shapes.as_slice() else {
                return Err(unreadable(format!(
                    "a wire of the net {name:?} has no single shape"
                )));
            };
            if shape.head() != Some("path") {
                return Err(mismatch(format!(
                    "a wire of the net {name:?} has a shape that is not a path of straight pieces"
                )));
            }
            let items = shape.items();
            let layer_name = items
                .get(1)
                .and_then(Node::atom)
                .ok_or_else(|| unreadable("a path has no layer"))?;
            let layer = *layer_of.get(layer_name).ok_or_else(|| {
                mismatch(format!(
                    "the session has the layer {layer_name:?}, which is not a copper layer"
                ))
            })?;
            let width_text = items
                .get(2)
                .and_then(Node::atom)
                .ok_or_else(|| unreadable("a path has no width"))?;
            let width = resolution.nm(width_text)?;
            if width <= 0 {
                return Err(mismatch(format!("a wire of the net {name:?} has no width")));
            }
            let coords: Vec<&str> = items[3..].iter().filter_map(Node::atom).collect();
            if coords.len() != items.len() - 3 || !coords.len().is_multiple_of(2) {
                return Err(unreadable(
                    "a path has a coordinate missing or a list in it",
                ));
            }
            let mut points = Vec::new();
            for pair in coords.chunks(2) {
                points.push((resolution.nm(pair[0])?, resolution.nm(pair[1])?));
            }
            let points = normalised(&points);
            if points.len() < 2 {
                return Err(mismatch(format!(
                    "a wire of the net {name:?} has no length"
                )));
            }
            let candidate = Candidate {
                net: name.to_owned(),
                kind: 0,
                layer,
                points,
                width,
                diameter: 0,
            };
            if existing.contains(&candidate) || !wanted.insert(candidate) {
                already_there += 1;
            }
        }
        for via in net.lists("via") {
            let items = via.items();
            let via_name = items
                .get(1)
                .and_then(Node::atom)
                .ok_or_else(|| unreadable("a via has no padstack name"))?;
            let (diameter, drill) = via_from_name(via_name).ok_or_else(|| {
                mismatch(format!(
                    "the via padstack {via_name:?} is not one that the export wrote"
                ))
            })?;
            let [Some(x), Some(y)] = [2, 3].map(|i| items.get(i).and_then(Node::atom)) else {
                return Err(unreadable("a via has no place"));
            };
            let candidate = Candidate {
                net: name.to_owned(),
                kind: 1,
                layer: 0,
                points: vec![(resolution.nm(x)?, resolution.nm(y)?)],
                width: drill,
                diameter,
            };
            if existing.contains(&candidate) || !wanted.insert(candidate) {
                already_there += 1;
            }
        }
    }

    // Ids in the order of the candidates, then the operation.
    let mut taken = BTreeSet::new();
    let mut ops = Vec::new();
    let mut tracks = Vec::new();
    let mut vias = Vec::new();
    let mut per_net: BTreeMap<String, (usize, usize, i64)> = BTreeMap::new();
    let mut length_nm = 0_i64;
    let first = copper[0];
    let last = copper[copper.len() - 1];
    for c in &wanted {
        let id = fresh_id(board, new_id, &mut taken, settings.id_attempts)?;
        let net = net_of[c.net.as_str()];
        let entry = per_net.entry(c.net.clone()).or_default();
        if c.kind == 0 {
            let mut path = Vec::new();
            for pair in c.points.windows(2) {
                path.push(Segment::Line {
                    start: point(pair[0])?,
                    end: point(pair[1])?,
                });
            }
            let track = Track {
                net,
                layer: copper[c.layer],
                width: Nm::new(c.width)
                    .map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
                path,
            };
            let length = length_of(&c.points);
            length_nm = length_nm.saturating_add(length);
            entry.0 += 1;
            entry.2 = entry.2.saturating_add(length);
            ops.push(Operation::AddTrack {
                id,
                track: track.clone(),
            });
            tracks.push((id, track));
        } else {
            let via = Via {
                net,
                at: point(c.points[0])?,
                drill: Nm::new(c.width)
                    .map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
                diameter: Nm::new(c.diameter)
                    .map_err(|e| RouteError::new(OUT_OF_RANGE, e.to_string()))?,
                from_layer: first,
                to_layer: last,
            };
            entry.1 += 1;
            ops.push(Operation::AddVia {
                id,
                via: via.clone(),
            });
            vias.push((id, via));
        }
    }
    let operation = Operation::Batch { ops };

    let mut result = board.clone();
    result
        .apply(&operation)
        .map_err(|e| RouteError::new(OPERATION_FAILED, e.to_string()))?;
    let before = findings(board, ctx);
    let after = findings(&result, ctx);
    let changes = diff(&before, &after);
    if changes.adds_error() {
        let shown: Vec<String> = changes
            .added
            .iter()
            .filter(|f| f.severity == eh3d_phoenix_rules::Severity::Error)
            .take(5)
            .map(|f| format!("{} {}", f.code, f.location))
            .collect();
        return Err(RouteError::new(
            RULES_REFUSE,
            format!("the session would add rule errors: {}", shown.join("; ")),
        ));
    }
    let unrouted_before = ratsnest(board, settings)?.len();
    let unrouted_after = ratsnest(&result, settings)?.len();
    Ok(SesPlan {
        operation,
        tracks,
        vias,
        already_there,
        length_nm,
        unrouted_before,
        unrouted_after,
        diff: changes,
        nets: per_net
            .into_iter()
            .map(|(name, (t, v, l))| (name, t, v, l))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(unit: &str, per: &str) -> Resolution {
        let node = parse(&format!("(resolution {unit} {per})")).unwrap();
        Resolution::parse(&node).unwrap()
    }

    #[test]
    fn coordinates_are_converted_to_nanometres_and_rounded() {
        assert_eq!(res("um", "1000").nm("12345").unwrap(), 12_345);
        assert_eq!(res("um", "10").nm("12345").unwrap(), 1_234_500);
        assert_eq!(res("mm", "1").nm("1.5").unwrap(), 1_500_000);
        assert_eq!(res("mil", "10").nm("-3").unwrap(), -7_620);
        assert_eq!(res("um", "1000").nm("0.4").unwrap(), 0);
        assert_eq!(res("um", "1000").nm("0.5").unwrap(), 1);
        assert_eq!(res("um", "1000").nm("-0.5").unwrap(), -1);
    }

    #[test]
    fn a_bad_resolution_or_number_is_refused() {
        for text in [
            "(resolution um)",
            "(resolution furlong 10)",
            "(resolution um 0)",
            "(resolution um 1.5)",
        ] {
            let node = parse(text).unwrap();
            assert_eq!(Resolution::parse(&node).unwrap_err().code, SES_UNREADABLE);
        }
        let r = res("um", "1000");
        assert_eq!(r.nm("abc").unwrap_err().code, SES_UNREADABLE);
        assert_eq!(r.nm("1e3").unwrap_err().code, SES_UNREADABLE);
        assert_eq!(r.nm("99999999999").unwrap_err().code, OUT_OF_RANGE);
    }

    #[test]
    fn the_size_of_a_via_is_read_from_its_name() {
        assert_eq!(
            via_from_name("Via_600000_300000_nm"),
            Some((600_000, 300_000))
        );
        assert_eq!(via_from_name("Via_600000_nm"), None);
        assert_eq!(via_from_name("Via_0_300000_nm"), None);
        assert_eq!(via_from_name("Via[0-1]_600:300_um"), None);
    }

    #[test]
    fn repeated_vertices_are_dropped_and_the_direction_is_fixed() {
        assert_eq!(normalised(&[(5, 5), (5, 5), (1, 1)]), vec![(1, 1), (5, 5)]);
        assert_eq!(normalised(&[(1, 1), (5, 5)]), vec![(1, 1), (5, 5)]);
        assert_eq!(normalised(&[(3, 3), (3, 3)]).len(), 1);
    }

    #[test]
    fn the_length_of_a_path_is_the_sum_of_its_pieces() {
        assert_eq!(length_of(&[(0, 0), (3_000, 4_000)]), 5_000);
        assert_eq!(length_of(&[(0, 0), (1_000, 0), (1_000, 2_000)]), 3_000);
        assert_eq!(length_of(&[(0, 0)]), 0);
    }

    #[test]
    fn text_that_is_not_a_session_is_refused() {
        let mut next = {
            let mut n = 0_u32;
            move || {
                n += 1;
                eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, 1, u64::from(n))
                    .unwrap()
            }
        };
        let board = Board::default();
        let ctx = RuleContext::default();
        let settings = RouteSettings::default();
        for (text, code) in [
            ("(pcb x)", SES_UNREADABLE),
            ("(session x)", SES_UNREADABLE),
            ("(session x (routes))", SES_UNREADABLE),
            ("(session x (routes (resolution um 10)))", SES_UNREADABLE),
            ("(session", SES_UNREADABLE),
        ] {
            let e = import(&board, &ctx, &settings, text, &mut next).unwrap_err();
            assert_eq!(e.code, code, "{text}");
        }
    }
}
