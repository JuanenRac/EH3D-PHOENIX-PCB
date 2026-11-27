// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing of the project file of the suite in the window: the panel «Attachments» with the
//! data sheets of the selected part, the bar that tells what is out of date or signed, the lists of
//! branches and of files to take, the question before a write and the window that chooses a file or
//! a folder. Everything that decides is in `suite_menu` and `part_datasheet`, in code that is tested.

use std::path::{Path, PathBuf};

use eframe::egui::{self, ScrollArea};
use eh3d_attachpanel::texts::{lang_of, tr, Text};
use eh3d_filedialog::{size_text, Action as Picked, Browser, Chosen, Filter, Mode, PlaceKind};
use eh3d_project::suitetext::{fill, Said, S};
use eh3d_project::suiteview;

use super::{dim, heading, ui_danger, App};
use crate::codes::{ViewError, FILE_UNUSABLE, SAVE_FAILED};
use crate::msg::Msg;
use crate::panels;
use crate::part_datasheet::{self, PartRef};
use crate::suite_menu::{code, design_of, Design, Outcome};
use crate::suite_text::SuiteMsg;
use crate::workspace::{suggested_save_path, Workspace};

/// The width the notice of a project that changed on disk wraps its lines to.
const CHANGED_WIDTH: f32 = 420.0;

/// A list of the project that is on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuiteList {
    Branches,
    Take,
    Extract,
}

/// What the window that chooses is open for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Purpose {
    Open,
    SaveAs,
    Import,
    ExtractTo(String),
}

pub(super) struct SuiteChooser {
    browser: Browser,
    purpose: Purpose,
}

impl App {
    /// The part that is selected: in the editor that has the document, else in the drawing.
    pub(super) fn selected_part(&self) -> Option<PartRef> {
        if let Some(brd) = &self.brd {
            return crate::board_gui::picked_part(brd);
        }
        if let Some(sch) = &self.sch {
            return crate::schematic_gui::picked_part(sch);
        }
        self.workspace
            .as_ref()
            .and_then(part_datasheet::of_workspace)
    }

    /// Shows the panel of the attachments with the data sheets of a part.
    pub(super) fn open_datasheet(&mut self, part: &PartRef) {
        let file = self.suite.file.clone();
        part_datasheet::show_sheets(&mut self.attachments.model, file.as_deref(), part);
        if file.is_none() {
            self.suite.status = None;
            self.error = Some(ViewError::new(
                FILE_UNUSABLE,
                S::NeedProject.tr(code(self.lang)),
            ));
        }
        self.show_panel(panels::id::ATTACHMENTS);
    }

    /// The button, in the other panels, that opens the data sheet of the selected part.
    pub(super) fn datasheet_button(&mut self, ui: &mut egui::Ui) {
        let Some(part) = self.selected_part() else {
            return;
        };
        let words = fill(SuiteMsg::SheetsOf.tr(self.lang), &[&part.label()]);
        if ui.button(words).clicked() {
            self.open_datasheet(&part);
        }
    }

    /// The attachments of the project of the suite, with the part that is selected above them.
    pub(super) fn panel_attachments(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        match self.selected_part() {
            Some(part) => {
                let label = part.label();
                ui.horizontal_wrapped(|ui| {
                    ui.label(heading(&fill(SuiteMsg::PartLine.tr(lang), &[&label])));
                    if ui
                        .button(fill(SuiteMsg::SheetsOf.tr(lang), &[&label]))
                        .clicked()
                    {
                        self.open_datasheet(&part);
                    }
                    let model = &self.attachments.model;
                    let chosen = (!model.busy())
                        .then(|| model.selected().map(|a| a.id.clone()))
                        .flatten();
                    let link = egui::Button::new(fill(SuiteMsg::LinkToPart.tr(lang), &[&label]));
                    if ui.add_enabled(chosen.is_some(), link).clicked() {
                        if let Some(request) = chosen.and_then(|id| part.link_request(&id)) {
                            self.attachments.model.request(request);
                        }
                    }
                });
            }
            None if self.workspace.is_some() => {
                ui.add(egui::Label::new(dim(SuiteMsg::ChoosePart.tr(lang))).wrap());
            }
            None => {}
        }
        let file = self.suite.file.clone();
        if self.attachments.show(ui, file.as_deref(), code(lang)) {
            self.suite.refresh();
        }
    }

