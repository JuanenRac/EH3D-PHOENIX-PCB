// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `eagle-check` and `eagle-convert`, and the helpers that `check`, `bom` and `pnp`
//! use for `--rules-dru` and `--variant`: design rule files, CAM jobs, layers, attributes and
//! variants of EAGLE files, and the report of what a rewrite keeps.
//!
//! Exit codes: 0 done, 1 an error finding (the tree rewrite changed the file, a rule file that is
//! not valid), 2 the arguments or the input are unusable. Nothing is written unless `--out` names a
//! new file; an existing file is never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_eagle::xml::{parse, Limits};
use eh3d_phoenix_eagle_deep::codes::{
    INPUT_UNREADABLE, NOT_EAGLE_FILE, OPTION_INVALID, OUTPUT_FAILED,
};
use eh3d_phoenix_eagle_deep::dru::DruFile;
use eh3d_phoenix_eagle_deep::{
    cam_to_profile, check_eagle, decode_text, dru_to_rules, lines, read_layers, read_variants,
    rule_file_to_dru, standard_table, CamJob, DeepError, Finding, LayerMap, Severity, VariantView,
};
use eh3d_phoenix_rules::{Lang, RuleFile};

/// The layer map that is read when `--layers` is not given and the file exists.
const DEFAULT_LAYERS: &str = "config/eagle_layers.cfg";

/// The largest input file that is read, in bytes.
const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix eagle-check [--layers FILE] [--json] [--lang CODE] <file.brd|file.sch|file.lbr>
phoenix eagle-convert dru-to-rules [--name NAME] [--out FILE] [--lang CODE] <file.dru>
phoenix eagle-convert rules-to-dru [--base FILE.dru] [--out FILE] [--lang CODE] <rules.json>
phoenix eagle-convert cam-to-profile [--name NAME] [--layers FILE] [--out FILE] [--lang CODE] <file.cam>
phoenix eagle-convert layers [--layers FILE] [--lang CODE] [file.brd|file.sch|file.lbr]
phoenix eagle-convert variants [--variant NAME] [--lang CODE] <file.brd|file.sch>

  eagle-check   reads an EAGLE file and tells what a rewrite keeps: it writes the XML tree and reads
                it again to prove that nothing changes, and it names (with the codes
                PHOENIX-EAGLEX-nnn) the kinds of element that the design model of a board or
                schematic simplifies or leaves out, the ones nothing here interprets (kept byte for
                byte), the layers of the file (user layers need a role in the layer map), the
                variants that are not defined and the entities that the model has no place for. The
                exit code is 1 when the rewrite of the tree changes the file
  eagle-convert dru-to-rules   a design rule file (.dru) as a rule file of this tool
                (phoenix-rules/1); the parameters folded together or without equivalent are named
  eagle-convert rules-to-dru   a rule file as a design rule file; with --base the lines of that file
                are kept and only the parameters the rules give are set; the rules that EAGLE has no
                parameter for are named
  eagle-convert cam-to-profile a CAM job (.cam: the text of the old versions, XML or JSON) as an
                export profile file (config/export_profiles.json); the options, layers and devices
                that the profile does not respect are named
  eagle-convert layers         the table of the standard layers with their roles, or the layers of a
                file with the role of each (the user layers from the layer map)
  eagle-convert variants       the variants and attributes of a schematic or board; with --variant the
                parts that are not fitted in it and the values it changes
  --layers      the roles of the user layers (default: config/eagle_layers.cfg when it exists)
  --name        the name of the rule set or of the export profile
  --out         write the result to a new file (an existing file is never replaced); without it the
                result is printed and the findings go to the error output
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt

A prototype: the formats are written from their documentation and no file saved by EAGLE has been
compared with them. Nothing is written without --out.
";

fn bad(detail: &str) -> DeepError {
    DeepError::new(OPTION_INVALID, detail)
}

fn read_input(path: &str) -> Result<String, DeepError> {
    let unreadable = |e: std::io::Error| DeepError::new(INPUT_UNREADABLE, format!("{path}: {e}"));
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    if size > MAX_INPUT_BYTES {
        return Err(DeepError::new(
            INPUT_UNREADABLE,
            format!("{path} is {size} bytes, the limit is {MAX_INPUT_BYTES}"),
        ));
    }
    let bytes = std::fs::read(path).map_err(unreadable)?;
    Ok(decode_text(&bytes).0)
}

