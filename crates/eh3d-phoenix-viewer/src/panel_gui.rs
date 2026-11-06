// SPDX-License-Identifier: GPL-3.0-or-later
//! The window of the panel of boards (built with the `gui` feature): the copies of the boards, the
//! frame, the tabs, the V-score lines, the fiducials and the holes drawn on layers that can be
//! hidden, the measures of the panel and the part of its area that the boards take. The scene
//! and the layout are decided in [`crate::panel_scene`], in code that is tested; this file only
//! draws them and passes on what the person does. It has not been seen on a screen.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use eh3d_phoenix_panel::Joint;
use eh3d_phoenix_rules::Lang;

use crate::codes::ViewError;
use crate::gui::{ACCENT, DANGER, DIM};
use crate::msg::Msg;
use crate::panel_scene::{parse_files, PanelLayerKind, PanelSession, PanelShape};
use crate::panel_text::PanelText;
use crate::prim::P;
use crate::settings::ViewerSettings;
use crate::view::{View, ZoomLimits};

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ACCENT)
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(DIM)
}

/// The window of a panel.
pub struct PanelUi {
    session: Option<PanelSession>,
    files_text: String,
    config_text: String,
    columns: u32,
    rows: u32,
    v_score: bool,
    view: View,
    fit_pending: bool,
    error: Option<ViewError>,
    cursor: Option<P>,
    picked: Option<usize>,
}

