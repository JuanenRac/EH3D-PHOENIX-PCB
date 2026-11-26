// SPDX-License-Identifier: GPL-3.0-or-later
//! What the entries of the menu File for a project file of the suite (`.eh3d`) do in the window:
//! open it, save it and save it under another name without losing a branch, import a design of
//! another program keeping the original, take a file of another branch and extract a branch.
//!
//! Only the branch `pcb/` is written. A project that was not opened here, a branch that changed
//! since it was read, a signature that a write removes and a design that an import replaces are
//! asked about before anything is written, and a project that is written over is copied first. The
//! window (`gui`) only draws and asks, so all of this is tested without a screen.

use std::fs;
use std::path::{Path, PathBuf};

use eh3d_phoenix_assembly3d::outline2d::outline2d_with_folder;
use eh3d_phoenix_project::pcb_branch::{
    read_pcb, write_pcb, NewPcbProject, PcbError, PcbParts, PCB_BRANCH, PCB_FORMAT,
};
use eh3d_phoenix_rules::Lang;
use eh3d_project::appbranch::{self, BranchError, BranchKit};
use eh3d_project::suitetext::{Said, S};
use eh3d_project::suiteview::{self, name_of, ProjectView};

use crate::load::{self, Document, Loaded};
use crate::session::origin_of;
use crate::settings::ViewerSettings;
use crate::suite_text::SuiteMsg;
use crate::workspace::{Doc, Workspace};

/// The file of the branch `pcb/` that lists what was taken from other branches.
pub const MANIFEST: &str = "pcb.json";

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The design of the window as the branch keeps it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Design {
    /// What a save puts into the branch.
    pub parts: PcbParts,
    /// The file the document was read from.
    pub input: Option<PathBuf>,
    /// Whether the document has changes that are not saved.
    pub dirty: bool,
}

/// The design of the document of the window: a board with the outline that the programs of
/// enclosures read, or a schematic.
pub fn design_of(workspace: &Workspace) -> Result<Design, String> {
    let mut parts = PcbParts::default();
    match &workspace.doc {
        Doc::Board(session) => {
            parts.board = Some(session.board.to_bytes().map_err(|e| e.to_string())?);
            parts.outline2d =
                outline2d_with_folder(&session.board, Path::new("config")).map_err(|e| e.detail)?;
        }
        Doc::Schematic(session) => {
            parts.schematic = Some(session.schematic.to_bytes().map_err(|e| e.to_string())?);
        }
    }
    Ok(Design {
        parts,
        input: workspace.input.clone(),
        dirty: workspace.dirty(),
    })
}

/// Something that writes a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Save into this project.
    Save(PathBuf),
    /// Import this file of another program.
    Import(PathBuf),
    /// Take this file of that branch.
    Take(String, String),
}

/// One thing a person has to agree to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// One of the words every program of the suite says.
    Suite(Said),
    /// One of this program.
    Own(SuiteMsg),
}

impl Warning {
    /// The warning in a language.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Warning::Suite(said) => said.text(code(lang)),
            Warning::Own(msg) => msg.tr(lang).to_owned(),
        }
    }
}

/// A write that waits for the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// What was asked.
    pub action: Action,
    /// What has to be agreed to.
    pub warnings: Vec<Warning>,
}

/// How an entry ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// It was done.
    Done,
    /// The window has no project yet: a name has to be asked for.
    NeedsName,
    /// Nothing was written yet: the person has to agree ([`SuiteState::pending`]).
    Asked,
    /// It was refused, with why.
    Refused(String),
}

/// What the window keeps about its project file of the suite.
#[derive(Debug, Default)]
pub struct SuiteState {
    /// The project the window works on.
    pub file: Option<PathBuf>,
    /// What it held when it was last looked at.
    pub view: Option<ProjectView>,
    /// The last thing that was done.
    pub status: Option<Said>,
    /// What is to be told about it.
    pub notes: Vec<Said>,
    /// What was said in other words.
    pub extra: Vec<String>,
    /// The write that waits for the person.
    pub pending: Option<Pending>,
    /// The design that an import left for the window to show.
    pub show: Option<Loaded>,
    /// The content hash of the branch of the program when it was read or written.
    seen: Option<String>,
}

/// The code of a language for the words of the suite.
pub fn code(lang: Lang) -> &'static str {
    crate::panels::view_lang(lang).code()
}

