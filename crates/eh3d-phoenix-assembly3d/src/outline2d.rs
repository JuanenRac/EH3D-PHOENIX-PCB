// SPDX-License-Identifier: GPL-3.0-or-later
//! The artifact `outline2d` (version 1) that this program hands to the programs of enclosures in
//! the branch `pcb/` of the `.eh3d` project: the outline of the board, its thickness, the unplated
//! holes, the room each part takes with its height and the openings that the connectors need.
//! Every length is a whole number of nanometres in the frame of the board, `y` upwards as seen
//! from the component side.
//!
//! Nothing is guessed. The parts and the connectors come from the table of models, and a part
//! whose footprint is not in it is left out; the thickness is the one of the settings of the
//! assembly, because the board model holds no stack-up. A connector is a part of the top face
//! whose reference starts with one of the prefixes of the settings and whose box comes within the
//! distance of the settings of a side of the rectangle that holds the board.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{ArcSettings, Board, PadRef};
use serde_json::{json, Value};

use crate::codes::{AssemblyError, SETTINGS_INVALID};
use crate::models::ModelTable;
use crate::scene::{build_scene, Part3d};
use crate::settings::AssemblySettings;
use crate::step::StepSolid;

/// The name of the file of the artifact in the branch `pcb/`.
pub const OUTLINE_FILE: &str = "outline2d.json";
/// The most parts the artifact holds.
const MAX_PARTS: usize = 10_000;
/// The most connectors the artifact holds.
const MAX_CONNECTORS: usize = 1_000;
/// The longest name, in characters.
const MAX_NAME_CHARS: usize = 64;

fn name(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect()
}

/// The unplated holes, in the order of the components and their pads.
fn holes(board: &Board) -> Vec<Value> {
    let mut out = Vec::new();
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        let unplated: Vec<_> = footprint
            .pads
            .iter()
            .filter(|(_, pad)| pad.is_npth())
            .collect();
        let only = unplated.len() == 1;
        for (pid, pad) in unplated {
            let (Some(at), Some(drill)) = (
                pad_position(
                    board,
                    PadRef {
                        component: *cid,
                        pad: *pid,
                    },
                ),
                pad.drill,
            ) else {
                continue;
            };
            let label = if only {
                component.refdes.clone()
            } else {
                format!("{}.{}", component.refdes, pad.number)
            };
            let mut hole = json!({
                "name": name(&label),
                "x": at.x_nm.get(),
                "y": at.y_nm.get(),
                "diameter": drill.get(),
            });
            // the identity of the part follows its one hole when it moves or is renamed
            if only {
                hole["id"] = Value::from(cid.to_string());
            }
            out.push(hole);
        }
    }
    out
}

/// The side of the rectangle of the board that a box comes nearest to, within `reach`.
fn side_of(
    bounds: ((i64, i64), (i64, i64)),
    part: ((i64, i64), (i64, i64)),
    reach: i64,
) -> Option<&'static str> {
    let ((x0, y0), (x1, y1)) = bounds;
    let ((px0, py0), (px1, py1)) = part;
    // the distance of the box to each side; a box that hangs over it is at no distance
    [
        ("front", py0 - y0),
        ("back", y1 - py1),
        ("left", px0 - x0),
        ("right", x1 - px1),
    ]
    .into_iter()
    .map(|(side, d)| (side, d.max(0)))
    .filter(|(_, d)| *d <= reach)
    .min_by_key(|(_, d)| *d)
    .map(|(side, _)| side)
}

/// True when the reference is one of a connector: a prefix of the list followed by a digit.
fn is_connector(reference: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|prefix| {
        reference
            .strip_prefix(prefix.as_str())
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    })
}

