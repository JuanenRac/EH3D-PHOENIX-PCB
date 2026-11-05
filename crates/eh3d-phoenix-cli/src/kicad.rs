// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix kicad-read`: reads a KiCad file (a schematic with its sheets, a board, a
//! symbol library, a footprint or a project) into the design model and prints what was kept,
//! changed and left out.
//!
//! Exit codes: 0 the file was read (the report says what was changed or left out), 1 the file
//! could not be read (a damaged file, a file of another kind, a limit of the settings; the report
//! says which), 2 the arguments or the files are unusable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_kicad::{
    fidelity_of, is_safe_relative_name, title, KicadImporter, KicadSettings, Language, QualityItem,
};

/// The usage text of the command.
pub const KICAD_USAGE: &str = "\
phoenix kicad-read [--settings FILE] [--lang xx] [--redacted] <file>

  kicad-read  read a KiCad 6 to 9 file into the design model and summarise it: a schematic
              (.kicad_sch, with the sheets it refers to that lie below its folder), a board
              (.kicad_pcb), a symbol library (.kicad_sym), a footprint (.kicad_mod) or a project
              (.kicad_pro); the report lists what was kept, changed and left out, the findings
              with their codes and the fidelity level. --redacted prints only counts and codes.
              The limits come from FILE (config/kicad.cfg); --lang is one of en, es, de, fr, it,
              ja, zh, pt. Exit code 0: read; 1: could not be read; 2: unusable input
";

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

/// The settings of the readers: the defaults, or the settings file when one is named.
pub(crate) fn load_settings(
    path: Option<&str>,
    language: Language,
) -> Result<KicadSettings, String> {
    let Some(path) = path else {
        return Ok(KicadSettings::default());
    };
    let size = std::fs::metadata(path)
        .map_err(|e| format!("cannot read {path}: {e}"))?
        .len();
    if size > MAX_SETTINGS_BYTES {
        return Err(format!("{path} is larger than {MAX_SETTINGS_BYTES} bytes"));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    KicadSettings::parse(&text).map_err(|e| e.message(language))
}

fn usage(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    let _ = err.write_all(KICAD_USAGE.as_bytes());
    2
}

pub(crate) fn to_list(items: &[QualityItem]) -> Vec<(String, usize)> {
    items
        .iter()
        .map(|i| (i.category.clone(), i.count))
        .collect()
}

pub(crate) fn section(text: &mut String, heading: &str, items: &[(String, usize)]) {
    if items.is_empty() {
        return;
    }
    text.push_str(heading);
    text.push_str(":\n");
    for (name, count) in items {
        text.push_str(&format!("  {name}: {count}\n"));
    }
}

/// The command.
pub(crate) fn kicad_read(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut file: Option<&str> = None;
    let mut settings_path: Option<&str> = None;
    let mut language = Language::En;
    let mut redacted = false;
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--settings" => match iter.next() {
                Some(path) => settings_path = Some(path),
                None => return usage(err, "--settings needs a file"),
            },
            "--lang" => match iter.next().and_then(Language::from_tag) {
                Some(l) => language = l,
                None => return usage(err, "--lang needs en, es, de, fr, it, ja, zh or pt"),
            },
            "--redacted" => redacted = true,
            other if other.starts_with("--") => {
                return usage(err, &format!("unknown option {other}"))
            }
            other => {
                if file.is_some() {
                    return usage(err, "only one file can be read at a time");
                }
                file = Some(other);
            }
        }
    }
    let Some(file) = file else {
        return usage(err, "a file is needed");
    };
    let settings = match load_settings(settings_path, language) {
        Ok(settings) => settings,
        Err(message) => return usage(err, &message),
    };
    let path = Path::new(file);
    let max = settings.max_file_bytes;
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return usage(err, &format!("cannot read {file}: {e}")),
    };
    if size > max {
        return usage(err, &format!("{file} is {size} bytes, the limit is {max}"));
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return usage(err, &format!("cannot read {file}: {e}")),
    };
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut loader = |name: &str| -> Option<Vec<u8>> {
        if !is_safe_relative_name(name) {
            return None;
        }
        let full = folder.join(name.replace('\\', "/"));
        if std::fs::metadata(&full).ok()?.len() > max {
            return None;
        }
        std::fs::read(&full).ok()
    };
    let importer = KicadImporter { settings };
    let outcome = importer.import_with_sheets(&bytes, &mut loader);

    let mut text = String::new();
    text.push_str(&format!("format: {}\n", outcome.format));
    if !redacted {
        text.push_str(&format!("file: {file}\n"));
    }
    text.push_str(&format!(
        "kind: {}\n",
        if outcome.kind.is_empty() {
            "unknown"
        } else {
            &outcome.kind
        }
    ));
    if !outcome.version.is_empty() {
        text.push_str(&format!("version: {}\n", outcome.version));
    }
    text.push_str(&format!(
        "usable: {}\nfidelity: {}\n",
        if outcome.usable { "yes" } else { "no" },
        fidelity_of(outcome.usable, &outcome.quality).name()
    ));
    section(&mut text, "kept", &to_list(&outcome.quality.kept));
    section(&mut text, "changed", &to_list(&outcome.quality.transformed));
    section(&mut text, "left out", &to_list(&outcome.quality.dropped));
    text.push_str(&format!("findings: {}\n", outcome.diagnostics.len()));
    if redacted {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for d in &outcome.diagnostics {
            let code = d.code.as_deref().unwrap_or("-").to_owned();
            *counts.entry(code).or_insert(0) += 1;
        }
        for (code, count) in counts {
            text.push_str(&format!("  {code}: {count}\n"));
        }
    } else {
        for d in &outcome.diagnostics {
            let code = d.code.as_deref().unwrap_or("-");
            let line = if d.line > 0 {
                format!(" line {}", d.line)
            } else {
                String::new()
            };
            let severity = format!("{:?}", d.severity).to_lowercase();
            let head = title(code, language).unwrap_or("");
            text.push_str(&format!(
                "  {severity} {code}{line}: {head}: {}\n",
                d.message
            ));
        }
    }
    let _ = out.write_all(text.as_bytes());
    if outcome.usable {
        0
    } else {
        1
    }
}
