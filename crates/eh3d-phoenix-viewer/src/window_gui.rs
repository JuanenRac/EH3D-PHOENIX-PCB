// SPDX-License-Identifier: GPL-3.0-or-later
//! The common parts of the window (built with the `gui` feature): the search, the measures, the
//! layers and their presets, the history, the appearance, the list of shortcuts and the help, and
//! the preferences that are remembered. Everything that can be decided without a screen is decided
//! in `search_index`, `measure_set`, `layer_presets`, `keymap`, `history_view`, `appearance`,
//! `help_text` and `prefs`, in code that is tested; this file only draws it with egui and passes on
//! what the person does. It has not been seen on a screen.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_rules::Lang;

use crate::appearance::{self, Theme, FONT_MAX, FONT_MIN, SCALE_MAX, SCALE_MIN};
use crate::codes::ViewError;
use crate::editmsg::EditMsg;
use crate::gui::{ui_accent, ui_danger, ui_dim};
use crate::help_text::HelpMode;
use crate::history_view::Move;
use crate::keymap::{Action, Binding, KeyName, Keymap};
use crate::layer_presets::{self, LayerControl, LayerPreset};
use crate::measure_set::{describe, Unit};
use crate::msg::Msg;
use crate::prefs::{self, Prefs, Saver};
use crate::prim::P;
use crate::search_index::{self, footprint_pairs, Hit, HitKind, SearchIndex, SearchState};
use crate::settings::ViewerSettings;
use crate::shot::ShotWindow;
use crate::view3d_gui::View3dUi;
use crate::viewer::Viewer;
use crate::window_text::WinText;
use crate::workspace::Workspace;

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ui_accent())
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ui_dim())
}

/// The keys of the toolkit that a key of the table stands for. The plus sign is the key of the
/// equals sign on many keyboards, so both are taken.
fn egui_keys(key: KeyName) -> &'static [egui::Key] {
    use egui::Key as K;
    match key {
        KeyName::A => &[K::A],
        KeyName::B => &[K::B],
        KeyName::C => &[K::C],
        KeyName::D => &[K::D],
        KeyName::E => &[K::E],
        KeyName::F => &[K::F],
        KeyName::G => &[K::G],
        KeyName::H => &[K::H],
        KeyName::I => &[K::I],
        KeyName::J => &[K::J],
        KeyName::K => &[K::K],
        KeyName::L => &[K::L],
        KeyName::M => &[K::M],
        KeyName::N => &[K::N],
        KeyName::O => &[K::O],
        KeyName::P => &[K::P],
        KeyName::Q => &[K::Q],
        KeyName::R => &[K::R],
        KeyName::S => &[K::S],
        KeyName::T => &[K::T],
        KeyName::U => &[K::U],
        KeyName::V => &[K::V],
        KeyName::W => &[K::W],
        KeyName::X => &[K::X],
        KeyName::Y => &[K::Y],
        KeyName::Z => &[K::Z],
        KeyName::Num0 => &[K::Num0],
        KeyName::Num1 => &[K::Num1],
        KeyName::Num2 => &[K::Num2],
        KeyName::Num3 => &[K::Num3],
        KeyName::Num4 => &[K::Num4],
        KeyName::Num5 => &[K::Num5],
        KeyName::Num6 => &[K::Num6],
        KeyName::Num7 => &[K::Num7],
        KeyName::Num8 => &[K::Num8],
        KeyName::Num9 => &[K::Num9],
        KeyName::F1 => &[K::F1],
        KeyName::F2 => &[K::F2],
        KeyName::F3 => &[K::F3],
        KeyName::F4 => &[K::F4],
        KeyName::F5 => &[K::F5],
        KeyName::F6 => &[K::F6],
        KeyName::F7 => &[K::F7],
        KeyName::F8 => &[K::F8],
        KeyName::F9 => &[K::F9],
        KeyName::F10 => &[K::F10],
        KeyName::F11 => &[K::F11],
        KeyName::F12 => &[K::F12],
        KeyName::Delete => &[K::Delete],
        KeyName::Escape => &[K::Escape],
        KeyName::Plus => &[K::Plus, K::Equals],
        KeyName::Minus => &[K::Minus],
        KeyName::Enter => &[K::Enter],
        KeyName::Tab => &[K::Tab],
        KeyName::Space => &[K::Space],
        KeyName::Up => &[K::ArrowUp],
        KeyName::Down => &[K::ArrowDown],
        KeyName::Left => &[K::ArrowLeft],
        KeyName::Right => &[K::ArrowRight],
        KeyName::PageUp => &[K::PageUp],
        KeyName::PageDown => &[K::PageDown],
        KeyName::Home => &[K::Home],
        KeyName::End => &[K::End],
        KeyName::Slash => &[K::Slash],
        KeyName::Comma => &[K::Comma],
        KeyName::Period => &[K::Period],
        KeyName::Backspace => &[K::Backspace],
    }
}