/// The artifact of a board, as canonical JSON; `None` when the board has no outline of three
/// corners or more. `resolve` reads a STEP file by the name written in the table.
pub fn outline2d(
    board: &Board,
    table: &ModelTable,
    settings: &AssemblySettings,
    resolve: &mut dyn FnMut(&str) -> Result<StepSolid, AssemblyError>,
) -> Result<Option<Vec<u8>>, AssemblyError> {
    let polygon = board.outline_polygon(&ArcSettings::default());
    if polygon.len() < 3 {
        return Ok(None);
    }
    let xs = polygon.iter().map(|p| p.x_nm.get());
    let ys = polygon.iter().map(|p| p.y_nm.get());
    let bounds = (
        (xs.clone().min().unwrap_or(0), ys.clone().min().unwrap_or(0)),
        (xs.max().unwrap_or(0), ys.max().unwrap_or(0)),
    );
    let scene = build_scene(board, None, table, settings, resolve)?;
    let thickness = scene.thickness_nm;
    // how far a part reaches from its own face
    let height = |p: &Part3d| {
        if p.bottom {
            -p.z_min
        } else {
            p.z_max - thickness
        }
    };
    let mut parts = Vec::new();
    let mut connectors = Vec::new();
    let mut tallest = None;
    for part in &scene.parts {
        let ((x0, y0), (x1, y1)) = part.plane_bounds();
        let high = height(part);
        if high <= 0 || x1 <= x0 || y1 <= y0 {
            continue;
        }
        if !part.bottom {
            tallest = tallest.max(Some(high));
        }
        if parts.len() < MAX_PARTS {
            parts.push(json!({
                "name": name(&part.reference),
                "x": x0,
                "y": y0,
                "width": x1 - x0,
                "depth": y1 - y0,
                "height": high,
                "side": if part.bottom { "bottom" } else { "top" },
            }));
        }
        if part.bottom
            || connectors.len() >= MAX_CONNECTORS
            || !is_connector(&part.reference, &settings.connector_prefixes)
        {
            continue;
        }
        let Some(side) = side_of(bounds, ((x0, y0), (x1, y1)), settings.connector_edge_nm) else {
            continue;
        };
        let (at, width) = if side == "front" || side == "back" {
            (x0 + (x1 - x0) / 2, x1 - x0)
        } else {
            (y0 + (y1 - y0) / 2, y1 - y0)
        };
        let above = (part.z_min - thickness).max(0);
        if part.z_max - thickness > above {
            connectors.push(json!({
                "name": name(&part.reference),
                "edge": side,
                "at": at,
                "width": width,
                "height": part.z_max - thickness - above,
                "above": above,
            }));
        }
    }
    // the keys of every object are written in order and every number is a whole one, so the same
    // board gives the same bytes
    let mut root: BTreeMap<&str, Value> = BTreeMap::new();
    root.insert("artifact", Value::from("outline2d"));
    root.insert("version", Value::from(1));
    root.insert("units", Value::from("nm"));
    root.insert(
        "outline",
        polygon
            .iter()
            .map(|p| json!([p.x_nm.get(), p.y_nm.get()]))
            .collect(),
    );
    root.insert("thickness", Value::from(thickness));
    if let Some(tallest) = tallest {
        root.insert("tallest", Value::from(tallest));
    }
    root.insert("holes", Value::from(holes(board)));
    root.insert("parts", Value::from(parts));
    root.insert("connectors", Value::from(connectors));
    let mut bytes = serde_json::to_vec(&root)
        .map_err(|e| AssemblyError::new(SETTINGS_INVALID, e.to_string()))?;
    bytes.push(b'\n');
    Ok(Some(bytes))
}

/// A text file of the folder, at most `max` bytes; `None` when it is not there.
fn read_small(path: &Path, max: u64) -> Result<Option<String>, AssemblyError> {
    let fail =
        |why: String| AssemblyError::new(SETTINGS_INVALID, format!("{}: {why}", path.display()));
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(fail(e.to_string())),
    };
    let mut text = String::new();
    file.take(max + 1)
        .read_to_string(&mut text)
        .map_err(|e| fail(e.to_string()))?;
    if text.len() as u64 > max {
        return Err(fail("the file is too large".to_owned()));
    }
    Ok(Some(text))
}

/// The artifact with the settings of the folder of settings: `assembly3d.cfg` and the table of
/// models `models.csv`, each when it is there. STEP files named by the table are not read here, so
/// a part whose model is a STEP file is left out.
pub fn outline2d_with_folder(
    board: &Board,
    folder: &Path,
) -> Result<Option<Vec<u8>>, AssemblyError> {
    let defaults = AssemblySettings::default();
    let settings = match read_small(&folder.join("assembly3d.cfg"), defaults.max_file_bytes)? {
        Some(text) => AssemblySettings::parse(&text)?,
        None => defaults,
    };
    let table = match read_small(&folder.join("models.csv"), settings.max_file_bytes)? {
        Some(text) => ModelTable::parse(&text, settings.max_components)?,
        None => ModelTable {
            entries: BTreeMap::new(),
        },
    };
    outline2d(board, &table, &settings, &mut |file| {
        Err(AssemblyError::new(
            SETTINGS_INVALID,
            format!("the STEP file {file} is not read for the outline"),
        ))
    })
}
