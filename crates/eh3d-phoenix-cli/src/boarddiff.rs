// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `boarddiff`: the comparison of two revisions of a board, listing what the new one
//! has that the old one has not, what it lost and what it changed.
//!
//! Exit codes: 0 the boards do not differ in anything compared, 1 they differ, 2 the arguments or
//! the input are unusable. Nothing is written: the report goes to the output stream.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_boarddiff::codes::{INPUT_UNREADABLE, OPTION_INVALID};
use eh3d_phoenix_boarddiff::{compare_boards, report_json, report_text, DiffError, DiffSettings};
use eh3d_phoenix_rules::Lang;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/boarddiff.cfg";

pub(crate) const USAGE: &str =
    "phoenix boarddiff [--config FILE] [--json] [--lang CODE] <old board> <new board>

  boarddiff     the comparison of two revisions of a board: the layers, nets and parts that only
                one has, the parts with another value, footprint, place, turn or side, the pins
                on another net, the copper of each net on each layer (the number and the length of
                the tracks), the vias, the zones and the outline, in a fixed order, with the
                codes PHOENIX-DIFF-<NNN>. Parts are matched by reference and pins by reference
                and pad number, never by the ids of the objects; exit code 1 when they differ.
                A prototype, read-only
  --config      the tolerances, the kinds of object compared, the parts and nets left out and the
                limits (default: config/boarddiff.cfg when it exists)
  --json        print the report as JSON, the same every time
  --lang CODE   the words in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, DiffError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        files: Vec::new(),
    };
    let bad = |what: &str| DiffError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, DiffError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => options.config = Some(value("--config needs a file")?),
            other if other.starts_with("--") => {
                return Err(DiffError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<DiffSettings, DiffError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(DiffSettings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| DiffError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            DiffSettings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &DiffError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

fn boarddiff(options: &Options, out: &mut dyn Write) -> Result<i32, DiffError> {
    let [old_path, new_path] = options.files.as_slice() else {
        return Err(DiffError::new(
            OPTION_INVALID,
            "give exactly two board files, the old one first",
        ));
    };
    let settings = load_settings(options.config.as_deref())?;
    let old = read_board_file(old_path).map_err(|e| DiffError::new(INPUT_UNREADABLE, e))?;
    let new = read_board_file(new_path).map_err(|e| DiffError::new(INPUT_UNREADABLE, e))?;
    let report = compare_boards(&old, &new, &settings)?;
    let text = if options.json {
        report_json(&report)
    } else {
        report_text(&report, options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(report.differs()))
}

pub(crate) fn boarddiff_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    match boarddiff(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
