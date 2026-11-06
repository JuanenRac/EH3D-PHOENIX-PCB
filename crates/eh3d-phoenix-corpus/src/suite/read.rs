// SPDX-License-Identifier: GPL-3.0-or-later
//! The readers and writers that run on one piece of the corpus. Each kind gives an [`Outcome`]:
//! whether the file was read or refused, the diagnostic codes seen and the measures.

use std::fs;
use std::path::Path;

use eh3d_phoenix_board::rules::RuleSet;
use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle::{read_bytes, Limits};
use eh3d_phoenix_eagle_board::{read_board_with_limits, read_schematic_with_limits};
use eh3d_phoenix_erc::{check as erc_check, ErcConfig};
use eh3d_phoenix_gerber::{import_file, Content, ImportSettings};
use eh3d_phoenix_kicad::{
    read_footprint, read_pcb, read_project, read_schematic, read_symbol_library, KicadSettings,
    QualityItem,
};
use eh3d_phoenix_route::live::RuleContext;
use eh3d_phoenix_route::settings::RouteSettings;
use eh3d_phoenix_route::{dsn, ses};
use eh3d_phoenix_sim::{operating_point, parse, Circuit, SimSettings};
use serde_json::Value;

use super::codes::{SuiteError, CORPUS_INCONSISTENT, EXPECTATION_INVALID, FILE_TOO_LARGE};
use super::entry::Entry;
use super::facts::{board_measures, id_source};
use super::outcome::Outcome;
use super::settings::CorpusSettings;

/// Reads a data file of the corpus, refusing one over the limit.
pub fn read_file(
    folder: &Path,
    name: &str,
    settings: &CorpusSettings,
) -> Result<Vec<u8>, SuiteError> {
    let path = folder.join(name);
    let unreadable =
        |e: std::io::Error| SuiteError::new(CORPUS_INCONSISTENT, format!("{name}: {e}"));
    let size = fs::metadata(&path).map_err(unreadable)?.len();
    if size > settings.max_file_bytes {
        return Err(SuiteError::new(
            FILE_TOO_LARGE,
            format!(
                "{name}: {size} bytes, the limit is {}",
                settings.max_file_bytes
            ),
        ));
    }
    fs::read(&path).map_err(|e| SuiteError::new(CORPUS_INCONSISTENT, format!("{name}: {e}")))
}

/// The limits of an entry as numbers, checked against the names the reader knows. `@size` is the
/// size of the data file and `@size-1` one byte less.
pub fn limits_of(
    entry: &Entry,
    size: u64,
    allowed: &[&str],
) -> Result<Vec<(String, u64)>, SuiteError> {
    let mut out = Vec::new();
    for (key, value) in &entry.limits {
        if !allowed.contains(&key.as_str()) {
            return Err(SuiteError::new(
                EXPECTATION_INVALID,
                format!("{}: the limit {key} is not one of this reader", entry.id),
            ));
        }
        let number = match value {
            Value::Number(n) => n.as_u64(),
            Value::String(s) if s == "@size" => Some(size),
            Value::String(s) if s == "@size-1" => Some(size.saturating_sub(1)),
            _ => None,
        };
        let number = number.ok_or_else(|| {
            SuiteError::new(
                EXPECTATION_INVALID,
                format!(
                    "{}: the limit {key} is not a whole number, @size or @size-1",
                    entry.id
                ),
            )
        })?;
        out.push((key.clone(), number));
    }
    Ok(out)
}

fn settings_text(limits: &[(String, u64)]) -> String {
    let mut text = String::new();
    for (key, value) in limits {
        text.push_str(&format!("{key} = {value}\n"));
    }
    text
}

fn kicad_settings(entry: &Entry, size: u64) -> Result<KicadSettings, SuiteError> {
    let limits = limits_of(
        entry,
        size,
        &[
            "max_file_bytes",
            "max_depth",
            "max_nodes",
            "max_string_bytes",
            "max_items",
            "max_sheets",
            "max_sheet_depth",
            "max_diagnostics",
            "max_project_bytes",
        ],
    )?;
    KicadSettings::parse(&settings_text(&limits))
        .map_err(|e| SuiteError::new(EXPECTATION_INVALID, format!("{}: {e}", entry.id)))
}

