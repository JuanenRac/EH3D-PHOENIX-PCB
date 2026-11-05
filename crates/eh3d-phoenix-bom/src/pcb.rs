// SPDX-License-Identifier: GPL-3.0-or-later
//! The estimate of what the bare boards cost, from the area of the outline, the number of copper
//! layers, the thickness of the stack-up and the number of holes, with the tariffs of the
//! settings (`config/costs.cfg`). The tariffs that come with the program are an example and not
//! the prices of any maker.
//!
//! For one board: the area in square centimetres divided by the part of the panel that holds
//! boards, times the price of a square centimetre for the layers, plus the surcharge when the board
//! is thicker than the reference, plus the cost of each hole. For the batch: the fixed cost plus
//! the boards times the cost of one board, and at least the least order. All of it in whole
//! millionths, rounded half up.

use eh3d_phoenix_board::{Board, LayerKind};
use eh3d_phoenix_stackup::Stackup;

use crate::codes::{Finding, Severity, PCB_ESTIMATE, PCB_NOT_COSTED};
use crate::money::{mul_div, MICRO};
use crate::settings::CostSettings;

/// The estimate of the boards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcbCost {
    /// The copper layers.
    pub layers: u32,
    /// The area of the outline, in square nanometres.
    pub area_nm2: i128,
    /// The thickness of the stack-up, when one was given.
    pub thickness_nm: Option<i64>,
    /// The drilled holes of one board: vias and through-hole pads.
    pub holes: u64,
    /// The cost of one board, in millionths.
    pub per_board_micro: i128,
    /// The fixed cost of the order, in millionths.
    pub setup_micro: i128,
    /// The cost of the batch, in millionths.
    pub batch_micro: i128,
}

/// The area enclosed by a polygon given as pairs of nanometres, in square nanometres.
pub fn polygon_area(points: &[(i64, i64)]) -> i128 {
    let mut twice: i128 = 0;
    for (i, a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        twice += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    (twice.abs() + 1) / 2
}

/// The cost of the boards, or `None` with a finding when the board does not allow one.
pub fn pcb_cost(
    board: &Board,
    stackup: Option<&Stackup>,
    settings: &CostSettings,
    findings: &mut Vec<Finding>,
) -> Option<PcbCost> {
    let mut refuse = |why: String| {
        findings.push(Finding::new(PCB_NOT_COSTED, Severity::Warning, None, why));
        None
    };
    if board.outline.len() < 3 {
        return refuse("the outline has fewer than three points".to_owned());
    }
    let points: Vec<(i64, i64)> = board
        .outline
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect();
    let area_nm2 = polygon_area(&points);
    if area_nm2 == 0 {
        return refuse("the outline encloses no area".to_owned());
    }
    let counted = board
        .layers
        .values()
        .filter(|l| l.kind == LayerKind::Copper)
        .count();
    let layers = if board.copper_stack.is_empty() {
        counted
    } else {
        board.copper_stack.len()
    };
    if layers == 0 {
        return refuse("the board has no copper layer".to_owned());
    }
    let layers = u32::try_from(layers).unwrap_or(u32::MAX);
    let Some(rate) = settings.pcb_area_rate_micro.get(&layers).copied() else {
        return refuse(format!(
            "the settings have no tariff for {layers} copper layers"
        ));
    };
    let thickness_nm = stackup.and_then(|s| s.total_thickness_nm().ok());

    let through_pads: usize = board
        .components
        .values()
        .filter_map(|c| board.footprints.get(&c.footprint))
        .map(|f| f.pads.values().filter(|p| p.drill.is_some()).count())
        .sum();
    let holes = (board.vias.len() + through_pads) as u64;

    let whole = 100 * i128::from(MICRO);
    let area_micro_cm2 = mul_div(area_nm2, 1, 100_000_000);
    let panel_micro_cm2 = mul_div(
        area_micro_cm2,
        whole,
        i128::from(settings.pcb_utilisation_percent_micro),
    );
    let mut per_board = mul_div(panel_micro_cm2, i128::from(rate), i128::from(MICRO));
    if thickness_nm.is_some_and(|t| t > settings.pcb_thickness_reference_nm) {
        per_board = mul_div(
            per_board,
            whole + i128::from(settings.pcb_thickness_surcharge_percent_micro),
            whole,
        );
    }
    per_board += i128::from(holes) * i128::from(settings.pcb_drill_cost_micro);
    let setup = i128::from(settings.pcb_setup_cost_micro);
    let batch = (setup + i128::from(settings.boards) * per_board)
        .max(i128::from(settings.pcb_min_order_micro));
    findings.push(Finding::new(
        PCB_ESTIMATE,
        Severity::Info,
        None,
        format!("{layers} layers, tariffs of the settings"),
    ));
    Some(PcbCost {
        layers,
        area_nm2,
        thickness_nm,
        holes,
        per_board_micro: per_board,
        setup_micro: setup,
        batch_micro: batch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_area_of_a_polygon_is_exact_in_either_direction() {
        let square = [
            (0, 0),
            (10_000_000, 0),
            (10_000_000, 10_000_000),
            (0, 10_000_000),
        ];
        assert_eq!(polygon_area(&square), 100_000_000_000_000);
        let mut reversed = square;
        reversed.reverse();
        assert_eq!(polygon_area(&reversed), 100_000_000_000_000);
        let triangle = [(0, 0), (4_000_000, 0), (0, 3_000_000)];
        assert_eq!(polygon_area(&triangle), 6_000_000_000_000);
        let line = [(0, 0), (1, 1), (2, 2)];
        assert_eq!(polygon_area(&line), 0);
    }
}
