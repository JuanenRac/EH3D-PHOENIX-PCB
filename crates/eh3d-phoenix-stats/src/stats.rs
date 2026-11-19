// SPDX-License-Identifier: GPL-3.0-or-later
//! The statistics of a board: how big it is, what it holds, how much copper each layer has and
//! which drills it uses.
//!
//! * **Copper of a layer** is the sum of four parts, each in square micrometres: the pads of the
//!   parts (the shape less the hole of a plated pad; a hole that is not plated has no copper), the
//!   tracks (length times width and the round ends), the vias (the ring between the drill and the
//!   pad, on every copper layer the via crosses) and the zones (the outline less its cutouts,
//!   before the clearances). The parts overlap one another where a track ends on a pad or a zone
//!   surrounds both, so the sum is an upper bound, and the copper of a layer is never counted
//!   above the area of the board.
//! * **Coverage** is the copper of the layer over the area of the board, in hundredths of a
//!   percent.
//! * **Balance** compares the coverage of the layers that mirror each other, see [`crate::balance`].
//! * **Drills** are counted by diameter: the vias, the plated pads and the holes that are not
//!   plated.
//!
//! Everything is an integer, the order is fixed, and nothing depends on the time or the machine,
//! so the same board gives the same bytes.

use std::collections::BTreeMap;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, Side};
use eh3d_phoenix_model::Point;
use serde::Serialize;

use crate::balance::{judge, BalancePair};
use crate::codes::{
    Finding, Severity, StatsError, LIMIT_EXCEEDED, NO_BOARD_AREA, REGION_UNEVEN,
    ZONE_OUTLINE_COUNTED,
};
use crate::geom::{circle_nm2, pad_copper_nm2, polygon_area_nm2, segment_length_nm, to_um2, whole};
use crate::regions::{capped_nm2, coverage_x100, spread_x100, Grid};
use crate::report::hundredths;
use crate::settings::StatsSettings;

/// The name of the format of the JSON report.
pub const FORMAT: &str = "phoenix-stats/1";

/// The size and the area of the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoardArea {
    /// The width of the box of the outline, in nanometres.
    pub width_nm: i64,
    /// The height of the box of the outline, in nanometres.
    pub height_nm: i64,
    /// The area inside the outline, in square micrometres.
    pub outline_um2: i64,
    /// The area of the openings inside the outline, in square micrometres.
    pub cutouts_um2: i64,
    /// The area of the board: the outline less its openings, in square micrometres.
    pub area_um2: i64,
}

/// What the board holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// The placed components.
    pub components: usize,
    /// The components on the top side.
    pub components_top: usize,
    /// The components on the bottom side.
    pub components_bottom: usize,
    /// The nets.
    pub nets: usize,
    /// The pads of the placed components.
    pub pads: usize,
    /// The pads without a drill.
    pub smd_pads: usize,
    /// The pads with a plated drill.
    pub through_pads: usize,
    /// The drilled holes that are not plated.
    pub unplated_holes: usize,
    /// The tracks.
    pub tracks: usize,
    /// The pieces of all the tracks.
    pub track_segments: usize,
    /// The length of all the tracks, in nanometres.
    pub track_length_nm: i64,
    /// The vias.
    pub vias: usize,
    /// The zones.
    pub zones: usize,
    /// The text and strokes of the silkscreen.
    pub silkscreen_items: usize,
}

/// The copper of one layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayerStats {
    /// The name of the layer.
    pub layer: String,
    /// Its place in the stack from 1 (the top); 0 when the board gives no order.
    pub position: usize,
    /// The pads with copper on it.
    pub pads: usize,
    /// The tracks on it.
    pub tracks: usize,
    /// The length of the tracks on it, in nanometres.
    pub track_length_nm: i64,
    /// The vias that cross it.
    pub vias: usize,
    /// The zones on it.
    pub zones: usize,
    /// The copper of its pads, in square micrometres.
    pub pads_um2: i64,
    /// The copper of its tracks, in square micrometres.
    pub tracks_um2: i64,
    /// The copper of its vias, in square micrometres.
    pub vias_um2: i64,
    /// The copper of its zones, in square micrometres.
    pub zones_um2: i64,
    /// The four parts together, never above the area of the board, in square micrometres.
    pub copper_um2: i64,
    /// The copper over the area of the board, in hundredths of a percent.
    pub coverage_x100: i64,
}