fn kit() -> BranchKit<'static> {
    BranchKit::new(PCB_BRANCH, PCB_FORMAT, MANIFEST, VERSION)
}

/// What the branch holds; nothing when the project has no branch `pcb/`.
fn held(path: &Path) -> Result<PcbParts, String> {
    match read_pcb(path) {
        Ok(parts) => Ok(parts),
        Err(PcbError::NoBranch) => Ok(PcbParts::default()),
        Err(e) => Err(e.to_string()),
    }
}

fn nothing_to_write(parts: &PcbParts) -> bool {
    parts.board.is_none() && parts.schematic.is_none()
}

/// Copies a project that is going to be written, beside it with the suffix of the backups.
fn backup(path: &Path, suffix: &str) -> Result<(), String> {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    fs::copy(path, PathBuf::from(name))
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn new_project(target: &Path) -> NewPcbProject {
    NewPcbProject {
        name: target
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        ..NewPcbProject::default()
    }
}

fn stem_of(path: Option<&Path>) -> String {
    path.and_then(Path::file_stem)
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "board".to_owned())
}

impl SuiteState {
    /// The window works on this project from now on.
    pub fn adopt(&mut self, path: &Path) {
        let view = suiteview::view(path, PCB_BRANCH).ok();
        self.seen = view.as_ref().and_then(|v| v.own_hash.clone());
        self.view = view;
        self.file = Some(path.to_path_buf());
        self.status = None;
        self.extra = Vec::new();
        self.pending = None;
        self.notes = appbranch::open_project(path)
            .and_then(|mut project| kit().load(&mut project))
            .and_then(|copy| copy.report())
            .ok()
            .flatten()
            .map(|report| suiteview::report_lines(&report))
            .unwrap_or_default();
    }

    /// The window works on no project.
    pub fn close(&mut self) {
        *self = SuiteState::default();
    }

    /// Reads the project again to tell what is out of date now.
    pub fn refresh(&mut self) {
        if let Some(file) = &self.file {
            self.view = suiteview::view(file, PCB_BRANCH).ok();
        }
    }

    /// The lines that stay in sight while the window works on a project; the flag marks the ones
    /// that ask for attention.
    pub fn banner(&self, lang: Lang) -> Vec<(String, bool)> {
        let Some(file) = &self.file else {
            return Vec::new();
        };
        let code = code(lang);
        let mut lines = suiteview::banner(file, self.view.as_ref(), PCB_BRANCH, code);
        lines.extend(self.status.iter().map(|s| (s.text(code), false)));
        lines.extend(self.notes.iter().map(|n| (n.text(code), false)));
        lines.extend(self.extra.iter().map(|n| (n.clone(), true)));
        lines
    }

    /// Opens a project: the design of the branch `pcb/` for the window to show, or nothing when
    /// the branch holds none yet (the document of the window stays and saving puts it there).
    pub fn open(
        &mut self,
        path: &Path,
        settings: &ViewerSettings,
    ) -> Result<Option<Loaded>, String> {
        if !suiteview::is_project(path) {
            return Err(Said::plain(S::NeedExtension).text("en"));
        }
        // a project that cannot be read leaves everything as it was
        drop(appbranch::open_project(path).map_err(|e| e.to_string())?);
        let parts = held(path)?;
        let loaded = if nothing_to_write(&parts) {
            None
        } else {
            Some(load::read_document(path, settings).map_err(|e| e.message(Lang::En))?)
        };
        self.adopt(path);
        let name = name_of(path);
        self.status = Some(if loaded.is_some() {
            Said(S::Opened, vec![name])
        } else {
            Said(S::NoWork, vec![name, PCB_BRANCH.to_owned()])
        });
        Ok(loaded)
    }

    /// The name offered for a project that is saved for the first time or under another name.
    pub fn suggested_name(&self, input: Option<&Path>) -> String {
        format!("{}.eh3d", stem_of(self.file.as_deref().or(input)))
    }

    /// Saves into the project of the window, or says that it has none.
    pub fn save(&mut self, design: Option<&Design>, settings: &ViewerSettings) -> Outcome {
        match self.file.clone() {
            Some(file) => self.write(&file, design, false, settings),
            None => Outcome::NeedsName,
        }
    }

