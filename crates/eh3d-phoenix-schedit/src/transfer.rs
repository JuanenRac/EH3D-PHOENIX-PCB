// SPDX-License-Identifier: GPL-3.0-or-later
//! What leaves the editor: the transfer of the schematic to a board (a dry run first, as a package
//! that is previewed and then applied as one step of the history of the board), and the net list
//! for the simulator.

use std::collections::BTreeSet;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_schematic::{Id, Schematic};
use eh3d_phoenix_sim::{netlist_from_schematic, to_spice, ExportReport, SimSettings};
use eh3d_phoenix_sync::{plan_forward, ForwardPackage, SyncSettings};

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

/// The net list of a schematic for the simulator, as SPICE text, with the report of what could not
/// be translated.
#[derive(Debug, Clone, PartialEq)]
pub struct NetlistExport {
    /// The netlist.
    pub text: String,
    /// What was translated and what was not.
    pub report: ExportReport,
}

/// Translates a schematic for the simulator.
pub fn export_netlist(schematic: &Schematic, settings: &SimSettings) -> NetlistExport {
    let (netlist, report) = netlist_from_schematic(schematic, settings);
    NetlistExport {
        text: to_spice(&netlist),
        report,
    }
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