/// The copper of one layer in one cell of the grid over the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegionCell {
    /// The column of the cell, from 0 at the least x.
    pub column: usize,
    /// The row of the cell, from 0 at the least y.
    pub row: usize,
    /// The area of the board inside the cell (the outline less its cutouts), in square
    /// micrometres.
    pub area_um2: i64,
    /// The copper of the layer in the cell, never above its area, in square micrometres.
    pub copper_um2: i64,
    /// The copper over the area of the cell, in hundredths of a percent.
    pub coverage_x100: i64,
}

/// The copper of one layer by region of the board (`docs/STATS.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegionStats {
    /// The name of the layer.
    pub layer: String,
    /// The columns of the grid.
    pub columns: usize,
    /// The rows of the grid.
    pub rows: usize,
    /// The cells, row by row from the least y and in each row from the least x.
    pub cells: Vec<RegionCell>,
}

/// The holes of one diameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DrillStats {
    /// The diameter of the drill, in nanometres.
    pub diameter_nm: i64,
    /// The vias with this drill.
    pub vias: usize,
    /// The plated pads with this drill.
    pub plated_pads: usize,
    /// The holes that are not plated with this drill.
    pub unplated_pads: usize,
    /// The three together.
    pub total: usize,
}

/// How tightly the board is filled, per square centimetre, in hundredths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Density {
    /// The components per square centimetre of board, times 100.
    pub components_per_cm2_x100: i64,
    /// The pads per square centimetre of board, times 100.
    pub pads_per_cm2_x100: i64,
    /// The drilled holes (vias and drilled pads) per square centimetre of board, times 100.
    pub holes_per_cm2_x100: i64,
}

/// The statistics of a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoardStats {
    /// The name of the format.
    pub format: &'static str,
    /// The name of the board.
    pub name: String,
    /// Its size and area.
    pub board: BoardArea,
    /// What it holds.
    pub counts: Counts,
    /// The copper of each layer, from the top.
    pub layers: Vec<LayerStats>,
    /// The drills, from the smallest diameter.
    pub drills: Vec<DrillStats>,
    /// How tightly it is filled.
    pub density: Density,
    /// The pairs of layers that mirror each other and the difference of their copper; empty when
    /// the check is off or the board has fewer than two copper layers.
    pub balance: Vec<BalancePair>,
    /// The copper of each layer by region of the board; empty (and not written in the JSON)
    /// unless the settings cut the box of the outline into more than one cell.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<RegionStats>,
    /// What the report did on its own that the user may want to know.
    pub findings: Vec<Finding>,
}

