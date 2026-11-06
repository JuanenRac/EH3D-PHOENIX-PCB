// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `netlist`: the simulation netlist of a schematic, made from a table of models with
//! no netlist written by hand. It is a dry run that prints the netlist, and writes it only to a
//! new file named with `--out`: an input file, or a file that exists, is never written over.
//!
//! The same preparation (read the schematic, run the electrical rule check, make the netlist) is
//! used by `sim --schematic`, which does not write the netlist at all.
//!
//! Exit codes: 0 done and nothing stops a simulation, 1 done and the report has an error (a part
//! without a usable model, a source in short circuit, no ground, an error of the rule check), 2
//! the arguments, the input or the output are unusable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_erc::{check, ErcConfig, Report as ErcReport};
use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_sim::netcodes::{GenIssue, ERC_ERRORS, OUTPUT_REFUSED};
use eh3d_phoenix_sim::{generate_netlist, ModelTable, NetlistText, SimSettings};
use serde_json::json;

use crate::check::{id_source, read_text};
use crate::design::{has_extension, load_schematic, same_file, write_atomic};

pub(crate) const USAGE: &str = "\
phoenix netlist [--models FILE] [--out NEW.cir] [--json] [--lang CODE] [--config FILE]
                [--erc-config FILE] [--no-erc] <schematic>

  netlist       make the simulation netlist of a schematic (an EAGLE .sch file, a KiCad
                .kicad_sch file or the JSON of the design model) from a table of models: the
                model of each part comes from a rule by value, by symbol name or by the prefix of
                its designator, or from the attributes of the part; values are read as 4k7, 1u5
                or 2M2; nets get names that do not change when something else is edited. The
                electrical rule check runs first. Everything that cannot be simulated is listed
                with a code PHOENIX-NETLIST-<NNN>. A dry run: the netlist is printed and nothing
                is written unless --out names a new file (an input file, or a file that exists,
                is never written over)
  --models      the table of models (config/sim_models.cfg shows every kind of line and is the
                default); the files it includes are looked for in its folder
  --config      the settings of the simulator (config/sim.cfg), for the names of the ground nets
  --erc-config  the settings of the rule check (config/erc.cfg); --no-erc skips the check
  --json        print the netlist and the report as JSON. --lang CODE writes the report in en, es,
                de, fr, it, ja, zh or pt. Exit code: 0 nothing stops a simulation, 1 the report
                has an error, 2 unusable input
";

struct Options {
    models: Option<String>,
    out: Option<String>,
    config: Option<String>,
    erc_config: Option<String>,
    no_erc: bool,
    json: bool,
    lang: Lang,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        models: None,
        out: None,
        config: None,
        erc_config: None,
        no_erc: false,
        json: false,
        lang: Lang::En,
        file: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--no-erc" => options.no_erc = true,
            "--models" | "--out" | "--config" | "--erc-config" | "--lang" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("{arg} needs a value"))?
                    .to_string();
                match *arg {
                    "--models" => options.models = Some(value),
                    "--out" => options.out = Some(value),
                    "--config" => options.config = Some(value),
                    "--erc-config" => options.erc_config = Some(value),
                    _ => {
                        options.lang =
                            Lang::from_tag(&value).ok_or("unknown language code".to_owned())?;
                    }
                }
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

/// What `netlist` and `sim --schematic` read.
pub(crate) struct Inputs<'a> {
    /// The schematic.
    pub(crate) schematic: &'a str,
    /// The table of models, when it is not the built-in one.
    pub(crate) models: Option<&'a str>,
    /// The settings of the rule check, when they are not the built-in ones.
    pub(crate) erc_config: Option<&'a str>,
    /// Skip the rule check.
    pub(crate) no_erc: bool,
}

/// The netlist, the rule check that came before it and the folder of the files it includes.
pub(crate) struct Prepared {
    pub(crate) generated: NetlistText,
    pub(crate) erc: Option<ErcReport>,
    pub(crate) folder: PathBuf,
}

