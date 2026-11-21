// SPDX-License-Identifier: GPL-3.0-or-later
//! The window (built with the `gui` feature): it draws the state of [`Workspace`] with egui on
//! eframe and passes on what the person does. Everything that can be decided without a screen is
//! decided in the workspace, in code that is tested. This file has not been seen on a screen.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke};
use eh3d_dock::core::AboutInfo;
use eh3d_dock::gui::Dock;
use eh3d_dock::menu::{Chosen, DockAction, MenuBar, StdAction};
use eh3d_phoenix_route::{Mode, RouteSettings};
use eh3d_phoenix_rules::Lang;

use crate::appearance;
use crate::board_gui::{self, BoardEditUi};
use crate::board_text::BoardMsg;
use crate::canvas_engine::{self, Engine, FrameLog};
use crate::cfg_edit::CfgEditor;
use crate::cfg_gui;
use crate::cfg_text::CfgMsg;
use crate::codes::{ViewError, FILE_UNUSABLE, WINDOW_FAILED};
use crate::console_log::{ConsoleLog, Level};
use crate::dock_text::DockMsg;
use crate::drc_gui;
use crate::editmsg::EditMsg;
use crate::gerber_gui::GerberUi;
use crate::help_text::HelpMode;
use crate::keymap::Action;
use crate::launcher_gui;
use crate::launcher_session::{Launch, Launcher};
use crate::launcher_text::LauncherText;
use crate::library_gui::LibraryUi;
use crate::library_text::LibText;
use crate::msg::Msg;
use crate::object_list;
use crate::panel_gui::PanelUi;
use crate::panel_scene::parse_files;
use crate::panel_text::PanelText;
use crate::panels::{self, Cmd, LayoutWatch, MenuFacts};
use crate::prim::{arc_points, triangulate, Shape, P};
use crate::print_gui::PrintUi;
use crate::report_panel::Report;
use crate::scene::SceneKind;
use crate::schematic_gui::{self, SchematicEditUi};
use crate::schematic_text::SchMsg;
use crate::settings::ViewerSettings;
use crate::shot::{ShotRequest, ShotWindow};
use crate::view3d_gui::View3dUi;
use crate::view3d_text::View3dText;
use crate::viewer::Tool;
use crate::window_gui::{self, FrameContext, WindowUi};
use crate::workspace::{erc_line, finding_line, suggested_save_path, Workspace};

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

// the colours of the controls in the theme in use (amber on charcoal in the dark one, which the
// programs of the suite start with); they change when the person chooses another theme
pub(crate) fn ui_text() -> Color32 {
    rgba(appearance::current().text)
}

pub(crate) fn ui_dim() -> Color32 {
    rgba(appearance::current().dim)
}

pub(crate) fn ui_accent() -> Color32 {
    rgba(appearance::current().accent)
}

pub(crate) fn ui_ok() -> Color32 {
    rgba(appearance::current().ok)
}

pub(crate) fn ui_danger() -> Color32 {
    rgba(appearance::current().danger)
}

/// What draws the primitives of the scene in the canvas. The window draws with the painter of egui; the
/// engine of the decision record of the interface stack (a callback of wgpu fed by the same primitives) is
/// another implementation of this trait, chosen by the setting `canvas_engine`, so that changing the engine
/// does not touch the viewer or the panels.
trait CanvasEngine {
    /// Which engine this is.
    fn engine(&self) -> Engine;
    /// Draws the visible layers of the workspace in a rectangle of the window.
    fn paint(&self, workspace: &Workspace, painter: &egui::Painter, rect: Rect);
}

/// The painter of egui: it tessellates the primitives on the processor at every frame.
struct PainterEngine;

impl CanvasEngine for PainterEngine {
    fn engine(&self) -> Engine {
        Engine::Painter
    }

    fn paint(&self, workspace: &Workspace, painter: &egui::Painter, rect: Rect) {
        paint(workspace, painter, rect);
    }
}

/// The engine for a request, and whether it is not the one that was asked for (one that is not built makes
/// the painter draw).
fn engine_for(asked: Engine) -> (Box<dyn CanvasEngine>, bool) {
    let choice = canvas_engine::choose(asked);
    let engine: Box<dyn CanvasEngine> = match choice.engine {
        Engine::Painter => Box::new(PainterEngine),
        // the callback of wgpu is not written, and `choose` never gives it
        Engine::Wgpu => Box::new(PainterEngine),
    };
    (engine, choice.fell_back)
}

fn language_name(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "English",
        Lang::Es => "Español",
        Lang::De => "Deutsch",
        Lang::Fr => "Français",
        Lang::It => "Italiano",
        Lang::Ja => "日本語",
        Lang::Zh => "中文",
        Lang::Pt => "Português (Portugal)",
    }
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ui_accent())
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ui_dim())
}

struct App {
    lang: Lang,
    settings: ViewerSettings,
    route_settings: RouteSettings,
    workspace: Option<Workspace>,
    path_text: String,
    save_text: String,
    rules_text: String,
    board_text: String,
    board_save_text: String,
    error: Option<ViewError>,
    fit_pending: bool,
    /// True while the drag in progress belongs to a tool and not to the panning of the drawing.
    tool_drag: bool,
    focus_save: bool,
    /// The schematic editor, while the person is editing the schematic that is open.
    sch: Option<SchematicEditUi>,
    /// The board editor, while the person is editing the board that is open.
    brd: Option<BoardEditUi>,
    /// The library mode, while a library of footprints, symbols and devices is open.
    lib: LibraryUi,
    /// The start screen, shown while nothing is open: the typed path and the recent files.
    launcher: Launcher,
    /// What the start screen could not do last (an empty path, dropped files that do not open).
    launch_note: Option<LauncherText>,
    /// The window of the imported Gerber and Excellon files, while a folder of them is shown.
    gerber: Option<GerberUi>,
    /// The 3D window of the assembled board, while it is shown.
    view3d: Option<View3dUi>,
    /// The window of the panel of boards, while it is shown.
    panel: Option<PanelUi>,
    /// The table of models that the 3D window reads.
    models_text: String,
    /// The controls that export the open document as SVG or PDF.
    print: PrintUi,
    /// The search, the measures, the layers, the history, the appearance, the shortcuts, the help
    /// and the preferences that are remembered.
    win: WindowUi,
    /// The window of the settings files, while it is open.
    rules: Option<CfgEditor>,
    /// The actions whose keys were pressed in this frame.
    actions: Vec<Action>,
    /// What the console panel shows.
    console: ConsoleLog,
    /// The words typed to filter the list of components.
    comp_filter: String,
    /// What the menus asked of the frame (show a panel, ...), done after the frame is drawn.
    pending_dock: Vec<DockAction>,
    /// The files that were opened in this frame, for the list of recent files of the menu.
    recent_added: Vec<String>,
    /// The box of the path takes the keyboard in the next frame.
    focus_path: bool,
    /// What draws the primitives in the canvas.
    engine: Box<dyn CanvasEngine>,
    /// The time of the last frames, for the report panel.
    frames: FrameLog,
    /// The picture mode, while the window is there only to be photographed.
    shot: Option<ShotState>,
    /// The panel that has the focus in the frame, for the help and for the preferences.
    focused: Option<String>,
}

/// How the picture mode ended: nothing yet, the file was written, or the reason it was not.
type ShotOutcome = Arc<Mutex<Option<Result<(), String>>>>;

/// The frames after the last one asked for in which the picture may still come back.
const SHOT_PATIENCE_FRAMES: u32 = 600;

/// The picture being taken.
struct ShotState {
    request: ShotRequest,
    frame: u32,
    outcome: ShotOutcome,
}

impl App {
    /// With `persist` false the start screen keeps its list of recent files in memory only.
    fn new(
        lang: Lang,
        settings: ViewerSettings,
        win: WindowUi,
        initial: Option<PathBuf>,
        persist: bool,
    ) -> App {
        let launcher = if persist {
            Launcher::from_env(&settings, &|name: &str| std::env::var(name).ok())
        } else {
            Launcher::new(&settings, None)
        };
        let (engine, fell_back) = engine_for(settings.canvas_engine);
        let mut app = App {
            lang,
            settings: settings.clone(),
            route_settings: RouteSettings::default(),
            workspace: None,
            path_text: String::new(),
            save_text: String::new(),
            rules_text: String::new(),
            board_text: String::new(),
            board_save_text: String::new(),
            error: None,
            fit_pending: false,
            tool_drag: false,
            focus_save: false,
            sch: None,
            brd: None,
            lib: LibraryUi::new(eh3d_phoenix_libedit::LibSettings::default()),
            launcher,
            launch_note: None,
            gerber: None,
            view3d: None,
            panel: None,
            models_text: String::new(),
            print: PrintUi::new(),
            win,
            rules: None,
            actions: Vec::new(),
            console: ConsoleLog::new(settings.console_max_lines),
            comp_filter: String::new(),
            pending_dock: Vec::new(),
            recent_added: Vec::new(),
            focus_path: false,
            engine,
            frames: FrameLog::new(settings.frame_log_frames),
            shot: None,
            focused: None,
        };
        if fell_back {
            app.console
                .push(Level::Info, DockMsg::EngineFellBack.tr(lang));
        }
        if let Some(path) = initial {
            app.path_text = path.to_string_lossy().into_owned();
            app.open(&path);
        }
        app
    }

