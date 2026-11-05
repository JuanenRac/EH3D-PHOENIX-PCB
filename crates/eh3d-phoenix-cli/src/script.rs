// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `script` and `cam-import`: scripts of the old command language run on a board or a
//! schematic, the compatibility report of scripts, and output jobs read into an export profile.
//!
//! A run is a dry run unless `--out` is given and `--dry-run` is not: the result is then written to
//! a new file (an existing file is never replaced, and an input is never overwritten), together with
//! the files that `WRITE` asked for when the settings allow them. Nothing runs a program.
//!
//! Exit codes: 0 done, 1 the script was not carried out (its diagnostics are printed), 2 the
//! arguments or the input are unusable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle_board::read_schematic;
use eh3d_phoenix_rules::{Lang, RuleFile, RuleValues};
use eh3d_phoenix_schematic::Schematic;
use eh3d_phoenix_script::codes::{Diag, FILE_FAILED, INPUT_UNREADABLE, OPTION_INVALID};
use eh3d_phoenix_script::files::write_new;
use eh3d_phoenix_script::{
    analyse_path, commit_writes, import_cam, parse_script, run_board, run_schematic, Run,
    RunContext, ScriptSettings,
};
use serde_json::json;

use crate::check::{id_source, read_text};
use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/script.cfg";

pub(crate) const USAGE: &str = "\
phoenix script run [--config FILE] [--rules FILE] [--dry-run] [--out FILE] [--json] [--lang CODE]
                   <script> <board or schematic>
phoenix script report [--config FILE] [--json] [--html FILE] [--lang CODE] <folder or script>
phoenix cam-import [--config FILE] [--json] [--out FILE] [--lang CODE] <job.cam>

  script run    run a script of the old command language (WIRE, VIA, MOVE, CHANGE, DRC, ...) on a
                board (an EAGLE .brd file or the JSON of the model) or, for the commands that do
                not edit a board, on a schematic. It is a dry run: the script is carried out on a
                copy and only the outcome is printed. With --out FILE (and without --dry-run) the
                board that results is written to a new file; an existing file is never replaced and
                the input is never overwritten. All or nothing: an error stops the whole run. The
                file commands of a script (SCRIPT, OPEN, WRITE) need the permissions and folders
                of the settings (config/script.cfg) and only use paths below the script's folder
  script report the compatibility of scripts, computed without running them: every command is
                equivalent, adapted (with the difference) or pending (with the cause); one line per
                script and the totals of a folder; --html FILE writes one self-contained page to a
                new file
  cam-import    read an output job (.cam) into a profile of the export: which files, extensions and
                options could be translated and which not; --out FILE writes the profile file
  --rules       the JSON rule file that DRC checks against (without it every rule is off)
  --config      the settings file (default: config/script.cfg when it exists)
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    config: Option<String>,
    rules: Option<String>,
    out: Option<String>,
    html: Option<String>,
    dry_run: bool,
    json: bool,
    lang: Lang,
    files: Vec<String>,
}

fn bad(detail: impl Into<String>) -> Diag {
    Diag::plain(OPTION_INVALID, detail)
}

fn parse_options(args: &[&str]) -> Result<Options, Diag> {
    let mut options = Options {
        config: None,
        rules: None,
        out: None,
        html: None,
        dry_run: false,
        json: false,
        lang: Lang::En,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--dry-run" => options.dry_run = true,
            "--lang" => {
                let tag = iter.next().ok_or_else(|| bad("--lang needs a code"))?;
                options.lang = Lang::from_tag(tag)
                    .ok_or_else(|| bad("--lang is not a language of the suite"))?;
            }
            "--config" => {
                options.config =
                    Some((*iter.next().ok_or_else(|| bad("--config needs a file"))?).to_owned());
            }
            "--rules" => {
                options.rules =
                    Some((*iter.next().ok_or_else(|| bad("--rules needs a file"))?).to_owned());
            }
            "--out" => {
                options.out =
                    Some((*iter.next().ok_or_else(|| bad("--out needs a file"))?).to_owned());
            }
            "--html" => {
                options.html =
                    Some((*iter.next().ok_or_else(|| bad("--html needs a file"))?).to_owned());
            }
            other if other.starts_with("--") => {
                return Err(bad(format!("unknown option {other}")));
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<ScriptSettings, Diag> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_SETTINGS)
            .is_file()
            .then_some(DEFAULT_SETTINGS)
    });
    match path {
        None => Ok(ScriptSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| Diag::plain(INPUT_UNREADABLE, m))?;
            ScriptSettings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, diag: &Diag, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", diag.message(lang));
    2
}

fn usage(err: &mut dyn Write) -> i32 {
    let _ = err.write_all(USAGE.as_bytes());
    2
}

fn has_extension(path: &str, wanted: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(wanted))
}

enum Document {
    Board(Board),
    Schematic(Schematic),
}

