// SPDX-License-Identifier: GPL-3.0-or-later
//! The window of the library mode (built with the `gui` feature): the lists of footprints, symbols
//! and device sets, the editor of a footprint or a symbol with a fine grid, snapping and
//! measures, the editor of the properties of an item, the editor of a device set, the checks of
//! the library, the generator of footprints and the plan to replace by a master. Everything that
//! can be decided without a screen is decided in [`LibrarySession`], in code that is tested; this
//! file only draws it and passes on what the person does. It has not been seen on a screen.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use eh3d_phoenix_libedit::codes::{title, LibError};
use eh3d_phoenix_libedit::footprints::{FootprintConfig, FAMILIES};
use eh3d_phoenix_libedit::geom::Shape;
use eh3d_phoenix_libedit::model::{DeviceSet, Item, Kind, PadShape};
use eh3d_phoenix_libedit::ops::{OpeningKind, Operation};
use eh3d_phoenix_libedit::{LibSettings, Severity};
use eh3d_phoenix_rules::Lang;

use crate::editmsg::EditMsg;
use crate::gui::{ui_accent, ui_danger, ui_dim, ui_ok, ui_text};
use crate::library_session::{LibShape, LibTool, LibView, LibrarySession};
use crate::library_text::LibText;
use crate::msg::Msg;

// the colours of the theme Forge, the same as the rest of the window
const BACKGROUND: Color32 = Color32::from_rgb(0x0f, 0x0c, 0x0a);
const GRID: Color32 = Color32::from_rgb(0x3a, 0x30, 0x27);
const AXES: Color32 = Color32::from_rgb(0x6a, 0x5a, 0x48);
const MEASURE: Color32 = Color32::from_rgb(0x4a, 0xc8, 0xff);

/// The most findings the panel lists.
const MAX_FINDINGS_SHOWN: usize = 300;
/// The most points of the grid drawn at once.
const MAX_GRID_POINTS: i64 = 40_000;

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ui_accent())
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ui_dim())
}

/// The colour a layer is drawn in.
fn layer_colour(layer: u32) -> Color32 {
    match layer {
        1 => Color32::from_rgb(0xd8, 0x4a, 0x4a),
        16 => Color32::from_rgb(0x4a, 0x7a, 0xd8),
        21 | 22 => Color32::from_rgb(0xf4, 0xec, 0xe0),
        25 | 27 => Color32::from_rgb(0x9b, 0xe3, 0x4a),
        29 | 30 => Color32::from_rgba_unmultiplied(0x30, 0xc0, 0xa8, 110),
        31 | 32 => Color32::from_rgba_unmultiplied(0xd8, 0xb0, 0x40, 110),
        39 => Color32::from_rgb(0xc8, 0x50, 0xc8),
        51 => Color32::from_rgb(0x90, 0x90, 0x80),
        94 => Color32::from_rgb(0xff, 0xb0, 0x00),
        95 | 96 => Color32::from_rgb(0x9b, 0xe3, 0x4a),
        _ => Color32::from_rgb(0xa0, 0xa0, 0xa0),
    }
}

/// What the panels keep between two frames.
#[derive(Default)]
struct UiState {
    new_name: String,
    rename_text: String,
    save_text: String,
    plan_text: String,
    master_text: String,
    confirm: bool,
    family: usize,
    params_text: String,
    /// The item being edited: its place, the item as the library has it and the edited copy.
    draft: Option<(usize, Item, Item)>,
    mask_mm: f64,
    paste_mm: f64,
    device: usize,
    variant_name: String,
    prefix_text: String,
    prefix_for: String,
    gate_symbol: usize,
    variant_package: usize,
}

/// The library mode of the window.
pub struct LibraryUi {
    /// The library being edited, once one is open.
    pub session: Option<LibrarySession>,
    settings: LibSettings,
    footprints: FootprintConfig,
    state: UiState,
    tool_drag: bool,
    error: Option<LibError>,
}

impl LibraryUi {
    /// The library mode with the settings of the editor.
    pub fn new(settings: LibSettings) -> LibraryUi {
        let state = UiState {
            mask_mm: settings.mask_expansion_nm as f64 / 1e6,
            paste_mm: settings.paste_expansion_nm as f64 / 1e6,
            ..UiState::default()
        };
        LibraryUi {
            session: None,
            settings,
            footprints: FootprintConfig::builtin(),
            state,
            tool_drag: false,
            error: None,
        }
    }