impl BoardStats {
    /// True when a finding is an error.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// What is added up for one layer, in square nanometres.
#[derive(Default)]
struct Acc {
    pads: usize,
    tracks: usize,
    track_length_nm: i64,
    vias: usize,
    zones: usize,
    pads_nm2: i128,
    tracks_nm2: i128,
    vias_nm2: i128,
    zones_nm2: i128,
}

fn per_cm2_x100(count: usize, area_um2: i64) -> i64 {
    if area_um2 <= 0 {
        return 0;
    }
    // 1 cm2 is 100 000 000 um2
    let value = (count as i128) * 100 * 100_000_000 / i128::from(area_um2);
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn drill_entry(drills: &mut BTreeMap<i64, DrillStats>, diameter: i64) -> &mut DrillStats {
    drills.entry(diameter).or_insert(DrillStats {
        diameter_nm: diameter,
        vias: 0,
        plated_pads: 0,
        unplated_pads: 0,
        total: 0,
    })
}

fn board_area(board: &Board) -> (BoardArea, i128) {
    let outline = polygon_area_nm2(&board.outline);
    let cutouts: i128 = board
        .outline_cutouts
        .iter()
        .map(|c| polygon_area_nm2(c))
        .sum();
    let area = (outline - cutouts).max(0);
    let xs = board.outline.iter().map(|p| p.x_nm.get());
    let ys = board.outline.iter().map(|p| p.y_nm.get());
    let width = xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0);
    let height = ys.clone().max().unwrap_or(0) - ys.min().unwrap_or(0);
    (
        BoardArea {
            width_nm: width,
            height_nm: height,
            outline_um2: to_um2(outline),
            cutouts_um2: to_um2(cutouts),
            area_um2: to_um2(area),
        },
        area,
    )
}

/// The copper layers in the order of the stack; without a stack, in the order of their ids.
fn copper_layers(board: &Board) -> Vec<Id> {
    if board.copper_stack.is_empty() {
        board
            .layers
            .iter()
            .filter(|(_, l)| l.kind == LayerKind::Copper)
            .map(|(id, _)| *id)
            .collect()
    } else {
        board.copper_stack.clone()
    }
}

/// The corners of a polygon as plain pairs of nanometres.
fn xy(points: &[Point]) -> Vec<(i64, i64)> {
    points
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect()
}

/// The grid that the settings cut the box of the outline into, when it has more than one cell
/// and the board has an area.
fn region_grid(board: &Board, settings: &StatsSettings, area_nm2: i128) -> Option<Grid> {
    if settings.region_columns * settings.region_rows <= 1 || area_nm2 <= 0 {
        return None;
    }
    let xs = board.outline.iter().map(|p| p.x_nm.get());
    let ys = board.outline.iter().map(|p| p.y_nm.get());
    let min = (xs.clone().min()?, ys.clone().min()?);
    let max = (xs.max()?, ys.max()?);
    Grid::new(min, max, settings.region_columns, settings.region_rows)
}

/// The statistics of a board.
pub fn compute(board: &Board, settings: &StatsSettings) -> Result<BoardStats, StatsError> {
    let items = board.tracks.len()
        + board.vias.len()
        + board.zones.len()
        + board.components.len()
        + board.pad_nets.len()
        + board.silkscreen.len();
    if items > settings.max_items {
        return Err(StatsError::new(
            LIMIT_EXCEEDED,
            format!("{items} objects, the limit is {}", settings.max_items),
        ));
    }
    let (area, area_nm2) = board_area(board);
    let order = copper_layers(board);
    let index: BTreeMap<Id, usize> = order.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut acc: Vec<Acc> = order.iter().map(|_| Acc::default()).collect();
    // the copper of each layer in each cell of the grid of regions, in square nanometres; the
    // cells of a layer are empty when there is no grid
    let grid = region_grid(board, settings, area_nm2);
    let cell_count = grid.as_ref().map_or(0, Grid::cells);
    let mut region_acc: Vec<Vec<f64>> = order.iter().map(|_| vec![0.0; cell_count]).collect();
    let mut drills: BTreeMap<i64, DrillStats> = BTreeMap::new();
    let mut counts = Counts {
        components: board.components.len(),
        components_top: 0,
        components_bottom: 0,
        nets: board.nets.len(),
        pads: 0,
        smd_pads: 0,
        through_pads: 0,
        unplated_holes: 0,
        tracks: board.tracks.len(),
        track_segments: 0,
        track_length_nm: 0,
        vias: board.vias.len(),
        zones: board.zones.len(),
        silkscreen_items: board.silkscreen.len(),
    };

    for (component_id, component) in &board.components {
        match component.side {
            Side::Top => counts.components_top += 1,
            Side::Bottom => counts.components_bottom += 1,
        }
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            counts.pads += 1;
            match (pad.drill, pad.plated) {
                (None, _) => counts.smd_pads += 1,
                (Some(d), true) => {
                    counts.through_pads += 1;
                    let entry = drill_entry(&mut drills, d.get());
                    entry.plated_pads += 1;
                    entry.total += 1;
                }
                (Some(d), false) => {
                    counts.unplated_holes += 1;
                    let entry = drill_entry(&mut drills, d.get());
                    entry.unplated_pads += 1;
                    entry.total += 1;
                }
            }
            if pad.is_npth() {
                continue;
            }
            let copper = pad_copper_nm2(pad);
            // a pad is wholly in the cell that holds its centre
            let cell = grid.as_ref().and_then(|g| {
                let at = pad_position(
                    board,
                    PadRef {
                        component: *component_id,
                        pad: *pad_id,
                    },
                )?;
                Some(g.cell_of(at.x_nm.get(), at.y_nm.get()))
            });
            for layer in &pad.layers {
                if let Some(&i) = index.get(layer) {
                    acc[i].pads += 1;
                    acc[i].pads_nm2 += copper;
                    if let Some(c) = cell {
                        region_acc[i][c] += copper as f64;
                    }
                }
            }
        }
    }

