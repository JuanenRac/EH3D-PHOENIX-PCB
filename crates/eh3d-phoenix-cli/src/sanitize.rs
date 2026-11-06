// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `sanitize`: the analysis of a folder of EAGLE libraries for duplicates. It reads
//! and never changes the libraries; with `--out` it writes a report (HTML and JSON) and a plan.
//! With `--apply` it works out what the plan would do to a copy of the libraries (a dry run that
//! prints the confirmation code of the plan), and with `--apply --confirm CODE --out NEW` it
//! writes that copy to a NEW folder; neither the folder that was read nor a folder that exists is
//! ever written.
//!
//! Exit codes: 0 done, 2 the arguments, the settings, the folder or the output are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sanitizer::codes::{SanError, OPTION_INVALID, SETTINGS_INVALID};
use eh3d_phoenix_sanitizer::{
    apply_plan, confirmation_code, plan_of, preview_plan, read_folder, sanitize, write_folder,
    write_outputs, ApplyOptions, SanitizerSettings,
};

pub(crate) const USAGE: &str = "\
phoenix sanitize [--out BASE] [--threshold N] [--config FILE] [--lang xx] [--json] <folder>
phoenix sanitize --apply [--group ID ...] [--allow-near] [--threshold N] [--config FILE] [--json] <folder>
phoenix sanitize --apply --confirm CODE --out NEWFOLDER [--group ID ...] [--allow-near] <folder>

  sanitize    read the .lbr libraries under <folder> and find the footprints, symbols and device
              sets that are duplicated or nearly duplicated: an inventory, a canonical geometric
              hash, a score against a threshold, groups to confirm by hand and the item that
              would be kept in each; nothing is rewritten and nothing is merged
  --out       write BASE.html (one page), BASE.json (the same report) and BASE.plan.json (the
              plan of the master library, which this program never applies)
  --threshold the lowest score, 0 to 100, at which two items are grouped (the settings give the
              default)
  --config    a settings file (config/sanitizer.cfg shows every key)
  --lang      the language of the page, one of en, es, de, fr, it, ja, zh, pt
  --json      print the report as JSON instead of the summary
  --apply     work out what the plan would do to a COPY of the libraries: the footprints and the
              symbols of the groups are replaced by the master item under the name they had (so
              nothing that uses them by name moves), and the steps that would turn, mirror or shift
              a part, that rename pads or pins, that are device sets or whose geometry is only close
              are left out with their reason. Without --confirm it is a dry run that prints the
              confirmation code of the plan
  --confirm   the confirmation code of the plan, as the dry run printed it; it changes when the
              plan changes
  --out       with --apply: the NEW folder that receives the copy of every library (it must not
              exist and must not be inside the folder that was read)
  --group     with --apply: apply only this group (repeat it for several); the others are left out
  --allow-near with --apply: also replace items whose geometry is close and not the same
";

/// The largest settings file that is read, in bytes.
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