impl PanelUi {
    /// Opens the window with the boards that were asked for (none: an empty window to type them).
    pub fn new(files: &[PathBuf], config: Option<&Path>, settings: &ViewerSettings) -> PanelUi {
        let mut ui = PanelUi {
            session: None,
            files_text: files
                .iter()
                .map(|f| f.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(", "),
            config_text: config
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or_default(),
            columns: 2,
            rows: 2,
            v_score: false,
            view: View::default(),
            fit_pending: true,
            error: None,
            cursor: None,
            picked: None,
        };
        if !files.is_empty() {
            ui.lay_out(settings);
        }
        ui
    }

    /// Reads the boards and the settings file again and lays the panel out.
    fn lay_out(&mut self, settings: &ViewerSettings) {
        let files = parse_files(&self.files_text);
        let config = self.config_text.trim();
        let config = (!config.is_empty()).then(|| PathBuf::from(config));
        match PanelSession::open(&files, config.as_deref(), settings) {
            Ok(session) => {
                self.columns = session.settings.columns;
                self.rows = session.settings.rows;
                self.v_score = session.settings.joint == Joint::VScore;
                self.session = Some(session);
                self.error = None;
                self.fit_pending = true;
                self.picked = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Lays the panel out again with the matrix and the joint of the window.
    fn change_matrix(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut changed = session.settings.clone();
        changed.columns = self.columns;
        changed.rows = self.rows;
        changed.joint = if self.v_score {
            Joint::VScore
        } else {
            Joint::MouseBites
        };
        if self.v_score {
            changed.gap_nm = 0;
        } else if changed.gap_nm == 0 {
            changed.gap_nm = eh3d_phoenix_panel::PanelSettings::default().gap_nm;
        }
        match session.relayout(changed) {
            Ok(()) => {
                self.error = None;
                self.fit_pending = true;
                self.picked = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Draws the window in the whole of the central area. The answer is true when the person
    /// closes it.
    pub fn show(&mut self, ui: &mut egui::Ui, lang: Lang, settings: &ViewerSettings) -> bool {
        let mut close = false;
        egui::Panel::top("panel_top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(PanelText::Title.tr(lang))
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                if ui.button(Msg::Fit.tr(lang)).clicked() {
                    self.fit_pending = true;
                }
                if ui.button(PanelText::Close.tr(lang)).clicked() {
                    close = true;
                }
            });
        });
        egui::Panel::bottom("panel_status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(error) = &self.error {
                    ui.label(egui::RichText::new(error.message(lang)).color(DANGER));
                } else {
                    ui.label(dim(PanelText::ReadOnly.tr(lang)));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::Prototype.tr(lang)).color(ACCENT));
                });
            });
        });
        egui::Panel::left("panel_side")
            .default_size(320.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.files_panel(ui, lang, settings);
                    ui.separator();
                    self.layers_panel(ui, lang);
                    ui.separator();
                    self.measures_panel(ui, lang);
                });
            });
        self.canvas(ui, settings);
        close
    }

    fn files_panel(&mut self, ui: &mut egui::Ui, lang: Lang, settings: &ViewerSettings) {
        ui.label(heading(PanelText::BoardFiles.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut self.files_text).desired_width(300.0));
        ui.label(dim(PanelText::SettingsFile.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut self.config_text).desired_width(300.0));
        if ui.button(PanelText::LayOut.tr(lang)).clicked() {
            self.lay_out(settings);
        }
        if self.session.is_none() {
            ui.label(dim(PanelText::Nothing.tr(lang)));
            return;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(PanelText::Columns.tr(lang));
            changed |= ui
                .add(egui::DragValue::new(&mut self.columns).range(1..=64))
                .changed();
            ui.label(PanelText::Rows.tr(lang));
            changed |= ui
                .add(egui::DragValue::new(&mut self.rows).range(1..=64))
                .changed();
        });
        changed |= ui
            .checkbox(&mut self.v_score, PanelText::VScore.tr(lang))
            .changed();
        if changed {
            self.change_matrix();
        }
    }

    fn layers_panel(&mut self, ui: &mut egui::Ui, lang: Lang) {
        ui.label(heading(Msg::Layers.tr(lang)));
        let Some(session) = self.session.as_mut() else {
            return;
        };
        ui.horizontal(|ui| {
            if ui.small_button(Msg::ShowAll.tr(lang)).clicked() {
                session.scene.set_all_visible(true);
            }
            if ui.small_button(Msg::HideAll.tr(lang)).clicked() {
                session.scene.set_all_visible(false);
            }
        });
        for kind in PanelLayerKind::ALL {
            let mut visible = session.scene.visible(kind);
            let count = session.scene.count(kind);
            ui.horizontal(|ui| {
                if ui.checkbox(&mut visible, kind.text().tr(lang)).changed() {
                    session.scene.set_visible(kind, visible);
                }
                let (swatch, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().rect_filled(swatch, 2.0, rgba(kind.colour()));
                ui.label(dim(count.to_string()));
            });
        }
    }

    fn measures_panel(&mut self, ui: &mut egui::Ui, lang: Lang) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        for (index, line) in session.scene.summary(lang).iter().enumerate() {
            if index == 0 {
                ui.label(heading(line));
            } else {
                ui.label(dim(line.as_str()));
            }
        }
        if let Some(copy) = self.picked.and_then(|i| session.scene.copies.get(i)) {
            ui.label(format!("{} {}", PanelText::Board.tr(lang), copy.board + 1));
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui, settings: &ViewerSettings) {
        let background = rgba(settings.colour("background"));
        let limits = ZoomLimits {
            min: settings.zoom_min_px_per_mm,
            max: settings.zoom_max_px_per_mm,
        };
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            painter.rect_filled(rect, 0.0, background);
            self.view
                .resize(f64::from(rect.width()), f64::from(rect.height()));
            if self.fit_pending && rect.width() > 40.0 && rect.height() > 40.0 {
                if let Some(session) = self.session.as_ref() {
                    let bounds = session.scene.bounds();
                    self.view.fit(bounds, settings.fit_margin_fraction, limits);
                }
                self.fit_pending = false;
            }
            let local = |p: Pos2| {
                let at = p - rect.min;
                (f64::from(at.x), f64::from(at.y))
            };
            if response.double_clicked() {
                self.fit_pending = true;
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.view.pan_px(f64::from(d.x), f64::from(d.y));
            }
            self.cursor = None;
            if let Some(p) = response.hover_pos() {
                let (sx, sy) = local(p);
                self.cursor = Some(self.view.to_world(sx, sy));
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                if scroll.abs() > f32::EPSILON || (pinch - 1.0).abs() > 1e-6 {
                    let factor =
                        (f64::from(scroll) * settings.zoom_wheel_rate).exp() * f64::from(pinch);
                    self.view.zoom_at(sx, sy, factor, limits);
                }
            }
            if response.clicked() {
                if let (Some(p), Some(session)) =
                    (response.interact_pointer_pos(), self.session.as_ref())
                {
                    let (sx, sy) = local(p);
                    let at = self.view.to_world(sx, sy);
                    self.picked = session.scene.copy_at(at);
                }
            }
            self.paint(&painter, rect);
        });
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect) {
        let painter = painter.with_clip_rect(rect);
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let view = self.view;
        let at = |p: P| -> Pos2 {
            let (x, y) = view.to_screen(p);
            Pos2::new(rect.min.x + x as f32, rect.min.y + y as f32)
        };
        let scale = view.scale as f32;
        for kind in PanelLayerKind::ALL {
            let colour = rgba(kind.colour());
            for layer in session
                .scene
                .layers
                .iter()
                .filter(|l| l.kind == kind && l.visible)
            {
                for shape in &layer.shapes {
                    match shape {
                        PanelShape::Rect { min, max, filled } => {
                            let r = Rect::from_two_pos(at(*min), at(*max));
                            if *filled {
                                painter.rect_filled(r, 0.0, colour);
                            } else {
                                let corners = vec![
                                    r.left_top(),
                                    r.right_top(),
                                    r.right_bottom(),
                                    r.left_bottom(),
                                ];
                                painter.add(egui::Shape::closed_line(
                                    corners,
                                    Stroke::new(1.0, colour),
                                ));
                            }
                        }
                        PanelShape::Line { from, to } => {
                            painter.line_segment([at(*from), at(*to)], Stroke::new(1.5, colour));
                        }
                        PanelShape::Circle {
                            centre,
                            radius,
                            filled,
                        } => {
                            let r = (*radius as f32 * scale).max(1.5);
                            if *filled {
                                painter.circle_filled(at(*centre), r, colour);
                            } else {
                                painter.circle_stroke(at(*centre), r, Stroke::new(1.0, colour));
                            }
                        }
                        PanelShape::Label { at: place, text } => {
                            painter.text(
                                at(*place),
                                egui::Align2::CENTER_CENTER,
                                text.as_str(),
                                egui::FontId::proportional(16.0),
                                DIM,
                            );
                        }
                    }
                }
            }
        }
        if let Some(copy) = self.picked.and_then(|i| session.scene.copies.get(i)) {
            let r = Rect::from_two_pos(at(copy.min), at(copy.max));
            let corners = vec![
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
            ];
            painter.add(egui::Shape::closed_line(corners, Stroke::new(2.0, ACCENT)));
        }
        if let Some(p) = self.cursor {
            painter.text(
                rect.left_bottom() + egui::vec2(8.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                format!("x {:.3}  y {:.3} mm", p.x, p.y),
                egui::FontId::proportional(13.0),
                DIM,
            );
        }
    }
}