    /// True while a library is open.
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    /// Reads a library file and shows it; a file that cannot be used leaves the mode as it was.
    pub fn open(&mut self, path: &Path) {
        match LibrarySession::open(path, self.settings.clone()) {
            Ok(session) => {
                self.state.save_text = suggested_path(path).to_string_lossy().into_owned();
                self.session = Some(session);
                self.error = None;
                self.tool_drag = false;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// Starts a new library with nothing in it.
    pub fn new_library(&mut self) {
        self.state.save_text = "library_new.lbr".to_owned();
        self.session = Some(LibrarySession::empty(self.settings.clone()));
        self.error = None;
        self.tool_drag = false;
    }

    /// Leaves the library mode; what was not saved is lost.
    pub fn close(&mut self) {
        self.session = None;
        self.tool_drag = false;
    }

    /// Takes the error of the last attempt to open a file.
    pub fn take_error(&mut self) -> Option<LibError> {
        self.error.take()
    }

    /// Draws the library mode in the panels and the canvas of the window.
    pub fn show(&mut self, ui: &mut egui::Ui, lang: Lang) {
        let LibraryUi {
            session,
            footprints,
            state,
            tool_drag,
            error,
            ..
        } = self;
        let Some(s) = session.as_mut() else {
            return;
        };
        left_panel(ui, s, state, footprints, lang);
        right_panel(ui, s, state, lang);
        bottom_panel(ui, s, error.as_ref(), lang);
        canvas(ui, s, tool_drag, lang);
    }
}

/// The name of a new file next to the library, with `_edit` added.
fn suggested_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "library".to_owned());
    path.with_file_name(format!("{stem}_edit.lbr"))
}

fn kind_text(kind: Kind) -> LibText {
    match kind {
        Kind::Package => LibText::Footprints,
        Kind::Symbol => LibText::Symbols,
        Kind::DeviceSet => LibText::Devices,
    }
}

fn kind_of(word: &str) -> Kind {
    match word {
        "package" => Kind::Package,
        "symbol" => Kind::Symbol,
        _ => Kind::DeviceSet,
    }
}

fn left_panel(
    ui: &mut egui::Ui,
    s: &mut LibrarySession,
    state: &mut UiState,
    footprints: &FootprintConfig,
    lang: Lang,
) {
    egui::Panel::left("library_lists")
        .default_size(290.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                ui.label(heading(LibText::Title.tr(lang)));
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(s.can_undo(), |ui| {
                        if ui.button(EditMsg::Undo.tr(lang)).clicked() {
                            s.undo();
                        }
                    });
                    ui.add_enabled_ui(s.can_redo(), |ui| {
                        if ui.button(EditMsg::Redo.tr(lang)).clicked() {
                            s.redo();
                        }
                    });
                    if s.dirty() {
                        ui.label(egui::RichText::new(LibText::Unsaved.tr(lang)).color(ui_danger()));
                    }
                });
                ui.separator();
                let mut kind = s.kind;
                ui.horizontal_wrapped(|ui| {
                    for k in [Kind::Package, Kind::Symbol, Kind::DeviceSet] {
                        ui.selectable_value(&mut kind, k, kind_text(k).tr(lang));
                    }
                });
                if kind != s.kind {
                    s.select_entity(kind, None);
                }
                names_list(ui, s);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.new_name).desired_width(120.0));
                    if ui.button(LibText::New.tr(lang)).clicked() {
                        let name = state.new_name.clone();
                        if s.create_entity(s.kind, &name) {
                            state.new_name.clear();
                        }
                    }
                });
                if s.name.is_some() {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.rename_text).desired_width(120.0),
                        );
                        if ui.button(LibText::Rename.tr(lang)).clicked() {
                            let to = state.rename_text.clone();
                            s.rename_entity(&to);
                        }
                        if ui.button(LibText::Delete.tr(lang)).clicked() {
                            s.delete_entity();
                        }
                    });
                }
                ui.separator();
                tools_section(ui, s, lang);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(LibText::RunCheck.tr(lang)).clicked() {
                        s.check();
                    }
                });
                ui.separator();
                save_section(ui, s, state, lang);
                ui.separator();
                ui.collapsing(LibText::Generator.tr(lang), |ui| {
                    generator_section(ui, s, state, footprints, lang);
                });
                ui.collapsing(LibText::Plan.tr(lang), |ui| {
                    plan_section(ui, s, state, lang);
                });
            });
        });
}

/// The list of the names of the kind that is shown, as rows of a scroll area.
fn names_list(ui: &mut egui::Ui, s: &mut LibrarySession) {
    let names = s.names(s.kind);
    let mut chosen = s.name.clone();
    let row = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
    egui::ScrollArea::vertical()
        .id_salt("library_names")
        .max_height(220.0)
        .show_rows(ui, row, names.len(), |ui, range| {
            for name in &names[range] {
                ui.selectable_value(&mut chosen, Some(name.clone()), name.as_str());
            }
        });
    if chosen != s.name {
        s.select_entity(s.kind, chosen);
    }
}

fn tools_section(ui: &mut egui::Ui, s: &mut LibrarySession, lang: Lang) {
    if s.kind == Kind::DeviceSet {
        return;
    }
    let mut tool = s.tool;
    ui.horizontal_wrapped(|ui| {
        for t in LibTool::ALL {
            let fits = if s.kind == Kind::Symbol {
                t.fits_symbol()
            } else {
                t.fits_package()
            };
            if fits {
                ui.selectable_value(&mut tool, t, t.text().tr(lang));
            }
        }
    });
    if tool != s.tool {
        s.tool = tool;
        s.cancel();
    }
    ui.horizontal(|ui| {
        ui.label(dim(LibText::Grid.tr(lang)));
        let mut grid_mm = s.grid_nm as f64 / 1e6;
        if ui
            .add(
                egui::DragValue::new(&mut grid_mm)
                    .speed(0.005)
                    .range(0.001..=10.0)
                    .fixed_decimals(3),
            )
            .changed()
        {
            s.grid_nm = (grid_mm * 1e6).round() as i64;
        }
        ui.checkbox(&mut s.snap, LibText::Snap.tr(lang));
    });
    ui.label(dim(LibText::Shortcuts.tr(lang)));
}

fn save_section(ui: &mut egui::Ui, s: &mut LibrarySession, state: &mut UiState, lang: Lang) {
    ui.label(dim(LibText::SavePath.tr(lang)));
    ui.add(egui::TextEdit::singleline(&mut state.save_text).desired_width(250.0));
    if ui.button(LibText::SaveAsNew.tr(lang)).clicked() {
        let target = PathBuf::from(state.save_text.trim());
        s.save_as(&target);
    }
}

