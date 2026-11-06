// SPDX-License-Identifier: GPL-3.0-or-later
//! The start screen of the window (built with the `gui` feature): it draws the state of
//! [`Launcher`] and passes on what the person asks for. What a typed path, a dropped file or a
//! recent file means is decided in `launcher_session`, in code that is tested. This file has not
//! been seen on a screen.

use eframe::egui;
use eh3d_phoenix_rules::Lang;

use crate::gui::{ACCENT, DANGER, DIM};
use crate::launcher_session::{kind_text, Launch, Launcher};
use crate::launcher_text::LauncherText;
use crate::msg::Msg;
use crate::panel_text::PanelText;

/// Draws the start screen in the space left by the panels. The note is the last thing that
/// could not be done (an empty path, dropped files that cannot be opened). The answer is what
/// the person asked for in this frame, if anything.
pub fn show(
    ui: &mut egui::Ui,
    launcher: &mut Launcher,
    lang: Lang,
    note: &mut Option<LauncherText>,
) -> Option<Launch> {
    let mut launch = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(28.0);
        ui.label(
            egui::RichText::new("EH3D-PHOENIX-PCB")
                .strong()
                .size(30.0)
                .color(ACCENT),
        );
        ui.label(egui::RichText::new(LauncherText::Welcome.tr(lang)).size(18.0));
        ui.add_space(16.0);

        ui.label(egui::RichText::new(LauncherText::TypePath.tr(lang)).color(DIM));
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut launcher.typed).desired_width(440.0);
            let response = ui.add(edit);
            let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button(Msg::Open.tr(lang)).clicked() || enter {
                match launcher.typed_launch() {
                    Ok(asked) => {
                        *note = None;
                        launch = Some(asked);
                    }
                    Err(text) => *note = Some(text),
                }
            }
            if ui.button(LauncherText::GerberFolder.tr(lang)).clicked() {
                match launcher.gerber_launch() {
                    Ok(asked) => {
                        *note = None;
                        launch = Some(asked);
                    }
                    Err(text) => *note = Some(text),
                }
            }
            if ui.button(PanelText::Open.tr(lang)).clicked() {
                match launcher.panel_launch() {
                    Ok(asked) => {
                        *note = None;
                        launch = Some(asked);
                    }
                    Err(text) => *note = Some(text),
                }
            }
            if ui.button(LauncherText::NewLibrary.tr(lang)).clicked() {
                *note = None;
                launch = Some(Launch::NewLibrary);
            }
        });
        ui.label(egui::RichText::new(LauncherText::DropHere.tr(lang)).color(DIM));
        ui.label(egui::RichText::new(LauncherText::GerberHint.tr(lang)).color(DIM));
        ui.label(egui::RichText::new(PanelText::Hint.tr(lang)).color(DIM));
        if let Some(text) = *note {
            ui.label(egui::RichText::new(text.tr(lang)).color(DANGER));
        }

        ui.add_space(18.0);
        ui.separator();
        ui.label(
            egui::RichText::new(LauncherText::Recent.tr(lang))
                .strong()
                .color(ACCENT),
        );
        let rows = launcher.rows(&|path| path.exists());
        if rows.is_empty() {
            ui.label(egui::RichText::new(LauncherText::NoRecent.tr(lang)).color(DIM));
        }
        let mut forget = None;
        for row in &rows {
            ui.push_id(("recent", row.index), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(row.exists, egui::Button::new(row.name.as_str()))
                        .clicked()
                    {
                        *note = None;
                        launch = Some(Launch::Open(row.path.clone()));
                    }
                    if let Some(kind) = row.kind {
                        ui.label(egui::RichText::new(kind_text(kind).tr(lang)).color(ACCENT));
                    }
                    ui.label(egui::RichText::new(row.folder.as_str()).color(DIM));
                    if !row.exists {
                        ui.label(egui::RichText::new(LauncherText::Missing.tr(lang)).color(DANGER));
                    }
                    if ui.small_button(LauncherText::Forget.tr(lang)).clicked() {
                        forget = Some(row.index);
                    }
                });
            });
        }
        if let Some(index) = forget {
            launcher.forget(index);
        }
        if !rows.is_empty()
            && ui
                .small_button(LauncherText::ClearRecent.tr(lang))
                .clicked()
        {
            launcher.clear();
        }
        if launcher.save_failed {
            ui.label(egui::RichText::new(LauncherText::SaveFailed.tr(lang)).color(DANGER));
        }
    });
    launch
}