    /// Reads and draws a file; a file that cannot be used leaves the drawing as it was.
    fn open(&mut self, path: &Path) {
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lbr"))
        {
            self.lib.open(path);
            if self.lib.is_open() {
                self.error = None;
                self.launcher.opened(path);
                self.recent_added.push(path.to_string_lossy().into_owned());
                self.show_panel(panels::id::LIBRARY_EDITOR);
            } else if let Some(e) = self.lib.take_error() {
                self.error = Some(ViewError::new(FILE_UNUSABLE, e.message(Lang::En)));
            }
            return;
        }
        match Workspace::open(path, &self.settings, &self.route_settings) {
            Ok(workspace) => {
                self.save_text = suggested_save_path(path).to_string_lossy().into_owned();
                self.workspace = Some(workspace);
                if let Some(w) = self.workspace.as_mut() {
                    self.win.after_open(w, &self.settings);
                }
                self.error = None;
                self.fit_pending = true;
                self.tool_drag = false;
                self.sch = None;
                self.brd = None;
                self.launcher.opened(path);
                self.recent_added.push(path.to_string_lossy().into_owned());
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Asks the frame to show a panel (and to put it in front) after this frame is drawn.
    fn show_panel(&mut self, id: &str) {
        self.pending_dock
            .push(DockAction::ShowPanel(id.to_string()));
    }

    /// Asks the frame to hide a panel after this frame is drawn.
    fn hide_panel(&mut self, id: &str) {
        self.pending_dock
            .push(DockAction::HidePanel(id.to_string()));
    }

    /// Does what the start screen asked for. What is opened shows in a panel of its own beside the
    /// others; nothing that is open is closed by it.
    fn launch(&mut self, asked: Launch) {
        match asked {
            Launch::Open(path) => {
                self.path_text = path.to_string_lossy().into_owned();
                self.open(&path);
            }
            Launch::OpenGerber(folder) => match GerberUi::open(&folder, &self.settings) {
                Ok(window) => {
                    self.gerber = Some(window);
                    self.error = None;
                    self.show_panel(panels::id::GERBER);
                }
                Err(error) => self.error = Some(error),
            },
            Launch::OpenPanel(text) => {
                self.panel = Some(PanelUi::new(&parse_files(&text), None, &self.settings));
                self.error = None;
                self.show_panel(panels::id::BOARDS);
            }
            Launch::NewLibrary => {
                self.lib.new_library();
                self.show_panel(panels::id::LIBRARY_EDITOR);
            }
        }
    }

    /// Starts the picture mode: shows the window asked for (`file` is what the document windows
    /// open) and keeps what is needed to take the picture and close.
    fn begin_shot(&mut self, request: ShotRequest, file: Option<&Path>, outcome: ShotOutcome) {
        match (request.window, file) {
            (ShotWindow::View3d, _) => self.open_3d(),
            (ShotWindow::Panel, Some(path)) => {
                self.launch(Launch::OpenPanel(path.to_string_lossy().into_owned()));
            }
            (ShotWindow::Gerber, Some(path)) => {
                self.launch(Launch::OpenGerber(path.to_path_buf()));
            }
            _ => self.win.show_window(request.window),
        }
        self.shot = Some(ShotState {
            request,
            frame: 0,
            outcome,
        });
    }

    /// Counts the frames, asks for the picture after the frames waited, writes it and closes the
    /// window when it is written or when it does not come.
    fn shot_tick(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        shot.frame += 1;
        ctx.request_repaint();
        let due = 3 + shot.request.wait;
        let finished = match shot.outcome.lock() {
            Ok(slot) => slot.is_some(),
            Err(_) => true,
        };
        if finished {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if shot.frame == due {
            let (output, replace) = (shot.request.output.clone(), shot.request.replace);
            let (outcome, repaint) = (shot.outcome.clone(), ctx.clone());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |image| {
                    let result = save_shot_png(&image, &output, replace);
                    if let Ok(mut slot) = outcome.lock() {
                        *slot = Some(result);
                    }
                    repaint.request_repaint();
                }),
            ));
        } else if shot.frame > due + SHOT_PATIENCE_FRAMES {
            if let Ok(mut slot) = shot.outcome.lock() {
                *slot = Some(Err(
                    "the picture did not come back from the window".to_owned()
                ));
            }
        }
    }

    /// Opens the 3D window of the board that is open, with the table of models that is typed.
    fn open_3d(&mut self) {
        let Some(session) = self.workspace.as_ref().and_then(|w| w.board_session()) else {
            return;
        };
        let typed = self.models_text.trim();
        let table = (!typed.is_empty()).then(|| PathBuf::from(typed));
        match View3dUi::open(&session.board, table.as_deref(), &self.settings) {
            Ok(window) => {
                self.view3d = Some(window);
                self.error = None;
                self.show_panel(panels::id::VIEW3D);
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// The line of a panel that needs a document while nothing is open.
    fn no_document(ui: &mut egui::Ui, lang: Lang) {
        ui.add_space(6.0);
        ui.add(egui::Label::new(dim(DockMsg::NoDocument.tr(lang))).wrap());
    }

    /// The line of a panel that reads the document while an editor has it.
    fn editor_has_document(ui: &mut egui::Ui, lang: Lang) {
        ui.add_space(6.0);
        ui.add(egui::Label::new(dim(DockMsg::EditorHasDocument.tr(lang))).wrap());
    }

    /// The box that chooses the language of the window.
    fn language_combo(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::ComboBox::from_id_salt("language")
            .selected_text(language_name(self.lang))
            .show_ui(ui, |ui| {
                for l in Lang::ALL {
                    ui.selectable_value(&mut self.lang, l, language_name(l));
                }
            });
        ui.label(dim(Msg::Language.tr(lang)));
    }

    /// The tasks: the file of the document and the tools of the window that the menus also have.
    fn panel_tasks(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.label(heading(DockMsg::SectionDocument.tr(lang)));
            ui.add(egui::Label::new(dim(Msg::PathHint.tr(lang))).wrap());
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 90.0).clamp(80.0, 420.0);
                let edit = egui::TextEdit::singleline(&mut self.path_text).desired_width(width);
                let response = ui.add(edit);
                if self.focus_path {
                    response.request_focus();
                    self.focus_path = false;
                }
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(Msg::Open.tr(lang)).clicked() || enter {
                    let path = PathBuf::from(self.path_text.trim());
                    self.open(&path);
                }
            });
            let editing = self.sch.is_some() || self.brd.is_some();
            if let Some(w) = self.workspace.as_mut() {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            !editing && w.can_undo(),
                            egui::Button::new(EditMsg::Undo.tr(lang)),
                        )
                        .clicked()
                    {
                        w.undo();
                    }
                    if ui
                        .add_enabled(
                            !editing && w.can_redo(),
                            egui::Button::new(EditMsg::Redo.tr(lang)),
                        )
                        .clicked()
                    {
                        w.redo();
                    }
                    if w.dirty() {
                        ui.label(egui::RichText::new(EditMsg::Unsaved.tr(lang)).color(ui_danger()));
                    }
                });
                ui.label(dim(EditMsg::SavePath.tr(lang)));
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 90.0).clamp(80.0, 420.0);
                    let save = egui::TextEdit::singleline(&mut self.save_text).desired_width(width);
                    let response = ui.add(save);
                    if self.focus_save {
                        response.request_focus();
                        self.focus_save = false;
                    }
                    if ui.button(EditMsg::SaveAs.tr(lang)).clicked() {
                        let target = PathBuf::from(self.save_text.trim());
                        w.save_as(&target);
                    }
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| self.print.show(ui, w, lang));
            }
            ui.separator();
            ui.label(heading(DockMsg::SectionTools.tr(lang)));
            ui.horizontal_wrapped(|ui| {
                if ui.button(LibText::Title.tr(lang)).clicked() {
                    self.launch(Launch::NewLibrary);
                }
                if ui.button(CfgMsg::Title.tr(lang)).clicked() && self.rules.take().is_none() {
                    self.rules = Some(CfgEditor::new(&self.settings));
                }
                let has_scene = self.workspace.is_some();
                if ui
                    .add_enabled(has_scene, egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
            });
            if self.workspace.as_ref().is_some_and(|w| w.is_board()) {
                ui.label(dim(View3dText::ModelsTable.tr(lang)));
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 90.0).clamp(80.0, 300.0);
                    let table =
                        egui::TextEdit::singleline(&mut self.models_text).desired_width(width);
                    ui.add(table);
                    if ui.button(View3dText::Show3d.tr(lang)).clicked() {
                        self.open_3d();
                    }
                });
            }
            ui.separator();
            ui.horizontal_wrapped(|ui| self.language_combo(ui));
        });
    }

    /// The tools of the document and the settings of the tool in use; while an editor of the board or the
    /// schematic has the document, the tools of the editor.
    fn panel_properties(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Some(workspace) = self.workspace.as_mut() else {
            App::no_document(ui, lang);
            return;
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            let mut leave = false;
            if let Some(sch) = self.sch.as_mut() {
                leave = schematic_gui::tools_panel(ui, sch, lang);
                if leave {
                    schematic_gui::finish(sch, workspace);
                }
            }
            if leave {
                self.sch = None;
                self.fit_pending = true;
            }
            if self.sch.is_some() {
                return;
            }
            let mut leave_board = false;
            if let Some(brd) = self.brd.as_mut() {
                leave_board = board_gui::tools_panel(ui, brd, lang);
                if leave_board {
                    board_gui::finish(brd, workspace);
                }
            }
            if leave_board {
                self.brd = None;
                self.fit_pending = true;
            }
            if self.brd.is_some() {
                return;
            }
            let board = workspace.is_board();
            let viewer = &mut workspace.viewer;
            let kind = match viewer.scene.kind {
                SceneKind::Board => Msg::Board,
                SceneKind::Schematic => Msg::Schematic,
            };
            ui.label(heading(kind.tr(lang)));
            ui.label(dim(format!(
                "{} {} · {} {} · {} {}",
                viewer.scene.prims.len(),
                Msg::Primitives.tr(lang),
                viewer.scene.items.len(),
                Msg::Items.tr(lang),
                viewer.scene.nets.len(),
                Msg::Nets.tr(lang)
            )));
            let not_drawn = viewer.scene.skipped + workspace.unread;
            if not_drawn > 0 {
                ui.label(dim(format!("{} {}", not_drawn, Msg::Skipped.tr(lang))));
            }
            ui.separator();
            let mut tool = workspace.viewer.tool;
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut tool, Tool::Select, Msg::ToolSelect.tr(lang));
                ui.selectable_value(&mut tool, Tool::Measure, Msg::ToolMeasure.tr(lang));
            });
            if board {
                ui.label(heading(EditMsg::EditTitle.tr(lang)));
                ui.horizontal_wrapped(|ui| {
                    for (value, text) in [
                        (Tool::MoveComponent, EditMsg::ToolMove),
                        (Tool::RotateComponent, EditMsg::ToolRotate),
                        (Tool::DeleteTrack, EditMsg::ToolDeleteTrack),
                        (Tool::MoveVertex, EditMsg::ToolMoveVertex),
                        (Tool::SetWidth, EditMsg::ToolWidth),
                        (Tool::AddVia, EditMsg::ToolVia),
                        (Tool::Route, EditMsg::ToolRoute),
                    ] {
                        ui.selectable_value(&mut tool, value, text.tr(lang));
                    }
                });
            }
            if tool != workspace.viewer.tool {
                workspace.set_tool(tool);
            }
            if !board && ui.button(SchMsg::EditSchematic.tr(lang)).clicked() {
                self.sch = schematic_gui::start(workspace);
                self.fit_pending = true;
            }
            if board && ui.button(BoardMsg::EditBoard.tr(lang)).clicked() {
                self.brd = board_gui::start(workspace);
                self.fit_pending = true;
            }
            if board {
                ui.horizontal(|ui| {
                    ui.label(dim(EditMsg::ActiveWidth.tr(lang)));
                    ui.add(
                        egui::DragValue::new(&mut workspace.active_width_mm)
                            .speed(0.01)
                            .range(0.02..=10.0)
                            .fixed_decimals(3),
                    );
                });
                ui.radio_value(
                    &mut workspace.route_mode,
                    Mode::Around,
                    EditMsg::ModeAround.tr(lang),
                );
                ui.radio_value(
                    &mut workspace.route_mode,
                    Mode::Stop,
                    EditMsg::ModeStop.tr(lang),
                );
                ui.checkbox(&mut workspace.show_ratsnest, EditMsg::ShowRatsnest.tr(lang));
            }
            if ui.button(Msg::ClearSelection.tr(lang)).clicked() {
                workspace.cancel();
            }
            ui.label(dim(EditMsg::ShortcutsHint.tr(lang)));
        });
    }

    /// The layers of the document, each with its box and its colour.
    fn panel_layers(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if self.sch.is_some() || self.brd.is_some() {
            App::editor_has_document(ui, lang);
            return;
        }
        let Some(workspace) = self.workspace.as_mut() else {
            App::no_document(ui, lang);
            return;
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            let viewer = &mut workspace.viewer;
            ui.horizontal(|ui| {
                if ui.small_button(Msg::ShowAll.tr(lang)).clicked() {
                    viewer.set_all_visible(true);
                }
                if ui.small_button(Msg::HideAll.tr(lang)).clicked() {
                    viewer.set_all_visible(false);
                }
            });
            let counts = viewer.layer_counts();
            for i in 0..viewer.scene.layers.len() {
                let (name, mut visible, colour) = {
                    let layer = &viewer.scene.layers[i];
                    let name = Msg::for_role(layer.role)
                        .map_or_else(|| layer.name.clone(), |m| m.tr(lang).to_string());
                    (name, layer.visible, layer.colour)
                };
                let count = counts.get(i).copied().unwrap_or(0);
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut visible, format!("{name} ({count})"))
                        .changed()
                    {
                        viewer.set_layer_visible(i, visible);
                    }
                    let mut chosen = rgba(colour);
                    if ui.color_edit_button_srgba(&mut chosen).changed() {
                        viewer
                            .set_layer_colour(i, [chosen.r(), chosen.g(), chosen.b(), chosen.a()]);
                    }
                });
            }
        });
    }

    /// The components of the board (or the parts of the schematic) with a filter, and the counts of the objects.
    fn panel_components(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if self.sch.is_some() || self.brd.is_some() {
            App::editor_has_document(ui, lang);
            return;
        }
        let Some(workspace) = self.workspace.as_mut() else {
            App::no_document(ui, lang);
            return;
        };
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.comp_filter)
                .hint_text(DockMsg::ComponentsFilter.tr(lang))
                .desired_width(f32::INFINITY),
        );
        let list = object_list::components(
            &workspace.viewer.scene,
            &self.comp_filter,
            self.settings.component_rows_max,
        );
        ui.label(dim(format!(
            "{} / {} {}",
            list.rows.len(),
            list.total,
            DockMsg::Shown.tr(lang)
        )));
        let counts = object_list::counts(&workspace.viewer.scene);
        ui.collapsing(DockMsg::ObjectsTitle.tr(lang), |ui| {
            for (kind, n) in &counts {
                ui.label(dim(format!("{}: {n}", kind.name(lang))));
            }
        });
        ui.separator();
        let selected = workspace.viewer.selected;
        let mut chosen: Option<u32> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in &list.rows {
                if ui
                    .selectable_label(selected == Some(row.item), &row.label)
                    .clicked()
                {
                    chosen = Some(row.item);
                }
            }
        });
        if let Some(item) = chosen {
            object_list::focus(&mut workspace.viewer, item);
        }
    }

    /// The nets, with the connections of each that are still to be routed.
    fn panel_nets(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if self.sch.is_some() || self.brd.is_some() {
            App::editor_has_document(ui, lang);
            return;
        }
        let Some(workspace) = self.workspace.as_mut() else {
            App::no_document(ui, lang);
            return;
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            net_list(ui, workspace, lang);
        });
    }

    /// The problems of the rules of the board, or the electrical rule check of the schematic.
    fn panel_drc(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if self.sch.is_some() || self.brd.is_some() {
            App::editor_has_document(ui, lang);
            return;
        }
        let Some(workspace) = self.workspace.as_mut() else {
            App::no_document(ui, lang);
            return;
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            if workspace.is_board() {
                board_panel(ui, workspace, &mut self.rules_text, lang);
            } else {
                schematic_panel(
                    ui,
                    workspace,
                    &mut self.board_text,
                    &mut self.board_save_text,
                    lang,
                );
            }
        });
    }

    /// The libraries: a new one, and the ones that were opened lately.
    fn panel_library(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.add_space(4.0);
        ui.label(heading(LibText::Title.tr(lang)));
        ui.add(egui::Label::new(dim(DockMsg::LibraryHint.tr(lang))).wrap());
        if ui
            .button(crate::launcher_text::LauncherText::NewLibrary.tr(lang))
            .clicked()
        {
            self.launch(Launch::NewLibrary);
        }
        ui.separator();
        ui.label(dim(crate::launcher_text::LauncherText::Recent.tr(lang)));
        let libraries: Vec<PathBuf> = self
            .launcher
            .recent()
            .entries()
            .iter()
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lbr")))
            .cloned()
            .collect();
        if libraries.is_empty() {
            ui.label(dim(crate::launcher_text::LauncherText::NoRecent.tr(lang)));
        }
        let mut open: Option<PathBuf> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for path in &libraries {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                if ui
                    .selectable_label(false, name)
                    .on_hover_text(path.to_string_lossy())
                    .clicked()
                {
                    open = Some(path.clone());
                }
            }
        });
        if let Some(path) = open {
            self.path_text = path.to_string_lossy().into_owned();
            self.open(&path);
        }
    }

    /// What the window had to say: the notes, the errors and the answers of the checks, in order.
    fn panel_console(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.small_button(DockMsg::ConsoleClear.tr(lang)).clicked() {
                self.console.clear();
            }
        });
        if self.console.is_empty() {
            ui.label(dim(DockMsg::ConsoleEmpty.tr(lang)));
        }
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in self.console.lines() {
                    let colour = match line.level {
                        Level::Error => ui_danger(),
                        Level::Info => ui_text(),
                    };
                    ui.add(egui::Label::new(egui::RichText::new(&line.text).color(colour)).wrap());
                }
            });
    }

    /// The report of the document: its counts, the problems of the rules and the connections still to be routed.
    fn panel_report(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if self.sch.is_some() || self.brd.is_some() {
            App::editor_has_document(ui, lang);
            return;
        }
        let Some(workspace) = self.workspace.as_ref() else {
            App::no_document(ui, lang);
            return;
        };
        let mut report = Report::of_scene(&workspace.viewer.scene);
        if let Some(session) = workspace.board_session() {
            report = report
                .with_findings(&session.findings)
                .with_unrouted(session.rat.len());
        } else if let Some(session) = workspace.schematic_session() {
            report = report.with_severities(session.erc.confirmed.iter().map(|f| f.severity));
        }
        ui.add_space(4.0);
        if ui.button(DockMsg::ReportCopy.tr(lang)).clicked() {
            ui.ctx().copy_text(report.text(lang));
        }
        ui.separator();
        for line in report.lines(lang) {
            ui.label(line);
        }
        if let Some(s) = self.frames.stats() {
            ui.separator();
            ui.label(dim(format!(
                "{}: {:.1} / {:.1} / {:.1} ms ({} · {})",
                DockMsg::FrameTime.tr(lang),
                s.median_ms,
                s.p95_ms,
                s.p99_ms,
                s.frames,
                self.engine.engine().key()
            )));
        }
    }

    /// The board or the schematic, drawn; the start screen while nothing is open.
    fn panel_board_view(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let background = rgba(self.settings.colour("background"));
        if let (Some(sch), Some(workspace)) = (self.sch.as_mut(), self.workspace.as_mut()) {
            schematic_gui::canvas(ui, sch, workspace, lang, background, &mut self.fit_pending);
            return;
        }
        if let (Some(brd), Some(workspace)) = (self.brd.as_mut(), self.workspace.as_mut()) {
            board_gui::canvas(ui, brd, workspace, lang, background, &mut self.fit_pending);
            return;
        }
        if self.workspace.is_none() {
            // nothing is open: the start screen
            ui.painter().rect_filled(ui.max_rect(), 0.0, background);
            if let Some(asked) =
                launcher_gui::show(ui, &mut self.launcher, lang, &mut self.launch_note)
            {
                self.launch(asked);
            }
            return;
        }
        let size = ui.available_size();
        let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, background);
        let Some(workspace) = self.workspace.as_mut() else {
            return;
        };
        workspace
            .viewer
            .resize(f64::from(rect.width()), f64::from(rect.height()));
        if self.fit_pending && rect.width() > 40.0 && rect.height() > 40.0 {
            workspace.viewer.fit();
            self.fit_pending = false;
        }
        let local = |p: Pos2| {
            let at = p - rect.min;
            (f64::from(at.x), f64::from(at.y))
        };
        if response.double_clicked() && !workspace.viewer.tool.edits() {
            workspace.viewer.fit();
        }
        if response.drag_started() {
            if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
                let (sx, sy) = local(p);
                self.tool_drag = workspace.press(sx, sy);
            }
        }
        if response.dragged() {
            if self.tool_drag {
                if let Some(p) = response.interact_pointer_pos() {
                    let (sx, sy) = local(p);
                    workspace.drag_to(sx, sy);
                }
            } else {
                let d = response.drag_delta();
                workspace.viewer.pan_px(f64::from(d.x), f64::from(d.y));
            }
        }
        if response.drag_stopped() && self.tool_drag {
            let end = response
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.latest_pos()));
            match end {
                Some(p) => {
                    let (sx, sy) = local(p);
                    workspace.release(sx, sy);
                }
                None => workspace.drag = None,
            }
            self.tool_drag = false;
        }
        match response.hover_pos() {
            Some(p) => {
                let (sx, sy) = local(p);
                workspace.hover(sx, sy);
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                if scroll.abs() > f32::EPSILON || (pinch - 1.0).abs() > 1e-6 {
                    workspace
                        .viewer
                        .zoom_at_px(sx, sy, f64::from(scroll), f64::from(pinch));
                }
            }
            None => workspace.leave(),
        }
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                let (sx, sy) = local(p);
                workspace.click(sx, sy);
            }
        }
        for action in &self.actions {
            if let Some(shortcut) = action.workspace_shortcut() {
                workspace.shortcut(shortcut);
            }
        }
        if workspace.want_save {
            workspace.want_save = false;
            self.focus_save = true;
        }
        self.engine.paint(workspace, &painter, rect);
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let note = self.workspace.as_ref().and_then(|w| w.note.as_ref());
                if let Some(error) = &self.error {
                    ui.label(egui::RichText::new(error.message(lang)).color(ui_danger()));
                } else if let Some(text) = self.win.error_text(lang) {
                    ui.label(egui::RichText::new(text).color(ui_danger()));
                } else if let Some(note) = note {
                    let colour = if note.is_error() {
                        ui_danger()
                    } else {
                        ui_accent()
                    };
                    ui.label(egui::RichText::new(note.text(lang)).color(colour));
                } else if let Some(w) = &self.workspace {
                    ui.label(egui::RichText::new(w.viewer.status(lang)).color(ui_ok()));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::Prototype.tr(lang)).color(ui_accent()));
                    ui.label(dim(Msg::ZoomHint.tr(lang)));
                });
            });
        });
    }
}

