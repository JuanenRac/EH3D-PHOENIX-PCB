// SPDX-License-Identifier: GPL-3.0-or-later
//! The push tool of the board editor in the window (built with the `gui` feature). It draws the
//! state of [`ShoveTool`] with egui, with a preview of the tracks that the push would move, and
//! passes on the clicks; every decision is taken in `board_shove` and in `eh3d-phoenix-boardedit`,
//! in code that is tested. This file has not been seen on a screen and has not been used with a
//! mouse.

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_boardedit::EditState;
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::board_shove::ShoveTool;
use crate::board_shove_text::ShoveMsg;

const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const MOVED: Color32 = Color32::from_rgb(0x5f, 0xd0, 0xb0);

/// Draws the points of the new track, the line through them and the preview: the tracks that
/// would move in their place now (thin, dim) and where the push would put them (thick).
pub fn draw(painter: &egui::Painter, tool: &ShoveTool, to_screen: &dyn Fn(Point) -> Pos2) {
    if !tool.on {
        return;
    }
    if let Some(preview) = &tool.preview {
        for moved in &preview.moves {
            for piece in &moved.before {
                painter.line_segment(
                    [to_screen(piece.start()), to_screen(piece.end())],
                    Stroke::new(1.0, DIM),
                );
            }
            for piece in &moved.after {
                painter.line_segment(
                    [to_screen(piece.start()), to_screen(piece.end())],
                    Stroke::new(2.5, MOVED),
                );
            }
        }
    }
    if tool.points.is_empty() {
        return;
    }
    let points: Vec<Pos2> = tool.points.iter().map(|p| to_screen(*p)).collect();
    for p in &points {
        painter.circle_stroke(*p, 4.0, Stroke::new(1.0, ACCENT));
    }
    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, Stroke::new(1.5, ACCENT)));
    }
}

/// The section of the panel of the tools for the push.
pub fn panel(ui: &mut egui::Ui, tool: &mut ShoveTool, state: &mut EditState, lang: Lang) {
    ui.separator();
    ui.checkbox(&mut tool.on, ShoveMsg::Mode.tr(lang));
    if !tool.on {
        return;
    }
    let nets = ShoveTool::nets(state);
    if nets.is_empty() {
        ui.label(egui::RichText::new(ShoveMsg::NoNets.tr(lang)).color(DIM));
        return;
    }
    let before = tool.selected.min(nets.len() - 1);
    let mut chosen = before;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(ShoveMsg::NetLabel.tr(lang)).color(DIM));
        egui::ComboBox::from_id_salt("board-shove")
            .selected_text(nets[chosen].1.clone())
            .show_ui(ui, |ui| {
                for (i, (_, name)) in nets.iter().enumerate() {
                    ui.selectable_value(&mut chosen, i, name.as_str());
                }
            });
    });
    tool.selected = chosen;
    if chosen != before {
        tool.refresh(state);
    }
    ui.label(
        egui::RichText::new(format!(
            "{} {} · {}",
            tool.points.len(),
            ShoveMsg::Points.tr(lang),
            ShoveMsg::Hint.tr(lang)
        ))
        .color(DIM),
    );
    if tool.preview.is_some() {
        let text = if tool.moved() == 0 {
            ShoveMsg::Nothing.tr(lang).to_owned()
        } else {
            format!("{} {}", tool.moved(), ShoveMsg::Moves.tr(lang))
        };
        ui.label(egui::RichText::new(text).color(MOVED));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(ShoveMsg::Apply.tr(lang)).clicked() {
            tool.accept(state);
        }
        if ui.button(ShoveMsg::RemoveLast.tr(lang)).clicked() {
            tool.remove_last(state);
        }
        if ui.button(ShoveMsg::Clear.tr(lang)).clicked() {
            tool.clear();
        }
    });
}
