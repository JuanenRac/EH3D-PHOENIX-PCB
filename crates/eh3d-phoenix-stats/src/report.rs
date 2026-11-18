// SPDX-License-Identifier: GPL-3.0-or-later
//! The text and the JSON of a statistics report. The same board always gives the same bytes:
//! the order is fixed, the numbers are integers and nothing depends on the time or the machine.
//! The titles of the text are in the eight languages of the suite; the JSON keeps its keys.

use eh3d_phoenix_rules::Lang;

use crate::codes::{Finding, Severity};
use crate::stats::BoardStats;

/// The words of the text report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The title of the report.
    Title,
    /// The size and area of the board.
    Board,
    /// The openings of the outline.
    Cutouts,
    /// The parts placed.
    Components,
    /// The top side.
    Top,
    /// The bottom side.
    Bottom,
    /// The nets.
    Nets,
    /// The pads.
    Pads,
    /// The pads without a drill.
    SmdPads,
    /// The pads with a plated drill.
    ThroughPads,
    /// The holes that are not plated.
    UnplatedHoles,
    /// The tracks.
    Tracks,
    /// The length of the tracks.
    TrackLength,
    /// The vias.
    Vias,
    /// The zones.
    Zones,
    /// The text and strokes of the silkscreen.
    Silkscreen,
    /// The copper layers.
    CopperLayers,
    /// The copper of a layer.
    Copper,
    /// The share of the board covered.
    Coverage,
    /// The drills.
    Drills,
    /// The plated pads of a drill.
    PlatedPads,
    /// The holes that are not plated, of a drill.
    UnplatedPads,
    /// The density.
    Density,
    /// The holes.
    Holes,
    /// The findings.
    Findings,
    /// Nothing to list.
    None,
}

/// Every label with its text in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese, in the order of [`Label`].
const LABELS: &[(Label, [&str; 8])] = &[
    (
        Label::Title,
        [
            "Board statistics",
            "Estadísticas de la placa",
            "Statistik der Platine",
            "Statistiques de la carte",
            "Statistiche della scheda",
            "基板の統計",
            "电路板统计",
            "Estatísticas da placa",
        ],
    ),
    (
        Label::Board,
        [
            "Board",
            "Placa",
            "Platine",
            "Carte",
            "Scheda",
            "基板",
            "电路板",
            "Placa",
        ],
    ),
    (
        Label::Cutouts,
        [
            "Cutouts",
            "Recortes",
            "Ausschnitte",
            "Découpes",
            "Ritagli",
            "切り欠き",
            "开槽",
            "Recortes",
        ],
    ),
    (
        Label::Components,
        [
            "Components",
            "Componentes",
            "Bauteile",
            "Composants",
            "Componenti",
            "部品",
            "元件",
            "Componentes",
        ],
    ),
    (
        Label::Top,
        [
            "top", "arriba", "oben", "dessus", "sopra", "上面", "顶层", "em cima",
        ],
    ),
    (
        Label::Bottom,
        [
            "bottom", "abajo", "unten", "dessous", "sotto", "下面", "底层", "em baixo",
        ],
    ),
    (
        Label::Nets,
        [
            "Nets",
            "Redes",
            "Netze",
            "Réseaux",
            "Reti",
            "ネット",
            "网络",
            "Redes",
        ],
    ),
    (
        Label::Pads,
        [
            "Pads",
            "Pads",
            "Pads",
            "Pastilles",
            "Piazzole",
            "パッド",
            "焊盘",
            "Pastilhas",
        ],
    ),
    (
        Label::SmdPads,
        [
            "surface-mount",
            "de montaje superficial",
            "SMD",
            "CMS",
            "SMD",
            "表面実装",
            "表贴",
            "de montagem à superfície",
        ],
    ),
    (
        Label::ThroughPads,
        [
            "through-hole",
            "pasantes",
            "durchkontaktiert",
            "traversantes",
            "passanti",
            "スルーホール",
            "通孔",
            "furo metalizado",
        ],
    ),
    (
        Label::UnplatedHoles,
        [
            "unplated holes",
            "agujeros sin metalizar",
            "Löcher ohne Durchkontaktierung",
            "trous non métallisés",
            "fori non metallizzati",
            "非メッキ穴",
            "非金属化孔",
            "furos não metalizados",
        ],
    ),
    (
        Label::Tracks,
        [
            "Tracks",
            "Pistas",
            "Leiterbahnen",
            "Pistes",
            "Piste",
            "配線",
            "走线",
            "Pistas",
        ],
    ),
    (
        Label::TrackLength,
        [
            "length",
            "longitud",
            "Länge",
            "longueur",
            "lunghezza",
            "長さ",
            "长度",
            "comprimento",
        ],
    ),
    (
        Label::Vias,
        [
            "Vias",
            "Vías",
            "Durchkontaktierungen",
            "Vias",
            "Via",
            "ビア",
            "过孔",
            "Vias",
        ],
    ),
    (
        Label::Zones,
        [
            "Zones",
            "Zonas",
            "Zonen",
            "Zones",
            "Zone",
            "ゾーン",
            "铜区",
            "Zonas",
        ],
    ),
    (
        Label::Silkscreen,
        [
            "Silkscreen items",
            "Elementos de serigrafía",
            "Bestückungsdruck-Elemente",
            "Éléments de sérigraphie",
            "Elementi di serigrafia",
            "シルクの要素",
            "丝印元素",
            "Elementos de serigrafia",
        ],
    ),
    (
        Label::CopperLayers,
        [
            "Copper layers",
            "Capas de cobre",
            "Kupferlagen",
            "Couches de cuivre",
            "Strati di rame",
            "銅箔層",
            "铜层",
            "Camadas de cobre",
        ],
    ),
    (
        Label::Copper,
        [
            "copper", "cobre", "Kupfer", "cuivre", "rame", "銅", "铜", "cobre",
        ],
    ),
    (
        Label::Coverage,
        [
            "coverage",
            "cobertura",
            "Abdeckung",
            "couverture",
            "copertura",
            "被覆率",
            "覆盖率",
            "cobertura",
        ],
    ),
    (
        Label::Drills,
        [
            "Drills",
            "Taladros",
            "Bohrungen",
            "Perçages",
            "Fori",
            "ドリル",
            "钻孔",
            "Furos",
        ],
    ),
    (
        Label::PlatedPads,
        [
            "plated pads",
            "pads metalizados",
            "durchkontaktierte Pads",
            "pastilles métallisées",
            "piazzole metallizzate",
            "メッキパッド",
            "金属化焊盘",
            "pastilhas metalizadas",
        ],
    ),
    (
        Label::UnplatedPads,
        [
            "unplated",
            "sin metalizar",
            "nicht durchkontaktiert",
            "non métallisés",
            "non metallizzati",
            "非メッキ",
            "非金属化",
            "não metalizados",
        ],
    ),
    (
        Label::Density,
        [
            "Density per cm2",
            "Densidad por cm2",
            "Dichte pro cm2",
            "Densité par cm2",
            "Densità per cm2",
            "cm2 あたりの密度",
            "每平方厘米密度",
            "Densidade por cm2",
        ],
    ),
    (
        Label::Holes,
        [
            "holes", "agujeros", "Löcher", "trous", "fori", "穴", "孔", "furos",
        ],
    ),
    (
        Label::Findings,
        [
            "Notes",
            "Notas",
            "Hinweise",
            "Remarques",
            "Note",
            "注記",
            "说明",
            "Notas",
        ],
    ),
    (
        Label::None,
        [
            "none", "ninguno", "keine", "aucun", "nessuno", "なし", "无", "nenhum",
        ],
    ),
];

