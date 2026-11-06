// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `parts`: the local parts base. `search` and `show` read it, `check` looks for what
//! is wrong in it, `import` reads a CSV file into a NEW base and `export` writes a base to a NEW
//! file. Nothing is ever written over a file that exists or over an input file, and nothing is
//! fetched from a network.
//!
//! Exit codes: 0 done, 1 `check` found an error or `show` did not find the part, 2 the arguments,
//! the settings, the files or the output are unusable.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_eagle::document::{read_bytes, DocumentKind};
use eh3d_phoenix_eagle::Limits;
use eh3d_phoenix_ipclib::{library, FamilySet};
use eh3d_phoenix_parts::codes::{
    FILE_UNUSABLE, FORMAT_INVALID, OPTION_INVALID, OUTPUT_FAILED, PART_NOT_FOUND,
};
use eh3d_phoenix_parts::report::{
    check_json, check_text, counts, search_json, search_text, show_text,
};
use eh3d_phoenix_parts::write::part_json;
use eh3d_phoenix_parts::{
    check_db, load, parse_any, to_csv, to_json, write_new, CheckContext, Index, PartsDb,
    PartsError, PartsSettings, Query, Status,
};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sim::ModelTable;

use crate::design::same_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/parts.cfg";

pub(crate) const USAGE: &str = "\
phoenix parts search [--base PATH] [--value V] [--footprint F] [--mpn TEXT] [--maker TEXT]
                     [--status active|not-recommended|obsolete] [--limit N] [--json] [WORDS...]
phoenix parts show   [--base PATH] [--json] <id>
phoenix parts check  [--base PATH] [--lib PATH ...] [--no-ipc] [--models FILE] [--json] [--lang xx]
phoenix parts import [--out NEW.json] [--config FILE] <file.csv | file.json>
phoenix parts export [--out NEW.csv | NEW.json] [--base PATH]

  parts      the local parts base: components kept in JSON (phoenix-parts/1) and CSV files of your
             own, in one file or one folder (--base, default parts), with value, tolerance, power,
             voltage, footprint, symbol, maker, part number, free parameters, simulation model,
             suppliers with prices by quantity break, state (active, not-recommended, obsolete),
             notes and alternatives. Nothing is fetched from a network.
  search     the parts whose text holds all the WORDS and that pass the filters, in the order of
             the ids; a value is compared without case, blanks or the ohm sign
  show       one part in full
  check      what is wrong in the base (PHOENIX-PARTS-007 to 013): ids used twice, footprints that
             are not in the IPC library (unless --no-ipc) or in the libraries given with --lib,
             part numbers with other data, prices without a currency, price breaks out of order,
             simulation models that the table (--models, default the built-in one) does not have
             and alternatives that do not exist. Exit code 1 when there is an error
  import     read a CSV or JSON file into a base; a dry run unless --out names a NEW .json file
  export     write the base as CSV or JSON; a dry run unless --out names a NEW file
  --config   the limits (default: config/parts.cfg when it exists)
  --lang xx  the messages in en, es, de, fr, it, ja, zh or pt
  The files that the base is read from are never changed, and a file that exists is never
  replaced. To complete a bill of materials from the base use `phoenix bom --parts-db PATH`.
";

