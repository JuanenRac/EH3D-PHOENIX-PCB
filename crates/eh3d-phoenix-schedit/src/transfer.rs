// SPDX-License-Identifier: GPL-3.0-or-later
//! What leaves the editor: the transfer of the schematic to a board (a dry run first, as a package
//! that is previewed and then applied as one step of the history of the board), and the net list
//! for the simulator, with the result of the operating point net by net.

use std::collections::BTreeSet;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_schematic::{HierConfig, Id, Schematic};
use eh3d_phoenix_sim::{
    generate_netlist, generate_netlist_hier, operating_table, ModelTable, NetlistReport,
    NetlistText, OpTable, SimError, SimSettings,
};
use eh3d_phoenix_sync::{
    compare, plan_annotation, plan_forward, AnnotatePlan, AnnotateSettings, DiffReport,
    ForwardPackage, SyncSettings,
};

/// The time stamp of the ids that the transfer makes.
const STAMP_MS: u64 = 1_900_000_000_000;

/// A supply of ids for the objects the transfer creates on the board: always the same ids in the
/// same order, never one that the schematic or the board already uses.
pub struct IdSupply {
    taken: BTreeSet<Id>,
    next: u64,
}

impl IdSupply {
    /// A supply that avoids the ids of both documents.
    pub fn new(schematic: &Schematic, board: &Board) -> IdSupply {
        let mut taken: BTreeSet<Id> = BTreeSet::new();
        taken.extend(board.layers.keys());
        taken.extend(board.nets.keys());
        taken.extend(board.footprints.keys());
        for footprint in board.footprints.values() {
            taken.extend(footprint.pads.keys());
        }
        taken.extend(board.components.keys());
        taken.extend(board.tracks.keys());
        taken.extend(board.vias.keys());
        taken.extend(board.zones.keys());
        taken.extend(board.silkscreen.keys());
        taken.extend(schematic.symbols.keys());
        for symbol in schematic.symbols.values() {
            taken.extend(symbol.pins.keys());
        }
        taken.extend(schematic.parts.keys());
        taken.extend(schematic.nets.keys());
        taken.extend(schematic.sheets.keys());
        taken.extend(schematic.wires.keys());
        taken.extend(schematic.labels.keys());
        taken.extend(schematic.junctions.keys());
        taken.extend(schematic.power_ports.keys());
        IdSupply { taken, next: 0 }
    }

    /// The next id that is not taken.
    pub fn fresh(&mut self) -> Id {
        loop {
            self.next += 1;
            if let Ok(id) = Id::from_parts(STAMP_MS, (self.next & 0x0fff) as u16, self.next) {
                if self.taken.insert(id) {
                    return id;
                }
            }
        }
    }
}

/// The dry run of the transfer of a schematic to a board: nothing is changed, the package says what
/// would be.
pub fn plan_transfer(
    schematic: &Schematic,
    board: &Board,
    settings: &SyncSettings,
) -> ForwardPackage {
    let mut supply = IdSupply::new(schematic, board);
    let mut new_id = || supply.fresh();
    plan_forward(schematic, board, settings, &mut new_id)
}

/// The dry run of the annotation back into the schematic: what the board would change in the
/// schematic (references, renumbering by position, values, footprints, swaps of equivalent pins
/// and gates), each change with its state, as data for a window or a table. Nothing is changed;
/// the plan is applied to a copy with its confirmation code.
pub fn plan_reverse_annotation(
    schematic: &Schematic,
    board: &Board,
    settings: &AnnotateSettings,
) -> AnnotatePlan {
    plan_annotation(schematic, board, settings)
}

/// The differences and conflicts between a schematic and a board, in both directions at once, as
/// data for a window or a table.
pub fn compare_documents(schematic: &Schematic, board: &Board) -> DiffReport {
    compare(schematic, board)
}

/// The net list of a schematic for the simulator, as SPICE text, with the report of what could not
/// be translated. It is made by the generator of the simulator crate with its built-in table of
/// models (`config/sim_models.cfg`).
#[derive(Debug, Clone, PartialEq)]
pub struct NetlistExport {
    /// The netlist.
    pub text: String,
    /// What was translated and what was not, with a code and a severity for each issue.
    pub report: NetlistReport,
}

/// The netlist of a schematic, with every instance of every sheet drawn out when it has sheet
/// symbols.
fn generate(schematic: &Schematic, table: &ModelTable, settings: &SimSettings) -> NetlistText {
    generate_netlist_hier(schematic, &HierConfig::default(), table, None, settings)
        .unwrap_or_else(|_| generate_netlist(schematic, table, None, settings))
}

/// Translates a schematic for the simulator.
pub fn export_netlist(schematic: &Schematic, settings: &SimSettings) -> NetlistExport {
    let generated = generate(schematic, &ModelTable::built_in(), settings);
    NetlistExport {
        text: generated.text,
        report: generated.report,
    }
}