    /// Saves under another name. A new file gets every branch of the project that is open; a file
    /// that is there keeps its own branches and only `pcb/` is written, once the person agreed.
    pub fn save_as(
        &mut self,
        target: &Path,
        design: Option<&Design>,
        settings: &ViewerSettings,
    ) -> Outcome {
        self.write(&suiteview::with_extension(target), design, false, settings)
    }

    /// Keeps the original of a design of another program (`.brd`, `.sch`) in the project and
    /// makes it the design of the branch.
    pub fn import(
        &mut self,
        source: &Path,
        design: Option<&Design>,
        settings: &ViewerSettings,
    ) -> Outcome {
        self.bring_in(source, design, false, settings)
    }

    /// Takes a file of another branch into the project, with the reference that tells later when
    /// it is out of date.
    pub fn take(&mut self, branch: &str, rel: &str, settings: &ViewerSettings) -> Outcome {
        self.take_file(branch, rel, false, settings)
    }

    /// The person agreed to the write that was waiting.
    pub fn confirm(&mut self, design: Option<&Design>, settings: &ViewerSettings) -> Outcome {
        match self.pending.take().map(|p| p.action) {
            Some(Action::Save(target)) => self.write(&target, design, true, settings),
            Some(Action::Import(source)) => self.bring_in(&source, design, true, settings),
            Some(Action::Take(branch, rel)) => self.take_file(&branch, &rel, true, settings),
            None => Outcome::Done,
        }
    }

    /// The person did not agree: nothing was written and nothing is.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// Writes the files of a branch into a folder; a file that is there is never written over.
    pub fn extract(&mut self, branch: &str, dir: &Path) -> Outcome {
        let Some(file) = self.file.clone() else {
            return need_project();
        };
        match appbranch::extract(&file, branch, dir) {
            Ok(count) => {
                self.status = Some(Said(
                    S::Extracted,
                    vec![count.to_string(), dir.display().to_string()],
                ));
                Outcome::Done
            }
            Err(e) => Outcome::Refused(e.to_string()),
        }
    }

    /// What every write has to be agreed to for this target.
    fn warnings(&self, target: &Path) -> Result<Vec<Warning>, String> {
        let found = suiteview::warnings(
            target,
            self.file.as_deref(),
            self.seen.as_deref(),
            PCB_BRANCH,
        )?;
        Ok(found.into_iter().map(Warning::Suite).collect())
    }

    fn ask(&mut self, action: Action, warnings: Vec<Warning>) -> Outcome {
        self.pending = Some(Pending { action, warnings });
        Outcome::Asked
    }

    fn write(
        &mut self,
        target: &Path,
        design: Option<&Design>,
        agreed: bool,
        settings: &ViewerSettings,
    ) -> Outcome {
        self.pending = None;
        if !suiteview::is_project(target) {
            return Outcome::Refused(S::NeedExtension.tr("en").to_owned());
        }
        let current = self.file.clone();
        let existed = target.exists();
        let same = current
            .as_deref()
            .is_some_and(|c| suiteview::same_file(c, target));
        // a new file is first a copy of the project that is open, with every branch
        let copy_from = current.filter(|c| !existed && c.is_file());
        let mut parts = design.map(|d| d.parts.clone()).unwrap_or_default();
        let base = if existed {
            Some(target.to_path_buf())
        } else {
            copy_from.clone()
        };
        let before = match base.as_deref().map(held).transpose() {
            Ok(found) => found.unwrap_or_default(),
            Err(e) => return Outcome::Refused(e),
        };
        // what the branch already holds byte for byte is not written again
        if parts.board.is_some() && parts.board == before.board {
            parts.board = None;
            parts.outline2d = None;
        }
        if parts.schematic.is_some() && parts.schematic == before.schematic {
            parts.schematic = None;
        }
        let writes = !nothing_to_write(&parts);
        if !writes && copy_from.is_none() {
            if design.is_none() || !existed {
                return Outcome::Refused(S::NothingToSave.tr("en").to_owned());
            }
            if !same {
                self.adopt(target);
            }
            self.status = Some(Said::plain(S::NothingNew));
            return Outcome::Done;
        }
        if writes && !agreed {
            match self.warnings(target) {
                Ok(warnings) if warnings.is_empty() => {}
                Ok(warnings) => return self.ask(Action::Save(target.to_path_buf()), warnings),
                Err(e) => return Outcome::Refused(e),
            }
        }
        if let Some(from) = &copy_from {
            if let Err(e) = suiteview::copy_project(from, target) {
                return Outcome::Refused(e);
            }
        }
        if writes {
            if existed {
                if let Err(e) = backup(target, &settings.backup_suffix) {
                    return Outcome::Refused(e);
                }
            }
            // a design that came from another program keeps its original in a branch that is new
            if nothing_to_write(&before) {
                parts.origin = design.and_then(|d| d.input.as_deref()).and_then(origin_of);
            }
            if let Err(e) = write_pcb(target, None, &parts, &new_project(target), VERSION) {
                // a copy that could not be written to is not left behind
                if copy_from.is_some() {
                    let _ = fs::remove_file(target);
                }
                return Outcome::Refused(e.to_string());
            }
        }
        self.adopt(target);
        self.status = Some(Said(S::Saved, vec![PCB_BRANCH.to_owned(), name_of(target)]));
        Outcome::Done
    }

