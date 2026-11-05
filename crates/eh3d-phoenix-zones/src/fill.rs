// SPDX-License-Identifier: GPL-3.0-or-later
//! The fill of the zones of a board.
//!
//! A zone is filled in these steps, always in the same order:
//!
//! 1. the outline is cut into convex pieces (the outline itself when it is convex, else triangles);
//! 2. every piece of copper of another net on the layer, the edge of the board, the pads of the net
//!    that are kept apart and the zones of a higher priority are turned into convex polygons that
//!    hold the copper grown by the clearance;
//! 3. each such polygon is taken out of every piece it touches (exact integer clipping, the cut
//!    points rounded to the nanometre);
//! 4. the thermal reliefs are taken out the same way: four (or two) blocks around each pad of the
//!    net, leaving the spokes;
//! 5. pieces outside the board are dropped, the rest are grouped into islands by the borders they
//!    share, and an island is dropped when it is smaller than the minimum area, narrower than the
//!    minimum width or touches no copper of the net.
//!
//! The width test uses the convex hull of the island, so it only finds islands that are narrow as
//! a whole (a strip, a sliver); a narrow neck inside a wide island is not found.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id, Zone};
use serde::{Deserialize, Serialize};

use crate::codes::{
    Finding, Severity, ZoneError, ARC_CAUTIOUS, FILL_EMPTY, ISLAND_NARROW, ISLAND_SMALL,
    ISLAND_UNCONNECTED, LIMIT_EXCEEDED, OUTLINE_UNUSABLE, PRIORITY_TIE, RELIEF_IMPOSSIBLE,
    ZONE_NOT_FOUND,
};
use crate::geom::{
    area2, bbox, bbox_grow, bbox_overlap, convex_hull, drop_collinear, grown_hull, inside_convex,
    inside_polygon, is_convex, is_simple, make_ccw, narrower_than, separated, share_edge,
    subtract_convex, triangulate, usable, vertex_mean, Bx, P,
};
use crate::items::{collect, copper_layers, half_up, Item, Kind, LayerItems};
use crate::rules::{ZoneEntry, ZoneFile, ZoneRules};
use crate::settings::{Connection, ZoneSettings};

/// The name of the format of the file of filled zones.
pub const FILL_FORMAT: &str = "phoenix-zone-fill/1";

/// A zone after the fill: the rules it was filled by and the polygons of copper. The polygons are
/// counter-clockwise, start at their smallest point and are sorted, so the same board always gives
/// the same bytes. They may touch along their sides and never overlap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilledZone {
    /// The zone.
    pub zone: Id,
    /// Its net.
    pub net: Id,
    /// Its layer.
    pub layer: Id,
    /// The priority it was filled with.
    pub priority: i32,
    /// The clearance of the zone itself, in nanometres (the rules may ask for more).
    pub clearance_nm: i64,
    /// The narrowest island that was kept, in nanometres.
    pub min_width_nm: i64,
    /// The smallest island that was kept, in square nanometres.
    pub min_island_area_nm2: i64,
    /// How its pads were joined.
    pub connection: Connection,
    /// The area of the copper, in square nanometres.
    pub area_nm2: i64,
    /// The polygons, each a list of `[x, y]` in nanometres.
    pub polygons: Vec<Vec<[i64; 2]>>,
}

/// The file of filled zones (`phoenix zones fill --out`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FillFile {
    /// Must be [`FILL_FORMAT`].
    pub format: String,
    /// The name of the board it was made for.
    pub board: String,
    /// The tolerance the check allows for the rounded cut points, in nanometres.
    pub snap_tolerance_nm: i64,
    /// The zones, sorted by id.
    pub zones: Vec<FilledZone>,
}

/// What happened to one zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZoneStats {
    /// The zone.
    pub zone: Id,
    /// The convex pieces its outline was cut into.
    pub pieces: usize,
    /// The polygons of copper after the fill.
    pub polygons: usize,
    /// The islands that were kept.
    pub islands_kept: usize,
    /// The islands that were dropped.
    pub islands_removed: usize,
    /// The area of the copper, in square nanometres.
    pub area_nm2: i64,
}

