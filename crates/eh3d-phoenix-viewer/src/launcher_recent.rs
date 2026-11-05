// SPDX-License-Identifier: GPL-3.0-or-later
//! The list of recent files of the start screen, and where the window keeps its files. The list
//! is a plain text file, one path per line, the newest first; how many entries it keeps and how
//! the file is called are settings (`recent_max`, `recent_file`), not values of the code.

use std::path::{Path, PathBuf};

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
            "brd" => Some(EntryKind::Board),
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

/// The file that holds the recent files, named by the setting `recent_file`.
pub fn recent_path(get: &dyn Fn(&str) -> Option<String>, file_name: &str) -> Option<PathBuf> {
    config_dir(get).map(|dir| dir.join(file_name))
}

/// The recent files, the newest first, with a limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentList {
    max: usize,
    entries: Vec<PathBuf>,
}

impl RecentList {
    /// An empty list that keeps at most `max` entries (at least one).
    pub fn new(max: usize) -> RecentList {
        RecentList {
            max: max.max(1),
            entries: Vec::new(),
        }
    }

    /// Reads the text of the file: one path per line, the newest first; blank lines and lines
    /// that start with `#` are ignored, a path that repeats is kept once, and what goes beyond
    /// the limit is dropped.
    pub fn parse(text: &str, max: usize) -> RecentList {
        let mut list = RecentList::new(max);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let path = PathBuf::from(line);
            if list.entries.len() < list.max && !list.entries.contains(&path) {
                list.entries.push(path);
            }
        }
        list
    }

    /// The text to write to the file.
    pub fn to_text(&self) -> String {
        let mut text = String::from("# recent files, the newest first\n");
        for path in &self.entries {
            text.push_str(&path.to_string_lossy());
            text.push('\n');
        }
        text
    }

    /// The entries, the newest first.
    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    /// How many entries the list keeps at most.
    pub fn max(&self) -> usize {
        self.max
    }

    /// Puts a file first. A path that the file could not hold (empty, starting with `#`, or
    /// with a line break) is refused and the answer is false; a path that was in the list moves
    /// to the front; the oldest entries go when the limit is passed.
    pub fn add(&mut self, path: &Path) -> bool {
        let text = path.to_string_lossy();
        if text.trim().is_empty()
            || text.trim_start().starts_with('#')
            || text.contains(['\n', '\r'])
        {
            return false;
        }
        self.entries.retain(|entry| entry != path);
        self.entries.insert(0, path.to_path_buf());
        self.entries.truncate(self.max);
        true
    }

    /// Takes out the entry at a position; the answer is false when there is none.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.entries.len() {
            self.entries.remove(index);
            true
        } else {
            false
        }
    }

    /// Empties the list.
    pub fn clear(&mut self) {
        self.entries.clear();
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
            recent_path(&home, "recent.cfg"),
            Some(
                PathBuf::from("/h")
                    .join(".config")
                    .join("eh3d-phoenix-pcb")
                    .join("recent.cfg")
            )
        );
    }

    #[test]
    fn the_newest_file_goes_first_and_a_repeat_moves_to_the_front() {
        let mut list = RecentList::new(3);
        assert!(list.add(Path::new("a.brd")));
        assert!(list.add(Path::new("b.sch")));
        assert!(list.add(Path::new("a.brd")));
        assert_eq!(
            list.entries(),
            [PathBuf::from("a.brd"), PathBuf::from("b.sch")]
        );
    }

    #[test]
    fn the_oldest_entries_go_when_the_limit_is_passed() {
        let mut list = RecentList::new(2);
        for name in ["a.brd", "b.brd", "c.brd"] {
            assert!(list.add(Path::new(name)));
        }
        assert_eq!(
            list.entries(),
            [PathBuf::from("c.brd"), PathBuf::from("b.brd")]
        );
        assert_eq!(RecentList::new(0).max(), 1);
    }

    #[test]
    fn a_path_the_file_could_not_hold_is_refused() {
        let mut list = RecentList::new(5);
        assert!(!list.add(Path::new("")));
        assert!(!list.add(Path::new("#hidden.brd")));
        assert!(!list.add(Path::new("two\nlines.brd")));
        assert!(list.entries().is_empty());
    }

    #[test]
    fn the_text_round_trips_and_ignores_comments_blanks_and_repeats() {
        let mut list = RecentList::new(5);
        list.add(Path::new("one.brd"));
        list.add(Path::new("two.sch"));
        let text = list.to_text();
        assert!(text.starts_with("# "));
        assert_eq!(RecentList::parse(&text, 5), list);
        let messy = "# note\n\n  b.brd  \na.brd\nb.brd\n";
        let parsed = RecentList::parse(messy, 5);
        assert_eq!(
            parsed.entries(),
            [PathBuf::from("b.brd"), PathBuf::from("a.brd")]
        );
    }

    #[test]
    fn reading_keeps_the_newest_up_to_the_limit() {
        let parsed = RecentList::parse("a.brd\nb.brd\nc.brd\n", 2);
        assert_eq!(
            parsed.entries(),
            [PathBuf::from("a.brd"), PathBuf::from("b.brd")]
        );
    }

    #[test]
    fn removing_and_clearing() {
        let mut list = RecentList::parse("a.brd\nb.brd\n", 5);
        assert!(!list.remove(2));
        assert!(list.remove(0));
        assert_eq!(list.entries(), [PathBuf::from("b.brd")]);
        list.clear();
        assert!(list.entries().is_empty());
    }
}
