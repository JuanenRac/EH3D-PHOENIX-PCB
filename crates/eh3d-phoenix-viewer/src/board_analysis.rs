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
use eh3d_phoenix_boarddiff::{compare_boards, report_text as diff_report_text, DiffSettings};
use eh3d_phoenix_bom::codes::{FILE_TOO_LARGE, INPUT_UNREADABLE};
use eh3d_phoenix_bom::{
    bom_text, build_bom, plan_sourcing, sourcing_text, BomError, BomInputs, CostSettings,
    PartsFile, PriceBook,
};
use eh3d_phoenix_lengths::{analyse as analyse_lengths, LengthSettings};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stats::{compute, report_text as stats_report_text, StatsSettings};
use eh3d_phoenix_sync::nets_board::board_table;
use eh3d_phoenix_sync::nets_codes::title as nets_title;
use eh3d_phoenix_sync::nets_text::{lint as lint_nets, summary as nets_summary, NetsSettings};
use eh3d_phoenix_testpoints::{analyse, report_text as testpoints_report_text, TestPointSettings};

use crate::board_analysis_text::AnalysisMsg;
use crate::codes::column;

/// The settings file of the statistics.
const STATS_CONFIG: &str = "config/stats.cfg";

/// The settings file of the test points.
const TESTPOINTS_CONFIG: &str = "config/testpoints.cfg";

/// The settings file of the lengths.
const LENGTHS_CONFIG: &str = "config/lengths.cfg";

/// The settings file of the bill of materials.
const COSTS_CONFIG: &str = "config/costs.cfg";

/// The settings file of the nets.
const NETS_CONFIG: &str = "config/nets.cfg";

/// The settings file of the comparison of boards.
const DIFF_CONFIG: &str = "config/boarddiff.cfg";

/// How many of the largest nets the report of the nets lists.
const NETS_TOP: usize = 5;

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
    /// The supplier assistant: the bill priced at each supplier from a price file, with the
    /// shipping, the least order and the purchase split among suppliers.
    Sourcing,
    /// The nets: how many nets, parts and pins, the nets with the most pins and the findings of
    /// the lint of `phoenix nets --lint`, with the settings of `config/nets.cfg`.
    Nets,
    /// What changed in the board since the state that was marked, by the comparison of
    /// `phoenix boarddiff` with the settings of `config/boarddiff.cfg`.
    Changes,
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
    /// The settings of the nets: the limits of the lint and the nets it leaves out.
    pub nets: NetsSettings,
    /// The settings of the comparison of boards: the tolerances and what is left out.
    pub diff: DiffSettings,
    /// The price file of the supplier assistant; empty when none was given.
    pub prices_file: String,
}

