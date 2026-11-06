// SPDX-License-Identifier: GPL-3.0-or-later
//! The return path of the tracks: a track that crosses the edge or a split of the copper next to
//! it, and the stubs that a via leaves beyond the last layer a signal uses.
//!
//! The copper next to a signal layer is the zones of the copper layers above and below it in
//! `copper_stack`. A track is sampled every `plane_sample_nm`; the zone each sample lies in (by the
//! outline of the zone, not by the fill with its antipads and thermal reliefs) is compared with
//! the one of the sample before. A track is reported when every neighbour that has zones changes
//! along it. A via stub is the part of the barrel between the last layer with a track end at the
//! via and the end of the via.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id, Track};
use eh3d_phoenix_stackup::{Layer as StackLayer, StackupJob};
use serde::Serialize;

use crate::codes::{Finding, Severity, NO_REFERENCE, PLANE_SPLIT, STUB_LONG};
use crate::netdata::{layer_name, net_name, P};
use crate::phys::{bandwidth_ghz, round_to};
use crate::settings::SiSettings;

/// A zone as the net it belongs to and its outline.
type ZoneOutline = (Id, Vec<P>);

/// A neighbouring layer along which the copper changes: the layer, the number of changes, the
/// first point after a change and whether the track runs over no copper of it.
type Broken = (Id, usize, Option<P>, bool);

/// The most points taken along one track.
const MAX_SAMPLES: f64 = 20_000.0;

/// A track whose return path is broken.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlaneCrossing {
    /// The net of the track.
    pub net: String,
    /// The layer of the track.
    pub layer: String,
    /// The layer of the copper that was looked at.
    pub reference: String,
    /// How many times the copper under the track changes.
    pub changes: usize,
    /// Whether the track runs over no copper of that layer at all.
    pub over_nothing: bool,
    /// Where the first change is, in millimetres.
    pub x_mm: f64,
    /// Where the first change is, in millimetres.
    pub y_mm: f64,
}

/// A stub of a via.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StubRow {
    /// The net of the via.
    pub net: String,
    /// Where the via is, in millimetres.
    pub x_mm: f64,
    /// Where the via is, in millimetres.
    pub y_mm: f64,
    /// Which end of the via the stub is on: `top` or `bottom`.
    pub side: &'static str,
    /// The length of the stub, in millimetres.
    pub length_mm: f64,
    /// The delay of the stub, in picoseconds.
    pub delay_ps: f64,
    /// The frequency at which the stub is a quarter of a wavelength, in GHz.
    pub resonance_ghz: f64,
}

/// The result for a board.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReturnReport {
    /// The tracks that were looked at over copper.
    pub tracks_checked: usize,
    /// The tracks whose return path is broken.
    pub crossings: Vec<PlaneCrossing>,
    /// The stubs of the vias.
    pub stubs: Vec<StubRow>,
}

/// Whether a point lies inside a polygon (the last point joins the first); the points on the
/// boundary count as either, decided by the usual rule of the crossing number.
fn inside(polygon: &[P], p: P) -> bool {
    let mut odd = false;
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        if (a.1 > p.1) != (b.1 > p.1) {
            let dy = i128::from(b.1) - i128::from(a.1);
            let lhs = (i128::from(p.0) - i128::from(a.0)) * dy;
            let rhs = (i128::from(b.0) - i128::from(a.0)) * (i128::from(p.1) - i128::from(a.1));
            let left_of_edge = if dy > 0 { lhs < rhs } else { lhs > rhs };
            if left_of_edge {
                odd = !odd;
            }
        }
    }
    odd
}

/// The net of the first zone that holds the point.
fn zone_at(zones: &[ZoneOutline], p: P) -> Option<Id> {
    zones
        .iter()
        .find(|(_, outline)| inside(outline, p))
        .map(|(net, _)| *net)
}

/// The points along a track, every `step` nanometres at most.
fn samples(track: &Track, step: f64) -> Vec<P> {
    let mut points = Vec::new();
    for segment in &track.path {
        let (a, b) = (segment.start(), segment.end());
        let (ax, ay) = (a.x_nm.get(), a.y_nm.get());
        let (bx, by) = (b.x_nm.get(), b.y_nm.get());
        let length = ((bx - ax) as f64).hypot((by - ay) as f64);
        let pieces = (length / step).ceil().max(1.0) as i64;
        for k in 0..pieces {
            let t = k as f64 / pieces as f64;
            points.push((
                ax + ((bx - ax) as f64 * t).round() as i64,
                ay + ((by - ay) as f64 * t).round() as i64,
            ));
        }
        if points.last() != Some(&(bx, by)) {
            points.push((bx, by));
        }
    }
    points
}

/// How the copper under a series of points changes: the number of changes, the first point after
/// a change and whether no point lies over a zone.
fn walk(zones: &[ZoneOutline], points: &[P]) -> (usize, Option<P>, bool) {
    let mut changes = 0;
    let mut first = None;
    let mut previous: Option<Option<Id>> = None;
    let mut any = false;
    for p in points {
        let here = zone_at(zones, *p);
        any |= here.is_some();
        if let Some(before) = previous {
            if before != here {
                changes += 1;
                if first.is_none() {
                    first = Some(*p);
                }
            }
        }
        previous = Some(here);
    }
    (changes, first, !any)
}