/// The parameters typed as `name=value name=value`.
fn parse_params(text: &str) -> BTreeMap<String, String> {
    text.split_whitespace()
        .filter_map(|word| word.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

fn generator_section(
    ui: &mut egui::Ui,
    s: &mut LibrarySession,
    state: &mut UiState,
    footprints: &FootprintConfig,
    lang: Lang,
) {
    let family = FAMILIES[state.family.min(FAMILIES.len() - 1)];
    ui.horizontal(|ui| {
        ui.label(dim(LibText::Family.tr(lang)));
        egui::ComboBox::from_id_salt("library_family")
            .selected_text(family)
            .show_ui(ui, |ui| {
                for (i, f) in FAMILIES.iter().enumerate() {
                    ui.selectable_value(&mut state.family, i, *f);
                }
            });
    });
    ui.label(dim(LibText::Parameters.tr(lang)));
    ui.add(egui::TextEdit::singleline(&mut state.params_text).desired_width(250.0));
    if ui.button(LibText::AddToLibrary.tr(lang)).clicked() {
        let params = parse_params(&state.params_text);
        s.add_generated(family, &params, footprints);
    }
    ui.label(dim(LibText::Prototype.tr(lang)));
}

fn plan_section(ui: &mut egui::Ui, s: &mut LibrarySession, state: &mut UiState, lang: Lang) {
    ui.label(dim(LibText::PlanNotApplied.tr(lang)));
    ui.label(dim(LibText::PlanFile.tr(lang)));
    ui.add(egui::TextEdit::singleline(&mut state.plan_text).desired_width(250.0));
    ui.label(dim(LibText::MasterFolder.tr(lang)));
    ui.add(egui::TextEdit::singleline(&mut state.master_text).desired_width(250.0));
    if ui.button(LibText::LoadPlan.tr(lang)).clicked() {
        let path = PathBuf::from(state.plan_text.trim());
        s.load_plan(&path);
    }
    ui.checkbox(&mut state.confirm, LibText::Confirm.tr(lang));
    let steps = s.plan.as_ref().map(|p| p.steps.clone()).unwrap_or_default();
    let mut replace: Option<usize> = None;
    for (i, step) in steps.iter().enumerate() {
        ui.push_id(("library_step", i), |ui| {
            ui.horizontal_wrapped(|ui| {
                let text = format!(
                    "{} {} -> {} ({}, {})",
                    step.kind, step.name, step.master_name, step.master_library, step.score
                );
                ui.add(egui::Label::new(egui::RichText::new(text)).wrap());
                let usable = state.confirm && !step.turned;
                if ui
                    .add_enabled(usable, egui::Button::new(LibText::Replace.tr(lang)))
                    .clicked()
                {
                    replace = Some(i);
                }
            });
        });
    }
    if let Some(i) = replace {
        let folder = PathBuf::from(state.master_text.trim());
        s.replace_with_master(i, &folder, state.confirm);
    }
}

fn right_panel(ui: &mut egui::Ui, s: &mut LibrarySession, state: &mut UiState, lang: Lang) {
    egui::Panel::right("library_properties")
        .default_size(340.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                if s.kind == Kind::DeviceSet {
                    deviceset_editor(ui, s, state, lang);
                } else {
                    properties(ui, s, state, lang);
                }
                ui.separator();
                findings(ui, s, lang);
            });
        });
}

/// A length in millimetres, shown with three decimals and kept in nanometres.
fn nm_row(ui: &mut egui::Ui, label: &str, value: &mut i64) -> bool {
    let mut mm = *value as f64 / 1e6;
    let changed = ui
        .horizontal(|ui| {
            ui.label(dim(label));
            ui.add(egui::DragValue::new(&mut mm).speed(0.01).fixed_decimals(3))
                .changed()
        })
        .inner;
    if changed {
        *value = (mm * 1e6).round() as i64;
    }
    changed
}

fn text_row(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    ui.horizontal(|ui| {
        ui.label(dim(label));
        ui.add(egui::TextEdit::singleline(value).desired_width(150.0))
            .changed()
    })
    .inner
}

fn number_row(ui: &mut egui::Ui, label: &str, value: &mut u32, max: u32) -> bool {
    ui.horizontal(|ui| {
        ui.label(dim(label));
        ui.add(egui::DragValue::new(value).range(0..=max)).changed()
    })
    .inner
}

fn rotation_row(ui: &mut egui::Ui, label: &str, id: &str, rot: &mut String) {
    ui.horizontal(|ui| {
        ui.label(dim(label));
        let shown = if rot.is_empty() {
            "0°".to_owned()
        } else {
            rot.clone()
        };
        egui::ComboBox::from_id_salt(id)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (value, text) in [
                    ("", "0°"),
                    ("R90", "90°"),
                    ("R180", "180°"),
                    ("R270", "270°"),
                ] {
                    ui.selectable_value(rot, value.to_owned(), text);
                }
            });
    });
}

fn keyword_row(ui: &mut egui::Ui, label: &str, id: &str, value: &mut String, words: &[&str]) {
    ui.horizontal(|ui| {
        ui.label(dim(label));
        egui::ComboBox::from_id_salt(id)
            .selected_text(value.clone())
            .show_ui(ui, |ui| {
                for w in words {
                    ui.selectable_value(value, (*w).to_owned(), *w);
                }
            });
    });
}

