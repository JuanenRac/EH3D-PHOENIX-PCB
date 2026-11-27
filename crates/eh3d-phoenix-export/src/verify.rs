//! The internal check of an export: it reads the written files back with the parsers of
//! EH3D-CORE-LIBS and compares what it finds with the model of the board, so that a part of the
//! board lost on the way to a file is found by a test and not by the manufacturer.
//!
//! What is compared:
//!
//! * every layer file: the number of flashes (pads, and vias on copper) and where each one is,
//!   and on copper also the number of regions (zones), of lines and of arcs outside the regions
//!   (tracks);
//! * the outline file: the number of lines and the bounding box against the outline of the board;
//! * the drill files: the number of holes and slots against the vias and the drilled pads, the
//!   place and the diameter of each (the middle of a slot), the units, and the order of the tools
//!   by diameter;
//! * the placement file: one line per component with the reference designators of the board, and
//!   the bill of materials: quantities that add up to the number of components.
//!
//! Silkscreen and documentation layers are only read back (their strokes depend on the font). A
//! file that cannot be read gives `PHOENIX-EXPORT-018`; a difference gives `PHOENIX-EXPORT-014`.

use std::collections::BTreeSet;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_parser_excellon::Units as DrillUnits;
use eh3d_parser_gerber::{Operation, Polarity};
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, Segment};

use crate::assembly::parse_csv;
use crate::codes;
use crate::excellon::{via_span, ViaSpan};
use crate::fab::{FabFile, FabSet, FileRole};
use crate::gerber::{arc_centre, pad_is_region, via_on_layer};
use crate::profile::{DrillStyle, Profile};

fn unreadable(out: &mut Vec<Diagnostic>, file: &FabFile, detail: &str) {
    out.push(codes::diagnostic(
        codes::VERIFY_UNREADABLE,
        Severity::Error,
        &format!("{}: {detail}", file.path),
    ));
}

fn compare(out: &mut Vec<Diagnostic>, file: &FabFile, what: &str, model: usize, found: usize) {
    if model != found {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!(
                "{}: {what}: the model has {model}, the file has {found}",
                file.path
            ),
        ));
    }
}

/// A thing of a file as `[diameter, x, y]` in millimetres (the diameter 0 for a flash).
type Spot = [f64; 3];

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

/// How many of `found` have no partner of their own in `model` within `tolerance`.
fn unmatched(mut model: Vec<Spot>, found: &[Spot], tolerance: f64) -> usize {
    model.sort_by(|a, b| a[1].total_cmp(&b[1]));
    let mut used = vec![false; model.len()];
    found
        .iter()
        .filter(|spot| {
            let first = model.partition_point(|m| m[1] < spot[1] - tolerance);
            let partner = (first..model.len())
                .take_while(|&i| model[i][1] <= spot[1] + tolerance)
                .find(|&i| !used[i] && (0..3).all(|k| (model[i][k] - spot[k]).abs() <= tolerance));
            match partner {
                Some(i) => {
                    used[i] = true;
                    false
                }
                None => true,
            }
        })
        .count()
}

/// Reports the things of a file that are not where the model has them. A different number of them
/// is reported by [`compare`] already.
fn compare_places(
    out: &mut Vec<Diagnostic>,
    file: &FabFile,
    what: &str,
    model: Vec<Spot>,
    found: &[Spot],
    tolerance: f64,
) {
    if model.len() != found.len() {
        return;
    }
    let lost = unmatched(model, found, tolerance);
    if lost > 0 {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!(
                "{}: {lost} {what} are not where the model has them",
                file.path
            ),
        ));
    }
}

/// The pads of the board that appear on `layer`, as where those written as flashes are and the
/// number written as regions (pads with rounded corners). A hole that is not plated has no copper.
fn pads_on(board: &Board, layer: Id) -> (Vec<Spot>, usize) {
    // a hole that is not plated has no copper and takes no paste
    let copper = board
        .layers
        .get(&layer)
        .is_some_and(|l| matches!(l.kind, LayerKind::Copper | LayerKind::Paste));
    let (mut flashes, mut regions) = (Vec::new(), 0);
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let at = pad_position(
                board,
                PadRef {
                    component: *cid,
                    pad: *pid,
                },
            );
            let Some(at) = at else { continue };
            if pad.layers.contains(&layer) && !(copper && pad.is_npth()) {
                if pad_is_region(pad, component) {
                    regions += 1;
                } else {
                    flashes.push([0.0, mm(at.x_nm.get()), mm(at.y_nm.get())]);
                }
            }
        }
    }
    (flashes, regions)
}