/// The tracks that cross the edge or a split of the copper next to them.
pub(crate) fn planes(
    board: &Board,
    settings: &SiSettings,
    findings: &mut Vec<Finding>,
) -> (usize, Vec<PlaneCrossing>) {
    if board.copper_stack.len() < 2 {
        findings.push(Finding::new(
            NO_REFERENCE,
            Severity::Info,
            None,
            "the board has no order of copper layers (copper_stack), so no track is looked at over its plane",
        ));
        return (0, Vec::new());
    }
    let mut zones: BTreeMap<Id, Vec<ZoneOutline>> = BTreeMap::new();
    for zone in board.zones.values() {
        let outline: Vec<P> = zone
            .outline
            .iter()
            .map(|p| (p.x_nm.get(), p.y_nm.get()))
            .collect();
        zones
            .entry(zone.layer)
            .or_default()
            .push((zone.net, outline));
    }
    let mut told: BTreeSet<Id> = BTreeSet::new();
    let mut checked = 0;
    let mut crossings = Vec::new();
    for track in board.tracks.values() {
        let Some(at) = board.copper_stack.iter().position(|l| *l == track.layer) else {
            continue;
        };
        let mut neighbours: Vec<Id> = Vec::new();
        if at > 0 {
            neighbours.push(board.copper_stack[at - 1]);
        }
        if let Some(next) = board.copper_stack.get(at + 1) {
            neighbours.push(*next);
        }
        neighbours.retain(|n| zones.contains_key(n));
        if neighbours.is_empty() {
            if told.insert(track.layer) {
                findings.push(Finding::new(
                    NO_REFERENCE,
                    Severity::Info,
                    Some(layer_name(board, track.layer).as_str()),
                    "no zone on the copper layers next to this layer",
                ));
            }
            continue;
        }
        checked += 1;
        let total: f64 = track
            .path
            .iter()
            .map(|s| {
                let (a, b) = (s.start(), s.end());
                ((a.x_nm.get() - b.x_nm.get()) as f64).hypot((a.y_nm.get() - b.y_nm.get()) as f64)
            })
            .sum();
        let step = (settings.plane_sample_nm as f64).max(total / MAX_SAMPLES);
        let points = samples(track, step);
        let mut broken: Vec<Broken> = Vec::new();
        for n in &neighbours {
            let (changes, first, nothing) = walk(&zones[n], &points);
            if changes > 0 || nothing {
                broken.push((*n, changes, first, nothing));
            }
        }
        if broken.len() != neighbours.len() {
            continue;
        }
        let (reference, changes, first, nothing) = broken[0];
        let (x, y) = first.or_else(|| points.first().copied()).unwrap_or((0, 0));
        crossings.push(PlaneCrossing {
            net: net_name(board, track.net),
            layer: layer_name(board, track.layer),
            reference: layer_name(board, reference),
            changes,
            over_nothing: nothing,
            x_mm: round_to(x as f64 / 1.0e6, 3),
            y_mm: round_to(y as f64 / 1.0e6, 3),
        });
    }
    crossings.sort_by(|a, b| {
        a.net
            .cmp(&b.net)
            .then(a.layer.cmp(&b.layer))
            .then(a.x_mm.total_cmp(&b.x_mm))
            .then(a.y_mm.total_cmp(&b.y_mm))
    });
    for c in &crossings {
        let detail = if c.over_nothing {
            format!(
                "the track on {} runs over no copper of {}, near ({:.2}, {:.2}) mm",
                c.layer, c.reference, c.x_mm, c.y_mm
            )
        } else {
            format!(
                "the copper of {} under the track on {} changes {} time(s), first near ({:.2}, {:.2}) mm",
                c.reference, c.layer, c.changes, c.x_mm, c.y_mm
            )
        };
        findings.push(Finding::new(
            PLANE_SPLIT,
            Severity::Warning,
            Some(c.net.as_str()),
            detail,
        ));
    }
    (checked, crossings)
}

/// The length of the barrel between the middle of the copper of two layers, in nanometres, from
/// the stack-up.
fn stack_span(job: &StackupJob, first: &str, second: &str) -> Option<i64> {
    let layers = &job.stackup.layers;
    let i = job.stackup.find(first)?;
    let j = job.stackup.find(second)?;
    let (lo, hi) = (i.min(j), i.max(j));
    let mut sum: i64 = 0;
    for (k, layer) in layers.iter().enumerate() {
        if k < lo || k > hi {
            continue;
        }
        let t = layer.thickness_nm().ok()?;
        let copper_end = matches!(layer, StackLayer::Copper { .. }) && (k == lo || k == hi);
        sum = sum.saturating_add(if copper_end { t / 2 } else { t });
    }
    Some(sum)
}

