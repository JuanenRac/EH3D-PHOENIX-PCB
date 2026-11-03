//! The layers of EAGLE and how the layers of a board map onto them.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Id, LayerKind, Side};

use crate::codes::{self, ITEM_LEFT_OUT, ITEM_SIMPLIFIED};

/// The copper layers of the board from the top to the bottom: the stack when it is given and
/// sound, otherwise the copper layers in the order of their ids.
pub(crate) fn effective_copper(board: &Board) -> Vec<Id> {
    if !board.copper_stack.is_empty() && board.stack_problems().is_empty() {
        return board.copper_stack.clone();
    }
    board
        .layers
        .iter()
        .filter(|(_, l)| l.kind == LayerKind::Copper)
        .map(|(id, _)| *id)
        .collect()
}

/// The EAGLE number of a copper layer by its place: the first is 1, the last 16, the ones between
/// 2, 3, ... `None` when there are more than 16.
pub(crate) fn copper_number(index: usize, count: usize) -> Option<u32> {
    if count > 16 {
        return None;
    }
    Some(if index == 0 {
        1
    } else if index + 1 == count {
        16
    } else {
        index as u32 + 1
    })
}

/// The EAGLE number of a layer that is not copper.
pub(crate) fn fixed_number(kind: LayerKind, side: Option<Side>) -> u32 {
    let bottom = side == Some(Side::Bottom);
    match kind {
        LayerKind::Outline => 20,
        LayerKind::Silkscreen => 21 + u32::from(bottom),
        LayerKind::SolderMask => 29 + u32::from(bottom),
        LayerKind::Paste => 31 + u32::from(bottom),
        LayerKind::Documentation => 51 + u32::from(bottom),
        LayerKind::Copper => 1,
    }
}

/// What a layer number means: its kind and side, `None` for numbers that are not read.
pub(crate) fn meaning(number: u32) -> Option<(LayerKind, Option<Side>)> {
    let side = Some(if number.is_multiple_of(2) {
        Side::Bottom
    } else {
        Side::Top
    });
    Some(match number {
        1..=16 => (LayerKind::Copper, None),
        20 => (LayerKind::Outline, None),
        21 | 22 => (LayerKind::Silkscreen, side),
        29 | 30 => (LayerKind::SolderMask, side),
        31 | 32 => (LayerKind::Paste, side),
        51 | 52 => (LayerKind::Documentation, side),
        _ => return None,
    })
}

/// The name EAGLE gives a layer number.
pub(crate) fn name_of(number: u32) -> String {
    match number {
        1 => "Top".to_owned(),
        16 => "Bottom".to_owned(),
        2..=15 => format!("Route{number}"),
        20 => "Dimension".to_owned(),
        21 => "tPlace".to_owned(),
        22 => "bPlace".to_owned(),
        29 => "tStop".to_owned(),
        30 => "bStop".to_owned(),
        31 => "tCream".to_owned(),
        32 => "bCream".to_owned(),
        51 => "tDocu".to_owned(),
        _ => "bDocu".to_owned(),
    }
}

/// The EAGLE number of every layer of the board, and what had to be given up to get them.
pub(crate) fn assign(board: &Board) -> (BTreeMap<Id, u32>, Vec<Diagnostic>) {
    let mut numbers = BTreeMap::new();
    let mut diagnostics = Vec::new();
    let copper = effective_copper(board);
    for (index, id) in copper.iter().enumerate() {
        match copper_number(index, copper.len()) {
            Some(n) => {
                numbers.insert(*id, n);
            }
            None => diagnostics.push(codes::diagnostic(
                ITEM_LEFT_OUT,
                Severity::Warning,
                &format!("copper layer {id}: EAGLE has at most 16 copper layers"),
            )),
        }
    }
    for (id, layer) in &board.layers {
        if layer.kind == LayerKind::Copper {
            continue;
        }
        if layer.side.is_none() && layer.kind != LayerKind::Outline {
            diagnostics.push(codes::diagnostic(
                ITEM_SIMPLIFIED,
                Severity::Info,
                &format!("layer {id} does not say its side and is written as a top layer"),
            ));
        }
        numbers.insert(*id, fixed_number(layer.kind, layer.side));
    }
    (numbers, diagnostics)
}