    for track in board.tracks.values() {
        counts.track_segments += track.path.len();
        let length = track
            .path
            .iter()
            .fold(0_i64, |sum, s| sum.saturating_add(segment_length_nm(*s)));
        counts.track_length_nm = counts.track_length_nm.saturating_add(length);
        if let Some(&i) = index.get(&track.layer) {
            let width = track.width.get();
            let body = i128::from(length) * i128::from(width);
            let ends = whole(circle_nm2(width));
            acc[i].tracks += 1;
            acc[i].track_length_nm = acc[i].track_length_nm.saturating_add(length);
            acc[i].tracks_nm2 += body + ends;
            if let Some(g) = &grid {
                // each piece is shared between the cells in proportion to its length in each,
                // and the round ends are put in the cells of the two ends of the track
                for segment in &track.path {
                    let (a, b) = (segment.start(), segment.end());
                    let area = segment_length_nm(*segment) as f64 * width as f64;
                    let shares = g
                        .segment_shares((a.x_nm.get(), a.y_nm.get()), (b.x_nm.get(), b.y_nm.get()));
                    for (cell, share) in shares {
                        region_acc[i][cell] += area * share;
                    }
                }
                if let (Some(first), Some(last)) = (track.path.first(), track.path.last()) {
                    let half = ends as f64 / 2.0;
                    let (start, end) = (first.start(), last.end());
                    region_acc[i][g.cell_of(start.x_nm.get(), start.y_nm.get())] += half;
                    region_acc[i][g.cell_of(end.x_nm.get(), end.y_nm.get())] += half;
                }
            }
        }
    }

    for via in board.vias.values() {
        let entry = drill_entry(&mut drills, via.drill.get());
        entry.vias += 1;
        entry.total += 1;
        let ring = whole(circle_nm2(via.diameter.get()) - circle_nm2(via.drill.get()));
        let from = index.get(&via.from_layer).copied();
        let to = index.get(&via.to_layer).copied();
        let crossed: Vec<usize> = match (from, to) {
            (Some(a), Some(b)) if !board.copper_stack.is_empty() => (a.min(b)..=a.max(b)).collect(),
            (Some(a), Some(b)) if a == b => vec![a],
            (Some(a), Some(b)) => vec![a, b],
            (Some(a), None) | (None, Some(a)) => vec![a],
            (None, None) => Vec::new(),
        };
        let via_cell = grid
            .as_ref()
            .map(|g| g.cell_of(via.at.x_nm.get(), via.at.y_nm.get()));
        for i in crossed {
            acc[i].vias += 1;
            acc[i].vias_nm2 += ring;
            if let Some(c) = via_cell {
                region_acc[i][c] += ring as f64;
            }
        }
    }

    for zone in board.zones.values() {
        let Some(&i) = index.get(&zone.layer) else {
            continue;
        };
        acc[i].zones += 1;
        if settings.count_zones {
            let cutouts: i128 = zone.cutouts.iter().map(|c| polygon_area_nm2(c)).sum();
            acc[i].zones_nm2 += (polygon_area_nm2(&zone.outline) - cutouts).max(0);
            if let Some(g) = &grid {
                // the outline and the cutouts are clipped to each cell
                let mut areas = g.polygon_areas(&xy(&zone.outline));
                for cutout in &zone.cutouts {
                    for (total, cut) in areas.iter_mut().zip(g.polygon_areas(&xy(cutout))) {
                        *total -= cut;
                    }
                }
                for (cell, area) in areas.into_iter().enumerate() {
                    region_acc[i][cell] += area.max(0.0);
                }
            }
        }
    }

