// SPDX-License-Identifier: GPL-3.0-or-later
//! The text and the JSON of a test point report. The same board always gives the same bytes: the
//! order is fixed, the numbers are integers and nothing depends on the time or the machine. The
//! titles of the text are in the eight languages of the suite; the JSON keeps its keys.

use eh3d_phoenix_rules::Lang;

use crate::analyse::TestPointReport;
use crate::codes::{Finding, Severity};

/// The words of the text report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The title of the report.
    Title,
    /// The test points.
    TestPoints,
    /// The side that is probed.
    ProbeSide,
    /// The nets that were asked to have a test point.
    Nets,
    /// The nets that are covered.
    Covered,
    /// The share of the nets covered.
    Coverage,
    /// The size of a test point.
    Size,
    /// The nets without access.
    Uncovered,
    /// The findings.
    Notes,
    /// Nothing to list.
    None,
}

/// Every label with its text in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
const LABELS: &[(Label, [&str; 8])] = &[
    (
        Label::Title,
        [
            "Test points of the board",
            "Puntos de prueba de la placa",
            "Testpunkte der Platine",
            "Points de test de la carte",
            "Punti di test della scheda",
            "基板のテストポイント",
            "电路板测试点",
            "Pontos de teste da placa",
        ],
    ),
    (
        Label::TestPoints,
        [
            "Test points",
            "Puntos de prueba",
            "Testpunkte",
            "Points de test",
            "Punti di test",
            "テストポイント",
            "测试点",
            "Pontos de teste",
        ],
    ),
    (
        Label::ProbeSide,
        [
            "probe side",
            "cara que se sondea",
            "abgetastete Seite",
            "face sondée",
            "lato sondato",
            "プローブ面",
            "探测面",
            "face sondada",
        ],
    ),
    (
        Label::Nets,
        [
            "Nets asked",
            "Redes pedidas",
            "Geforderte Netze",
            "Réseaux demandés",
            "Reti richieste",
            "対象のネット",
            "要求的网络",
            "Redes pedidas",
        ],
    ),
    (
        Label::Covered,
        [
            "covered",
            "cubiertas",
            "abgedeckt",
            "couverts",
            "coperte",
            "カバー済み",
            "已覆盖",
            "cobertas",
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
            "カバー率",
            "覆盖率",
            "cobertura",
        ],
    ),
    (
        Label::Size,
        [
            "size",
            "tamaño",
            "Größe",
            "taille",
            "dimensione",
            "大きさ",
            "尺寸",
            "tamanho",
        ],
    ),
    (
        Label::Uncovered,
        [
            "Nets without access",
            "Redes sin acceso",
            "Netze ohne Zugang",
            "Réseaux sans accès",
            "Reti senza accesso",
            "アクセスできないネット",
            "无法接触的网络",
            "Redes sem acesso",
        ],
    ),
    (
        Label::Notes,
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
            "none", "ninguna", "keine", "aucun", "nessuna", "なし", "无", "nenhuma",
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
pub fn report_text(report: &TestPointReport, lang: Lang) -> String {
    let l = |which: Label| label(which, lang);
    let mut out = format!("{}: {}\n", l(Label::Title), report.name);
    out.push_str(&format!(
        "{}: {} ({}: {})\n",
        l(Label::TestPoints),
        report.test_points.len(),
        l(Label::ProbeSide),
        report.probe_side
    ));
    out.push_str(&format!(
        "{}: {}; {}: {}; {}: {} %\n",
        l(Label::Nets),
        report.nets_asked,
        l(Label::Covered),
        report.nets_covered,
        l(Label::Coverage),
        hundredths(report.coverage_x100)
    ));
    for p in &report.test_points {
        out.push_str(&format!(
            "  {}:{} {} {} {} mm {} mm, {} {} mm\n",
            p.refdes,
            p.pad,
            p.net.as_deref().unwrap_or("-"),
            p.side,
            millimetres(p.x_nm),
            millimetres(p.y_nm),
            l(Label::Size),
            millimetres(p.size_nm)
        ));
    }
    out.push_str(&format!("{}:\n", l(Label::Uncovered)));
    let open: Vec<&str> = report
        .nets
        .iter()
        .filter(|n| !n.covered)
        .map(|n| n.net.as_str())
        .collect();
    if open.is_empty() {
        out.push_str(&format!("  {}\n", l(Label::None)));
    }
    for net in open {
        out.push_str(&format!("  {net}\n"));
    }
    if !report.findings.is_empty() {
        out.push_str(&format!("{}:\n", l(Label::Notes)));
        out.push_str(&findings_text(&report.findings, lang));
    }
    out
}

/// The report as JSON.
pub fn report_json(report: &TestPointReport) -> String {
    let mut text = serde_json::to_string_pretty(report).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{NEAR_EDGE, NO_NET};

    #[test]
    fn every_word_has_a_text_in_each_language() {
        for (which, row) in LABELS {
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{which:?} column {column}");
            }
        }
        assert_eq!(LABELS.len(), 10);
        assert_eq!(label(Label::Uncovered, Lang::Es), "Redes sin acceso");
        assert_eq!(label(Label::Uncovered, Lang::Pt), "Redes sem acesso");
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
        assert_eq!(millimetres(1_270_000), "1.270000");
        assert_eq!(hundredths(7_500), "75.00");
        assert_eq!(hundredths(5), "0.05");
    }

    #[test]
    fn the_findings_are_listed_errors_first_and_counted() {
        let list = vec![
            Finding::new(NO_NET, Severity::Info, Some("TP1:1"), ""),
            Finding::new(NEAR_EDGE, Severity::Error, Some("TP2:1"), "0.2 mm"),
        ];
        assert_eq!(counts(&list), (1, 0, 1));
        let text = findings_text(&list, Lang::De);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("error: PHOENIX-TP-010"),
            "{}",
            lines[0]
        );
        assert!(lines[1].starts_with("info: PHOENIX-TP-011"), "{}", lines[1]);
        assert_eq!(findings_text(&[], Lang::En), "");
    }
}