/// The result of a fill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FillReport {
    /// The filled zones.
    pub fill: FillFile,
    /// One entry per zone that was filled or tried.
    pub stats: Vec<ZoneStats>,
    /// What to know about the fill.
    pub findings: Vec<Finding>,
}

/// A zone with its limits settled and its outline cut into convex pieces.
struct Resolved {
    id: Id,
    net: Id,
    layer: Id,
    priority: i32,
    clearance: i64,
    min_width: i64,
    min_island: i64,
    connection: Connection,
    outline: Vec<P>,
    pieces: Vec<Vec<P>>,
}

/// What a zone of a higher priority leaves to the ones below it.
struct Prior {
    net: Id,
    layer: Id,
    priority: i32,
    pieces: Vec<Vec<P>>,
}

/// A polygon to take out, with its box.
type Hull = (Bx, Vec<P>);

/// What a fill reads.
pub(crate) struct Ctx<'a> {
    pub rules: &'a ZoneRules,
    pub settings: &'a ZoneSettings,
    pub items: &'a LayerItems,
    pub board_outline: Vec<P>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(
        board: &'a Board,
        rules: &'a ZoneRules,
        settings: &'a ZoneSettings,
        items: &'a LayerItems,
    ) -> Ctx<'a> {
        let board_outline: Vec<P> = if board.outline.len() >= 3 {
            board
                .outline
                .iter()
                .map(|p| (p.x_nm.get(), p.y_nm.get()))
                .collect()
        } else {
            Vec::new()
        };
        Ctx {
            rules,
            settings,
            items,
            board_outline,
        }
    }
}

fn outline_of(zone: &Zone) -> Vec<P> {
    zone.outline
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect()
}

fn failure(code: &'static str, id: Id, detail: &str) -> Finding {
    Finding::new(code, Severity::Error, Some(id), detail)
}

fn prepare(
    id: Id,
    zone: &Zone,
    entry: Option<&ZoneEntry>,
    settings: &ZoneSettings,
) -> Result<Resolved, Finding> {
    if zone.outline.len() > settings.max_outline_points {
        return Err(failure(
            LIMIT_EXCEEDED,
            id,
            "the outline has more points than max_outline_points",
        ));
    }
    let mut outline = outline_of(zone);
    crate::geom::dedupe(&mut outline);
    if outline.len() < 3 {
        return Err(failure(OUTLINE_UNUSABLE, id, "fewer than three points"));
    }
    make_ccw(&mut outline);
    if area2(&outline) == 0 {
        return Err(failure(OUTLINE_UNUSABLE, id, "the outline has no area"));
    }
    if !is_simple(&outline) {
        return Err(failure(
            OUTLINE_UNUSABLE,
            id,
            "two sides of the outline touch",
        ));
    }
    let pieces = if is_convex(&outline) {
        vec![outline.clone()]
    } else {
        match triangulate(&outline) {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Err(failure(
                    OUTLINE_UNUSABLE,
                    id,
                    "the outline could not be cut into convex pieces",
                ))
            }
        }
    };
    Ok(Resolved {
        id,
        net: zone.net,
        layer: zone.layer,
        priority: entry.map_or(0, |e| e.priority),
        clearance: entry
            .and_then(|e| e.clearance_nm)
            .unwrap_or_else(|| zone.clearance.get()),
        min_width: entry
            .and_then(|e| e.min_width_nm)
            .unwrap_or(settings.min_width_nm),
        min_island: entry
            .and_then(|e| e.min_island_area_nm2)
            .unwrap_or(settings.min_island_area_nm2),
        connection: entry
            .and_then(|e| e.connection)
            .unwrap_or(settings.pad_connection),
        outline,
        pieces,
    })
}

