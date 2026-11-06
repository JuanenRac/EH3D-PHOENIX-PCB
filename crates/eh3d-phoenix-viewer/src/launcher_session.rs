// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of the start screen of the window, without a screen: what a typed path, a dropped
//! file or a click on a recent file asks for, the list of recent files and the file where it is
//! kept. The window (`launcher_gui`, behind the `gui` feature) only draws this and passes on what
//! the person does.

use std::path::{Path, PathBuf};

use eh3d_phoenix_rules::Lang;

use crate::launcher_recent::{recent_path, EntryKind, RecentList};
use crate::launcher_text::LauncherText;
use crate::settings::ViewerSettings;

/// What the start screen asks the window to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// Open a file; the kind of the file decides the mode (board, schematic or library).
    Open(PathBuf),
    /// Show the Gerber and Excellon files of a folder, read-only.
    OpenGerber(PathBuf),
    /// Lay the typed board files (separated by commas) out as a panel of boards.
    OpenPanel(String),
    /// Start an empty library.
    NewLibrary,
}

/// One line of the list of recent files, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRow {
    /// The position in the list (for forgetting it).
    pub index: usize,
    /// The path.
    pub path: PathBuf,
    /// The name of the file.
    pub name: String,
    /// The folder that holds it.
    pub folder: String,
    /// What it is, when the window knows.
    pub kind: Option<EntryKind>,
    /// False when the file is no longer there.
    pub exists: bool,
}

/// The text of a kind, in a language.
pub fn kind_text(kind: EntryKind) -> LauncherText {
    match kind {
        EntryKind::Board => LauncherText::KindBoard,
        EntryKind::Schematic => LauncherText::KindSchematic,
        EntryKind::Library => LauncherText::KindLibrary,
        EntryKind::Data => LauncherText::KindData,
    }
}

/// The language named by the environment (`LC_ALL`, `LC_MESSAGES` or `LANG`, such as
/// `pt_PT.UTF-8`), or `None` when none of them names one of the eight.
pub fn lang_from_env(get: &dyn Fn(&str) -> Option<String>) -> Option<Lang> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| get(name))
        .find_map(|value| Lang::from_tag(value.split(['.', '@']).next().unwrap_or("")))
}

/// Takes the quotes and the blanks off a typed path (the Windows menu "Copy as path" adds
/// quotes).
pub fn clean_path(text: &str) -> String {
    let trimmed = text.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(trimmed);
    unquoted.trim().to_string()
}

/// The start screen: the typed path and the recent files with the file that keeps them.
#[derive(Debug, Clone)]
pub struct Launcher {
    /// The path being typed.
    pub typed: String,
    recent: RecentList,
    store: Option<PathBuf>,
    /// True when the last attempt to save the list failed.
    pub save_failed: bool,
}

impl Launcher {
    /// A start screen that keeps its list in `store` (none keeps it in memory only) and reads
    /// what was kept there before; a file that is missing or unreadable gives an empty list.
    pub fn new(settings: &ViewerSettings, store: Option<PathBuf>) -> Launcher {
        let text = store
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        Launcher {
            typed: String::new(),
            recent: RecentList::parse(&text, settings.recent_max),
            store,
            save_failed: false,
        }
    }

    /// A start screen whose list is in the settings folder of the user, found in the
    /// environment (`get` reads one variable).
    pub fn from_env(settings: &ViewerSettings, get: &dyn Fn(&str) -> Option<String>) -> Launcher {
        Launcher::new(settings, recent_path(get, &settings.recent_file))
    }

    /// The recent files.
    pub fn recent(&self) -> &RecentList {
        &self.recent
    }

    /// The file where the list is kept.
    pub fn store(&self) -> Option<&Path> {
        self.store.as_deref()
    }

