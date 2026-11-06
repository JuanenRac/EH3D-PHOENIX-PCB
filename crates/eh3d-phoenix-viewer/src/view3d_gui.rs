// SPDX-License-Identifier: GPL-3.0-or-later
//! The 3D window of the assembled board (built with the `gui` feature): the board and its
//! components drawn in perspective with an orbiting camera, the selection of a component by a
//! click and the marks of the collisions, of the components that are too tall and of the ones
//! outside the outline. The camera, the projection, the order of the faces and the selection are
//! decided in [`crate::view3d`], in code that is tested; this file only draws them with the
//! painter of egui and passes on what the person does. It has not been seen on a screen.

use std::path::Path;

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_assembly3d::AssemblySettings;
use eh3d_phoenix_board::Board;
use eh3d_phoenix_rules::Lang;

use crate::codes::ViewError;
use crate::gui::{ACCENT, DANGER, DIM};
use crate::msg::Msg;
use crate::settings::ViewerSettings;
use crate::view3d::{
    assemble_with_table_file, face_colour, part_info, pick, summary, Assembly, Camera, Selection,
};
use crate::view3d_text::View3dText;

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ACCENT)
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(DIM)
}

/// The 3D window of an assembly.
pub struct View3dUi {
    assembly: Assembly,
    camera: Camera,
    selection: Option<Selection>,
}

impl View3dUi {
    /// Builds the assembly of a board with the table of models in a file (or none) and prepares
    /// the camera.
    pub fn open(
        board: &Board,
        table: Option<&Path>,
        settings: &ViewerSettings,
    ) -> Result<View3dUi, ViewError> {
        let assembly =
            assemble_with_table_file(board, table, &AssemblySettings::default(), settings)?;
        let camera = assembly.camera(settings);
        Ok(View3dUi {
            assembly,
            camera,
            selection: None,
        })
    }

