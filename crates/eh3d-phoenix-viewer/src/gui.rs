// SPDX-License-Identifier: GPL-3.0-or-later
//! The window (built with the `gui` feature): it draws the state of [`Workspace`] with egui on
//! eframe and passes on what the person does. Everything that can be decided without a screen is
//! decided in the workspace, in code that is tested. This file has not been seen on a screen.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke};
use eh3d_phoenix_route::{Mode, RouteSettings};
use eh3d_phoenix_rules::Lang;

use crate::board_gui::{self, BoardEditUi};
use crate::board_text::BoardMsg;
use crate::codes::{ViewError, FILE_UNUSABLE, WINDOW_FAILED};
use crate::editmsg::EditMsg;
use crate::launcher_gui;
use crate::launcher_session::{Launch, Launcher};
use crate::launcher_text::LauncherText;
use crate::library_gui::LibraryUi;
use crate::library_text::LibText;
use crate::msg::Msg;
use crate::prim::{arc_points, triangulate, Shape, P};
use crate::scene::SceneKind;
use crate::schematic_gui::{self, SchematicEditUi};
use crate::schematic_text::SchMsg;
use crate::settings::ViewerSettings;
use crate::viewer::Tool;
use crate::workspace::{erc_line, finding_line, suggested_save_path, Key, Shortcut, Workspace};

// the colours of the theme Forge, the one the programs of the suite start with: amber on charcoal
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
pub(crate) const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
pub(crate) const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);

/// The keys of the toolkit that the editing answers to and what they mean. The plus sign is the
/// key of the equals sign on many keyboards, so both are taken.
const KEYS: [(egui::Key, Key); 19] = [
    (egui::Key::Z, Key::Z),
    (egui::Key::Y, Key::Y),
    (egui::Key::S, Key::S),
    (egui::Key::R, Key::R),
    (egui::Key::V, Key::V),
    (egui::Key::M, Key::M),
    (egui::Key::G, Key::G),
    (egui::Key::O, Key::O),
    (egui::Key::D, Key::D),
    (egui::Key::E, Key::E),
    (egui::Key::W, Key::W),
    (egui::Key::I, Key::I),
    (egui::Key::T, Key::T),
    (egui::Key::F, Key::F),
    (egui::Key::Delete, Key::Delete),
    (egui::Key::Escape, Key::Escape),
    (egui::Key::Plus, Key::Plus),
    (egui::Key::Equals, Key::Plus),
    (egui::Key::Minus, Key::Minus),
];

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
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
    egui::RichText::new(text).strong().color(ACCENT)
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(DIM)
}

/// The keys pressed in this frame, unless a text box has the keyboard.
fn shortcuts(ctx: &egui::Context) -> Vec<Shortcut> {
    if ctx.egui_wants_keyboard_input() {
        return Vec::new();
    }
    ctx.input(|i| {
        KEYS.iter()
            .filter(|(pressed, _)| i.key_pressed(*pressed))
            .map(|(_, key)| Shortcut {
                key: *key,
                ctrl: i.modifiers.command,
                shift: i.modifiers.shift,
            })
            .collect()
    })
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
}