/// True when the item of the layer is copper the zone of net `net` keeps apart: the copper of
/// another net or of none, and a pad or a via of its own net when it is not to be joined.
pub(crate) fn keeps_apart(item: &Item, net: Id, pad: Connection, via: Connection) -> bool {
    if item.net != Some(net) {
        return true;
    }
    match item.kind {
        Kind::Pad => pad == Connection::None,
        Kind::Via => via == Connection::None,
        Kind::Track => false,
    }
}

/// The four (or two) blocks of a thermal relief around an item, as convex polygons, or `None`
/// when the spokes are too wide for the ring.
fn relief_blocks(item: &Item, s: &ZoneSettings) -> Option<Vec<Vec<P>>> {
    let inner = item.radius.max(s.thermal_inner_radius_nm);
    let outer = inner
        .saturating_add(s.thermal_gap_nm)
        .max(s.thermal_outer_radius_nm);
    let h = half_up(s.thermal_spoke_width_nm);
    // the corner of the circumscribed octagon: tan(22.5 degrees) times the radius, rounded up
    let t = (i128::from(outer) * 414_214 + 999_999) / 1_000_000;
    let m = i64::try_from(t).unwrap_or(i64::MAX).saturating_add(1);
    if outer <= 0 || h >= m {
        return None;
    }
    let (cx, cy) = item.centre;
    let place = |pts: &[P]| -> Vec<P> { pts.iter().map(|&(x, y)| (cx + x, cy + y)).collect() };
    let mut blocks: Vec<Vec<P>> = Vec::new();
    if s.thermal_spokes == 4 {
        let mut current: Vec<P> = vec![(h, h), (outer, h), (outer, m), (m, outer), (h, outer)];
        for _ in 0..4 {
            blocks.push(place(&current));
            current = current.iter().map(|&(x, y)| (-y, x)).collect();
        }
    } else {
        let top: Vec<P> = vec![
            (-outer, h),
            (outer, h),
            (outer, m),
            (m, outer),
            (-m, outer),
            (-outer, m),
        ];
        let bottom: Vec<P> = top.iter().map(|&(x, y)| (-x, -y)).collect();
        blocks.push(place(&top));
        blocks.push(place(&bottom));
    }
    Some(blocks)
}

fn add_hull(out: &mut Vec<Hull>, zone_box: Bx, hull: Vec<P>) {
    if hull.len() >= 3 {
        let b = bbox(&hull);
        if bbox_overlap(b, zone_box) {
            out.push((b, hull));
        }
    }
}

/// Everything the zone has to keep away from, as convex polygons, in a fixed order. The flag says
/// that an arc was among it.
fn obstacles(
    ctx: &Ctx<'_>,
    z: &Resolved,
    higher: &[&Prior],
    findings: &mut Vec<Finding>,
) -> (Vec<Hull>, bool) {
    let sides = ctx.settings.circle_sides;
    let zone_box = bbox(&z.outline);
    let mut out: Vec<Hull> = Vec::new();
    let mut arcs = false;
    let edge = ctx.rules.min_edge_clearance_nm;
    let n = ctx.board_outline.len();
    if n >= 3 && edge > 0 {
        for (i, &a) in ctx.board_outline.iter().enumerate() {
            let b = ctx.board_outline[(i + 1) % n];
            add_hull(&mut out, zone_box, grown_hull(&[a, b], edge, sides));
        }
    }
    if let Some(list) = ctx.items.get(&z.layer) {
        for item in list {
            let apart = keeps_apart(item, z.net, z.connection, ctx.settings.via_connection);
            if !apart {
                let join = match item.kind {
                    Kind::Pad => z.connection,
                    Kind::Via => ctx.settings.via_connection,
                    Kind::Track => Connection::Solid,
                };
                if join == Connection::Thermal {
                    match relief_blocks(item, ctx.settings) {
                        Some(blocks) => {
                            for block in blocks {
                                add_hull(&mut out, zone_box, block);
                            }
                        }
                        None => findings.push(Finding::new(
                            RELIEF_IMPOSSIBLE,
                            Severity::Warning,
                            Some(z.id),
                            format!(
                                "{}: the spokes are too wide, it is joined solid",
                                item.label
                            ),
                        )),
                    }
                }
                continue;
            }
            let need = ctx.rules.clearance(z.clearance, z.net, item.net);
            if !bbox_overlap(bbox_grow(item.bounds, need), zone_box) {
                continue;
            }
            if item.arc {
                arcs = true;
            }
            add_hull(
                &mut out,
                zone_box,
                grown_hull(&item.centres, item.half.saturating_add(need), sides),
            );
        }
    }
    for prior in higher {
        let need = if prior.net == z.net {
            0
        } else {
            ctx.rules.clearance(z.clearance, z.net, Some(prior.net))
        };
        for piece in &prior.pieces {
            add_hull(&mut out, zone_box, grown_hull(piece, need, sides));
        }
    }
    (out, arcs)
}

