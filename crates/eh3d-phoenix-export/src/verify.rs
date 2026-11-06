//! The internal check of an export: it reads the written files back with the parsers of
//! EH3D-CORE-LIBS and compares what it finds with the model of the board, so that a part of the
//! board lost on the way to a file is found by a test and not by the manufacturer.
//!
//! What is compared:
//!
//! * every layer file: the number of flashes (pads, and vias on copper), and on copper also the
//!   number of regions (zones), of lines and of arcs outside the regions (tracks);
//! * the outline file: the number of lines and the bounding box against the outline of the board;
//! * the drill files: the number of holes and slots against the vias and the drilled pads, the
//!   units, and the order of the tools by diameter;
//! * the placement file: one line per component with the reference designators of the board, and
//!   the bill of materials: quantities that add up to the number of components.
//!
//! Silkscreen and documentation layers are only read back (their strokes depend on the font). A
//! file that cannot be read gives `PHOENIX-EXPORT-018`; a difference gives `PHOENIX-EXPORT-014`.

use std::collections::BTreeSet;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_parser_excellon::Units as DrillUnits;
use eh3d_parser_gerber::Operation;
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, Segment};

use crate::assembly::parse_csv;
use crate::codes;
use crate::fab::{FabFile, FabSet, FileRole};
use crate::gerber::{arc_centre, via_on_layer};
use crate::profile::Profile;

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

/// The number of pads of the board that appear on `layer`.
fn pads_on(board: &Board, layer: Id) -> usize {
    let mut count = 0;
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
            if pad.layers.contains(&layer) && at.is_some() {
                count += 1;
            }
        }
    }
    count
}

/// The number of drilled pads, plated and not plated.
fn drilled_pads(board: &Board) -> (usize, usize) {
    let (mut plated, mut not_plated) = (0, 0);
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
            if pad.drill.is_none() || at.is_none() {
                continue;
            }
            let on_copper = pad.layers.iter().any(|l| {
                board
                    .layers
                    .get(l)
                    .is_some_and(|layer| layer.kind == LayerKind::Copper)
            });
            if on_copper {
                plated += 1;
            } else {
                not_plated += 1;
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
    let (mut flashes, mut lines, mut arcs, mut regions) = (0usize, 0usize, 0usize, 0usize);
    let mut in_region = false;
    let mut bbox: Option<(f64, f64, f64, f64)> = None;
    for command in &read.commands {
        match &command.op {
            Operation::Flash { .. } => flashes += 1,
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
    match kind {
        LayerKind::Copper => {
            let vias = board
                .vias
                .values()
                .filter(|v| via_on_layer(board, v, layer))
                .count();
            compare(out, file, "flashes", pads_on(board, layer) + vias, flashes);
            let zones = board
                .zones
                .values()
                .filter(|z| z.layer == layer && z.outline.len() >= 3)
                .count();
            compare(out, file, "regions", zones, regions);
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
            compare(out, file, "flashes", pads_on(board, layer), flashes);
        }
        LayerKind::Outline => {
            if board.outline.len() >= 3 {
                compare(out, file, "lines", board.outline.len(), lines);
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

fn check_drill(board: &Board, file: &FabFile, plated: bool, out: &mut Vec<Diagnostic>) {
    let read = eh3d_parser_excellon::parse(file.text());
    if !read.is_clean() {
        unreadable(out, file, "the Excellon file has errors");
        return;
    }
    let (plated_pads, not_plated_pads) = drilled_pads(board);
    let model = if plated {
        board.vias.len() + plated_pads
    } else {
        not_plated_pads
    };
    compare(
        out,
        file,
        "holes",
        model,
        read.drills.len() + read.slots.len(),
    );
    if read.units != Some(DrillUnits::Millimetres) {
        out.push(codes::diagnostic(
            codes::VERIFY_MISMATCH,
            Severity::Error,
            &format!("{}: the units are not declared as millimetres", file.path),
        ));
    }
    let diameters: Vec<f64> = read.tools.values().map(|t| t.diameter).collect();
    if diameters.windows(2).any(|w| w[0] > w[1]) {
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
            FileRole::Drill { plated } => check_drill(board, file, *plated, &mut out),
            FileRole::Placement => check_placement(board, profile, file, &mut out),
            FileRole::Bom => check_bom(board, profile, file, &mut out),
        }
    }
    let (plated, not_plated) = drilled_pads(board);
    for (plated_file, model) in [(true, board.vias.len() + plated), (false, not_plated)] {
        let present = set.files.iter().any(|f| {
            f.role
                == FileRole::Drill {
                    plated: plated_file,
                }
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
    out
}