    /// The window works on the project file that was opened, or on none for another file.
    pub(super) fn follow_opened(&mut self, path: &Path) {
        let before = self.suite.file.clone();
        if suiteview::is_project(path) {
            self.suite.adopt(path);
        } else {
            self.suite.close();
        }
        if before != self.suite.file {
            self.attachments.model.set_filter(None);
        }
    }

    fn design(&mut self) -> Option<Design> {
        let workspace = self.workspace.as_ref()?;
        match design_of(workspace) {
            Ok(design) => Some(design),
            Err(e) => {
                self.error = Some(ViewError::new(SAVE_FAILED, e));
                None
            }
        }
    }

    /// Whether the panel of the attachments is writing: a write of the window waits for it.
    fn attaching(&mut self) -> bool {
        let busy = self.attachments.model.busy();
        if busy {
            let words = tr(lang_of(code(self.lang)), Text::Working);
            self.error = Some(ViewError::new(SAVE_FAILED, words));
        }
        busy
    }

    /// What follows an entry of the suite: a question stays on the screen, a name is asked for,
    /// a refusal is told, and a write marks the document as saved.
    fn after_suite(&mut self, outcome: Outcome, saved: bool) {
        match outcome {
            Outcome::Done => {
                self.error = None;
                if let Some(loaded) = self.suite.show.take() {
                    let file = self.suite.file.clone();
                    self.show_loaded(loaded, file.as_deref());
                } else if saved {
                    if let (Some(workspace), Some(file)) =
                        (self.workspace.as_mut(), self.suite.file.as_deref())
                    {
                        workspace.mark_saved(file);
                    }
                }
            }
            Outcome::NeedsName => self.suite_ask(Purpose::SaveAs),
            Outcome::Asked => {}
            Outcome::Changed => self.error = None,
            Outcome::Refused(why) => self.error = Some(ViewError::new(SAVE_FAILED, why)),
        }
    }

