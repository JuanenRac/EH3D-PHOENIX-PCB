// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `boardedit`: applies a file of editing operations to a board. It is a dry run
//! unless `--out` names a new file, and it never writes over an input file or over a file that
//! exists.
//!
//! Exit codes: 0 done and the rules prove no error on the result, 1 done and the rules prove an
//! error on the result, 2 the arguments, the input, an operation or the output are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_boardedit::codes::{
    BoardEditError, OPERATIONS_INVALID, OUTPUT_NOT_ALLOWED, SETTINGS_INVALID,
};
use eh3d_phoenix_boardedit::{
    apply_operations, export_issues, poured_board, read_operations, BoardEdit, BoardEditSettings,
    Target, ZoneData, ZoneIssue,
};
use eh3d_phoenix_eagle_board::write_board;
use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_zones::ZoneSettings;
use serde_json::json;

use crate::check::read_text;
use crate::design::{has_extension, same_file};
use crate::pairs::load_pair_config;
use crate::route::{load_board, load_context, write_new};

/// The settings file that is read when `--settings` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/boardedit.cfg";

/// The settings of the fill of zones that are read when `--zones-config` is not given and the file
/// exists.
const DEFAULT_ZONES_CONFIG: &str = "config/zones.cfg";

pub(crate) const USAGE: &str = "\
phoenix boardedit [--rules FILE] [--settings FILE] [--pair-config FILE] [--pairs FILE] [--json]
                  [--zones FILE] [--zones-config FILE] [--zones-out FILE] [--poured-out FILE]
                  [--lang CODE] --apply OPERATIONS.json [--out FILE] <board>

  boardedit     apply a file of editing operations to a board (an EAGLE .brd file or the JSON of
                the design model): place, move, turn, mirror, lock and remove components; draw
                orthogonal and 45-degree tracks with the width of the net class; add vias; change
                layer; delete a stretch, a track, a via or a net; paste a net with the shortest
                route between two pads; tune a net to a length with meanders; draw the two tracks
                of a differential pair side by side (route_pair, with a via for each net where
                the pair changes layer), make the shorter net of a pair as long as the other
                with meanders (tune_pair) and move the pieces whose gap is out of tolerance
                (fix_pair_gap); route a track and push aside the tracks of other nets that its
                clearance would invade (route_shove, all or nothing, within the depth and the
                number of tracks of the settings); put arcs in tracks (route_arc at one vertex,
                round_corners on a track or a net); copper zones (zone_add, zone_edit,
                zone_remove, zone_fill, zone_priority, zone_cutout) and the paste of a net or a
                group with an offset, a turn and a mirror (paste_group); the selection of
                several objects (select) and the move, delete and align of the selection
                (move_selection, delete_selection, align_selection). Each operation is
                checked by the design rules and refused if it adds a rule error (an operation can
                be forced), and the first one that is refused ends the run and names its number.
                It is a dry run: nothing is written unless --out names a new file (.json for the
                design model, .brd for EAGLE); an input file, or a file that exists, is never
                written over. A prototype tried on boards made by hand only
  --apply       the file of operations: a JSON list, or an object with the list under \"ops\"
  --rules       the JSON rule file (format phoenix-rules/1 or 2); an EAGLE board gives its own
                design rules when the option is left out
  --settings    the grid, the width of a track and the meanders (default: config/boardedit.cfg
                when it exists)
  --pair-config the settings and the pairs of the pair operations (default:
                config/diffpairs.cfg when it exists)
  --pairs       a JSON file of pairs (format phoenix-diffpairs/1) for the pair operations
  --zones       the properties and fills of the zones kept earlier (format phoenix-zone-data/1)
  --zones-config the radii and limits of the fill (default: config/zones.cfg when it exists)
  --zones-out   write the properties and fills of the zones to a new file
  --poured-out  write the board with every filled zone replaced by the polygons of its copper
                to a new .json file, for the writers of files that know only outlines
  --json        print the report as JSON. --lang CODE writes the rule titles in en, es, de, fr, it,
                ja, zh or pt. Exit code: 0 no error proven on the result, 1 an error is proven by
                the rules, 2 unusable input
";