/// The fields of an item. Returns true when one changed.
fn edit_item(ui: &mut egui::Ui, item: &mut Item, lang: Lang) -> bool {
    let mut changed = false;
    match item {
        Item::Pad(p) => {
            changed |= text_row(ui, LibText::Name.tr(lang), &mut p.name);
            changed |= nm_row(ui, "X", &mut p.x);
            changed |= nm_row(ui, "Y", &mut p.y);
            changed |= nm_row(ui, LibText::Drill.tr(lang), &mut p.drill);
            changed |= nm_row(ui, LibText::Diameter.tr(lang), &mut p.diameter);
            ui.horizontal(|ui| {
                ui.label(dim(LibText::Shape.tr(lang)));
                egui::ComboBox::from_id_salt("library_pad_shape")
                    .selected_text(p.shape.keyword())
                    .show_ui(ui, |ui| {
                        for shape in [
                            PadShape::Round,
                            PadShape::Square,
                            PadShape::Octagon,
                            PadShape::Long,
                            PadShape::Offset,
                        ] {
                            changed |= ui
                                .selectable_value(&mut p.shape, shape, shape.keyword())
                                .changed();
                        }
                    });
            });
            rotation_row(
                ui,
                LibText::Rotation.tr(lang),
                "library_pad_rot",
                &mut p.rot,
            );
        }
        Item::Smd(p) => {
            changed |= text_row(ui, LibText::Name.tr(lang), &mut p.name);
            changed |= nm_row(ui, "X", &mut p.x);
            changed |= nm_row(ui, "Y", &mut p.y);
            changed |= nm_row(ui, LibText::Width.tr(lang), &mut p.dx);
            changed |= nm_row(ui, LibText::Height.tr(lang), &mut p.dy);
            changed |= number_row(ui, LibText::Layer.tr(lang), &mut p.layer, 255);
            changed |= number_row(ui, LibText::Roundness.tr(lang), &mut p.roundness, 100);
            rotation_row(
                ui,
                LibText::Rotation.tr(lang),
                "library_smd_rot",
                &mut p.rot,
            );
        }
        Item::Wire(w) => {
            changed |= nm_row(ui, "X1", &mut w.x1);
            changed |= nm_row(ui, "Y1", &mut w.y1);
            changed |= nm_row(ui, "X2", &mut w.x2);
            changed |= nm_row(ui, "Y2", &mut w.y2);
            changed |= nm_row(ui, LibText::LineWidth.tr(lang), &mut w.width);
            changed |= number_row(ui, LibText::Layer.tr(lang), &mut w.layer, 255);
            let mut degrees = w.curve as f64 / 1e6;
            let moved = ui
                .horizontal(|ui| {
                    ui.label(dim(LibText::ArcAngle.tr(lang)));
                    ui.add(
                        egui::DragValue::new(&mut degrees)
                            .speed(1.0)
                            .range(-359.0..=359.0)
                            .fixed_decimals(1),
                    )
                    .changed()
                })
                .inner;
            if moved {
                w.curve = (degrees * 1e6).round() as i64;
                changed = true;
            }
        }
        Item::Rect(r) => {
            changed |= nm_row(ui, "X1", &mut r.x1);
            changed |= nm_row(ui, "Y1", &mut r.y1);
            changed |= nm_row(ui, "X2", &mut r.x2);
            changed |= nm_row(ui, "Y2", &mut r.y2);
            changed |= number_row(ui, LibText::Layer.tr(lang), &mut r.layer, 255);
            rotation_row(
                ui,
                LibText::Rotation.tr(lang),
                "library_rect_rot",
                &mut r.rot,
            );
        }
        Item::Circle(c) => {
            changed |= nm_row(ui, "X", &mut c.x);
            changed |= nm_row(ui, "Y", &mut c.y);
            changed |= nm_row(ui, LibText::Radius.tr(lang), &mut c.radius);
            changed |= nm_row(ui, LibText::LineWidth.tr(lang), &mut c.width);
            changed |= number_row(ui, LibText::Layer.tr(lang), &mut c.layer, 255);
        }
        Item::Text(t) => {
            changed |= text_row(ui, LibText::ToolText.tr(lang), &mut t.content);
            changed |= nm_row(ui, "X", &mut t.x);
            changed |= nm_row(ui, "Y", &mut t.y);
            changed |= nm_row(ui, LibText::TextHeight.tr(lang), &mut t.size);
            changed |= number_row(ui, LibText::Layer.tr(lang), &mut t.layer, 255);
            rotation_row(
                ui,
                LibText::Rotation.tr(lang),
                "library_text_rot",
                &mut t.rot,
            );
        }
        Item::Hole(h) => {
            changed |= nm_row(ui, "X", &mut h.x);
            changed |= nm_row(ui, "Y", &mut h.y);
            changed |= nm_row(ui, LibText::Drill.tr(lang), &mut h.drill);
        }
        Item::Pin(p) => {
            text_row(ui, LibText::Name.tr(lang), &mut p.name);
            nm_row(ui, "X", &mut p.x);
            nm_row(ui, "Y", &mut p.y);
            keyword_row(
                ui,
                LibText::ElectricalType.tr(lang),
                "library_pin_direction",
                &mut p.direction,
                &["nc", "in", "out", "io", "oc", "pwr", "pas", "hiz", "sup"],
            );
            keyword_row(
                ui,
                LibText::PinLength.tr(lang),
                "library_pin_length",
                &mut p.length,
                &["point", "short", "middle", "long"],
            );
            rotation_row(
                ui,
                &format!(
                    "{} ({})",
                    LibText::Rotation.tr(lang),
                    LibText::ToolPin.tr(lang)
                ),
                "library_pin_rot",
                &mut p.rot,
            );
            changed = true;
        }
        Item::Other(_) => {
            ui.label(dim("..."));
        }
    }
    changed
}

/// The properties of the item selected, the openings of a pad and the origin.
fn properties(ui: &mut egui::Ui, s: &mut LibrarySession, state: &mut UiState, lang: Lang) {
    ui.label(heading(LibText::Properties.tr(lang)));
    let current = s.selected_item().cloned();
    match (s.selected, current) {
        (Some(index), Some(item)) => {
            let reset = match &state.draft {
                Some((i, base, _)) => *i != index || *base != item,
                None => true,
            };
            if reset {
                state.draft = Some((index, item.clone(), item.clone()));
            }
            let mut apply: Option<Item> = None;
            if let Some((_, _, edited)) = state.draft.as_mut() {
                ui.label(dim(format!("#{index} · {}", edited.kind())));
                let _ = edit_item(ui, edited, lang);
                if ui.button(LibText::Apply.tr(lang)).clicked() {
                    apply = Some(edited.clone());
                }
            }
            if let Some(new_item) = apply {
                s.replace_selected(new_item);
            }
            if item.is_pad() {
                openings(ui, s, state, lang);
            }
        }
        _ => {
            state.draft = None;
        }
    }
    if s.name.is_some() {
        if let Some((x, y)) = s.cursor {
            if ui.button(LibText::OriginToCursor.tr(lang)).clicked() {
                s.origin_to(x, y);
            }
        }
    }
}

