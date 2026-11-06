// SPDX-License-Identifier: GPL-3.0-or-later
//! The window of the imported Gerber and Excellon files (built with the `gui` feature): the layers
//! of a folder drawn as filled shapes, the list of layers with visibility, order and opacity, the
//! measure between two points and the comparison with a board of the project. The scene, the snap
//! and the comparison are decided in [`crate::gerber_scene`], in code that is tested; this file
//! only draws them and passes on what the person does. It has not been seen on a screen.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use eh3d_phoenix_rules::Lang;

use crate::codes::ViewError;
use crate::gerber_scene::{
    board_scene, compare_scenes, load_board, open_folder, Comparison, DiffKind, GerberScene,
};
use crate::gerber_text::{role_title, GerberText};
use crate::gui::{ACCENT, DANGER, DIM};
use crate::measure::Measure;
use crate::msg::Msg;
use crate::prim::P;
use crate::settings::ViewerSettings;
use crate::view::{View, ZoomLimits};

const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ACCENT)
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(DIM)
}

/// What the person asked for in the list of layers.
enum Action {
    Visible(usize, bool),
    Opacity(usize, f64),
    Up(usize),
    Down(usize),
}

/// One line of the list of layers, ready to draw.
struct Row {
    index: usize,
    title: String,
    visible: bool,
    opacity: f64,
    colour: [u8; 4],
    drawable: bool,
    details: String,
    problem: Option<String>,
}

/// The window of an imported set of files.
pub struct GerberUi {
    folder: PathBuf,
    scene: GerberScene,
    skipped: usize,
    view: View,
    fit_pending: bool,
    measure: Measure,
    snap: bool,
    board_text: String,
    comparison: Option<Comparison>,
    show_diff: bool,
    error: Option<ViewError>,
    cursor: Option<P>,
}

impl GerberUi {
    /// Reads the folder and prepares the window.
    pub fn open(folder: &Path, settings: &ViewerSettings) -> Result<GerberUi, ViewError> {
        let (scene, skipped) = open_folder(folder, settings)?;
        Ok(GerberUi {
            folder: folder.to_path_buf(),
            scene,
            skipped,
            view: View::default(),
            fit_pending: true,
            measure: Measure::default(),
            snap: true,
            board_text: String::new(),
            comparison: None,
            show_diff: true,
            error: None,
            cursor: None,
        })
    }

