// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `check` and `connectivity`: read EAGLE files, run the rule engine or the
//! connectivity analysis, and print the report as text or JSON.
//!
//! Exit codes: 0 nothing is proven wrong, 1 at least one error is proven, 2 the arguments or the
//! input are unusable.

use std::collections::BTreeMap;
use std::io::Write;

use eh3d_phoenix_eagle_board::{read_board, read_schematic};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, Nm, ObjectId, StepClock};
use eh3d_phoenix_rules::{
    check_board, check_classes, check_connectivity, Lang, NetClasses, Report, RuleFile, RuleValues,
};

/// The largest input file that is read, in bytes.
const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix check [--rules FILE] [--json] [--lang CODE] <board.brd>

  check         run the design rules on an EAGLE board. The limits come from FILE (a JSON rule
                file, format phoenix-rules/1 or phoenix-rules/2, which adds net classes with
                limits of their own for tracks and vias) or, without --rules, from the
                <designrules> of the board. The report keeps confirmed and inconclusive findings
                in two lists

phoenix connectivity [--rules FILE] [--json] [--lang CODE] <schematic.sch> <board.brd>

  connectivity  the copper of the board (shorts, unrouted nets) and the board against the schematic
                (missing nets, unconnected pins, joined nets, names). The tolerance for copper
                that touches comes from the rule file

  --json        print the report as JSON. --lang CODE writes the text in en, es, de, fr, it, ja,
                zh or pt. Exit code 0: nothing proven wrong; 1: an error is proven; 2: unusable input
";

struct Options {
    json: bool,
    lang: Lang,
    rules: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        rules: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            "--rules" => {
                let path = iter.next().ok_or("--rules needs a file")?;
                options.rules = Some((*path).to_owned());
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

pub(crate) fn read_text(path: &str) -> Result<String, String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("cannot read {path}: {e}"))?
        .len();
    if size > MAX_INPUT_BYTES {
        return Err(format!(
            "{path} is {size} bytes, the limit is {MAX_INPUT_BYTES}"
        ));
    }
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

/// Ids for the objects read from a file: the same file always gives the same ids.
pub(crate) fn id_source() -> Result<impl FnMut() -> ObjectId, String> {
    let mut generator = IdGenerator::new(
        StepClock::new(1_700_000_000_000, 1),
        DeterministicRandom::new(1),
    );
    let mut last = generator.generate().map_err(|e| e.to_string())?;
    Ok(move || {
        if let Ok(next) = generator.generate() {
            last = next;
        }
        last
    })
}

fn finish(report: &Report, options: &Options, out: &mut dyn Write) -> i32 {
    let text = if options.json {
        report.to_json()
    } else {
        report.to_text(options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    if report.passes() {
        0
    } else {
        1
    }
}

fn usage(err: &mut dyn Write) -> i32 {
    let _ = err.write_all(USAGE.as_bytes());
    2
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

pub(crate) fn check(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return usage(err);
        }
    };
    let [board_path] = options.files.as_slice() else {
        return usage(err);
    };
    let text = match read_text(board_path) {
        Ok(text) => text,
        Err(message) => return fail(err, &message),
    };
    let (rules, classes) = match &options.rules {
        Some(path) => {
            match read_text(path).and_then(|t| RuleFile::from_json(&t).map_err(|e| e.to_string())) {
                Ok(file) => (file.values, file.classes),
                Err(message) => return fail(err, &message),
            }
        }
        None => match RuleValues::from_eagle_board(&text) {
            Ok((values, notes)) => {
                for note in notes {
                    let _ = writeln!(err, "note: {}", note.message);
                }
                (values, NetClasses::default())
            }
            Err(error) => {
                return fail(
                    err,
                    &format!("{error}; give the limits with --rules FILE (a JSON rule file)"),
                )
            }
        },
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let read = match read_board(&text, &mut new_id) {
        Ok(read) => read,
        Err(diagnostic) => return fail(err, &diagnostic.message),
    };
    if !read.diagnostics.is_empty() {
        let _ = writeln!(
            err,
            "note: {} parts of the board file were not read or were read in a simpler form",
            read.diagnostics.len()
        );
    }
    let mut report = check_board(&read.board, &rules);
    if !classes.is_empty() {
        report.merge(check_classes(&read.board, &classes));
    }
    finish(&report, &options, out)
}

pub(crate) fn connectivity(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return usage(err);
        }
    };
    let [schematic_path, board_path] = options.files.as_slice() else {
        return usage(err);
    };
    let tolerance = match &options.rules {
        Some(path) => match read_text(path)
            .and_then(|t| RuleValues::from_json(&t).map_err(|e| e.to_string()))
        {
            Ok((_, values)) => values.connect_tolerance,
            Err(message) => return fail(err, &message),
        },
        None => Nm::ZERO,
    };
    let (schematic_text, board_text) = match (read_text(schematic_path), read_text(board_path)) {
        (Ok(s), Ok(b)) => (s, b),
        (Err(message), _) | (_, Err(message)) => return fail(err, &message),
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let board = match read_board(&board_text, &mut new_id) {
        Ok(read) => read.board,
        Err(diagnostic) => return fail(err, &diagnostic.message),
    };
    let footprints: BTreeMap<String, ObjectId> = board
        .footprints
        .iter()
        .map(|(id, footprint)| (footprint.name.clone(), *id))
        .collect();
    let schematic = match read_schematic(&schematic_text, &footprints, &mut new_id) {
        Ok(read) => read.schematic,
        Err(diagnostic) => return fail(err, &diagnostic.message),
    };
    let report = check_connectivity(&schematic, &board, tolerance);
    finish(&report, &options, out)
}