fn openings(ui: &mut egui::Ui, s: &mut LibrarySession, state: &mut UiState, lang: Lang) {
    ui.separator();
    for (kind, label, value) in [
        (OpeningKind::Mask, LibText::MaskOpening, 0usize),
        (OpeningKind::Paste, LibText::PasteOpening, 1usize),
    ] {
        ui.horizontal(|ui| {
            ui.label(dim(label.tr(lang)));
            let expansion = if value == 0 {
                &mut state.mask_mm
            } else {
                &mut state.paste_mm
            };
            ui.add(
                egui::DragValue::new(expansion)
                    .speed(0.005)
                    .range(-5.0..=5.0)
                    .fixed_decimals(3),
            );
            let nm = (*expansion * 1e6).round() as i64;
            if ui.small_button(LibText::Set.tr(lang)).clicked() {
                s.set_opening(kind, Some(nm));
            }
            if ui.small_button(LibText::Automatic.tr(lang)).clicked() {
                s.set_opening(kind, None);
            }
        });
    }
    ui.label(dim(LibText::Expansion.tr(lang)));
}

/// The gates, the variants and the connections of a device set.
fn deviceset_editor(ui: &mut egui::Ui, s: &mut LibrarySession, state: &mut UiState, lang: Lang) {
    let Some(name) = s.name.clone() else {
        return;
    };
    let Some(set) = s.library().deviceset(&name).cloned() else {
        return;
    };
    ui.label(heading(&format!("{} {}", LibText::Devices.tr(lang), name)));
    let mut ops: Vec<Operation> = Vec::new();
    if state.prefix_for != name {
        state.prefix_for = name.clone();
        state.prefix_text = set.prefix.clone();
    }
    ui.horizontal(|ui| {
        ui.label(dim(LibText::Prefix.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut state.prefix_text).desired_width(60.0));
        if ui.small_button(LibText::Apply.tr(lang)).clicked() {
            ops.push(Operation::SetPrefix {
                set: name.clone(),
                prefix: state.prefix_text.clone(),
            });
        }
    });
    ui.separator();
    gates_section(ui, s, state, &set, &name, &mut ops, lang);
    ui.separator();
    variants_section(ui, s, state, &set, &name, &mut ops, lang);
    if !ops.is_empty() {
        s.apply(ops);
    }
}

fn gates_section(
    ui: &mut egui::Ui,
    s: &LibrarySession,
    state: &mut UiState,
    set: &DeviceSet,
    name: &str,
    ops: &mut Vec<Operation>,
    lang: Lang,
) {
    ui.label(heading(LibText::Gates.tr(lang)));
    for gate in &set.gates {
        ui.horizontal(|ui| {
            ui.label(format!("{}: {}", gate.name, gate.symbol));
            if ui.small_button(LibText::Delete.tr(lang)).clicked() {
                ops.push(Operation::RemoveGate {
                    set: name.to_owned(),
                    name: gate.name.clone(),
                });
            }
        });
    }
    let symbols = s.names(Kind::Symbol);
    if symbols.is_empty() {
        return;
    }
    state.gate_symbol = state.gate_symbol.min(symbols.len() - 1);
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("library_gate_symbol")
            .selected_text(symbols[state.gate_symbol].as_str())
            .show_ui(ui, |ui| {
                for (i, symbol) in symbols.iter().enumerate() {
                    ui.selectable_value(&mut state.gate_symbol, i, symbol.as_str());
                }
            });
        if ui.small_button(LibText::AddGate.tr(lang)).clicked() {
            let mut n = set.gates.len() + 1;
            while set.gates.iter().any(|g| g.name == format!("G${n}")) {
                n += 1;
            }
            ops.push(Operation::AddGate {
                set: name.to_owned(),
                name: format!("G${n}"),
                symbol: symbols[state.gate_symbol].clone(),
                x: 0,
                y: 0,
            });
        }
    });
}