#[derive(Default)]
struct Options {
    sub: String,
    base: Option<String>,
    config: Option<String>,
    lang: Option<Lang>,
    json: bool,
    value: String,
    footprint: String,
    mpn: String,
    maker: String,
    status: Option<Status>,
    limit: Option<usize>,
    libs: Vec<String>,
    models: Option<String>,
    no_ipc: bool,
    out: Option<String>,
    positional: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, PartsError> {
    let bad = |what: &str| PartsError::new(OPTION_INVALID, what.to_owned());
    let Some((sub, rest)) = args.split_first() else {
        return Err(bad(
            "a subcommand is needed: search, show, check, import or export",
        ));
    };
    if !matches!(*sub, "search" | "show" | "check" | "import" | "export") {
        return Err(bad(&format!("unknown subcommand {sub}")));
    }
    let mut options = Options {
        sub: (*sub).to_owned(),
        ..Options::default()
    };
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, PartsError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--no-ipc" => options.no_ipc = true,
            "--base" => options.base = Some(value("--base needs a path")?),
            "--config" => options.config = Some(value("--config needs a file")?),
            "--value" => options.value = value("--value needs a value")?,
            "--footprint" => options.footprint = value("--footprint needs a name")?,
            "--mpn" => options.mpn = value("--mpn needs a text")?,
            "--maker" => options.maker = value("--maker needs a text")?,
            "--lib" => options.libs.push(value("--lib needs a path")?),
            "--models" => options.models = Some(value("--models needs a file")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang =
                    Some(Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--status" => {
                let word = value("--status needs a word")?;
                options.status = Some(
                    Status::parse(&word)
                        .ok_or_else(|| bad("--status is active, not-recommended or obsolete"))?,
                );
            }
            "--limit" => {
                let n = value("--limit needs a number")?;
                options.limit = Some(
                    n.parse::<usize>()
                        .ok()
                        .filter(|n| *n >= 1)
                        .ok_or_else(|| bad("--limit is a whole number from 1"))?,
                );
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => options.positional.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn read_limited(path: &str, limit: u64) -> Result<String, PartsError> {
    let unusable = |e: String| PartsError::new(FILE_UNUSABLE, format!("{path}: {e}"));
    let size = std::fs::metadata(path)
        .map_err(|e| unusable(e.to_string()))?
        .len();
    if size > limit {
        return Err(unusable(format!("larger than {limit} bytes")));
    }
    std::fs::read_to_string(path).map_err(|e| unusable(e.to_string()))
}

fn settings_of(config: Option<&str>) -> Result<PartsSettings, PartsError> {
    let path = config.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        Some(path) => PartsSettings::parse(&read_limited(path, 1024 * 1024)?),
        None => Ok(PartsSettings::default()),
    }
}

/// The names of the footprints of the libraries `.lbr` of a file or of a folder (the files of the
/// top of the folder only).
fn own_footprints(path: &str, limit: u64, names: &mut BTreeSet<String>) -> Result<(), PartsError> {
    let unusable = |e: String| PartsError::new(FILE_UNUSABLE, format!("{path}: {e}"));
    let meta = std::fs::metadata(path).map_err(|e| unusable(e.to_string()))?;
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    if meta.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| unusable(e.to_string()))? {
            let entry = entry.map_err(|e| unusable(e.to_string()))?;
            let file = entry.path();
            let is_library = file
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("lbr"));
            if is_library && entry.file_type().is_ok_and(|t| t.is_file()) {
                files.push(file);
            }
        }
        files.sort();
    } else {
        files.push(path.into());
    }
    for file in files {
        let shown = file.display().to_string();
        let size = std::fs::metadata(&file)
            .map_err(|e| unusable(format!("{shown}: {e}")))?
            .len();
        if size > limit {
            return Err(unusable(format!("{shown}: larger than {limit} bytes")));
        }
        let bytes = std::fs::read(&file).map_err(|e| unusable(format!("{shown}: {e}")))?;
        let reading = read_bytes(&bytes, &Limits::default());
        match reading.summary {
            Some(summary) if summary.kind == DocumentKind::Library => {
                names.extend(summary.packages.into_iter().map(|p| p.name));
            }
            _ => return Err(unusable(format!("{shown}: not an EAGLE library"))),
        }
    }
    Ok(())
}

fn out_path_ok(out: &str, inputs: &[&str]) -> Result<(), PartsError> {
    for input in inputs {
        if same_file(out, input) {
            return Err(PartsError::new(
                OUTPUT_FAILED,
                format!("{out} is an input file; give another file"),
            ));
        }
    }
    if Path::new(out).exists() {
        return Err(PartsError::new(
            OUTPUT_FAILED,
            format!("{out} exists and is never replaced"),
        ));
    }
    Ok(())
}