/// A word of the report in a language.
pub fn label(which: Label, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    LABELS
        .iter()
        .find(|(l, _)| *l == which)
        .map_or("", |(_, row)| row[column])
}

/// A length in nanometres as millimetres with six decimals, exactly.
pub fn millimetres(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
}

/// An area in square micrometres as square millimetres with six decimals, exactly.
pub fn square_millimetres(um2: i64) -> String {
    let sign = if um2 < 0 { "-" } else { "" };
    let a = um2.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
}

/// A number of hundredths with two decimals, exactly.
pub fn hundredths(x100: i64) -> String {
    let sign = if x100 < 0 { "-" } else { "" };
    let a = x100.unsigned_abs();
    format!("{sign}{}.{:02}", a / 100, a % 100)
}

/// The numbers of the findings by severity: errors, warnings and information.
pub fn counts(findings: &[Finding]) -> (usize, usize, usize) {
    let of = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    (
        of(Severity::Error),
        of(Severity::Warning),
        of(Severity::Info),
    )
}

/// The findings as text, one line each, errors first.
pub fn findings_text(findings: &[Finding], lang: Lang) -> String {
    let mut sorted: Vec<&Finding> = findings.iter().collect();
    sorted.sort_by_key(|f| std::cmp::Reverse(f.severity));
    let mut out = String::new();
    for finding in sorted {
        out.push_str(&finding.line(lang));
        out.push('\n');
    }
    out
}

