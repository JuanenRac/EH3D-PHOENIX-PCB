// SPDX-License-Identifier: GPL-3.0-or-later
//! The zone tool of the board editor in the window (built with the `gui` feature). It draws the
//! state of [`ZoneTool`] with egui, with the polygon that is being drawn and the preview of the
//! fill, and passes on the clicks; every decision is taken in `board_zone` and in
//! `eh3d-phoenix-boardedit`, in code that is tested. This file has not been seen on a screen and
//! has not been used with a mouse.

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_boardedit::EditState;
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::board_shove::ShoveTool;
use crate::board_zone::{ZoneMode, ZoneTool};
use crate::board_zone_text::ZoneMsg;

const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const FILLED: Color32 = Color32::from_rgb(0x5f, 0xd0, 0xb0);
const HOLE: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);

/// Draws the polygon that is being drawn (its vertices as small circles, closed when it is) and
/// the polygons of copper of the preview of the fill.
pub fn draw(painter: &egui::Painter, tool: &ZoneTool, to_screen: &dyn Fn(Point) -> Pos2) {
    if !tool.on {
        return;
    }
    if let Some(preview) = &tool.preview {
        for (_, polygons) in &preview.zones {
            for polygon in polygons {
                if polygon.len() >= 3 {
                    let points: Vec<Pos2> = polygon.iter().map(|p| to_screen(*p)).collect();
                    painter.add(egui::Shape::closed_line(points, Stroke::new(1.0, FILLED)));
                }
            }
        }
    }
    if tool.points.is_empty() {
        return;
    }
    let colour = match tool.mode {
        ZoneMode::Outline => ACCENT,
        ZoneMode::Cutout => HOLE,
    };
    let points: Vec<Pos2> = tool.points.iter().map(|p| to_screen(*p)).collect();
    for p in &points {
        painter.circle_stroke(*p, 4.0, Stroke::new(1.0, colour));
    }
    if points.len() >= 2 {
        if tool.closed {
            painter.add(egui::Shape::closed_line(points, Stroke::new(1.5, colour)));
        } else {
            painter.add(egui::Shape::line(points, Stroke::new(1.5, colour)));
        }
    }
}

/// The section of the panel of the tools for the zones.
pub fn panel(ui: &mut egui::Ui, tool: &mut ZoneTool, state: &mut EditState, lang: Lang) {
    ui.separator();
    ui.checkbox(&mut tool.on, ZoneMsg::Mode.tr(lang));
    if !tool.on {
        return;
    }
    let nets = ShoveTool::nets(state);
    if nets.is_empty() {
        ui.label(egui::RichText::new(ZoneMsg::NoNets.tr(lang)).color(DIM));
    } else {
        let mut chosen = tool.net_index.min(nets.len() - 1);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(ZoneMsg::NetLabel.tr(lang)).color(DIM));
            egui::ComboBox::from_id_salt("board-zone")
                .selected_text(nets[chosen].1.clone())
                .show_ui(ui, |ui| {
                    for (i, (_, name)) in nets.iter().enumerate() {
                        ui.selectable_value(&mut chosen, i, name.as_str());
                    }
                });
        });
        tool.net_index = chosen;
    }
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(ZoneMsg::Priority.tr(lang)).color(DIM));
        ui.add(egui::DragValue::new(&mut tool.priority).range(-100..=100));
    });
    let mut cutout = tool.mode == ZoneMode::Cutout;
    if ui
        .checkbox(&mut cutout, ZoneMsg::CutoutMode.tr(lang))
        .changed()
    {
        tool.mode = if cutout {
            ZoneMode::Cutout
        } else {
            ZoneMode::Outline
        };
    }
    ui.label(
        egui::RichText::new(format!(
            "{} {} · {}",
            tool.points.len(),
            ZoneMsg::Points.tr(lang),
            ZoneMsg::Hint.tr(lang)
        ))
        .color(DIM),
    );
    ui.horizontal_wrapped(|ui| {
        if ui.button(ZoneMsg::Close.tr(lang)).clicked() {
            tool.close();
        }
        let apply = match (tool.mode, tool.editing) {
            (ZoneMode::Cutout, _) => ZoneMsg::ApplyCutout,
            (ZoneMode::Outline, Some(_)) => ZoneMsg::ApplyOutline,
            (ZoneMode::Outline, None) => ZoneMsg::Add,
        };
        if ui.button(apply.tr(lang)).clicked() {
            tool.accept(state);
        }
        if ui.button(ZoneMsg::RemoveLast.tr(lang)).clicked() {
            tool.remove_last();
        }
        if ui.button(ZoneMsg::Clear.tr(lang)).clicked() {
            tool.clear();
        }
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button(ZoneMsg::Preview.tr(lang)).clicked() {
            tool.refresh_preview(state);
        }
        if ui.button(ZoneMsg::Fill.tr(lang)).clicked() {
            tool.fill(state);
        }
    });
    if let Some(preview) = &tool.preview {
        ui.label(
            egui::RichText::new(format!(
                "{} {} · {} {}",
                preview.polygons(),
                ZoneMsg::Polygons.tr(lang),
                preview.islands_dropped,
                ZoneMsg::Dropped.tr(lang)
            ))
            .color(FILLED),
        );
    }
    let rows = ZoneTool::rows(state);
    if rows.is_empty() {
        return;
    }
    ui.label(egui::RichText::new(ZoneMsg::Order.tr(lang)).color(DIM));
    for row in rows {
        let mut text = format!("{} · {} · {}", row.priority, row.net, row.layer);
        if !row.filled {
            text.push_str(&format!(" · {}", ZoneMsg::NoFill.tr(lang)));
        }
        if ui
            .selectable_label(tool.selected == Some(row.id), text)
            .clicked()
        {
            tool.selected = Some(row.id);
        }
        if tool.selected == Some(row.id) {
            ui.horizontal_wrapped(|ui| {
                if ui.button(ZoneMsg::Raise.tr(lang)).clicked() {
                    tool.shift_priority(state, row.id, true);
                }
                if ui.button(ZoneMsg::Lower.tr(lang)).clicked() {
                    tool.shift_priority(state, row.id, false);
                }
                if ui.button(ZoneMsg::EditVertices.tr(lang)).clicked() {
                    tool.begin_edit(state, row.id);
                }
                if ui.button(ZoneMsg::Remove.tr(lang)).clicked() {
                    tool.remove(state, row.id);
                }
            });
        }
    }
    for line in ZoneTool::issue_lines(state, lang).iter().take(4) {
        ui.label(egui::RichText::new(line.as_str()).color(DIM));
    }
}
