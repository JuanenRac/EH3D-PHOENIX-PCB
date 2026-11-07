// SPDX-License-Identifier: GPL-3.0-or-later
//! The window of the settings files (the Rules window, built with the `gui` feature). It draws the
//! state of [`CfgEditor`] with egui and passes on what is typed or clicked: which file is open,
//! the table of its settings with the value, the default, the range that the program allows and
//! the comment of each, the reason and the line when the program refuses the text, the buttons
//! that put a default back or add a missing setting, and the saving under another name. What is
//! valid, what the range is and what may be written are decided in `cfg_edit` and in the readers
//! of the program, in code that is tested. This file has not been seen on a screen and has not
//! been used with a mouse.

use eframe::egui;
use eh3d_phoenix_rules::Lang;

use crate::cfg_edit::{CfgEditor, CfgNote};
use crate::cfg_kinds::KINDS;
use crate::cfg_text::CfgMsg;
use crate::gui::{ui_accent, ui_danger, ui_dim, ui_ok};

/// What the person asked for in a frame.
enum Action {
    /// A value was typed on a line.
    Edit(usize, String),
    /// The default was asked for on a line.
    Restore(usize),
    /// A missing setting was asked to be added.
    Add(String),
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ui_accent())
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ui_dim())
}

/// Draws the window. The answer is false when the person closed it.
pub fn window(ctx: &egui::Context, editor: &mut CfgEditor, lang: Lang) -> bool {
    let mut open = true;
    egui::Window::new(CfgMsg::Title.tr(lang))
        .open(&mut open)
        .default_width(860.0)
        .show(ctx, |ui| body(ui, editor, lang));
    open
}