/// The report as text, with the titles in a language.
pub fn report_text(stats: &BoardStats, lang: Lang) -> String {
    let l = |which: Label| label(which, lang);
    let b = &stats.board;
    let c = &stats.counts;
    let mut out = format!("{}: {}\n", l(Label::Title), stats.name);
    out.push_str(&format!(
        "{}: {} x {} mm; {} mm2; {}: {} mm2\n",
        l(Label::Board),
        millimetres(b.width_nm),
        millimetres(b.height_nm),
        square_millimetres(b.area_um2),
        l(Label::Cutouts),
        square_millimetres(b.cutouts_um2)
    ));
    out.push_str(&format!(
        "{}: {} ({}: {}; {}: {})\n",
        l(Label::Components),
        c.components,
        l(Label::Top),
        c.components_top,
        l(Label::Bottom),
        c.components_bottom
    ));
    out.push_str(&format!("{}: {}\n", l(Label::Nets), c.nets));
    out.push_str(&format!(
        "{}: {} ({}: {}; {}: {}; {}: {})\n",
        l(Label::Pads),
        c.pads,
        l(Label::SmdPads),
        c.smd_pads,
        l(Label::ThroughPads),
        c.through_pads,
        l(Label::UnplatedHoles),
        c.unplated_holes
    ));
    out.push_str(&format!(
        "{}: {} ({}: {} mm)\n",
        l(Label::Tracks),
        c.tracks,
        l(Label::TrackLength),
        millimetres(c.track_length_nm)
    ));
    out.push_str(&format!("{}: {}\n", l(Label::Vias), c.vias));
    out.push_str(&format!("{}: {}\n", l(Label::Zones), c.zones));
    out.push_str(&format!(
        "{}: {}\n",
        l(Label::Silkscreen),
        c.silkscreen_items
    ));
    out.push_str(&format!("{}:\n", l(Label::CopperLayers)));
    if stats.layers.is_empty() {
        out.push_str(&format!("  {}\n", l(Label::None)));
    }
    for layer in &stats.layers {
        out.push_str(&format!(
            "  {}: {} {} mm2 ({}: {}; {}: {}; {}: {}; {}: {}), {} {} %\n",
            layer.layer,
            l(Label::Copper),
            square_millimetres(layer.copper_um2),
            l(Label::Pads),
            square_millimetres(layer.pads_um2),
            l(Label::Tracks),
            square_millimetres(layer.tracks_um2),
            l(Label::Vias),
            square_millimetres(layer.vias_um2),
            l(Label::Zones),
            square_millimetres(layer.zones_um2),
            l(Label::Coverage),
            hundredths(layer.coverage_x100)
        ));
    }
    out.push_str(&format!("{}:\n", l(Label::Drills)));
    if stats.drills.is_empty() {
        out.push_str(&format!("  {}\n", l(Label::None)));
    }
    for drill in &stats.drills {
        out.push_str(&format!(
            "  {} mm: {} {}; {} {}; {} {}\n",
            millimetres(drill.diameter_nm),
            drill.vias,
            l(Label::Vias),
            drill.plated_pads,
            l(Label::PlatedPads),
            drill.unplated_pads,
            l(Label::UnplatedPads)
        ));
    }
    let d = &stats.density;
    out.push_str(&format!(
        "{}: {} {}; {} {}; {} {}\n",
        l(Label::Density),
        hundredths(d.components_per_cm2_x100),
        l(Label::Components),
        hundredths(d.pads_per_cm2_x100),
        l(Label::Pads),
        hundredths(d.holes_per_cm2_x100),
        l(Label::Holes)
    ));
    if !stats.findings.is_empty() {
        out.push_str(&format!("{}:\n", l(Label::Findings)));
        out.push_str(&findings_text(&stats.findings, lang));
    }
    out
}

/// The report as JSON.
pub fn report_json(stats: &BoardStats) -> String {
    let mut text = serde_json::to_string_pretty(stats).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{NO_BOARD_AREA, ZONE_OUTLINE_COUNTED};

    #[test]
    fn every_word_has_a_text_in_each_language() {
        for (which, row) in LABELS {
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{which:?} column {column}");
            }
        }
        assert_eq!(LABELS.len(), 26);
        assert_eq!(label(Label::Title, Lang::Es), "Estadísticas de la placa");
        assert_eq!(label(Label::Bottom, Lang::Pt), "em baixo");
    }

    #[test]
    fn no_word_is_listed_twice() {
        let names: Vec<String> = LABELS.iter().map(|(w, _)| format!("{w:?}")).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn numbers_are_written_exactly() {
        assert_eq!(millimetres(0), "0.000000");
        assert_eq!(millimetres(-250_000), "-0.250000");
        assert_eq!(square_millimetres(1_500_000), "1.500000");
        assert_eq!(hundredths(1234), "12.34");
        assert_eq!(hundredths(5), "0.05");
        assert_eq!(hundredths(-250), "-2.50");
    }

    #[test]
    fn the_findings_are_listed_errors_first_and_counted() {
        let list = vec![
            Finding::new(ZONE_OUTLINE_COUNTED, Severity::Info, None, "1 zone"),
            Finding::new(NO_BOARD_AREA, Severity::Warning, None, ""),
        ];
        assert_eq!(counts(&list), (0, 1, 1));
        let text = findings_text(&list, Lang::De);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("warning: PHOENIX-STATS-007"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("info: PHOENIX-STATS-006"),
            "{}",
            lines[1]
        );
        assert_eq!(findings_text(&[], Lang::En), "");
    }
}
