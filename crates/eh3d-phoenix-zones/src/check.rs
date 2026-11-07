// SPDX-License-Identifier: GPL-3.0-or-later
//! The check of a fill: the polygons of copper are measured against the board by exact integer
//! distances, independently of the way they were made.
//!
//! For each filled zone it proves, with a tolerance of the cut points rounded to the nanometre:
//! that every polygon lies inside the outline of the zone and inside the board, that every side of
//! every polygon keeps the clearance from the copper of other nets (and from the pads and vias of
//! its own net that are not joined) and from the edge of the board, that no such copper is inside
//! a polygon, and that no island is smaller or narrower than the limits of the zone. An island
//! that touches no copper of the net is a warning.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, Id};

use crate::codes::{
    Finding, Severity, ZoneError, CLEARANCE_VIOLATED, FILL_MISMATCH, ISLAND_NARROW, ISLAND_SMALL,
    ISLAND_UNCONNECTED, OUTSIDE_OUTLINE, OUT_OF_RANGE,
};
use crate::fill::{
    islands, join_points, judge, keeps_apart, Ctx, FillFile, FilledZone, FILL_FORMAT,
};
use crate::geom::{
    bbox, bbox_grow, bbox_overlap, inside_polygon, make_ccw, point_seg_ge, seg_seg_ge, P,
};
use crate::items::collect;
use crate::rules::ZoneRules;
use crate::settings::{IslandAction, ZoneSettings};

/// The largest coordinate a polygon may have, as for the rest of the design.
const COORD_LIMIT: i64 = (1 << 30) - 1;

fn error(code: &'static str, zone: Id, detail: String) -> Finding {
    Finding::new(code, Severity::Error, Some(zone), detail)
}

/// The polygons of a filled zone as lists of points, or why they cannot be used.
fn read_polygons(fz: &FilledZone) -> Result<Vec<Vec<P>>, Finding> {
    let mut out: Vec<Vec<P>> = Vec::with_capacity(fz.polygons.len());
    for (k, polygon) in fz.polygons.iter().enumerate() {
        if polygon.len() < 3 {
            return Err(error(
                FILL_MISMATCH,
                fz.zone,
                format!("polygon {k} has fewer than three points"),
            ));
        }
        let mut points: Vec<P> = Vec::with_capacity(polygon.len());
        for xy in polygon {
            if xy[0].abs() > COORD_LIMIT || xy[1].abs() > COORD_LIMIT {
                return Err(error(
                    OUT_OF_RANGE,
                    fz.zone,
                    format!("polygon {k} has a coordinate outside the design range"),
                ));
            }
            points.push((xy[0], xy[1]));
        }
        make_ccw(&mut points);
        out.push(points);
    }
    Ok(out)
}

/// True when the point is inside the polygon or no farther than `tol` from its border.
fn inside_or_near(p: P, polygon: &[P], tol: i64) -> bool {
    if inside_polygon(p, polygon) {
        return true;
    }
    let n = polygon.len();
    polygon
        .iter()
        .enumerate()
        .any(|(i, &a)| !point_seg_ge(p, a, polygon[(i + 1) % n], i128::from(tol) + 1))
}

/// True when every side of the polygon is at least `limit` from the segment `ab`.
fn polygon_clear(polygon: &[P], a: P, b: P, limit: i128) -> bool {
    let n = polygon.len();
    polygon
        .iter()
        .enumerate()
        .all(|(i, &p)| seg_seg_ge(p, polygon[(i + 1) % n], a, b, limit))
}