fn body(ui: &mut egui::Ui, editor: &mut CfgEditor, lang: Lang) {
    let mut chosen: Option<usize> = None;
    let mut reload = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(dim(CfgMsg::Folder.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut editor.dir).desired_width(160.0));
        ui.label(dim(CfgMsg::File.tr(lang)));
        let selected = editor.selected;
        egui::ComboBox::from_id_salt("cfg-file")
            .selected_text(KINDS[selected].file)
            .show_ui(ui, |ui| {
                for (i, kind) in KINDS.iter().enumerate() {
                    if ui.selectable_label(i == selected, kind.file).clicked() {
                        chosen = Some(i);
                    }
                }
            });
        if ui.button(CfgMsg::Reload.tr(lang)).clicked() {
            reload = true;
        }
    });
    if let Some(i) = chosen {
        editor.select(i);
    } else if reload {
        editor.reload();
    }
    if let Some(note) = &editor.note {
        let (text, colour) = match note {
            CfgNote::Read(path) => (
                format!("{}: {}", CfgMsg::Read.tr(lang), path.display()),
                ui_dim(),
            ),
            CfgNote::NotFound(path) => (
                format!("{} ({})", CfgMsg::NotFound.tr(lang), path.display()),
                ui_dim(),
            ),
            CfgNote::Saved(path) => (
                format!("{}: {}", CfgMsg::Saved.tr(lang), path.display()),
                ui_ok(),
            ),
            CfgNote::Failed(error) => (error.message(lang), ui_danger()),
        };
        ui.add(egui::Label::new(egui::RichText::new(text).color(colour)).wrap());
    }
    // what the program says about the text as it is now
    let issue = editor.doc.issue();
    match &issue {
        None => {
            ui.label(egui::RichText::new(CfgMsg::Valid.tr(lang)).color(ui_ok()));
        }
        Some(issue) => {
            let mut text = format!("{}:", CfgMsg::NotValid.tr(lang));
            if !issue.code.is_empty() {
                text.push_str(&format!(" {}", issue.code));
            }
            if let Some(line) = issue.line {
                text.push_str(&format!(" ({} {line})", CfgMsg::Line.tr(lang)));
            }
            text.push_str(&format!(" {}", issue.detail));
            ui.add(egui::Label::new(egui::RichText::new(text).color(ui_danger())).wrap());
        }
    }
    let changed = editor.doc.changed_count(editor.ranges());
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut editor.only_changed, CfgMsg::OnlyChanged.tr(lang));
        ui.label(dim(format!("{changed} {}", CfgMsg::Changed.tr(lang))));
        let modified = editor.doc.is_modified();
        if ui
            .add_enabled(modified, egui::Button::new(CfgMsg::Discard.tr(lang)))
            .clicked()
        {
            editor.doc.revert();
        }
    });
    ui.separator();
    let rows = editor.rows();
    let mut actions: Vec<Action> = Vec::new();
    egui::ScrollArea::vertical()
        .max_height(380.0)
        .show(ui, |ui| {
            egui::Grid::new("cfg-table")
                .striped(true)
                .num_columns(6)
                .show(ui, |ui| {
                    ui.label(heading(CfgMsg::Setting.tr(lang)));
                    ui.label(heading(CfgMsg::Value.tr(lang)));
                    ui.label("");
                    ui.label(heading(CfgMsg::Default.tr(lang)));
                    ui.label(heading(CfgMsg::Allowed.tr(lang)));
                    ui.label(heading(CfgMsg::Help.tr(lang)));
                    ui.end_row();
                    for row in &rows {
                        let mut name = egui::RichText::new(row.key.clone());
                        if row.changed {
                            name = name.strong();
                        }
                        if row.refused {
                            name = name.color(ui_danger());
                        }
                        ui.label(name);
                        let mut text = row.value.clone();
                        let mut edit = egui::TextEdit::singleline(&mut text).desired_width(150.0);
                        if row.refused {
                            edit = edit.text_color(ui_danger());
                        }
                        if ui.add(edit).changed() {
                            actions.push(Action::Edit(row.line, text));
                        }
                        if row.changed && row.default.is_some() {
                            if ui.small_button(CfgMsg::Restore.tr(lang)).clicked() {
                                actions.push(Action::Restore(row.line));
                            }
                        } else {
                            ui.label("");
                        }
                        ui.label(dim(row.default.clone().unwrap_or_else(|| "-".to_owned())));
                        let range = row
                            .range
                            .as_ref()
                            .map_or_else(|| "-".to_owned(), |r| r.describe());
                        if row.range.as_ref().is_some_and(|r| r.has_gaps()) {
                            let shown = ui.label(dim(format!("{range} *")));
                            let _ = shown.on_hover_text(CfgMsg::Gaps.tr(lang));
                        } else {
                            ui.label(dim(range));
                        }
                        let help = ui.add(egui::Label::new(dim(row.help.clone())).truncate());
                        if !row.help.is_empty() {
                            let _ = help.on_hover_text(row.help.clone());
                        }
                        ui.end_row();
                    }
                });
        });
    ui.add(egui::Label::new(dim(CfgMsg::RangeNote.tr(lang))).wrap());
    let missing = editor.doc.missing();
    if !missing.is_empty() {
        ui.collapsing(CfgMsg::Missing.tr(lang), |ui| {
            ui.horizontal_wrapped(|ui| {
                for key in &missing {
                    let label = format!("{} {key}", CfgMsg::Add.tr(lang));
                    if ui.button(label).clicked() {
                        actions.push(Action::Add(key.clone()));
                    }
                }
            });
        });
    }
    ui.separator();
    let mut save = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(dim(CfgMsg::SaveAs.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut editor.save_text).desired_width(360.0));
        ui.checkbox(&mut editor.overwrite, CfgMsg::Replace.tr(lang));
        let valid = issue.is_none();
        if ui
            .add_enabled(valid, egui::Button::new(CfgMsg::SaveAs.tr(lang)))
            .clicked()
        {
            save = true;
        }
    });
    for action in actions {
        match action {
            Action::Edit(line, text) => {
                let _ = editor.doc.set_value(line, &text);
            }
            Action::Restore(line) => {
                let _ = editor.doc.reset_value(line);
            }
            Action::Add(key) => {
                let _ = editor.doc.add_missing(&key);
            }
        }
    }
    if save {
        editor.save();
    }
}
