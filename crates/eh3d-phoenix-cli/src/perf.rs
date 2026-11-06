// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `perf`: a read-only report of how much work the checks of a board do with the
//! spatial index and without it. It counts and never times: the number of objects, the cells the
//! index uses, and for each module the comparisons made with the index and the comparisons of
//! every pair. The same board and settings always give the same report.
//!
//! Exit codes: 0 done, 1 the settings would not let the index be built (`PHOENIX-SPATIAL-002` or
//! `003`), 2 the arguments or the input are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_model::Nm;
use eh3d_phoenix_route::{ratsnest_with, RatsnestWork, RouteSettings};
use eh3d_phoenix_rules::{clearance_work, ClearanceWork, IndexMode, RuleValues};
use eh3d_phoenix_spatial::codes::{INPUT_UNREADABLE, OPTION_INVALID};
use eh3d_phoenix_spatial::{SpatialConfig, SpatialError, LANGUAGES};
use serde_json::{json, Value};

use crate::export::read_board_file;

pub(crate) const USAGE: &str = "\
phoenix perf <board> [--config FILE] [--clearance-mm N] [--json] [--lang CODE]

  perf          how much work the checks of a board do with the spatial index and without it, in
                comparisons and not in seconds: the objects of the board, the cells the index uses
                and, for the copper clearance check and the ratsnest, the comparisons made with the
                index and the comparisons of every pair. Read-only, the same every time; a
                prototype. Exit code 1 when the settings would not let the index be built
  --config      the settings of the index (default: config/spatial.cfg when it exists)
  --clearance-mm the clearance the copper clearance check is counted for (default 0.2)
  --json        print the report as JSON. --lang CODE writes the titles in en, es, de, fr, it,
                ja, zh or pt
";

const DEFAULT_CONFIG: &str = "config/spatial.cfg";
const DEFAULT_CLEARANCE_NM: i64 = 200_000;
/// The largest clearance accepted, in millimetres.
const MAX_CLEARANCE_MM: f64 = 1000.0;

struct Options {
    config: Option<String>,
    clearance_nm: i64,
    json: bool,
    lang: String,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, SpatialError> {
    let bad = |what: &str| SpatialError::new(OPTION_INVALID, what.to_owned());
    let mut o = Options {
        config: None,
        clearance_nm: DEFAULT_CLEARANCE_NM,
        json: false,
        lang: "en".to_owned(),
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, SpatialError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => o.json = true,
            "--config" => o.config = Some(value("--config needs a file")?),
            "--clearance-mm" => {
                let text = value("--clearance-mm needs a number")?;
                let mm: f64 = text
                    .parse()
                    .map_err(|_| bad("--clearance-mm is not a number"))?;
                if !mm.is_finite() || mm <= 0.0 || mm > MAX_CLEARANCE_MM {
                    return Err(bad("--clearance-mm must be above 0 and at most 1000"));
                }
                o.clearance_nm = (mm * 1_000_000.0).round() as i64;
            }
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                if !LANGUAGES.contains(&tag.as_str()) {
                    return Err(bad("unknown language code"));
                }
                o.lang = tag;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn load_config(path: Option<&str>) -> Result<SpatialConfig, SpatialError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(SpatialConfig::default()),
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| SpatialError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            SpatialConfig::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &SpatialError, tag: &str) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(tag));
    2
}

/// What the board holds, for the report.
struct Objects {
    tracks: usize,
    vias: usize,
    zones: usize,
    components: usize,
    nets: usize,
}

fn objects_of(board: &Board) -> Objects {
    Objects {
        tracks: board.tracks.len(),
        vias: board.vias.len(),
        zones: board.zones.len(),
        components: board.components.len(),
        nets: board.nets.len(),
    }
}

/// The ratsnest counted both ways, or the code of the error that stopped it.
type RatsnestCounts = Result<(RatsnestWork, RatsnestWork), &'static str>;

fn ratsnest_counts(board: &Board) -> RatsnestCounts {
    let settings = RouteSettings::default();
    let indexed = ratsnest_with(board, &settings, IndexMode::Grid).map_err(|e| e.code)?;
    let every_pair = ratsnest_with(board, &settings, IndexMode::Naive).map_err(|e| e.code)?;
    Ok((indexed.1, every_pair.1))
}