/// The findings of the rules of a board, what the last edit changed, the rule file and the nets.
fn board_panel(ui: &mut egui::Ui, workspace: &mut Workspace, rules_text: &mut String, lang: Lang) {
    ui.label(heading(EditMsg::RuleFindings.tr(lang)));
    ui.horizontal(|ui| {
        ui.label(dim(EditMsg::RulesPath.tr(lang)));
        ui.add(egui::TextEdit::singleline(rules_text).desired_width(150.0));
        if ui.button(EditMsg::LoadRules.tr(lang)).clicked() {
            let path = PathBuf::from(rules_text.trim());
            workspace.load_rules(&path);
        }
    });
    let shown = workspace.viewer.settings.findings_shown;
    let mut jump: Option<Option<[i64; 2]>> = None;
    if let Some(session) = workspace.board_session() {
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                EditMsg::Added.tr(lang),
                session.last_diff.added.len()
            ))
            .color(ui_danger()),
        );
        for (i, finding) in session.last_diff.added.iter().take(shown).enumerate() {
            ui.push_id(("added", i), |ui| {
                finding_row(ui, &finding_line(finding, lang), ui_danger(), lang, || {
                    jump = Some(finding.at_nm);
                });
            });
        }
        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                EditMsg::Removed.tr(lang),
                session.last_diff.removed.len()
            ))
            .color(ui_ok()),
        );
        for (i, finding) in session.last_diff.removed.iter().take(shown).enumerate() {
            ui.push_id(("removed", i), |ui| {
                finding_row(ui, &finding_line(finding, lang), ui_ok(), lang, || {
                    jump = Some(finding.at_nm);
                });
            });
        }
    }
    if let Some(place) = jump {
        workspace.jump_to_place(place);
    }
    // the problems of the rules with their filters, their order and the steps between them
    drc_gui::section(ui, workspace, lang);
}

