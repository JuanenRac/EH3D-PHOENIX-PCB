// SPDX-License-Identifier: GPL-3.0-or-later
//! The only way a script reaches a file.
//!
//! A path from a script is refused unless it is relative, stays below the folder of the script
//! (no `..`, no drive, no root, no link on the way) and lies inside one of the folders that the
//! settings allow. Nothing here runs a program.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::codes::{Diag, FILE_FAILED, PATH_REFUSED};
use crate::settings::ScriptSettings;

fn refused(why: &str) -> Diag {
    Diag::plain(PATH_REFUSED, why.to_owned())
}

/// The parts of a relative path with the empty ones and the `.` dropped; `None` when the path is
/// not a plain relative path.
fn segments(path: &str) -> Option<Vec<&str>> {
    if path.is_empty() || path.len() > 260 || path.contains('\0') || path.contains(':') {
        return None;
    }
    if path.starts_with('/') || path.starts_with('\\') || Path::new(path).is_absolute() {
        return None;
    }
    let mut out = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => return None,
            other => out.push(other),
        }
    }
    Some(out)
}

/// The path below `base` that `relative` names, when the settings allow it.
pub fn resolve(
    base: Option<&Path>,
    relative: &str,
    settings: &ScriptSettings,
) -> Result<PathBuf, Diag> {
    let Some(base) = base else {
        return Err(refused("there is no folder to take relative paths from"));
    };
    let Some(parts) = segments(relative) else {
        return Err(refused(
            "only a relative path without drive, root or .. is allowed",
        ));
    };
    if parts.is_empty() {
        return Err(refused("the path names no file"));
    }
    let inside = settings.allowed_dirs.iter().any(|dir| {
        segments(dir).is_some_and(|allowed| {
            allowed.len() < parts.len() && parts[..allowed.len()] == allowed[..]
        })
    });
    if !inside {
        return Err(refused("the path is not inside an allowed folder"));
    }
    let mut path = base.to_path_buf();
    for part in &parts {
        path.push(part);
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() {
                return Err(refused("a link on the way is not followed"));
            }
        }
    }
    Ok(path)
}

/// Reads a file of at most `limit` bytes.
pub fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, Diag> {
    let fail = |e: std::io::Error| Diag::plain(FILE_FAILED, format!("{}: {e}", path.display()));
    let file = std::fs::File::open(path).map_err(fail)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Diag::plain(FILE_FAILED, format!("{}: {e}", path.display())))?;
    if bytes.len() as u64 > limit {
        return Err(Diag::plain(
            FILE_FAILED,
            format!("{} is larger than {limit} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

/// Writes a new file; an existing file is never replaced. The folder is made when it is missing.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Diag> {
    let fail = |e: std::io::Error| Diag::plain(FILE_FAILED, format!("{}: {e}", path.display()));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(fail)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| Diag::plain(FILE_FAILED, format!("{}: {e}", path.display())))?;
    file.write_all(bytes)
        .map_err(|e| Diag::plain(FILE_FAILED, format!("{}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(dirs: &[&str]) -> ScriptSettings {
        ScriptSettings {
            allowed_dirs: dirs.iter().map(|d| (*d).to_owned()).collect(),
            ..ScriptSettings::default()
        }
    }

    #[test]
    fn nothing_is_allowed_when_no_folder_is_listed() {
        let dir = tempfile::tempdir().unwrap();
        let e = resolve(Some(dir.path()), "a.json", &settings(&[])).unwrap_err();
        assert_eq!(e.code, PATH_REFUSED);
    }

    #[test]
    fn a_path_below_an_allowed_folder_resolves_below_the_base() {
        let dir = tempfile::tempdir().unwrap();
        let s = settings(&["."]);
        let p = resolve(Some(dir.path()), "a/b.json", &s).unwrap();
        assert_eq!(p, dir.path().join("a").join("b.json"));
        let p = resolve(Some(dir.path()), "./a\\b.json", &s).unwrap();
        assert_eq!(p, dir.path().join("a").join("b.json"));
        let s = settings(&["out"]);
        assert!(resolve(Some(dir.path()), "out/x.json", &s).is_ok());
        assert!(resolve(Some(dir.path()), "other/x.json", &s).is_err());
        assert!(resolve(Some(dir.path()), "x.json", &s).is_err());
        assert!(resolve(Some(dir.path()), "out", &s).is_err());
    }

    #[test]
    fn absolute_paths_drives_parents_and_odd_names_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let s = settings(&["."]);
        for bad in [
            "",
            ".",
            "/etc/passwd",
            "\\windows\\x",
            "C:\\x.json",
            "c:x.json",
            "../x.json",
            "a/../../x.json",
            "a\\..\\x.json",
            "a\0b",
            "//server/share/x",
        ] {
            let e = resolve(Some(dir.path()), bad, &s).unwrap_err();
            assert_eq!(e.code, PATH_REFUSED, "{bad:?}");
        }
        let long = "a/".repeat(200);
        assert!(resolve(Some(dir.path()), &long, &s).is_err());
    }

    #[test]
    fn without_a_base_folder_nothing_resolves() {
        assert!(resolve(None, "a.json", &settings(&["."])).is_err());
    }

    #[test]
    fn reading_is_bounded_and_writing_never_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("f.txt");
        write_new(&path, b"hello").unwrap();
        assert_eq!(read_limited(&path, 5).unwrap(), b"hello");
        assert_eq!(read_limited(&path, 4).unwrap_err().code, FILE_FAILED);
        assert_eq!(write_new(&path, b"other").unwrap_err().code, FILE_FAILED);
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        let missing = dir.path().join("none.txt");
        assert_eq!(read_limited(&missing, 5).unwrap_err().code, FILE_FAILED);
    }
}