impl App {
    fn new(lang: Lang, settings: ViewerSettings, initial: Option<PathBuf>) -> App {
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
            launcher: Launcher::from_env(&settings, &|name: &str| std::env::var(name).ok()),
            launch_note: None,
        };
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
                self.workspace = None;
                self.sch = None;
                self.brd = None;
                self.error = None;
                self.launcher.opened(path);
            } else if let Some(e) = self.lib.take_error() {
                self.error = Some(ViewError::new(FILE_UNUSABLE, e.message(Lang::En)));
            }
            return;
        }
        match Workspace::open(path, &self.settings, &self.route_settings) {
            Ok(workspace) => {
                self.lib.close();
                self.save_text = suggested_save_path(path).to_string_lossy().into_owned();
                self.workspace = Some(workspace);
                self.error = None;
                self.fit_pending = true;
                self.tool_drag = false;
                self.sch = None;
                self.brd = None;
                self.launcher.opened(path);
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Does what the start screen asked for.
    fn launch(&mut self, asked: Launch) {
        match asked {
            Launch::Open(path) => {
                self.path_text = path.to_string_lossy().into_owned();
                self.open(&path);
            }
            Launch::NewLibrary => {
                self.lib.new_library();
                self.workspace = None;
                self.sch = None;
                self.brd = None;
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("EH3D-PHOENIX-PCB")
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                ui.label(dim(Msg::PathHint.tr(lang)));
                let edit = egui::TextEdit::singleline(&mut self.path_text).desired_width(300.0);
                let response = ui.add(edit);
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(Msg::Open.tr(lang)).clicked() || enter {
                    let path = PathBuf::from(self.path_text.trim());
                    self.open(&path);
                }
                if ui.button(LibText::Title.tr(lang)).clicked() {
                    self.lib.new_library();
                    self.workspace = None;
                    self.sch = None;
                    self.brd = None;
                }
                let has_scene = self.workspace.is_some();
                if ui
                    .add_enabled(has_scene, egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
                let editing = self.sch.is_some() || self.brd.is_some();
                if let Some(w) = self.workspace.as_mut() {
                    ui.separator();
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
                    ui.separator();
                    ui.label(dim(EditMsg::SavePath.tr(lang)));
                    let save = egui::TextEdit::singleline(&mut self.save_text).desired_width(260.0);
                    let response = ui.add(save);
                    if self.focus_save {
                        response.request_focus();
                        self.focus_save = false;
                    }
                    if ui.button(EditMsg::SaveAs.tr(lang)).clicked() {
                        let target = PathBuf::from(self.save_text.trim());
                        w.save_as(&target);
                    }
                    if w.dirty() {
                        ui.label(egui::RichText::new(EditMsg::Unsaved.tr(lang)).color(DANGER));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(language_name(self.lang))
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.lang, l, language_name(l));
                            }
                        });
                    ui.label(dim(Msg::Language.tr(lang)));
                });
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::left("layers")
            .default_size(260.0)
            .show(ui, |ui| {
                let Some(workspace) = self.workspace.as_mut() else {
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
                    ui.separator();
                    ui.label(heading(Msg::Layers.tr(lang)));
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
                                viewer.set_layer_colour(
                                    i,
                                    [chosen.r(), chosen.g(), chosen.b(), chosen.a()],
                                );
                            }
                        });
                    }
                });
            });
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::right("findings")
            .default_size(340.0)
            .show(ui, |ui| {
                let Some(workspace) = self.workspace.as_mut() else {
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
            });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let note = self.workspace.as_ref().and_then(|w| w.note.as_ref());
                if let Some(error) = &self.error {
                    ui.label(egui::RichText::new(error.message(lang)).color(DANGER));
                } else if let Some(note) = note {
                    let colour = if note.is_error() { DANGER } else { ACCENT };
                    ui.label(egui::RichText::new(note.text(lang)).color(colour));
                } else if let Some(w) = &self.workspace {
                    ui.label(egui::RichText::new(w.viewer.status(lang)).color(OK));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::Prototype.tr(lang)).color(ACCENT));
                    ui.label(dim(Msg::ZoomHint.tr(lang)));
                });
            });
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let background = rgba(self.settings.colour("background"));
        egui::CentralPanel::no_frame().show(ui, |ui| {
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
            for shortcut in shortcuts(ui.ctx()) {
                workspace.shortcut(shortcut);
            }
            if workspace.want_save {
                workspace.want_save = false;
                self.focus_save = true;
            }
            paint(workspace, &painter, rect);
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
            .color(DANGER),
        );
        for (i, finding) in session.last_diff.added.iter().take(shown).enumerate() {
            ui.push_id(("added", i), |ui| {
                finding_row(ui, &finding_line(finding, lang), DANGER, lang, || {
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
            .color(OK),
        );
        for (i, finding) in session.last_diff.removed.iter().take(shown).enumerate() {
            ui.push_id(("removed", i), |ui| {
                finding_row(ui, &finding_line(finding, lang), OK, lang, || {
                    jump = Some(finding.at_nm);
                });
            });
        }
        ui.separator();
        let total = session.findings.len();
        ui.label(heading(&format!(
            "{} ({total})",
            EditMsg::RuleFindings.tr(lang)
        )));
        if total == 0 {
            ui.label(dim(EditMsg::NoFindings.tr(lang)));
        }
        for (i, finding) in session.findings.iter().take(shown).enumerate() {
            ui.push_id(("finding", i), |ui| {
                finding_row(ui, &finding_line(finding, lang), TEXT, lang, || {
                    jump = Some(finding.at_nm);
                });
            });
        }
    }
    if let Some(place) = jump {
        workspace.jump_to_place(place);
    }
    ui.separator();
    ui.label(heading(EditMsg::NetList.tr(lang)));
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
                    eh3d_phoenix_rules::Severity::Error => DANGER,
                    eh3d_phoenix_rules::Severity::Warning => ACCENT,
                    eh3d_phoenix_rules::Severity::Info => TEXT,
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
                        finding_row(ui, &erc_line(finding, lang), DIM, lang, || {
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
    // the measure, over everything
    if let Some(a) = viewer.measure.a {
        painter.circle_filled(at(a), 4.0, measure_colour);
        if let Some(b) = viewer.measure.b {
            painter.line_segment([at(a), at(b)], Stroke::new(1.5, measure_colour));
            painter.circle_filled(at(b), 4.0, measure_colour);
            if let Some(d) = viewer.measure.distance() {
                let middle = P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                painter.text(
                    at(middle),
                    egui::Align2::CENTER_BOTTOM,
                    format!("{d:.3} mm"),
                    egui::FontId::proportional(14.0),
                    measure_colour,
                );
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // a file dropped on the window is opened
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            match Launcher::dropped_launch(&dropped) {
                Ok(asked) => {
                    self.launch_note = None;
                    self.launch(asked);
                }
                Err(note) => self.launch_note = Some(note),
            }
        }
        if self.lib.is_open() {
            self.top_bar(ui);
            self.lib.show(ui, self.lang);
            return;
        }
        self.top_bar(ui);
        self.status_bar(ui);
        self.side_panel(ui);
        self.right_panel(ui);
        self.canvas(ui);
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

/// The colours of the theme Forge for the controls of egui.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x11, 0x0e);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110);
    v.selection.stroke = Stroke::new(1.5, ACCENT);
    v.widgets.inactive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110));
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x28, 0x1c);
    v.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Opens the window. A file given here is opened at once.
pub fn run(
    initial: Option<PathBuf>,
    lang: Lang,
    settings: ViewerSettings,
) -> Result<(), ViewError> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1480.0, 800.0])
        .with_min_inner_size([900.0, 480.0])
        .with_title("EH3D-PHOENIX-PCB");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EH3D-PHOENIX-PCB",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(lang, settings, initial)))
        }),
    )
    .map_err(|e| ViewError::new(WINDOW_FAILED, e.to_string()))
}