fn clearance_value(indexed: &ClearanceWork, every_pair: &ClearanceWork, limit_nm: i64) -> Value {
    json!({
        "module": "clearance",
        "clearance_nm": limit_nm,
        "items": indexed.items,
        "comparisons_indexed": indexed.comparisons,
        "comparisons_every_pair": every_pair.comparisons,
        "cell_nm": indexed.cell_nm.to_string(),
        "cells_used": indexed.cells_used,
        "entries": indexed.entries,
        "refused": indexed.refused,
    })
}

fn ratsnest_value(counts: &RatsnestCounts) -> Value {
    match counts {
        Ok((indexed, every_pair)) => json!({
            "module": "ratsnest",
            "nets": indexed.nets,
            "islands": indexed.islands,
            "anchors": indexed.anchors,
            "comparisons_indexed": indexed.comparisons,
            "comparisons_every_pair": every_pair.comparisons,
            "error": Value::Null,
        }),
        Err(code) => json!({
            "module": "ratsnest",
            "error": code,
        }),
    }
}

fn text_of(
    file: &str,
    objects: &Objects,
    config: &SpatialConfig,
    clearance: &Value,
    ratsnest: &Value,
) -> String {
    let mut out = format!("performance of {file}\n");
    out.push_str(&format!(
        "objects: {} tracks, {} vias, {} zones, {} components, {} nets\n",
        objects.tracks, objects.vias, objects.zones, objects.components, objects.nets
    ));
    out.push_str(&format!(
        "settings: cell_nm {}, max_cells {}, max_cells_per_item {}, max_items {}\n",
        config.cell_nm, config.max_cells, config.max_cells_per_item, config.max_items
    ));
    for module in [clearance, ratsnest] {
        let name = module["module"].as_str().unwrap_or("");
        if let Some(code) = module["error"].as_str() {
            out.push_str(&format!("{name}: not counted, {code}\n"));
            continue;
        }
        out.push_str(&format!(
            "{name}: comparisons with the index {}, of every pair {}",
            module["comparisons_indexed"], module["comparisons_every_pair"]
        ));
        if name == "clearance" {
            out.push_str(&format!(
                ", {} items, {} cells used, cell {} nm",
                module["items"],
                module["cells_used"],
                module["cell_nm"].as_str().unwrap_or("")
            ));
            if let Some(code) = module["refused"].as_str() {
                out.push_str(&format!(", index refused: {code}"));
            }
        } else {
            out.push_str(&format!(
                ", {} nets, {} islands, {} anchors",
                module["nets"], module["islands"], module["anchors"]
            ));
        }
        out.push('\n');
    }
    out
}

pub(crate) fn perf_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(e) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &e, "en");
        }
    };
    let tag = options.lang.as_str();
    let [path] = options.files.as_slice() else {
        let _ = err.write_all(USAGE.as_bytes());
        return fail(
            err,
            &SpatialError::new(OPTION_INVALID, "give exactly one board file"),
            tag,
        );
    };
    let config = match load_config(options.config.as_deref()) {
        Ok(config) => config,
        Err(e) => return fail(err, &e, tag),
    };
    let board = match read_board_file(path) {
        Ok(board) => board,
        Err(e) => return fail(err, &SpatialError::new(INPUT_UNREADABLE, e), tag),
    };
    let rules = RuleValues {
        min_clearance: Nm::new(options.clearance_nm).unwrap_or(Nm::ZERO),
        ..RuleValues::default()
    };
    let indexed = clearance_work(&board, &rules, IndexMode::Grid, &config);
    let every_pair = clearance_work(&board, &rules, IndexMode::Naive, &config);
    let clearance = clearance_value(&indexed, &every_pair, options.clearance_nm);
    let ratsnest = ratsnest_value(&ratsnest_counts(&board));
    let objects = objects_of(&board);
    if options.json {
        let value = json!({
            "format": "phoenix-perf/1",
            "objects": {
                "tracks": objects.tracks,
                "vias": objects.vias,
                "zones": objects.zones,
                "components": objects.components,
                "nets": objects.nets,
            },
            "settings": {
                "cell_nm": config.cell_nm,
                "max_cells": config.max_cells,
                "max_cells_per_item": config.max_cells_per_item,
                "max_items": config.max_items,
            },
            "modules": [clearance, ratsnest],
        });
        let text = serde_json::to_string(&value).unwrap_or_default();
        let _ = writeln!(out, "{text}");
    } else {
        let _ = out.write_all(text_of(path, &objects, &config, &clearance, &ratsnest).as_bytes());
    }
    if let Some(code) = indexed.refused {
        let _ = writeln!(err, "{code}: {}", eh3d_phoenix_spatial::title(code, tag));
        return 1;
    }
    0
}
