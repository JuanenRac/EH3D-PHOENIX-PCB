// SPDX-License-Identifier: GPL-3.0-or-later
//! The reports of the board editor, without a screen: the statistics of the board as it is now
//! (`phoenix stats`), its test points (`phoenix testpoints`) and the lengths of its nets
//! (`phoenix lengths`) and its bill of materials (`phoenix bom`, without prices), made with the
//! settings of `config/stats.cfg`, `config/testpoints.cfg`, `config/lengths.cfg` and
//! `config/costs.cfg` when those files are there and good, else with the built-in ones, and kept
//! as the text of a language until the next one is asked for or the report is closed. The window
//! shows this text in `board_analysis_gui`, behind the `gui` feature.

use eh3d_phoenix_board::Board;
use eh3d_phoenix_bom::{bom_text, build_bom, BomInputs, CostSettings, PartsFile, PriceBook};
use eh3d_phoenix_lengths::{analyse as analyse_lengths, LengthSettings};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stats::{compute, report_text as stats_report_text, StatsSettings};
use eh3d_phoenix_testpoints::{analyse, report_text as testpoints_report_text, TestPointSettings};

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

    const KINDS: [AnalysisKind; 4] = [
        AnalysisKind::Stats,
        AnalysisKind::TestPoints,
        AnalysisKind::Lengths,
        AnalysisKind::Bom,
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
    fn the_report_follows_the_language_and_is_the_same_every_time() {
        let board = Board::default();
        let settings = AnalysisSettings::default();
        for kind in KINDS {
            let en = report_for(kind, &board, &settings, Lang::En);
            assert!(!en.is_empty(), "{kind:?}");
            assert_eq!(en, report_for(kind, &board, &settings, Lang::En));
            let de = report_for(kind, &board, &settings, Lang::De);
            assert_ne!(en, de, "{kind:?}");
        }
    }
}
