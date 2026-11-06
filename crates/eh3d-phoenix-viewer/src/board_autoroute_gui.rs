// SPDX-License-Identifier: GPL-3.0-or-later
//! The autorouter of the board editor in the window (built with the `gui` feature). It draws the
//! preview of [`AutorouteTool`] with egui and passes on the buttons; every decision is taken in
//! `board_autoroute` and in `eh3d-phoenix-autoroute`, in code that is tested. This file has not
//! been seen on a screen and has not been used with a mouse.

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_boardedit::EditState;
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::board_autoroute::AutorouteTool;
use crate::board_autoroute_text::AutorouteMsg;
use crate::gui::{ui_accent, ui_danger, ui_dim};

const PREVIEW: Color32 = Color32::from_rgb(0x6f, 0xd0, 0x8a);

/// Draws the tracks and the vias that the autorouter would add, when the preview is still good.
pub fn draw(
    painter: &egui::Painter,
    tool: &AutorouteTool,
    state: &EditState,
    to_screen: &dyn Fn(Point) -> Pos2,
) {
    let Some(plan) = tool.current(state) else {
        return;
    };
    for track in &plan.tracks {
        let points: Vec<Pos2> = track.points.iter().map(|p| to_screen(*p)).collect();
        painter.add(egui::Shape::line(points, Stroke::new(2.5, PREVIEW)));
    }
    for via in &plan.vias {
        painter.circle_stroke(to_screen(via.at), 4.0, Stroke::new(1.5, PREVIEW));
    }
}

/// The section of the panel of the tools for the autorouter.
pub fn panel(ui: &mut egui::Ui, tool: &mut AutorouteTool, state: &mut EditState, lang: Lang) {
    ui.separator();
    ui.label(
        egui::RichText::new(AutorouteMsg::Title.tr(lang))
            .strong()
            .color(ui_accent()),
    );
    ui.label(egui::RichText::new(AutorouteMsg::Hint.tr(lang)).color(ui_dim()));
    if let Some(error) = &tool.error {
        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                AutorouteMsg::Refused.tr(lang),
                error.message(lang.tag())
            ))
            .color(ui_danger()),
        );
    }
    if let Some(plan) = tool.current(state) {
        let r = &plan.report;
        let text = if r.connections_total == 0 {
            AutorouteMsg::Nothing.tr(lang).to_owned()
        } else {
            format!(
                "{}/{} {} · {} {} · {} {} · {} {}",
                r.connections_routed,
                r.connections_total,
                AutorouteMsg::Routed.tr(lang),
                r.tracks_added,
                AutorouteMsg::Tracks.tr(lang),
                r.vias_added,
                AutorouteMsg::Vias.tr(lang),
                r.unrouted.len(),
                AutorouteMsg::Unrouted.tr(lang)
            )
        };
        ui.label(egui::RichText::new(text).color(PREVIEW));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(AutorouteMsg::Run.tr(lang)).clicked() {
            tool.preview(state);
        }
        let ready = tool.current(state).is_some();
        if ui
            .add_enabled(ready, egui::Button::new(AutorouteMsg::Apply.tr(lang)))
            .clicked()
        {
            tool.accept(state);
        }
        if ui
            .add_enabled(ready, egui::Button::new(AutorouteMsg::Discard.tr(lang)))
            .clicked()
        {
            tool.clear();
        }
    });
}