    let layers: Vec<LayerStats> = order
        .iter()
        .zip(&acc)
        .enumerate()
        .map(|(i, (id, a))| {
            let name = board
                .layers
                .get(id)
                .map_or_else(|| "?".to_owned(), |l| l.name.clone());
            let sum = a.pads_nm2 + a.tracks_nm2 + a.vias_nm2 + a.zones_nm2;
            let copper = if area_nm2 > 0 { sum.min(area_nm2) } else { sum };
            let copper_um2 = to_um2(copper);
            let coverage = if area.area_um2 > 0 {
                i64::try_from(i128::from(copper_um2) * 10_000 / i128::from(area.area_um2))
                    .unwrap_or(i64::MAX)
            } else {
                0
            };
            LayerStats {
                layer: name,
                position: if board.copper_stack.is_empty() {
                    0
                } else {
                    i + 1
                },
                pads: a.pads,
                tracks: a.tracks,
                track_length_nm: a.track_length_nm,
                vias: a.vias,
                zones: a.zones,
                pads_um2: to_um2(a.pads_nm2),
                tracks_um2: to_um2(a.tracks_nm2),
                vias_um2: to_um2(a.vias_nm2),
                zones_um2: to_um2(a.zones_nm2),
                copper_um2,
                coverage_x100: coverage,
            }
        })
        .collect();

    let mut regions = Vec::new();
    let mut region_findings = Vec::new();
    if let Some(g) = &grid {
        // the area of the board in each cell: the outline less its cutouts, clipped to the cell
        let mut cell_areas = g.polygon_areas(&xy(&board.outline));
        for cutout in &board.outline_cutouts {
            for (total, cut) in cell_areas.iter_mut().zip(g.polygon_areas(&xy(cutout))) {
                *total -= cut;
            }
        }
        let areas: Vec<i128> = cell_areas.iter().map(|a| capped_nm2(*a, 0)).collect();
        let limit = settings.region_max_difference_x100;
        for (layer, sums) in layers.iter().zip(&region_acc) {
            let mut cells = Vec::with_capacity(g.cells());
            let mut judged = Vec::with_capacity(g.cells());
            for (cell, (area, sum)) in areas.iter().zip(sums).enumerate() {
                let copper = capped_nm2(*sum, *area);
                let coverage = coverage_x100(copper, *area);
                judged.push((*area, coverage));
                cells.push(RegionCell {
                    column: cell % g.columns(),
                    row: cell / g.columns(),
                    area_um2: to_um2(*area),
                    copper_um2: to_um2(copper),
                    coverage_x100: coverage,
                });
            }
            if limit > 0 {
                if let Some((low, high)) = spread_x100(&judged) {
                    if high - low > limit {
                        region_findings.push(Finding::new(
                            REGION_UNEVEN,
                            settings.balance_severity,
                            Some(&layer.layer),
                            format!(
                                "coverage from {} % to {} %, difference {} points, limit {}",
                                hundredths(low),
                                hundredths(high),
                                hundredths(high - low),
                                hundredths(limit)
                            ),
                        ));
                    }
                }
            }
            regions.push(RegionStats {
                layer: layer.layer.clone(),
                columns: g.columns(),
                rows: g.rows(),
                cells,
            });
        }
    }

    let holes = counts.through_pads + counts.unplated_holes + counts.vias;
    let density = Density {
        components_per_cm2_x100: per_cm2_x100(counts.components, area.area_um2),
        pads_per_cm2_x100: per_cm2_x100(counts.pads, area.area_um2),
        holes_per_cm2_x100: per_cm2_x100(holes, area.area_um2),
    };

    let mut findings = Vec::new();
    if area.area_um2 == 0 {
        findings.push(Finding::new(
            NO_BOARD_AREA,
            Severity::Warning,
            None,
            "the coverage and the density are zero",
        ));
    }
    if settings.count_zones && counts.zones > 0 {
        findings.push(Finding::new(
            ZONE_OUTLINE_COUNTED,
            Severity::Info,
            None,
            format!("{} zones", counts.zones),
        ));
    }

    let (balance, balance_findings) = judge(&layers, !board.copper_stack.is_empty(), settings);
    findings.extend(balance_findings);
    findings.extend(region_findings);

    Ok(BoardStats {
        format: FORMAT,
        name: board.name.clone(),
        board: area,
        counts,
        layers,
        drills: drills.into_values().collect(),
        density,
        balance,
        regions,
        findings,
    })
}