/// Measures a fill against the board and the rules. A finding of severity error is a proven break
/// of a rule. The fill must have been made for a board of the same name and the same zones.
pub fn check_fill(
    board: &Board,
    rules: &ZoneRules,
    settings: &ZoneSettings,
    file: &FillFile,
) -> Result<Vec<Finding>, ZoneError> {
    settings.validate()?;
    if file.format != FILL_FORMAT {
        return Err(ZoneError::new(
            FILL_MISMATCH,
            format!("the format of the fill must be {FILL_FORMAT}"),
        ));
    }
    if file.board != board.name {
        return Err(ZoneError::new(
            FILL_MISMATCH,
            "the fill was made for a board of another name",
        ));
    }
    let items = collect(board, settings.arc_piece_degrees);
    let ctx = Ctx::new(board, rules, settings, &items);
    let tol = settings.snap_tolerance_nm;
    let mut findings: Vec<Finding> = Vec::new();
    for fz in &file.zones {
        let Some(zone) = board.zones.get(&fz.zone) else {
            findings.push(error(
                FILL_MISMATCH,
                fz.zone,
                "the zone is not on the board".to_owned(),
            ));
            continue;
        };
        if zone.net != fz.net || zone.layer != fz.layer {
            findings.push(error(
                FILL_MISMATCH,
                fz.zone,
                "the net or the layer of the zone changed".to_owned(),
            ));
            continue;
        }
        let polygons = match read_polygons(fz) {
            Ok(p) => p,
            Err(finding) => {
                findings.push(finding);
                continue;
            }
        };
        let outline: Vec<P> = zone
            .outline
            .iter()
            .map(|p| (p.x_nm.get(), p.y_nm.get()))
            .collect();
        // inside the zone and inside the board
        for (k, polygon) in polygons.iter().enumerate() {
            let in_zone = polygon.iter().all(|&v| inside_or_near(v, &outline, tol));
            let in_board = ctx.board_outline.is_empty()
                || polygon
                    .iter()
                    .all(|&v| inside_or_near(v, &ctx.board_outline, tol));
            if !in_zone || !in_board {
                findings.push(error(
                    OUTSIDE_OUTLINE,
                    fz.zone,
                    format!("polygon {k} leaves the zone or the board"),
                ));
            }
        }
        // clearance from the copper of other nets
        let mut reported: BTreeSet<String> = BTreeSet::new();
        if let Some(list) = items.get(&fz.layer) {
            for item in list {
                if !keeps_apart(item, fz.net, fz.connection, settings.via_connection) {
                    continue;
                }
                let need = rules.clearance(fz.clearance_nm, fz.net, item.net);
                let reach = bbox_grow(item.bounds, need);
                let limit = i128::from(item.half) + i128::from(need) - i128::from(tol);
                for polygon in &polygons {
                    if !bbox_overlap(bbox(polygon), reach) {
                        continue;
                    }
                    let clear = item
                        .segs
                        .iter()
                        .all(|&(a, b)| polygon_clear(polygon, a, b, limit))
                        && !item.centres.iter().any(|&c| inside_polygon(c, polygon));
                    if !clear && reported.insert(item.label.clone()) {
                        findings.push(error(
                            CLEARANCE_VIOLATED,
                            fz.zone,
                            format!("{} is closer than {} nm", item.label, need),
                        ));
                    }
                }
            }
        }
        // clearance from the edge of the board
        let n = ctx.board_outline.len();
        if n >= 3 && rules.min_edge_clearance_nm > 0 {
            let limit = i128::from(rules.min_edge_clearance_nm) - i128::from(tol);
            let mut close = false;
            for (i, &a) in ctx.board_outline.iter().enumerate() {
                let b = ctx.board_outline[(i + 1) % n];
                if polygons
                    .iter()
                    .any(|polygon| !polygon_clear(polygon, a, b, limit))
                {
                    close = true;
                }
            }
            if close {
                findings.push(error(
                    CLEARANCE_VIOLATED,
                    fz.zone,
                    format!(
                        "copper is closer than {} nm to the edge of the board",
                        rules.min_edge_clearance_nm
                    ),
                ));
            }
        }
        // islands
        let groups = islands(&polygons, tol);
        let join = join_points(
            &ctx,
            fz.net,
            fz.layer,
            fz.connection,
            settings.via_connection,
        );
        for info in judge(&polygons, groups, fz.min_width_nm, &join) {
            let area = info.area2 / 2;
            if area < i128::from(fz.min_island_area_nm2) {
                findings.push(error(
                    ISLAND_SMALL,
                    fz.zone,
                    format!("an island of {area} square nm"),
                ));
            }
            if info.narrow {
                findings.push(error(
                    ISLAND_NARROW,
                    fz.zone,
                    format!("an island of {} pieces", info.members.len()),
                ));
            }
            if !info.connected && settings.island_action() != IslandAction::Keep {
                findings.push(Finding::new(
                    ISLAND_UNCONNECTED,
                    Severity::Warning,
                    Some(fz.zone),
                    format!("an island of {} pieces", info.members.len()),
                ));
            }
        }
    }
    Ok(findings)
}

/// True when a finding is a proven break of a rule.
pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Error)
}