/// The drilled pads, plated and not plated.
fn drilled_pads(board: &Board) -> (Vec<Spot>, Vec<Spot>) {
    let (mut plated, mut not_plated) = (Vec::new(), Vec::new());
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let at = pad_position(
                board,
                PadRef {
                    component: *cid,
                    pad: *pid,
                },
            );
            let (Some(drill), Some(at)) = (pad.drill, at) else {
                continue;
            };
            let hole = [mm(drill.get()), mm(at.x_nm.get()), mm(at.y_nm.get())];
            let on_copper = pad.layers.iter().any(|l| {
                board
                    .layers
                    .get(l)
                    .is_some_and(|layer| layer.kind == LayerKind::Copper)
            });
            if on_copper && pad.plated {
                plated.push(hole);
            } else {
                not_plated.push(hole);
            }
        }
    }
    (plated, not_plated)
}

fn check_layer(
    board: &Board,
    profile: &Profile,
    file: &FabFile,
    layer: Id,
    kind: LayerKind,
    out: &mut Vec<Diagnostic>,
) {
    let read = eh3d_parser_gerber::parse(file.text());
    if !read.is_clean() {
        unreadable(out, file, "the Gerber file has errors");
        return;
    }
    let (mut lines, mut arcs, mut regions) = (0usize, 0usize, 0usize);
    let mut flashes: Vec<Spot> = Vec::new();
    let mut in_region = false;
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    // the objects of clear polarity take copper away and are not counted
    let mut clear = false;
    for command in &read.commands {
        if let Operation::Polarity(p) = &command.op {
            clear = matches!(p, Polarity::Clear);
            continue;
        }
        if clear {
            continue;
        }
        match &command.op {
            Operation::Flash { at } => flashes.push([0.0, at.x, at.y]),
            Operation::Line { from, to } => {
                if !in_region {
                    lines += 1;
                }
                for p in [from, to] {
                    bbox = Some(match bbox {
                        None => (p.x, p.y, p.x, p.y),
                        Some((x0, y0, x1, y1)) => {
                            (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y))
                        }
                    });
                }
            }
            Operation::Arc { .. } => {
                if !in_region {
                    arcs += 1;
                }
            }
            Operation::RegionStart => {
                regions += 1;
                in_region = true;
            }
            Operation::RegionEnd => in_region = false,
            _ => {}
        }
    }
    // the last digit the file keeps
    let step = 10f64.powi(-(profile.gerber.decimal_digits as i32)) + 1e-9;
    match kind {
        LayerKind::Copper => {
            let (mut model, pad_regions) = pads_on(board, layer);
            model.extend(
                board
                    .vias
                    .values()
                    .filter(|v| via_on_layer(board, v, layer))
                    .map(|v| [0.0, mm(v.at.x_nm.get()), mm(v.at.y_nm.get())]),
            );
            compare(out, file, "flashes", model.len(), flashes.len());
            compare_places(out, file, "flashes", model, &flashes, step);
            let small = crate::islands::dropped_zones(
                board,
                layer,
                profile.gerber.zone_island_min_area_nm2,
            );
            let zones = board
                .zones
                .iter()
                .filter(|(id, z)| {
                    z.layer == layer && z.outline.len() >= 3 && !small.zones.contains(*id)
                })
                .count();
            compare(out, file, "regions", zones + pad_regions, regions);
            let (mut model_lines, mut model_arcs) = (0usize, 0usize);
            for track in board.tracks.values().filter(|t| t.layer == layer) {
                for segment in &track.path {
                    match segment {
                        Segment::Line { .. } => model_lines += 1,
                        Segment::Arc { start, end, sweep } => {
                            if arc_centre(*start, *end, sweep.get()).is_some() {
                                model_arcs += 1;
                            }
                        }
                    }
                }
            }
            compare(out, file, "lines", model_lines, lines);
            compare(out, file, "arcs", model_arcs, arcs);
        }
        LayerKind::SolderMask | LayerKind::Paste => {
            let (model, pad_regions) = pads_on(board, layer);
            compare(out, file, "flashes", model.len(), flashes.len());
            compare_places(out, file, "flashes", model, &flashes, step);
            compare(out, file, "regions", pad_regions, regions);
        }
        LayerKind::Outline => {
            if board.outline.len() >= 3 {
                let cutout_lines: usize = board
                    .outline_cutouts
                    .iter()
                    .filter(|c| c.len() >= 3)
                    .map(Vec::len)
                    .sum();
                // an edge of the outline that is an arc is written as an arc and not as a line
                let arc_edges = crate::gerber::outline_arc_count(board);
                compare(
                    out,
                    file,
                    "lines",
                    board.outline.len() - arc_edges + cutout_lines,
                    lines,
                );
                compare(out, file, "arcs", arc_edges, arcs);
                let xs = board.outline.iter().map(|p| p.x_nm.get());
                let ys = board.outline.iter().map(|p| p.y_nm.get());
                let to_mm = |v: Option<i64>| v.unwrap_or(0) as f64 / 1_000_000.0;
                let model = (
                    to_mm(xs.clone().min()),
                    to_mm(ys.clone().min()),
                    to_mm(xs.max()),
                    to_mm(ys.max()),
                );
                // half of the last digit the file keeps, and a little for the float
                let tolerance = 0.5 * 10f64.powi(-(profile.gerber.decimal_digits as i32)) + 1e-9;
                let same = bbox.is_some_and(|(x0, y0, x1, y1)| {
                    (x0 - model.0).abs() <= tolerance
                        && (y0 - model.1).abs() <= tolerance
                        && (x1 - model.2).abs() <= tolerance
                        && (y1 - model.3).abs() <= tolerance
                });
                if !same {
                    out.push(codes::diagnostic(
                        codes::VERIFY_MISMATCH,
                        Severity::Error,
                        &format!("{}: the bounding box of the outline", file.path),
                    ));
                }
            }
        }
        LayerKind::Silkscreen | LayerKind::Documentation => {}
    }
}

