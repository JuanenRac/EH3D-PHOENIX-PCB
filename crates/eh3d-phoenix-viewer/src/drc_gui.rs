// SPDX-License-Identifier: GPL-3.0-or-later
//! The panel of problems of a board in the window (built with the `gui` feature). It draws the
//! state of [`DrcPanel`] with egui and passes on what is clicked; the filters, the order, the
//! current problem and the step to the next one are decided in `drc_panel` and in `Workspace`, in
//! code that is tested. This file has not been seen on a screen and has not been used with a
//! mouse.

use eframe::egui;
use eh3d_phoenix_rules::{Lang, Severity};

use crate::drc_panel::{DrcPanel, DrcSort, FindingKey};
use crate::drc_text::{category_name, DrcMsg};
use crate::editmsg::EditMsg;
use crate::gui::{ui_accent, ui_danger, ui_dim, ui_text};
use crate::workspace::{finding_line, Doc, Workspace};

/// What the person asked for in a frame.
enum Action {
    /// Go to the next problem (true) or the previous one (false).
    Step(bool),
    /// Go to the problem of a row.
    GoTo(FindingKey, Option<[i64; 2]>),
}

fn heading(text: String) -> egui::RichText {
    egui::RichText::new(text).strong().color(ui_accent())
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ui_dim())
}

/// The list of the problems of the rules of a board with its filters, its order and the buttons
/// that go to the next and the previous problem. Nothing is drawn for a schematic.
pub fn section(ui: &mut egui::Ui, workspace: &mut Workspace, lang: Lang) {
    let listed = workspace.viewer.settings.findings_shown;
    let mut action: Option<Action> = None;
    {
        let Workspace { doc, drc, .. } = &mut *workspace;
        let Doc::Board(session) = doc else {
            return;
        };
        let findings = &session.findings;
        let total = findings.len();
        let counts = DrcPanel::counts(findings);
        let (errors, warnings, information) = counts.iter().fold((0, 0, 0), |sum, c| {
            (sum.0 + c.errors, sum.1 + c.warnings, sum.2 + c.information)
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut drc.filter.errors,
                format!("{} ({errors})", EditMsg::SevError.tr(lang)),
            );
            ui.checkbox(
                &mut drc.filter.warnings,
                format!("{} ({warnings})", EditMsg::SevWarning.tr(lang)),
            );
            ui.checkbox(
                &mut drc.filter.information,
                format!("{} ({information})", EditMsg::SevInfo.tr(lang)),
            );
        });
        if !counts.is_empty() {
            ui.label(dim(DrcMsg::Categories.tr(lang)));
            ui.horizontal_wrapped(|ui| {
                for count in &counts {
                    let shown = drc.filter.category_shown(count.name);
                    let label = format!("{} ({})", category_name(count.name, lang), count.total());
                    if ui.selectable_label(shown, label).clicked() {
                        drc.filter.toggle_category(count.name);
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(dim(DrcMsg::Order.tr(lang)));
            egui::ComboBox::from_id_salt("drc-order")
                .selected_text(DrcMsg::of_sort(drc.sort).tr(lang))
                .show_ui(ui, |ui| {
                    for sort in DrcSort::ALL {
                        ui.selectable_value(&mut drc.sort, sort, DrcMsg::of_sort(sort).tr(lang));
                    }
                });
            if ui
                .add_enabled(
                    !drc.filter.is_all(),
                    egui::Button::new(DrcMsg::ShowAll.tr(lang)),
                )
                .clicked()
            {
                drc.filter.show_all();
            }
        });
        let visible = drc.visible(findings);
        ui.label(heading(format!(
            "{} ({} {} / {total})",
            DrcMsg::Title.tr(lang),
            visible.len(),
            DrcMsg::Shown.tr(lang)
        )));
        ui.horizontal_wrapped(|ui| {
            if ui.button(DrcMsg::Previous.tr(lang)).clicked() {
                action = Some(Action::Step(false));
            }
            if ui.button(DrcMsg::Next.tr(lang)).clicked() {
                action = Some(Action::Step(true));
            }
        });
        if total == 0 {
            ui.label(dim(EditMsg::NoFindings.tr(lang)));
        } else if visible.is_empty() {
            ui.label(dim(DrcMsg::NoMatch.tr(lang)));
        } else if !visible.iter().any(|f| f.at_nm.is_some()) {
            ui.label(dim(DrcMsg::NoPlace.tr(lang)));
        }
        for (i, finding) in visible.iter().take(listed).enumerate() {
            ui.push_id(("drc", i), |ui| {
                let colour = match finding.severity {
                    Severity::Error => ui_danger(),
                    Severity::Warning => ui_accent(),
                    Severity::Info => ui_text(),
                };
                let current = drc.is_current(finding);
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button(EditMsg::GoTo.tr(lang)).clicked() {
                        action = Some(Action::GoTo(FindingKey::of(finding), finding.at_nm));
                    }
                    let text = egui::RichText::new(finding_line(finding, lang)).color(colour);
                    let text = if current { text.strong() } else { text };
                    ui.add(egui::Label::new(text).wrap());
                });
            });
        }
        if visible.len() > listed {
            ui.label(dim(format!("… {}", visible.len() - listed)));
        }
    }
    match action {
        Some(Action::Step(forward)) => {
            workspace.drc_step(forward);
        }
        Some(Action::GoTo(key, at_nm)) => {
            workspace.drc_go_to(key, at_nm);
        }
        None => {}
    }
}