/// The nets, each a button that lights it; the connections still to be routed are counted.
fn net_list(ui: &mut egui::Ui, workspace: &mut Workspace, lang: Lang) {
    let highlighted = workspace.viewer.highlight;
    let mut chosen: Option<u32> = None;
    for row in workspace.net_rows() {
        let text = if row.unrouted == 0 {
            row.name.clone()
        } else {
            format!(
                "{} ({} {})",
                row.name,
                row.unrouted,
                EditMsg::Unrouted.tr(lang)
            )
        };
        if ui
            .selectable_label(highlighted == Some(row.index), text)
            .clicked()
        {
            chosen = Some(row.index);
        }
    }
    if let Some(index) = chosen {
        workspace.viewer.select_net(index);
    }
}

/// One line of a list of findings with the button that goes to its place.
fn finding_row(ui: &mut egui::Ui, line: &str, colour: Color32, lang: Lang, mut go: impl FnMut()) {
    ui.horizontal_wrapped(|ui| {
        if ui.small_button(EditMsg::GoTo.tr(lang)).clicked() {
            go();
        }
        ui.add(egui::Label::new(egui::RichText::new(line).color(colour)).wrap());
    });
}

/// The electrical rule check of a schematic and its synchronisation with a board.
fn schematic_panel(
    ui: &mut egui::Ui,
    workspace: &mut Workspace,
    board_text: &mut String,
    board_save_text: &mut String,
    lang: Lang,
) {
    ui.label(heading(EditMsg::ErcPanel.tr(lang)));
    if ui.button(EditMsg::RunErc.tr(lang)).clicked() {
        workspace.run_erc();
    }
    let shown = workspace.viewer.settings.findings_shown;
    let mut jump: Option<String> = None;
    if let Some(session) = workspace.schematic_session() {
        let total = session.erc.confirmed.len();
        ui.label(dim(total.to_string()));
        if total == 0 {
            ui.label(dim(EditMsg::NoFindings.tr(lang)));
        }
        for (i, finding) in session.erc.confirmed.iter().take(shown).enumerate() {
            ui.push_id(("erc", i), |ui| {
                let colour = match finding.severity {
                    eh3d_phoenix_rules::Severity::Error => ui_danger(),
                    eh3d_phoenix_rules::Severity::Warning => ui_accent(),
                    eh3d_phoenix_rules::Severity::Info => ui_text(),
                };
                finding_row(ui, &erc_line(finding, lang), colour, lang, || {
                    jump = Some(finding.location.clone());
                });
            });
        }
        if !session.erc.inconclusive.is_empty() {
            ui.collapsing(format!("… {}", session.erc.inconclusive.len()), |ui| {
                for (i, finding) in session.erc.inconclusive.iter().take(shown).enumerate() {
                    ui.push_id(("unjudged", i), |ui| {
                        finding_row(ui, &erc_line(finding, lang), ui_dim(), lang, || {
                            jump = Some(finding.location.clone());
                        });
                    });
                }
            });
        }
    }
    if let Some(location) = jump {
        workspace.jump_to_location(&location);
    }
    ui.separator();
    ui.label(heading(EditMsg::SyncPanel.tr(lang)));
    ui.horizontal(|ui| {
        ui.label(dim(EditMsg::CompanionBoard.tr(lang)));
        ui.add(egui::TextEdit::singleline(board_text).desired_width(150.0));
        if ui.button(EditMsg::LoadBoard.tr(lang)).clicked() {
            let path = PathBuf::from(board_text.trim());
            if workspace.open_companion(&path) {
                *board_save_text = suggested_save_path(&path).to_string_lossy().into_owned();
            }
        }
    });
    let has_board = workspace
        .schematic_session()
        .is_some_and(|s| s.companion.is_some());
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                has_board,
                egui::Button::new(EditMsg::TransferToBoard.tr(lang)),
            )
            .clicked()
        {
            workspace.plan_sync(true);
        }
        if ui
            .add_enabled(has_board, egui::Button::new(EditMsg::BackAnnotate.tr(lang)))
            .clicked()
        {
            workspace.plan_sync(false);
        }
    });
    let preview = workspace
        .schematic_session()
        .and_then(|s| s.preview_text(lang).map(|t| (t, s.preview_has_changes())));
    if let Some((text, has_changes)) = preview {
        ui.label(heading(EditMsg::PreviewTitle.tr(lang)));
        if has_changes {
            ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap());
        } else {
            ui.label(dim(EditMsg::InSync.tr(lang)));
            ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap());
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    has_changes,
                    egui::Button::new(EditMsg::ApplyPackage.tr(lang)),
                )
                .clicked()
            {
                workspace.apply_package();
            }
            if ui.button(EditMsg::DiscardPackage.tr(lang)).clicked() {
                workspace.discard_package();
            }
        });
    }
    let board_changed = workspace
        .schematic_session()
        .and_then(|s| s.companion.as_ref())
        .is_some_and(|c| c.dirty());
    if board_changed {
        ui.horizontal(|ui| {
            ui.label(dim(EditMsg::SavePath.tr(lang)));
            ui.add(egui::TextEdit::singleline(board_save_text).desired_width(150.0));
            if ui.button(EditMsg::SaveAs.tr(lang)).clicked() {
                let target = PathBuf::from(board_save_text.trim());
                workspace.save_companion_as(&target);
            }
        });
    }
}

