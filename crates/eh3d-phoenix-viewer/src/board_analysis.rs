// SPDX-License-Identifier: GPL-3.0-or-later
//! The reports of the board editor, without a screen: the statistics of the board as it is now
//! (`phoenix stats`), its test points (`phoenix testpoints`) and the lengths of its nets
//! (`phoenix lengths`) and its bill of materials (`phoenix bom`, without prices), made with the
//! settings of `config/stats.cfg`, `config/testpoints.cfg`, `config/lengths.cfg` and
//! `config/costs.cfg` when those files are there and good, else with the built-in ones, and kept
//! as the text of a language until the next one is asked for or the report is closed. The window
//! shows this text in `board_analysis_gui`, behind the `gui` feature.

use std::fmt::Write as _;

use eh3d_phoenix_board::{mm_text, Board, DimensionKind};
use eh3d_phoenix_bom::{bom_text, build_bom, BomInputs, CostSettings, PartsFile, PriceBook};
use eh3d_phoenix_lengths::{analyse as analyse_lengths, LengthSettings};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stats::{compute, report_text as stats_report_text, StatsSettings};
use eh3d_phoenix_testpoints::{analyse, report_text as testpoints_report_text, TestPointSettings};

use crate::board_analysis_text::AnalysisMsg;

/// The settings file of the statistics.
const STATS_CONFIG: &str = "config/stats.cfg";

/// The settings file of the test points.
const TESTPOINTS_CONFIG: &str = "config/testpoints.cfg";

/// The settings file of the lengths.
const LENGTHS_CONFIG: &str = "config/lengths.cfg";

/// The settings file of the bill of materials.
const COSTS_CONFIG: &str = "config/costs.cfg";

/// The largest settings file that is read, in bytes.
const MAX_CONFIG_BYTES: u64 = 1 << 20;

/// Which report was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisKind {
    /// The size, counts, copper, drills and density of the board.
    Stats,
    /// The test points and the nets that no probe reaches.
    TestPoints,
    /// The copper length of each net and the findings of the lengths.
    Lengths,
    /// The parts of the board grouped by value and footprint, without prices.
    Bom,
    /// The dimensions of the board: kind, measure, layer and the two points.
    Dimensions,
}

/// The settings of the reports.
#[derive(Debug, Clone, Default)]
pub struct AnalysisSettings {
    /// The settings of the statistics.
    pub stats: StatsSettings,
    /// The settings of the test points.
    pub testpoints: TestPointSettings,
    /// The settings of the lengths.
    pub lengths: LengthSettings,
    /// The settings of the bill of materials.
    pub costs: CostSettings,
}

/// The state of the reports of the board editor.
#[derive(Debug, Default)]
pub struct AnalysisTool {
    /// The last report: which one it is and its text in the language it was asked for.
    pub shown: Option<(AnalysisKind, String)>,
}