/// The holes of the vias of the board whose span is `span`.
fn vias_of_span(board: &Board, span: ViaSpan) -> Vec<Spot> {
    board
        .vias
        .values()
        .filter(|v| via_span(board, v) == span)
        .map(|v| [mm(v.drill.get()), mm(v.at.x_nm.get()), mm(v.at.y_nm.get())])
        .collect()
}

/// The holes the model has for a drill file of `plated` holes (the through vias and the
/// plated pads) or of holes that are not plated.
fn model_holes(board: &Board, plated: bool) -> Vec<Spot> {
    let (plated_pads, not_plated_pads) = drilled_pads(board);
    if plated {
        let mut holes = vias_of_span(board, ViaSpan::Through);
        holes.extend(plated_pads);
        holes
    } else {
        not_plated_pads
    }
}

/// Compares a drill file with the model. `by_diameter` is true when the tools must be listed from
/// the smallest diameter (not so in a mixed file, which lists those of the plated holes first).
fn check_drill(
    profile: &Profile,
    file: &FabFile,
    model: Vec<Spot>,
    by_diameter: bool,
    out: &mut Vec<Diagnostic>,
) {
    let read = eh3d_parser_excellon::parse(file.text());
    if !read.is_clean() {
        unreadable(out, file, "the Excellon file has errors");
        return;
    }
    let diameter = |tool: u32| read.tools.get(&tool).map_or(0.0, |t| t.diameter);
    // a slot is where its middle is
    let found: Vec<Spot> = read
        .drills
        .iter()
        .map(|h| [diameter(h.tool), h.at.x, h.at.y])
        .chain(read.slots.iter().map(|s| {
            [
                diameter(s.tool),
                (s.from.x + s.to.x) / 2.0,
                (s.from.y + s.to.y) / 2.0,
            ]
        }))
        .collect();
    compare(out, file, "holes", model.len(), found.len());
    let digits = match profile.drill.style {
        DrillStyle::Decimal => profile.drill.decimals,
        DrillStyle::Fixed => profile.drill.fixed_decimal_digits,
    };
    let step = 10f64.powi(-(digits as i32)) + 1e-9;
    compare_places(out, file, "holes", model, &found, step);
    if read.units != Some(DrillUnits::Millimetres) {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!("{}: the units are not declared as millimetres", file.path),
        ));
    }
    let diameters: Vec<f64> = read.tools.values().map(|t| t.diameter).collect();
    if by_diameter && diameters.windows(2).any(|w| w[0] > w[1]) {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!(
                "{}: the tools are not in the order of their diameters",
                file.path
            ),
        ));
    }
}

