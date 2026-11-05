// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `libedit` and `footprint`: editing an EAGLE library with a file of operations, and
//! making a footprint from parameters. Both are a dry run unless `--out` names a new file, and
//! neither writes over an input file or over a file that exists.
//!
//! Exit codes: 0 done and the check of the library finds no error, 1 done and it finds an error,
//! 2 the arguments, the input, an operation or the output are unusable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use eh3d_phoenix_libedit::codes::{LibError, FILE_UNUSABLE, NAME_USED, OPTION_INVALID};
use eh3d_phoenix_libedit::footprints::FootprintConfig;
use eh3d_phoenix_libedit::{
    apply_all, check_library, generate, parse_operations, read_library, replace_plan_from_json,
    save_new, LibSettings, Library, Severity,
};
use eh3d_phoenix_rules::Lang;
use serde_json::json;

pub(crate) const USAGE: &str = "\
phoenix libedit [--apply OPERATIONS.json] [--plan SANITIZER.plan.json] [--out NEW.lbr]
                [--config FILE] [--json] [--lang xx] <library.lbr>

  libedit    edit an EAGLE library with a file of operations (create, rename and delete symbols,
             footprints and devices; add, move, change and delete pads, lines, arcs, texts, pins
             and holes; openings of the solder mask and paste; gates and the connection of pins to
             pads), all or nothing, and check the library (codes PHOENIX-LIB-001 to 014). A dry run:
             nothing is written unless --out names a new file; the library is read back from the
             file that was written and compared, a copy of the original is kept next to the new
             file, and a file that exists is never written over
  --plan     show the steps of a plan of the sanitizer that would replace items of this library by
             the master candidate; they are never applied by this command
  --config   the settings (config/libedit.cfg shows every key)
  --json     print the report as JSON. --lang xx writes the check in en, es, de, fr, it, ja, zh or
             pt. Exit code: 0 no error found, 1 the check finds an error, 2 unusable input
";

pub(crate) const FOOTPRINT_USAGE: &str = "\
phoenix footprint <family> [--param NAME=VALUE ...] [--config FILE] [--into LIBRARY.lbr]
                  [--out NEW.lbr] [--json] [--lang xx]

  footprint  make a footprint from parameters by the method of IPC-7351B, with the dimensions of
             config/footprints.cfg, which are a prototype and not checked against a datasheet.
             Families: chip (size=0201..2512), soic (pins, variant=narrow|wide), tssop (pins,
             variant=4_4|6_1), qfn (pins, body, pitch or variant, ep), qfp (pins, body, pitch or
             variant), dip (pins, row=narrow|wide|mm) and header (pins, rows=1|2, pitch). A dry
             run: nothing is written unless --out names a new file; --into starts from a copy of a
             library, and an input file, or a file that exists, is never written over
";

/// The largest settings, operations or plan file that is read, in bytes.
const MAX_SMALL_FILE_BYTES: u64 = 16 * 1024 * 1024;

fn read_small(path: &str) -> Result<String, LibError> {
    let unusable = |e: String| LibError::new(FILE_UNUSABLE, format!("{path}: {e}"));
    let length = std::fs::metadata(path)
        .map_err(|e| unusable(e.to_string()))?
        .len();
    if length > MAX_SMALL_FILE_BYTES {
        return Err(unusable(format!(
            "larger than {MAX_SMALL_FILE_BYTES} bytes"
        )));
    }
    std::fs::read_to_string(path).map_err(|e| unusable(e.to_string()))
}

fn read_library_file(path: &str, settings: &LibSettings) -> Result<Library, LibError> {
    let unusable = |e: String| LibError::new(FILE_UNUSABLE, format!("{path}: {e}"));
    let length = std::fs::metadata(path)
        .map_err(|e| unusable(e.to_string()))?
        .len();
    if length > settings.max_file_bytes {
        return Err(unusable(format!(
            "larger than {} bytes",
            settings.max_file_bytes
        )));
    }
    let bytes = std::fs::read(path).map_err(|e| unusable(e.to_string()))?;
    read_library(&bytes, &settings.limits())
        .map_err(|e| LibError::new(e.code, format!("{path}: {}", e.detail)))
}

struct Options {
    apply: Option<String>,
    plan: Option<String>,
    out: Option<String>,
    config: Option<String>,
    json: bool,
    lang: Lang,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, LibError> {
    let bad = |what: &str| LibError::new(OPTION_INVALID, what.to_owned());
    let mut options = Options {
        apply: None,
        plan: None,
        out: None,
        config: None,
        json: false,
        lang: Lang::En,
        file: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--apply" => {
                let v = iter.next().ok_or_else(|| bad("--apply needs a file"))?;
                options.apply = Some((*v).to_owned());
            }
            "--plan" => {
                let v = iter.next().ok_or_else(|| bad("--plan needs a file"))?;
                options.plan = Some((*v).to_owned());
            }
            "--out" => {
                let v = iter.next().ok_or_else(|| bad("--out needs a file"))?;
                options.out = Some((*v).to_owned());
            }
            "--config" => {
                let v = iter.next().ok_or_else(|| bad("--config needs a file"))?;
                options.config = Some((*v).to_owned());
            }
            "--lang" => {
                let v = iter.next().ok_or_else(|| bad("--lang needs a code"))?;
                options.lang = Lang::from_tag(v)
                    .ok_or_else(|| bad("--lang is not a language of the suite"))?;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")))
            }
            file => {
                if options.file.is_some() {
                    return Err(bad("only one library"));
                }
                options.file = Some(file.to_owned());
            }
        }
    }
    Ok(options)
}