    /// Writes the list; the folder is made when it does not exist. The answer is false when it
    /// could not be written, and `save_failed` says so for the window.
    pub fn save(&mut self) -> bool {
        let Some(path) = self.store.as_deref() else {
            self.save_failed = false;
            return true;
        };
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(path, self.recent.to_text()))
            .is_ok();
        self.save_failed = !written;
        written
    }

    /// Notes a file that was opened: it goes first in the list, which is saved. The path is made
    /// absolute so that the list does not depend on the folder the program started in.
    pub fn opened(&mut self, path: &Path) {
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        if self.recent.add(&absolute) {
            self.save();
        }
    }

    /// Takes one file out of the list.
    pub fn forget(&mut self, index: usize) {
        if self.recent.remove(index) {
            self.save();
        }
    }

    /// Empties the list.
    pub fn clear(&mut self) {
        self.recent.clear();
        self.save();
    }

    /// What the typed path asks for, or the note to show when it asks for nothing. A folder is
    /// shown as a set of Gerber and Excellon files.
    pub fn typed_launch(&self) -> Result<Launch, LauncherText> {
        let path = clean_path(&self.typed);
        if path.is_empty() {
            Err(LauncherText::EnterPath)
        } else if Path::new(&path).is_dir() {
            Ok(Launch::OpenGerber(PathBuf::from(path)))
        } else {
            Ok(Launch::Open(PathBuf::from(path)))
        }
    }

    /// What the button of the Gerber folder asks for: the typed path as a folder of Gerber and
    /// Excellon files, or the note to show when nothing is typed.
    pub fn gerber_launch(&self) -> Result<Launch, LauncherText> {
        let path = clean_path(&self.typed);
        if path.is_empty() {
            Err(LauncherText::EnterPath)
        } else {
            Ok(Launch::OpenGerber(PathBuf::from(path)))
        }
    }

    /// What the button of the panel asks for: the typed text as the board files of a panel, or the
    /// note to show when nothing is typed.
    pub fn panel_launch(&self) -> Result<Launch, LauncherText> {
        let text = self.typed.trim();
        if text.is_empty() {
            Err(LauncherText::EnterPath)
        } else {
            Ok(Launch::OpenPanel(text.to_owned()))
        }
    }

    /// What the files dropped on the window ask for: the first one that is a folder (Gerber and
    /// Excellon files), a board, a schematic, a library or a JSON file, or the note to show when
    /// there is none.
    pub fn dropped_launch(files: &[PathBuf]) -> Result<Launch, LauncherText> {
        files
            .iter()
            .find_map(|path| {
                if path.is_dir() {
                    Some(Launch::OpenGerber(path.clone()))
                } else {
                    EntryKind::of(path).map(|_| Launch::Open(path.clone()))
                }
            })
            .ok_or(LauncherText::NothingToOpen)
    }

    /// The lines of the list of recent files. `exists` says whether a file is still there.
    pub fn rows(&self, exists: &dyn Fn(&Path) -> bool) -> Vec<RecentRow> {
        self.recent
            .entries()
            .iter()
            .enumerate()
            .map(|(index, path)| RecentRow {
                index,
                path: path.clone(),
                name: path
                    .file_name()
                    .map_or_else(|| path.to_string_lossy(), |n| n.to_string_lossy())
                    .into_owned(),
                folder: path
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                kind: EntryKind::of(path),
                exists: exists(path),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(max: usize) -> ViewerSettings {
        ViewerSettings::parse(&format!("recent_max = {max}\n")).unwrap()
    }

    #[test]
    fn a_typed_path_is_cleaned_and_an_empty_one_asks_for_a_path() {
        let mut launcher = Launcher::new(&settings(5), None);
        assert_eq!(launcher.typed_launch(), Err(LauncherText::EnterPath));
        launcher.typed = "   ".to_string();
        assert_eq!(launcher.typed_launch(), Err(LauncherText::EnterPath));
        launcher.typed = "  \"C:\\boards\\a b.brd\"  ".to_string();
        assert_eq!(
            launcher.typed_launch(),
            Ok(Launch::Open(PathBuf::from("C:\\boards\\a b.brd")))
        );
        assert_eq!(clean_path("\"only the start"), "\"only the start");
    }

    #[test]
    fn the_first_dropped_file_that_can_be_opened_is_the_one() {
        let files = vec![
            PathBuf::from("readme.txt"),
            PathBuf::from("main.SCH"),
            PathBuf::from("other.brd"),
        ];
        assert_eq!(
            Launcher::dropped_launch(&files),
            Ok(Launch::Open(PathBuf::from("main.SCH")))
        );
        assert_eq!(
            Launcher::dropped_launch(&[PathBuf::from("a.zip")]),
            Err(LauncherText::NothingToOpen)
        );
        assert_eq!(
            Launcher::dropped_launch(&[]),
            Err(LauncherText::NothingToOpen)
        );
    }

    #[test]
    fn a_folder_is_shown_as_gerber_files_whether_typed_dropped_or_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let mut launcher = Launcher::new(&settings(5), None);
        launcher.typed = format!("\"{}\"", dir.path().display());
        assert_eq!(
            launcher.typed_launch(),
            Ok(Launch::OpenGerber(dir.path().to_path_buf()))
        );
        assert_eq!(
            Launcher::dropped_launch(&[PathBuf::from("a.zip"), dir.path().to_path_buf()]),
            Ok(Launch::OpenGerber(dir.path().to_path_buf()))
        );
        // the button takes the typed path as a folder even when it is not there yet
        launcher.typed = "/nowhere/gerbers".to_string();
        assert_eq!(
            launcher.gerber_launch(),
            Ok(Launch::OpenGerber(PathBuf::from("/nowhere/gerbers")))
        );
        assert_eq!(
            launcher.typed_launch(),
            Ok(Launch::Open(PathBuf::from("/nowhere/gerbers")))
        );
        launcher.typed.clear();
        assert_eq!(launcher.gerber_launch(), Err(LauncherText::EnterPath));
    }

    #[test]
    fn the_panel_button_takes_the_typed_text_as_the_board_files() {
        let mut launcher = Launcher::new(&settings(5), None);
        assert_eq!(launcher.panel_launch(), Err(LauncherText::EnterPath));
        launcher.typed = "  a.brd, b.json  ".to_string();
        assert_eq!(
            launcher.panel_launch(),
            Ok(Launch::OpenPanel("a.brd, b.json".to_string()))
        );
    }

    #[test]
    fn the_language_comes_from_the_first_variable_that_names_one() {
        let get = |name: &str| match name {
            "LC_ALL" => Some("C".to_string()),
            "LANG" => Some("pt_PT.UTF-8".to_string()),
            _ => None,
        };
        assert_eq!(lang_from_env(&get), Some(Lang::Pt));
        let german = |name: &str| (name == "LC_MESSAGES").then(|| "de_DE@euro".to_string());
        assert_eq!(lang_from_env(&german), Some(Lang::De));
        let none = |_: &str| -> Option<String> { None };
        assert_eq!(lang_from_env(&none), None);
    }

    #[test]
    fn an_opened_file_is_kept_in_the_file_and_read_back_by_the_next_start() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("deeper").join("recent.cfg");
        let board = dir.path().join("a.brd");
        let schematic = dir.path().join("b.sch");
        let mut launcher = Launcher::new(&settings(5), Some(store.clone()));
        assert!(launcher.recent().entries().is_empty());
        launcher.opened(&board);
        launcher.opened(&schematic);
        launcher.opened(&board);
        assert!(!launcher.save_failed);
        let next = Launcher::new(&settings(5), Some(store));
        assert_eq!(next.recent().entries(), [board, schematic]);
    }

    #[test]
    fn forgetting_and_clearing_are_saved() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("recent.cfg");
        let mut launcher = Launcher::new(&settings(5), Some(store.clone()));
        launcher.opened(&dir.path().join("a.brd"));
        launcher.opened(&dir.path().join("b.brd"));
        launcher.forget(0);
        launcher.forget(9);
        let after_forget = Launcher::new(&settings(5), Some(store.clone()));
        assert_eq!(after_forget.recent().entries(), [dir.path().join("a.brd")]);
        launcher.clear();
        let after_clear = Launcher::new(&settings(5), Some(store));
        assert!(after_clear.recent().entries().is_empty());
    }

    #[test]
    fn the_limit_comes_from_the_settings() {
        let dir = tempfile::tempdir().unwrap();
        let mut launcher = Launcher::new(&settings(2), None);
        for name in ["a.brd", "b.brd", "c.brd"] {
            launcher.opened(&dir.path().join(name));
        }
        assert_eq!(launcher.recent().entries().len(), 2);
        assert_eq!(launcher.recent().entries()[0], dir.path().join("c.brd"));
        assert!(launcher.store().is_none());
        assert!(launcher.save());
    }

    #[test]
    fn a_list_that_cannot_be_written_is_reported_and_never_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        // the "folder" of the list is a file, so the list can be neither made nor written
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let mut launcher = Launcher::new(&settings(5), Some(blocker.join("recent.cfg")));
        launcher.opened(&dir.path().join("a.brd"));
        assert!(launcher.save_failed);
    }

    #[test]
    fn the_rows_say_what_each_file_is_and_whether_it_is_still_there() {
        let mut launcher = Launcher::new(&settings(5), None);
        launcher.opened(Path::new("/work/boards/main.brd"));
        launcher.opened(Path::new("/work/old/gone.lbr"));
        let exists = |path: &Path| path.ends_with("main.brd");
        let rows = launcher.rows(&exists);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].index, 0);
        assert_eq!(rows[0].name, "gone.lbr");
        assert_eq!(rows[0].kind, Some(EntryKind::Library));
        assert!(!rows[0].exists);
        assert_eq!(rows[1].name, "main.brd");
        assert_eq!(rows[1].kind, Some(EntryKind::Board));
        assert!(rows[1].exists);
        assert!(rows[1].folder.ends_with("boards"));
    }

    #[test]
    fn every_kind_has_a_text() {
        for kind in [
            EntryKind::Board,
            EntryKind::Schematic,
            EntryKind::Library,
            EntryKind::Data,
        ] {
            assert!(!kind_text(kind).tr(Lang::En).is_empty());
        }
        assert_eq!(kind_text(EntryKind::Library), LauncherText::KindLibrary);
    }
}
