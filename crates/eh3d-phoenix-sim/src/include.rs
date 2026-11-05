// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading a netlist from a file, with `.include` and with a separate file of models.
//!
//! An `.include` is allowed only for a file that sits in the same folder as the file that names
//! it: the name is a plain file name (no folder, no `..`, no drive, no absolute path) and the
//! file, once its links are resolved, must still be in that folder. The depth of the nesting and
//! the number of files are bounded by the settings (`max_include_depth`, `max_include_files`),
//! which also stops a file that includes itself. The text of the files is put in place of the
//! `.include` line and read as one netlist; an error then names the line of the file it comes
//! from, and the file when it is not the main one.

use std::fs;
use std::path::{Path, PathBuf};

use crate::codes::{SimError, INCLUDE_REFUSED, INPUT_UNREADABLE};
use crate::netlist::Netlist;
use crate::parse::parse;
use crate::settings::SimSettings;

/// Where a line of the expanded text comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Origin {
    /// The name of the file as written (the path for the main file).
    file: String,
    /// The line in that file, counted from 1.
    line: usize,
}

/// The expanded lines and the files read so far.
struct Expansion {
    lines: Vec<(String, Origin)>,
    files: usize,
}

fn unreadable(path: &Path, why: impl std::fmt::Display) -> SimError {
    SimError::new(INPUT_UNREADABLE, format!("{}: {why}", path.display()))
}

fn read_text(path: &Path, settings: &SimSettings) -> Result<String, SimError> {
    let length = fs::metadata(path).map_err(|e| unreadable(path, e))?.len();
    if length > settings.max_file_bytes {
        return Err(unreadable(
            path,
            format!(
                "the file is larger than max_file_bytes ({})",
                settings.max_file_bytes
            ),
        ));
    }
    let bytes = fs::read(path).map_err(|e| unreadable(path, e))?;
    String::from_utf8(bytes).map_err(|_| unreadable(path, "the file is not UTF-8 text"))
}

/// The name an `.include` line asks for, without quotes, when the line is an `.include`.
fn include_name(line: &str) -> Option<String> {
    let body = line.split(';').next().unwrap_or("").trim();
    let mut words = body.split_whitespace();
    let first = words.next()?.to_ascii_lowercase();
    if first != ".include" && first != ".inc" {
        return None;
    }
    let rest = body[first.len()..].trim();
    Some(rest.trim_matches(['"', '\'']).to_string())
}

/// True when the first word of the line is `.end`.
fn is_end(line: &str) -> bool {
    let body = line.split(';').next().unwrap_or("");
    body.split_whitespace()
        .next()
        .is_some_and(|w| w.eq_ignore_ascii_case(".end"))
}

/// Checks that a name is a plain file name and finds the file in `dir`.
fn resolve(dir: &Path, name: &str, origin: &Origin) -> Result<PathBuf, SimError> {
    let refuse = |why: &str| {
        SimError::at(
            INCLUDE_REFUSED,
            origin.line,
            format!("{}: .include {name}: {why}", origin.file),
        )
    };
    if name.is_empty() {
        return Err(refuse("a file name is needed"));
    }
    let plain = !name.contains(['/', '\\', ':', '\0']) && name != "." && !name.contains("..");
    if !plain {
        return Err(refuse(
            "only a file in the same folder is allowed (a plain file name, no folder, no '..', no drive)",
        ));
    }
    let path = dir.join(name);
    let found = path
        .canonicalize()
        .map_err(|e| refuse(&format!("the file cannot be found ({e})")))?;
    if !found.is_file() {
        return Err(refuse("it is not a file"));
    }
    let folder = dir
        .canonicalize()
        .map_err(|e| refuse(&format!("the folder cannot be read ({e})")))?;
    if found.parent() != Some(folder.as_path()) {
        return Err(refuse("the file is not in the same folder"));
    }
    Ok(path)
}