/// Draws every primitive of the visible layers that the window shows, in the order of the layers,
/// then the ratsnest, what is in progress, the mark of a finding and the measure over them.
fn paint(workspace: &Workspace, painter: &egui::Painter, rect: Rect) {
    let viewer = &workspace.viewer;
    let painter = painter.with_clip_rect(rect);
    let view = viewer.view;
    let scale = view.scale;
    let at = |p: P| -> Pos2 {
        let (x, y) = view.to_screen(p);
        Pos2::new(rect.min.x + x as f32, rect.min.y + y as f32)
    };
    let px = |mm: f64| (mm * scale) as f32;
    let settings = &viewer.settings;
    let highlight = rgba(settings.colour("highlight"));
    for index in viewer.visible(view.visible_area()) {
        let prim = &viewer.scene.prims[index as usize];
        let layer = &viewer.scene.layers[prim.layer as usize];
        let marked = (prim.net.is_some() && prim.net == viewer.highlight)
            || viewer.selected == Some(prim.item);
        let colour = if marked {
            highlight
        } else {
            rgba(layer.colour)
        };
        let stroke_width = px(prim.width).max(if marked { 2.0 } else { 1.0 });
        match &prim.shape {
            Shape::Line { a, b } => {
                let (a, b) = (at(*a), at(*b));
                painter.line_segment([a, b], Stroke::new(stroke_width, colour));
                if stroke_width > 2.0 {
                    painter.circle_filled(a, stroke_width / 2.0, colour);
                    painter.circle_filled(b, stroke_width / 2.0, colour);
                }
            }
            Shape::Circle { centre, radius } => {
                let r = px(*radius).max(0.6);
                if prim.filled {
                    painter.circle_filled(at(*centre), r, colour);
                } else {
                    painter.circle_stroke(at(*centre), r, Stroke::new(stroke_width, colour));
                }
            }
            Shape::Arc {
                centre,
                radius,
                start,
                sweep,
            } => {
                let points = arc_points(
                    *centre,
                    *radius,
                    *start,
                    *sweep,
                    settings.arc_max_error_px / scale,
                    settings.arc_min_segments,
                    settings.arc_max_segments,
                );
                let line: Vec<Pos2> = points.iter().map(|p| at(*p)).collect();
                painter.add(egui::Shape::line(line, Stroke::new(stroke_width, colour)));
            }
            Shape::Polygon { points } => {
                let line: Vec<Pos2> = points.iter().map(|p| at(*p)).collect();
                let triangles = if prim.filled && points.len() <= settings.max_fill_points {
                    triangulate(points)
                } else {
                    Vec::new()
                };
                if triangles.is_empty() {
                    painter.add(egui::Shape::closed_line(
                        line,
                        Stroke::new(stroke_width, colour),
                    ));
                } else {
                    let mut mesh = egui::epaint::Mesh::default();
                    for pos in &line {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: *pos,
                            uv: egui::epaint::WHITE_UV,
                            color: colour,
                        });
                    }
                    for t in &triangles {
                        mesh.add_triangle(t[0], t[1], t[2]);
                    }
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
            Shape::Text {
                at: origin,
                height,
                text,
            } => {
                let size = px(*height);
                if f64::from(size) >= settings.text_min_px {
                    painter.text(
                        at(*origin),
                        egui::Align2::LEFT_BOTTOM,
                        text,
                        egui::FontId::proportional((size * 1.3).min(200.0)),
                        colour,
                    );
                }
            }
        }
    }
    // the connections still to be routed, as thin lines; those of the highlighted net stand out
    if workspace.show_ratsnest {
        if let Some(session) = workspace.board_session() {
            let thin = rgba(settings.colour("ratsnest"));
            let width = settings.ratsnest_width_px as f32;
            for segment in &session.rat_segments {
                let lit = viewer.highlight == Some(segment.net);
                let (colour, w) = if lit {
                    (highlight, width * 2.0)
                } else {
                    (thin, width)
                };
                painter.line_segment([at(segment.a), at(segment.b)], Stroke::new(w, colour));
            }
        }
    }
    // what an edit in progress would do
    let measure_colour = rgba(settings.colour("measure"));
    for (a, b) in workspace.preview_lines() {
        painter.line_segment([at(a), at(b)], Stroke::new(1.5, measure_colour));
    }
    if let Some(p) = workspace.marker {
        painter.circle_stroke(
            at(p),
            settings.marker_radius_px as f32,
            Stroke::new(2.0, highlight),
        );
    }
    // the measures, over everything
    window_gui::paint_measures(&painter, viewer, &at, measure_colour);
}

impl App {
    /// Which mode of the window is shown, for the help and for the preferences: the one of the panel
    /// that has the focus when it is a mode of its own that is open, and else the one of the document.
    fn help_mode(&self) -> HelpMode {
        let open = |panel: &str| self.focused.as_deref() == Some(panel);
        if open(panels::id::LIBRARY_EDITOR) && self.lib.is_open() {
            HelpMode::Library
        } else if open(panels::id::GERBER) && self.gerber.is_some() {
            HelpMode::Gerber
        } else if open(panels::id::VIEW3D) && self.view3d.is_some() {
            HelpMode::View3d
        } else if open(panels::id::BOARDS) && self.panel.is_some() {
            HelpMode::Panel
        } else {
            match &self.workspace {
                Some(w) if w.is_board() => HelpMode::Board,
                Some(_) => HelpMode::Schematic,
                None => HelpMode::Start,
            }
        }
    }

    /// The search, the measures, the layers, the history, the appearance, the shortcuts and the help,
    /// and the preferences, after the rest of the window is drawn.
    fn common(&mut self, ctx: &egui::Context) {
        self.shot_tick(ctx);
        let mode = self.help_mode();
        let actions = std::mem::take(&mut self.actions);
        let frame = FrameContext {
            lang: self.lang,
            mode,
            fit_pending: self.fit_pending,
            editing: self.sch.is_some() || self.brd.is_some(),
            workspace: self.workspace.as_mut(),
            view3d: self.view3d.as_mut(),
        };
        self.win.frame(ctx, &mut self.settings, &actions, frame);
    }