fn load_layer_map(named: Option<&str>) -> Result<LayerMap, DeepError> {
    let path = named.or_else(|| {
        Path::new(DEFAULT_LAYERS)
            .is_file()
            .then_some(DEFAULT_LAYERS)
    });
    match path {
        Some(path) => LayerMap::parse(&read_input(path)?),
        None => Ok(LayerMap::default()),
    }
}

/// Writes `text` to a file that must not exist.
fn write_new(path: &str, text: &str) -> Result<(), DeepError> {
    let failed = |e: std::io::Error| DeepError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

/// The values of the design rule file `path` for `phoenix check --rules-dru`, and the notes that
/// say what was folded, not used or not understood.
pub(crate) fn dru_values(
    path: &str,
    lang: Lang,
) -> Result<(eh3d_phoenix_rules::RuleValues, Vec<String>), String> {
    let text = read_input(path).map_err(|e| e.message(lang))?;
    let dru = DruFile::parse(&text).map_err(|e| e.message(lang))?;
    let converted = dru_to_rules(&dru);
    let notes = converted.findings.iter().map(|f| f.line(lang)).collect();
    Ok((converted.values, notes))
}

/// The view of the variant `variant` of the EAGLE file `path`, for `--variant` of `bom` and `pnp`.
pub(crate) fn variant_view(path: &str, variant: &str, lang: Lang) -> Result<VariantView, String> {
    let lower = path.to_ascii_lowercase();
    if !(lower.ends_with(".brd") || lower.ends_with(".sch")) {
        return Err(
            "the variants come from an EAGLE board or schematic file (.brd or .sch); name it with --variant-file"
                .to_owned(),
        );
    }
    let text = read_input(path).map_err(|e| e.message(lang))?;
    let root = parse(&text, &Limits::default())
        .map_err(|e| DeepError::new(NOT_EAGLE_FILE, format!("{path}: {e}")).message(lang))?;
    read_variants(&root)
        .view(variant)
        .map_err(|e| e.message(lang))
}

struct Options {
    json: bool,
    lang: Lang,
    layers: Option<String>,
    name: Option<String>,
    base: Option<String>,
    variant: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, DeepError> {
    let mut o = Options {
        json: false,
        lang: Lang::En,
        layers: None,
        name: None,
        base: None,
        variant: None,
        out: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, DeepError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => o.json = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                o.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--layers" => o.layers = Some(value("--layers needs a file")?),
            "--name" => o.name = Some(value("--name needs a name")?),
            "--base" => o.base = Some(value("--base needs a file")?),
            "--variant" => o.variant = Some(value("--variant needs a name")?),
            "--out" => o.out = Some(value("--out needs a file")?),
            other if other.starts_with("--") => {
                return Err(DeepError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn fail(err: &mut dyn Write, error: &DeepError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Delivers a result: to a new file when `--out` is given (then the findings go to the output),
/// otherwise printed with the findings on the error output.
fn deliver(
    o: &Options,
    text: &str,
    findings: &[Finding],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match &o.out {
        Some(path) => {
            if let Err(error) = write_new(path, text) {
                return fail(err, &error, o.lang);
            }
            let _ = writeln!(out, "written: {path}");
            let _ = out.write_all(lines(findings, o.lang).as_bytes());
        }
        None => {
            let _ = out.write_all(text.as_bytes());
            let _ = err.write_all(lines(findings, o.lang).as_bytes());
        }
    }
    i32::from(findings.iter().any(|f| f.severity == Severity::Error))
}

pub(crate) fn eagle_check_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let o = match parse_options(args) {
        Ok(o) => o,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    let [path] = o.files.as_slice() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let result = load_layer_map(o.layers.as_deref())
        .and_then(|map| read_input(path).and_then(|text| check_eagle(&text, &map)));
    match result {
        Ok(check) => {
            let text = if o.json {
                check.to_json()
            } else {
                check.to_text(o.lang)
            };
            let _ = out.write_all(text.as_bytes());
            i32::from(check.has_errors())
        }
        Err(error) => fail(err, &error, o.lang),
    }
}

pub(crate) fn eagle_convert_command(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let Some((&what, tail)) = args.split_first() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let o = match parse_options(tail) {
        Ok(o) => o,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    let result = match what {
        "dru-to-rules" => dru_to_rules_command(&o),
        "rules-to-dru" => rules_to_dru_command(&o),
        "cam-to-profile" => cam_command(&o),
        "layers" => layers_command(&o),
        "variants" => variants_command(&o),
        other => Err(DeepError::new(
            OPTION_INVALID,
            format!("unknown conversion {other}"),
        )),
    };
    match result {
        Ok((text, findings)) => deliver(&o, &text, &findings, out, err),
        Err(error) => {
            if error.code == OPTION_INVALID {
                let _ = err.write_all(USAGE.as_bytes());
            }
            fail(err, &error, o.lang)
        }
    }
}

type Done = Result<(String, Vec<Finding>), DeepError>;

fn one_file(o: &Options) -> Result<&str, DeepError> {
    match o.files.as_slice() {
        [path] => Ok(path.as_str()),
        _ => Err(bad("give exactly one input file")),
    }
}

fn dru_to_rules_command(o: &Options) -> Done {
    let dru = DruFile::parse(&read_input(one_file(o)?)?)?;
    let converted = dru_to_rules(&dru);
    let name = o.name.clone().unwrap_or_else(|| {
        dru.lines
            .iter()
            .find_map(|l| match l {
                eh3d_phoenix_eagle_deep::dru::DruLine::Description { lang, text }
                    if lang == "en" =>
                {
                    Some(text.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| "eagle-dru".to_owned())
    });
    Ok((converted.values.to_json(&name), converted.findings))
}

fn rules_to_dru_command(o: &Options) -> Done {
    let text = read_input(one_file(o)?)?;
    let file =
        RuleFile::from_json(&text).map_err(|e| DeepError::new(INPUT_UNREADABLE, e.to_string()))?;
    let base = match &o.base {
        Some(path) => Some(DruFile::parse(&read_input(path)?)?),
        None => None,
    };
    let written = rule_file_to_dru(&file, base.as_ref());
    Ok((written.dru.to_text(), written.findings))
}

fn cam_command(o: &Options) -> Done {
    let job = CamJob::parse(&read_input(one_file(o)?)?)?;
    let map = load_layer_map(o.layers.as_deref())?;
    let name = o.name.clone().unwrap_or_else(|| "from-cam".to_owned());
    let done = cam_to_profile(&job, &name, &map);
    Ok((done.profile_json, done.findings))
}

fn layers_command(o: &Options) -> Done {
    let map = load_layer_map(o.layers.as_deref())?;
    let mut text = String::new();
    match o.files.as_slice() {
        [] => {
            for (number, name, role) in standard_table() {
                text.push_str(&format!("{number}\t{name}\t{}\n", role.token()));
            }
            Ok((text, Vec::new()))
        }
        [path] => {
            let source = read_input(path)?;
            let root = parse(&source, &Limits::default())
                .map_err(|e| DeepError::new(NOT_EAGLE_FILE, format!("{path}: {e}")))?;
            let read = read_layers(&root, &map);
            for layer in &read.layers {
                text.push_str(&format!(
                    "{}\t{}\t{}\n",
                    layer.number,
                    layer.name,
                    layer.role.token()
                ));
            }
            Ok((text, read.findings))
        }
        _ => Err(bad("give at most one input file")),
    }
}

fn variants_command(o: &Options) -> Done {
    let path = one_file(o)?;
    let source = read_input(path)?;
    let root = parse(&source, &Limits::default())
        .map_err(|e| DeepError::new(NOT_EAGLE_FILE, format!("{path}: {e}")))?;
    let set = read_variants(&root);
    let mut text = format!(
        "variants: {}\n",
        if set.defs.is_empty() {
            "none".to_owned()
        } else {
            set.names().join(", ")
        }
    );
    if let Some(current) = set.current() {
        text.push_str(&format!("current: {current}\n"));
    }
    text.push_str(&format!("parts: {}\n", set.parts.len()));
    let with_attributes = set
        .parts
        .iter()
        .filter(|p| !p.attributes.is_empty())
        .count();
    text.push_str(&format!("parts with attributes: {with_attributes}\n"));
    let mut findings = set.check();
    if let Some(name) = &o.variant {
        let view = set.view(name)?;
        text.push_str(&format!("variant {name}:\n"));
        for (refdes, part) in &view.parts {
            let base = set
                .parts
                .iter()
                .find(|p| p.name() == refdes)
                .map_or("", |p| p.value());
            if !part.populated {
                text.push_str(&format!("  {refdes}\tnot fitted\n"));
            } else if part.value != base {
                text.push_str(&format!("  {refdes}\tvalue {} (was {base})\n", part.value));
            }
        }
    }
    findings.sort_by_key(|f| f.severity);
    Ok((text, findings))
}
