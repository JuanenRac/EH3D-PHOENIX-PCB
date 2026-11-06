// SPDX-License-Identifier: GPL-3.0-or-later
//! The export of images and PDF in the window (built with the `gui` feature): a format, the place
//! to write to and a button. The pages are made and written in [`crate::print_export`], in code
//! that is tested; this file only draws the controls. It has not been seen on a screen.

use std::path::{Path, PathBuf};

use eframe::egui;
use eh3d_phoenix_print::{Format, PrintConfig};
use eh3d_phoenix_rules::Lang;

use crate::gui::{ui_danger, ui_dim};
use crate::print_export::{default_config, export, suggested_path};
use crate::print_text::PrintMsg;
use crate::workspace::Workspace;

/// The state of the controls of the export.
pub struct PrintUi {
    pdf: bool,
    target: String,
    proposed_for: Option<(Option<PathBuf>, bool)>,
    note: Option<(bool, String)>,
}

impl PrintUi {
    /// The controls, with a PDF as the format.
    pub fn new() -> PrintUi {
        PrintUi {
            pdf: true,
            target: String::new(),
            proposed_for: None,
            note: None,
        }
    }

    fn format(&self) -> Format {
        if self.pdf {
            Format::Pdf
        } else {
            Format::Svg
        }
    }

    /// Proposes a place again when the file or the format changed since the last time.
    fn propose(&mut self, input: Option<&Path>) {
        let key = (input.map(Path::to_path_buf), self.pdf);
        if self.proposed_for.as_ref() != Some(&key) {
            self.target = suggested_path(input, self.format())
                .to_string_lossy()
                .into_owned();
            self.proposed_for = Some(key);
        }
    }

    fn run(&mut self, w: &Workspace, lang: Lang) {
        let result = default_config().and_then(|cfg: PrintConfig| {
            export(w, Path::new(self.target.trim()), self.format(), &cfg, lang)
        });
        self.note = Some(match result {
            Ok(paths) => {
                let names: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                (
                    true,
                    format!("{}: {}", PrintMsg::Written.tr(lang), names.join(", ")),
                )
            }
            Err(error) => (false, error.message(lang)),
        });
    }

    /// Draws the controls and exports when the button is pressed.
    pub fn show(&mut self, ui: &mut egui::Ui, w: &Workspace, lang: Lang) {
        self.propose(w.input.as_deref());
        ui.label(egui::RichText::new(PrintMsg::Target.tr(lang)).color(ui_dim()));
        ui.add(egui::TextEdit::singleline(&mut self.target).desired_width(200.0));
        ui.radio_value(&mut self.pdf, true, "PDF");
        ui.radio_value(&mut self.pdf, false, "SVG");
        if ui.button(PrintMsg::Export.tr(lang)).clicked() {
            self.run(w, lang);
        }
        if let Some((ok, text)) = &self.note {
            let colour = if *ok { ui_dim() } else { ui_danger() };
            ui.label(egui::RichText::new(text.as_str()).color(colour));
        }
    }
}

impl Default for PrintUi {
    fn default() -> Self {
        PrintUi::new()
    }
}
