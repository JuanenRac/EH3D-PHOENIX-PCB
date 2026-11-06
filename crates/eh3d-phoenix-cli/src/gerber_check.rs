// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `gerber-check`: check the structure of Gerber X2 and Excellon files and report
//! what is wrong with codes `PHOENIX-GX2-<NNN>`.
//!
//! The files are only read. Exit codes: 0 done and no error found (warnings do not count), 1 a
//! file has an error or could not be read, 2 the arguments or the settings are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_gerber::codes::{
    Finding, GbrError, INPUT_TOO_LARGE, INPUT_UNREADABLE, OPTION_INVALID,
};
use eh3d_phoenix_gerber::x2check::{check_text, has_errors};
use eh3d_phoenix_gerber::ImportSettings;
use eh3d_phoenix_rules::Lang;
use serde_json::json;

use crate::gerber_import::{collect, load_settings};

pub(crate) const USAGE: &str = "\
phoenix gerber-check [--config FILE] [--json] [--lang CODE] <file or folder> ...

  gerber-check  check the structure of Gerber X2 and Excellon files: the end command, the format
                before the drawing, apertures and tools defined before they are used, regions
                closed, attributes set before they are deleted, the names, values and characters
                of attributes and the file attributes; the files are only read; the codes are
                PHOENIX-GX2-nnn; a prototype that has not been compared with any manufacturer's
                tool
  --config      the limits (default: config/gerber_import.cfg when it exists)
  --json        print the findings as JSON
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

#[derive(Default)]
struct Options {
    json: bool,
    lang: Option<Lang>,
    config: Option<String>,
    inputs: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, GbrError> {
    let mut o = Options::default();
    let bad = |what: &str| GbrError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => o.json = true,
            "--lang" => {
                let tag = iter.next().ok_or_else(|| bad("--lang needs a value"))?;
                o.lang = Some(Lang::from_tag(tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--config" => {
                let path = iter.next().ok_or_else(|| bad("--config needs a value"))?;
                o.config = Some((*path).to_owned());
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => o.inputs.push(other.to_owned()),
        }
    }
    Ok(o)
}

/// The findings of one file, or the reason it could not be read.
fn check_file(path: &Path, settings: &ImportSettings) -> Result<Vec<Finding>, GbrError> {
    let length = std::fs::metadata(path)
        .map_err(|e| GbrError::new(INPUT_UNREADABLE, format!("{}: {e}", path.display())))?
        .len();
    if length > settings.max_file_bytes as u64 {
        return Err(GbrError::new(
            INPUT_TOO_LARGE,
            format!("{length} bytes, the limit is {}", settings.max_file_bytes),
        ));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| GbrError::new(INPUT_UNREADABLE, format!("{}: {e}", path.display())))?;
    Ok(check_text(&String::from_utf8_lossy(&bytes)))
}

pub(crate) fn gerber_check_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang.unwrap_or(Lang::En);
    if options.inputs.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let settings = match load_settings(options.config.as_deref()) {
        Ok(settings) => settings,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let entries = match collect(&options.inputs, &settings) {
        Ok(entries) => entries,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let results: Vec<(String, Result<Vec<Finding>, GbrError>)> = entries
        .into_iter()
        .map(|(name, path)| (name, check_file(&path, &settings)))
        .collect();
    let failed = results
        .iter()
        .filter(|(_, r)| match r {
            Ok(findings) => has_errors(findings),
            Err(_) => true,
        })
        .count();
    if options.json {
        let files: Vec<serde_json::Value> = results
            .iter()
            .map(|(name, result)| match result {
                Ok(findings) => json!({
                    "name": name,
                    "findings": findings.iter().map(|f| json!({
                        "code": f.code,
                        "severity": f.severity.word(),
                        "line": f.line,
                        "detail": f.detail,
                    })).collect::<Vec<_>>(),
                }),
                Err(error) => json!({
                    "name": name,
                    "error": { "code": error.code, "detail": error.detail },
                }),
            })
            .collect();
        let value = json!({ "files": files, "failed": failed });
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    } else {
        for (name, result) in &results {
            match result {
                Ok(findings) if findings.is_empty() => {
                    let _ = writeln!(out, "{name}: no finding");
                }
                Ok(findings) => {
                    let _ = writeln!(out, "{name}: {} finding(s)", findings.len());
                    for finding in findings {
                        let _ = writeln!(out, "  {}", finding.message(lang));
                    }
                }
                Err(error) => {
                    let _ = writeln!(out, "{name}: error {}", error.message(lang));
                }
            }
        }
        let _ = writeln!(out, "{} files, {failed} with errors", results.len());
    }
    i32::from(failed > 0)
}