struct Options {
    apply: Option<String>,
    out: Option<String>,
    rules: Option<String>,
    settings: Option<String>,
    pair_config: Option<String>,
    pairs: Option<String>,
    zones: Option<String>,
    zones_config: Option<String>,
    zones_out: Option<String>,
    poured_out: Option<String>,
    json: bool,
    lang: Lang,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        apply: None,
        out: None,
        rules: None,
        settings: None,
        pair_config: None,
        pairs: None,
        zones: None,
        zones_config: None,
        zones_out: None,
        poured_out: None,
        json: false,
        lang: Lang::En,
        file: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--apply" => {
                let value = iter.next().ok_or("--apply needs a file")?;
                options.apply = Some((*value).to_owned());
            }
            "--out" => {
                let value = iter.next().ok_or("--out needs a file")?;
                options.out = Some((*value).to_owned());
            }
            "--rules" => {
                let value = iter.next().ok_or("--rules needs a file")?;
                options.rules = Some((*value).to_owned());
            }
            "--settings" => {
                let value = iter.next().ok_or("--settings needs a file")?;
                options.settings = Some((*value).to_owned());
            }
            "--pair-config" => {
                let value = iter.next().ok_or("--pair-config needs a file")?;
                options.pair_config = Some((*value).to_owned());
            }
            "--pairs" => {
                let value = iter.next().ok_or("--pairs needs a file")?;
                options.pairs = Some((*value).to_owned());
            }
            "--zones" => {
                let value = iter.next().ok_or("--zones needs a file")?;
                options.zones = Some((*value).to_owned());
            }
            "--zones-config" => {
                let value = iter.next().ok_or("--zones-config needs a file")?;
                options.zones_config = Some((*value).to_owned());
            }
            "--zones-out" => {
                let value = iter.next().ok_or("--zones-out needs a file")?;
                options.zones_out = Some((*value).to_owned());
            }
            "--poured-out" => {
                let value = iter.next().ok_or("--poured-out needs a file")?;
                options.poured_out = Some((*value).to_owned());
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => {
                if options.file.is_some() {
                    return Err("only one board".to_owned());
                }
                options.file = Some(other.to_owned());
            }
        }
    }
    Ok(options)
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

/// Checks the output before anything is done, so a refusal costs nothing.
fn check_output(path: &str, inputs: &[&str]) -> Result<(), BoardEditError> {
    let refuse = |why: String| BoardEditError::new(OUTPUT_NOT_ALLOWED, why);
    if inputs.iter().any(|input| same_file(path, input)) {
        return Err(refuse(format!("{path} is an input file")));
    }
    if Path::new(path).exists() {
        return Err(refuse(format!("{path} exists; give a new file")));
    }
    if !has_extension(path, "json") && !has_extension(path, "brd") {
        return Err(refuse(format!(
            "{path}: only .json (design model) and .brd (EAGLE) are written"
        )));
    }
    Ok(())
}

/// Checks a new JSON file (the zone data, the poured board) before anything is done.
fn check_new_json(path: &str, inputs: &[&str]) -> Result<(), BoardEditError> {
    let refuse = |why: String| BoardEditError::new(OUTPUT_NOT_ALLOWED, why);
    if inputs.iter().any(|input| same_file(path, input)) {
        return Err(refuse(format!("{path} is an input file")));
    }
    if Path::new(path).exists() {
        return Err(refuse(format!("{path} exists; give a new file")));
    }
    if !has_extension(path, "json") {
        return Err(refuse(format!("{path}: only .json is written here")));
    }
    Ok(())
}

fn load_zone_settings(path: Option<&str>) -> Result<ZoneSettings, String> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_ZONES_CONFIG)
            .is_file()
            .then_some(DEFAULT_ZONES_CONFIG)
    });
    match path {
        None => Ok(ZoneSettings::default()),
        Some(path) => {
            let text = read_text(path)?;
            ZoneSettings::parse(&text).map_err(|e| e.to_string())
        }
    }
}

/// What the zones say now and what each format cannot say about them.
fn zone_lines(edit: &BoardEdit, lang: Lang) -> Vec<String> {
    let mut lines: Vec<String> = edit
        .zone_issues()
        .iter()
        .map(|issue| issue.line(lang))
        .collect();
    for (name, target) in [
        ("gerber", Target::Gerber),
        ("dsn", Target::Dsn),
        ("kicad", Target::Kicad),
    ] {
        let issues: Vec<ZoneIssue> = export_issues(edit.board(), edit.zone_data(), target);
        lines.extend(
            issues
                .iter()
                .map(|issue| format!("{name}: {}", issue.line(lang))),
        );
    }
    lines
}

fn load_settings(path: Option<&str>) -> Result<BoardEditSettings, BoardEditError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_SETTINGS)
            .is_file()
            .then_some(DEFAULT_SETTINGS)
    });
    match path {
        None => Ok(BoardEditSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| BoardEditError::new(SETTINGS_INVALID, m))?;
            BoardEditSettings::parse(&text)
        }
    }
}