fn settings_of(config: &Option<String>) -> Result<LibSettings, LibError> {
    match config {
        Some(path) => LibSettings::parse(&read_small(path)?),
        None => Ok(LibSettings::default()),
    }
}

fn libedit_inner(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, (LibError, Lang)> {
    let options = parse_options(args).map_err(|e| (e, Lang::En))?;
    let lang = options.lang;
    let fail = |e: LibError| (e, lang);
    let Some(file) = options.file.as_deref() else {
        let _ = err.write_all(USAGE.as_bytes());
        return Err(fail(LibError::new(OPTION_INVALID, "a library is needed")));
    };
    let settings = settings_of(&options.config).map_err(fail)?;
    let original = read_library_file(file, &settings).map_err(fail)?;
    let operations = match &options.apply {
        Some(path) => parse_operations(&read_small(path).map_err(fail)?).map_err(fail)?,
        None => Vec::new(),
    };
    let edited = apply_all(&original, &operations).map_err(|(index, e)| {
        fail(LibError::new(
            e.code,
            format!(
                "operation {} ({}): {}",
                index + 1,
                operations[index].label(),
                e.detail
            ),
        ))
    })?;
    let plan = match &options.plan {
        Some(path) => {
            Some(replace_plan_from_json(&read_small(path).map_err(fail)?, file).map_err(fail)?)
        }
        None => None,
    };
    let report = check_library(&edited, &settings);
    let saved = match &options.out {
        Some(target) => {
            Some(save_new(&edited, Some(Path::new(file)), Path::new(target)).map_err(fail)?)
        }
        None => None,
    };
    let labels: Vec<String> = operations.iter().map(|o| o.label()).collect();
    if options.json {
        let value = json!({
            "library": file,
            "operations": labels,
            "written": saved.as_ref().map(|s| s.path.to_string_lossy().into_owned()),
            "sha256": saved.as_ref().map(|s| s.sha256.clone()),
            "backup": saved.as_ref().and_then(|s| s.backup.as_ref()).map(|b| b.to_string_lossy().into_owned()),
            "kept_as_xml": edited.kept_raw,
            "plan": plan,
            "check": report,
        });
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    } else {
        let _ = writeln!(
            out,
            "library {file}: {} footprints, {} symbols, {} device sets",
            edited.packages.len(),
            edited.symbols.len(),
            edited.devicesets.len()
        );
        if edited.kept_raw > 0 {
            let _ = writeln!(
                out,
                "{} items are kept as the XML they came in (not editable here)",
                edited.kept_raw
            );
        }
        for (n, label) in labels.iter().enumerate() {
            let _ = writeln!(out, "operation {}: {label}", n + 1);
        }
        if let Some(plan) = &plan {
            let _ = writeln!(
                out,
                "plan of the sanitizer: {} steps for this library, none applied (they need the confirmation of the owner)",
                plan.steps.len()
            );
            for step in &plan.steps {
                let _ = writeln!(
                    out,
                    "  {} {} -> {} of {} (score {}{})",
                    step.kind,
                    step.name,
                    step.master_name,
                    step.master_library,
                    step.score,
                    if step.turned { ", needs a turn" } else { "" }
                );
            }
        }
        let _ = out.write_all(report.to_text(lang).as_bytes());
        match &saved {
            Some(s) => {
                let _ = writeln!(
                    out,
                    "written: {} ({} bytes, sha256 {})",
                    s.path.display(),
                    s.bytes,
                    s.sha256
                );
                if let Some(b) = &s.backup {
                    let _ = writeln!(out, "copy of the original: {}", b.display());
                }
                let _ = writeln!(out, "the file was read back and is the same library");
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written (name a new file with --out)"
                );
            }
        }
    }
    let has_error = report
        .findings
        .iter()
        .any(|f| f.severity == Severity::Error);
    Ok(i32::from(has_error))
}

/// Runs the command `libedit` with the arguments that follow its name. Returns the exit code.
pub(crate) fn libedit_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match libedit_inner(args, out, err) {
        Ok(code) => code,
        Err((e, lang)) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}

struct FootprintOptions {
    family: Option<String>,
    params: BTreeMap<String, String>,
    config: Option<String>,
    into: Option<String>,
    out: Option<String>,
    json: bool,
    lang: Lang,
}