    fn bring_in(
        &mut self,
        source: &Path,
        design: Option<&Design>,
        agreed: bool,
        settings: &ViewerSettings,
    ) -> Outcome {
        self.pending = None;
        let Some(file) = self.file.clone() else {
            return need_project();
        };
        let Some(mut origin) = origin_of(source) else {
            return Outcome::Refused(SuiteMsg::ImportKinds.tr(Lang::En).to_owned());
        };
        let loaded = match load::read_document(source, settings) {
            Ok(loaded) => loaded,
            Err(e) => return Outcome::Refused(e.message(Lang::En)),
        };
        origin.not_read = loaded.unread as u64;
        let before = match held(&file) {
            Ok(found) => found,
            Err(e) => return Outcome::Refused(e),
        };
        let (parts, replaced) = match &loaded.document {
            Document::Board(board) => {
                let bytes = match board.to_bytes() {
                    Ok(bytes) => bytes,
                    Err(e) => return Outcome::Refused(e.to_string()),
                };
                let outline = match outline2d_with_folder(board, Path::new("config")) {
                    Ok(outline) => outline,
                    Err(e) => return Outcome::Refused(e.detail),
                };
                let parts = PcbParts {
                    board: Some(bytes),
                    outline2d: outline,
                    origin: Some(origin),
                    ..PcbParts::default()
                };
                (parts, before.board.map(|_| SuiteMsg::ReplaceBoard))
            }
            Document::Schematic(schematic) => {
                let bytes = match schematic.to_bytes() {
                    Ok(bytes) => bytes,
                    Err(e) => return Outcome::Refused(e.to_string()),
                };
                let parts = PcbParts {
                    schematic: Some(bytes),
                    origin: Some(origin),
                    ..PcbParts::default()
                };
                (parts, before.schematic.map(|_| SuiteMsg::ReplaceSchematic))
            }
        };
        if !agreed {
            let mut warnings = match self.warnings(&file) {
                Ok(warnings) => warnings,
                Err(e) => return Outcome::Refused(e),
            };
            warnings.extend(replaced.map(Warning::Own));
            if design.is_some_and(|d| d.dirty) {
                warnings.push(Warning::Own(SuiteMsg::UnsavedLost));
            }
            if !warnings.is_empty() {
                return self.ask(Action::Import(source.to_path_buf()), warnings);
            }
        }
        if let Err(e) = backup(&file, &settings.backup_suffix) {
            return Outcome::Refused(e);
        }
        if let Err(e) = write_pcb(&file, None, &parts, &new_project(&file), VERSION) {
            return Outcome::Refused(e.to_string());
        }
        self.adopt(&file);
        self.status = Some(Said(S::Imported, vec![name_of(source)]));
        self.show = Some(loaded);
        Outcome::Done
    }