    /// Draws the window in the whole of the central area. The answer is true when the person
    /// closes it.
    pub fn show(&mut self, ui: &mut egui::Ui, lang: Lang, settings: &ViewerSettings) -> bool {
        let mut close = false;
        egui::Panel::top("view3d_top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(View3dText::Title.tr(lang))
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                ui.label(dim(self.assembly.scene.name.as_str()));
                if ui.button(View3dText::Reset.tr(lang)).clicked() {
                    self.camera = self.assembly.camera(settings);
                }
                if ui.button(View3dText::Close.tr(lang)).clicked() {
                    close = true;
                }
            });
        });
        egui::Panel::bottom("view3d_status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(dim(View3dText::Controls.tr(lang)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::Prototype.tr(lang)).color(ACCENT));
                });
            });
        });
        egui::Panel::right("view3d_info")
            .default_size(320.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.info_panel(ui, lang, settings);
                });
            });
        self.canvas(ui, settings);
        close
    }

    fn info_panel(&self, ui: &mut egui::Ui, lang: Lang, settings: &ViewerSettings) {
        let total = summary(&self.assembly.scene, &self.assembly.checks);
        ui.label(heading(View3dText::Board.tr(lang)));
        ui.label(format!(
            "{}: {:.3}",
            View3dText::Thickness.tr(lang),
            total.board_thickness_mm
        ));
        ui.label(format!("{}: {}", View3dText::Parts.tr(lang), total.parts));
        if total.parts == 0 {
            ui.label(dim(View3dText::NoModels.tr(lang)));
        }
        ui.separator();
        ui.label(heading(View3dText::Legend.tr(lang)));
        for (name, text, count) in [
            ("view3d_collision", View3dText::Collisions, total.collisions),
            ("view3d_too_tall", View3dText::TooTall, total.too_tall),
            ("view3d_outside", View3dText::Outside, total.outside),
        ] {
            ui.horizontal(|ui| {
                let (swatch, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(swatch, 2.0, rgba(settings.colour(name)));
                ui.label(format!("{}: {count}", text.tr(lang)));
            });
        }
        if let Some((reference, reach)) = &total.tallest {
            ui.label(format!(
                "{}: {reference} ({reach:.2} mm)",
                View3dText::Tallest.tr(lang)
            ));
        }
        ui.separator();
        ui.label(heading(View3dText::Selected.tr(lang)));
        match self.selection {
            Some(Selection::Part(index)) => {
                if let Some(info) = part_info(&self.assembly.scene, &self.assembly.checks, index) {
                    let side = if info.bottom {
                        View3dText::SideBottom
                    } else {
                        View3dText::SideTop
                    };
                    ui.label(format!(
                        "{}: {}",
                        View3dText::Reference.tr(lang),
                        info.reference
                    ));
                    ui.label(format!(
                        "{}: {}",
                        View3dText::Footprint.tr(lang),
                        info.footprint
                    ));
                    ui.label(format!("{}: {}", View3dText::Side.tr(lang), side.tr(lang)));
                    ui.label(format!(
                        "{}: {:.3}",
                        View3dText::Height.tr(lang),
                        info.height_mm
                    ));
                    ui.label(format!(
                        "{}: {:.3}",
                        View3dText::Reach.tr(lang),
                        info.reach_mm
                    ));
                    ui.label(dim(format!(
                        "z {:.3} … {:.3} mm",
                        info.z_min_mm, info.z_max_mm
                    )));
                    if !info.collides_with.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                View3dText::CollidesWith.tr(lang),
                                info.collides_with.join(", ")
                            ))
                            .color(DANGER),
                        );
                    }
                    if info.too_tall {
                        ui.label(egui::RichText::new(View3dText::TooTall.tr(lang)).color(DANGER));
                    }
                    if info.outside {
                        ui.label(egui::RichText::new(View3dText::Outside.tr(lang)).color(DANGER));
                    }
                }
            }
            Some(Selection::Board) => {
                ui.label(View3dText::Board.tr(lang));
                ui.label(format!(
                    "{}: {:.3}",
                    View3dText::Thickness.tr(lang),
                    total.board_thickness_mm
                ));
            }
            None => {
                ui.label(dim(View3dText::ClickHint.tr(lang)));
            }
        }
        let lines: Vec<String> = self
            .assembly
            .scene
            .findings
            .iter()
            .chain(self.assembly.checks.findings.iter())
            .take(settings.findings_shown)
            .map(|f| f.line(lang))
            .collect();
        if !lines.is_empty() {
            ui.separator();
            ui.label(heading(View3dText::Findings.tr(lang)));
            for line in lines {
                ui.add(egui::Label::new(dim(line)).wrap());
            }
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui, settings: &ViewerSettings) {
        let background = rgba(settings.colour("background"));
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            painter.rect_filled(rect, 0.0, background);
            self.camera
                .resize(f64::from(rect.width()), f64::from(rect.height()));
            let shift = ui.input(|i| i.modifiers.shift);
            let panning = response.dragged_by(egui::PointerButton::Secondary)
                || response.dragged_by(egui::PointerButton::Middle)
                || (response.dragged_by(egui::PointerButton::Primary) && shift);
            if panning {
                let d = response.drag_delta();
                self.camera.pan(f64::from(d.x), f64::from(d.y), settings);
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let d = response.drag_delta();
                self.camera.orbit(f64::from(d.x), f64::from(d.y), settings);
            }
            if response.hover_pos().is_some() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll.abs() > f32::EPSILON {
                    self.camera.zoom(f64::from(scroll), settings);
                }
            }
            let frame = self.camera.frame(&self.assembly.model);
            if response.clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    let local = p - rect.min;
                    self.selection = pick(
                        &frame,
                        &self.assembly.model,
                        f64::from(local.x),
                        f64::from(local.y),
                    )
                    .map(|(_, selection)| selection);
                }
            }
            self.paint(&painter, rect, settings, &frame);
        });
    }

    fn paint(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        settings: &ViewerSettings,
        frame: &[crate::view3d::ProjectedFace],
    ) {
        let painter = painter.with_clip_rect(rect);
        let model = &self.assembly.model;
        let at = |p: (f64, f64)| Pos2::new(rect.min.x + p.0 as f32, rect.min.y + p.1 as f32);
        let mut mesh = egui::epaint::Mesh::default();
        for projected in frame {
            let face = &model.faces[projected.face];
            let reference = face
                .part
                .and_then(|i| self.assembly.scene.parts.get(i))
                .map(|p| p.reference.as_str());
            let state = face
                .part
                .and_then(|i| self.assembly.states.get(i).copied())
                .unwrap_or_default();
            let selected = match self.selection {
                Some(Selection::Part(i)) => face.part == Some(i),
                Some(Selection::Board) => face.part.is_none(),
                None => false,
            };
            let colour = rgba(face_colour(
                settings,
                face,
                reference,
                state,
                selected,
                projected.shade,
            ));
            let base = mesh.vertices.len() as u32;
            for p in &projected.points {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: at(*p),
                    uv: egui::epaint::WHITE_UV,
                    color: colour,
                });
            }
            for t in &face.tris {
                mesh.add_triangle(base + t[0] as u32, base + t[1] as u32, base + t[2] as u32);
            }
            if mesh.vertices.len() > 60_000 {
                painter.add(egui::Shape::mesh(std::mem::take(&mut mesh)));
            }
        }
        painter.add(egui::Shape::mesh(mesh));
        // the outline of what is selected, over the rest
        if let Some(Selection::Part(index)) = self.selection {
            let edge = Stroke::new(2.0, rgba(settings.colour("view3d_selected")));
            for projected in frame
                .iter()
                .filter(|f| model.faces[f.face].part == Some(index))
            {
                let line: Vec<Pos2> = projected.points.iter().map(|p| at(*p)).collect();
                painter.add(egui::Shape::closed_line(line, edge));
            }
        }
    }
}
