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
use eh3d_phoenix_boardedit::{apply_operations, read_operations, BoardEdit, BoardEditSettings};
use eh3d_phoenix_eagle_board::write_board;
use eh3d_phoenix_rules::{Lang, Severity};
use serde_json::json;

use crate::check::read_text;
use crate::design::{has_extension, same_file};
use crate::route::{load_board, load_context, write_new};

/// The settings file that is read when `--settings` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/boardedit.cfg";

pub(crate) const USAGE: &str = "\
phoenix boardedit [--rules FILE] [--settings FILE] [--json] [--lang CODE] --apply OPERATIONS.json
                  [--out FILE] <board>

  boardedit     apply a file of editing operations to a board (an EAGLE .brd file or the JSON of
                the design model): place, move, turn, mirror, lock and remove components; draw
                orthogonal and 45-degree tracks with the width of the net class; add vias; change
                layer; delete a stretch, a track, a via or a net; paste a net with the shortest
                route between two pads; tune a net to a length with meanders. Each operation is
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
  --json        print the report as JSON. --lang CODE writes the rule titles in en, es, de, fr, it,
                ja, zh or pt. Exit code: 0 no error proven on the result, 1 an error is proven by
                the rules, 2 unusable input
";

struct Options {
    apply: Option<String>,
    out: Option<String>,
    rules: Option<String>,
    settings: Option<String>,
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
        if let Err(e) = check_output(path, &inputs) {
            return fail(err, &e.to_string());
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
    let mut edit = match BoardEdit::new(board, ctx, settings) {
        Ok(edit) => edit,
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

    if options.json {
        let value = json!({
            "schema": "phoenix-boardedit/1",
            "result": report.to_value(),
            "rule_errors": rule_errors,
            "written": written,
        });
        let _ = writeln!(out, "{value}");
    } else {
        let _ = out.write_all(report.to_text(options.lang).as_bytes());
        let _ = writeln!(out, "rule errors on the result: {rule_errors}");
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