/// Adds the lines of a file, replacing each `.include` by the lines of its file. With
/// `is_main` the first line is the title and is kept as it is; an included file contributes all
/// its lines except its `.end`.
fn expand(
    path: &Path,
    shown: &str,
    is_main: bool,
    depth: usize,
    settings: &SimSettings,
    out: &mut Expansion,
) -> Result<(), SimError> {
    out.files += 1;
    if out.files > settings.max_include_files {
        return Err(SimError::new(
            INCLUDE_REFUSED,
            format!(
                "{shown}: more than max_include_files ({}) files are read",
                settings.max_include_files
            ),
        ));
    }
    let text = read_text(path, settings)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    for (index, raw) in text.lines().enumerate() {
        let origin = Origin {
            file: shown.to_string(),
            line: index + 1,
        };
        if is_main && index == 0 {
            out.lines.push((raw.to_string(), origin));
            continue;
        }
        if let Some(name) = include_name(raw) {
            if depth >= settings.max_include_depth {
                return Err(SimError::at(
                    INCLUDE_REFUSED,
                    origin.line,
                    format!(
                        "{shown}: .include {name}: nested deeper than max_include_depth ({}); does a file include itself?",
                        settings.max_include_depth
                    ),
                ));
            }
            let target = resolve(&dir, &name, &origin)?;
            expand(&target, &name, false, depth + 1, settings, out)?;
            continue;
        }
        if !is_main && is_end(raw) {
            continue;
        }
        if out.lines.len() >= settings.max_lines {
            return Err(SimError::at(
                crate::codes::LIMIT_EXCEEDED,
                origin.line,
                format!(
                    "{shown}: more than max_lines ({}) lines",
                    settings.max_lines
                ),
            ));
        }
        out.lines.push((raw.to_string(), origin));
    }
    Ok(())
}

