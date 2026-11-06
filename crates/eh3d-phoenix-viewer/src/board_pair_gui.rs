// SPDX-License-Identifier: GPL-3.0-or-later
//! The pair tool of the board editor in the window (built with the `gui` feature). It draws the
//! state of [`PairTool`] with egui and passes on the clicks; every decision is taken in
//! `board_pair` and in `eh3d-phoenix-boardedit`, in code that is tested. This file has not been seen
//! on a screen and has not been used with a mouse.

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_boardedit::{EditState, PairConfig};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::board_pair::PairTool;
use crate::board_pair_text::PairMsg;

const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);

/// The most findings that the panel lists.
const LISTED: usize = 6;

/// The settings and the pairs of the pair tool: `config/diffpairs.cfg` when it is there and good,
/// else the built-in ones.
pub fn load_config() -> PairConfig {
    let path = "config/diffpairs.cfg";
    let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= 1 << 20);
    if !small {
        return PairConfig::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| PairConfig::parse(&text).ok())
        .unwrap_or_default()
}

/// Draws the points of the centre line that were clicked and the line through them.
pub fn draw(painter: &egui::Painter, tool: &PairTool, to_screen: &dyn Fn(Point) -> Pos2) {
    if !tool.on || tool.points.is_empty() {
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

/// The section of the panel of the tools for the pairs.
pub fn panel(ui: &mut egui::Ui, tool: &mut PairTool, state: &mut EditState, lang: Lang) {
    ui.separator();
    ui.checkbox(&mut tool.on, PairMsg::Mode.tr(lang));
    if !tool.on {
        return;
    }
    let names = PairTool::pair_names(state);
    if names.is_empty() {
        ui.label(egui::RichText::new(PairMsg::NoPairs.tr(lang)).color(DIM));
        return;
    }
    let mut chosen = tool.selected.min(names.len() - 1);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(PairMsg::PairLabel.tr(lang)).color(DIM));
        egui::ComboBox::from_id_salt("board-pair")
            .selected_text(names[chosen].clone())
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    ui.selectable_value(&mut chosen, i, name.as_str());
                }
            });
    });
    tool.selected = chosen;
    ui.label(
        egui::RichText::new(format!(
            "{} {} · {}",
            tool.points.len(),
            PairMsg::Points.tr(lang),
            PairMsg::Hint.tr(lang)
        ))
        .color(DIM),
    );
    ui.horizontal_wrapped(|ui| {
        if ui.button(PairMsg::Draw.tr(lang)).clicked() {
            tool.route(state);
        }
        if ui.button(PairMsg::Tune.tr(lang)).clicked() {
            tool.tune(state);
        }
        if ui.button(PairMsg::FixGap.tr(lang)).clicked() {
            tool.fix_gap(state);
        }
        if ui.button(PairMsg::Check.tr(lang)).clicked() {
            tool.check(state);
        }
        if ui.button(PairMsg::Clear.tr(lang)).clicked() {
            tool.clear();
        }
    });
    if let Some(report) = &tool.report {
        let (errors, warnings) = report.counts();
        ui.label(
            egui::RichText::new(format!(
                "{errors} {} · {warnings} {}",
                PairMsg::Errors.tr(lang),
                PairMsg::Warnings.tr(lang)
            ))
            .color(if errors > 0 { DANGER } else { DIM }),
        );
        for finding in report.findings.iter().take(LISTED) {
            ui.label(egui::RichText::new(format!("{} {}", finding.code, finding.pair)).color(DIM));
        }
    }
}