fn load_document(path: &str) -> Result<Document, Diag> {
    let unreadable = |m: String| Diag::plain(INPUT_UNREADABLE, m);
    if has_extension(path, "sch") {
        let text = read_text(path).map_err(unreadable)?;
        let mut new_id = id_source().map_err(unreadable)?;
        let footprints = BTreeMap::new();
        return read_schematic(&text, &footprints, &mut new_id)
            .map(|read| Document::Schematic(read.schematic))
            .map_err(|d| Diag::plain(INPUT_UNREADABLE, format!("{path}: {}", d.message)));
    }
    if has_extension(path, "json") {
        let text = read_text(path).map_err(unreadable)?;
        if let Ok(board) = Board::from_bytes(text.as_bytes()) {
            return Ok(Document::Board(board));
        }
        return Schematic::from_bytes(text.as_bytes())
            .map(Document::Schematic)
            .map_err(|e| {
                Diag::plain(
                    INPUT_UNREADABLE,
                    format!("{path} is neither a board nor a schematic: {e}"),
                )
            });
    }
    read_board_file(path)
        .map(Document::Board)
        .map_err(unreadable)
}

fn load_rules(options: &Options, document_path: &str) -> Result<RuleValues, Diag> {
    if let Some(path) = &options.rules {
        let text = read_text(path).map_err(|m| Diag::plain(INPUT_UNREADABLE, m))?;
        let file = RuleFile::from_json(&text)
            .map_err(|e| Diag::plain(INPUT_UNREADABLE, format!("{path}: {e}")))?;
        return Ok(file.values);
    }
    if has_extension(document_path, "brd") {
        if let Ok(text) = read_text(document_path) {
            if let Ok((rules, _)) = RuleValues::from_eagle_board(&text) {
                return Ok(rules);
            }
        }
    }
    Ok(RuleValues::default())
}

fn run_summary(run: &Run, options: &Options, written: &[String], out: &mut dyn Write) {
    if options.json {
        let diagnostics: Vec<serde_json::Value> = run
            .diagnostics
            .iter()
            .chain(run.warnings.iter())
            .map(|d| {
                json!({
                    "code": d.code,
                    "line": d.line,
                    "column": d.col,
                    "detail": d.detail,
                    "warning": run.warnings.contains(d),
                })
            })
            .collect();
        let value = json!({
            "schema": "phoenix-script-run/1",
            "ok": run.ok,
            "executed": run.executed,
            "skipped": run.skipped,
            "operations": run.operations,
            "failed_checks": run.failed_checks,
            "diagnostics": diagnostics,
            "messages": run.messages,
            "written": written,
        });
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        let _ = writeln!(out, "{text}");
        return;
    }
    let lang = options.lang;
    let _ = writeln!(
        out,
        "result: {}",
        if run.ok {
            "carried out"
        } else {
            "stopped, nothing changed"
        }
    );
    let _ = writeln!(
        out,
        "commands: {} carried out, {} skipped; operations: {}",
        run.executed, run.skipped, run.operations
    );
    for d in &run.warnings {
        let _ = writeln!(out, "warning: {}", d.message(lang));
    }
    for d in &run.diagnostics {
        let _ = writeln!(out, "error: {}", d.message(lang));
    }
    for m in &run.messages {
        let _ = write!(out, "{m}");
    }
    if written.is_empty() {
        let _ = writeln!(out, "dry run: nothing was written");
    }
    for path in written {
        let _ = writeln!(out, "written: {path}");
    }
}