/// Reads a netlist from a file. `.include` lines are replaced by the files they name (same folder
/// only). With `models` the lines of that file (any folder; its own `.include` lines follow the
/// same rule relative to it) are put right after the title, before the netlist: a library of
/// `.model` and `.subckt` lines that the netlist uses.
pub fn read_netlist_file(
    path: &Path,
    models: Option<&Path>,
    settings: &SimSettings,
) -> Result<Netlist, SimError> {
    let shown = path.display().to_string();
    let mut main = Expansion {
        lines: Vec::new(),
        files: 0,
    };
    expand(path, &shown, true, 0, settings, &mut main)?;
    let mut lines = main.lines;
    if let Some(library) = models {
        let mut extra = Expansion {
            lines: Vec::new(),
            files: main.files,
        };
        expand(
            library,
            &library.display().to_string(),
            false,
            0,
            settings,
            &mut extra,
        )?;
        let at = lines.len().min(1);
        lines.splice(at..at, extra.lines);
    }
    let mut text = String::new();
    for (line, _) in &lines {
        text.push_str(line);
        text.push('\n');
    }
    parse(&text, settings).map_err(|mut error| {
        if let Some(number) = error.line {
            if let Some((_, origin)) = lines.get(number - 1) {
                error.line = Some(origin.line);
                if origin.file != shown {
                    error.detail = format!("[{}] {}", origin.file, error.detail);
                }
            }
        }
        error
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{DUPLICATE_NAME, NETLIST_SYNTAX, UNDEFINED};

    /// A folder of its own for one test.
    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("phoenix-sim-include-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn read(path: &Path) -> Result<Netlist, SimError> {
        read_netlist_file(path, None, &SimSettings::default())
    }

    #[test]
    fn a_file_in_the_same_folder_is_included() {
        let dir = folder("same");
        put(&dir, "parts.lib", "* models\n.model dm D(IS=1e-14)\n.end\n");
        let main = put(
            &dir,
            "main.cir",
            "t\n.include parts.lib\nV1 a 0 5\nR1 a d 1k\nD1 d 0 dm\n",
        );
        let nl = read(&main).unwrap();
        assert!(nl.models.contains_key("dm"));
        assert_eq!(nl.elements.len(), 3);
        // the same with quotes and the short form
        let quoted = put(
            &dir,
            "quoted.cir",
            "t\n.INC \"parts.lib\"\nV1 a 0 5\nR1 a d 1k\nD1 d 0 dm\n",
        );
        assert_eq!(read(&quoted).unwrap(), nl);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_error_names_the_line_and_the_file_it_comes_from() {
        let dir = folder("lines");
        put(&dir, "bad.lib", "* one\n* two\n.model dm D(IS=oops)\n");
        let main = put(&dir, "main.cir", "t\nR1 a 0 1k\n.include bad.lib\n");
        let e = read(&main).unwrap_err();
        assert_eq!(e.code, crate::codes::VALUE_INVALID);
        assert_eq!(e.line, Some(3));
        assert!(e.detail.starts_with("[bad.lib]"), "{}", e.detail);
        // an error of the main file keeps its own line even after an include above it
        put(&dir, "ok.lib", "* a\n* b\n* c\n");
        let after = put(&dir, "after.cir", "t\n.include ok.lib\nR1 a 0\n");
        let e = read(&after).unwrap_err();
        assert_eq!(e.code, NETLIST_SYNTAX);
        assert_eq!(e.line, Some(3));
        assert!(!e.detail.starts_with('['), "{}", e.detail);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn other_folders_and_odd_names_are_refused() {
        let dir = folder("refused");
        fs::create_dir_all(dir.join("sub")).unwrap();
        put(&dir.join("sub"), "x.lib", "* x\n");
        put(&dir, "ok.lib", "* x\n");
        for name in [
            "sub/x.lib",
            "sub\\x.lib",
            "../x.lib",
            "..\\x.lib",
            "..",
            ".",
            "/etc/passwd",
            "C:x.lib",
            "C:\\x.lib",
            "",
        ] {
            let main = put(
                &dir,
                "main.cir",
                &format!("t\nR1 a 0 1k\n.include {name}\n"),
            );
            let e = read(&main).unwrap_err();
            assert_eq!(e.code, INCLUDE_REFUSED, "{name}: {e}");
            assert_eq!(e.line, Some(3), "{name}");
        }
        let missing = put(&dir, "missing.cir", "t\nR1 a 0 1k\n.include nothing.lib\n");
        assert_eq!(read(&missing).unwrap_err().code, INCLUDE_REFUSED);
        let folder_name = put(&dir, "folder.cir", "t\nR1 a 0 1k\n.include sub\n");
        assert_eq!(read(&folder_name).unwrap_err().code, INCLUDE_REFUSED);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_that_includes_itself_and_too_many_files_are_stopped() {
        let dir = folder("loops");
        let main = put(&dir, "loop.cir", "t\nR1 a 0 1k\n.include loop.cir\n");
        let e = read(&main).unwrap_err();
        assert_eq!(e.code, INCLUDE_REFUSED);
        assert!(e.detail.contains("max_include_depth"), "{}", e.detail);
        let s = SimSettings {
            max_include_depth: 8,
            max_include_files: 3,
            ..SimSettings::default()
        };
        let e = read_netlist_file(&main, None, &s).unwrap_err();
        assert_eq!(e.code, INCLUDE_REFUSED);
        assert!(e.detail.contains("max_include_files"), "{}", e.detail);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_model_file_goes_in_after_the_title() {
        let dir = folder("models");
        let library = put(
            &dir,
            "models.lib",
            "* library\n.model dm D(IS=1e-14)\n.model nm NMOS(VTO=1)\n",
        );
        let main = put(&dir, "main.cir", "t\nV1 a 0 5\nR1 a d 1k\nD1 d 0 dm\n");
        let s = SimSettings::default();
        let nl = read_netlist_file(&main, Some(&library), &s).unwrap();
        assert_eq!(nl.title, "t");
        assert_eq!(nl.models.len(), 2);
        // without it the model is not there, and the error names the line of the main file
        let e = read_netlist_file(&main, None, &s).unwrap_err();
        assert_eq!(e.code, UNDEFINED);
        assert_eq!(e.line, Some(4));
        // a model defined in both files is a duplicate and names the file of the second one
        let twice = put(&dir, "twice.cir", "t\n.model dm D\nD1 a 0 dm\nR1 a 0 1\n");
        let e = read_netlist_file(&twice, Some(&library), &s).unwrap_err();
        assert_eq!(e.code, DUPLICATE_NAME);
        assert_eq!(e.line, Some(2));
        // an unreadable model file is an input error
        let e = read_netlist_file(&main, Some(&dir.join("nothing.lib")), &s).unwrap_err();
        assert_eq!(e.code, INPUT_UNREADABLE);
        fs::remove_dir_all(&dir).unwrap();
    }
}
