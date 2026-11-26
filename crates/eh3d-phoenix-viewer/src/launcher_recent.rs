// SPDX-License-Identifier: GPL-3.0-or-later
//! The list of recent files, and where the window keeps its files. There is one list, the one of
//! the menu *Open recent* of the frame of the suite, which the start screen shows too. Earlier
//! versions kept a second list for the start screen (a plain text file, one path per line, the
//! newest first, named by the setting `recent_file`): its entries are moved into the one list the
//! first time. How many entries the list keeps is the setting `recent_max`.

use std::io::Read;
use std::path::{Path, PathBuf};

use eh3d_dock::core::DockCore;
use eh3d_dock::recent_store::RecentStore;

/// The folder of the settings of the user is named like this under the application data folder
/// of Windows.
const FOLDER_WINDOWS: &str = "EH3D-PHOENIX-PCB";
/// And like this under the configuration folder of the other systems.
const FOLDER_OTHER: &str = "eh3d-phoenix-pcb";
/// The name of the file where the window writes an error that kept it from opening.
pub const ERROR_FILE: &str = "last_error.txt";

/// What a file is for the start screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// An EAGLE board (`.brd`).
    Board,
    /// An EAGLE schematic (`.sch`).
    Schematic,
    /// An EAGLE library (`.lbr`).
    Library,
    /// The JSON of a board or a schematic of the model.
    Data,
}

impl EntryKind {
    /// The kind of a file from its extension, in any case; `None` for a file the window does not
    /// open.
    pub fn of(path: &Path) -> Option<EntryKind> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "brd" | "eh3d" => Some(EntryKind::Board),
            "sch" => Some(EntryKind::Schematic),
            "lbr" => Some(EntryKind::Library),
            "json" => Some(EntryKind::Data),
            _ => None,
        }
    }
}

/// The settings folder of the user, from the environment: `APPDATA` on Windows, then
/// `XDG_CONFIG_HOME`, then `.config` in `HOME`. An empty value counts as missing. `get` reads one
/// variable, so the choice is tested without touching the real environment.
pub fn config_dir(get: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let value = |name: &str| get(name).filter(|v| !v.trim().is_empty());
    if let Some(base) = value("APPDATA") {
        return Some(PathBuf::from(base).join(FOLDER_WINDOWS));
    }
    if let Some(base) = value("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(base).join(FOLDER_OTHER));
    }
    value("HOME").map(|home| PathBuf::from(home).join(".config").join(FOLDER_OTHER))
}

/// The file of the list that earlier versions kept for the start screen, named by the setting
/// `recent_file`.
pub fn start_list_path(get: &dyn Fn(&str) -> Option<String>, file_name: &str) -> Option<PathBuf> {
    config_dir(get).map(|dir| dir.join(file_name))
}

/// The paths of the old list of the start screen, the newest first: blank lines and lines that
/// start with `#` are ignored and a path that repeats is kept once.
fn old_entries(text: &str) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    for line in text.lines().map(str::trim) {
        if !line.is_empty() && !line.starts_with('#') && !entries.iter().any(|e| e == line) {
            entries.push(line.to_string());
        }
    }
    entries
}

/// A name next to `path` that no file has, for the old list once it has been moved over.
fn set_aside_name(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    (1..1000)
        .map(|n| match n {
            1 => path.with_file_name(format!("{name}.moved")),
            _ => path.with_file_name(format!("{name}.moved-{n}")),
        })
        .find(|candidate| !candidate.exists())
}