fn run_inner(
    options: &Options,
    settings: &PartsSettings,
    lang: Lang,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, PartsError> {
    let bad = |what: &str| PartsError::new(OPTION_INVALID, what.to_owned());
    let base = options.base.as_deref().unwrap_or("parts");
    match options.sub.as_str() {
        "search" => {
            let db = load(Path::new(base), settings)?;
            let index = Index::build(&db);
            let limit = options
                .limit
                .unwrap_or(settings.max_results)
                .min(settings.max_results);
            let found = index.search(&Query {
                text: options.positional.join(" "),
                value: options.value.clone(),
                footprint: options.footprint.clone(),
                mpn: options.mpn.clone(),
                manufacturer: options.maker.clone(),
                status: options.status,
                limit,
            });
            let text = if options.json {
                search_json(&found)
            } else {
                search_text(&found)
            };
            let _ = out.write_all(text.as_bytes());
            Ok(0)
        }
        "show" => {
            let [id] = options.positional.as_slice() else {
                return Err(bad("give exactly one id"));
            };
            let db = load(Path::new(base), settings)?;
            let index = Index::build(&db);
            let Some(part) = index.get(id) else {
                let e = PartsError::new(PART_NOT_FOUND, id.clone());
                let _ = writeln!(err, "error: {}", e.message(lang));
                return Ok(1);
            };
            let text = if options.json {
                let mut text = serde_json::to_string_pretty(&part_json(part)).unwrap_or_default();
                text.push('\n');
                text
            } else {
                show_text(part)
            };
            let _ = out.write_all(text.as_bytes());
            Ok(0)
        }
        "check" => {
            if !options.positional.is_empty() {
                return Err(bad("check takes no file; use --base"));
            }
            let db = load(Path::new(base), settings)?;
            let mut names: BTreeSet<String> = BTreeSet::new();
            let mut world = !options.libs.is_empty();
            for lib in &options.libs {
                own_footprints(lib, settings.max_file_bytes, &mut names)?;
            }
            if !options.no_ipc {
                let set = FamilySet::builtin()
                    .map_err(|e| PartsError::new(FILE_UNUSABLE, e.to_string()))?;
                let footprints =
                    library(&set).map_err(|e| PartsError::new(FILE_UNUSABLE, e.to_string()))?;
                names.extend(footprints.into_iter().map(|f| f.name));
                world = true;
            }
            let table = match &options.models {
                Some(path) => ModelTable::parse(&read_limited(path, settings.max_file_bytes)?)
                    .map_err(|e| PartsError::new(FILE_UNUSABLE, format!("{path}: {e:?}")))?,
                None => ModelTable::built_in(),
            };
            let footprint_known = |name: &str| names.contains(name);
            let model_known = |name: &str| table.model_line(name).is_some();
            let findings = check_db(
                &db,
                &CheckContext {
                    footprint_known: world.then_some(&footprint_known as &dyn Fn(&str) -> bool),
                    sim_model_known: Some(&model_known),
                },
            );
            let text = if options.json {
                check_json(&findings, db.parts.len())
            } else {
                check_text(&findings, db.parts.len(), lang)
            };
            let _ = out.write_all(text.as_bytes());
            let (errors, _, _) = counts(&findings);
            Ok(i32::from(errors > 0))
        }
        "import" => {
            let [input] = options.positional.as_slice() else {
                return Err(bad("give exactly one file to import"));
            };
            let text = read_limited(input, settings.max_file_bytes)?;
            let parts = parse_any(input, &text, settings)?;
            let db = PartsDb { parts };
            let findings = check_db(&db, &CheckContext::default());
            let (errors, _, _) = counts(&findings);
            let _ = writeln!(out, "{} parts read from {input}", db.parts.len());
            let _ = out.write_all(check_text(&findings, db.parts.len(), lang).as_bytes());
            match &options.out {
                None => {
                    let _ = writeln!(out, "dry run: nothing was written; give --out NEW.json");
                    Ok(i32::from(errors > 0))
                }
                Some(target) => {
                    if !target.to_ascii_lowercase().ends_with(".json") {
                        return Err(bad("--out of an import is a .json file"));
                    }
                    out_path_ok(target, &[input.as_str()])?;
                    if errors > 0 {
                        return Err(PartsError::new(
                            FORMAT_INVALID,
                            "the base has errors: nothing was written",
                        ));
                    }
                    write_new(Path::new(target), &to_json(&db.parts))?;
                    let _ = writeln!(out, "written: {target}");
                    Ok(0)
                }
            }
        }
        "export" => {
            if !options.positional.is_empty() {
                return Err(bad("export takes no file; use --base"));
            }
            let db = load(Path::new(base), settings)?;
            let Some(target) = &options.out else {
                let _ = writeln!(
                    out,
                    "{} parts in {base}; dry run: nothing was written; give --out NEW.csv or NEW.json",
                    db.parts.len()
                );
                return Ok(0);
            };
            let lower = target.to_ascii_lowercase();
            let text = if lower.ends_with(".json") {
                to_json(&db.parts)
            } else if lower.ends_with(".csv") {
                to_csv(&db.parts)?
            } else {
                return Err(bad("--out of an export is a .csv or .json file"));
            };
            out_path_ok(target, &[base])?;
            write_new(Path::new(target), &text)?;
            let _ = writeln!(out, "written: {target} ({} parts)", db.parts.len());
            Ok(0)
        }
        _ => Err(bad("unknown subcommand")),
    }
}

/// Runs the command with the arguments that follow its name. Returns the exit code.
pub(crate) fn parts_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(e) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", e.message(Lang::En));
            return 2;
        }
    };
    let lang = options.lang.unwrap_or(Lang::En);
    let settings = match settings_of(options.config.as_deref()) {
        Ok(settings) => settings,
        Err(e) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            return 2;
        }
    };
    match run_inner(&options, &settings, lang, out, err) {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "search", "--value", "10k", "--status", "nrnd", "--limit", "5", "--lang", "pt", "0603",
            "x7r",
        ])
        .unwrap();
        assert_eq!(o.sub, "search");
        assert_eq!(o.value, "10k");
        assert_eq!(o.status, Some(Status::NotRecommended));
        assert_eq!(o.limit, Some(5));
        assert_eq!(o.lang, Some(Lang::Pt));
        assert_eq!(o.positional, vec!["0603".to_owned(), "x7r".to_owned()]);
        for bad in [
            vec![],
            vec!["nope"],
            vec!["search", "--limit", "0"],
            vec!["search", "--limit", "x"],
            vec!["search", "--status", "maybe"],
            vec!["search", "--lang", "xx"],
            vec!["search", "--base"],
            vec!["search", "--nope"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
    }
}