/// The state of the reports of the board editor.
#[derive(Debug, Default)]
pub struct AnalysisTool {
    /// The last report: which one it is and its text in the language it was asked for.
    pub shown: Option<(AnalysisKind, String)>,
    /// The path of the price file that the supplier assistant reads, as it was typed.
    pub prices_path: String,
    /// The board as it was when the state was marked; the report of the changes compares the
    /// board as it is now with it.
    pub baseline: Option<Board>,
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
            nets: read_config(NETS_CONFIG)
                .and_then(|text| NetsSettings::parse(&text).ok())
                .unwrap_or_default(),
            diff: read_config(DIFF_CONFIG)
                .and_then(|text| DiffSettings::parse(&text).ok())
                .unwrap_or_default(),
            prices_file: String::new(),
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
        AnalysisKind::Sourcing => sourcing_report(board, settings, lang),
        AnalysisKind::Nets => nets_report(board, settings, lang),
        AnalysisKind::Changes => changes_report(None, board, settings, lang),
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

/// Reads the price file at `path`, no larger than the limit of the settings of the bill and with
/// no more rows than they allow.
fn read_prices(path: &str, costs: &CostSettings) -> Result<PriceBook, BomError> {
    let unreadable = |e: std::io::Error| BomError::new(INPUT_UNREADABLE, format!("{path}: {e}"));
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    if size > costs.max_file_bytes {
        return Err(BomError::new(
            FILE_TOO_LARGE,
            format!(
                "{path} is {size} bytes, the limit is {}",
                costs.max_file_bytes
            ),
        ));
    }
    let text = std::fs::read_to_string(path).map_err(unreadable)?;
    PriceBook::parse(&text, costs.max_items)
}

/// The report of the supplier assistant of a board in a language (the text of
/// `phoenix bom --sourcing`): the bill priced at each supplier of the price file named in the
/// settings, with the shipping and the least order of `config/costs.cfg`. Without a price file it
/// says that one is needed; a file that cannot be read gives the message of the refusal.
pub fn sourcing_report(board: &Board, settings: &AnalysisSettings, lang: Lang) -> String {
    let path = settings.prices_file.trim();
    if path.is_empty() {
        return AnalysisMsg::PricesNeeded.tr(lang).to_owned();
    }
    let prices = match read_prices(path, &settings.costs) {
        Ok(prices) => prices,
        Err(error) => return error.message(lang),
    };
    let parts = PartsFile::default();
    let inputs = BomInputs {
        board,
        schematic: None,
        parts: &parts,
        prices: &prices,
        settings: &settings.costs,
        stackup: None,
    };
    match build_bom(&inputs) {
        Ok(bom) => sourcing_text(&plan_sourcing(&bom, &prices, &settings.costs), &bom),
        Err(error) => error.message(lang),
    }
}

/// What changed in a board since the state that was marked, in a language: the report of the
/// comparison of two boards (`phoenix boarddiff`), the marked one being the old board, with the
/// settings of `config/boarddiff.cfg`. Without a marked state it says that one must be marked.
pub fn changes_report(
    baseline: Option<&Board>,
    board: &Board,
    settings: &AnalysisSettings,
    lang: Lang,
) -> String {
    let Some(baseline) = baseline else {
        return AnalysisMsg::MarkNeeded.tr(lang).to_owned();
    };
    match compare_boards(baseline, board, &settings.diff) {
        Ok(report) => diff_report_text(&report, lang),
        Err(error) => error.message(lang),
    }
}

/// The report of the nets of a board in a language: the number of nets, parts and pins, the nets
/// with the most pins, what could not be written as a netlist and the findings of the lint of
/// `phoenix nets --lint`, judged by the same function with the settings of `config/nets.cfg`, so
/// that the window and the command give the same findings. A board with no net that has a pin
/// says so.
pub fn nets_report(board: &Board, settings: &AnalysisSettings, lang: Lang) -> String {
    let (table, notes) = board_table(board);
    if table.nets.is_empty() {
        return AnalysisMsg::NetsNone.tr(lang).to_owned();
    }
    let sum = nets_summary(&table, NETS_TOP);
    let result = lint_nets(&table, &settings.nets);
    let mut out = format!(
        "{}: {}\n{}: {}\n{}: {}\n",
        AnalysisMsg::Nets.tr(lang),
        sum.nets,
        AnalysisMsg::Parts.tr(lang),
        sum.parts,
        AnalysisMsg::Pins.tr(lang),
        sum.pins
    );
    let _ = writeln!(out, "{}:", AnalysisMsg::LargestNets.tr(lang));
    for (name, pins) in &sum.largest {
        let _ = writeln!(out, "  {name}: {pins}");
    }
    for note in &notes {
        let _ = writeln!(out, "{}: {}", note.code, note.detail);
    }
    let _ = writeln!(out, "{}: {}", AnalysisMsg::Findings.tr(lang), result.total);
    for finding in &result.findings {
        let _ = write!(
            out,
            "{}: {}: {}",
            finding.code,
            nets_title(finding.code, column(lang)),
            finding.net
        );
        if let Some(other) = &finding.other {
            let _ = write!(out, " ~ {other}");
        }
        out.push('\n');
    }
    if result.total > result.findings.len() {
        let _ = writeln!(out, "... {}", result.total - result.findings.len());
    }
    out
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
            DimensionKind::Radius => AnalysisMsg::KindRadius,
            DimensionKind::Diameter => AnalysisMsg::KindDiameter,
            DimensionKind::Angle => AnalysisMsg::KindAngle,
        };
        let layer = board
            .layers
            .get(&dimension.layer)
            .map_or("?", |l| l.name.as_str());
        // an angle goes through a third point
        let third = dimension.third.map_or(String::new(), |p| {
            format!(" -> ({}, {})", mm_text(p.x_nm.get()), mm_text(p.y_nm.get()))
        });
        let _ = writeln!(
            out,
            "{}: {}, {layer}, ({}, {}) -> ({}, {}){third} mm",
            kind.tr(lang),
            dimension.measure_with_unit(),
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
        let mut settings = AnalysisSettings::load();
        settings.prices_file = self.prices_path.clone();
        let text = match kind {
            AnalysisKind::Changes => changes_report(self.baseline.as_ref(), board, &settings, lang),
            _ => report_for(kind, board, &settings, lang),
        };
        self.shown = Some((kind, text));
    }