fn run_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(d) => {
            let _ = writeln!(err, "error: {}", d.message(Lang::En));
            return usage(err);
        }
    };
    let lang = options.lang;
    let [script_path, document_path] = options.files.as_slice() else {
        return usage(err);
    };
    let settings = match load_settings(options.config.as_deref()) {
        Ok(s) => s,
        Err(d) => return fail(err, &d, lang),
    };
    let apply = options.out.is_some() && !options.dry_run;
    if let Some(target) = &options.out {
        if apply && Path::new(target).exists() {
            return fail(
                err,
                &Diag::plain(
                    FILE_FAILED,
                    format!("{target} exists and is never replaced"),
                ),
                lang,
            );
        }
    }
    let script_text = match std::fs::metadata(script_path) {
        Ok(m) if m.len() > settings.max_script_bytes as u64 => {
            return fail(
                err,
                &Diag::plain(
                    INPUT_UNREADABLE,
                    format!(
                        "{script_path} is larger than {} bytes",
                        settings.max_script_bytes
                    ),
                ),
                lang,
            )
        }
        Ok(_) => match std::fs::read_to_string(script_path) {
            Ok(text) => text,
            Err(e) => {
                return fail(
                    err,
                    &Diag::plain(INPUT_UNREADABLE, format!("{script_path}: {e}")),
                    lang,
                )
            }
        },
        Err(e) => {
            return fail(
                err,
                &Diag::plain(INPUT_UNREADABLE, format!("{script_path}: {e}")),
                lang,
            )
        }
    };
    let document = match load_document(document_path) {
        Ok(d) => d,
        Err(d) => return fail(err, &d, lang),
    };
    let rules = match load_rules(&options, document_path) {
        Ok(r) => r,
        Err(d) => return fail(err, &d, lang),
    };
    let parsed = parse_script(&script_text, &settings);
    let base = Path::new(script_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut ctx = RunContext::new(&settings);
    ctx.rules = rules;
    ctx.lang = lang;
    ctx.base_dir = Some(base);
    let run = match &document {
        Document::Board(board) => run_board(board, &parsed, &ctx),
        Document::Schematic(schematic) => run_schematic(schematic, &parsed, &ctx),
    };
    if !apply {
        let _ = writeln!(
            err,
            "note: dry run, nothing is written (give --out FILE to write the result)"
        );
    }
    let mut written: Vec<String> = Vec::new();
    if apply && run.ok {
        if let (Some(target), Some(board)) = (&options.out, &run.board) {
            let result = board
                .to_bytes()
                .map_err(|e| Diag::plain(FILE_FAILED, e.to_string()))
                .and_then(|bytes| write_new(Path::new(target), &bytes));
            match result {
                Ok(()) => written.push(target.clone()),
                Err(d) => return fail(err, &d, lang),
            }
        } else if options.out.is_some() {
            return fail(
                err,
                &bad("a schematic is not changed: --out is for a board"),
                lang,
            );
        }
        match commit_writes(&run) {
            Ok(()) => written.extend(
                run.writes
                    .iter()
                    .map(|w| w.path.to_string_lossy().into_owned()),
            ),
            Err(d) => return fail(err, &d, lang),
        }
    }
    run_summary(&run, &options, &written, out);
    if run.ok {
        0
    } else {
        1
    }
}

fn report_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(d) => {
            let _ = writeln!(err, "error: {}", d.message(Lang::En));
            return usage(err);
        }
    };
    let lang = options.lang;
    let [target] = options.files.as_slice() else {
        return usage(err);
    };
    let settings = match load_settings(options.config.as_deref()) {
        Ok(s) => s,
        Err(d) => return fail(err, &d, lang),
    };
    let folder = match analyse_path(Path::new(target), &settings) {
        Ok(f) => f,
        Err(d) => return fail(err, &d, lang),
    };
    if let Some(page) = &options.html {
        if let Err(d) = write_new(Path::new(page), folder.to_html(lang).as_bytes()) {
            return fail(err, &d, lang);
        }
        let _ = writeln!(out, "written: {page}");
    }
    let text = if options.json {
        folder.to_json()
    } else {
        folder.to_text(lang)
    };
    let _ = out.write_all(text.as_bytes());
    0
}

pub(crate) fn script_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match args.split_first() {
        Some((&"run", tail)) => run_command(tail, out, err),
        Some((&"report", tail)) => report_command(tail, out, err),
        _ => usage(err),
    }
}

pub(crate) fn cam_import_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(d) => {
            let _ = writeln!(err, "error: {}", d.message(Lang::En));
            return usage(err);
        }
    };
    let lang = options.lang;
    let [job_path] = options.files.as_slice() else {
        return usage(err);
    };
    let settings = match load_settings(options.config.as_deref()) {
        Ok(s) => s,
        Err(d) => return fail(err, &d, lang),
    };
    let length = match std::fs::metadata(job_path) {
        Ok(m) => m.len(),
        Err(e) => {
            return fail(
                err,
                &Diag::plain(INPUT_UNREADABLE, format!("{job_path}: {e}")),
                lang,
            )
        }
    };
    if length > settings.max_cam_bytes as u64 {
        return fail(
            err,
            &Diag::plain(
                INPUT_UNREADABLE,
                format!("{job_path} is larger than {} bytes", settings.max_cam_bytes),
            ),
            lang,
        );
    }
    let bytes = match std::fs::read(job_path) {
        Ok(b) => b,
        Err(e) => {
            return fail(
                err,
                &Diag::plain(INPUT_UNREADABLE, format!("{job_path}: {e}")),
                lang,
            )
        }
    };
    let name = Path::new(job_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let job = match import_cam(&name, &String::from_utf8_lossy(&bytes), &settings) {
        Ok(job) => job,
        Err(d) => return fail(err, &d, lang),
    };
    if let Some(target) = &options.out {
        if let Err(d) = write_new(Path::new(target), job.profile_json().as_bytes()) {
            return fail(err, &d, lang);
        }
        let _ = writeln!(out, "written: {target}");
    }
    let text = if options.json {
        job.to_json()
    } else {
        job.to_text(lang)
    };
    let _ = out.write_all(text.as_bytes());
    0
}