fn parse_footprint_options(args: &[&str]) -> Result<FootprintOptions, LibError> {
    let bad = |what: &str| LibError::new(OPTION_INVALID, what.to_owned());
    let mut options = FootprintOptions {
        family: None,
        params: BTreeMap::new(),
        config: None,
        into: None,
        out: None,
        json: false,
        lang: Lang::En,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--param" => {
                let v = iter.next().ok_or_else(|| bad("--param needs NAME=VALUE"))?;
                let (name, value) = v
                    .split_once('=')
                    .ok_or_else(|| bad("--param needs NAME=VALUE"))?;
                if options
                    .params
                    .insert(name.trim().to_owned(), value.trim().to_owned())
                    .is_some()
                {
                    return Err(bad(&format!(
                        "the parameter {} is given twice",
                        name.trim()
                    )));
                }
            }
            "--config" => {
                let v = iter.next().ok_or_else(|| bad("--config needs a file"))?;
                options.config = Some((*v).to_owned());
            }
            "--into" => {
                let v = iter.next().ok_or_else(|| bad("--into needs a library"))?;
                options.into = Some((*v).to_owned());
            }
            "--out" => {
                let v = iter.next().ok_or_else(|| bad("--out needs a file"))?;
                options.out = Some((*v).to_owned());
            }
            "--lang" => {
                let v = iter.next().ok_or_else(|| bad("--lang needs a code"))?;
                options.lang = Lang::from_tag(v)
                    .ok_or_else(|| bad("--lang is not a language of the suite"))?;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")))
            }
            family => {
                if options.family.is_some() {
                    return Err(bad("only one family"));
                }
                options.family = Some(family.to_owned());
            }
        }
    }
    Ok(options)
}

fn footprint_inner(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, (LibError, Lang)> {
    let options = parse_footprint_options(args).map_err(|e| (e, Lang::En))?;
    let lang = options.lang;
    let fail = |e: LibError| (e, lang);
    let Some(family) = options.family.as_deref() else {
        let _ = err.write_all(FOOTPRINT_USAGE.as_bytes());
        return Err(fail(LibError::new(OPTION_INVALID, "a family is needed")));
    };
    let cfg = match &options.config {
        Some(path) => FootprintConfig::parse(&read_small(path).map_err(fail)?).map_err(fail)?,
        None => FootprintConfig::builtin(),
    };
    let made = generate(family, &options.params, &cfg).map_err(fail)?;
    let settings = LibSettings::default();
    let mut library = match &options.into {
        Some(path) => read_library_file(path, &settings).map_err(fail)?,
        None => Library::empty(),
    };
    if library.package_index(&made.package.name).is_some() {
        return Err(fail(LibError::new(
            NAME_USED,
            format!("footprint {}", made.package.name),
        )));
    }
    library.packages.push(Arc::new(made.package.clone()));
    let report = check_library(&library, &settings);
    let ours = report.about("package", &made.package.name);
    let saved = match &options.out {
        Some(target) => Some(
            save_new(
                &library,
                options.into.as_deref().map(Path::new),
                Path::new(target),
            )
            .map_err(fail)?,
        ),
        None => None,
    };
    let mut has_error = false;
    for f in &ours {
        has_error |= f.severity == Severity::Error;
    }
    if options.json {
        let pads: Vec<_> = made
            .package
            .items
            .iter()
            .filter(|i| i.is_pad())
            .map(|i| serde_json::to_value(i).unwrap_or_default())
            .collect();
        let value = json!({
            "name": made.package.name,
            "notes": made.notes,
            "pads": pads,
            "items": made.package.items.len(),
            "written": saved.as_ref().map(|s| s.path.to_string_lossy().into_owned()),
            "findings": ours,
        });
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        );
    } else {
        let _ = writeln!(out, "footprint {}", made.package.name);
        for note in &made.notes {
            let _ = writeln!(out, "  {note}");
        }
        for item in made.package.items.iter().filter(|i| i.is_pad()) {
            if let Some((x, y)) = item.pad_centre() {
                let _ = writeln!(
                    out,
                    "  pad {} at ({}, {}) nm",
                    item.name().unwrap_or(""),
                    x,
                    y
                );
            }
        }
        for f in &ours {
            let _ = writeln!(
                out,
                "  {} {}: {}",
                f.severity.word(),
                f.code,
                eh3d_phoenix_libedit::codes::title(f.code, lang)
            );
        }
        match &saved {
            Some(s) => {
                let _ = writeln!(out, "written: {} ({} bytes)", s.path.display(), s.bytes);
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written (name a new file with --out)"
                );
            }
        }
    }
    Ok(i32::from(has_error))
}

/// Runs the command `footprint` with the arguments that follow its name. Returns the exit code.
pub(crate) fn footprint_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match footprint_inner(args, out, err) {
        Ok(code) => code,
        Err((e, lang)) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}