    fn take_file(
        &mut self,
        branch: &str,
        rel: &str,
        agreed: bool,
        settings: &ViewerSettings,
    ) -> Outcome {
        self.pending = None;
        let Some(file) = self.file.clone() else {
            return need_project();
        };
        if !agreed {
            match self.warnings(&file) {
                Ok(warnings) if warnings.is_empty() => {}
                Ok(warnings) => {
                    return self.ask(Action::Take(branch.to_owned(), rel.to_owned()), warnings)
                }
                Err(e) => return Outcome::Refused(e),
            }
        }
        let limit = settings.max_file_bytes;
        let artifact = Path::new(rel)
            .file_stem()
            .map_or_else(|| "file".to_owned(), |s| s.to_string_lossy().into_owned());
        let taken = kit().take(
            &file,
            branch,
            rel,
            &artifact,
            |bytes| {
                if bytes.len() as u64 > limit {
                    return Err(BranchError::Refused(format!(
                        "{branch}/{rel}: the file is larger than {limit} bytes"
                    )));
                }
                Ok(())
            },
            |copy, target| copy.add_to_list("taken", target),
        );
        match taken {
            Ok((_, notices)) => {
                let said: Vec<String> = notices.iter().map(ToString::to_string).collect();
                let (words, other) = suiteview::notices(&said);
                self.adopt(&file);
                self.notes.extend(words);
                self.extra = other;
                self.status = Some(Said(S::Taken, vec![branch.to_owned(), rel.to_owned()]));
                Outcome::Done
            }
            Err(e) => Outcome::Refused(e.to_string()),
        }
    }
}