fn read_config(path: &str) -> Option<String> {
    let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= MAX_CONFIG_BYTES);
    if !small {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

impl AnalysisSettings {
    /// The settings files of the config folder when they are there and good, else the built-in
    /// settings, one report at a time.
    pub fn load() -> AnalysisSettings {
        AnalysisSettings {
            stats: read_config(STATS_CONFIG)
                .and_then(|text| StatsSettings::parse(&text).ok())
                .unwrap_or_default(),
            testpoints: read_config(TESTPOINTS_CONFIG)
                .and_then(|text| TestPointSettings::parse(&text).ok())
                .unwrap_or_default(),
            lengths: read_config(LENGTHS_CONFIG)
                .and_then(|text| LengthSettings::parse(&text).ok())
                .unwrap_or_default(),
            costs: read_config(COSTS_CONFIG)
                .and_then(|text| CostSettings::parse(&text).ok())
                .unwrap_or_default(),
        }
    }
}

/// The text of a report of a board in a language, with the settings given. A board the report
/// refuses (too many objects) gives the message of the refusal.
pub fn report_for(
    kind: AnalysisKind,
    board: &Board,
    settings: &AnalysisSettings,
    lang: Lang,
) -> String {
    match kind {
        AnalysisKind::Stats => match compute(board, &settings.stats) {
            Ok(report) => stats_report_text(&report, lang),
            Err(error) => error.message(lang),
        },
        AnalysisKind::TestPoints => match analyse(board, &settings.testpoints) {
            Ok(report) => testpoints_report_text(&report, lang),
            Err(error) => error.message(lang),
        },
        AnalysisKind::Lengths => match analyse_lengths(board, &settings.lengths, None, None) {
            Ok(report) => eh3d_phoenix_lengths::report_text(&report, lang),
            Err(error) => error.message(lang),
        },
        AnalysisKind::Dimensions => dimensions_text(board, lang),
        AnalysisKind::Bom => {
            let (parts, prices) = (PartsFile::default(), PriceBook::default());
            let inputs = BomInputs {
                board,
                schematic: None,
                parts: &parts,
                prices: &prices,
                settings: &settings.costs,
                stackup: None,
            };
            match build_bom(&inputs) {
                Ok(bom) => bom_text(&bom, lang),
                Err(error) => error.message(lang),
            }
        }
    }
}

/// The list of the dimensions of a board in a language, in the order of their ids: the number,
/// and for each its kind, its measure in millimetres, its layer and the two points it measures.
pub fn dimensions_text(board: &Board, lang: Lang) -> String {
    if board.dimensions.is_empty() {
        return AnalysisMsg::DimensionsNone.tr(lang).to_owned();
    }
    let mut out = format!(
        "{}: {}\n",
        AnalysisMsg::Dimensions.tr(lang),
        board.dimensions.len()
    );
    for dimension in board.dimensions.values() {
        let kind = match dimension.kind {
            DimensionKind::Aligned => AnalysisMsg::KindAligned,
            DimensionKind::Horizontal => AnalysisMsg::KindHorizontal,
            DimensionKind::Vertical => AnalysisMsg::KindVertical,
        };
        let layer = board
            .layers
            .get(&dimension.layer)
            .map_or("?", |l| l.name.as_str());
        let _ = writeln!(
            out,
            "{}: {} mm, {layer}, ({}, {}) -> ({}, {}) mm",
            kind.tr(lang),
            dimension.measure_text(),
            mm_text(dimension.from.x_nm.get()),
            mm_text(dimension.from.y_nm.get()),
            mm_text(dimension.to.x_nm.get()),
            mm_text(dimension.to.y_nm.get())
        );
    }
    out
}

impl AnalysisTool {
    /// Makes a report of the board as it is now and keeps its text.
    pub fn run(&mut self, kind: AnalysisKind, board: &Board, lang: Lang) {
        let text = report_for(kind, board, &AnalysisSettings::load(), lang);
        self.shown = Some((kind, text));
    }

    /// Closes the report.
    pub fn clear(&mut self) {
        self.shown = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [AnalysisKind; 5] = [
        AnalysisKind::Stats,
        AnalysisKind::TestPoints,
        AnalysisKind::Lengths,
        AnalysisKind::Bom,
        AnalysisKind::Dimensions,
    ];

    #[test]
    fn a_report_is_kept_until_it_is_closed() {
        let board = Board::default();
        let mut tool = AnalysisTool::default();
        assert!(tool.shown.is_none());
        tool.run(AnalysisKind::Stats, &board, Lang::En);
        let (kind, text) = tool.shown.clone().unwrap();
        assert_eq!(kind, AnalysisKind::Stats);
        assert!(!text.is_empty());
        tool.run(AnalysisKind::Lengths, &board, Lang::Es);
        assert_eq!(tool.shown.as_ref().unwrap().0, AnalysisKind::Lengths);
        tool.clear();
        assert!(tool.shown.is_none());
    }

    #[test]
    fn the_dimensions_are_listed_in_the_language_with_their_numbers() {
        use eh3d_phoenix_board::{Dimension, Layer, LayerKind};
        use eh3d_phoenix_model::{Nm, Point};
        let mut board = Board::default();
        assert_eq!(
            dimensions_text(&board, Lang::En),
            "The board has no dimensions"
        );
        assert_eq!(dimensions_text(&board, Lang::Es), "La placa no tiene cotas");
        let layer = eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, 1, 1).unwrap();
        board.layers.insert(
            layer,
            Layer {
                name: "Notes".to_string(),
                kind: LayerKind::Documentation,
                side: None,
            },
        );
        let point = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
        for (n, kind, from, to) in [
            (
                2,
                DimensionKind::Aligned,
                point(0, 0),
                point(3_000_000, 4_000_000),
            ),
            (
                3,
                DimensionKind::Horizontal,
                point(-1_500_000, 0),
                point(11_000_000, 2_000_000),
            ),
        ] {
            board.dimensions.insert(
                eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, n, u64::from(n))
                    .unwrap(),
                Dimension {
                    layer,
                    kind,
                    from,
                    to,
                    offset: Nm::new(1_000_000).unwrap(),
                    text_height: Nm::new(1_000_000).unwrap(),
                    stroke: Nm::new(100_000).unwrap(),
                },
            );
        }
        assert_eq!(
            dimensions_text(&board, Lang::En),
            "Dimensions: 2\nAligned: 5 mm, Notes, (0, 0) -> (3, 4) mm\nHorizontal: 12.5 mm, Notes, (-1.5, 0) -> (11, 2) mm\n"
        );
        assert!(dimensions_text(&board, Lang::De).starts_with("Bemaßungen: 2\nAusgerichtet: 5 mm"));
        assert!(dimensions_text(&board, Lang::Ja).starts_with("寸法: 2\n平行: 5 mm"));
        let mut tool = AnalysisTool::default();
        tool.run(AnalysisKind::Dimensions, &board, Lang::Pt);
        assert!(tool
            .shown
            .unwrap()
            .1
            .starts_with("Cotas: 2\nAlinhada: 5 mm"));
    }

    #[test]
    fn the_report_follows_the_language_and_is_the_same_every_time() {
        let board = Board::default();
        let settings = AnalysisSettings::default();
        let mut differs = 0;
        for kind in KINDS {
            let en = report_for(kind, &board, &settings, Lang::En);
            assert!(!en.is_empty(), "{kind:?}");
            assert_eq!(en, report_for(kind, &board, &settings, Lang::En));
            let de = report_for(kind, &board, &settings, Lang::De);
            if en != de {
                differs += 1;
            }
        }
        assert!(differs > 0, "no report follows the language");
    }
}