    /// A file dropped on the window is opened.
    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if dropped.is_empty() {
            return;
        }
        match Launcher::dropped_launch(&dropped) {
            Ok(asked) => {
                self.launch_note = None;
                self.launch(asked);
            }
            Err(note) => self.launch_note = Some(note),
        }
    }

    /// What the window knows when it decides which entries of the menus can be chosen.
    fn menu_facts(&self) -> MenuFacts {
        MenuFacts {
            has_document: self.workspace.is_some(),
            is_board: self.workspace.as_ref().is_some_and(|w| w.is_board()),
            editing: self.sch.is_some() || self.brd.is_some(),
            can_undo: self.workspace.as_ref().is_some_and(|w| w.can_undo()),
            can_redo: self.workspace.as_ref().is_some_and(|w| w.can_redo()),
            actions: self.brd.as_ref().map_or(0, board_gui::actions_mask),
        }
    }

    /// Writes to the console what the window says in its status bar when it changes: the error of a file,
    /// the error of the preferences and the note of the document.
    fn feed_console(&mut self) {
        let lang = self.lang;
        let error = self
            .error
            .as_ref()
            .map(|e| e.message(lang))
            .unwrap_or_default();
        self.console.feed(0, Level::Error, &error);
        let prefs = self.win.error_text(lang).unwrap_or_default();
        self.console.feed(1, Level::Error, &prefs);
        let (level, note) = match self.workspace.as_ref().and_then(|w| w.note.as_ref()) {
            Some(note) if note.is_error() => (Level::Error, note.text(lang)),
            Some(note) => (Level::Info, note.text(lang)),
            None => (Level::Info, String::new()),
        };
        self.console.feed(2, level, &note);
    }

    /// Does what a menu entry asks for.
    fn run_cmd(&mut self, cmd: Cmd, ctx: &egui::Context) {
        match cmd {
            Cmd::Std(action) => self.run_std(action, ctx),
            Cmd::Fit => self.fit_pending = true,
            Cmd::ClearSelection => {
                if let Some(w) = self.workspace.as_mut() {
                    w.cancel();
                }
            }
            Cmd::RunErc => {
                if let Some(w) = self.workspace.as_mut() {
                    w.run_erc();
                    self.pending_dock
                        .push(DockAction::ShowPanel(panels::id::DRC.to_string()));
                }
            }
            Cmd::View3d => self.open_3d(),
            Cmd::SettingsFiles => {
                if self.rules.take().is_none() {
                    self.rules = Some(CfgEditor::new(&self.settings));
                }
            }
            Cmd::Search => self.win.show_window(ShotWindow::Search),
            Cmd::Measures => self.win.show_window(ShotWindow::Measures),
            Cmd::LayerPresets => self.win.show_window(ShotWindow::Layers),
            Cmd::History => self.win.show_window(ShotWindow::History),
            Cmd::Shortcuts => self.win.show_window(ShotWindow::Shortcuts),
            Cmd::BoardAction(action) => {
                if let Some(brd) = self.brd.as_mut() {
                    board_gui::run_action(brd, action);
                }
            }
        }
    }

    /// Does what an entry that every program has asks for.
    fn run_std(&mut self, action: StdAction, ctx: &egui::Context) {
        let show_tasks = DockAction::ShowPanel(panels::id::TASKS.to_string());
        match action {
            StdAction::New => self.launch(Launch::NewLibrary),
            StdAction::Open => {
                self.pending_dock.push(show_tasks);
                self.focus_path = true;
            }
            StdAction::Save | StdAction::SaveAs => {
                if let (Some(w), Some(key)) =
                    (self.workspace.as_mut(), Action::Save.workspace_shortcut())
                {
                    w.shortcut(key);
                }
                self.pending_dock.push(show_tasks);
            }
            StdAction::Export => self.pending_dock.push(show_tasks),
            StdAction::Undo => {
                if let Some(w) = self.workspace.as_mut() {
                    w.undo();
                }
            }
            StdAction::Redo => {
                if let Some(w) = self.workspace.as_mut() {
                    w.redo();
                }
            }
            StdAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            StdAction::Preferences => self.win.show_window(ShotWindow::Appearance),
            StdAction::OnlineHelp => self.win.show_window(ShotWindow::Help),
            StdAction::Cut | StdAction::Copy | StdAction::Paste | StdAction::CheckUpdates => {}
        }
    }

    /// The line of a panel of a mode of its own while nothing of its kind is open.
    fn mode_empty(ui: &mut egui::Ui, lang: Lang) {
        ui.add_space(6.0);
        ui.add(egui::Label::new(dim(DockMsg::ModePanelEmpty.tr(lang))).wrap());
    }

    /// The editor of a library of footprints, symbols and devices.
    fn panel_library_editor(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if !self.lib.is_open() {
            App::mode_empty(ui, lang);
            return;
        }
        let mut close = false;
        egui::Panel::top("library_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(heading(LibText::Title.tr(lang)));
                close = ui.button(PanelText::Close.tr(lang)).clicked();
            });
        });
        self.lib.show(ui, lang);
        if close {
            self.lib.close();
            self.hide_panel(panels::id::LIBRARY_EDITOR);
        }
    }

    /// The imported Gerber and Excellon files of a folder.
    fn panel_gerber(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Some(window) = self.gerber.as_mut() else {
            App::mode_empty(ui, lang);
            return;
        };
        if window.show(ui, lang, &self.settings) {
            self.gerber = None;
            self.hide_panel(panels::id::GERBER);
        }
    }

    /// The 3D view of the assembled board.
    fn panel_view3d(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Some(window) = self.view3d.as_mut() else {
            App::mode_empty(ui, lang);
            return;
        };
        if window.show(ui, lang, &self.settings) {
            self.view3d = None;
            self.hide_panel(panels::id::VIEW3D);
        }
    }

    /// The panel of boards laid out for manufacturing.
    fn panel_boards(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Some(window) = self.panel.as_mut() else {
            App::mode_empty(ui, lang);
            return;
        };
        if window.show(ui, lang, &self.settings) {
            self.panel = None;
            self.hide_panel(panels::id::BOARDS);
        }
    }
}

/// How a panel of the window is drawn: with the program and the room the frame gives it.
type PanelDraw = fn(&mut App, &mut egui::Ui);

/// The window: the frame of the suite with the panels of the program in it and the menu bar above.
struct Shell {
    app: Rc<RefCell<App>>,
    dock: Dock,
    bar: MenuBar<Cmd>,
    watch: LayoutWatch,
    /// The frames that are left before the arrangement of the panels is looked at again.
    countdown: usize,
}

impl Shell {
    fn new(app: App) -> Shell {
        let lang = app.lang;
        let dock_text = panels::dock_text(app.win.dock_pairs());
        let app = Rc::new(RefCell::new(app));
        let mut dock = Dock::new(panels::defs(), panels::workspaces());
        let draw: [(&str, PanelDraw); 14] = [
            (panels::id::BOARD_VIEW, App::panel_board_view),
            (panels::id::LAYERS, App::panel_layers),
            (panels::id::PROPERTIES, App::panel_properties),
            (panels::id::COMPONENTS, App::panel_components),
            (panels::id::NETS, App::panel_nets),
            (panels::id::DRC, App::panel_drc),
            (panels::id::LIBRARY, App::panel_library),
            (panels::id::CONSOLE, App::panel_console),
            (panels::id::REPORT, App::panel_report),
            (panels::id::TASKS, App::panel_tasks),
            (panels::id::LIBRARY_EDITOR, App::panel_library_editor),
            (panels::id::GERBER, App::panel_gerber),
            (panels::id::VIEW3D, App::panel_view3d),
            (panels::id::BOARDS, App::panel_boards),
        ];
        for (name, f) in draw {
            let a = app.clone();
            dock.register(name, move |ui| f(&mut a.borrow_mut(), ui));
        }
        dock.load_config(&dock_text);
        // a picture of a workspace: the arrangement of that workspace, which is not written anywhere
        let asked = app.borrow().shot.as_ref().and_then(|s| s.request.workspace);
        if let Some(name) = asked {
            let _ = dock.apply(DockAction::SwitchWorkspace(name.to_string()));
        }
        dock.set_language(panels::view_lang(lang));
        let watch = LayoutWatch::new(&dock.core().config_pairs());
        Shell {
            app,
            dock,
            bar: panels::menu_bar(),
            watch,
            countdown: 1,
        }
    }

    /// What the program says about itself in the window «About».
    fn about(lang: Lang) -> AboutInfo {
        AboutInfo {
            app_name: "EH3D-PHOENIX-PCB".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            license: "GPL-3.0-or-later".to_string(),
            extra: vec![Msg::Prototype.tr(lang).to_string()],
        }
    }

    /// Does what was chosen in the menus and what the menus asked of the frame, and keeps the list of recent files.
    fn after(&mut self, chosen: Option<Chosen<Cmd>>, ctx: &egui::Context) {
        let mut app = self.app.borrow_mut();
        match chosen {
            Some(Chosen::App(cmd)) => app.run_cmd(cmd, ctx),
            Some(Chosen::Recent(path)) => {
                app.path_text = path.clone();
                app.open(Path::new(&path));
            }
            Some(Chosen::Dock(_)) | None => {}
        }
        for action in std::mem::take(&mut app.pending_dock) {
            let _ = self.dock.apply(action);
        }
        for path in std::mem::take(&mut app.recent_added) {
            self.dock.core_mut().recent_mut().push(&path);
        }
    }

    /// Keeps the arrangement of the panels in the preferences when it changed; it looks at it a few times
    /// a second and not at every frame.
    fn keep_layout(&mut self) {
        let mut app = self.app.borrow_mut();
        if app.shot.is_some() {
            return;
        }
        self.countdown = self.countdown.saturating_sub(1);
        if self.countdown > 0 {
            return;
        }
        self.countdown = app.settings.dock_check_every_frames.max(1);
        let pairs = self.dock.core().config_pairs();
        if self.watch.changed(&pairs) {
            app.win.keep_dock(&pairs);
        }
    }
}

impl Shell {
    /// Draws one frame of the window in the space of `ui`.
    fn run(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let (lang, facts) = {
            let mut app = self.app.borrow_mut();
            app.focused = self.dock.core().focus().map(str::to_owned);
            app.actions = window_gui::pressed_actions(&ctx, &app.settings.keymap);
            let frame_ms = f64::from(ctx.input(|i| i.unstable_dt)) * 1000.0;
            app.frames.push(frame_ms);
            app.handle_drops(&ctx);
            app.feed_console();
            (app.lang, app.menu_facts())
        };
        self.dock.set_language(panels::view_lang(lang));
        self.dock.set_about(Shell::about(lang));
        self.dock
            .set_menu_state(panels::disabled_entries(&facts), Vec::new());
        let mut chosen = None;
        egui::Panel::top("menu").show(ui, |ui| {
            chosen = self.dock.menu_bar(ui, &self.bar);
        });
        self.app.borrow_mut().status_bar(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.dock.show(ui));
        self.after(chosen, &ctx);
        self.keep_layout();
        let mut app = self.app.borrow_mut();
        app.common(&ctx);
        let lang = app.lang;
        if let Some(editor) = app.rules.as_mut() {
            if !cfg_gui::window(&ctx, editor, lang) {
                app.rules = None;
            }
        }
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.run(ui);
    }
}