    fn rows(&self, lang: Lang) -> Vec<Row> {
        self.scene
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let counts = if layer.holes + layer.slots > 0 {
                    format!(
                        "{} {} / {} {}",
                        layer.holes,
                        GerberText::Holes.tr(lang),
                        layer.slots,
                        GerberText::Slots.tr(lang)
                    )
                } else {
                    format!(
                        "{} {} / {} {}",
                        layer.dark,
                        GerberText::Dark.tr(lang),
                        layer.clear,
                        GerberText::Clear.tr(lang)
                    )
                };
                let problem = if let Some(error) = &layer.error {
                    Some(format!("{}: {error}", GerberText::NotRead.tr(lang)))
                } else if layer.incomplete {
                    Some(GerberText::Incomplete.tr(lang).to_owned())
                } else {
                    None
                };
                Row {
                    index,
                    title: role_title(layer.role, lang),
                    visible: layer.visible,
                    opacity: layer.opacity,
                    colour: layer.colour,
                    drawable: layer.error.is_none(),
                    details: format!("{} · {counts}", layer.name),
                    problem,
                }
            })
            .collect()
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Visible(i, visible) => self.scene.set_visible(i, visible),
            Action::Opacity(i, opacity) => self.scene.set_opacity(i, opacity),
            Action::Up(i) => {
                self.scene.move_up(i);
            }
            Action::Down(i) => {
                self.scene.move_down(i);
            }
        }
    }

    fn compare(&mut self, settings: &ViewerSettings) {
        let path = PathBuf::from(self.board_text.trim());
        let result = load_board(&path, settings)
            .and_then(|board| board_scene(&board, settings))
            .map(|reference| compare_scenes(&self.scene, &reference, settings, true));
        match result {
            Ok(comparison) => {
                self.comparison = Some(comparison);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Draws the window in the whole of the central area. The answer is true when the person
    /// closes it.
    pub fn show(&mut self, ui: &mut egui::Ui, lang: Lang, settings: &ViewerSettings) -> bool {
        let mut close = false;
        egui::Panel::top("gerber_top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(GerberText::Title.tr(lang))
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                ui.label(dim(format!(
                    "{}: {}",
                    GerberText::Folder.tr(lang),
                    self.folder.display()
                )));
                if self.skipped > 0 {
                    ui.label(dim(format!(
                        "· {} {}",
                        self.skipped,
                        GerberText::SkippedFiles.tr(lang)
                    )));
                }
                if ui.button(Msg::Fit.tr(lang)).clicked() {
                    self.fit_pending = true;
                }
                if ui.button(GerberText::Close.tr(lang)).clicked() {
                    close = true;
                }
            });
        });
        egui::Panel::bottom("gerber_status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(error) = &self.error {
                    ui.label(egui::RichText::new(error.message(lang)).color(DANGER));
                } else {
                    ui.label(dim(GerberText::ReadOnly.tr(lang)));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::Prototype.tr(lang)).color(ACCENT));
                });
            });
        });
        egui::Panel::left("gerber_layers")
            .default_size(300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.layers_panel(ui, lang);
                    ui.separator();
                    self.measure_panel(ui, lang);
                    ui.separator();
                    self.compare_panel(ui, lang, settings);
                });
            });
        self.canvas(ui, settings);
        close
    }

    fn layers_panel(&mut self, ui: &mut egui::Ui, lang: Lang) {
        ui.label(heading(Msg::Layers.tr(lang)));
        ui.horizontal(|ui| {
            if ui.small_button(Msg::ShowAll.tr(lang)).clicked() {
                self.scene.set_all_visible(true);
            }
            if ui.small_button(Msg::HideAll.tr(lang)).clicked() {
                self.scene.set_all_visible(false);
            }
        });
        if self.scene.truncated {
            ui.label(egui::RichText::new(GerberText::Truncated.tr(lang)).color(DANGER));
        }
        let mut action = None;
        // the layer drawn last is the first of the list
        for row in self.rows(lang).iter().rev() {
            ui.push_id(("gerber_layer", row.index), |ui| {
                ui.horizontal(|ui| {
                    let mut visible = row.visible;
                    let check = egui::Checkbox::new(&mut visible, row.title.as_str());
                    if ui.add_enabled(row.drawable, check).changed() {
                        action = Some(Action::Visible(row.index, visible));
                    }
                    let (swatch, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(swatch, 2.0, rgba(row.colour));
                    if ui.small_button(GerberText::MoveUp.tr(lang)).clicked() {
                        action = Some(Action::Up(row.index));
                    }
                    if ui.small_button(GerberText::MoveDown.tr(lang)).clicked() {
                        action = Some(Action::Down(row.index));
                    }
                });
                ui.label(dim(row.details.as_str()));
                if let Some(problem) = &row.problem {
                    ui.label(egui::RichText::new(problem.as_str()).color(DANGER));
                }
                if row.drawable {
                    let mut opacity = row.opacity;
                    let slider = egui::Slider::new(&mut opacity, 0.05..=1.0)
                        .text(GerberText::Opacity.tr(lang));
                    if ui.add(slider).changed() {
                        action = Some(Action::Opacity(row.index, opacity));
                    }
                }
            });
        }
        if let Some(action) = action {
            self.apply(action);
        }
        ui.label(dim(GerberText::ClearNote.tr(lang)));
    }

    fn measure_panel(&mut self, ui: &mut egui::Ui, lang: Lang) {
        ui.label(heading(GerberText::Measure.tr(lang)));
        ui.checkbox(&mut self.snap, GerberText::Snap.tr(lang));
        ui.label(dim(GerberText::MeasureHint.tr(lang)));
        if let (Some((dx, dy)), Some(d)) = (self.measure.delta(), self.measure.distance()) {
            ui.label(format!("{}: {d:.4} mm", Msg::Distance.tr(lang)));
            ui.label(dim(format!("dx {dx:.4} · dy {dy:.4}")));
            if let Some(angle) = self.measure.angle_deg() {
                ui.label(dim(format!("{}: {angle:.2}°", Msg::Angle.tr(lang))));
            }
        }
        if ui.small_button(Msg::ClearSelection.tr(lang)).clicked() {
            self.measure.clear();
        }
    }

    fn compare_panel(&mut self, ui: &mut egui::Ui, lang: Lang, settings: &ViewerSettings) {
        ui.label(heading(GerberText::Compare.tr(lang)));
        ui.label(dim(GerberText::BoardFile.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut self.board_text).desired_width(260.0));
        if ui.button(GerberText::CompareNow.tr(lang)).clicked() {
            self.compare(settings);
        }
        let Some(comparison) = &self.comparison else {
            return;
        };
        ui.checkbox(&mut self.show_diff, GerberText::ShowDifferences.tr(lang));
        ui.label(heading(GerberText::Differences.tr(lang)));
        ui.label(dim(format!(
            "{}: {:.3}, {:.3}",
            GerberText::Shift.tr(lang),
            comparison.shift.x,
            comparison.shift.y
        )));
        for diff in &comparison.layers {
            let title = role_title(Some(diff.role), lang);
            if diff.identical() {
                ui.label(
                    egui::RichText::new(format!(
                        "{title} ({}): {}",
                        diff.name,
                        GerberText::Identical.tr(lang)
                    ))
                    .color(OK),
                );
            } else {
                ui.label(egui::RichText::new(format!("{title} ({})", diff.name)).color(DANGER));
                ui.label(dim(format!(
                    "  {}: {:.3} mm² · {}: {:.3} mm² · {} {:.3}",
                    GerberText::OnlyFiles.tr(lang),
                    diff.only_files_mm2(),
                    GerberText::OnlyBoard.tr(lang),
                    diff.only_board_mm2(),
                    GerberText::CellSize.tr(lang),
                    diff.cell_mm
                )));
            }
        }
        for name in &comparison.unmatched {
            ui.label(dim(format!("{name}: {}", GerberText::NoMatch.tr(lang))));
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
                let bounds = self.scene.bounds();
                self.view.fit(bounds, settings.fit_margin_fraction, limits);
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
                if let Some(p) = response.interact_pointer_pos() {
                    let (sx, sy) = local(p);
                    let mut at = self.view.to_world(sx, sy);
                    if self.snap {
                        let radius = settings.gerber_snap_px / self.view.scale;
                        if let Some(found) = self.scene.snap(at, radius) {
                            at = found;
                        }
                    }
                    self.measure.click(at);
                }
            }
            self.paint(&painter, rect, settings, background);
        });
    }

    fn paint(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        settings: &ViewerSettings,
        background: Color32,
    ) {
        let painter = painter.with_clip_rect(rect);
        let view = self.view;
        let at = |p: P| -> Pos2 {
            let (x, y) = view.to_screen(p);
            Pos2::new(rect.min.x + x as f32, rect.min.y + y as f32)
        };
        let area = view.visible_area();
        for layer in self
            .scene
            .layers
            .iter()
            .filter(|l| l.visible && l.is_drawable())
        {
            let colour = rgba(layer.draw_colour());
            let mut mesh = egui::epaint::Mesh::default();
            for poly in &layer.polys {
                if !poly.bounds.intersects(&area) {
                    continue;
                }
                // a clear object takes away what the dark ones before it drew
                let fill = if poly.dark { colour } else { background };
                if poly.tris.is_empty() {
                    let line: Vec<Pos2> = poly.points.iter().map(|p| at(*p)).collect();
                    painter.add(egui::Shape::closed_line(line, Stroke::new(1.0, fill)));
                    continue;
                }
                let base = mesh.vertices.len() as u32;
                for p in &poly.points {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: at(*p),
                        uv: egui::epaint::WHITE_UV,
                        color: fill,
                    });
                }
                for t in &poly.tris {
                    mesh.add_triangle(base + t[0], base + t[1], base + t[2]);
                }
                if mesh.vertices.len() > 60_000 {
                    painter.add(egui::Shape::mesh(std::mem::take(&mut mesh)));
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        if self.show_diff {
            if let Some(comparison) = &self.comparison {
                for diff in &comparison.layers {
                    let shown = self
                        .scene
                        .layers
                        .iter()
                        .any(|l| l.name == diff.name && l.visible);
                    if !shown {
                        continue;
                    }
                    for run in &diff.runs {
                        let colour = match run.kind {
                            DiffKind::OnlyFiles => rgba(settings.colour("gerber_only_files")),
                            DiffKind::OnlyBoard => rgba(settings.colour("gerber_only_board")),
                        };
                        painter.rect_filled(
                            Rect::from_two_pos(at(run.min), at(run.max)),
                            0.0,
                            colour,
                        );
                    }
                }
            }
        }
        let measure_colour = rgba(settings.colour("measure"));
        if let Some(a) = self.measure.a {
            painter.circle_filled(at(a), 4.0, measure_colour);
            if let Some(b) = self.measure.b {
                painter.line_segment([at(a), at(b)], Stroke::new(1.5, measure_colour));
                painter.circle_filled(at(b), 4.0, measure_colour);
                if let Some(d) = self.measure.distance() {
                    let middle = P::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    painter.text(
                        at(middle),
                        egui::Align2::CENTER_BOTTOM,
                        format!("{d:.4} mm"),
                        egui::FontId::proportional(14.0),
                        measure_colour,
                    );
                }
            }
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
