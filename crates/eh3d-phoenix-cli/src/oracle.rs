// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `oracle`: run the reference cases of the suite against the real calculations, or
//! list them.
//!
//! Exit codes: 0 every case that can run is inside its tolerance (pending cases are listed and do
//! not count), 1 a case is outside its tolerance or failed, 2 the arguments or the input are
//! unusable. The command starts no other program and writes nothing unless `--out` names a new
//! file; an existing file is never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_oracle::codes::{OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_oracle::{bundled_files, list_cases, read_dir, run_files, CaseFile, OracleError};
use eh3d_phoenix_rules::Lang;

pub(crate) const USAGE: &str = "\
phoenix oracle run  [--dir FOLDER] [--suite NAME] [--out FILE] [--json] [--lang CODE]
phoenix oracle list [--dir FOLDER] [--suite NAME]

  run           run the reference cases (published values with their source and tolerance)
                against the real calculations of the stack-up, RF, simulator, rules, lengths
                and zones crates and report each case as ok, outside the tolerance, failed or
                pending. No other program is started and nothing is written unless --out names
                a new file (an existing file is never replaced). Exit 1 when a case is outside
                its tolerance or failed. Passing the cases does not compare the calculations
                with EAGLE, KiCad, a field solver or a measurement
  list          list the cases, one per line: id, status, area and title
  --dir         read the .json files of a folder instead of the cases built into the program
  --suite       only the cases of one suite
  --out         write the JSON report to a new file
  --lang CODE   the words and messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    dir: Option<String>,
    suite: Option<String>,
    out: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, OracleError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        dir: None,
        suite: None,
        out: None,
    };
    let bad = |what: &str| OracleError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, OracleError> {
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
            "--dir" => options.dir = Some(value("--dir needs a folder")?),
            "--suite" => options.suite = Some(value("--suite needs a name")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            other => {
                return Err(OracleError::new(
                    OPTION_INVALID,
                    format!("unknown option or argument {other}"),
                ))
            }
        }
    }
    Ok(options)
}

fn load(options: &Options) -> Result<Vec<CaseFile>, OracleError> {
    match &options.dir {
        Some(dir) => read_dir(Path::new(dir)),
        None => bundled_files(),
    }
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), OracleError> {
    let failed = |e: std::io::Error| OracleError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn run(options: &Options, out: &mut dyn Write) -> Result<i32, OracleError> {
    let files = load(options)?;
    let report = run_files(&files, options.suite.as_deref());
    if let Some(path) = &options.out {
        write_new(path, &report.to_json())?;
    }
    let text = if options.json {
        report.to_json()
    } else {
        let mut text = report.to_text(options.lang);
        if let Some(path) = &options.out {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(report.has_problems()))
}

fn list(options: &Options, out: &mut dyn Write) -> Result<i32, OracleError> {
    let files = load(options)?;
    let _ = out.write_all(list_cases(&files, options.suite.as_deref()).as_bytes());
    Ok(0)
}

pub(crate) fn oracle_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some((&action, tail)) = args.split_first() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    if action != "run" && action != "list" {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(tail) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Lang::En));
            return 2;
        }
    };
    let result = if action == "run" {
        run(&options, out)
    } else {
        list(&options, out)
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}