    /// The design that was read from a project, or imported into it, is shown.
    fn show_loaded(&mut self, loaded: crate::load::Loaded, file: Option<&Path>) {
        let workspace = Workspace::new(
            loaded,
            file.map(Path::to_path_buf),
            self.settings.clone(),
            self.route_settings.clone(),
        );
        match workspace {
            Ok(workspace) => {
                if let Some(file) = file {
                    self.save_text = suggested_save_path(file).to_string_lossy().into_owned();
                    self.path_text = file.to_string_lossy().into_owned();
                    self.launcher.opened(file);
                }
                self.workspace = Some(workspace);
                if let Some(w) = self.workspace.as_mut() {
                    self.win.after_open(w, &self.settings);
                }
                self.error = None;
                self.fit_pending = true;
                self.tool_drag = false;
                self.sch = None;
                self.brd = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Opens a project chosen in the menu of the suite.
    pub(super) fn suite_open(&mut self, path: &Path) {
        let before = self.suite.file.clone();
        match self.suite.open(path, &self.settings) {
            Ok(loaded) => {
                self.error = None;
                if before != self.suite.file {
                    self.attachments.model.set_filter(None);
                }
                if let Some(loaded) = loaded {
                    self.show_loaded(loaded, Some(path));
                }
            }
            Err(why) => self.error = Some(ViewError::new(FILE_UNUSABLE, why)),
        }
    }

    pub(super) fn suite_save(&mut self) {
        if self.attaching() {
            return;
        }
        let design = self.design();
        let outcome = self.suite.save(design.as_ref(), &self.settings);
        self.after_suite(outcome, true);
    }

    /// The person chose a file or a folder in the window of the suite.
    pub(super) fn suite_chosen(&mut self, purpose: &Purpose, path: PathBuf) {
        match purpose {
            Purpose::Open => self.suite_open(&path),
            Purpose::SaveAs => {
                if !self.attaching() {
                    let design = self.design();
                    let outcome = self.suite.save_as(&path, design.as_ref(), &self.settings);
                    self.after_suite(outcome, true);
                }
            }
            Purpose::Import => {
                if !self.attaching() {
                    let design = self.design();
                    let outcome = self.suite.import(&path, design.as_ref(), &self.settings);
                    self.after_suite(outcome, false);
                }
            }
            Purpose::ExtractTo(branch) => {
                let outcome = self.suite.extract(branch, &path);
                self.after_suite(outcome, false);
            }
        }
    }

    /// Opens the window that chooses.
    pub(super) fn suite_ask(&mut self, purpose: Purpose) {
        let from = self
            .suite
            .file
            .clone()
            .or_else(|| self.workspace.as_ref().and_then(|w| w.input.clone()));
        let start = from
            .as_deref()
            .and_then(Path::parent)
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let (mode, filter, ext) = match &purpose {
            Purpose::Open => (Mode::Open, Filter::new(&["eh3d"]), None),
            Purpose::SaveAs => (Mode::Save, Filter::new(&["eh3d"]), Some("eh3d")),
            Purpose::Import => (Mode::Open, Filter::new(&["brd", "sch"]), None),
            Purpose::ExtractTo(_) => (Mode::Open, Filter::any(), None),
        };
        let mut browser = Browser::new(mode, &start, &start, vec![filter], ext);
        if purpose == Purpose::SaveAs {
            browser.name_text = self.suite.suggested_name(from.as_deref());
        }
        self.suite_chooser = Some(SuiteChooser { browser, purpose });
    }

    pub(super) fn show_suite_list(&mut self, list: SuiteList) {
        self.suite.refresh();
        self.suite_list = Some(list);
    }

    /// The project of the suite stays in sight: its name, what is out of date, what a write would
    /// unsign and what was last done.
    pub(super) fn suite_bar(&mut self, ui: &mut egui::Ui) {
        let lines = self.suite.banner(self.lang);
        if lines.is_empty() {
            return;
        }
        egui::Panel::top("suite").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (text, attention) in lines {
                    if attention {
                        ui.label(egui::RichText::new(text).strong().color(ui_danger()));
                    } else {
                        ui.label(dim(text));
                    }
                    ui.separator();
                }
            });
        });
    }

    /// The windows of the suite that float over the frame.
    pub(super) fn suite_windows(&mut self, ctx: &egui::Context) {
        self.suite_list_window(ctx);
        self.suite_confirm_window(ctx);
        self.suite_changed_window(ctx);
        self.suite_chooser_window(ctx);
    }

    /// What is offered when a write is refused because another program saved the project: read it
    /// again, or save under another name.
    fn suite_changed_window(&mut self, ctx: &egui::Context) {
        let Some(file) = self.suite.changed.clone() else {
            return;
        };
        let lang = self.lang;
        let code = code(lang);
        let line = Said(S::ChangedOnDisk, vec![suiteview::name_of(&file)]).text(code);
        let unsaved = self.workspace.as_ref().is_some_and(Workspace::dirty);
        let mut again = false;
        let mut save_as = false;
        let mut close = false;
        egui::Window::new(S::ConfirmTitle.tr(code))
            .id(egui::Id::unique("suite-changed"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(CHANGED_WIDTH);
                ui.add(
                    egui::Label::new(egui::RichText::new(line.as_str()).color(ui_danger())).wrap(),
                );
                // reading it again replaces the document of the window
                if unsaved {
                    ui.add(egui::Label::new(dim(SuiteMsg::UnsavedLost.tr(lang))).wrap());
                }
                ui.horizontal_wrapped(|ui| {
                    again = ui.button(S::ReadAgain.tr(code)).clicked();
                    save_as = ui.button(S::MenuSaveAs.tr(code)).clicked();
                    close = ui.button(S::Close.tr(code)).clicked();
                });
            });
        // Escape closes it; nothing is read and nothing is written
        close |= ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if again {
            self.suite.dismiss_changed();
            self.suite_open(&file);
        } else if save_as {
            self.suite.dismiss_changed();
            self.suite_ask(Purpose::SaveAs);
        } else if close {
            self.suite.dismiss_changed();
        }
    }

    /// The list of the branches, of the files to take or of the branches to extract.
    fn suite_list_window(&mut self, ctx: &egui::Context) {
        let Some(list) = self.suite_list else {
            return;
        };
        let (Some(view), code) = (self.suite.view.clone(), code(self.lang)) else {
            self.suite_list = None;
            return;
        };
        let title = match list {
            SuiteList::Branches => S::MenuBranches,
            SuiteList::Take => S::MenuTake,
            SuiteList::Extract => S::MenuExtract,
        };
        let mut open = true;
        let mut take = None;
        let mut extract = None;
        let mut again = false;
        egui::Window::new(title.tr(code))
            .id(egui::Id::unique("suite-list"))
            .open(&mut open)
            .collapsible(false)
            .default_size([620.0, 360.0])
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| match list {
                        SuiteList::Branches => {
                            egui::Grid::new("suite-branches")
                                .num_columns(5)
                                .striped(true)
                                .show(ui, |ui| {
                                    for word in [
                                        S::ColBranch,
                                        S::ColApp,
                                        S::ColFormat,
                                        S::ColFiles,
                                        S::ColState,
                                    ] {
                                        ui.label(heading(word.tr(code)));
                                    }
                                    ui.end_row();
                                    for b in &view.branches {
                                        ui.label(b.id.as_str());
                                        ui.label(b.app.as_str());
                                        ui.label(b.format.as_str());
                                        ui.label(b.files.to_string());
                                        let state =
                                            egui::RichText::new(suiteview::state_text(b, code));
                                        ui.label(if b.stale.is_empty() {
                                            state
                                        } else {
                                            state.color(ui_danger())
                                        });
                                        ui.end_row();
                                    }
                                });
                        }
                        SuiteList::Take => {
                            let words = if view.takeable.is_empty() {
                                S::NothingToTake
                            } else {
                                S::ChooseFile
                            };
                            ui.label(words.tr(code));
                            for (branch, name) in &view.takeable {
                                if ui
                                    .selectable_label(false, format!("{branch}/{name}"))
                                    .clicked()
                                {
                                    take = Some((branch.clone(), name.clone()));
                                }
                            }
                        }
                        SuiteList::Extract => {
                            ui.label(S::ChooseBranch.tr(code));
                            for b in &view.branches {
                                if ui.selectable_label(false, b.id.as_str()).clicked() {
                                    extract = Some(b.id.clone());
                                }
                            }
                        }
                    });
                ui.separator();
                if ui.button(S::ReadAgain.tr(code)).clicked() {
                    again = true;
                }
            });
        if again {
            self.suite.refresh();
        }
        if !open {
            self.suite_list = None;
        }
        if let Some((branch, name)) = take {
            self.suite_list = None;
            if !self.attaching() {
                let outcome = self.suite.take(&branch, &name, &self.settings);
                self.after_suite(outcome, false);
            }
        }
        if let Some(branch) = extract {
            self.suite_list = None;
            self.suite_ask(Purpose::ExtractTo(branch));
        }
    }

    /// The question before a project is written when there is something to agree to.
    fn suite_confirm_window(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.suite.pending else {
            return;
        };
        let lang = self.lang;
        let code = code(lang);
        let lines: Vec<String> = pending.warnings.iter().map(|w| w.text(lang)).collect();
        let saves = matches!(pending.action, crate::suite_menu::Action::Save(_));
        let mut answer = None;
        egui::Window::new(S::ConfirmTitle.tr(code))
            .id(egui::Id::unique("suite-confirm"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                for line in &lines {
                    ui.label(egui::RichText::new(line.as_str()).color(ui_danger()));
                }
                ui.horizontal(|ui| {
                    if ui.button(S::Write.tr(code)).clicked() {
                        answer = Some(true);
                    }
                    if ui.button(tr(lang_of(code), Text::Cancel)).clicked() {
                        answer = Some(false);
                    }
                });
            });
        match answer {
            Some(true) => {
                if !self.attaching() {
                    let design = self.design();
                    let outcome = self.suite.confirm(design.as_ref(), &self.settings);
                    self.after_suite(outcome, saves);
                }
            }
            Some(false) => self.suite.cancel(),
            None => {}
        }
    }

    /// The window that chooses a file or a folder.
    fn suite_chooser_window(&mut self, ctx: &egui::Context) {
        let Some(mut chooser) = self.suite_chooser.take() else {
            return;
        };
        let lang = self.lang;
        let code = code(lang);
        let words = lang_of(code);
        let folder = matches!(chooser.purpose, Purpose::ExtractTo(_));
        let title = match chooser.purpose {
            Purpose::Open => S::MenuOpen.tr(code),
            Purpose::SaveAs => S::MenuSaveAs.tr(code),
            Purpose::Import => S::MenuImport.tr(code),
            Purpose::ExtractTo(_) => S::MenuExtract.tr(code),
        };
        let accept = match chooser.purpose {
            Purpose::Open | Purpose::Import => Msg::Open.tr(lang),
            Purpose::SaveAs => S::Write.tr(code),
            Purpose::ExtractTo(_) => S::Extract.tr(code),
        };
        let mut open = true;
        let mut cancelled = false;
        let mut picked = None;
        egui::Window::new(title)
            .id(egui::Id::unique("suite-chooser"))
            .open(&mut open)
            .collapsible(false)
            .default_size([560.0, 420.0])
            .show(ctx, |ui| {
                let browser = &mut chooser.browser;
                let places = browser.places.clone();
                ui.horizontal_wrapped(|ui| {
                    for (i, place) in places.iter().enumerate() {
                        let name = match place.kind {
                            PlaceKind::Examples => tr(words, Text::ProjectFolder).to_owned(),
                            PlaceKind::Desktop => tr(words, Text::Desktop).to_owned(),
                            PlaceKind::Documents => tr(words, Text::Documents).to_owned(),
                            PlaceKind::Downloads => tr(words, Text::Downloads).to_owned(),
                            PlaceKind::Drive => place.name.clone(),
                        };
                        if ui.button(name).clicked() {
                            browser.go_place(i);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button(tr(words, Text::Up)).clicked() {
                        browser.up();
                    }
                    let edit = egui::TextEdit::singleline(&mut browser.path_text)
                        .desired_width(ui.available_width());
                    let typed = ui.add(edit);
                    if typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        browser.go_typed_path();
                    }
                });
                let mut clicked = None;
                let mut activated = None;
                ScrollArea::vertical()
                    .id_salt("suite-folder")
                    .max_height(260.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, entry) in browser.entries.iter().enumerate() {
                            let text = if entry.is_dir {
                                egui::RichText::new(format!("▸ {}", entry.name)).strong()
                            } else {
                                egui::RichText::new(format!(
                                    "{}   {}",
                                    entry.name,
                                    size_text(entry.size)
                                ))
                            };
                            let line = ui.selectable_label(browser.sel == Some(i), text);
                            if line.double_clicked() {
                                activated = Some(i);
                            } else if line.clicked() {
                                clicked = Some(i);
                            }
                        }
                    });
                if let Some(i) = clicked {
                    browser.select(i);
                }
                if let Some(i) = activated {
                    if let Picked::Chosen(path) = browser.activate(i) {
                        // a double click on a file is the button, except where a folder is asked for
                        if !folder {
                            picked = Some(path);
                        }
                    }
                }
                if !folder {
                    ui.add(
                        egui::TextEdit::singleline(&mut browser.name_text)
                            .desired_width(f32::INFINITY),
                    );
                }
                ui.horizontal(|ui| {
                    if ui.button(accept).clicked() {
                        picked = if folder {
                            browser.dir.clone()
                        } else {
                            match browser.confirm() {
                                // a project that is there is asked about before it is written
                                Ok(Chosen::File(path) | Chosen::Overwrite(path)) => Some(path),
                                Err(_) => None,
                            }
                        };
                    }
                    if ui.button(tr(words, Text::Cancel)).clicked() {
                        cancelled = true;
                    }
                });
            });
        match picked {
            Some(path) => self.suite_chosen(&chooser.purpose, path),
            None if open && !cancelled => self.suite_chooser = Some(chooser),
            None => {}
        }
    }
}