    /// Marks the board as it is now: the report of the changes compares the board with this
    /// state from now on. The marked state replaces an earlier one.
    pub fn mark(&mut self, board: &Board, lang: Lang) {
        self.baseline = Some(board.clone());
        self.shown = Some((
            AnalysisKind::Changes,
            AnalysisMsg::Marked.tr(lang).to_owned(),
        ));
    }

    /// Closes the report.
    pub fn clear(&mut self) {
        self.shown = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [AnalysisKind; 7] = [
        AnalysisKind::Stats,
        AnalysisKind::TestPoints,
        AnalysisKind::Lengths,
        AnalysisKind::Bom,
        AnalysisKind::Dimensions,
        AnalysisKind::Sourcing,
        AnalysisKind::Nets,
    ];

    /// A board with the nets GND (R1.2, R2.2, TP1.1) and SIG (R1.1, R2.1); `lonely` adds a part
    /// J1 whose only pin is on a third net named `N$3`.
    fn board_with_nets(lonely: bool) -> Board {
        use eh3d_phoenix_board::{Component, Footprint, Id, Net, Pad, PadNet, PadShape, Side};
        use eh3d_phoenix_model::{ObjectId, Udeg};
        let id = |n: u32| -> Id {
            ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
        };
        let pad = |number: &str| Pad {
            number: number.into(),
            shape: PadShape::Circle {
                diameter: eh3d_phoenix_model::Nm::new(800_000).unwrap(),
            },
            at: eh3d_phoenix_model::Point::from_nm(0, 0).unwrap(),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![id(1)],
            roundness: 0,
            plated: true,
        };
        let mut board = Board::default();
        board.nets.insert(id(2), Net { name: "GND".into() });
        board.nets.insert(id(3), Net { name: "SIG".into() });
        board.nets.insert(id(4), Net { name: "N$3".into() });
        let mut two = Footprint {
            name: "R_0603".into(),
            pads: Default::default(),
        };
        two.pads.insert(id(11), pad("1"));
        two.pads.insert(id(12), pad("2"));
        board.footprints.insert(id(10), two);
        let part = |refdes: &str, footprint: Id| Component {
            refdes: refdes.into(),
            value: String::new(),
            symbol: None,
            footprint,
            at: eh3d_phoenix_model::Point::from_nm(0, 0).unwrap(),
            rotation: Udeg::ZERO,
            side: Side::Top,
        };
        let place = |board: &mut Board, n: u32, refdes: &str, pins: &[(u32, u32)]| {
            board.components.insert(id(n), part(refdes, id(10)));
            for (pad_id, net) in pins {
                board.pad_nets.push(PadNet {
                    component: id(n),
                    pad: id(*pad_id),
                    net: id(*net),
                });
            }
        };
        place(&mut board, 20, "R1", &[(11, 3), (12, 2)]);
        place(&mut board, 21, "R2", &[(11, 3), (12, 2)]);
        place(&mut board, 22, "TP1", &[(12, 2)]);
        if lonely {
            place(&mut board, 23, "J1", &[(11, 4)]);
        }
        board.pad_nets.sort_by_key(PadNet::pad_ref);
        board
    }

    #[test]
    fn the_changes_need_a_marked_state_and_say_so_in_each_language() {
        let settings = AnalysisSettings::default();
        let board = Board::default();
        let mut seen = Vec::new();
        for lang in Lang::ALL {
            let text = changes_report(None, &board, &settings, lang);
            assert_eq!(text, AnalysisMsg::MarkNeeded.tr(lang));
            assert!(!seen.contains(&text), "{lang:?} repeats a text");
            seen.push(text);
        }
        assert_eq!(
            report_for(AnalysisKind::Changes, &board, &settings, Lang::En),
            "No state was marked: press Mark this state first"
        );
    }

    #[test]
    fn the_changes_are_the_report_of_the_comparison_of_the_marked_state_with_the_board() {
        use eh3d_phoenix_board::Net;
        use eh3d_phoenix_model::ObjectId;
        let settings = AnalysisSettings::default();
        let marked = Board::default();
        let direct = |old: &Board, new: &Board, lang: Lang| {
            diff_report_text(
                &compare_boards(old, new, &DiffSettings::default()).unwrap(),
                lang,
            )
        };
        // the same board: no differences
        let same = changes_report(Some(&marked), &marked, &settings, Lang::En);
        assert_eq!(same, direct(&marked, &marked, Lang::En));
        assert!(same.contains("No differences"), "{same}");
        // a net more is a difference, written in the language asked for
        let mut now = marked.clone();
        let id = ObjectId::from_parts(1_700_000_000_000, 7, 7).unwrap();
        now.nets.insert(id, Net { name: "GND".into() });
        for lang in Lang::ALL {
            let text = changes_report(Some(&marked), &now, &settings, lang);
            assert_eq!(text, direct(&marked, &now, lang));
            assert!(
                text.contains("PHOENIX-DIFF-006") && text.contains("GND"),
                "{text}"
            );
        }
        // and, the other way round, a net less
        let lost = changes_report(Some(&now), &marked, &settings, Lang::En);
        assert!(
            lost.contains("PHOENIX-DIFF-006") && lost.contains("GND"),
            "{lost}"
        );
    }

    #[test]
    fn marking_keeps_the_board_and_the_changes_follow_the_edits_after_it() {
        use eh3d_phoenix_board::Net;
        use eh3d_phoenix_model::ObjectId;
        let mut tool = AnalysisTool::default();
        let mut board = Board::default();
        tool.run(AnalysisKind::Changes, &board, Lang::En);
        assert_eq!(
            tool.shown.as_ref().unwrap().1,
            "No state was marked: press Mark this state first"
        );
        tool.mark(&board, Lang::Es);
        assert!(tool.baseline.is_some());
        assert_eq!(
            tool.shown,
            Some((
                AnalysisKind::Changes,
                "Se marcó el estado de la placa".to_owned()
            ))
        );
        tool.run(AnalysisKind::Changes, &board, Lang::En);
        assert!(tool.shown.as_ref().unwrap().1.contains("No differences"));
        // the board is edited after the mark
        let id = ObjectId::from_parts(1_700_000_000_000, 9, 9).unwrap();
        board.nets.insert(id, Net { name: "VCC".into() });
        tool.run(AnalysisKind::Changes, &board, Lang::En);
        assert!(tool.shown.as_ref().unwrap().1.contains("VCC"));
        // marking again takes the new state as the one to compare with
        tool.mark(&board, Lang::En);
        tool.run(AnalysisKind::Changes, &board, Lang::En);
        assert!(tool.shown.as_ref().unwrap().1.contains("No differences"));
    }

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
                    third: None,
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
        // a radius, a diameter and an angle, with their units and the third point of the angle
        {
            let mut board = board.clone();
            let more = |n: u16, kind, from, to, third, offset: i64| {
                (
                    eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, n, u64::from(n))
                        .unwrap(),
                    Dimension {
                        layer,
                        kind,
                        from,
                        to,
                        third,
                        offset: Nm::new(offset).unwrap(),
                        text_height: Nm::new(1_000_000).unwrap(),
                        stroke: Nm::new(100_000).unwrap(),
                    },
                )
            };
            board.dimensions.clear();
            for (id, d) in [
                more(
                    4,
                    DimensionKind::Radius,
                    point(0, 0),
                    point(5_000_000, 0),
                    None,
                    0,
                ),
                more(
                    5,
                    DimensionKind::Diameter,
                    point(0, 0),
                    point(8_000_000, 0),
                    None,
                    0,
                ),
                more(
                    6,
                    DimensionKind::Angle,
                    point(0, 0),
                    point(10_000_000, 0),
                    Some(point(0, 10_000_000)),
                    5_000_000,
                ),
            ] {
                board.dimensions.insert(id, d);
            }
            assert_eq!(
            dimensions_text(&board, Lang::En),
            "Dimensions: 3\nRadius: 5 mm, Notes, (0, 0) -> (5, 0) mm\nDiameter: 8 mm, Notes, (0, 0) -> (8, 0) mm\nAngle: 90\u{b0}, Notes, (0, 0) -> (10, 0) -> (0, 10) mm\n"
        );
            assert!(dimensions_text(&board, Lang::Pt).contains("\nÂngulo: 90\u{b0}"));
        }
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
    fn the_supplier_report_asks_for_a_price_file_and_reads_the_one_typed() {
        let board = Board::default();
        let mut settings = AnalysisSettings::default();
        assert_eq!(
            sourcing_report(&board, &settings, Lang::En),
            AnalysisMsg::PricesNeeded.tr(Lang::En)
        );
        assert_ne!(
            sourcing_report(&board, &settings, Lang::Pt),
            sourcing_report(&board, &settings, Lang::En)
        );
        // a file that is not there is refused with its code and its path
        let folder = tempfile::tempdir().unwrap();
        let missing = folder.path().join("none.csv");
        settings.prices_file = missing.to_string_lossy().into_owned();
        let refused = sourcing_report(&board, &settings, Lang::En);
        assert!(refused.starts_with("PHOENIX-BOM-016"), "{refused}");
        // a file that is larger than the limit of the settings is refused
        let file = folder.path().join("prices.csv");
        std::fs::write(
            &file,
            "value,footprint,supplier,min_qty,unit_price
10k,R0603,A,1,0.01
",
        )
        .unwrap();
        settings.prices_file = file.to_string_lossy().into_owned();
        settings.costs.max_file_bytes = 10;
        let large = sourcing_report(&board, &settings, Lang::En);
        assert!(large.starts_with("PHOENIX-BOM-002"), "{large}");
        // a good file gives the plan of the assistant
        settings.costs.max_file_bytes = 1 << 20;
        let plan = sourcing_report(&board, &settings, Lang::En);
        assert!(plan.starts_with("suppliers: "), "{plan}");
        // and the tool keeps the path it was given and uses it
        let mut tool = AnalysisTool {
            prices_path: settings.prices_file.clone(),
            ..AnalysisTool::default()
        };
        tool.run(AnalysisKind::Sourcing, &board, Lang::En);
        assert_eq!(tool.shown.as_ref().unwrap().0, AnalysisKind::Sourcing);
    }

