// SPDX-License-Identifier: GPL-3.0-or-later
//! The tools «Bus» and «Bus entry» in the panel of the schematic editor (built with the `gui`
//! feature): the two buttons, the name of the bus to place, and the choice of the bus and the member
//! for an entry. What a click does is decided in [`crate::schematic_bus`], in code that is tested.
//! This file has not been seen on a screen.

use eframe::egui;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schedit::SchEdit;

use crate::gui::ui_dim;
use crate::schematic_bus::{buses_on, members_of, BusMode, BusUi};
use crate::schematic_hier_text::HierMsg;

fn dim(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(ui_dim())
}

/// The section of the panel with the two tools of the buses.
pub fn section(ui: &mut egui::Ui, bus: &mut BusUi, edit: &mut SchEdit, lang: Lang) {
    ui.horizontal(|ui| {
        for (mode, text) in [
            (BusMode::Bus, HierMsg::ToolBus),
            (BusMode::Entry, HierMsg::ToolBusEntry),
        ] {
            if ui
                .selectable_label(bus.mode == mode, text.tr(lang))
                .clicked()
            {
                bus.toggle(mode);
            }
        }
    });
    match bus.mode {
        BusMode::Off => {}
        BusMode::Bus => {
            ui.label(dim(HierMsg::HintBus.tr(lang)));
            ui.horizontal(|ui| {
                ui.label(dim(HierMsg::BusName.tr(lang)));
                ui.add(egui::TextEdit::singleline(&mut bus.spec).desired_width(140.0));
            });
        }
        BusMode::Entry => {
            ui.label(dim(HierMsg::HintBusEntry.tr(lang)));
            let Some(sheet) = edit.sheet() else {
                return;
            };
            bus.settle(edit.schematic(), sheet);
            let list = buses_on(edit.schematic(), sheet);
            if list.is_empty() {
                ui.label(dim(HierMsg::NoBuses.tr(lang)));
                return;
            }
            let selected = list
                .iter()
                .find(|(id, _)| Some(*id) == bus.bus)
                .map(|(_, name)| name.clone())
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(dim(HierMsg::BusPick.tr(lang)));
                egui::ComboBox::from_id_salt("schematic-bus")
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for (id, name) in &list {
                            ui.selectable_value(&mut bus.bus, Some(*id), name.as_str());
                        }
                    });
            });
            if let Some(id) = bus.bus {
                let members = members_of(edit.schematic(), id);
                ui.horizontal(|ui| {
                    ui.label(dim(HierMsg::Member.tr(lang)));
                    egui::ComboBox::from_id_salt("schematic-bus-member")
                        .selected_text(bus.member.clone())
                        .show_ui(ui, |ui| {
                            for member in &members {
                                ui.selectable_value(
                                    &mut bus.member,
                                    member.clone(),
                                    member.as_str(),
                                );
                            }
                        });
                });
            }
        }
    }
}