/// The points where copper of the net touches the fill on the layer.
pub(crate) fn join_points(
    ctx: &Ctx<'_>,
    net: Id,
    layer: Id,
    pad: Connection,
    via: Connection,
) -> Vec<P> {
    let mut points: Vec<P> = Vec::new();
    if let Some(list) = ctx.items.get(&layer) {
        for item in list {
            if item.net != Some(net) {
                continue;
            }
            match item.kind {
                Kind::Pad if pad != Connection::None => points.push(item.centre),
                Kind::Via if via != Connection::None => points.push(item.centre),
                Kind::Track => {
                    for (a, b) in &item.segs {
                        points.push(*a);
                        points.push(*b);
                    }
                }
                _ => {}
            }
        }
    }
    points
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Groups the pieces into islands: pieces that share a stretch of border are one island. The
/// islands are lists of indexes, each list and the list of lists in ascending order.
pub(crate) fn islands(pieces: &[Vec<P>], tol: i64) -> Vec<Vec<usize>> {
    let n = pieces.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let boxes: Vec<Bx> = pieces.iter().map(|p| bbox_grow(bbox(p), tol)).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (boxes[i].0, i));
    for (a, &i) in order.iter().enumerate() {
        for &j in &order[a + 1..] {
            if boxes[j].0 > boxes[i].2 {
                break;
            }
            if boxes[j].1 <= boxes[i].3
                && boxes[i].1 <= boxes[j].3
                && share_edge(&pieces[i], &pieces[j], tol)
            {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri.max(rj)] = ri.min(rj);
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }
    let mut out: Vec<Vec<usize>> = groups.into_values().collect();
    out.sort();
    out
}

/// What is known of one island.
pub(crate) struct IslandInfo {
    pub members: Vec<usize>,
    pub area2: i128,
    pub narrow: bool,
    pub connected: bool,
}

/// Measures the islands: the area, whether the island is narrower than `min_width` as a whole and
/// whether one of the `join` points is inside it.
pub(crate) fn judge(
    pieces: &[Vec<P>],
    groups: Vec<Vec<usize>>,
    min_width: i64,
    join: &[P],
) -> Vec<IslandInfo> {
    groups
        .into_iter()
        .map(|members| {
            let area2_sum: i128 = members.iter().map(|&m| area2(&pieces[m])).sum();
            let narrow = min_width > 0 && {
                let all: Vec<P> = members
                    .iter()
                    .flat_map(|&m| pieces[m].iter().copied())
                    .collect();
                narrower_than(&convex_hull(&all), min_width)
            };
            let connected = join
                .iter()
                .any(|&p| members.iter().any(|&m| inside_convex(p, &pieces[m])));
            IslandInfo {
                members,
                area2: area2_sum,
                narrow,
                connected,
            }
        })
        .collect()
}

/// A polygon in its fixed form: counter-clockwise, without points on its sides, starting at its
/// smallest point.
fn canonical(poly: &[P]) -> Vec<P> {
    let mut p = drop_collinear(poly);
    make_ccw(&mut p);
    let at = p
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| **v)
        .map_or(0, |(i, _)| i);
    p.rotate_left(at);
    p
}