/// The operating point of a schematic, net by net (the voltage of each net, the current of each
/// source), as data for a window or a table. The models come from `table`; the files that the
/// table includes are looked for in `folder`.
pub fn netlist_results(
    schematic: &Schematic,
    table: &ModelTable,
    folder: Option<&Path>,
    settings: &SimSettings,
) -> Result<OpTable, SimError> {
    let generated = generate(schematic, table, settings);
    operating_table(&generated, folder, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::fixture;
    use eh3d_phoenix_sync::codes::FOOTPRINT_UNUSABLE;

    #[test]
    fn parts_without_a_footprint_are_left_out_of_the_package_and_said_so() {
        let f = fixture();
        let package = plan_transfer(&f.schematic, &Board::default(), &SyncSettings::default());
        assert!(package.is_empty());
        assert_eq!(package.skipped.len(), 2);
        assert!(package.skipped.iter().all(|s| s.code == FOOTPRINT_UNUSABLE));
    }

    #[test]
    fn the_plan_of_the_annotation_and_the_comparison_come_as_data_and_change_nothing() {
        let f = fixture();
        let before = f.schematic.clone();
        let board = Board::default();
        let plan = plan_reverse_annotation(&f.schematic, &board, &AnnotateSettings::default());
        // both parts have no component on the board: they are listed and nothing is applied
        assert_eq!(plan.items.len(), 2);
        assert!(plan.is_empty());
        let report = compare_documents(&f.schematic, &board);
        assert_eq!(report.differences(), 2);
        assert_eq!(report.conflicts(), 0);
        assert_eq!(f.schematic, before);
    }

    #[test]
    fn the_supply_of_ids_is_the_same_every_time_and_avoids_the_documents() {
        let f = fixture();
        let board = Board::default();
        let mut a = IdSupply::new(&f.schematic, &board);
        let mut b = IdSupply::new(&f.schematic, &board);
        let first: Vec<Id> = (0..5).map(|_| a.fresh()).collect();
        let second: Vec<Id> = (0..5).map(|_| b.fresh()).collect();
        assert_eq!(first, second);
        let distinct: BTreeSet<Id> = first.iter().copied().collect();
        assert_eq!(distinct.len(), 5);
        assert!(first.iter().all(|id| !f.schematic.parts.contains_key(id)));
    }

    #[test]
    fn the_results_of_a_schematic_come_net_by_net() {
        use eh3d_phoenix_schematic::{NetPin, Part, SchematicNet};
        let f = fixture();
        let mut s = f.schematic.clone();
        let symbol = s.parts[&f.r1].symbol;
        let pin_of = |number: &str| -> Id {
            *s.symbols[&symbol]
                .pins
                .iter()
                .find(|(_, pin)| pin.number == number)
                .map(|(id, _)| id)
                .unwrap()
        };
        let (p1, p2) = (pin_of("1"), pin_of("2"));
        let id = |n: u16| Id::from_parts(1_800_000_000_001, n, u64::from(n)).unwrap();
        let v1 = id(1);
        s.parts.insert(
            v1,
            Part {
                refdes: "V1".into(),
                value: "10".into(),
                symbol,
                footprint: None,
                placement: None,
            },
        );
        for net in s.nets.values_mut() {
            net.name = "OUT".into();
        }
        let mut pins_in = vec![
            NetPin { part: v1, pin: p1 },
            NetPin {
                part: f.r1,
                pin: p1,
            },
        ];
        pins_in.sort();
        s.nets.insert(
            id(2),
            SchematicNet {
                name: "IN".into(),
                pins: pins_in,
            },
        );
        let mut pins_gnd = vec![
            NetPin { part: v1, pin: p2 },
            NetPin {
                part: f.r2,
                pin: p2,
            },
        ];
        pins_gnd.sort();
        s.nets.insert(
            id(3),
            SchematicNet {
                name: "GND".into(),
                pins: pins_gnd,
            },
        );
        let settings = SimSettings::default();
        let table = netlist_results(&s, &ModelTable::built_in(), None, &settings).unwrap();
        let volts = |net: &str| table.voltages.iter().find(|v| v.net == net).unwrap().volts;
        // two resistors of 10 kohm in series across 10 V
        assert!((volts("OUT") - 5.0).abs() < 1e-9);
        assert!((volts("IN") - 10.0).abs() < 1e-9);
        assert_eq!(volts("GND"), 0.0);
        assert_eq!(table.currents.len(), 1);
        assert_eq!(table.currents[0].refdes, "V1");
        assert!((table.currents[0].amps + 0.5e-3).abs() < 1e-9);
        // the text of the net list comes from the same generator and is clean
        let export = export_netlist(&s, &settings);
        assert!(export.report.is_clean(), "{:?}", export.report.issues);
        assert!(export.text.contains("V1 in 0 "));
        // the schematic of the fixture alone has no ground and no source
        assert!(netlist_results(&f.schematic, &ModelTable::built_in(), None, &settings).is_err());
    }

    #[test]
    fn the_netlist_has_the_parts_and_the_report_says_what_is_missing() {
        let f = fixture();
        let export = export_netlist(&f.schematic, &SimSettings::default());
        assert_eq!(
            export.report.translated,
            vec!["R1".to_owned(), "R2".to_owned()]
        );
        assert!(export.text.contains("R1"));
        assert!(export.text.contains("R2"));
        // nothing is a ground, and two pins are on no net
        assert!(!export.report.is_clean());
    }
}