/// True when the keys of a binding are pressed in this frame. The modifiers must be the ones of
/// the binding, except for the plus sign, which on many keyboards is typed with Shift.
fn binding_pressed(input: &egui::InputState, binding: Binding) -> bool {
    if binding.ctrl != input.modifiers.command {
        return false;
    }
    if binding.key != KeyName::Plus && binding.shift != input.modifiers.shift {
        return false;
    }
    egui_keys(binding.key)
        .iter()
        .any(|key| input.key_pressed(*key))
}

/// The actions whose keys are pressed in this frame, unless a text box has the keyboard.
pub fn pressed_actions(ctx: &egui::Context, keymap: &Keymap) -> Vec<Action> {
    if ctx.egui_wants_keyboard_input() {
        return Vec::new();
    }
    ctx.input(|input| {
        Action::ALL
            .iter()
            .filter(|action| {
                keymap
                    .keys(**action)
                    .iter()
                    .any(|binding| binding_pressed(input, *binding))
            })
            .copied()
            .collect()
    })
}

fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> Color32 {
    let channel = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(
        channel(a[0], b[0]),
        channel(a[1], b[1]),
        channel(a[2], b[2]),
    )
}

/// Applies the look of the settings to the toolkit: the colours of the controls of the theme, the
/// scale of the interface and the size of the letters. The colours in use are also kept for the
/// panels, which read them when they draw.
pub fn apply_look(ctx: &egui::Context, settings: &ViewerSettings) {
    let palette = appearance::ui_palette(settings);
    appearance::set_current(palette);
    ctx.set_zoom_factor(appearance::zoom_factor(settings.ui_scale_percent));
    let factor = appearance::font_factor(settings.font_size_pt) as f32;
    ctx.global_style_mut(|style| {
        for (text_style, font) in egui::Style::default().text_styles {
            style.text_styles.insert(
                text_style,
                egui::FontId::new(font.size * factor, font.family),
            );
        }
    });
    let mut v = if settings.theme == Theme::Light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    let accent = rgba(palette.accent);
    v.panel_fill = rgba(palette.panel);
    v.window_fill = rgba(palette.raised);
    v.extreme_bg_color = if settings.theme == Theme::Dark {
        Color32::from_rgb(0x14, 0x11, 0x0e)
    } else {
        rgba(palette.raised)
    };
    v.override_text_color = Some(rgba(palette.text));
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(
        palette.accent[0],
        palette.accent[1],
        palette.accent[2],
        110,
    );
    v.selection.stroke = Stroke::new(1.5, accent);
    v.widgets.inactive.bg_stroke = Stroke::new(
        1.0,
        Color32::from_rgba_unmultiplied(
            palette.accent[0],
            palette.accent[1],
            palette.accent[2],
            110,
        ),
    );
    v.widgets.inactive.weak_bg_fill = rgba(palette.raised);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, accent);
    v.widgets.hovered.weak_bg_fill = mix(palette.raised, palette.accent, 0.18);
    v.widgets.active.bg_stroke = Stroke::new(2.0, accent);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Draws the measures: the ones that were kept and the one in progress, each with its reading in
/// the unit of the settings.
pub fn paint_measures(
    painter: &egui::Painter,
    viewer: &Viewer,
    at: &dyn Fn(P) -> Pos2,
    colour: Color32,
) {
    let settings = &viewer.settings;
    let draw = |a: P, b: Option<P>| {
        painter.circle_filled(at(a), 4.0, colour);
        if let Some(b) = b {
            painter.line_segment([at(a), at(b)], Stroke::new(1.5, colour));
            painter.circle_filled(at(b), 4.0, colour);
            let middle = P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            painter.text(
                at(middle),
                egui::Align2::CENTER_BOTTOM,
                settings.format_length(a.dist(b)),
                egui::FontId::proportional(14.0),
                colour,
            );
        }
    };
    for (a, b) in viewer.measures.items() {
        draw(*a, Some(*b));
    }
    if let Some(a) = viewer.measure.a {
        draw(a, viewer.measure.b);
    }
}

