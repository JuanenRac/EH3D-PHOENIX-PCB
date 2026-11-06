// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `gerber-import`: read Gerber RS-274X and Excellon files for review, and, with
//! `--roundtrip`, export a board and compare the files that come out with it.
//!
//! The files that are read are never written. `--out` writes the JSON summary to a file that does
//! not exist yet; an existing file, and any file that was given as an input, is refused.
//!
//! Exit codes: 0 done, 1 a file could not be read or the round trip found a difference, 2 the
//! arguments, the settings or the output are unusable.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_export::{Profile, ProfileSet};
use eh3d_phoenix_gerber::codes::{
    GbrError, INPUT_TOO_LARGE, INPUT_UNREADABLE, LIMIT_EXCEEDED, OPTION_INVALID, SETTINGS_INVALID,
};
use eh3d_phoenix_gerber::report::{has_errors, summary_json, summary_text};
use eh3d_phoenix_gerber::{
    import_file, round_trip, Content, ImportSettings, ImportedFile, ImportedSet, RoundTrip,
};
use eh3d_phoenix_rules::Lang;
use serde_json::json;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/gerber_import.cfg";

/// The profile file that is read when `--profiles` is not given and the file exists.
const DEFAULT_PROFILES: &str = "config/export_profiles.json";

/// The largest settings or profile file that is read, in bytes.
const MAX_SMALL_FILE_BYTES: u64 = 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix gerber-import [--config FILE] [--json] [--lang CODE] [--out FILE] <file or folder> ...
phoenix gerber-import [--config FILE] [--json] [--lang CODE] [--profile NAME] [--profiles FILE]
                      --roundtrip <board>

  gerber-import  read Gerber RS-274X and Excellon files as read-only geometry for review: the layer
                 of each file (from its X2 attributes or its name), the apertures, flashes, lines,
                 arcs and regions, the drill tools, holes and slots, with a code for what could not
                 be read; the files are only read. A folder is read file by file, not below
  --roundtrip    export the board with a profile, read the files back and compare them with the
                 board (exit 1 when they differ); nothing is written
  --config       the limits and defaults (default: config/gerber_import.cfg when it exists)
  --out FILE     write the JSON summary to a new file (an existing file is never replaced)
  --json         print the summary as JSON
  --lang CODE    the messages in en, es, de, fr, it, ja, zh or pt
";