fn fill_one(
    ctx: &Ctx<'_>,
    z: &Resolved,
    higher: &[&Prior],
) -> (Option<FilledZone>, ZoneStats, Vec<Finding>) {
    let mut findings: Vec<Finding> = Vec::new();
    let mut stats = ZoneStats {
        zone: z.id,
        pieces: z.pieces.len(),
        polygons: 0,
        islands_kept: 0,
        islands_removed: 0,
        area_nm2: 0,
    };
    let (hulls, arcs) = obstacles(ctx, z, higher, &mut findings);
    if arcs {
        findings.push(Finding::new(
            ARC_CAUTIOUS,
            Severity::Info,
            Some(z.id),
            "the pieces of an arc are widened by their sagitta",
        ));
    }
    let mut current: Vec<Vec<P>> = z.pieces.clone();
    for (obox, hull) in &hulls {
        let mut next: Vec<Vec<P>> = Vec::with_capacity(current.len());
        for piece in std::mem::take(&mut current) {
            if !bbox_overlap(bbox(&piece), *obox) {
                next.push(piece);
                continue;
            }
            match subtract_convex(&piece, hull) {
                None => next.push(piece),
                Some(parts) => next.extend(parts),
            }
        }
        current = next;
        if current.len() > ctx.settings.max_fragments {
            findings.push(failure(
                LIMIT_EXCEEDED,
                z.id,
                "the zone was cut into more pieces than max_fragments",
            ));
            return (None, stats, findings);
        }
    }
    current.retain(|p| usable(p));
    if !ctx.board_outline.is_empty() {
        current.retain(|p| inside_polygon(vertex_mean(p), &ctx.board_outline));
    }
    let tol = ctx.settings.snap_tolerance_nm;
    let groups = islands(&current, tol);
    let join = join_points(
        ctx,
        z.net,
        z.layer,
        z.connection,
        ctx.settings.via_connection,
    );
    let infos = judge(&current, groups, z.min_width, &join);
    let mut kept: Vec<Vec<P>> = Vec::new();
    for info in infos {
        let area = info.area2 / 2;
        let reason = if area < i128::from(z.min_island) {
            Some((ISLAND_SMALL, "smaller than the minimum area"))
        } else if info.narrow {
            Some((ISLAND_NARROW, "narrower than the minimum width"))
        } else if !info.connected && !ctx.settings.keep_unconnected_islands {
            Some((ISLAND_UNCONNECTED, "touches no copper of the net"))
        } else {
            None
        };
        match reason {
            Some((code, why)) => {
                stats.islands_removed += 1;
                findings.push(Finding::new(
                    code,
                    Severity::Info,
                    Some(z.id),
                    format!(
                        "island of {} pieces and {} square nm dropped: {}",
                        info.members.len(),
                        area,
                        why
                    ),
                ));
            }
            None => {
                stats.islands_kept += 1;
                for m in info.members {
                    kept.push(current[m].clone());
                }
            }
        }
    }
    let mut polygons: Vec<Vec<P>> = kept.iter().map(|p| canonical(p)).collect();
    polygons.sort();
    polygons.dedup();
    let total: i128 = polygons.iter().map(|p| area2(p)).sum::<i128>() / 2;
    stats.polygons = polygons.len();
    stats.area_nm2 = i64::try_from(total).unwrap_or(i64::MAX);
    if polygons.is_empty() {
        findings.push(Finding::new(
            FILL_EMPTY,
            Severity::Warning,
            Some(z.id),
            "nothing is left of the zone",
        ));
    }
    let filled = FilledZone {
        zone: z.id,
        net: z.net,
        layer: z.layer,
        priority: z.priority,
        clearance_nm: z.clearance,
        min_width_nm: z.min_width,
        min_island_area_nm2: z.min_island,
        connection: z.connection,
        area_nm2: stats.area_nm2,
        polygons: polygons
            .iter()
            .map(|p| p.iter().map(|&(x, y)| [x, y]).collect())
            .collect(),
    };
    (Some(filled), stats, findings)
}

