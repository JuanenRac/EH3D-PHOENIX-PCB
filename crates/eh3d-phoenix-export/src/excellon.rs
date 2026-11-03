//! The holes of the board as one Excellon drill file.
//!
//! Vias and drilled pads give round holes. Tools are numbered from 1 in the order of their
//! diameters; the holes are ordered by tool, then by Y, then by X, so the same board always gives
//! the same bytes. The model does not say which holes are plated, so one file holds them all.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_parser_excellon::{ExcellonFile, Hit, Point as EPoint, Tool, Units};
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, PadRef, PadShape};
use eh3d_writer_excellon::{ExcellonOptions, WriteError};

use crate::codes;

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

/// Writes the drill file. `None` when the board has no hole.
pub fn write_drills(board: &Board) -> Result<Option<(String, Vec<Diagnostic>)>, WriteError> {
    let mut diagnostics = Vec::new();
    // (diameter, y, x) in nanometres
    let mut holes: Vec<(i64, i64, i64)> = Vec::new();
    for v in board.vias.values() {
        holes.push((v.drill.get(), v.at.y_nm.get(), v.at.x_nm.get()));
    }
    let mut non_round = 0usize;
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let Some(drill) = pad.drill else {
                continue;
            };
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *cid,
                    pad: *pid,
                },
            ) else {
                continue;
            };
            if !matches!(pad.shape, PadShape::Circle { .. }) {
                non_round += 1;
            }
            holes.push((drill.get(), at.y_nm.get(), at.x_nm.get()));
        }
    }
    if non_round > 0 {
        diagnostics.push(codes::diagnostic(
            codes::SLOT_AS_HOLE,
            Severity::Info,
            &format!("{non_round} pad(s)"),
        ));
    }
    if holes.is_empty() {
        return Ok(None);
    }
    let mut diameters: Vec<i64> = holes.iter().map(|h| h.0).collect();
    diameters.sort_unstable();
    diameters.dedup();
    let numbers: BTreeMap<i64, u32> = diameters
        .iter()
        .enumerate()
        .map(|(i, d)| (*d, 1 + i as u32))
        .collect();
    holes.sort_unstable();
    let file = ExcellonFile {
        units: Some(Units::Millimetres),
        zeros: None,
        tools: numbers
            .iter()
            .map(|(d, n)| {
                (
                    *n,
                    Tool {
                        number: *n,
                        diameter: mm(*d),
                        feed: None,
                        speed: None,
                    },
                )
            })
            .collect(),
        drills: holes
            .iter()
            .map(|(d, y, x)| Hit {
                line: 0,
                tool: numbers[d],
                at: EPoint {
                    x: mm(*x),
                    y: mm(*y),
                },
            })
            .collect(),
        slots: Vec::new(),
        routes: Vec::new(),
        ended: true,
        diagnostics: Vec::new(),
    };
    let text = eh3d_writer_excellon::write(&file, &ExcellonOptions::default())?;
    Ok(Some((text, diagnostics)))
}
