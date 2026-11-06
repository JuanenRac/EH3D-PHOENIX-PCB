// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `hier`: the tree of sheets of a schematic, the instances of every sheet (a sheet
//! shown by two sheet symbols is two instances), the nets with the instances they touch, and the
//! problems of the sheet symbols, the pins, the ports and the buses. It only reads: nothing is
//! written.
//!
//! Exit codes: 0 no error, 1 the report has an error, 2 the arguments or the input are unusable.

use std::collections::BTreeMap;
use std::io::Write;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{code_title, HierConfig, Schematic};
use serde_json::json;

use crate::check::{id_source, read_text};
use crate::design::load_schematic;

pub(crate) const USAGE: &str = "\
phoenix hier [--config FILE] [--json] [--lang CODE] <schematic>

  hier          the hierarchy of sheets of a schematic (an EAGLE .sch file, a KiCad .kicad_sch
                file or the JSON of the design model): the sheets and the sheet symbols that show
                them, the instances (a sheet shown N times is N instances, with references such
                as U1/R1), the nets by hierarchical name with the instances where they have a
                pin, and the problems of the sheet symbols, pins, ports and buses with codes
                PHOENIX-HIER-<NNN> and PHOENIX-BUS-<NNN>. It only reads: nothing is written
  --config      the settings of the expansion (config/hierarchy.cfg shows every line and is the
                default)
  --json        print the report as JSON. --lang CODE writes the titles of the problems in en,
                es, de, fr, it, ja, zh or pt. Exit code: 0 no error, 1 the report has an error,
                2 unusable input
";

struct Options {
    config: Option<String>,
    json: bool,
    lang: Lang,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        config: None,
        json: false,
        lang: Lang::En,
        file: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--config" | "--lang" => {
                let value = iter
                    .next()
                    .ok_or_else(|| format!("{arg} needs a value"))?
                    .to_string();
                if *arg == "--config" {
                    options.config = Some(value);
                } else {
                    options.lang =
                        Lang::from_tag(&value).ok_or("unknown language code".to_owned())?;
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

/// One problem as the report shows it.
struct Problem {
    code: String,
    severity: String,
    title: String,
    detail: String,
}

fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

fn problems_of(schematic: &Schematic, cfg: &HierConfig, lang: Lang) -> Vec<Problem> {
    let mut found = schematic.hier_problems(cfg);
    found.extend(schematic.bus_problems());
    found
        .iter()
        .map(|d| {
            let code = d.code.as_deref().unwrap_or("").to_owned();
            let detail = d
                .message
                .split_once(": ")
                .map_or(d.message.as_str(), |(_, rest)| rest)
                .to_owned();
            Problem {
                title: code_title(&code, column(lang)).to_owned(),
                code,
                severity: format!("{:?}", d.severity).to_lowercase(),
                detail,
            }
        })
        .collect()
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

/// The command `hier`.
pub(crate) fn hier_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    let cfg = match options.config.as_deref() {
        Some(path) => match read_text(path)
            .and_then(|text| HierConfig::parse(&text).map_err(|e| format!("{path}: {e}")))
        {
            Ok(cfg) => cfg,
            Err(message) => return fail(err, &message),
        },
        None => HierConfig::default(),
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let schematic = match load_schematic(file, &BTreeMap::new(), &mut new_id) {
        Ok(schematic) => schematic,
        Err(message) => return fail(err, &message),
    };
    let expanded = match schematic.expand(&cfg) {
        Ok(expanded) => expanded,
        Err(e) => return fail(err, &e.to_string()),
    };
    let mut problems = problems_of(&schematic, &cfg, options.lang);
    // the loops and limits are in the checks above; the references that repeat are found by the
    // expansion
    for d in &expanded.problems {
        let code = d.code.as_deref().unwrap_or("");
        if code == eh3d_phoenix_schematic::hier::REFDES_REPEATED {
            problems.push(Problem {
                title: code_title(code, column(options.lang)).to_owned(),
                code: code.to_owned(),
                severity: format!("{:?}", d.severity).to_lowercase(),
                detail: d
                    .message
                    .split_once(": ")
                    .map_or(d.message.as_str(), |(_, rest)| rest)
                    .to_owned(),
            });
        }
    }
    let rows = expanded.net_rows();
    let sheet_name = |id: &eh3d_phoenix_schematic::Id| {
        schematic
            .sheets
            .get(id)
            .map_or_else(String::new, |s| s.name.clone())
    };
    let has_error = problems.iter().any(|p| p.severity == "error");

    let text = if options.json {
        let instances: Vec<serde_json::Value> = expanded
            .hierarchy
            .instances
            .iter()
            .map(|i| {
                json!({
                    "name": i.name,
                    "path": i.path,
                    "sheet": sheet_name(&i.sheet),
                    "symbol": i.block.and_then(|b| schematic.sheet_blocks.get(&b)).map(|b| b.name.clone()),
                    "parent": i.parent,
                })
            })
            .collect();
        let nets: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| json!({"name": r.name, "pins": r.pins, "instances": r.instances}))
            .collect();
        let found: Vec<serde_json::Value> = problems
            .iter()
            .map(|p| {
                json!({
                    "code": p.code,
                    "severity": p.severity,
                    "title": p.title,
                    "detail": p.detail,
                })
            })
            .collect();
        let report = json!({
            "name": schematic.name,
            "sheets": schematic.sheets.len(),
            "sheet_symbols": schematic.sheet_blocks.len(),
            "buses": schematic.buses.len(),
            "parts": expanded.schematic.parts.len(),
            "instances": instances,
            "nets": nets,
            "problems": found,
        });
        let mut text = serde_json::to_string_pretty(&report).unwrap_or_default();
        text.push('\n');
        text
    } else {
        let mut text = format!(
            "hierarchy of {}: {} sheets, {} sheet symbols, {} instances, {} parts, {} nets\n",
            schematic.name,
            schematic.sheets.len(),
            schematic.sheet_blocks.len(),
            expanded.hierarchy.instances.len(),
            expanded.schematic.parts.len(),
            rows.len()
        );
        text.push_str("instances:\n");
        for i in &expanded.hierarchy.instances {
            let shown = if i.name.is_empty() {
                expanded.top_name.as_str()
            } else {
                i.name.as_str()
            };
            let by = i
                .block
                .and_then(|b| schematic.sheet_blocks.get(&b))
                .map_or_else(String::new, |b| format!(" (symbol {})", b.name));
            text.push_str(&format!("  {shown}  {}{by}\n", sheet_name(&i.sheet)));
        }
        text.push_str("nets:\n");
        for r in &rows {
            text.push_str(&format!(
                "  {}  {} pins  {}\n",
                r.name,
                r.pins,
                r.instances.join(" ")
            ));
        }
        if problems.is_empty() {
            text.push_str("problems: none\n");
        } else {
            text.push_str("problems:\n");
            for p in &problems {
                text.push_str(&format!(
                    "  {} {}: {} ({})\n",
                    p.code, p.severity, p.title, p.detail
                ));
            }
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    i32::from(has_error)
}