/// What the window knows about the state of the program in this frame.
pub struct FrameContext<'a> {
    /// The language of the window.
    pub lang: Lang,
    /// The mode that is shown.
    pub mode: HelpMode,
    /// True while the drawing is still waiting to be fitted to the window.
    pub fit_pending: bool,
    /// True while the editor of a schematic or a board has the document.
    pub editing: bool,
    /// The document that is open and can be looked at, when there is one.
    pub workspace: Option<&'a mut Workspace>,
    /// The 3D window, when it is shown.
    pub view3d: Option<&'a mut View3dUi>,
}

/// The state of the common parts of the window.
pub struct WindowUi {
    prefs: Prefs,
    path: Option<PathBuf>,
    saver: Saver,
    error: Option<(WinText, ViewError)>,
    search: SearchState,
    index: SearchIndex,
    search_open: bool,
    search_focus: bool,
    measures_open: bool,
    layers_open: bool,
    history_open: bool,
    appearance_open: bool,
    shortcuts_open: bool,
    help_open: bool,
    layer_control: LayerControl,
    order: Vec<usize>,
    preset_name: String,
    note: Option<WinText>,
    restore_zoom: Option<f64>,
}

impl WindowUi {
    /// Reads the preferences from the settings folder of the user. A file that is not valid is not
    /// used and the reason is kept to be shown.
    pub fn load(settings: &ViewerSettings) -> WindowUi {
        let path = prefs::prefs_path(&|name: &str| std::env::var(name).ok(), &settings.prefs_file);
        let mut error = None;
        let prefs = match path.as_deref() {
            Some(file) => match prefs::load(file, settings.prefs_max_bytes, settings.preset_max) {
                Ok(prefs) => prefs,
                Err(e) => {
                    error = Some((WinText::PrefsInvalid, e));
                    empty_prefs(settings)
                }
            },
            None => empty_prefs(settings),
        };
        WindowUi::with(prefs, path, error)
    }

    /// The preferences of a window that reads and writes none: the picture mode starts from the
    /// defaults of the settings and leaves the file of the person as it is.
    pub fn ephemeral(settings: &ViewerSettings) -> WindowUi {
        WindowUi::with(empty_prefs(settings), None, None)
    }

    /// Opens the window of the toolbox that a picture asks for. The windows that show a
    /// document are not toolbox windows and are left to the caller.
    pub fn show_window(&mut self, window: ShotWindow) {
        match window {
            ShotWindow::Search => {
                self.search_open = true;
                self.search_focus = true;
            }
            ShotWindow::Measures => self.measures_open = true,
            ShotWindow::Layers => self.layers_open = true,
            ShotWindow::History => self.history_open = true,
            ShotWindow::Appearance => self.appearance_open = true,
            ShotWindow::Shortcuts => self.shortcuts_open = true,
            ShotWindow::Help => self.help_open = true,
            ShotWindow::Start
            | ShotWindow::Document
            | ShotWindow::View3d
            | ShotWindow::Panel
            | ShotWindow::Gerber => {}
        }
    }

    fn with(prefs: Prefs, path: Option<PathBuf>, error: Option<(WinText, ViewError)>) -> WindowUi {
        WindowUi {
            prefs,
            path,
            saver: Saver::default(),
            error,
            search: SearchState::default(),
            index: SearchIndex::default(),
            search_open: false,
            search_focus: false,
            measures_open: false,
            layers_open: false,
            history_open: false,
            appearance_open: false,
            shortcuts_open: false,
            help_open: false,
            layer_control: LayerControl::default(),
            order: Vec::new(),
            preset_name: String::new(),
            note: None,
            restore_zoom: None,
        }
    }

    /// Puts what the person chose in an earlier run into the settings.
    pub fn adopt(&self, settings: &mut ViewerSettings) {
        self.prefs.apply_to(settings);
    }

    /// The size the window had when it was last closed, in logical pixels.
    pub fn window_size(&self) -> Option<[f32; 2]> {
        self.prefs.window.map(|(w, h)| [w as f32, h as f32])
    }

    /// The text for the status bar when the preferences could not be read or written.
    pub fn error_text(&self, lang: Lang) -> Option<String> {
        self.error
            .as_ref()
            .map(|(what, e)| format!("{}: {}", what.tr(lang), e.detail))
    }

    /// A document was opened: the search and the layers start again, and the layers and the zoom
    /// that the preferences remember for this kind of document are put back.
    pub fn after_open(&mut self, workspace: &mut Workspace, settings: &ViewerSettings) {
        self.search.clear();
        self.index = SearchIndex::default();
        self.layer_control = LayerControl::default();
        self.order.clear();
        self.restore_zoom = None;
        let key = if workspace.is_board() {
            "board"
        } else {
            "schematic"
        };
        if let Some(layers) = self.prefs.layers.get(key) {
            let preset = LayerPreset {
                name: key.to_string(),
                layers: layers.clone(),
            };
            let applied = layer_presets::apply(&mut workspace.viewer.scene, &preset, settings);
            self.order = applied.order;
        }
        self.restore_zoom = self.prefs.zoom.get(key).copied();
    }