    #[test]
    fn the_report_of_the_nets_counts_them_lists_the_largest_and_says_the_findings() {
        let settings = AnalysisSettings::default();
        assert_eq!(
            nets_report(&board_with_nets(false), &settings, Lang::En),
            "Nets: 2\nParts: 3\nPins: 5\nLargest nets:\n  GND: 3\n  SIG: 2\nFindings: 0\n"
        );
        assert_eq!(
            nets_report(&board_with_nets(false), &settings, Lang::Es),
            "Redes: 2\nPiezas: 3\nPines: 5\nRedes más grandes:\n  GND: 3\n  SIG: 2\nHallazgos: 0\n"
        );
        assert_eq!(
            nets_report(&board_with_nets(true), &settings, Lang::En),
            "Nets: 3\nParts: 4\nPins: 6\nLargest nets:\n  GND: 3\n  SIG: 2\n  N$3: 1\nFindings: 1\nPHOENIX-NETS-017: Net has a single pin: N$3\n"
        );
        let pt = nets_report(&board_with_nets(true), &settings, Lang::Pt);
        assert!(
            pt.contains("PHOENIX-NETS-017: A rede tem um só pino: N$3\n"),
            "{pt}"
        );
        assert!(
            pt.starts_with("Redes: 3\nPeças: 4\nPinos: 6\nMaiores redes:\n"),
            "{pt}"
        );
    }