/// The stubs of the vias.
pub(crate) fn stubs(
    board: &Board,
    settings: &SiSettings,
    job: Option<&StackupJob>,
    findings: &mut Vec<Finding>,
) -> Vec<StubRow> {
    let mut ends: BTreeMap<Id, BTreeSet<(Id, P)>> = BTreeMap::new();
    for track in board.tracks.values() {
        let set = ends.entry(track.net).or_default();
        for segment in &track.path {
            for p in [segment.start(), segment.end()] {
                set.insert((track.layer, (p.x_nm.get(), p.y_nm.get())));
            }
        }
    }
    let mut rows = Vec::new();
    for via in board.vias.values() {
        let stack = &board.copper_stack;
        let (Some(from), Some(to)) = (
            stack.iter().position(|l| *l == via.from_layer),
            stack.iter().position(|l| *l == via.to_layer),
        ) else {
            continue;
        };
        let (lo, hi) = (from.min(to), from.max(to));
        if lo == hi {
            continue;
        }
        let at = (via.at.x_nm.get(), via.at.y_nm.get());
        let Some(net_ends) = ends.get(&via.net) else {
            continue;
        };
        let used: Vec<usize> = (lo..=hi)
            .filter(|k| net_ends.contains(&(stack[*k], at)))
            .collect();
        let (Some(first_used), Some(last_used)) = (used.first().copied(), used.last().copied())
        else {
            continue;
        };
        let span = |a: usize, b: usize| -> i64 {
            let from_stack = job.and_then(|j| {
                stack_span(
                    j,
                    &layer_name(board, stack[a]),
                    &layer_name(board, stack[b]),
                )
            });
            from_stack.unwrap_or_else(|| {
                settings.default_via_length_nm * (b - a) as i64 / (hi - lo) as i64
            })
        };
        let mut found: Vec<(&'static str, i64)> = Vec::new();
        if first_used > lo {
            found.push(("top", span(lo, first_used)));
        }
        if last_used < hi {
            found.push(("bottom", span(last_used, hi)));
        }
        let net = net_name(board, via.net);
        let tr = settings.rise_for(&net);
        for (side, length_nm) in found {
            let length_mm = length_nm as f64 / 1.0e6;
            let delay_ps = length_mm * settings.via_delay_ps_per_mm;
            let resonance_ghz = if delay_ps > 0.0 {
                250.0 / delay_ps
            } else {
                0.0
            };
            let too_long =
                length_nm > settings.max_stub_nm || 2.0 * delay_ps > tr * settings.stub_tr_fraction;
            if too_long {
                let in_band = resonance_ghz > 0.0 && resonance_ghz < bandwidth_ghz(tr);
                let severity = if in_band {
                    Severity::Error
                } else {
                    Severity::Warning
                };
                findings.push(Finding::new(
                    STUB_LONG,
                    severity,
                    Some(net.as_str()),
                    format!(
                        "{side} stub of {length_mm:.2} mm ({delay_ps:.1} ps) resonates near {resonance_ghz:.1} GHz; the edge needs {:.2} GHz",
                        bandwidth_ghz(tr)
                    ),
                ));
            }
            rows.push(StubRow {
                net: net.clone(),
                x_mm: round_to(at.0 as f64 / 1.0e6, 3),
                y_mm: round_to(at.1 as f64 / 1.0e6, 3),
                side,
                length_mm: round_to(length_mm, 3),
                delay_ps: round_to(delay_ps, 2),
                resonance_ghz: round_to(resonance_ghz, 2),
            });
        }
    }
    rows.sort_by(|a, b| {
        a.net
            .cmp(&b.net)
            .then(a.x_mm.total_cmp(&b.x_mm))
            .then(a.y_mm.total_cmp(&b.y_mm))
            .then(a.side.cmp(b.side))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_inside_a_square_and_not_outside_it() {
        let square = [(0, 0), (10, 0), (10, 10), (0, 10)];
        assert!(inside(&square, (5, 5)));
        assert!(!inside(&square, (15, 5)));
        assert!(!inside(&square, (5, -1)));
        assert!(!inside(&square, (-1, 5)));
        // a concave outline: an L
        let l = [(0, 0), (10, 0), (10, 4), (4, 4), (4, 10), (0, 10)];
        assert!(inside(&l, (2, 8)));
        assert!(inside(&l, (8, 2)));
        assert!(!inside(&l, (8, 8)));
        assert!(!inside(&[(0, 0), (1, 1)], (0, 0)));
    }

    #[test]
    fn the_copper_under_a_series_of_points_is_followed() {
        let a: Id = "01890a5d-ac96-7ab2-80e2-4536629c1301".parse().unwrap();
        let zones: Vec<ZoneOutline> = vec![(a, vec![(0, 0), (10, 0), (10, 10), (0, 10)])];
        let (changes, first, nothing) = walk(&zones, &[(1, 5), (5, 5), (9, 5)]);
        assert_eq!((changes, first, nothing), (0, None, false));
        let (changes, first, nothing) = walk(&zones, &[(5, 5), (12, 5), (20, 5)]);
        assert_eq!((changes, first, nothing), (1, Some((12, 5)), false));
        let (changes, _, nothing) = walk(&zones, &[(20, 5), (30, 5)]);
        assert_eq!((changes, nothing), (0, true));
        assert_eq!(walk(&zones, &[]), (0, None, true));
    }
}