fn gerber_settings(entry: &Entry, size: u64) -> Result<ImportSettings, SuiteError> {
    let limits = limits_of(
        entry,
        size,
        &[
            "max_file_bytes",
            "max_commands",
            "max_apertures",
            "max_objects",
            "max_region_points",
            "max_macro_primitives",
            "coordinate_limit_nm",
        ],
    )?;
    ImportSettings::parse(&settings_text(&limits))
        .map_err(|e| SuiteError::new(EXPECTATION_INVALID, format!("{}: {e}", entry.id)))
}

fn quality(
    o: &mut Outcome,
    kept: &[QualityItem],
    transformed: &[QualityItem],
    dropped: &[QualityItem],
) {
    for (prefix, list) in [
        ("kept", kept),
        ("transformed", transformed),
        ("dropped", dropped),
    ] {
        for item in list {
            o.put(&format!("{prefix}.{}", item.category), item.count);
        }
    }
}

/// The text of a file that is meant to be text: invalid UTF-8 is turned into replacement
/// characters, never refused here.
fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn eagle(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let mut limits = Limits::default();
    for (key, value) in limits_of(
        entry,
        bytes.len() as u64,
        &["max_bytes", "max_depth", "max_nodes"],
    )? {
        let value = usize::try_from(value).unwrap_or(usize::MAX);
        match key.as_str() {
            "max_bytes" => limits.max_bytes = value,
            "max_depth" => limits.max_depth = value,
            _ => limits.max_nodes = value,
        }
    }
    let reading = read_bytes(bytes, &limits);
    let mut o = Outcome::default();
    for d in &reading.diagnostics {
        if let Some(c) = d.code.as_deref() {
            o.code(c);
        }
    }
    match &reading.summary {
        None => o.refused = true,
        Some(s) => {
            o.put("kind", s.kind.name());
            o.put("version", s.version.as_str());
            o.put("layers", s.layers);
            o.put("packages", s.packages.len());
            o.put(
                "package_pads",
                s.packages.iter().map(|p| p.pads).sum::<usize>(),
            );
            o.put(
                "package_smds",
                s.packages.iter().map(|p| p.smds).sum::<usize>(),
            );
            o.put("symbols", s.symbols.len());
            o.put(
                "symbol_pins",
                s.symbols.iter().map(|p| p.pins).sum::<usize>(),
            );
            o.put("device_sets", s.device_sets.len());
            o.put("parts", s.parts);
            o.put("nets", s.nets);
            o.put("routed_items", s.routed_items);
            o.put("sheets", s.sheets);
            o.put("embedded_libraries", s.embedded_libraries);
            for (name, n) in &s.uninterpreted {
                o.put(&format!("uninterpreted.{name}"), *n);
            }
        }
    }
    Ok(o)
}

/// The findings of the electrical rule check as measures (`erc.confirmed`, `erc.<code>`) and codes.
fn erc_measures(o: &mut Outcome, report: &eh3d_phoenix_erc::Report) {
    o.put("erc.confirmed", report.confirmed.len());
    o.put("erc.inconclusive", report.inconclusive.len());
    for f in report.confirmed.iter().chain(report.inconclusive.iter()) {
        o.code(f.code);
    }
    for f in &report.confirmed {
        let key = format!("erc.{}", f.code);
        let seen = o.measures.get(&key).and_then(Value::as_u64).unwrap_or(0);
        o.put(&key, seen + 1);
    }
}