fn variants_section(
    ui: &mut egui::Ui,
    s: &LibrarySession,
    state: &mut UiState,
    set: &DeviceSet,
    name: &str,
    ops: &mut Vec<Operation>,
    lang: Lang,
) {
    ui.label(heading(LibText::Variants.tr(lang)));
    let packages = s.names(Kind::Package);
    for (i, device) in set.devices.iter().enumerate() {
        ui.horizontal(|ui| {
            let label = format!(
                "{:?} · {}",
                device.name,
                device.package.as_deref().unwrap_or("-")
            );
            ui.selectable_value(&mut state.device, i, label);
            if ui.small_button(LibText::Delete.tr(lang)).clicked() {
                ops.push(Operation::RemoveDevice {
                    set: name.to_owned(),
                    name: device.name.clone(),
                });
            }
        });
    }
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.variant_name).desired_width(70.0));
        if !packages.is_empty() {
            state.variant_package = state.variant_package.min(packages.len() - 1);
            egui::ComboBox::from_id_salt("library_variant_package")
                .selected_text(packages[state.variant_package].as_str())
                .show_ui(ui, |ui| {
                    for (i, p) in packages.iter().enumerate() {
                        ui.selectable_value(&mut state.variant_package, i, p.as_str());
                    }
                });
        }
        if ui.small_button(LibText::AddVariant.tr(lang)).clicked() {
            ops.push(Operation::AddDevice {
                set: name.to_owned(),
                name: state.variant_name.clone(),
                package: packages.get(state.variant_package).cloned(),
            });
        }
    });
    let Some(device) = set.devices.get(state.device) else {
        return;
    };
    ui.separator();
    ui.label(heading(LibText::PinToPad.tr(lang)));
    let package_name = device.package.clone();
    // the footprint of the selected variant, to link it to another one
    ui.horizontal(|ui| {
        ui.label(dim(LibText::Footprint.tr(lang)));
        egui::ComboBox::from_id_salt("library_link_package")
            .selected_text(package_name.clone().unwrap_or_else(|| "-".to_owned()))
            .show_ui(ui, |ui| {
                let mut chosen = package_name.clone();
                if ui.selectable_value(&mut chosen, None, "-").clicked() {
                    ops.push(Operation::LinkDevice {
                        set: name.to_owned(),
                        device: device.name.clone(),
                        package: None,
                    });
                }
                for p in &packages {
                    if ui
                        .selectable_value(&mut chosen, Some(p.clone()), p.as_str())
                        .clicked()
                    {
                        ops.push(Operation::LinkDevice {
                            set: name.to_owned(),
                            device: device.name.clone(),
                            package: Some(p.clone()),
                        });
                    }
                }
            });
    });
    let Some(package) = package_name.as_deref().and_then(|p| s.library().package(p)) else {
        return;
    };
    let pads: Vec<String> = package
        .pads()
        .filter_map(|(_, i)| i.name().map(str::to_owned))
        .collect();
    if ui.button(LibText::AutoConnect.tr(lang)).clicked() {
        ops.push(Operation::AutoConnect {
            set: name.to_owned(),
            device: device.name.clone(),
        });
    }
    for gate in &set.gates {
        let Some(symbol) = s.library().symbol(&gate.symbol) else {
            continue;
        };
        for (_, pin) in symbol.pins() {
            let current = device
                .connects
                .iter()
                .find(|c| c.gate == gate.name && c.pin == pin.name)
                .map(|c| c.pad.clone());
            ui.push_id(("library_connect", &gate.name, &pin.name), |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{}.{}", gate.name, pin.name));
                    let shown = current
                        .clone()
                        .unwrap_or_else(|| LibText::NoPad.tr(lang).to_owned());
                    egui::ComboBox::from_id_salt("pad")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            let mut chosen = current.clone();
                            if ui
                                .selectable_value(&mut chosen, None, LibText::NoPad.tr(lang))
                                .clicked()
                                && current.is_some()
                            {
                                ops.push(Operation::Disconnect {
                                    set: name.to_owned(),
                                    device: device.name.clone(),
                                    gate: gate.name.clone(),
                                    pin: pin.name.clone(),
                                });
                            }
                            for pad in &pads {
                                if ui
                                    .selectable_value(&mut chosen, Some(pad.clone()), pad.as_str())
                                    .clicked()
                                {
                                    ops.push(Operation::Connect {
                                        set: name.to_owned(),
                                        device: device.name.clone(),
                                        gate: gate.name.clone(),
                                        pin: pin.name.clone(),
                                        pad: pad.clone(),
                                    });
                                }
                            }
                        });
                });
            });
        }
    }
}

fn findings(ui: &mut egui::Ui, s: &mut LibrarySession, lang: Lang) {
    ui.label(heading(LibText::Findings.tr(lang)));
    if s.report.findings.is_empty() {
        ui.label(egui::RichText::new(LibText::NoFindings.tr(lang)).color(ui_ok()));
        return;
    }
    ui.label(dim(format!(
        "{} / {}",
        s.report.errors(),
        s.report.warnings()
    )));
    let mut go: Option<(Kind, String)> = None;
    for (i, f) in s
        .report
        .findings
        .iter()
        .take(MAX_FINDINGS_SHOWN)
        .enumerate()
    {
        ui.push_id(("library_finding", i), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.small_button(EditMsg::GoTo.tr(lang)).clicked() {
                    go = Some((kind_of(f.kind), f.name.clone()));
                }
                let colour = if f.severity == Severity::Error {
                    ui_danger()
                } else {
                    ui_accent()
                };
                let mut line = format!("{} {}: {}", f.code, f.name, title(f.code, lang));
                if !f.detail.is_empty() {
                    line.push_str(&format!(" ({})", f.detail));
                }
                ui.add(egui::Label::new(egui::RichText::new(line).color(colour)).wrap());
            });
        });
    }
    if let Some((kind, name)) = go {
        s.select_entity(kind, Some(name));
    }
}

fn bottom_panel(ui: &mut egui::Ui, s: &LibrarySession, error: Option<&LibError>, lang: Lang) {
    egui::Panel::bottom("library_status").show(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(e) = error {
                ui.label(egui::RichText::new(e.message(lang)).color(ui_danger()));
            } else if let Some(note) = &s.note {
                let colour = if note.is_error() {
                    ui_danger()
                } else {
                    ui_accent()
                };
                ui.label(egui::RichText::new(note.text(lang)).color(colour));
            }
            if let Some((x, y)) = s.cursor {
                ui.label(dim(format!(
                    "{:.3}, {:.3} mm",
                    x as f64 / 1e6,
                    y as f64 / 1e6
                )));
            }
            if let Some(d) = s.measure_distance() {
                ui.label(
                    egui::RichText::new(format!("{}: {d:.3} mm", Msg::Distance.tr(lang)))
                        .color(MEASURE),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(LibText::Title.tr(lang)).color(ui_accent()));
            });
        });
    });
}

fn pos(view: &LibView, rect: Rect, x: f64, y: f64) -> Pos2 {
    let (sx, sy) = view.to_screen(x, y);
    Pos2::new(rect.min.x + sx as f32, rect.min.y + sy as f32)
}