struct Options {
    apply: bool,
    confirm: Option<String>,
    groups: Vec<String>,
    allow_near: bool,
    out: Option<String>,
    threshold: Option<u32>,
    config: Option<String>,
    lang: Lang,
    json: bool,
    folder: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, SanError> {
    let bad = |what: &str| SanError::new(OPTION_INVALID, what.to_owned());
    let mut options = Options {
        apply: false,
        confirm: None,
        groups: Vec::new(),
        allow_near: false,
        out: None,
        threshold: None,
        config: None,
        lang: Lang::En,
        json: false,
        folder: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--apply" => options.apply = true,
            "--allow-near" => options.allow_near = true,
            "--confirm" => {
                let value = iter.next().ok_or_else(|| bad("--confirm needs the code"))?;
                options.confirm = Some((*value).to_owned());
            }
            "--group" => {
                let value = iter.next().ok_or_else(|| bad("--group needs an id"))?;
                options.groups.push((*value).to_owned());
            }
            "--out" => {
                let value = iter.next().ok_or_else(|| bad("--out needs a path"))?;
                options.out = Some((*value).to_owned());
            }
            "--config" => {
                let value = iter.next().ok_or_else(|| bad("--config needs a file"))?;
                options.config = Some((*value).to_owned());
            }
            "--threshold" => {
                let value = iter
                    .next()
                    .ok_or_else(|| bad("--threshold needs a number"))?;
                let n: u32 = value
                    .parse()
                    .map_err(|_| bad("--threshold is not a whole number"))?;
                if n > 100 {
                    return Err(bad("--threshold must be between 0 and 100"));
                }
                options.threshold = Some(n);
            }
            "--lang" => {
                let value = iter.next().ok_or_else(|| bad("--lang needs a code"))?;
                options.lang = Lang::from_tag(value)
                    .ok_or_else(|| bad("--lang is not a language of the suite"))?;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            folder => {
                if options.folder.is_some() {
                    return Err(bad("only one folder"));
                }
                options.folder = Some(folder.to_owned());
            }
        }
    }
    if !options.apply
        && (options.confirm.is_some() || !options.groups.is_empty() || options.allow_near)
    {
        return Err(bad("--confirm, --group and --allow-near need --apply"));
    }
    if options.apply && options.confirm.is_some() != options.out.is_some() {
        return Err(bad(
            "--apply writes only with both --confirm CODE and --out NEWFOLDER",
        ));
    }
    Ok(options)
}

fn read_settings(path: &str) -> Result<SanitizerSettings, SanError> {
    let invalid = |detail: String| SanError::new(SETTINGS_INVALID, detail);
    let length = std::fs::metadata(path)
        .map_err(|e| invalid(e.to_string()))?
        .len();
    if length > MAX_CONFIG_BYTES {
        return Err(invalid(format!(
            "the file is larger than {MAX_CONFIG_BYTES} bytes"
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|e| invalid(e.to_string()))?;
    SanitizerSettings::parse(&text)
}

fn run_inner(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), (SanError, Lang)> {
    let options = parse_options(args).map_err(|e| (e, Lang::En))?;
    let lang = options.lang;
    let fail = |e: SanError| (e, lang);
    let Some(folder) = options.folder else {
        let _ = err.write_all(USAGE.as_bytes());
        return Err(fail(SanError::new(OPTION_INVALID, "a folder is needed")));
    };
    let mut settings = match &options.config {
        Some(path) => read_settings(path).map_err(fail)?,
        None => SanitizerSettings::default(),
    };
    if let Some(threshold) = options.threshold {
        settings.threshold = threshold;
    }
    let report = sanitize(Path::new(&folder), &settings).map_err(fail)?;
    if options.apply {
        let plan = plan_of(&report);
        let files = read_folder(Path::new(&folder), &settings).map_err(fail)?;
        let apply_options = ApplyOptions {
            groups: options.groups.clone(),
            allow_near: options.allow_near,
        };
        let code = confirmation_code(&plan);
        let outcome = match (&options.confirm, &options.out) {
            (Some(confirm), Some(new_folder)) => {
                let outcome =
                    apply_plan(&plan, &files, confirm, &apply_options, &settings).map_err(fail)?;
                let written =
                    write_folder(&outcome.files, Path::new(new_folder), Path::new(&folder))
                        .map_err(fail)?;
                let _ = writeln!(
                    err,
                    "note: wrote {} libraries to {new_folder}",
                    written.len()
                );
                outcome
            }
            _ => preview_plan(&plan, &files, &apply_options, &settings).map_err(fail)?,
        };
        let dry = options.confirm.is_none();
        if options.json {
            let mut value: serde_json::Value =
                serde_json::from_str(&outcome.to_json()).unwrap_or_default();
            value["confirmation_code"] = serde_json::json!(code);
            value["dry_run"] = serde_json::json!(dry);
            let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
            text.push('\n');
            let _ = out.write_all(text.as_bytes());
        } else {
            let _ = writeln!(out, "confirmation code: {code}");
            let _ = out.write_all(outcome.to_text().as_bytes());
            if dry {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written; give --confirm {code} --out NEWFOLDER to write the copy"
                );
            }
        }
        return Ok(());
    }
    if let Some(base) = &options.out {
        let paths = write_outputs(&report, Path::new(base), lang, &settings).map_err(fail)?;
        for p in &paths {
            let _ = writeln!(out, "written: {}", p.display());
        }
    }
    let text = if options.json {
        report.to_json()
    } else {
        report.to_text()
    };
    let _ = out.write_all(text.as_bytes());
    Ok(())
}

/// Runs the command with the arguments that follow its name. Returns the exit code.
pub(crate) fn sanitize_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match run_inner(args, out, err) {
        Ok(()) => 0,
        Err((e, lang)) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}
