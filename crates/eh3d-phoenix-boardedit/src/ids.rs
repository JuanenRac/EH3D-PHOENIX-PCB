// SPDX-License-Identifier: GPL-3.0-or-later
//! The ids of the parts that the editor makes. They are deterministic: the same board and the same
//! operations always give the same ids, and an id that the board already uses is skipped.

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::ObjectId;

use crate::codes::{BoardEditError, OPERATION_REFUSED};

const SUPPLY_UNIX_MS: u64 = 1_800_000_001_000;
const SUPPLY_RANDOM: u64 = 0x4245_4449_0000_0000;

/// The supply of new ids.
#[derive(Debug, Clone)]
pub struct IdSupply {
    counter: u64,
    last: Id,
}

impl IdSupply {
    /// A supply that starts at the first id.
    pub fn new() -> Result<IdSupply, BoardEditError> {
        let last = ObjectId::from_parts(SUPPLY_UNIX_MS, 0, SUPPLY_RANDOM)
            .map_err(|e| BoardEditError::new(OPERATION_REFUSED, e.to_string()))?;
        Ok(IdSupply { counter: 0, last })
    }

    fn next_raw(&mut self) -> Id {
        self.counter += 1;
        let n = self.counter;
        let made = ObjectId::from_parts(
            SUPPLY_UNIX_MS,
            (n & 0xfff) as u16,
            SUPPLY_RANDOM.wrapping_add(n),
        );
        if let Ok(id) = made {
            self.last = id;
        }
        self.last
    }

    /// The next id that no part of the board uses.
    pub fn next_for(&mut self, board: &Board) -> Id {
        let mut id = self.next_raw();
        for _ in 0..1024 {
            let taken = board.tracks.contains_key(&id)
                || board.vias.contains_key(&id)
                || board.zones.contains_key(&id)
                || board.components.contains_key(&id)
                || board.nets.contains_key(&id)
                || board.footprints.contains_key(&id)
                || board.layers.contains_key(&id)
                || board.silkscreen.contains_key(&id);
            if !taken {
                break;
            }
            id = self.next_raw();
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ids_are_the_same_every_time_and_never_repeat() {
        let board = Board::default();
        let mut a = IdSupply::new().unwrap();
        let mut b = IdSupply::new().unwrap();
        let first: Vec<Id> = (0..5).map(|_| a.next_for(&board)).collect();
        let second: Vec<Id> = (0..5).map(|_| b.next_for(&board)).collect();
        assert_eq!(first, second);
        for (i, x) in first.iter().enumerate() {
            assert!(!first[..i].contains(x));
        }
    }

    #[test]
    fn an_id_that_the_board_uses_is_skipped() {
        let mut probe = IdSupply::new().unwrap();
        let taken = probe.next_for(&Board::default());
        let mut board = Board::default();
        board.nets.insert(
            taken,
            eh3d_phoenix_board::Net {
                name: "N".to_owned(),
            },
        );
        let mut supply = IdSupply::new().unwrap();
        assert_ne!(supply.next_for(&board), taken);
    }
}