/// Fills a convex polygon with a colour.
fn fill_convex(painter: &egui::Painter, points: &[Pos2], colour: Color32) {
    if points.len() < 3 {
        return;
    }
    let mut mesh = egui::epaint::Mesh::default();
    for p in points {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *p,
            uv: egui::epaint::WHITE_UV,
            color: colour,
        });
    }
    for i in 1..points.len() - 1 {
        mesh.add_triangle(0, i as u32, i as u32 + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn canvas(ui: &mut egui::Ui, s: &mut LibrarySession, tool_drag: &mut bool, lang: Lang) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, BACKGROUND);
        s.view.width = f64::from(rect.width());
        s.view.height = f64::from(rect.height());
        if s.fit_pending && rect.width() > 40.0 && rect.height() > 40.0 {
            let bounds = s.bounds();
            s.view.fit(bounds);
            s.fit_pending = false;
        }
        if s.name.is_none() || s.kind == Kind::DeviceSet {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                LibText::NoteNoEntity.tr(lang),
                egui::FontId::proportional(18.0),
                ui_dim(),
            );
            return;
        }
        let local = |p: Pos2| (f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y));
        if response.drag_started() {
            if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
                let (sx, sy) = local(p);
                let (x, y) = s.world(sx, sy);
                *tool_drag = s.press(x, y);
            }
        }
        if response.dragged() {
            if *tool_drag {
                if let Some(p) = response.interact_pointer_pos() {
                    let (sx, sy) = local(p);
                    let (x, y) = s.world(sx, sy);
                    s.drag_to(x, y);
                }
            } else {
                let d = response.drag_delta();
                s.view.pan_px(f64::from(d.x), f64::from(d.y));
            }
        }
        if response.drag_stopped() && *tool_drag {
            let end = response
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.latest_pos()));
            match end {
                Some(p) => {
                    let (sx, sy) = local(p);
                    let (x, y) = s.world(sx, sy);
                    s.release(x, y);
                }
                None => s.drag = None,
            }
            *tool_drag = false;
        }
        if let Some(p) = response.hover_pos() {
            let (sx, sy) = local(p);
            let (x, y) = s.world(sx, sy);
            s.cursor = Some(s.snapped(x, y));
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            if scroll.abs() > f32::EPSILON || (pinch - 1.0).abs() > 1e-6 {
                s.view.zoom_at(sx, sy, f64::from(scroll), f64::from(pinch));
            }
        }
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                let (sx, sy) = local(p);
                let (x, y) = s.world(sx, sy);
                s.click(x, y);
            }
        }
        keys(ui, s);
        paint(s, &painter, rect);
    });
}

/// The keys of the library mode, unless a text box has the keyboard.
fn keys(ui: &egui::Ui, s: &mut LibrarySession) {
    let ctx = ui.ctx();
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let (delete, turn, escape, undo, redo) = ctx.input(|i| {
        let command = i.modifiers.command;
        (
            i.key_pressed(egui::Key::Delete),
            i.key_pressed(egui::Key::R) && !command,
            i.key_pressed(egui::Key::Escape),
            command && i.key_pressed(egui::Key::Z),
            command && i.key_pressed(egui::Key::Y),
        )
    });
    if delete {
        s.delete_selected();
    }
    if turn {
        s.rotate_selected();
    }
    if escape {
        s.cancel();
    }
    if undo {
        s.undo();
    }
    if redo {
        s.redo();
    }
}