    #[test]
    fn a_board_without_a_net_with_a_pin_says_so_in_the_language() {
        let settings = AnalysisSettings::default();
        let board = Board::default();
        assert_eq!(
            nets_report(&board, &settings, Lang::En),
            "The board has no nets with pins"
        );
        assert_eq!(
            nets_report(&board, &settings, Lang::De),
            "Die Platine hat keine Netze mit Pins"
        );
        assert_eq!(
            nets_report(&board, &settings, Lang::Ja),
            "この基板にはピンのあるネットがありません"
        );
    }

    #[test]
    fn the_window_gives_the_findings_of_the_command_with_the_same_settings() {
        use eh3d_phoenix_sync::nets_text::lint;
        let board = board_with_nets(true);
        let mut settings = AnalysisSettings {
            nets: NetsSettings::parse(
                "lint_max_pins = 2\nlint_part_on_one_net = true\nlint_generated_names = true",
            )
            .unwrap(),
            ..AnalysisSettings::default()
        };
        let (table, _) = board_table(&board);
        let result = lint(&table, &settings.nets);
        assert_eq!(result.total, 3);
        let text = nets_report(&board, &settings, Lang::En);
        assert!(text.contains("Findings: 3\n"), "{text}");
        let lines: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.starts_with("Findings"))
            .skip(1)
            .collect();
        assert_eq!(
            lines,
            vec![
                "PHOENIX-NETS-020: Net has more pins than the limit of the lint: GND ~ 3",
                "PHOENIX-NETS-017: Net has a single pin: N$3",
                "PHOENIX-NETS-022: Net has a name made by the program: N$3",
            ]
        );
        // the nets that the settings leave out are left out of the window too
        settings.nets = NetsSettings::parse("ignore_nets = n$*, gnd").unwrap();
        let text = nets_report(&board, &settings, Lang::En);
        assert!(text.ends_with("Findings: 0\n"), "{text}");
        // a list that the limit cuts says how many are not listed
        settings.nets = NetsSettings::parse(
            "lint_max_pins = 2\nlint_generated_names = true\nmax_differences = 1",
        )
        .unwrap();
        let text = nets_report(&board, &settings, Lang::En);
        assert!(text.ends_with("... 2\n"), "{text}");
    }

    #[test]
    fn the_nets_report_is_kept_by_the_tool_and_follows_the_language() {
        let board = board_with_nets(true);
        let mut tool = AnalysisTool::default();
        tool.run(AnalysisKind::Nets, &board, Lang::En);
        let (kind, text) = tool.shown.clone().unwrap();
        assert_eq!(kind, AnalysisKind::Nets);
        assert!(text.starts_with("Nets: 3\n"), "{text}");
        tool.run(AnalysisKind::Nets, &board, Lang::Fr);
        assert!(tool.shown.as_ref().unwrap().1.starts_with("Réseaux: 3\n"));
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