/// The icon of the program for the window, from the file of the brand.
fn window_icon() -> Option<egui::IconData> {
    let bytes: &[u8] = include_bytes!("../../../images/brand/icon-256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

/// The fonts of the system that draw Japanese and Chinese are added to the ones of egui, which
/// have no such letters.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".into(), Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// Opens the window. A file given here is opened at once.
pub fn run(
    initial: Option<PathBuf>,
    lang: Lang,
    settings: ViewerSettings,
) -> Result<(), ViewError> {
    run_with(initial, None, None, lang, settings)
}

/// Opens the window only to photograph it: the file `initial` is opened, the window asked for is
/// shown, the picture is written as the request says and the window closes by itself. No file of
/// preferences and no list of recent files is read or written. The answer is an error when the
/// picture could not be written.
pub fn run_shot(
    initial: Option<PathBuf>,
    lang: Lang,
    settings: ViewerSettings,
    request: ShotRequest,
) -> Result<(), ViewError> {
    let failed = |what: String| ViewError::new(WINDOW_FAILED, what);
    if !request.replace && request.output.exists() {
        return Err(failed(format!(
            "{} exists; give --replace to write over it",
            request.output.display()
        )));
    }
    let folder = request
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf);
    if folder.is_some_and(|dir| !dir.is_dir()) {
        return Err(failed(format!(
            "the folder of {} does not exist",
            request.output.display()
        )));
    }
    run_with(initial, None, Some(request), lang, settings)
}

/// Opens the window with a panel of boards laid out: the board files and, when given, the settings
/// file of the panel.
pub fn run_panel(
    files: Vec<PathBuf>,
    config: Option<PathBuf>,
    lang: Lang,
    settings: ViewerSettings,
) -> Result<(), ViewError> {
    run_with(None, Some((files, config)), None, lang, settings)
}

fn run_with(
    initial: Option<PathBuf>,
    panel: Option<(Vec<PathBuf>, Option<PathBuf>)>,
    shot: Option<ShotRequest>,
    lang: Lang,
    settings: ViewerSettings,
) -> Result<(), ViewError> {
    let mut settings = settings;
    // in the picture mode the preferences of the person are neither read nor written
    let win = if shot.is_some() {
        WindowUi::ephemeral(&settings)
    } else {
        WindowUi::load(&settings)
    };
    win.adopt(&mut settings);
    let mut viewport = egui::ViewportBuilder::default().with_title("EH3D-PHOENIX-PCB");
    viewport = match &shot {
        Some(request) => {
            if let Some(theme) = request.theme {
                settings.set_theme(theme);
            }
            viewport.with_inner_size([request.size.0 as f32, request.size.1 as f32])
        }
        None => viewport
            .with_inner_size(win.window_size().unwrap_or([1480.0, 800.0]))
            .with_min_inner_size([900.0, 480.0]),
    };
    let outcome: ShotOutcome = Arc::new(Mutex::new(None));
    let outcome_for_app = outcome.clone();
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let taking_picture = shot.is_some();
    eframe::run_native(
        "EH3D-PHOENIX-PCB",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            // the window draws its own colours for every theme, so the toolkit keeps one
            cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
            window_gui::apply_look(&cc.egui_ctx, &settings);
            // the windows that open a folder or a panel of boards do not open the file as a board
            let opens_as_board = !matches!(
                shot.as_ref().map(|r| r.window),
                Some(ShotWindow::Gerber | ShotWindow::Panel)
            );
            let (first, window_file) = if opens_as_board {
                (initial, None)
            } else {
                (None, initial)
            };
            let mut app = App::new(lang, settings, win, first, !taking_picture);
            if let Some((files, config)) = panel {
                app.panel = Some(PanelUi::new(&files, config.as_deref(), &app.settings));
                app.show_panel(panels::id::BOARDS);
            }
            if let Some(request) = shot {
                app.begin_shot(request, window_file.as_deref(), outcome_for_app);
            }
            Ok(Box::new(Shell::new(app)))
        }),
    )
    .map_err(|e| ViewError::new(WINDOW_FAILED, e.to_string()))?;
    if !taking_picture {
        return Ok(());
    }
    let taken = outcome
        .lock()
        .map_err(|e| ViewError::new(WINDOW_FAILED, e.to_string()))?
        .take();
    match taken {
        Some(Ok(())) => Ok(()),
        Some(Err(reason)) => Err(ViewError::new(WINDOW_FAILED, reason)),
        None => Err(ViewError::new(
            WINDOW_FAILED,
            "the window closed before the picture was taken".to_owned(),
        )),
    }
}

