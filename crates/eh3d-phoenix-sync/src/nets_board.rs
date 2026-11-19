// SPDX-License-Identifier: GPL-3.0-or-later
//! The nets of a board as the table of the text netlist: the pins of the board as
//! `(net, part, pad)` and the table made from them with the notes about what could not be written
//! as it is. The command `phoenix nets` and the report of the board editor both start here, so
//! that they judge the same table.

use eh3d_phoenix_board::Board;

use crate::nets_text::{table_of_pins, NetTable, Note};

/// The pins of the board as `(net, part, pad)`: a pad of a part that a net names. An assignment
/// whose net, part, footprint or pad is not on the board is left out.
pub fn board_pins(board: &Board) -> Vec<(String, String, String)> {
    let mut pins = Vec::new();
    for assignment in &board.pad_nets {
        let Some(net) = board.nets.get(&assignment.net) else {
            continue;
        };
        let Some(component) = board.components.get(&assignment.component) else {
            continue;
        };
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        let Some(pad) = footprint.pads.get(&assignment.pad) else {
            continue;
        };
        pins.push((
            net.name.clone(),
            component.refdes.clone(),
            pad.number.clone(),
        ));
    }
    pins
}

/// The table of the nets of a board, with the notes about the names that could not be written.
pub fn board_table(board: &Board) -> (NetTable, Vec<Note>) {
    table_of_pins(board_pins(board))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_board_without_nets_has_no_pins_and_an_empty_table() {
        let board = Board::default();
        assert!(board_pins(&board).is_empty());
        let (table, notes) = board_table(&board);
        assert!(table.nets.is_empty());
        assert!(notes.is_empty());
    }
}
