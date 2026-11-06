// SPDX-License-Identifier: GPL-3.0-or-later
//! The notes of the consistency checks of a board that a report of `check` carries as information:
//! a polygon with more points than `max_polygon_points`, whose edges were not tested for crossing
//! (`PHOENIX-BOARD-017`). They never make a report fail.

use eh3d_phoenix_board::validate::{BoardLimits, POLYGON_NOT_CHECKED};
use eh3d_phoenix_board::Board;

use crate::report::{Finding, Report, Severity};

/// The place of the notes in the fixed order of the findings: after the ones of the further
/// checks (21 to 29) and of the profile of a board maker (40 to 45).
const NOTE_RANK: u8 = 50;

/// One finding of information for each polygon of the board (outline, cutout of the outline or of a
/// zone) that is over `limits.max_polygon_points` and was therefore tested for repeated points but
/// not for crossing edges. A board with no such polygon gives an empty report.
pub fn board_notes(board: &Board, limits: &BoardLimits) -> Report {
    let mut report = Report::new("check");
    for (place, points) in board.polygons_not_cross_checked(limits) {
        report.confirmed.push(
            Finding::new(
                NOTE_RANK,
                "polygon_not_cross_checked",
                POLYGON_NOT_CHECKED,
                Severity::Info,
                place,
            )
            .why(&format!(
                "{points} points are more than the limit max_polygon_points = {}",
                limits.max_polygon_points
            )),
        );
    }
    report.finish();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::Lang;
    use crate::report_export::category;
    use eh3d_phoenix_model::Point;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn square_board() -> Board {
        Board {
            outline: vec![pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)],
            ..Board::default()
        }
    }

    #[test]
    fn a_board_with_no_large_polygon_gives_no_note() {
        let board = square_board();
        let report = board_notes(&board, &BoardLimits::default());
        assert!(report.confirmed.is_empty() && report.inconclusive.is_empty());
        let at_the_limit = BoardLimits {
            max_polygon_points: 4,
        };
        assert!(board_notes(&board, &at_the_limit).confirmed.is_empty());
    }

    #[test]
    fn a_polygon_of_one_point_over_the_limit_gives_one_note_that_does_not_fail_the_report() {
        let board = square_board();
        let limits = BoardLimits {
            max_polygon_points: 3,
        };
        let report = board_notes(&board, &limits);
        assert_eq!(report.confirmed.len(), 1);
        assert!(report.inconclusive.is_empty());
        let f = &report.confirmed[0];
        assert_eq!(f.code, "PHOENIX-BOARD-017");
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.location, "the board");
        assert_eq!(
            f.detail,
            "4 points are more than the limit max_polygon_points = 3"
        );
        assert!(report.passes());
        assert_eq!(report.count(Severity::Info), 1);
        assert_eq!(report.count(Severity::Error), 0);
    }

    #[test]
    fn the_note_has_a_title_in_every_language_and_the_category_other() {
        assert_eq!(category("PHOENIX-BOARD-017"), "other");
        let report = board_notes(
            &square_board(),
            &BoardLimits {
                max_polygon_points: 3,
            },
        );
        let text = report.to_text(Lang::Pt);
        assert!(
            text.contains(
                "O polígono tem mais pontos do que o limite, as suas arestas não foram verificadas quanto a cruzamentos"
            ),
            "{text}"
        );
        for lang in Lang::ALL {
            assert!(
                !crate::codes::title("PHOENIX-BOARD-017", lang)
                    .unwrap_or_default()
                    .is_empty(),
                "{}",
                lang.tag()
            );
        }
    }
}
