// SPDX-License-Identifier: GPL-3.0-or-later
//! The tools «Bus» and «Bus entry» of the schematic editor, as data that is tested without a
//! screen: which bus and which member are chosen, what a click does, and the member that comes
//! next after an entry. The window itself is in `schematic_gui`, behind the `gui` feature, and has
//! not been seen on a screen.

use eh3d_phoenix_model::Point;
use eh3d_phoenix_schedit::SchEdit;
use eh3d_phoenix_schematic::{EditError, EditOp, Id, Schematic};

/// What a click on the drawing does while one of these tools is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BusMode {
    /// Neither tool is on: the clicks belong to the tool of the editor.
    #[default]
    Off,
    /// A click places the bus named in the box.
    Bus,
    /// A click takes the chosen member out of the chosen bus.
    Entry,
}

/// The state of the two tools.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BusUi {
    /// Which tool is on.
    pub mode: BusMode,
    /// The name of the bus to place (`D[0..7]`, `USB{DP DM}` or an alias).
    pub spec: String,
    /// The bus whose member an entry takes out.
    pub bus: Option<Id>,
    /// The member an entry takes out.
    pub member: String,
}

/// The buses drawn on a sheet, with their names.
pub fn buses_on(s: &Schematic, sheet: Id) -> Vec<(Id, String)> {
    s.buses
        .iter()
        .filter(|(_, b)| b.sheet == sheet)
        .map(|(id, b)| (*id, b.spec.clone()))
        .collect()
}

/// The members of a bus; empty when its name cannot be expanded.
pub fn members_of(s: &Schematic, bus: Id) -> Vec<String> {
    s.bus_member_names(bus).unwrap_or_default()
}

/// The member after `member` among the members of the bus, that no entry of the bus uses yet when
/// there is one; else the first member.
pub fn next_member(s: &Schematic, bus: Id, member: &str) -> String {
    let members = members_of(s, bus);
    let used = |m: &String| {
        s.bus_entries
            .values()
            .any(|e| e.bus == bus && e.member == *m)
    };
    let start = members
        .iter()
        .position(|m| m == member)
        .map_or(0, |i| i + 1);
    members
        .iter()
        .cycle()
        .skip(start)
        .take(members.len())
        .find(|m| !used(m))
        .or_else(|| members.first())
        .cloned()
        .unwrap_or_default()
}

impl BusUi {
    /// A state with both tools off.
    pub fn new() -> BusUi {
        BusUi::default()
    }

    /// True when one of the tools is on.
    pub fn active(&self) -> bool {
        self.mode != BusMode::Off
    }

    /// Turns a tool on, or off when it already is.
    pub fn toggle(&mut self, mode: BusMode) {
        self.mode = if self.mode == mode {
            BusMode::Off
        } else {
            mode
        };
    }

    /// Makes the bus and the member that are chosen ones that exist on the sheet: the first bus
    /// and its first member when the choice is gone.
    pub fn settle(&mut self, s: &Schematic, sheet: Id) {
        let buses = buses_on(s, sheet);
        if !self
            .bus
            .is_some_and(|b| buses.iter().any(|(id, _)| *id == b))
        {
            self.bus = buses.first().map(|(id, _)| *id);
            self.member.clear();
        }
        if let Some(bus) = self.bus {
            let members = members_of(s, bus);
            if !members.contains(&self.member) {
                self.member = members.first().cloned().unwrap_or_default();
            }
        }
    }

