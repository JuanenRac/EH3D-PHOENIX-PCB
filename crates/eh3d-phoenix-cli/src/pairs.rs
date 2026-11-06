// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `pairs`: checks the differential pairs of a board (the gap, the symmetry, the
//! uncoupled stretch, the lengths, the vias, the width and, with a stack-up, the impedance). It is
//! a dry run: nothing is ever written.
//!
//! Exit codes: 0 no error is proven, 1 a check of a pair is proven broken, 2 the arguments or the
//! input are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_boardedit::codes::{BoardEditError, OPERATIONS_INVALID};
use eh3d_phoenix_boardedit::pair_codes::PAIRS_INVALID;
use eh3d_phoenix_boardedit::{check_pairs, PairConfig};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::StackupJob;

use crate::check::read_text;
use crate::route::load_board;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/diffpairs.cfg";

pub(crate) const USAGE: &str = "\
phoenix pairs [--config FILE] [--pairs FILE] [--stackup FILE] [--json] [--lang CODE] <board>

  pairs         check the differential pairs of a board (an EAGLE .brd file or the JSON of the
                design model): the gap between the two tracks, the symmetry of the coupling, the
                length that runs uncoupled, the difference of the lengths, the vias that have no
                partner, the width of the tracks and, for a pair with a target and with a stack-up,
                the differential impedance. Every finding has a code PHOENIX-PAIR-<NNN>. A dry run:
                nothing is written. A prototype tried on boards made by hand only
  --config      the settings and the pairs (default: config/diffpairs.cfg when it exists)
  --pairs       a JSON file of pairs (format phoenix-diffpairs/1); a pair of it wins over a pair of
                the settings file with the same name
  --stackup     a stack-up job file (the file of `phoenix stackup`) for the differential impedance
  --json        print the report as JSON. --lang CODE writes the titles in en, es, de, fr, it, ja,
                zh or pt. Exit code: 0 no error proven, 1 an error is proven, 2 unusable input
";

struct Options {
    config: Option<String>,
    pairs: Option<String>,
    stackup: Option<String>,
    json: bool,
    lang: Lang,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        config: None,
        pairs: None,
        stackup: None,
        json: false,
        lang: Lang::En,
        file: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--config" => {
                let value = iter.next().ok_or("--config needs a file")?;
                options.config = Some((*value).to_owned());
            }
            "--pairs" => {
                let value = iter.next().ok_or("--pairs needs a file")?;
                options.pairs = Some((*value).to_owned());
            }
            "--stackup" => {
                let value = iter.next().ok_or("--stackup needs a file")?;
                options.stackup = Some((*value).to_owned());
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

/// The settings and the pairs: the settings file (the given one, or `config/diffpairs.cfg` when it
/// exists, or the built-in default) with the pairs of a JSON file on top.
pub(crate) fn load_pair_config(
    config: Option<&str>,
    pairs: Option<&str>,
) -> Result<PairConfig, BoardEditError> {
    let path = config.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    let mut loaded = match path {
        None => PairConfig::default(),
        Some(path) => {
            let text = read_text(path).map_err(|m| BoardEditError::new(PAIRS_INVALID, m))?;
            PairConfig::parse(&text)?
        }
    };
    if let Some(path) = pairs {
        let text = read_text(path).map_err(|m| BoardEditError::new(OPERATIONS_INVALID, m))?;
        loaded.merge_json(&text)?;
    }
    Ok(loaded)
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

/// The command `pairs`.
pub(crate) fn pairs_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let Some(file) = options.file.as_deref() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let config = match load_pair_config(options.config.as_deref(), options.pairs.as_deref()) {
        Ok(config) => config,
        Err(e) => return fail(err, &e.message(options.lang)),
    };
    let board = match load_board(file) {
        Ok(board) => board,
        Err(e) => return fail(err, &e.to_string()),
    };
    let job = match options.stackup.as_deref() {
        None => None,
        Some(path) => {
            let parsed = read_text(path)
                .and_then(|text| StackupJob::parse(&text).map_err(|e| format!("{path}: {e}")));
            match parsed {
                Ok(job) => Some(job),
                Err(message) => return fail(err, &message),
            }
        }
    };
    let report = match check_pairs(&board, &config, job.as_ref()) {
        Ok(report) => report,
        Err(e) => return fail(err, &e.message(options.lang)),
    };
    if options.json {
        let _ = writeln!(out, "{}", report.to_json());
    } else {
        let _ = out.write_all(report.to_text(options.lang).as_bytes());
    }
    i32::from(report.has_errors())
}
