// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `schedit`: applies a file of editing operations to a schematic. It is a dry run
//! unless `--out` names a new file, and it never writes over an input file or over a file that
//! exists.
//!
//! Exit codes: 0 done and the electrical rule check proves no error, 1 done and it proves an error,
//! 2 the arguments, the input, an operation or the output are unusable.

use std::collections::BTreeMap;
use std::io::Write;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle_board::write_schematic;
use eh3d_phoenix_erc::{check, ErcConfig};
use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_schedit::codes::{SchEditError, OPERATIONS_INVALID, OUTPUT_NOT_ALLOWED};
use eh3d_phoenix_schedit::{apply_operations, read_operations};
use serde_json::json;

use crate::check::{id_source, read_text};
use crate::design::{has_extension, load_schematic, same_file, write_atomic};

pub(crate) const USAGE: &str = "\
phoenix schedit [--erc-config FILE] [--json] [--lang CODE] --apply OPERATIONS.json [--out FILE]
                <schematic>

  schedit       apply a file of editing operations (place, move, turn, mirror and delete parts,
                copy and paste, values and references, wires, net labels, power ports, bus labels)
                to a schematic (an EAGLE .sch file or the JSON of the design model). All or
                nothing: the first operation that is refused ends the run and names its number.
                After the operations the nets are made again from what the drawing joins, and the
                electrical rule check is run on the result. It is a dry run: nothing is written
                unless --out names a new file (.json for the design model, .sch for EAGLE); an
                input file, or a file that exists, is never written over
  --apply       the file of operations: a JSON list, or an object with the list under \"ops\"
  --erc-config  the settings of the rule check (config/erc.cfg shows every key)
  --json        print the report as JSON. --lang CODE writes the rule check in en, es, de, fr, it,
                ja, zh or pt. Exit code: 0 no error proven, 1 an error is proven by the rule check,
                2 unusable input
";

struct Options {
    apply: Option<String>,
    out: Option<String>,
    erc_config: Option<String>,
    json: bool,
    lang: Lang,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        apply: None,
        out: None,
        erc_config: None,
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
            "--erc-config" => {
                let value = iter.next().ok_or("--erc-config needs a file")?;
                options.erc_config = Some((*value).to_owned());
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => {
                if options.file.is_some() {
                    return Err("only one schematic".to_owned());
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
fn check_output(path: &str, inputs: &[&str]) -> Result<(), SchEditError> {
    let refuse = |why: String| SchEditError::new(OUTPUT_NOT_ALLOWED, why);
    if inputs.iter().any(|input| same_file(path, input)) {
        return Err(refuse(format!("{path} is an input file")));
    }
    if std::path::Path::new(path).exists() {
        return Err(refuse(format!("{path} exists; give a new file")));
    }
    if !has_extension(path, "json") && !has_extension(path, "sch") {
        return Err(refuse(format!(
            "{path}: only .json (design model) and .sch (EAGLE) are written"
        )));
    }
    Ok(())
}

/// The command `schedit`.
pub(crate) fn schedit_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
        if let Err(e) = check_output(path, &[file, ops_path]) {
            return fail(err, &e.to_string());
        }
    }
    let erc_config = match &options.erc_config {
        Some(path) => match read_text(path)
            .and_then(|text| ErcConfig::parse(&text).map_err(|e| format!("{path}: {e}")))
        {
            Ok(config) => config,
            Err(message) => return fail(err, &message),
        },
        None => ErcConfig::default(),
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let schematic = match load_schematic(file, &BTreeMap::new(), &mut new_id) {
        Ok(schematic) => schematic,
        Err(message) => return fail(err, &message),
    };
    let ops = match read_text(ops_path)
        .map_err(|m| SchEditError::new(OPERATIONS_INVALID, m))
        .and_then(|text| read_operations(&text))
    {
        Ok(ops) => ops,
        Err(e) => return fail(err, &e.to_string()),
    };
    let (result, report) = match apply_operations(&schematic, &ops) {
        Ok(done) => done,
        Err(e) => return fail(err, &e.to_string()),
    };
    let erc = check(&result, &erc_config);

    let mut written: Option<String> = None;
    if let Some(path) = options.out.as_deref() {
        let bytes = if has_extension(path, "sch") {
            write_schematic(&Board::default(), &result)
                .text
                .into_bytes()
        } else {
            match result.to_bytes() {
                Ok(bytes) => bytes,
                Err(e) => return fail(err, &format!("cannot write the schematic: {e:?}")),
            }
        };
        if let Err(message) = write_atomic(path, &bytes, 5, 50) {
            return fail(err, &message);
        }
        written = Some(path.to_owned());
    }

    if options.json {
        let erc_value = serde_json::from_str::<serde_json::Value>(&erc.to_json())
            .unwrap_or(serde_json::Value::Null);
        let value = json!({
            "schema": "phoenix-schedit/1",
            "applied": report.applied,
            "created": report.created.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "parts": [report.parts.0, report.parts.1],
            "wires": [report.wires.0, report.wires.1],
            "nets": [report.nets.0, report.nets.1],
            "written": written,
            "erc": erc_value,
        });
        let _ = writeln!(out, "{value}");
    } else {
        let _ = out.write_all(report.to_text().as_bytes());
        let _ = out.write_all(erc.to_text(options.lang).as_bytes());
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
    if erc.count(Severity::Error) > 0 {
        1
    } else {
        0
    }
}