fn column(columns: &[crate::profile::Column], key: &str) -> Option<usize> {
    columns.iter().position(|c| c.key == key)
}

fn check_placement(board: &Board, profile: &Profile, file: &FabFile, out: &mut Vec<Diagnostic>) {
    let p = &profile.placement;
    let rows = parse_csv(file.text(), p.delimiter);
    let data = rows.get(1..).unwrap_or(&[]);
    compare(out, file, "lines", board.components.len(), data.len());
    if rows.iter().any(|r| r.len() != p.columns.len()) {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!(
                "{}: a line has another number of fields than the header",
                file.path
            ),
        ));
        return;
    }
    if let Some(index) = column(&p.columns, "ref") {
        let model: BTreeSet<&str> = board
            .components
            .values()
            .map(|c| c.refdes.as_str())
            .collect();
        let found: BTreeSet<&str> = data.iter().map(|r| r[index].as_str()).collect();
        if model != found {
            out.push(codes::diagnostic(
                codes::VERIFY_MISMATCH,
                Severity::Error,
                &format!("{}: the reference designators", file.path),
            ));
        }
    }
}

fn check_bom(board: &Board, profile: &Profile, file: &FabFile, out: &mut Vec<Diagnostic>) {
    let b = &profile.bom;
    let rows = parse_csv(file.text(), b.delimiter);
    if rows.iter().any(|r| r.len() != b.columns.len()) {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!(
                "{}: a line has another number of fields than the header",
                file.path
            ),
        ));
        return;
    }
    if let Some(index) = column(&b.columns, "quantity") {
        let total: usize = rows
            .iter()
            .skip(1)
            .map(|r| r[index].parse::<usize>().unwrap_or(0))
            .sum();
        compare(out, file, "components", board.components.len(), total);
    }
}

/// Reads the files of `set` back and compares them with `board`. The list is empty when the files
/// agree with the model.
pub fn verify(board: &Board, profile: &Profile, set: &FabSet) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for file in &set.files {
        match &file.role {
            FileRole::Layer { layer, kind, .. } => {
                check_layer(board, profile, file, *layer, *kind, &mut out)
            }
            FileRole::Drill { plated } => {
                check_drill(profile, file, model_holes(board, *plated), true, &mut out)
            }
            FileRole::DrillMixed => {
                let mut model = model_holes(board, true);
                model.extend(model_holes(board, false));
                check_drill(profile, file, model, false, &mut out)
            }
            FileRole::DrillSpan { from, to, buried } => {
                let span = if *buried {
                    ViaSpan::Buried(*from, *to)
                } else {
                    ViaSpan::Blind(*from, *to)
                };
                check_drill(profile, file, vias_of_span(board, span), true, &mut out)
            }
            FileRole::Placement => check_placement(board, profile, file, &mut out),
            FileRole::Bom => check_bom(board, profile, file, &mut out),
        }
    }
    for (plated_file, model) in [
        (true, model_holes(board, true).len()),
        (false, model_holes(board, false).len()),
    ] {
        let present = set.files.iter().any(|f| {
            f.role
                == FileRole::Drill {
                    plated: plated_file,
                }
                || f.role == FileRole::DrillMixed
        });
        if model > 0 && !present {
            out.push(codes::diagnostic(
                codes::VERIFY_MISMATCH,
                Severity::Error,
                &format!(
                    "the model has {model} {} hole(s) and there is no file for them",
                    if plated_file { "plated" } else { "not plated" }
                ),
            ));
        }
    }
    // every span of blind or buried vias has a file
    let mut spans: Vec<ViaSpan> = board
        .vias
        .values()
        .map(|v| via_span(board, v))
        .filter(|s| *s != ViaSpan::Through)
        .collect();
    spans.sort_unstable();
    spans.dedup();
    for span in spans {
        let present = set.files.iter().any(|f| match f.role {
            FileRole::DrillSpan { from, to, buried } => {
                span == if buried {
                    ViaSpan::Buried(from, to)
                } else {
                    ViaSpan::Blind(from, to)
                }
            }
            _ => false,
        });
        if !present {
            out.push(codes::diagnostic(
                codes::VERIFY_MISMATCH,
                Severity::Error,
                &format!("the model has vias of {span:?} and there is no file for them"),
            ));
        }
    }
    out
}
