// SPDX-License-Identifier: GPL-3.0-or-later
//! The reports of the board editor in the window (built with the `gui` feature). It draws the
//! state of [`AnalysisTool`] with egui and passes on the clicks; the reports are made in
//! `board_analysis`, in code that is tested. This file has not been seen on a screen and has not
//! been used with a mouse.

use eframe::egui;
use eh3d_phoenix_board::Board;
use eh3d_phoenix_rules::Lang;

use crate::board_analysis::{AnalysisKind, AnalysisTool};
use crate::board_analysis_text::AnalysisMsg;
use crate::gui::ui_dim;

/// The tallest the area of the report is, in points.
const REPORT_HEIGHT: f32 = 200.0;

/// The section of the panel of the reports.
pub fn panel(ui: &mut egui::Ui, tool: &mut AnalysisTool, board: &Board, lang: Lang) {
    ui.separator();
    ui.label(egui::RichText::new(AnalysisMsg::Title.tr(lang)).color(ui_dim()));
    ui.label(egui::RichText::new(AnalysisMsg::Hint.tr(lang)).color(ui_dim()));
    ui.horizontal_wrapped(|ui| {
        if ui.button(AnalysisMsg::Statistics.tr(lang)).clicked() {
            tool.run(AnalysisKind::Stats, board, lang);
        }
        if ui.button(AnalysisMsg::TestPoints.tr(lang)).clicked() {
            tool.run(AnalysisKind::TestPoints, board, lang);
        }
        if ui.button(AnalysisMsg::Lengths.tr(lang)).clicked() {
            tool.run(AnalysisKind::Lengths, board, lang);
        }
        if ui.button(AnalysisMsg::Bom.tr(lang)).clicked() {
            tool.run(AnalysisKind::Bom, board, lang);
        }
        if ui.button(AnalysisMsg::Dimensions.tr(lang)).clicked() {
            tool.run(AnalysisKind::Dimensions, board, lang);
        }
        if ui.button(AnalysisMsg::Sourcing.tr(lang)).clicked() {
            tool.run(AnalysisKind::Sourcing, board, lang);
        }
        if tool.shown.is_some() && ui.button(AnalysisMsg::Close.tr(lang)).clicked() {
            tool.clear();
        }
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(AnalysisMsg::PricesFile.tr(lang)).color(ui_dim()));
        ui.text_edit_singleline(&mut tool.prices_path);
    });
    if let Some((_, text)) = &tool.shown {
        // the text of the report goes to the clipboard as it is shown
        if ui.button(AnalysisMsg::Copy.tr(lang)).clicked() {
            ui.ctx().copy_text(text.clone());
        }
    }
    if let Some((_, text)) = &tool.shown {
        egui::ScrollArea::vertical()
            .id_salt("board-analysis")
            .max_height(REPORT_HEIGHT)
            .show(ui, |ui| {
                ui.monospace(text.as_str());
            });
    }
}
