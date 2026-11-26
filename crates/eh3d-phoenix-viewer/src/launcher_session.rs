// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of the start screen of the window, without a screen: what a typed path, a dropped
//! file or a click on a recent file asks for, and the recent files it shows. Those are the list of
//! the menu *Open recent* (kept by the frame of the suite): the start screen holds a copy of it and
//! the changes the person asked for, which `sync` puts into the list. The window (`launcher_gui`,
//! behind the `gui` feature) only draws this and passes on what the person does.

use std::path::{Path, PathBuf};

use eh3d_dock::recent::RecentFiles;
use eh3d_phoenix_rules::Lang;

use crate::launcher_recent::EntryKind;
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

/// A change of the list of recent files that the window asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RecentChange {
    Opened(String),
    Forget(String),
    Clear,
}

/// The start screen: the typed path and the recent files.
#[derive(Debug, Clone)]
pub struct Launcher {
    /// The path being typed.
    pub typed: String,
    recent: RecentFiles,
    changes: Vec<RecentChange>,
}

impl Launcher {
    /// A start screen with no recent files yet, which keeps at most `recent_max` of them.
    pub fn new(settings: &ViewerSettings) -> Launcher {
        Launcher {
            typed: String::new(),
            recent: RecentFiles::new(settings.recent_max),
            changes: Vec::new(),
        }
    }

    /// The recent files, the newest first.
    pub fn recent(&self) -> &RecentFiles {
        &self.recent
    }

    /// Puts what the window asked for into the list of the menu, and takes a copy of that list.
    pub fn sync(&mut self, shared: &mut RecentFiles) {
        for change in std::mem::take(&mut self.changes) {
            match change {
                RecentChange::Opened(path) => {
                    shared.push(&path);
                }
                RecentChange::Forget(path) => shared.remove(&path),
                RecentChange::Clear => shared.clear(),
            }
        }
        self.recent = shared.clone();
    }

    /// Notes a file that was opened: it goes first in the list. The path is made absolute so that
    /// the list does not depend on the folder the program started in.
    pub fn opened(&mut self, path: &Path) {
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let text = absolute.to_string_lossy().into_owned();
        if self.recent.push(&text) {
            self.changes.push(RecentChange::Opened(text));
        }
    }

    /// Takes one file out of the list.
    pub fn forget(&mut self, index: usize) {
        if let Some(path) = self.recent.items().get(index).cloned() {
            self.recent.remove(&path);
            self.changes.push(RecentChange::Forget(path));
        }
    }

    /// Empties the list.
    pub fn clear(&mut self) {
        self.recent.clear();
        self.changes.push(RecentChange::Clear);
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
            .items()
            .iter()
            .map(PathBuf::from)
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
                kind: EntryKind::of(&path),
                exists: exists(&path),
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
        let mut launcher = Launcher::new(&settings(5));
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
        let mut launcher = Launcher::new(&settings(5));
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
        let mut launcher = Launcher::new(&settings(5));
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

    fn texts(paths: &[&Path]) -> Vec<String> {
        paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn what_the_window_asks_for_reaches_the_list_of_the_menu() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.brd"), dir.path().join("b.sch"));
        let mut shared = RecentFiles::new(5);
        shared.push("/from/the/menu.lbr");
        let mut launcher = Launcher::new(&settings(5));
        launcher.opened(&a);
        launcher.opened(&b);
        launcher.opened(&a);
        launcher.sync(&mut shared);
        let mut wanted = texts(&[&a, &b]);
        wanted.push("/from/the/menu.lbr".to_string());
        assert_eq!(shared.items(), wanted);
        assert_eq!(launcher.recent(), &shared);
        // forgetting takes the entry of that place out of the list of the menu too
        launcher.forget(1);
        launcher.forget(9);
        launcher.sync(&mut shared);
        wanted.remove(1);
        assert_eq!(shared.items(), wanted);
        // what the menu cleared is gone from the start screen, and the other way round
        shared.clear();
        launcher.sync(&mut shared);
        assert!(launcher.recent().items().is_empty());
        shared.push("/x.brd");
        launcher.sync(&mut shared);
        launcher.clear();
        launcher.sync(&mut shared);
        assert!(shared.items().is_empty());
    }

    #[test]
    fn the_limit_comes_from_the_settings() {
        let dir = tempfile::tempdir().unwrap();
        let mut launcher = Launcher::new(&settings(2));
        for name in ["a.brd", "b.brd", "c.brd"] {
            launcher.opened(&dir.path().join(name));
        }
        assert_eq!(launcher.recent().items().len(), 2);
        assert_eq!(
            launcher.recent().items()[0],
            texts(&[&dir.path().join("c.brd")])[0]
        );
    }

    #[test]
    fn the_rows_say_what_each_file_is_and_whether_it_is_still_there() {
        let mut launcher = Launcher::new(&settings(5));
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