#[derive(Default)]
struct Options {
    json: bool,
    lang: Option<Lang>,
    config: Option<String>,
    out: Option<String>,
    roundtrip: Option<String>,
    profile: Option<String>,
    profiles: Option<String>,
    inputs: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, GbrError> {
    let mut o = Options::default();
    let bad = |what: &str| GbrError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| -> Result<String, GbrError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(&format!("{name} needs a value")))
        };
        match *arg {
            "--json" => o.json = true,
            "--lang" => {
                let tag = value("--lang")?;
                o.lang = Some(Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--config" => o.config = Some(value("--config")?),
            "--out" => o.out = Some(value("--out")?),
            "--roundtrip" => o.roundtrip = Some(value("--roundtrip")?),
            "--profile" => o.profile = Some(value("--profile")?),
            "--profiles" => o.profiles = Some(value("--profiles")?),
            other if other.starts_with("--") => {
                return Err(GbrError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.inputs.push(other.to_owned()),
        }
    }
    if o.roundtrip.is_some() && !o.inputs.is_empty() {
        return Err(bad("--roundtrip takes a board and no files"));
    }
    if o.roundtrip.is_none() && (o.profile.is_some() || o.profiles.is_some()) {
        return Err(bad("--profile and --profiles go with --roundtrip"));
    }
    Ok(o)
}

fn small_text(path: &str) -> Result<String, GbrError> {
    let size = std::fs::metadata(path)
        .map_err(|e| GbrError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?
        .len();
    if size > MAX_SMALL_FILE_BYTES {
        return Err(GbrError::new(
            INPUT_TOO_LARGE,
            format!("{path} is {size} bytes, the limit is {MAX_SMALL_FILE_BYTES}"),
        ));
    }
    std::fs::read_to_string(path)
        .map_err(|e| GbrError::new(INPUT_UNREADABLE, format!("{path}: {e}")))
}

pub(crate) fn load_settings(path: Option<&str>) -> Result<ImportSettings, GbrError> {
    let chosen = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match chosen {
        Some(path) => {
            let text = small_text(path).map_err(|e| {
                if e.code == INPUT_UNREADABLE || e.code == INPUT_TOO_LARGE {
                    GbrError::new(SETTINGS_INVALID, e.detail)
                } else {
                    e
                }
            })?;
            ImportSettings::parse(&text)
        }
        None => Ok(ImportSettings::default()),
    }
}

fn load_profile(options: &Options) -> Result<Profile, GbrError> {
    let name = options.profile.as_deref().unwrap_or("generic");
    let explicit = options.profiles.as_deref();
    let path = explicit.or_else(|| {
        Path::new(DEFAULT_PROFILES)
            .is_file()
            .then_some(DEFAULT_PROFILES)
    });
    let Some(path) = path else {
        return if name == "generic" {
            Ok(Profile::default())
        } else {
            Err(GbrError::new(
                OPTION_INVALID,
                format!("profile {name} needs a profile file (--profiles)"),
            ))
        };
    };
    let text = small_text(path)?;
    let set = ProfileSet::from_json(&text)
        .map_err(|e| GbrError::new(OPTION_INVALID, e.diagnostic().message))?;
    set.get(name)
        .cloned()
        .map_err(|e| GbrError::new(OPTION_INVALID, e.diagnostic().message))
}

/// The files to read: each input that is a file, and the files of each input that is a folder.
pub(crate) fn collect(
    inputs: &[String],
    settings: &ImportSettings,
) -> Result<Vec<(String, PathBuf)>, GbrError> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for input in inputs {
        let path = PathBuf::from(input);
        if path.is_dir() {
            let mut names: Vec<PathBuf> = Vec::new();
            let entries = std::fs::read_dir(&path)
                .map_err(|e| GbrError::new(INPUT_UNREADABLE, format!("{input}: {e}")))?;
            for entry in entries {
                let entry =
                    entry.map_err(|e| GbrError::new(INPUT_UNREADABLE, format!("{input}: {e}")))?;
                let file = entry.path();
                if file.is_file() {
                    names.push(file);
                }
                if names.len() > settings.max_files {
                    break;
                }
            }
            names.sort();
            for file in names {
                let name = file
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                found.push((name, file));
            }
        } else {
            let name = path
                .file_name()
                .map_or_else(|| input.clone(), |n| n.to_string_lossy().into_owned());
            found.push((name, path));
        }
        if found.len() > settings.max_files {
            return Err(GbrError::new(
                LIMIT_EXCEEDED,
                format!("more than {} files", settings.max_files),
            ));
        }
    }
    Ok(found)
}

fn read_all(entries: &[(String, PathBuf)], settings: &ImportSettings) -> ImportedSet {
    let mut files: Vec<ImportedFile> = Vec::new();
    for (name, path) in entries {
        let length = std::fs::metadata(path).map(|m| m.len());
        let failed = |error: GbrError| ImportedFile {
            name: name.clone(),
            identified: None,
            content: Content::Failed(error),
        };
        match length {
            Err(e) => files.push(failed(GbrError::new(
                INPUT_UNREADABLE,
                format!("{}: {e}", path.display()),
            ))),
            Ok(length) if length > settings.max_file_bytes as u64 => {
                files.push(failed(GbrError::new(
                    INPUT_TOO_LARGE,
                    format!("{length} bytes, the limit is {}", settings.max_file_bytes),
                )))
            }
            Ok(_) => match std::fs::read(path) {
                Ok(bytes) => files.push(import_file(name, &bytes, settings)),
                Err(e) => files.push(failed(GbrError::new(
                    INPUT_UNREADABLE,
                    format!("{}: {e}", path.display()),
                ))),
            },
        }
    }
    ImportedSet { files }
}

/// True when `candidate` names the same file as one of `inputs` (or the folder of one).
fn is_input(candidate: &Path, inputs: &[String]) -> bool {
    let Ok(wanted) = candidate.canonicalize() else {
        return false;
    };
    inputs
        .iter()
        .any(|i| Path::new(i).canonicalize().is_ok_and(|p| p == wanted))
}

fn write_new(path: &str, text: &str, inputs: &[String]) -> Result<(), GbrError> {
    if is_input(Path::new(path), inputs) {
        return Err(GbrError::new(
            OPTION_INVALID,
            format!("{path} is one of the inputs"),
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| GbrError::new(OPTION_INVALID, format!("{path}: {e}")))?;
    file.write_all(text.as_bytes())
        .map_err(|e| GbrError::new(OPTION_INVALID, format!("{path}: {e}")))
}

fn round_trip_json(trip: &RoundTrip) -> serde_json::Value {
    let differences: Vec<serde_json::Value> = trip
        .differences
        .iter()
        .map(|d| json!({ "file": d.file, "what": d.what }))
        .collect();
    json!({ "files": trip.files, "differences": differences })
}

pub(crate) fn gerber_import_command(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang.unwrap_or(Lang::En);
    if options.inputs.is_empty() && options.roundtrip.is_none() {
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
    if let Some(board_path) = &options.roundtrip {
        return roundtrip_command(&options, board_path, &settings, lang, out, err);
    }
    let entries = match collect(&options.inputs, &settings) {
        Ok(entries) => entries,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let set = read_all(&entries, &settings);
    let json_text = serde_json::to_string_pretty(&summary_json(&set)).unwrap_or_default() + "\n";
    if let Some(path) = &options.out {
        if let Err(error) = write_new(path, &json_text, &options.inputs) {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    }
    let text = if options.json {
        json_text
    } else {
        summary_text(&set, lang)
    };
    let _ = out.write_all(text.as_bytes());
    i32::from(has_errors(&set))
}

fn roundtrip_command(
    options: &Options,
    board_path: &str,
    settings: &ImportSettings,
    lang: Lang,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let profile = match load_profile(options) {
        Ok(profile) => profile,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let board = match read_board_file(board_path) {
        Ok(board) => board,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return 2;
        }
    };
    let trip = match round_trip(&board, &profile, settings) {
        Ok(trip) => trip,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let mut value = summary_json(&trip.imported);
    value["roundtrip"] = round_trip_json(&trip);
    let json_text = serde_json::to_string_pretty(&value).unwrap_or_default() + "\n";
    if let Some(path) = &options.out {
        let inputs = [board_path.to_owned()];
        if let Err(error) = write_new(path, &json_text, &inputs) {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    }
    if options.json {
        let _ = out.write_all(json_text.as_bytes());
    } else {
        let mut text = summary_text(&trip.imported, lang);
        text.push_str(&format!(
            "round trip: {} files read back, {} differences\n",
            trip.files,
            trip.differences.len()
        ));
        for finding in trip.difference_findings() {
            text.push_str(&format!("  {}\n", finding.message(lang)));
        }
        let _ = out.write_all(text.as_bytes());
    }
    i32::from(!trip.differences.is_empty() || has_errors(&trip.imported))
}
