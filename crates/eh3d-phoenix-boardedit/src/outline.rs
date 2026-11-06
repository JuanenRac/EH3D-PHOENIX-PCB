// SPDX-License-Identifier: GPL-3.0-or-later
//! The operation `outline_from_dxf`: the outline of the board from the contours of a DXF drawing.
//! The drawing is read and its lines and arcs are joined into closed contours (`eh3d-phoenix-dxf`),
//! the largest outer contour becomes the outline and the cutouts are listed in the notes, since the
//! board holds its outline only. The plan is one reversible operation of the board; the editor
//! tries it on a copy and keeps it only if the design rules add no error, so it is all or nothing
//! and one undo takes it back.

use eh3d_phoenix_board::{Board, Operation};
use eh3d_phoenix_dxf::{extract_outline, read_dxf, DxfSettings, Severity};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::codes::BoardEditError;
use crate::plan::{geometry, single, Plan};

/// The box around an outline, as "W by H mm".
fn size_text(points: &[Point]) -> String {
    let xs = points.iter().map(|p| p.x_nm.get());
    let ys = points.iter().map(|p| p.y_nm.get());
    let width = xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0);
    let height = ys.clone().max().unwrap_or(0) - ys.min().unwrap_or(0);
    format!(
        "{:.3} by {:.3} mm",
        width as f64 / 1_000_000.0,
        height as f64 / 1_000_000.0
    )
}

/// Plans the replacement of the outline by the outer contour of `dxf`.
pub(crate) fn plan_outline(
    board: &Board,
    dxf: &str,
    layer: Option<&str>,
    join_tolerance_nm: Option<i64>,
    settings_text: Option<&str>,
) -> Result<Plan, BoardEditError> {
    let settings = match settings_text {
        Some(text) => DxfSettings::parse(text).map_err(|e| geometry(e.message(Lang::En)))?,
        None => DxfSettings::default(),
    };
    let join = join_tolerance_nm.unwrap_or(settings.join_tolerance_nm);
    if join < 0 {
        return Err(geometry("the tolerance of the join is below 0"));
    }
    let read = read_dxf(dxf, &settings).map_err(|e| geometry(e.message(Lang::En)))?;
    let found = extract_outline(&read.drawing, layer, join, &settings);

    let mut notes: Vec<String> = Vec::new();
    let mut first_error: Option<String> = None;
    for finding in read
        .report
        .findings
        .iter()
        .chain(found.report.findings.iter())
    {
        let line = finding.line(Lang::En);
        if finding.severity == Severity::Error && first_error.is_none() {
            first_error = Some(line.clone());
        }
        notes.push(line);
    }
    let omitted = read.report.omitted + found.report.omitted;
    if omitted > 0 {
        notes.push(format!(
            "{omitted} more findings were counted and not listed"
        ));
    }
    if let Some(error) = first_error {
        return Err(geometry(error));
    }
    let Some(exterior) = found.exterior else {
        return Err(geometry("the drawing has no closed contour"));
    };
    let outline = exterior
        .iter()
        .map(|&(x, y)| Point::from_nm(x, y))
        .collect::<Result<Vec<Point>, _>>()
        .map_err(|e| geometry(e.to_string()))?;
    if outline == board.outline {
        notes.push("the outline is the one the board has".to_owned());
        let mut plan = single(Operation::Batch { ops: Vec::new() });
        plan.notes = notes;
        return Ok(plan);
    }
    notes.push(format!(
        "outline: {} corners, {} (it was {} corners, {})",
        outline.len(),
        size_text(&outline),
        board.outline.len(),
        size_text(&board.outline)
    ));
    let mut plan = single(Operation::SetOutline { outline });
    plan.notes = notes;
    Ok(plan)
}