    /// The bar with the buttons that show and hide the windows, drawn above everything else.
    pub fn toolbar(&mut self, ui: &mut egui::Ui, lang: Lang, has_document: bool) {
        egui::Panel::top("e8_window_bar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.toggle_value(&mut self.search_open, WinText::SearchTitle.tr(lang));
                ui.add_enabled_ui(has_document, |ui| {
                    ui.toggle_value(&mut self.measures_open, WinText::MeasureTitle.tr(lang));
                    ui.toggle_value(&mut self.layers_open, WinText::PresetsTitle.tr(lang));
                    ui.toggle_value(&mut self.history_open, WinText::HistoryTitle.tr(lang));
                });
                ui.toggle_value(&mut self.appearance_open, WinText::AppearanceTitle.tr(lang));
                ui.toggle_value(&mut self.shortcuts_open, WinText::ShortcutsTitle.tr(lang));
                ui.toggle_value(&mut self.help_open, WinText::HelpTitle.tr(lang));
                ui.label(dim(WinText::HelpHint.tr(lang)));
            });
        });
    }

    /// Does what the keys ask for, draws the windows that are open and takes care of the
    /// preferences. It is called once in every frame, after the rest of the window is drawn.
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        settings: &mut ViewerSettings,
        actions: &[Action],
        frame: FrameContext<'_>,
    ) {
        let FrameContext {
            lang,
            mode,
            fit_pending,
            editing,
            mut workspace,
            mut view3d,
        } = frame;
        if !fit_pending {
            if let (Some(zoom), Some(w)) = (self.restore_zoom, workspace.as_deref_mut()) {
                let limits = w.viewer.limits();
                w.viewer.view.scale = zoom.clamp(limits.min, limits.max);
                self.restore_zoom = None;
            }
        }
        for action in actions {
            // the editors of boards and schematics have keys of their own, so only the keys that
            // they do not use (the search, the history, the list of shortcuts and the help) work there
            let free = matches!(
                action,
                Action::Search
                    | Action::SearchNext
                    | Action::SearchPrevious
                    | Action::Help
                    | Action::ShortcutsPanel
                    | Action::HistoryPanel
            );
            if editing && !free {
                continue;
            }
            self.run_action(
                *action,
                settings,
                workspace.as_deref_mut(),
                view3d.as_deref_mut(),
            );
        }
        self.search_window(ctx, settings, lang, workspace.as_deref_mut(), view3d);
        self.measures_window(ctx, settings, lang, workspace.as_deref_mut());
        self.layers_window(ctx, settings, lang, workspace.as_deref_mut());
        self.history_window(ctx, settings, lang, editing, workspace.as_deref_mut());
        self.appearance_window(ctx, settings, lang, workspace.as_deref_mut());
        self.shortcuts_window(ctx, settings, lang);
        self.help_window(ctx, lang, mode);
        if let Some(w) = workspace.as_deref_mut() {
            w.viewer.settings.measure_unit = settings.measure_unit;
            w.viewer.settings.measure_decimals = settings.measure_decimals;
            w.viewer.settings.measure_snap = settings.measure_snap;
        }
        self.remember(ctx, settings, mode, fit_pending, workspace.as_deref());
    }

    fn run_action(
        &mut self,
        action: Action,
        settings: &mut ViewerSettings,
        workspace: Option<&mut Workspace>,
        view3d: Option<&mut View3dUi>,
    ) {
        match action {
            Action::Search => {
                self.search_open = true;
                self.search_focus = true;
            }
            Action::SearchNext | Action::SearchPrevious => {
                if action == Action::SearchNext {
                    self.search.go_next();
                } else {
                    self.search.go_previous();
                }
                self.search_open = true;
                self.go_current(settings, workspace, view3d);
            }
            Action::Help => self.help_open = !self.help_open,
            Action::ShortcutsPanel => self.shortcuts_open = !self.shortcuts_open,
            Action::HistoryPanel => self.history_open = !self.history_open,
            Action::SoloLayer => {
                if let Some(w) = workspace {
                    self.solo(w);
                }
            }
            Action::HighlightNet => {
                if let Some(w) = workspace {
                    layer_presets::highlight_selected_net(&mut w.viewer);
                }
            }
            Action::ClearMeasures => {
                if let Some(w) = workspace {
                    w.viewer.clear_measures();
                }
            }
            Action::ToggleUnit => {
                settings.measure_unit = settings.measure_unit.other();
                self.prefs.unit = Some(settings.measure_unit);
                self.saver.mark();
            }
            other => {
                if let (Some(n), Some(w)) = (other.preset_index(), workspace) {
                    self.apply_preset(n, w, settings);
                }
            }
        }
    }

    fn solo(&mut self, w: &mut Workspace) {
        if self.layer_control.active.is_none() {
            let from = layer_presets::layer_of_selection(&w.viewer).unwrap_or(0);
            self.layer_control.set_active(&w.viewer.scene, from);
        }
        self.layer_control.toggle_solo(&mut w.viewer.scene);
    }

    fn apply_preset(&mut self, at: usize, w: &mut Workspace, settings: &ViewerSettings) {
        if let Some(preset) = self.prefs.presets.get(at).cloned() {
            let applied = layer_presets::apply(&mut w.viewer.scene, &preset, settings);
            self.order = applied.order;
            self.layer_control.forget();
        }
    }

    fn run_search(
        &mut self,
        settings: &ViewerSettings,
        workspace: Option<&Workspace>,
        view3d: Option<&View3dUi>,
    ) {
        if let Some(v) = view3d {
            let parts = v.part_labels();
            let hits = search_index::search_parts(&parts, &self.search.query)
                .into_iter()
                .take(settings.search_max_results)
                .map(|at| Hit {
                    kind: HitKind::Component,
                    item: Some(at as u32),
                    net: None,
                    label: format!("{} {}", parts[at].0, parts[at].1)
                        .trim()
                        .to_string(),
                    score: 0,
                })
                .collect();
            self.search.set_hits(hits);
        } else if let Some(w) = workspace {
            let pairs = w
                .board_session()
                .map(|s| footprint_pairs(&s.board))
                .unwrap_or_default();
            self.index = SearchIndex::build(&w.viewer.scene, &pairs);
            self.search.run(&self.index, settings.search_max_results);
        } else {
            self.search.set_hits(Vec::new());
        }
    }

    fn go_current(
        &mut self,
        settings: &ViewerSettings,
        workspace: Option<&mut Workspace>,
        view3d: Option<&mut View3dUi>,
    ) {
        let Some(hit) = self.search.current().cloned() else {
            return;
        };
        if let Some(v) = view3d {
            if let Some(at) = hit.item {
                v.select_part(at as usize);
            }
        } else if let Some(w) = workspace {
            search_index::go_to(&mut w.viewer, &hit, settings.search_context_mm);
        }
    }

    fn search_window(
        &mut self,
        ctx: &egui::Context,
        settings: &ViewerSettings,
        lang: Lang,
        mut workspace: Option<&mut Workspace>,
        mut view3d: Option<&mut View3dUi>,
    ) {
        if !self.search_open {
            return;
        }
        let mut open = true;
        egui::Window::new(WinText::SearchTitle.tr(lang))
            .open(&mut open)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.label(dim(WinText::SearchHint.tr(lang)));
                let before = self.search.query.clone();
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.search.query).desired_width(f32::INFINITY),
                );
                if self.search_focus {
                    edit.request_focus();
                    self.search_focus = false;
                }
                if before != self.search.query {
                    self.run_search(settings, workspace.as_deref(), view3d.as_deref());
                    self.go_current(settings, workspace.as_deref_mut(), view3d.as_deref_mut());
                }
                let (down, up, enter) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::Enter),
                    )
                });
                let mut go = false;
                let mut moved = false;
                if edit.has_focus() || edit.lost_focus() {
                    if down {
                        self.search.go_next();
                        moved = true;
                        go = true;
                    } else if up {
                        self.search.go_previous();
                        moved = true;
                        go = true;
                    } else if enter && edit.lost_focus() {
                        go = true;
                    }
                }
                ui.label(dim(format!(
                    "{} {}",
                    self.search.hits().len(),
                    WinText::SearchResults.tr(lang)
                )));
                if self.search.hits().is_empty() && !self.search.query.trim().is_empty() {
                    ui.label(dim(WinText::SearchNothing.tr(lang)));
                }
                let mut clicked = None;
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (at, hit) in self.search.hits().iter().enumerate() {
                            let kind = match hit.kind {
                                HitKind::Component => WinText::KindComponent,
                                HitKind::Net => WinText::KindNet,
                                HitKind::Pad => WinText::KindPad,
                            };
                            let selected = at == self.search.cursor();
                            let row = ui.selectable_label(
                                selected,
                                format!("{}  {}", hit.label, kind.tr(lang)),
                            );
                            if selected && moved {
                                row.scroll_to_me(None);
                            }
                            if row.clicked() {
                                clicked = Some(at);
                            }
                        }
                    });
                if let Some(at) = clicked {
                    self.search.set_cursor(at);
                    go = true;
                }
                if go {
                    self.go_current(settings, workspace, view3d);
                }
            });
        self.search_open = open;
    }

    fn measures_window(
        &mut self,
        ctx: &egui::Context,
        settings: &mut ViewerSettings,
        lang: Lang,
        workspace: Option<&mut Workspace>,
    ) {
        let Some(w) = workspace else {
            return;
        };
        if !self.measures_open {
            return;
        }
        let mut open = true;
        egui::Window::new(WinText::MeasureTitle.tr(lang))
            .open(&mut open)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(dim(WinText::MeasureUnit.tr(lang)));
                    for unit in [Unit::Mm, Unit::Mil] {
                        if ui
                            .selectable_value(&mut settings.measure_unit, unit, unit.name())
                            .clicked()
                        {
                            self.prefs.unit = Some(unit);
                            self.saver.mark();
                        }
                    }
                    ui.add(egui::DragValue::new(&mut settings.measure_decimals).range(1..=6));
                });
                ui.checkbox(&mut settings.measure_snap, WinText::MeasureSnap.tr(lang));
                let (unit, decimals) = (settings.measure_unit, settings.measure_decimals);
                let kept = w.viewer.measures.items().to_vec();
                let mut delete = None;
                for (at, (a, b)) in kept.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(describe(*a, *b, unit, decimals));
                        if ui.small_button(WinText::Delete.tr(lang)).clicked() {
                            delete = Some(at);
                        }
                    });
                }
                if let Some(at) = delete {
                    w.viewer.measures.remove(at);
                }
                if let (Some(a), Some(b)) = (w.viewer.measure.a, w.viewer.measure.b) {
                    ui.label(describe(a, b, unit, decimals));
                }
                if kept.is_empty() && w.viewer.measure.a.is_none() {
                    ui.label(dim(WinText::MeasureNone.tr(lang)));
                }
                if let Some(angle) = w.viewer.measures.last_angle() {
                    ui.label(format!(
                        "{}: {angle:.2}°",
                        WinText::MeasureAngleBetween.tr(lang)
                    ));
                }
                if ui.button(WinText::MeasureClearAll.tr(lang)).clicked() {
                    w.viewer.clear_measures();
                }
            });
        self.measures_open = open;
    }

    fn layers_window(
        &mut self,
        ctx: &egui::Context,
        settings: &ViewerSettings,
        lang: Lang,
        workspace: Option<&mut Workspace>,
    ) {
        let Some(w) = workspace else {
            return;
        };
        if !self.layers_open {
            return;
        }
        let mut open = true;
        egui::Window::new(WinText::PresetsTitle.tr(lang))
            .open(&mut open)
            .default_width(360.0)
            .show(ctx, |ui| {
                let count = w.viewer.scene.layers.len();
                let order = layer_presets::complete_order(&self.order, count);
                let captured = layer_presets::capture(&w.viewer.scene, &order, settings);
                let mut shift: Option<(usize, bool)> = None;
                for (place, (layer, state)) in order.iter().zip(&captured).enumerate() {
                    let name = {
                        let l = &w.viewer.scene.layers[*layer];
                        Msg::for_role(l.role)
                            .map_or_else(|| l.name.clone(), |m| m.tr(lang).to_string())
                    };
                    ui.horizontal(|ui| {
                        let mut visible = state.visible;
                        if ui.checkbox(&mut visible, name).changed() {
                            w.viewer.set_layer_visible(*layer, visible);
                        }
                        let active = self.layer_control.active == Some(*layer);
                        if ui.radio(active, "").clicked() {
                            self.layer_control.set_active(&w.viewer.scene, *layer);
                        }
                        let mut opacity = state.opacity;
                        let drag = egui::DragValue::new(&mut opacity)
                            .range(0..=100)
                            .suffix("%");
                        if ui.add(drag).changed() {
                            layer_presets::set_opacity(
                                &mut w.viewer.scene,
                                *layer,
                                opacity,
                                settings,
                            );
                        }
                        if ui.small_button("^").clicked() {
                            shift = Some((place, true));
                        }
                        if ui.small_button("v").clicked() {
                            shift = Some((place, false));
                        }
                    });
                }
                if let Some((place, up)) = shift {
                    let mut moved = order.clone();
                    let other = if up {
                        place.checked_sub(1)
                    } else {
                        Some(place + 1).filter(|n| *n < moved.len())
                    };
                    if let Some(other) = other {
                        moved.swap(place, other);
                    }
                    self.order = moved;
                }
                ui.horizontal(|ui| {
                    let mut solo = self.layer_control.is_solo();
                    if ui.checkbox(&mut solo, WinText::Solo.tr(lang)).clicked() {
                        self.solo(w);
                    }
                    if ui.button(WinText::HighlightNet.tr(lang)).clicked() {
                        layer_presets::highlight_selected_net(&mut w.viewer);
                    }
                });
                ui.separator();
                ui.label(heading(WinText::PresetsTitle.tr(lang)));
                ui.horizontal(|ui| {
                    ui.label(dim(WinText::PresetName.tr(lang)));
                    ui.text_edit_singleline(&mut self.preset_name);
                    if ui.button(WinText::PresetSave.tr(lang)).clicked() {
                        let preset = LayerPreset {
                            name: self.preset_name.clone(),
                            layers: layer_presets::capture(&w.viewer.scene, &order, settings),
                        };
                        match self.prefs.presets.save(preset) {
                            Ok(()) => {
                                self.note = None;
                                self.saver.mark();
                            }
                            Err(text) => self.note = Some(text),
                        }
                    }
                });
                if let Some(text) = self.note {
                    ui.label(egui::RichText::new(text.tr(lang)).color(ui_danger()));
                }
                let names: Vec<String> = self
                    .prefs
                    .presets
                    .items()
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();
                if names.is_empty() {
                    ui.label(dim(WinText::PresetNone.tr(lang)));
                }
                let mut apply = None;
                let mut remove = None;
                for (at, name) in names.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}. {name}", at + 1));
                        if ui.button(WinText::PresetApply.tr(lang)).clicked() {
                            apply = Some(at);
                        }
                        if ui.small_button(WinText::Delete.tr(lang)).clicked() {
                            remove = Some(at);
                        }
                    });
                }
                if let Some(at) = apply {
                    self.apply_preset(at, w, settings);
                }
                if let Some(at) = remove {
                    if self.prefs.presets.remove(at) {
                        self.saver.mark();
                    }
                }
            });
        self.layers_open = open;
    }

    fn history_window(
        &mut self,
        ctx: &egui::Context,
        settings: &ViewerSettings,
        lang: Lang,
        editing: bool,
        workspace: Option<&mut Workspace>,
    ) {
        let Some(w) = workspace else {
            return;
        };
        if !self.history_open {
            return;
        }
        let view = w.history_view(lang, settings.history_shown);
        let mut go = Move::Stay;
        let mut open = true;
        egui::Window::new(WinText::HistoryTitle.tr(lang))
            .open(&mut open)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let undo = egui::Button::new(EditMsg::Undo.tr(lang));
                    if ui.add_enabled(!editing && w.can_undo(), undo).clicked() {
                        go = Move::Undo(1);
                    }
                    let redo = egui::Button::new(EditMsg::Redo.tr(lang));
                    if ui.add_enabled(!editing && w.can_redo(), redo).clicked() {
                        go = Move::Redo(1);
                    }
                });
                if view.rows.is_empty() {
                    ui.label(dim(WinText::HistoryEmpty.tr(lang)));
                }
                if view.hidden > 0 {
                    ui.label(dim(format!(
                        "{} {}",
                        view.hidden,
                        WinText::HistoryOlder.tr(lang)
                    )));
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (at, row) in view.rows.iter().enumerate() {
                            let now = at + 1 == view.done_count();
                            let text = if now {
                                format!("{}   ({})", row.label, WinText::HistoryNow.tr(lang))
                            } else {
                                row.label.clone()
                            };
                            let text = if row.done {
                                egui::RichText::new(text)
                            } else {
                                dim(text)
                            };
                            if ui
                                .add_enabled(!editing, egui::Button::selectable(now, text))
                                .clicked()
                            {
                                go = view.go_to(at);
                            }
                        }
                    });
            });
        self.history_open = open;
        match go {
            Move::Stay => {}
            Move::Undo(n) => {
                for _ in 0..n {
                    w.undo();
                }
            }
            Move::Redo(n) => {
                for _ in 0..n {
                    w.redo();
                }
            }
        }
    }

    fn appearance_window(
        &mut self,
        ctx: &egui::Context,
        settings: &mut ViewerSettings,
        lang: Lang,
        workspace: Option<&mut Workspace>,
    ) {
        if !self.appearance_open {
            return;
        }
        let mut open = true;
        let mut theme = settings.theme;
        let mut percent = settings.ui_scale_percent;
        let mut points = settings.font_size_pt;
        egui::Window::new(WinText::AppearanceTitle.tr(lang))
            .open(&mut open)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(dim(WinText::ThemeLabel.tr(lang)));
                    for t in Theme::ALL {
                        ui.selectable_value(&mut theme, t, t.label().tr(lang));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(dim(WinText::UiScale.tr(lang)));
                    ui.add(egui::Slider::new(&mut percent, SCALE_MIN..=SCALE_MAX));
                });
                ui.horizontal(|ui| {
                    ui.label(dim(WinText::FontSize.tr(lang)));
                    ui.add(egui::Slider::new(&mut points, FONT_MIN..=FONT_MAX));
                });
            });
        self.appearance_open = open;
        let theme_changed = theme != settings.theme;
        if theme_changed || percent != settings.ui_scale_percent || points != settings.font_size_pt
        {
            if theme_changed {
                settings.set_theme(theme);
                self.prefs.theme = Some(theme);
            }
            settings.ui_scale_percent = appearance::clamp_scale(percent);
            settings.font_size_pt = appearance::clamp_font(points);
            self.prefs.ui_scale_percent = Some(settings.ui_scale_percent);
            self.prefs.font_size_pt = Some(settings.font_size_pt);
            self.saver.mark();
            apply_look(ctx, settings);
            if let Some(w) = workspace {
                w.viewer.settings = settings.clone();
                if theme_changed {
                    appearance::recolour(&mut w.viewer.scene, settings);
                }
            }
        }
    }

    fn shortcuts_window(&mut self, ctx: &egui::Context, settings: &ViewerSettings, lang: Lang) {
        if !self.shortcuts_open {
            return;
        }
        let mut open = true;
        egui::Window::new(WinText::ShortcutsTitle.tr(lang))
            .open(&mut open)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(dim(WinText::ShortcutsHowTo.tr(lang)));
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        egui::Grid::new("e8_shortcuts_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                for (label, keys) in settings.keymap.rows(lang) {
                                    ui.label(label);
                                    ui.monospace(keys);
                                    ui.end_row();
                                }
                            });
                    });
            });
        self.shortcuts_open = open;
    }

    fn help_window(&mut self, ctx: &egui::Context, lang: Lang, mode: HelpMode) {
        if !self.help_open {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(WinText::HelpTitle.tr(lang))
            .open(&mut open)
            .default_width(420.0)
            .show(ctx, |ui| {
                for line in mode.lines(lang) {
                    ui.label(line);
                    ui.add_space(4.0);
                }
                ui.label(dim(WinText::HelpHint.tr(lang)));
                if ui.button(WinText::Close.tr(lang)).clicked() {
                    close = true;
                }
            });
        self.help_open = open && !close;
    }

    /// Keeps the zoom and the layers of the mode, the size of the window and the choices of the
    /// person in the preferences, and writes the file when something changed and the time between
    /// two writings has passed.
    fn remember(
        &mut self,
        ctx: &egui::Context,
        settings: &ViewerSettings,
        mode: HelpMode,
        fit_pending: bool,
        workspace: Option<&Workspace>,
    ) {
        let key = match mode {
            HelpMode::Board => Some("board"),
            HelpMode::Schematic => Some("schematic"),
            _ => None,
        };
        if let (Some(key), Some(w), false) = (key, workspace, fit_pending) {
            if self.restore_zoom.is_none() {
                let scale = w.viewer.view.scale;
                if self.prefs.note_zoom(key, scale, settings.zoom_note_step) {
                    self.saver.mark();
                }
                let layers = layer_presets::capture(&w.viewer.scene, &self.order, settings);
                if self.prefs.note_layers(key, layers) {
                    self.saver.mark();
                }
            }
        }
        let zoom = ctx.zoom_factor();
        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
            let size = (
                (rect.width() * zoom).round().clamp(200.0, 16_000.0) as u32,
                (rect.height() * zoom).round().clamp(200.0, 16_000.0) as u32,
            );
            if self.prefs.window != Some(size) {
                self.prefs.window = Some(size);
                self.saver.mark();
            }
        }
        let now = ctx.input(|i| i.time);
        if self.saver.due(now, settings.prefs_save_interval_s) {
            if let Some(path) = &self.path {
                let retries = u32::try_from(settings.prefs_write_retries).unwrap_or(u32::MAX);
                match prefs::save(path, &self.prefs, retries, settings.prefs_write_retry_ms) {
                    Ok(()) => self.error = None,
                    Err(e) => self.error = Some((WinText::PrefsSaveFailed, e)),
                }
            }
        }
    }
}

fn empty_prefs(settings: &ViewerSettings) -> Prefs {
    Prefs::parse("", settings.preset_max).unwrap_or_default()
}
