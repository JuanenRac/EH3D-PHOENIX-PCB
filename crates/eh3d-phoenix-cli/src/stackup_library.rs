// SPDX-License-Identifier: GPL-3.0-or-later
//! The library of typical stack-ups of `phoenix stackup --library`: the list of the stack-ups by
//! number of layers, and the impedance of the lines of one of them (a microstrip on each outer
//! layer, a stripline on each inner one) for a width or for a target impedance, with the same
//! report as a job file.
//!
//! Exit codes: 0 done, 1 a line could not be computed, 2 the arguments or the settings are
//! unusable. The figures of the library are not verified with any maker.

use std::io::Write;

use eh3d_phoenix_stackup::libstack::{Request, StackLibrary};
use eh3d_phoenix_stackup::{report_json, report_text, Language, StackupSettings};
use serde_json::{json, Value};

pub(crate) const USAGE: &str = "\
phoenix stackup --library list [--layers N] [--json]
phoenix stackup --library <ID> [--width-um W | --target-ohm Z] [--freq-ghz GHz] [--settings FILE] [--lang xx] [--json]
";

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    let _ = err.write_all(USAGE.as_bytes());
    2
}

fn mm(nm: i64) -> String {
    format!("{}.{:03}", nm / 1_000_000, (nm % 1_000_000) / 1000)
}

pub(crate) fn library_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut library: Option<&str> = None;
    let mut layers: Option<usize> = None;
    let mut width_um: Option<f64> = None;
    let mut target: Option<f64> = None;
    let mut freq = 1.0f64;
    let mut settings_path: Option<&str> = None;
    let mut language = Language::En;
    let mut json = false;
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--json" => json = true,
            "--library" | "--layers" | "--width-um" | "--target-ohm" | "--freq-ghz"
            | "--settings" | "--lang" => {
                let Some(value) = iter.next() else {
                    return fail(err, &format!("{arg} needs a value"));
                };
                match arg {
                    "--library" => library = Some(value),
                    "--settings" => settings_path = Some(value),
                    "--lang" => match Language::from_tag(value) {
                        Some(found) => language = found,
                        None => return fail(err, "--lang is not a language of the suite"),
                    },
                    "--layers" => match value.parse::<usize>() {
                        Ok(n) => layers = Some(n),
                        Err(_) => return fail(err, "--layers needs a number"),
                    },
                    _ => {
                        let Some(number) = value.parse::<f64>().ok().filter(|v| v.is_finite())
                        else {
                            return fail(err, &format!("{arg} needs a number"));
                        };
                        match arg {
                            "--width-um" => width_um = Some(number),
                            "--target-ohm" => target = Some(number),
                            _ => freq = number,
                        }
                    }
                }
            }
            other => return fail(err, &format!("{other} does not apply to --library")),
        }
    }
    let Some(which) = library else {
        return fail(err, "--library needs list or the identifier of a stack-up");
    };
    let lib = match StackLibrary::builtin() {
        Ok(lib) => lib,
        Err(error) => {
            let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
            return 2;
        }
    };
    if which == "list" {
        if width_um.is_some() || target.is_some() || settings_path.is_some() {
            return fail(err, "list takes only --layers and --json");
        }
        let stacks: Vec<_> = lib
            .stacks
            .iter()
            .filter(|s| layers.is_none_or(|n| s.layers == n))
            .collect();
        if json {
            let list: Vec<Value> = stacks
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "layers": s.layers,
                        "nominal_um": s.nominal_um,
                        "total_nm": s.total_nm,
                        "source": s.source,
                    })
                })
                .collect();
            let value = json!({
                "verified": lib.verified,
                "source": lib.source,
                "notes": lib.notes,
                "stackups": list,
            });
            let _ = writeln!(
                out,
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
        } else {
            let _ = writeln!(out, "id layers total_mm sold_for_mm");
            for s in &stacks {
                let _ = writeln!(
                    out,
                    "{} {} {} {}",
                    s.id,
                    s.layers,
                    mm(s.total_nm),
                    mm(s.nominal_um * 1000)
                );
            }
            let _ = writeln!(out, "not verified with the maker: {}", lib.source);
            for note in &lib.notes {
                let _ = writeln!(out, "note: {note}");
            }
        }
        return 0;
    }
    if layers.is_some() {
        return fail(err, "--layers is for list");
    }
    let request = match (width_um, target) {
        (Some(_), Some(_)) => return fail(err, "give --width-um or --target-ohm, not both"),
        (Some(w), None) => {
            if !(0.001..=1_000_000.0).contains(&w) {
                return fail(err, "--width-um is not between 0.001 and 1000000");
            }
            Request::Width((w * 1000.0).round() as i64)
        }
        (None, Some(z)) => Request::Target(z),
        (None, None) => Request::Target(50.0),
    };
    let settings = match settings_path {
        None => StackupSettings::default(),
        Some(path) => match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| StackupSettings::parse(&text).map_err(|e| e.to_string()))
        {
            Ok(settings) => settings,
            Err(message) => {
                let _ = writeln!(err, "error: cannot use the settings: {message}");
                return 2;
            }
        },
    };
    let job = match lib.job(which, request) {
        Ok(job) => job,
        Err(error) => {
            let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
            return 2;
        }
    };
    let outcomes = job.run(freq, &settings);
    let text = if json {
        let mut shown = report_json(&job.stackup.name, freq, &outcomes, language);
        shown.push('\n');
        shown
    } else {
        let mut shown = report_text(&job.stackup.name, freq, &outcomes, language);
        shown.push_str(&format!(
            "stack-up library: the figures are not verified with the maker ({})\n",
            lib.source
        ));
        shown
    };
    let _ = out.write_all(text.as_bytes());
    i32::from(outcomes.iter().any(|o| o.result.is_err()))
}