fn paint(s: &LibrarySession, painter: &egui::Painter, rect: Rect) {
    let painter = painter.with_clip_rect(rect);
    let view = s.view;
    paint_grid(s, &painter, rect);
    // the axes through the origin
    let origin = pos(&view, rect, 0.0, 0.0);
    painter.line_segment(
        [
            Pos2::new(rect.min.x, origin.y),
            Pos2::new(rect.max.x, origin.y),
        ],
        Stroke::new(1.0, AXES),
    );
    painter.line_segment(
        [
            Pos2::new(origin.x, rect.min.y),
            Pos2::new(origin.x, rect.max.y),
        ],
        Stroke::new(1.0, AXES),
    );
    for shape in s.shapes() {
        let marked = s.selected == Some(shape.index());
        paint_shape(&shape, &painter, rect, &view, marked);
    }
    // what the tool in hand is about to make
    if let (Some(first), Some(cursor)) = (s.first_click, s.cursor) {
        let a = pos(&view, rect, first.0 as f64 / 1e6, first.1 as f64 / 1e6);
        let b = pos(&view, rect, cursor.0 as f64 / 1e6, cursor.1 as f64 / 1e6);
        if s.tool == LibTool::Circle {
            painter.circle_stroke(a, a.distance(b), Stroke::new(1.0, ui_accent()));
        } else {
            painter.line_segment([a, b], Stroke::new(1.0, ui_accent()));
        }
        painter.circle_filled(a, 3.0, ui_accent());
    }
    // the measure, over everything
    if let Some(a) = s.measure[0] {
        let pa = pos(&view, rect, a.0 as f64 / 1e6, a.1 as f64 / 1e6);
        painter.circle_filled(pa, 4.0, MEASURE);
        if let Some(b) = s.measure[1] {
            let pb = pos(&view, rect, b.0 as f64 / 1e6, b.1 as f64 / 1e6);
            painter.line_segment([pa, pb], Stroke::new(1.5, MEASURE));
            painter.circle_filled(pb, 4.0, MEASURE);
            if let Some(d) = s.measure_distance() {
                painter.text(
                    Pos2::new((pa.x + pb.x) / 2.0, (pa.y + pb.y) / 2.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("{d:.3} mm"),
                    egui::FontId::proportional(14.0),
                    MEASURE,
                );
            }
        }
    }
}

fn paint_grid(s: &LibrarySession, painter: &egui::Painter, rect: Rect) {
    let view = s.view;
    let step = s.grid_nm as f64 / 1e6;
    if step <= 0.0 || step * view.scale < 8.0 {
        return;
    }
    let (left, top) = view.to_world(0.0, 0.0);
    let (right, bottom) = view.to_world(view.width, view.height);
    let (i0, i1) = ((left / step).floor() as i64, (right / step).ceil() as i64);
    let (j0, j1) = ((bottom / step).floor() as i64, (top / step).ceil() as i64);
    if (i1 - i0 + 1).saturating_mul(j1 - j0 + 1) > MAX_GRID_POINTS {
        return;
    }
    for i in i0..=i1 {
        for j in j0..=j1 {
            painter.circle_filled(
                pos(&view, rect, i as f64 * step, j as f64 * step),
                0.8,
                GRID,
            );
        }
    }
}

fn paint_shape(
    shape: &LibShape,
    painter: &egui::Painter,
    rect: Rect,
    view: &LibView,
    marked: bool,
) {
    let scale = view.scale;
    let at = |p: (f64, f64)| pos(view, rect, p.0, p.1);
    let px = |mm: f64| (mm * scale) as f32;
    let outline = if marked { ui_accent() } else { ui_text() };
    match shape {
        LibShape::Line {
            a, b, width, layer, ..
        } => {
            let colour = if marked {
                ui_accent()
            } else {
                layer_colour(*layer)
            };
            let w = px(*width).max(if marked { 2.0 } else { 1.0 });
            painter.line_segment([at(*a), at(*b)], Stroke::new(w, colour));
            if w > 2.0 {
                painter.circle_filled(at(*a), w / 2.0, colour);
                painter.circle_filled(at(*b), w / 2.0, colour);
            }
        }
        LibShape::Poly {
            points,
            closed,
            filled,
            width,
            layer,
            ..
        } => {
            let colour = if marked {
                ui_accent()
            } else {
                layer_colour(*layer)
            };
            let line: Vec<Pos2> = points.iter().map(|p| at(*p)).collect();
            if *filled {
                fill_convex(painter, &line, colour);
            } else if *closed {
                painter.add(egui::Shape::closed_line(
                    line,
                    Stroke::new(px(*width).max(1.0), colour),
                ));
            } else {
                painter.add(egui::Shape::line(
                    line,
                    Stroke::new(px(*width).max(if marked { 2.0 } else { 1.0 }), colour),
                ));
            }
        }
        LibShape::Disc {
            c,
            r,
            filled,
            width,
            layer,
            ..
        } => {
            let colour = if marked {
                ui_accent()
            } else {
                layer_colour(*layer)
            };
            if *filled {
                painter.circle_filled(at(*c), px(*r).max(0.6), colour);
            } else {
                painter.circle_stroke(
                    at(*c),
                    px(*r).max(0.6),
                    Stroke::new(px(*width).max(1.0), colour),
                );
            }
        }
        LibShape::Pad {
            shape,
            hole_r,
            name,
            layer,
            ..
        } => {
            let colour = layer_colour(*layer);
            let centre = match *shape {
                Shape::Circle { cx, cy, r } => {
                    painter.circle_filled(at((cx, cy)), px(r).max(1.0), colour);
                    if marked {
                        painter.circle_stroke(
                            at((cx, cy)),
                            px(r).max(1.0),
                            Stroke::new(2.0, ui_accent()),
                        );
                    }
                    (cx, cy)
                }
                Shape::Rect { cx, cy, hw, hh } => {
                    let a = at((cx - hw, cy + hh));
                    let b = at((cx + hw, cy - hh));
                    painter.rect_filled(Rect::from_two_pos(a, b), 0.0, colour);
                    if marked {
                        painter.add(egui::Shape::closed_line(
                            vec![a, Pos2::new(b.x, a.y), b, Pos2::new(a.x, b.y)],
                            Stroke::new(2.0, ui_accent()),
                        ));
                    }
                    (cx, cy)
                }
            };
            if *hole_r > 0.0 {
                painter.circle_filled(at(centre), px(*hole_r).max(1.0), BACKGROUND);
            }
            if scale >= 20.0 {
                painter.text(
                    at(centre),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );
            }
        }
        LibShape::Label {
            at: origin,
            text,
            size,
            layer,
            ..
        } => {
            let colour = if marked {
                ui_accent()
            } else {
                layer_colour(*layer)
            };
            painter.text(
                at(*origin),
                egui::Align2::LEFT_BOTTOM,
                text,
                egui::FontId::proportional(px(*size).clamp(8.0, 60.0)),
                colour,
            );
        }
        LibShape::Pin { from, to, name, .. } => {
            let colour = if marked {
                ui_accent()
            } else {
                layer_colour(94)
            };
            painter.line_segment(
                [at(*from), at(*to)],
                Stroke::new(if marked { 2.5 } else { 1.5 }, colour),
            );
            painter.circle_filled(at(*from), 3.0, outline);
            if scale >= 8.0 {
                painter.text(
                    at(*to),
                    egui::Align2::CENTER_BOTTOM,
                    name,
                    egui::FontId::proportional(11.0),
                    ui_text(),
                );
            }
        }
        LibShape::Hole { c, r, .. } => {
            painter.circle_stroke(at(*c), px(*r).max(1.0), Stroke::new(1.5, outline));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_are_read_as_name_equals_value() {
        let p = parse_params("pins=8  variant=narrow ignored  ep=3.1");
        assert_eq!(p.len(), 3);
        assert_eq!(p["pins"], "8");
        assert_eq!(p["variant"], "narrow");
        assert_eq!(p["ep"], "3.1");
        assert!(parse_params("").is_empty());
    }

    #[test]
    fn the_new_file_goes_next_to_the_library() {
        assert_eq!(
            suggested_path(Path::new("lib/parts.lbr")),
            Path::new("lib/parts_edit.lbr")
        );
    }

    #[test]
    fn layers_have_colours_and_the_kinds_of_the_lists_have_words() {
        assert_ne!(layer_colour(1), layer_colour(16));
        assert_eq!(layer_colour(21), layer_colour(22));
        assert_eq!(kind_of("package"), Kind::Package);
        assert_eq!(kind_of("symbol"), Kind::Symbol);
        assert_eq!(kind_of("deviceset"), Kind::DeviceSet);
        assert_eq!(kind_text(Kind::Symbol), LibText::Symbols);
    }
}