/// Reads the schematic and the table, runs the rule check and makes the netlist. An error is the
/// message to show, in the language asked for.
pub(crate) fn prepare(
    inputs: &Inputs<'_>,
    settings: &SimSettings,
    lang: Lang,
) -> Result<Prepared, String> {
    let table = match inputs.models {
        Some(path) => {
            let text = read_text(path)?;
            ModelTable::parse(&text).map_err(|e| format!("{path}: {}", e.message(lang)))?
        }
        None => ModelTable::built_in(),
    };
    let erc_config = match inputs.erc_config {
        Some(path) => {
            let text = read_text(path)?;
            ErcConfig::parse(&text).map_err(|e| format!("{path}: {e}"))?
        }
        None => ErcConfig::default(),
    };
    let mut new_id = id_source()?;
    let schematic = load_schematic(inputs.schematic, &BTreeMap::new(), &mut new_id)?;
    let erc = if inputs.no_erc {
        None
    } else {
        Some(check(&schematic, &erc_config))
    };
    let mut generated = generate_netlist(&schematic, &table, None, settings);
    if let Some(report) = &erc {
        let errors = report.count(Severity::Error);
        if errors > 0 {
            generated.report.issues.insert(
                0,
                GenIssue::new(
                    ERC_ERRORS,
                    Severity::Error,
                    format!("{errors} errors (run `phoenix erc` for the list, or --no-erc to skip the check)"),
                ),
            );
        }
    }
    let beside = inputs.models.unwrap_or(inputs.schematic);
    let folder = match Path::new(beside).parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    Ok(Prepared {
        generated,
        erc,
        folder,
    })
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

/// Checks the output before anything is done, so a refusal costs nothing.
fn check_output(path: &str, inputs: &[&str], lang: Lang) -> Result<(), String> {
    let refuse = |why: String| GenIssue::new(OUTPUT_REFUSED, Severity::Error, why).message(lang);
    if inputs.iter().any(|input| same_file(path, input)) {
        return Err(refuse(format!("{path} is an input file")));
    }
    if Path::new(path).exists() {
        return Err(refuse(format!("{path} exists; give a new file")));
    }
    let schematic_formats = ["json", "sch", "kicad_sch", "kicad_pcb", "brd", "lbr", "cfg"];
    if schematic_formats.iter().any(|e| has_extension(path, e)) {
        return Err(refuse(format!(
            "{path}: a netlist is not written to a file of a design format (use .cir, .sp or .net)"
        )));
    }
    Ok(())
}

fn issue_values(generated: &NetlistText, lang: Lang) -> Vec<serde_json::Value> {
    generated
        .report
        .issues
        .iter()
        .map(|i| {
            json!({
                "code": i.code,
                "severity": i.severity.as_str(),
                "detail": i.detail,
                "message": i.message(lang),
            })
        })
        .collect()
}

/// The command `netlist`.
pub(crate) fn netlist_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    let settings = match options.config.as_deref() {
        Some(path) => match read_text(path).and_then(|text| {
            SimSettings::parse(&text).map_err(|e| format!("{path}: {}", e.message(options.lang)))
        }) {
            Ok(settings) => settings,
            Err(message) => return fail(err, &message),
        },
        None => SimSettings::default(),
    };
    if let Some(path) = options.out.as_deref() {
        let mut inputs = vec![file];
        inputs.extend(options.models.as_deref());
        if let Err(message) = check_output(path, &inputs, options.lang) {
            return fail(err, &message);
        }
    }
    let prepared = match prepare(
        &Inputs {
            schematic: file,
            models: options.models.as_deref(),
            erc_config: options.erc_config.as_deref(),
            no_erc: options.no_erc,
        },
        &settings,
        options.lang,
    ) {
        Ok(prepared) => prepared,
        Err(message) => return fail(err, &message),
    };
    let generated = &prepared.generated;

    let mut written: Option<&str> = None;
    if let Some(path) = options.out.as_deref() {
        if let Err(message) = write_atomic(
            path,
            generated.text.as_bytes(),
            settings.write_retries,
            settings.write_retry_ms,
        ) {
            return fail(err, &message);
        }
        written = Some(path);
    }

    if options.json {
        let erc_value = prepared
            .erc
            .as_ref()
            .and_then(|r| serde_json::from_str::<serde_json::Value>(&r.to_json()).ok());
        let value = json!({
            "schema": "phoenix-netlist/1",
            "netlist": generated.text,
            "translated": generated.report.translated,
            "grounds": generated.report.grounds,
            "nodes": generated.report.nodes,
            "includes": generated.report.includes,
            "issues": issue_values(generated, options.lang),
            "erc": erc_value,
            "written": written,
        });
        let _ = writeln!(out, "{value}");
    } else {
        if written.is_none() {
            let _ = out.write_all(generated.text.as_bytes());
        }
        let _ = err.write_all(generated.report.to_text(options.lang).as_bytes());
        if let Some(report) = &prepared.erc {
            if report.count(Severity::Error) > 0 {
                let _ = err.write_all(report.to_text(options.lang).as_bytes());
            }
        }
        match written {
            Some(path) => {
                let _ = writeln!(out, "written: {path}");
                if !generated.report.includes.is_empty() {
                    let _ = writeln!(
                        out,
                        "keep the included files in the folder of {path}: {}",
                        generated.report.includes.join(", ")
                    );
                }
            }
            None => {
                let _ = writeln!(
                    err,
                    "dry run: nothing was written (--out FILE writes the netlist)"
                );
            }
        }
    }
    if generated.report.has_errors() {
        1
    } else {
        0
    }
}