fn push_ties(prepared: &[Resolved], findings: &mut Vec<Finding>) {
    for (i, a) in prepared.iter().enumerate() {
        for b in &prepared[i + 1..] {
            if a.layer != b.layer || a.priority != b.priority || a.net == b.net {
                continue;
            }
            let overlap = a
                .pieces
                .iter()
                .any(|pa| b.pieces.iter().any(|pb| !separated(pa, pb)));
            if overlap {
                findings.push(Finding::new(
                    PRIORITY_TIE,
                    Severity::Warning,
                    Some(a.id),
                    format!("overlaps zone {} of priority {}", b.id, b.priority),
                ));
            }
        }
    }
}

/// Fills every zone of the board. A zone whose outline cannot be filled, or that is cut into too
/// many pieces, gets an error finding and no copper; the others are filled. The zone file may
/// give a priority and limits to single zones; a zone it names that the board does not have is an
/// error and nothing is filled.
pub fn fill_board(
    board: &Board,
    rules: &ZoneRules,
    settings: &ZoneSettings,
    file: Option<&ZoneFile>,
) -> Result<FillReport, ZoneError> {
    settings.validate()?;
    if let Some(f) = file {
        for entry in &f.zones {
            if !board.zones.contains_key(&entry.zone) {
                return Err(ZoneError::new(
                    ZONE_NOT_FOUND,
                    format!("zone {} is not on the board", entry.zone),
                ));
            }
        }
    }
    let copper = copper_layers(board);
    let items = collect(board, settings.arc_piece_degrees);
    let ctx = Ctx::new(board, rules, settings, &items);
    let mut findings: Vec<Finding> = Vec::new();
    let mut prepared: Vec<Resolved> = Vec::new();
    for (id, zone) in &board.zones {
        if !copper.contains(&zone.layer) {
            findings.push(Finding::new(
                OUTLINE_UNUSABLE,
                Severity::Warning,
                Some(*id),
                "the layer of the zone is not a copper layer",
            ));
            continue;
        }
        match prepare(*id, zone, file.and_then(|f| f.entry(*id)), settings) {
            Ok(resolved) => prepared.push(resolved),
            Err(finding) => findings.push(finding),
        }
    }
    prepared.sort_by_key(|z| (Reverse(z.priority), z.id));
    push_ties(&prepared, &mut findings);
    let priors: Vec<Prior> = prepared
        .iter()
        .map(|z| Prior {
            net: z.net,
            layer: z.layer,
            priority: z.priority,
            pieces: z.pieces.clone(),
        })
        .collect();
    let mut zones: Vec<FilledZone> = Vec::new();
    let mut stats: Vec<ZoneStats> = Vec::new();
    for (k, z) in prepared.iter().enumerate() {
        let higher: Vec<&Prior> = priors[..k]
            .iter()
            .filter(|p| p.layer == z.layer && p.priority > z.priority)
            .collect();
        let (filled, zone_stats, zone_findings) = fill_one(&ctx, z, &higher);
        if let Some(f) = filled {
            zones.push(f);
        }
        stats.push(zone_stats);
        findings.extend(zone_findings);
    }
    zones.sort_by_key(|z| z.zone);
    stats.sort_by_key(|s| s.zone);
    Ok(FillReport {
        fill: FillFile {
            format: FILL_FORMAT.to_owned(),
            board: board.name.clone(),
            snap_tolerance_nm: settings.snap_tolerance_nm,
            zones,
        },
        stats,
        findings,
    })
}