/// Makes the list of the frame the one list of the program: it keeps at most `limit` entries, it
/// lives in `store` (none keeps it in memory only), and what the old list of the start screen
/// (`start_list`) holds and the list does not is added after it. The old file is then renamed, and
/// never deleted, so that an entry cleared later does not come back; it keeps its name, to be read
/// again at the next start, when the list could not be written. At most a megabyte of it is read.
pub fn keep_one_recent_list(
    core: &mut DockCore,
    store: Option<RecentStore>,
    start_list: Option<&Path>,
    limit: usize,
) {
    core.recent_mut().set_limit(limit);
    let Some(store) = store else {
        return;
    };
    core.keep_recent_in(store.clone());
    let Some(old) = start_list else {
        return;
    };
    let mut text = String::new();
    let read =
        std::fs::File::open(old).and_then(|file| file.take(1 << 20).read_to_string(&mut text));
    if read.is_err() {
        // not there, or not text: it is left as it is
        return;
    }
    let mut merged = core.recent().items().to_vec();
    for path in old_entries(&text) {
        if !merged.contains(&path) {
            merged.push(path);
        }
    }
    // an address is never written by the store, so it is not asked for either
    merged.retain(|path| !path.contains("://"));
    let list = core.recent_mut();
    list.clear();
    // pushed from the oldest, so that what does not fit is the oldest
    for path in merged.iter().rev() {
        list.push(path);
    }
    core.flush_recent();
    let written = core.recent().items().is_empty()
        || store
            .load(core.recent().limit())
            .is_some_and(|kept| kept.items() == core.recent().items());
    if written {
        if let Some(aside) = set_aside_name(old) {
            // a failure only means that it is read again, and gives the same list
            let _ = std::fs::rename(old, aside);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        }
    }

    #[test]
    fn the_kind_comes_from_the_extension_in_any_case() {
        assert_eq!(EntryKind::of(Path::new("a.brd")), Some(EntryKind::Board));
        assert_eq!(
            EntryKind::of(Path::new("A.SCH")),
            Some(EntryKind::Schematic)
        );
        assert_eq!(
            EntryKind::of(Path::new("x/y.Lbr")),
            Some(EntryKind::Library)
        );
        assert_eq!(
            EntryKind::of(Path::new("board.json")),
            Some(EntryKind::Data)
        );
        assert_eq!(EntryKind::of(Path::new("notes.txt")), None);
        assert_eq!(EntryKind::of(Path::new("noextension")), None);
    }

    #[test]
    fn the_settings_folder_follows_the_order_of_the_variables() {
        let all = env(&[("APPDATA", "/a"), ("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]);
        assert_eq!(
            config_dir(&all),
            Some(PathBuf::from("/a").join("EH3D-PHOENIX-PCB"))
        );
        let xdg = env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]);
        assert_eq!(
            config_dir(&xdg),
            Some(PathBuf::from("/x").join("eh3d-phoenix-pcb"))
        );
        let home = env(&[("HOME", "/h")]);
        assert_eq!(
            config_dir(&home),
            Some(PathBuf::from("/h").join(".config").join("eh3d-phoenix-pcb"))
        );
        let empty = env(&[("APPDATA", " "), ("HOME", "")]);
        assert_eq!(config_dir(&empty), None);
        assert_eq!(
            start_list_path(&home, "recent.cfg"),
            Some(
                PathBuf::from("/h")
                    .join(".config")
                    .join("eh3d-phoenix-pcb")
                    .join("recent.cfg")
            )
        );
    }

    fn core() -> DockCore {
        DockCore::new(crate::panels::defs(), crate::panels::workspaces())
    }

    /// The list of the menu in a folder of the test, never in the one of the person.
    fn store(folder: &Path) -> RecentStore {
        RecentStore::in_dir(&folder.join("EH3D"), eh3d_dock::about::Phoenix::Pcb)
    }

    fn start(folder: &Path, limit: usize) -> DockCore {
        let mut core = core();
        let old = folder.join("recent.cfg");
        keep_one_recent_list(&mut core, Some(store(folder)), Some(&old), limit);
        core
    }

    #[test]
    fn what_both_lists_held_is_in_the_one_list_and_the_old_file_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let mut menu = core();
        menu.keep_recent_in(store(dir.path()));
        menu.recent_mut().push("/work/older.brd");
        menu.recent_mut().push("/work/both.brd");
        menu.flush_recent();
        let old = dir.path().join("recent.cfg");
        let text =
            "# note\n\n  /work/start.sch  \n/work/both.brd\n/work/start.sch\n/work/a b.lbr\n";
        std::fs::write(&old, text).unwrap();
        let wanted = [
            "/work/both.brd",
            "/work/older.brd",
            "/work/start.sch",
            "/work/a b.lbr",
        ];
        let first = start(dir.path(), 20);
        assert_eq!(first.recent().items(), wanted);
        assert!(!old.exists());
        let aside = dir.path().join("recent.cfg.moved");
        assert_eq!(std::fs::read_to_string(&aside).unwrap(), text);
        // the next start reads the one list, and what is cleared stays cleared
        let mut second = start(dir.path(), 20);
        assert_eq!(second.recent().items(), wanted);
        second.recent_mut().clear();
        second.flush_recent();
        assert!(start(dir.path(), 20).recent().items().is_empty());
        // an earlier version that ran again leaves a new old list: no file is written over
        std::fs::write(&old, "/work/later.brd\n").unwrap();
        assert_eq!(start(dir.path(), 20).recent().items(), ["/work/later.brd"]);
        assert_eq!(std::fs::read_to_string(&aside).unwrap(), text);
        assert!(dir.path().join("recent.cfg.moved-2").exists());
    }

    #[test]
    fn the_start_screen_list_alone_is_moved_and_the_limit_drops_the_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("recent.cfg");
        std::fs::write(&old, "a.brd\nb.brd\nc.brd\n").unwrap();
        assert_eq!(start(dir.path(), 2).recent().items(), ["a.brd", "b.brd"]);
        assert_eq!(start(dir.path(), 2).recent().items(), ["a.brd", "b.brd"]);
        // what did not fit is still in the file that was set aside
        let aside = std::fs::read_to_string(dir.path().join("recent.cfg.moved")).unwrap();
        assert!(aside.contains("c.brd"));
    }

    #[test]
    fn only_the_first_megabyte_of_the_list_of_recent_files_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = String::from("first.brd\n");
        text.push_str(&"#".repeat(2 << 20));
        text.push_str("\nlate.brd\n");
        std::fs::write(dir.path().join("recent.cfg"), text).unwrap();
        assert_eq!(start(dir.path(), 5).recent().items(), ["first.brd"]);
    }

    #[test]
    fn the_old_file_stays_when_the_list_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        // the "folder" of the list is a file, so the list can be neither made nor written
        std::fs::write(dir.path().join("EH3D"), b"x").unwrap();
        let old = dir.path().join("recent.cfg");
        std::fs::write(&old, "a.brd\n").unwrap();
        assert_eq!(start(dir.path(), 5).recent().items(), ["a.brd"]);
        assert!(old.exists());
    }

    #[test]
    fn without_a_store_nothing_is_read_or_renamed() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("recent.cfg");
        std::fs::write(&old, "a.brd\n").unwrap();
        let mut core = core();
        keep_one_recent_list(&mut core, None, Some(&old), 7);
        assert!(core.recent().items().is_empty());
        assert_eq!(core.recent().limit(), 7);
        assert!(old.exists());
    }
}