fn need_project() -> Outcome {
    Outcome::Refused(S::NeedProject.tr("en").to_owned())
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_eagle_board::write_board;
    use eh3d_project::{BranchUpdate, OpenOptions, Project, SaveOptions, Writer};

    use super::*;
    use crate::sample;

    fn settings() -> ViewerSettings {
        ViewerSettings::default()
    }

    fn board_design() -> Design {
        Design {
            parts: PcbParts {
                board: Some(sample::board().to_bytes().unwrap()),
                ..PcbParts::default()
            },
            input: None,
            dirty: false,
        }
    }

    fn put_branch(path: &Path, id: &str, name: &str, bytes: &[u8]) {
        let project = Project::open(path, &OpenOptions::default()).unwrap();
        let mut update = BranchUpdate::new(id, "1.0", "0.0.0");
        update.entries.insert(name.to_owned(), bytes.to_vec());
        project
            .save_branch(&Writer::for_app(id), update, &SaveOptions::default())
            .unwrap();
    }

    fn branch_bytes(path: &Path, id: &str, name: &str) -> Vec<u8> {
        Project::open(path, &OpenOptions::default())
            .unwrap()
            .read_entry(id, name)
            .unwrap()
    }

    #[test]
    fn a_design_is_saved_opened_and_saved_under_another_name_with_every_branch() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.eh3d");
        let mut suite = SuiteState::default();
        let design = board_design();
        assert_eq!(suite.save(Some(&design), &settings()), Outcome::NeedsName);
        assert_eq!(
            suite.save_as(&dir.path().join("first"), Some(&design), &settings()),
            Outcome::Done
        );
        assert_eq!(suite.file.as_deref(), Some(first.as_path()));
        assert!(read_pcb(&first).unwrap().board.is_some());
        // nothing new: the project is not written again
        assert_eq!(suite.save(Some(&design), &settings()), Outcome::Done);
        assert_eq!(suite.status, Some(Said::plain(S::NothingNew)));

        put_branch(&first, "cad", "box.txt", b"a box");
        suite.refresh();
        let second = dir.path().join("second.eh3d");
        assert_eq!(
            suite.save_as(&second, Some(&design), &settings()),
            Outcome::Done
        );
        assert_eq!(branch_bytes(&second, "cad", "box.txt"), b"a box");
        assert_eq!(suite.file.as_deref(), Some(second.as_path()));
        assert!(suite.banner(Lang::En)[0].0.contains("second.eh3d"));

        let mut other = SuiteState::default();
        let loaded = other.open(&second, &settings()).unwrap().unwrap();
        assert_eq!(loaded.document, Document::Board(Box::new(sample::board())));
        assert!(other
            .view
            .as_ref()
            .unwrap()
            .takeable
            .contains(&("cad".to_owned(), "box.txt".to_owned())));
    }

    #[test]
    fn a_project_that_was_not_opened_here_is_written_only_after_a_yes_and_keeps_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let foreign = dir.path().join("foreign.eh3d");
        SuiteState::default().save_as(&foreign, Some(&board_design()), &settings());
        let schematic = Design {
            parts: PcbParts {
                schematic: Some(sample::schematic().to_bytes().unwrap()),
                ..PcbParts::default()
            },
            ..Design::default()
        };
        let mut suite = SuiteState::default();
        assert_eq!(
            suite.save_as(&foreign, Some(&schematic), &settings()),
            Outcome::Asked
        );
        let pending = suite.pending.clone().unwrap();
        assert!(pending.warnings[0].text(Lang::En).contains("foreign.eh3d"));
        suite.cancel();
        assert!(read_pcb(&foreign).unwrap().schematic.is_none());
        suite.save_as(&foreign, Some(&schematic), &settings());
        assert_eq!(suite.confirm(Some(&schematic), &settings()), Outcome::Done);
        let parts = read_pcb(&foreign).unwrap();
        assert!(parts.board.is_some() && parts.schematic.is_some());
        assert!(dir.path().join("foreign.eh3d.bak").is_file());
    }

    #[test]
    fn an_import_keeps_the_original_and_asks_before_it_replaces_a_design() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("in.brd");
        let text = write_board(&sample::board()).text;
        fs::write(&source, &text).unwrap();
        let mut suite = SuiteState::default();
        assert!(matches!(
            suite.import(&source, None, &settings()),
            Outcome::Refused(_)
        ));
        let project = dir.path().join("demo.eh3d");
        write_pcb(
            &project,
            None,
            &PcbParts::default(),
            &new_project(&project),
            VERSION,
        )
        .unwrap();
        suite.adopt(&project);
        assert_eq!(suite.import(&source, None, &settings()), Outcome::Done);
        assert!(suite.show.take().is_some());
        let origin = read_pcb(&project).unwrap().origin.unwrap();
        assert_eq!(origin.bytes, text.as_bytes());
        assert!(suite.notes.iter().any(|n| n.text("en").contains("in.brd")));

        let unsaved = Design {
            dirty: true,
            ..Design::default()
        };
        assert_eq!(
            suite.import(&source, Some(&unsaved), &settings()),
            Outcome::Asked
        );
        let warnings = &suite.pending.as_ref().unwrap().warnings;
        assert!(warnings.contains(&Warning::Own(SuiteMsg::ReplaceBoard)));
        assert!(warnings.contains(&Warning::Own(SuiteMsg::UnsavedLost)));
        assert_eq!(suite.confirm(Some(&unsaved), &settings()), Outcome::Done);

        let wrong = dir.path().join("notes.txt");
        fs::write(&wrong, b"text").unwrap();
        assert!(matches!(
            suite.import(&wrong, None, &settings()),
            Outcome::Refused(_)
        ));
    }

    #[test]
    fn a_file_taken_from_another_branch_is_out_of_date_when_its_source_changes() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("demo.eh3d");
        let mut suite = SuiteState::default();
        suite.save_as(&project, Some(&board_design()), &settings());
        put_branch(&project, "cad", "outline.dxf", b"one");
        suite.adopt(&project);
        assert_eq!(suite.take("cad", "outline.dxf", &settings()), Outcome::Done);
        assert_eq!(
            branch_bytes(&project, "pcb", "taken/cad/outline.dxf"),
            b"one"
        );
        // the design of the branch stays, and a later save keeps what was taken
        assert!(read_pcb(&project).unwrap().board.is_some());
        assert!(suite.view.as_ref().unwrap().stale.is_empty());
        put_branch(&project, "cad", "outline.dxf", b"two");
        suite.refresh();
        assert!(suite
            .banner(Lang::En)
            .iter()
            .any(|(_, attention)| *attention));

        let out = dir.path().join("out");
        assert_eq!(suite.extract("cad", &out), Outcome::Done);
        assert_eq!(
            fs::read(out.join("outline.dxf")).ok().as_deref(),
            Some(&b"two"[..])
        );
        assert!(matches!(
            suite.extract("nothing", &out),
            Outcome::Refused(_)
        ));
    }

    #[test]
    fn a_project_without_a_design_of_boards_is_opened_with_the_document_left_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("empty.eh3d");
        write_pcb(
            &project,
            None,
            &PcbParts::default(),
            &new_project(&project),
            VERSION,
        )
        .unwrap();
        let mut suite = SuiteState::default();
        assert_eq!(suite.open(&project, &settings()), Ok(None));
        assert_eq!(suite.file.as_deref(), Some(project.as_path()));
        assert!(suite
            .open(&dir.path().join("none.eh3d"), &settings())
            .is_err());
        assert_eq!(suite.file.as_deref(), Some(project.as_path()));
        suite.close();
        assert!(suite.banner(Lang::En).is_empty());
    }
}
