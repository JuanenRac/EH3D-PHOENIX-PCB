// SPDX-License-Identifier: GPL-3.0-or-later
//! The window (built with the `gui` feature): it draws the state of [`Viewer`] with egui on eframe
//! and passes on what the person does. Everything that can be decided without a screen is decided
//! in the viewer, in code that is tested. This file has not been seen on a screen.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke};
use eh3d_phoenix_rules::Lang;

use crate::codes::{ViewError, WINDOW_FAILED};
use crate::load;
use crate::msg::Msg;
use crate::prim::{arc_points, triangulate, Shape, P};
use crate::scene::SceneKind;
use crate::settings::ViewerSettings;
use crate::viewer::{Tool, Viewer};

// the colours of the theme Forge, the one the programs of the suite start with: amber on charcoal
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);

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

struct App {
    lang: Lang,
    settings: ViewerSettings,
    viewer: Option<Viewer>,
    path_text: String,
    error: Option<ViewError>,
    unread: usize,
    fit_pending: bool,
}

impl App {
    fn new(lang: Lang, settings: ViewerSettings, initial: Option<PathBuf>) -> App {
        let mut app = App {
            lang,
            settings,
            viewer: None,
            path_text: String::new(),
            error: None,
            unread: 0,
            fit_pending: false,
        };
        if let Some(path) = initial {
            app.path_text = path.to_string_lossy().into_owned();
            app.open(&path);
        }
        app
    }

    /// Reads and draws a file; a file that cannot be used leaves the drawing as it was.
    fn open(&mut self, path: &Path) {
        match load::open(path, &self.settings) {
            Ok((scene, unread)) => {
                self.viewer = Some(Viewer::new(scene, self.settings.clone()));
                self.unread = unread;
                self.error = None;
                self.fit_pending = true;
            }
            Err(error) => self.error = Some(error),
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
                ui.label(egui::RichText::new(Msg::PathHint.tr(lang)).color(DIM));
                let edit = egui::TextEdit::singleline(&mut self.path_text).desired_width(360.0);
                let response = ui.add(edit);
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button(Msg::Open.tr(lang)).clicked() || enter {
                    let path = PathBuf::from(self.path_text.trim());
                    self.open(&path);
                }
                let has_scene = self.viewer.is_some();
                if ui
                    .add_enabled(has_scene, egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(language_name(self.lang))
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.lang, l, language_name(l));
                            }
                        });
                    ui.label(egui::RichText::new(Msg::Language.tr(lang)).color(DIM));
                });
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let unread = self.unread;
        egui::Panel::left("layers")
            .default_size(260.0)
            .show(ui, |ui| {
                let Some(viewer) = self.viewer.as_mut() else {
                    return;
                };
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    let kind = match viewer.scene.kind {
                        SceneKind::Board => Msg::Board,
                        SceneKind::Schematic => Msg::Schematic,
                    };
                    ui.label(egui::RichText::new(kind.tr(lang)).strong().color(ACCENT));
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {} · {} {} · {} {}",
                            viewer.scene.prims.len(),
                            Msg::Primitives.tr(lang),
                            viewer.scene.items.len(),
                            Msg::Items.tr(lang),
                            viewer.scene.nets.len(),
                            Msg::Nets.tr(lang)
                        ))
                        .color(DIM),
                    );
                    let not_drawn = viewer.scene.skipped + unread;
                    if not_drawn > 0 {
                        ui.label(
                            egui::RichText::new(format!("{} {}", not_drawn, Msg::Skipped.tr(lang)))
                                .color(DIM),
                        );
                    }
                    ui.separator();
                    let mut tool = viewer.tool;
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut tool, Tool::Select, Msg::ToolSelect.tr(lang));
                        ui.selectable_value(&mut tool, Tool::Measure, Msg::ToolMeasure.tr(lang));
                    });
                    viewer.set_tool(tool);
                    if ui.button(Msg::ClearSelection.tr(lang)).clicked() {
                        viewer.clear_selection();
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(Msg::Layers.tr(lang))
                            .strong()
                            .color(ACCENT),
                    );
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

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(error) = &self.error {
                    ui.label(egui::RichText::new(error.message(lang)).color(DANGER));
                } else if let Some(viewer) = &self.viewer {
                    ui.label(egui::RichText::new(viewer.status(lang)).color(OK));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::Prototype.tr(lang)).color(ACCENT));
                    ui.label(egui::RichText::new(Msg::ZoomHint.tr(lang)).color(DIM));
                });
            });
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let background = rgba(self.settings.colour("background"));
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            painter.rect_filled(rect, 0.0, background);
            let Some(viewer) = self.viewer.as_mut() else {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    Msg::DropHint.tr(lang),
                    egui::FontId::proportional(18.0),
                    DIM,
                );
                return;
            };
            viewer.resize(f64::from(rect.width()), f64::from(rect.height()));
            if self.fit_pending && rect.width() > 40.0 && rect.height() > 40.0 {
                viewer.fit();
                self.fit_pending = false;
            }
            if response.double_clicked() {
                viewer.fit();
            }
            if response.dragged() {
                let d = response.drag_delta();
                viewer.pan_px(f64::from(d.x), f64::from(d.y));
            }
            match response.hover_pos() {
                Some(p) => {
                    let at = p - rect.min;
                    let (sx, sy) = (f64::from(at.x), f64::from(at.y));
                    viewer.hover(sx, sy);
                    let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                    if scroll.abs() > f32::EPSILON || (pinch - 1.0).abs() > 1e-6 {
                        viewer.zoom_at_px(sx, sy, f64::from(scroll), f64::from(pinch));
                    }
                }
                None => viewer.leave(),
            }
            if response.clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    let at = p - rect.min;
                    viewer.click(f64::from(at.x), f64::from(at.y));
                }
            }
            paint(viewer, &painter, rect);
        });
    }
}

/// Draws every primitive of the visible layers that the window shows, in the order of the layers,
/// and the measure over them.
fn paint(viewer: &Viewer, painter: &egui::Painter, rect: Rect) {
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
    // the measure, over everything
    let measure_colour = rgba(settings.colour("measure"));
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
        if let Some(path) = dropped.first() {
            self.path_text = path.to_string_lossy().into_owned();
            self.open(path);
        }
        self.top_bar(ui);
        self.status_bar(ui);
        self.side_panel(ui);
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
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([760.0, 480.0])
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