    /// What a click at `at` does: places the bus, or takes the member out of the bus. The point
    /// snaps like the points of the other tools. The error is the reason the editor gave, which it
    /// also keeps for the panel.
    pub fn click(&mut self, edit: &mut SchEdit, at: Point) -> Result<(), EditError> {
        let sheet = edit
            .sheet()
            .ok_or(EditError::BadSelection("the schematic has no sheet"))?;
        let at = edit.snap(at);
        match self.mode {
            BusMode::Off => Ok(()),
            BusMode::Bus => {
                edit.run(&EditOp::PlaceBus {
                    sheet,
                    at,
                    spec: self.spec.clone(),
                })?;
                Ok(())
            }
            BusMode::Entry => {
                let bus = self
                    .bus
                    .ok_or(EditError::BadSelection("choose a bus first"))?;
                edit.run(&EditOp::AddBusEntry {
                    bus,
                    member: self.member.clone(),
                    at,
                })?;
                self.member = next_member(edit.schematic(), bus, &self.member);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_erc::ErcConfig;
    use eh3d_phoenix_model::Nm;
    use eh3d_phoenix_schedit::SchEditSettings;

    fn pt(x_mm: i64, y_mm: i64) -> Point {
        Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
    }

    fn editor() -> SchEdit {
        let settings = SchEditSettings {
            snap: false,
            ..SchEditSettings::default()
        };
        let mut edit = SchEdit::new(Schematic::default(), settings, ErcConfig::default());
        edit.run(&EditOp::AddSheet {
            name: "Sheet 1".into(),
            width: Nm::new(297_000_000).unwrap(),
            height: Nm::new(210_000_000).unwrap(),
        })
        .unwrap();
        edit
    }

    #[test]
    fn a_tool_is_toggled_and_both_start_off() {
        let mut ui = BusUi::new();
        assert!(!ui.active());
        ui.toggle(BusMode::Bus);
        assert!(ui.active() && ui.mode == BusMode::Bus);
        ui.toggle(BusMode::Entry);
        assert_eq!(ui.mode, BusMode::Entry);
        ui.toggle(BusMode::Entry);
        assert!(!ui.active());
    }

    #[test]
    fn a_click_places_the_bus_and_then_takes_the_members_out_one_after_another() {
        let mut edit = editor();
        let mut ui = BusUi::new();
        ui.toggle(BusMode::Bus);
        ui.spec = "D[0..2]".into();
        ui.click(&mut edit, pt(20, 20)).unwrap();
        assert_eq!(edit.schematic().buses.len(), 1);
        // a name that is not a bus is refused and the editor keeps the reason
        ui.spec = "D[0..300]".into();
        assert!(ui.click(&mut edit, pt(30, 30)).is_err());
        assert!(edit.error.is_some());
        assert_eq!(edit.schematic().buses.len(), 1);
        // entries: the tool picks the first bus and its first member
        let sheet = edit.sheet().unwrap();
        ui.toggle(BusMode::Entry);
        ui.settle(edit.schematic(), sheet);
        assert_eq!(ui.member, "D0");
        for expected in ["D0", "D1", "D2"] {
            assert_eq!(ui.member, expected);
            ui.click(&mut edit, pt(40, 40)).unwrap();
        }
        assert_eq!(edit.schematic().bus_entries.len(), 3);
        // all the members are used: the next one is the first again
        assert_eq!(ui.member, "D0");
        // undo takes the last entry away
        assert!(edit.undo());
        assert_eq!(edit.schematic().bus_entries.len(), 2);
    }

    #[test]
    fn the_choice_follows_the_buses_of_the_sheet() {
        let mut edit = editor();
        let sheet = edit.sheet().unwrap();
        let mut ui = BusUi::new();
        ui.toggle(BusMode::Entry);
        // no bus: an entry needs one
        ui.settle(edit.schematic(), sheet);
        assert_eq!(ui.bus, None);
        assert!(ui.click(&mut edit, pt(10, 10)).is_err());
        edit.run(&EditOp::PlaceBus {
            sheet,
            at: pt(5, 5),
            spec: "USB{DP DM}".into(),
        })
        .unwrap();
        ui.settle(edit.schematic(), sheet);
        assert!(ui.bus.is_some());
        assert_eq!(ui.member, "USB.DP");
        assert_eq!(buses_on(edit.schematic(), sheet).len(), 1);
        assert_eq!(
            members_of(edit.schematic(), ui.bus.unwrap()),
            vec!["USB.DP", "USB.DM"]
        );
        // a member that is not one of the bus is set right
        ui.member = "nothing".into();
        ui.settle(edit.schematic(), sheet);
        assert_eq!(ui.member, "USB.DP");
        // the bus goes away: the choice goes with it
        let bus = ui.bus.unwrap();
        edit.run(&EditOp::RemoveBus { bus }).unwrap();
        ui.settle(edit.schematic(), sheet);
        assert_eq!(ui.bus, None);
    }

    #[test]
    fn the_next_member_skips_the_ones_that_are_used() {
        let mut edit = editor();
        let sheet = edit.sheet().unwrap();
        edit.run(&EditOp::PlaceBus {
            sheet,
            at: pt(5, 5),
            spec: "A[0..3]".into(),
        })
        .unwrap();
        let bus = *edit.schematic().buses.keys().next().unwrap();
        for (member, at) in [("A1", pt(10, 10)), ("A2", pt(12, 10))] {
            edit.run(&EditOp::AddBusEntry {
                bus,
                member: member.into(),
                at,
            })
            .unwrap();
        }
        assert_eq!(next_member(edit.schematic(), bus, "A0"), "A3");
        assert_eq!(next_member(edit.schematic(), bus, "A3"), "A0");
        assert_eq!(next_member(edit.schematic(), bus, "nothing"), "A0");
    }
}
