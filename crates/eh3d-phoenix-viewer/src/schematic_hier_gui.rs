// SPDX-License-Identifier: GPL-3.0-or-later
//! The sheets of a hierarchical schematic in the window of the schematic editor (built with the
//! `gui` feature): the row of tabs, the button that goes back out of a sheet, and the drawing of the
//! sheet symbols, their pins, the ports of the sheet and the buses. Every decision is taken in
//! [`crate::schematic_hier`], in code that is tested. This file has not been seen on a screen.

use eframe::egui::{self, Color32, Pos2, Stroke};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schedit::SchEdit;
use eh3d_phoenix_schematic::{Id, Schematic};

use crate::gui::{ui_accent, ui_dim};
use crate::schematic_hier::{marks, tabs, MarkKind, Navigation};
use crate::schematic_hier_text::HierMsg;

const BLOCK: Color32 = Color32::from_rgb(0xc2, 0x8b, 0xff);
const PIN: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const PORT: Color32 = Color32::from_rgb(0x6e, 0xc8, 0xff);
const BUS: Color32 = Color32::from_rgb(0x4d, 0xd0, 0xb0);

/// The row of tabs of the sheets, the button that goes back to the sheet we came through, and the
/// hint of how to open a sheet symbol. Returns true when the sheet that is shown changed.
pub fn tabs_section(
    ui: &mut egui::Ui,
    way: &mut Navigation,
    edit: &mut SchEdit,
    lang: Lang,
) -> bool {
    let mut changed = false;
    let list = tabs(edit.schematic());
    if list.len() > 1 {
        ui.label(
            egui::RichText::new(HierMsg::Sheets.tr(lang))
                .strong()
                .color(ui_accent()),
        );
        let current = edit.sheet();
        let mut picked: Option<Id> = None;
        ui.horizontal_wrapped(|ui| {
            for tab in &list {
                let indent = "> ".repeat(tab.depth);
                let text = if tab.instances > 1 {
                    format!(
                        "{indent}{} ({} {})",
                        tab.name,
                        tab.instances,
                        HierMsg::Instances.tr(lang)
                    )
                } else {
                    format!("{indent}{}", tab.name)
                };
                if ui.selectable_label(Some(tab.id) == current, text).clicked()
                    && Some(tab.id) != current
                {
                    picked = Some(tab.id);
                }
            }
        });
        if let Some(id) = picked {
            way.clear();
            changed = edit.set_sheet(id) || changed;
        }
    }
    if way.depth() > 0 && ui.button(HierMsg::LeaveSheet.tr(lang)).clicked() {
        if let Some(back) = way.leave(edit.schematic()) {
            changed = edit.set_sheet(back) || changed;
        }
    }
    if !edit.schematic().sheet_blocks.is_empty() {
        ui.label(egui::RichText::new(HierMsg::HintEnter.tr(lang)).color(ui_dim()));
    }
    changed
}

/// Draws the sheet symbols, their pins, the ports of the sheet, the buses and their entries of a
/// sheet. `to_screen` turns a point in millimetres into a point of the screen.
pub fn draw_marks(
    painter: &egui::Painter,
    to_screen: &dyn Fn(f64, f64) -> Pos2,
    s: &Schematic,
    sheet: Id,
    text_shown: bool,
) {
    let font = egui::FontId::proportional(11.0);
    for m in marks(s, sheet) {
        let at = to_screen(m.x_mm, m.y_mm);
        match m.kind {
            MarkKind::Block => {
                let far = to_screen(m.x_mm + m.w_mm, m.y_mm + m.h_mm);
                let corners = vec![at, Pos2::new(far.x, at.y), far, Pos2::new(at.x, far.y)];
                painter.add(egui::Shape::closed_line(corners, Stroke::new(1.5, BLOCK)));
                if text_shown {
                    painter.text(
                        Pos2::new(at.x + 2.0, far.y - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        &m.text,
                        font.clone(),
                        BLOCK,
                    );
                }
            }
            MarkKind::Pin => {
                painter.circle_filled(at, 3.0, PIN);
                if text_shown {
                    painter.text(
                        at + egui::vec2(4.0, -2.0),
                        egui::Align2::LEFT_BOTTOM,
                        &m.text,
                        font.clone(),
                        PIN,
                    );
                }
            }
            MarkKind::Port => {
                painter.circle_stroke(at, 4.0, Stroke::new(1.5, PORT));
                if text_shown {
                    painter.text(
                        at + egui::vec2(6.0, -2.0),
                        egui::Align2::LEFT_BOTTOM,
                        &m.text,
                        font.clone(),
                        PORT,
                    );
                }
            }
            MarkKind::Bus => {
                painter.line_segment([at, at + egui::vec2(36.0, 0.0)], Stroke::new(3.0, BUS));
                painter.text(
                    at + egui::vec2(0.0, -4.0),
                    egui::Align2::LEFT_BOTTOM,
                    &m.text,
                    font.clone(),
                    BUS,
                );
            }
            MarkKind::Entry => {
                painter.circle_filled(at, 2.5, BUS);
                if text_shown {
                    painter.text(
                        at + egui::vec2(4.0, -2.0),
                        egui::Align2::LEFT_BOTTOM,
                        &m.text,
                        font.clone(),
                        BUS,
                    );
                }
            }
        }
    }
}