/// Writes the picture the window gave back as a PNG file, a new file unless `replace` is true.
fn save_shot_png(image: &egui::ColorImage, path: &Path, replace: bool) -> Result<(), String> {
    let mut open = std::fs::OpenOptions::new();
    open.write(true);
    if replace {
        open.create(true).truncate(true);
    } else {
        open.create_new(true);
    }
    let file = open
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        image.size[0] as u32,
        image.size[1] as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let data: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    writer.write_image_data(&data).map_err(|e| e.to_string())?;
    // the end of the file is written here, so that a failure is reported and not lost in a drop
    writer.finish().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{Document, Loaded};
    use crate::sample;

    /// A window with nothing open, as a picture opens it (no file of preferences is read or written).
    fn app() -> App {
        let settings = ViewerSettings::default();
        let win = WindowUi::ephemeral(&settings);
        App::new(Lang::En, settings, win, None, false)
    }

    fn with_board(app: &mut App) {
        let loaded = Loaded {
            document: Document::Board(Box::new(sample::board())),
            unread: 0,
        };
        app.workspace = Workspace::new(
            loaded,
            None,
            ViewerSettings::default(),
            RouteSettings::default(),
        )
        .ok();
        assert!(app.workspace.is_some(), "the sample board does not open");
        app.fit_pending = true;
    }

    fn with_schematic(app: &mut App) {
        let loaded = Loaded {
            document: Document::Schematic(Box::new(sample::schematic())),
            unread: 0,
        };
        app.workspace = Workspace::new(
            loaded,
            None,
            ViewerSettings::default(),
            RouteSettings::default(),
        )
        .ok();
        assert!(
            app.workspace.is_some(),
            "the sample schematic does not open"
        );
        app.fit_pending = true;
    }

    /// Frames of the window drawn in a context with no screen.
    struct Harness {
        ctx: egui::Context,
        shell: Shell,
        t: f64,
    }

    impl Harness {
        fn new(app: App) -> Harness {
            Harness {
                ctx: egui::Context::default(),
                shell: Shell::new(app),
                t: 0.0,
            }
        }

        fn frames(&mut self, n: usize) {
            for _ in 0..n {
                self.t += 0.05;
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1400.0, 900.0))),
                    time: Some(self.t),
                    ..Default::default()
                };
                let shell = &mut self.shell;
                let out = self.ctx.run_ui(input, |ui| shell.run(ui));
                out.drop_without_applying_deltas();
            }
        }

        fn switch(&mut self, workspace: &str) {
            self.shell
                .dock
                .apply(DockAction::SwitchWorkspace(workspace.to_string()))
                .unwrap();
        }
    }

    #[test]
    fn every_workspace_draws_with_nothing_open() {
        let mut h = Harness::new(app());
        for name in [panels::WS_EDIT, panels::WS_REVIEW, panels::WS_COMPACT] {
            h.switch(name);
            h.frames(4);
            assert_eq!(h.shell.dock.core().workspaces().current_name(), name);
        }
    }

    #[test]
    fn every_workspace_draws_a_board_and_a_schematic() {
        for open in [with_board as fn(&mut App), with_schematic] {
            let mut a = app();
            open(&mut a);
            let mut h = Harness::new(a);
            for name in [panels::WS_EDIT, panels::WS_REVIEW, panels::WS_COMPACT] {
                h.switch(name);
                h.frames(4);
            }
            // the frames moved the pointer of nobody and changed no document
            assert!(h.shell.app.borrow().workspace.is_some());
        }
    }

    #[test]
    fn a_workspace_shows_the_panels_it_was_given() {
        let mut h = Harness::new(app());
        h.switch(panels::WS_EDIT);
        h.frames(2);
        let core = h.shell.dock.core();
        for p in [
            panels::id::BOARD_VIEW,
            panels::id::PROPERTIES,
            panels::id::LAYERS,
            panels::id::COMPONENTS,
            panels::id::CONSOLE,
        ] {
            assert!(core.layout().is_shown(p), "{p}");
        }
        h.switch(panels::WS_REVIEW);
        assert!(!h.shell.dock.core().layout().is_shown(panels::id::TASKS));
    }

    #[test]
    fn the_open_entry_shows_the_tasks_panel_and_asks_for_the_box_of_the_path() {
        let mut h = Harness::new(app());
        h.switch(panels::WS_REVIEW);
        h.frames(1);
        assert!(!h.shell.dock.core().layout().is_shown(panels::id::TASKS));
        let ctx = h.ctx.clone();
        h.shell
            .after(Some(Chosen::App(Cmd::Std(StdAction::Open))), &ctx);
        assert!(h.shell.dock.core().layout().is_shown(panels::id::TASKS));
        assert!(h.shell.app.borrow().focus_path);
        h.frames(2);
        assert!(!h.shell.app.borrow().focus_path);
    }

    #[test]
    fn a_file_that_is_opened_reaches_the_list_of_recent_files_of_the_menu() {
        let mut h = Harness::new(app());
        h.shell
            .app
            .borrow_mut()
            .recent_added
            .push("a.brd".to_string());
        let ctx = h.ctx.clone();
        h.shell.after(None, &ctx);
        assert_eq!(h.shell.dock.core().recent().items().len(), 1);
        assert!(h.shell.app.borrow().recent_added.is_empty());
    }

    #[test]
    fn the_arrangement_goes_to_the_preferences_and_comes_back_in_the_next_window() {
        let mut a = app();
        a.settings.dock_check_every_frames = 1;
        let mut h = Harness::new(a);
        h.frames(2);
        h.shell
            .dock
            .apply(DockAction::HidePanel(panels::id::LAYERS.to_string()))
            .unwrap();
        h.frames(3);
        let pairs = h.shell.app.borrow().win.dock_pairs().to_vec();
        assert!(!pairs.is_empty());
        assert!(pairs.iter().all(|(k, _)| k.starts_with("dock.")));
        // the next window reads them
        let mut b = app();
        b.win.keep_dock(&pairs);
        let h2 = Harness::new(b);
        assert!(!h2.shell.dock.core().layout().is_shown(panels::id::LAYERS));
        assert!(h2
            .shell
            .dock
            .core()
            .layout()
            .is_shown(panels::id::BOARD_VIEW));
    }

    #[test]
    fn a_quiet_window_writes_the_arrangement_once_and_never_again() {
        let mut a = app();
        a.settings.dock_check_every_frames = 1;
        let mut h = Harness::new(a);
        h.frames(5);
        // the arrangement is the factory one that was read: nothing to write
        assert!(h.shell.app.borrow().win.dock_pairs().is_empty());
    }

    #[test]
    fn the_errors_and_the_notes_reach_the_console_once() {
        let mut a = app();
        a.error = Some(ViewError::new(FILE_UNUSABLE, "x.brd".to_owned()));
        let mut h = Harness::new(a);
        h.frames(10);
        let lines = h.shell.app.borrow().console.len();
        assert_eq!(lines, 1);
        h.frames(10);
        assert_eq!(h.shell.app.borrow().console.len(), 1);
    }

    #[test]
    fn the_painter_draws_when_another_engine_is_asked_for() {
        let (engine, fell_back) = engine_for(Engine::Wgpu);
        assert!(fell_back);
        assert_eq!(engine.engine(), Engine::Painter);
        let (engine, fell_back) = engine_for(Engine::Painter);
        assert!(!fell_back);
        assert_eq!(engine.engine(), Engine::Painter);
        // a window that asked for wgpu says it in the console from the first frame
        let settings = ViewerSettings {
            canvas_engine: Engine::Wgpu,
            ..ViewerSettings::default()
        };
        let win = WindowUi::ephemeral(&settings);
        let a = App::new(Lang::En, settings, win, None, false);
        assert_eq!(a.console.len(), 1);
    }

    #[test]
    fn the_frame_times_are_kept_for_the_report() {
        let mut h = Harness::new(app());
        h.frames(6);
        assert_eq!(h.shell.app.borrow().frames.len(), 6);
    }

    impl Harness {
        fn shown(&self, panel: &str) -> bool {
            self.shell.dock.core().layout().is_shown(panel)
        }
    }

    fn gerber_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX0Y0D02*\nX100000Y0D01*\nM02*\n";
        std::fs::write(dir.path().join("board.gtl"), text).unwrap();
        dir
    }

    #[test]
    fn a_library_opens_in_a_panel_of_its_own_and_the_board_stays_where_it_was() {
        let mut a = app();
        with_board(&mut a);
        a.launch(Launch::NewLibrary);
        let mut h = Harness::new(a);
        h.frames(4);
        assert!(h.shown(panels::id::LIBRARY_EDITOR));
        assert!(h.shown(panels::id::BOARD_VIEW));
        let app = h.shell.app.borrow();
        assert!(app.lib.is_open());
        assert!(
            app.workspace.is_some(),
            "the board was closed by the library"
        );
        // the entries of the document are still on while the library is open
        assert!(!panels::disabled_entries(&app.menu_facts()).contains(&"file.export".to_string()));
    }

    #[test]
    fn the_3d_view_opens_in_its_panel_beside_the_board() {
        let mut a = app();
        with_board(&mut a);
        a.open_3d();
        assert!(
            a.view3d.is_some(),
            "{:?}",
            a.error.as_ref().map(|e| e.message(Lang::En))
        );
        let mut h = Harness::new(a);
        h.frames(4);
        assert!(h.shown(panels::id::VIEW3D));
        assert!(h.shown(panels::id::BOARD_VIEW));
        assert!(h.shell.app.borrow().workspace.is_some());
    }

    #[test]
    fn the_panel_of_boards_and_the_gerber_files_open_in_their_panels() {
        let folder = gerber_folder();
        let mut a = app();
        a.launch(Launch::OpenPanel(String::new()));
        a.launch(Launch::OpenGerber(folder.path().to_path_buf()));
        assert!(
            a.gerber.is_some(),
            "{:?}",
            a.error.as_ref().map(|e| e.message(Lang::En))
        );
        let mut h = Harness::new(a);
        h.frames(4);
        assert!(h.shown(panels::id::BOARDS));
        assert!(h.shown(panels::id::GERBER));
    }

    #[test]
    fn every_workspace_draws_with_all_the_modes_open() {
        let folder = gerber_folder();
        let mut a = app();
        with_board(&mut a);
        a.launch(Launch::NewLibrary);
        a.launch(Launch::OpenPanel(String::new()));
        a.launch(Launch::OpenGerber(folder.path().to_path_buf()));
        a.open_3d();
        let mut h = Harness::new(a);
        h.frames(2);
        for name in [panels::WS_EDIT, panels::WS_REVIEW, panels::WS_COMPACT] {
            h.switch(name);
            // each workspace has its own arrangement: the modes that are open are shown from the
            // View menu in the one the person changed to
            for panel in panels::id::MODES {
                h.shell
                    .dock
                    .apply(DockAction::ShowPanel(panel.to_string()))
                    .unwrap();
            }
            h.frames(3);
            for panel in panels::id::MODES {
                assert!(h.shown(panel), "{name}: {panel}");
            }
            assert!(h.shell.dock.core().layout_is_sound(), "{name}");
        }
        let app = h.shell.app.borrow();
        assert!(app.lib.is_open() && app.gerber.is_some());
        assert!(app.view3d.is_some() && app.panel.is_some());
    }

    #[test]
    fn a_panel_of_a_mode_with_nothing_open_says_so_and_draws() {
        let mut h = Harness::new(app());
        for panel in panels::id::MODES {
            h.shell
                .dock
                .apply(DockAction::ShowPanel(panel.to_string()))
                .unwrap();
        }
        h.frames(4);
        for panel in panels::id::MODES {
            assert!(h.shown(panel), "{panel}");
        }
        let app = h.shell.app.borrow();
        assert!(!app.lib.is_open() && app.gerber.is_none());
        assert!(app.view3d.is_none() && app.panel.is_none());
    }

    #[test]
    fn closing_a_mode_hides_its_panel_and_opening_it_again_shows_it() {
        let mut a = app();
        with_board(&mut a);
        a.launch(Launch::NewLibrary);
        let mut h = Harness::new(a);
        h.frames(3);
        assert!(h.shown(panels::id::LIBRARY_EDITOR));
        {
            let mut app = h.shell.app.borrow_mut();
            app.lib.close();
            app.hide_panel(panels::id::LIBRARY_EDITOR);
        }
        h.frames(3);
        assert!(!h.shown(panels::id::LIBRARY_EDITOR));
        h.shell.app.borrow_mut().launch(Launch::NewLibrary);
        h.frames(3);
        assert!(h.shown(panels::id::LIBRARY_EDITOR));
        assert!(h.shown(panels::id::BOARD_VIEW));
    }

    #[test]
    fn the_help_follows_the_panel_that_has_the_focus() {
        let mut a = app();
        with_board(&mut a);
        a.open_3d();
        let mut h = Harness::new(a);
        h.frames(3);
        assert_eq!(h.shell.app.borrow().help_mode(), HelpMode::View3d);
        h.shell
            .dock
            .apply(DockAction::ShowPanel(panels::id::BOARD_VIEW.to_string()))
            .unwrap();
        h.frames(2);
        assert_eq!(h.shell.app.borrow().help_mode(), HelpMode::Board);
    }

    #[test]
    fn a_picture_of_a_workspace_shows_it_and_writes_nothing() {
        for (asked, name) in [
            ("edit", panels::WS_EDIT),
            ("review", panels::WS_REVIEW),
            ("compact", panels::WS_COMPACT),
        ] {
            let mut a = app();
            with_board(&mut a);
            a.settings.dock_check_every_frames = 1;
            let request = ShotRequest {
                output: PathBuf::from("picture.png"),
                replace: true,
                size: (800, 600),
                theme: None,
                wait: 2,
                window: ShotWindow::Document,
                workspace: panels::workspace_named(asked),
            };
            a.begin_shot(request, None, Arc::new(Mutex::new(None)));
            let mut h = Harness::new(a);
            h.frames(8);
            assert_eq!(h.shell.dock.core().workspaces().current_name(), name);
            assert!(
                h.shell.app.borrow().win.dock_pairs().is_empty(),
                "{name}: the picture wrote the arrangement"
            );
        }
    }

    #[test]
    fn a_picture_without_a_workspace_keeps_the_factory_one() {
        let mut a = app();
        with_board(&mut a);
        let request = ShotRequest {
            output: PathBuf::from("picture.png"),
            replace: true,
            size: (800, 600),
            theme: None,
            wait: 2,
            window: ShotWindow::Document,
            workspace: None,
        };
        a.begin_shot(request, None, Arc::new(Mutex::new(None)));
        let h = Harness::new(a);
        assert_eq!(
            h.shell.dock.core().workspaces().current_name(),
            panels::WS_EDIT
        );
    }
}