fn eagle_board(bytes: &[u8], settings: &CorpusSettings) -> Outcome {
    let mut ids = id_source(1);
    let mut o = Outcome::default();
    match read_board_with_limits(&text_of(bytes), &mut ids, &settings.eagle_limits) {
        Ok(read) => {
            o.merge(board_measures(&read.board));
            for d in &read.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    o
}

fn eagle_schematic(entry: &Entry, bytes: &[u8], settings: &CorpusSettings) -> Outcome {
    let mut ids = id_source(1);
    let mut o = Outcome::default();
    let footprints = std::collections::BTreeMap::new();
    match read_schematic_with_limits(
        &text_of(bytes),
        &footprints,
        &mut ids,
        &settings.eagle_limits,
    ) {
        Ok(read) => {
            let s = &read.schematic;
            o.put("symbols", s.symbols.len());
            o.put("parts", s.parts.len());
            o.put("nets", s.nets.len());
            o.put("sheets", s.sheets.len());
            o.put("wires", s.wires.len());
            o.put("buses", s.buses.len());
            o.put("bus_entries", s.bus_entries.len());
            if entry.erc {
                let report = erc_check(s, &ErcConfig::default());
                erc_measures(&mut o, &report);
            }
            for d in &read.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    o
}

fn kicad_board(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let settings = kicad_settings(entry, bytes.len() as u64)?;
    let mut ids = id_source(1);
    let mut o = Outcome::default();
    match read_pcb(bytes, &settings, &mut ids) {
        Ok(r) => {
            o.merge(board_measures(&r.board));
            o.put("version", r.version.as_str());
            o.put("generator", r.generator.as_str());
            quality(
                &mut o,
                &r.quality.kept,
                &r.quality.transformed,
                &r.quality.dropped,
            );
            for d in &r.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    Ok(o)
}

fn kicad_schematic(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let settings = kicad_settings(entry, bytes.len() as u64)?;
    let mut ids = id_source(1);
    let mut o = Outcome::default();
    match read_schematic(bytes, &settings, &mut ids) {
        Ok(r) => {
            let s = &r.schematic;
            o.put("symbols", s.symbols.len());
            o.put("parts", s.parts.len());
            o.put("nets", s.nets.len());
            o.put("sheets", s.sheets.len());
            o.put("wires", s.wires.len());
            o.put("sheets_read", r.sheets_read);
            if entry.erc {
                let report = erc_check(s, &ErcConfig::default());
                erc_measures(&mut o, &report);
            }
            o.put("version", r.version.as_str());
            o.put("generator", r.generator.as_str());
            quality(
                &mut o,
                &r.quality.kept,
                &r.quality.transformed,
                &r.quality.dropped,
            );
            for d in &r.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    Ok(o)
}

fn kicad_footprint(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let settings = kicad_settings(entry, bytes.len() as u64)?;
    let mut ids = id_source(1);
    let mut o = Outcome::default();
    match read_footprint(bytes, &settings, &mut ids) {
        Ok(r) => {
            o.put("pads", r.footprint.pads.len());
            o.put("name", r.footprint.name.as_str());
            o.put("reference", r.reference.as_str());
            o.put("value", r.value.as_str());
            o.put("version", r.version.as_str());
            quality(
                &mut o,
                &r.quality.kept,
                &r.quality.transformed,
                &r.quality.dropped,
            );
            for d in &r.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    Ok(o)
}

fn kicad_symbols(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let settings = kicad_settings(entry, bytes.len() as u64)?;
    let mut ids = id_source(1);
    let mut o = Outcome::default();
    match read_symbol_library(bytes, &settings, &mut ids) {
        Ok(r) => {
            o.put("symbols", r.symbols.len());
            o.put("version", r.version.as_str());
            quality(
                &mut o,
                &r.quality.kept,
                &r.quality.transformed,
                &r.quality.dropped,
            );
            for d in &r.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    Ok(o)
}

fn kicad_project(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let settings = kicad_settings(entry, bytes.len() as u64)?;
    let mut o = Outcome::default();
    match read_project(bytes, &settings) {
        Ok(r) => {
            if let Some(v) = r.meta_version {
                o.put("meta_version", v);
            }
            o.put("net_classes", r.net_classes.len());
            o.put("design_rules", r.design_rules.len());
            o.put("text_variables", r.text_variables);
            for (name, value) in &r.design_rules {
                o.put(&format!("rule.{name}"), *value);
            }
            for class in &r.net_classes {
                if let Some(w) = class.track_width_nm {
                    o.put(&format!("class.{}.track_width_nm", class.name), w);
                }
            }
            for d in &r.diagnostics {
                if let Some(c) = d.code.as_deref() {
                    o.code(c);
                }
            }
        }
        Err(d) => {
            o.refused = true;
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
    }
    Ok(o)
}

fn gerber(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let settings = gerber_settings(entry, bytes.len() as u64)?;
    let name = Path::new(&entry.input)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let file = import_file(&name, bytes, &settings);
    let mut o = Outcome::default();
    match &file.content {
        Content::Gerber(layer) => {
            o.put("content", "gerber");
            o.put("units", layer.units.name());
            o.put("items", layer.items.len());
            o.put("flashes", layer.flashes());
            o.put("lines", layer.lines());
            o.put("arcs", layer.arcs());
            o.put("apertures", layer.apertures.len());
            o.put("ended", layer.ended);
            o.put("complete", layer.complete);
            o.put("attribute_sets", layer.attribute_sets.len());
        }
        Content::Drill(drill) => {
            o.put("content", "excellon");
            o.put("units", drill.units.name());
            o.put("hits", drill.hits.len());
            o.put("slots", drill.slots.len());
            o.put("tools", drill.tools.len());
            o.put("ended", drill.ended);
        }
        Content::Failed(e) => {
            o.refused = true;
            o.put("content", "failed");
            o.code(e.code);
        }
    }
    for finding in file.findings() {
        o.code(finding.code);
    }
    if let Some(found) = file.identified {
        o.put("role", found.role.name());
    }
    Ok(o)
}

fn board_from(bytes: &[u8]) -> Result<Board, String> {
    Board::from_bytes(bytes).map_err(|e| e.to_string())
}

fn board_json(entry: &Entry, bytes: &[u8], settings: &CorpusSettings) -> Outcome {
    let board = match board_from(bytes) {
        Ok(board) => board,
        Err(why) => {
            let mut o = Outcome::refusal(None);
            o.put("error", why);
            return o;
        }
    };
    let mut o = Outcome::default();
    o.merge(board_measures(&board));
    o.put(
        "consistent",
        board.is_consistent_with(&settings.board_limits),
    );
    for d in board.validate_with(&settings.board_limits) {
        if let Some(c) = d.code.as_deref() {
            o.code(c);
            let key = format!("validate.{c}");
            let seen = o.measures.get(&key).and_then(Value::as_u64).unwrap_or(0);
            o.put(&key, seen + 1);
        }
    }
    if let Some(text) = &entry.rules {
        let (rules, problems) = RuleSet::parse(text);
        for d in &problems {
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
        let report = board.check_rules(&rules);
        o.put("drc.confirmed", report.confirmed.len());
        o.put("drc.inconclusive", report.inconclusive.len());
        for d in report.confirmed.iter().chain(report.inconclusive.iter()) {
            if let Some(c) = d.code.as_deref() {
                o.code(c);
            }
        }
        for d in &report.confirmed {
            if let Some(c) = d.code.as_deref() {
                let key = format!("drc.{c}");
                let seen = o.measures.get(&key).and_then(Value::as_u64).unwrap_or(0);
                o.put(&key, seen + 1);
            }
        }
    }
    let connected = board.connectivity();
    o.put("connectivity.nets", connected.nets.len());
    o.put(
        "connectivity.islands",
        connected
            .nets
            .iter()
            .map(|n| n.islands.len())
            .sum::<usize>(),
    );
    o.put(
        "connectivity.missing",
        connected
            .nets
            .iter()
            .map(|n| n.missing_connections())
            .sum::<usize>(),
    );
    o
}

fn dsn_counts(o: &mut Outcome, prefix: &str, board: &Board) -> bool {
    match dsn::export(board, &RuleContext::default(), &RouteSettings::default()) {
        Ok(d) => {
            o.put(&format!("{prefix}layers"), d.layers);
            o.put(&format!("{prefix}components"), d.components);
            o.put(&format!("{prefix}pads"), d.pads);
            o.put(&format!("{prefix}nets"), d.nets);
            o.put(&format!("{prefix}padstacks"), d.padstacks);
            o.put(&format!("{prefix}tracks"), d.tracks);
            o.put(&format!("{prefix}vias"), d.vias);
            o.put(&format!("{prefix}zone_cutouts"), d.zone_cutouts);
            true
        }
        Err(e) => {
            o.code(e.code);
            o.notes.push(e.detail);
            false
        }
    }
}

fn dsn_kind(bytes: &[u8]) -> Outcome {
    match board_from(bytes) {
        Ok(board) => {
            let mut o = Outcome::default();
            if !dsn_counts(&mut o, "", &board) {
                o.refused = true;
                o.notes.clear();
            }
            o
        }
        Err(why) => {
            let mut o = Outcome::refusal(None);
            o.put("error", why);
            o
        }
    }
}

fn dsn_ses(bytes: &[u8], session: &[u8]) -> Outcome {
    let board = match board_from(bytes) {
        Ok(board) => board,
        Err(why) => {
            let mut o = Outcome::refusal(None);
            o.put("error", why);
            return o;
        }
    };
    let mut o = Outcome::default();
    if !dsn_counts(&mut o, "before.", &board) {
        o.refused = true;
        o.notes.clear();
        return o;
    }
    let mut ids = id_source(1000);
    let plan = ses::import(
        &board,
        &RuleContext::default(),
        &RouteSettings::default(),
        &text_of(session),
        &mut ids,
    );
    match plan {
        Err(e) => {
            return Outcome::refusal(Some(e.code));
        }
        Ok(plan) => {
            o.put("tracks", plan.tracks.len());
            o.put("vias", plan.vias.len());
            o.put("already_there", plan.already_there);
            o.put("length_nm", plan.length_nm);
            for (name, tracks, vias, length) in &plan.nets {
                o.put(&format!("net.{name}.tracks"), *tracks);
                o.put(&format!("net.{name}.vias"), *vias);
                o.put(&format!("net.{name}.length_nm"), *length);
            }
            let mut after = board.clone();
            if let Err(e) = plan.apply(&mut after) {
                return Outcome::refusal(Some(e.code));
            }
            o.put("consistent_after", after.is_consistent());
            if !dsn_counts(&mut o, "after.", &after) {
                o.refused = true;
                o.notes.clear();
            }
        }
    }
    o
}

fn spice(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let mut settings = SimSettings::default();
    for (key, value) in limits_of(entry, bytes.len() as u64, &["max_file_bytes"])? {
        if key == "max_file_bytes" {
            settings.max_file_bytes = value;
        }
    }
    let text = text_of(bytes);
    let mut o = Outcome::default();
    let netlist = match parse(&text, &settings) {
        Ok(n) => n,
        Err(e) => return Ok(Outcome::refusal(Some(e.code))),
    };
    let circuit = match Circuit::compile(&netlist, &settings) {
        Ok(c) => c,
        Err(e) => return Ok(Outcome::refusal(Some(e.code))),
    };
    let result = match operating_point(&circuit, &settings) {
        Ok(r) => r,
        Err(e) => return Ok(Outcome::refusal(Some(e.code))),
    };
    for label in entry.expect.probes.keys() {
        if let Some(v) = result.get(label) {
            o.probes.insert(label.clone(), v);
        }
    }
    Ok(o)
}

/// Runs the reader, the writer or the chain of an entry on its data file.
pub fn run_entry(
    entry: &Entry,
    folder: &Path,
    settings: &CorpusSettings,
) -> Result<Outcome, SuiteError> {
    let bytes = read_file(folder, &entry.input, settings)?;
    match entry.kind.as_str() {
        "eagle" => eagle(entry, &bytes),
        "eagle-board" => Ok(eagle_board(&bytes, settings)),
        "eagle-schematic" => Ok(eagle_schematic(entry, &bytes, settings)),
        "kicad-board" => kicad_board(entry, &bytes),
        "kicad-schematic" => kicad_schematic(entry, &bytes),
        "kicad-footprint" => kicad_footprint(entry, &bytes),
        "kicad-symbols" => kicad_symbols(entry, &bytes),
        "kicad-project" => kicad_project(entry, &bytes),
        "gerber" => gerber(entry, &bytes),
        "board-json" => Ok(board_json(entry, &bytes, settings)),
        "dsn" => Ok(dsn_kind(&bytes)),
        "dsn-ses" => {
            let name = entry.session.as_deref().unwrap_or_default();
            let session = read_file(folder, name, settings)?;
            Ok(dsn_ses(&bytes, &session))
        }
        "spice" => spice(entry, &bytes),
        "roundtrip" => super::trips::roundtrip(entry, &bytes, settings),
        "gerber-roundtrip" => super::trips::gerber_roundtrip(entry, &bytes),
        other => Err(SuiteError::new(
            EXPECTATION_INVALID,
            format!("{}: unknown kind {other:?}", entry.id),
        )),
    }
}