/// The command `boardedit`.
pub(crate) fn boardedit_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let (Some(file), Some(ops_path)) = (options.file.as_deref(), options.apply.as_deref()) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    if let Some(path) = options.out.as_deref() {
        let mut inputs = vec![file, ops_path];
        inputs.extend(options.rules.as_deref());
        inputs.extend(options.settings.as_deref());
        inputs.extend(options.pair_config.as_deref());
        inputs.extend(options.pairs.as_deref());
        inputs.extend(options.zones.as_deref());
        inputs.extend(options.zones_config.as_deref());
        if let Err(e) = check_output(path, &inputs) {
            return fail(err, &e.to_string());
        }
    }
    {
        let mut inputs = vec![file, ops_path];
        inputs.extend(options.rules.as_deref());
        inputs.extend(options.settings.as_deref());
        inputs.extend(options.pair_config.as_deref());
        inputs.extend(options.pairs.as_deref());
        inputs.extend(options.zones.as_deref());
        inputs.extend(options.zones_config.as_deref());
        inputs.extend(options.out.as_deref());
        for path in [options.zones_out.as_deref(), options.poured_out.as_deref()]
            .into_iter()
            .flatten()
        {
            if let Err(e) = check_new_json(path, &inputs) {
                return fail(err, &e.to_string());
            }
        }
        if let (Some(a), Some(b)) = (options.zones_out.as_deref(), options.poured_out.as_deref()) {
            if same_file(a, b) {
                return fail(err, "--zones-out and --poured-out name the same file");
            }
        }
    }
    let settings = match load_settings(options.settings.as_deref()) {
        Ok(settings) => settings,
        Err(e) => return fail(err, &e.to_string()),
    };
    let board = match load_board(file) {
        Ok(board) => board,
        Err(e) => return fail(err, &e.to_string()),
    };
    let ctx = match load_context(options.rules.as_deref(), file, &mut *err) {
        Ok(ctx) => ctx,
        Err(e) => return fail(err, &e.to_string()),
    };
    let ops = match read_text(ops_path)
        .map_err(|m| BoardEditError::new(OPERATIONS_INVALID, m))
        .and_then(|text| read_operations(&text))
    {
        Ok(ops) => ops,
        Err(e) => return fail(err, &e.to_string()),
    };
    let pair_config =
        match load_pair_config(options.pair_config.as_deref(), options.pairs.as_deref()) {
            Ok(config) => config,
            Err(e) => return fail(err, &e.to_string()),
        };
    let zone_settings = match load_zone_settings(options.zones_config.as_deref()) {
        Ok(settings) => settings,
        Err(m) => return fail(err, &m),
    };
    let zone_data = match options.zones.as_deref() {
        None => ZoneData::default(),
        Some(path) => match read_text(path)
            .map_err(|m| BoardEditError::new(OPERATIONS_INVALID, m))
            .and_then(|text| ZoneData::from_json(&text))
        {
            Ok(data) => data,
            Err(e) => return fail(err, &e.message(options.lang)),
        },
    };
    let mut edit = match BoardEdit::new(board, ctx, settings) {
        Ok(edit) => edit
            .with_pairs(pair_config)
            .with_zone_settings(zone_settings)
            .with_zone_data(zone_data),
        Err(e) => return fail(err, &e.to_string()),
    };
    let report = match apply_operations(&mut edit, &ops) {
        Ok(report) => report,
        Err(e) => return fail(err, &e.message(options.lang)),
    };
    let rule_errors = edit
        .findings()
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();

    let mut written: Option<String> = None;
    if let Some(path) = options.out.as_deref() {
        let bytes = if has_extension(path, "brd") {
            write_board(edit.board()).text.into_bytes()
        } else {
            match edit.board().to_bytes() {
                Ok(bytes) => bytes,
                Err(e) => return fail(err, &format!("cannot write the board: {e:?}")),
            }
        };
        if let Err(e) = write_new(path, &bytes) {
            return fail(err, &e.to_string());
        }
        written = Some(path.to_owned());
    }
    let mut zones_written: Option<String> = None;
    if let Some(path) = options.zones_out.as_deref() {
        if let Err(e) = write_new(path, edit.zone_data().to_json().as_bytes()) {
            return fail(err, &e.to_string());
        }
        zones_written = Some(path.to_owned());
    }
    let mut poured_written: Option<String> = None;
    if let Some(path) = options.poured_out.as_deref() {
        let bytes = match poured_board(edit.board(), edit.zone_data())
            .map_err(|e| e.message(options.lang))
            .and_then(|(board, _)| {
                board
                    .to_bytes()
                    .map_err(|e| format!("cannot write the board: {e:?}"))
            }) {
            Ok(bytes) => bytes,
            Err(m) => return fail(err, &m),
        };
        if let Err(e) = write_new(path, &bytes) {
            return fail(err, &e.to_string());
        }
        poured_written = Some(path.to_owned());
    }
    let zone_notes = zone_lines(&edit, options.lang);

    if options.json {
        let value = json!({
            "schema": "phoenix-boardedit/1",
            "result": report.to_value(),
            "rule_errors": rule_errors,
            "written": written,
            "zones": {
                "notes": zone_notes,
                "data_written": zones_written,
                "poured_written": poured_written,
            },
        });
        let _ = writeln!(out, "{value}");
    } else {
        let _ = out.write_all(report.to_text(options.lang).as_bytes());
        let _ = writeln!(out, "rule errors on the result: {rule_errors}");
        for line in &zone_notes {
            let _ = writeln!(out, "zones: {line}");
        }
        for path in [&zones_written, &poured_written].into_iter().flatten() {
            let _ = writeln!(out, "written: {path}");
        }
        match &written {
            Some(path) => {
                let _ = writeln!(out, "written: {path}");
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written (--out FILE writes the result)"
                );
            }
        }
    }
    if rule_errors > 0 {
        1
    } else {
        0
    }
}
