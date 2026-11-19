// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of one board against the capability profiles of several board makers: for each
//! profile the number of proven errors, warnings and information, how many things could not be
//! judged and whether the board passes; for each code of a finding how many times it is found with
//! each profile; the codes that only some of the profiles find; and the profiles with the fewest
//! errors. It reads reports that were made already (`check_board` with the limits of a profile,
//! merged with `dfm_findings`), so it judges nothing of its own.
//!
//! The figures of a profile are what its file says. A profile that is not verified against the
//! maker (`verified = false`, which is how the profiles that come with the program are) gives a
//! comparison of those figures and not of what the makers really build; the report says so for each
//! profile. The comparison is no recommendation of a maker.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::codes::{title, Lang};
use crate::report::{Report, Severity};

/// The check of one board against one profile.
#[derive(Debug, Clone)]
pub struct ProfileRun {
    /// The name the person gave the profile on the command line.
    pub label: String,
    /// Whether the profile says that its figures were checked against the maker.
    pub verified: bool,
    /// The proven and the inconclusive findings of the check with that profile.
    pub report: Report,
}

/// The numbers of one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRow {
    /// The name of the profile.
    pub label: String,
    /// Whether its figures are verified.
    pub verified: bool,
    /// Whether no proven finding is an error.
    pub passes: bool,
    /// Proven errors.
    pub errors: usize,
    /// Proven warnings.
    pub warnings: usize,
    /// Proven information.
    pub information: usize,
    /// What could not be judged.
    pub inconclusive: usize,
}

/// How often a code is found with each profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRow {
    /// The code.
    pub code: &'static str,
    /// The number of proven findings with it, for each profile in the order of the profiles.
    pub counts: Vec<usize>,
}

/// The comparison of the runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfmComparison {
    /// One row for each profile, in the order they were given.
    pub profiles: Vec<ProfileRow>,
    /// One row for each code that is found with any profile, in the order of the codes.
    pub codes: Vec<CodeRow>,
}

/// Compares the runs. The order of the runs is kept; the codes are in alphabetical order.
pub fn compare_profiles(runs: &[ProfileRun]) -> DfmComparison {
    let profiles: Vec<ProfileRow> = runs
        .iter()
        .map(|run| ProfileRow {
            label: run.label.clone(),
            verified: run.verified,
            passes: run.report.passes(),
            errors: run.report.count(Severity::Error),
            warnings: run.report.count(Severity::Warning),
            information: run.report.count(Severity::Info),
            inconclusive: run.report.inconclusive.len(),
        })
        .collect();
    let mut by_code: BTreeMap<&'static str, Vec<usize>> = BTreeMap::new();
    for (index, run) in runs.iter().enumerate() {
        for finding in &run.report.confirmed {
            by_code
                .entry(finding.code)
                .or_insert_with(|| vec![0; runs.len()])[index] += 1;
        }
    }
    let codes = by_code
        .into_iter()
        .map(|(code, counts)| CodeRow { code, counts })
        .collect();
    DfmComparison { profiles, codes }
}

impl DfmComparison {
    /// True when at least one profile accepts the board.
    pub fn any_passes(&self) -> bool {
        self.profiles.iter().any(|p| p.passes)
    }

    /// The labels of the profiles with the fewest proven errors, in the order given.
    pub fn fewest_errors(&self) -> Vec<&str> {
        let least = self.profiles.iter().map(|p| p.errors).min();
        self.profiles
            .iter()
            .filter(|p| Some(p.errors) == least)
            .map(|p| p.label.as_str())
            .collect()
    }

    /// The codes that some profiles find and others do not, each with the labels of the profiles
    /// that find it.
    pub fn only_on_some(&self) -> Vec<(&'static str, Vec<&str>)> {
        self.codes
            .iter()
            .filter(|row| row.counts.contains(&0) && row.counts.iter().any(|c| *c > 0))
            .map(|row| {
                let with: Vec<&str> = row
                    .counts
                    .iter()
                    .zip(&self.profiles)
                    .filter(|(count, _)| **count > 0)
                    .map(|(_, p)| p.label.as_str())
                    .collect();
                (row.code, with)
            })
            .collect()
    }

    /// The comparison as text in a language: a line for each profile, a line for each code with
    /// its count for each profile, the codes only some profiles find and the profiles with the
    /// fewest errors.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{}: {}", word("profiles", lang), self.profiles.len());
        for p in &self.profiles {
            let _ = writeln!(
                out,
                "- {} ({}: {}): {}: {} | {}: {} | {}: {} | {}: {} | {}: {}",
                p.label,
                word("verified", lang),
                yes_no(p.verified, lang),
                word("passes", lang),
                yes_no(p.passes, lang),
                word("errors", lang),
                p.errors,
                word("warnings", lang),
                p.warnings,
                word("information", lang),
                p.information,
                word("not_judged", lang),
                p.inconclusive
            );
        }
        if self.codes.is_empty() {
            let _ = writeln!(out, "{}", word("none", lang));
        } else {
            let _ = writeln!(out, "{}:", word("by_code", lang));
            for row in &self.codes {
                let counts: Vec<String> = row
                    .counts
                    .iter()
                    .zip(&self.profiles)
                    .map(|(count, p)| format!("{} {count}", p.label))
                    .collect();
                let _ = writeln!(
                    out,
                    "{} {}: {}",
                    row.code,
                    title(row.code, lang).unwrap_or(""),
                    counts.join(" | ")
                );
            }
        }
        let some = self.only_on_some();
        if !some.is_empty() {
            let _ = writeln!(out, "{}:", word("only_some", lang));
            for (code, labels) in some {
                let _ = writeln!(
                    out,
                    "{} {}: {}",
                    code,
                    title(code, lang).unwrap_or(""),
                    labels.join(", ")
                );
            }
        }
        let _ = writeln!(
            out,
            "{}: {}",
            word("fewest", lang),
            self.fewest_errors().join(", ")
        );
        out
    }

    /// The comparison as JSON (`phoenix-dfm-compare/1`): the profiles with their numbers, the
    /// codes with their counts by profile, the codes only some profiles find and the profiles
    /// with the fewest errors. The same text for the same runs.
    pub fn to_json(&self) -> String {
        let profiles: Vec<Value> = self
            .profiles
            .iter()
            .map(|p| {
                json!({
                    "name": p.label,
                    "verified": p.verified,
                    "passes": p.passes,
                    "errors": p.errors,
                    "warnings": p.warnings,
                    "information": p.information,
                    "inconclusive": p.inconclusive,
                })
            })
            .collect();
        let codes: Vec<Value> = self
            .codes
            .iter()
            .map(|row| {
                let counts: serde_json::Map<String, Value> = row
                    .counts
                    .iter()
                    .zip(&self.profiles)
                    .map(|(count, p)| (p.label.clone(), json!(count)))
                    .collect();
                json!({ "code": row.code, "counts": counts })
            })
            .collect();
        let only: Vec<Value> = self
            .only_on_some()
            .into_iter()
            .map(|(code, labels)| json!({ "code": code, "profiles": labels }))
            .collect();
        let value = json!({
            "schema": "phoenix-dfm-compare/1",
            "any_passes": self.any_passes(),
            "profiles": profiles,
            "codes": codes,
            "only_on_some": only,
            "fewest_errors": self.fewest_errors(),
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn yes_no(value: bool, lang: Lang) -> &'static str {
    word(if value { "yes" } else { "no" }, lang)
}

/// The words of the comparison in the eight languages: English, Spanish, German, French, Italian,
/// Japanese, Chinese and European Portuguese.
const WORDS: &[(&str, [&str; 8])] = &[
    (
        "profiles",
        [
            "Profiles compared",
            "Perfiles comparados",
            "Verglichene Profile",
            "Profils comparés",
            "Profili confrontati",
            "比較したプロファイル",
            "比较的配置文件",
            "Perfis comparados",
        ],
    ),
    (
        "verified",
        [
            "verified",
            "verificado",
            "geprüft",
            "vérifié",
            "verificato",
            "検証済み",
            "已核实",
            "verificado",
        ],
    ),
    ("yes", ["yes", "sí", "ja", "oui", "sì", "はい", "是", "sim"]),
    (
        "no",
        ["no", "no", "nein", "non", "no", "いいえ", "否", "não"],
    ),
    (
        "passes",
        [
            "passes",
            "cumple",
            "bestanden",
            "réussi",
            "superato",
            "合格",
            "通过",
            "cumpre",
        ],
    ),
    (
        "errors",
        [
            "errors",
            "errores",
            "Fehler",
            "erreurs",
            "errori",
            "エラー",
            "错误",
            "erros",
        ],
    ),
    (
        "warnings",
        [
            "warnings",
            "avisos",
            "Warnungen",
            "avertissements",
            "avvisi",
            "警告",
            "警告信息",
            "avisos",
        ],
    ),
    (
        "information",
        [
            "information",
            "información",
            "Informationen",
            "informations",
            "informazioni",
            "情報",
            "提示",
            "informações",
        ],
    ),
    (
        "not_judged",
        [
            "not judged",
            "sin juzgar",
            "nicht beurteilt",
            "non jugés",
            "non giudicati",
            "判定不能",
            "未判定",
            "não avaliados",
        ],
    ),
    (
        "by_code",
        [
            "Proven findings by code",
            "Hallazgos probados por código",
            "Nachgewiesene Befunde je Code",
            "Constats prouvés par code",
            "Rilevamenti accertati per codice",
            "コード別の確定した指摘",
            "按代码列出的已证实发现",
            "Constatações comprovadas por código",
        ],
    ),
    (
        "only_some",
        [
            "Only on some of the profiles",
            "Solo en algunos perfiles",
            "Nur bei einigen Profilen",
            "Seulement sur certains profils",
            "Solo in alcuni profili",
            "一部のプロファイルのみ",
            "仅在部分配置文件中",
            "Apenas em alguns perfis",
        ],
    ),
    (
        "fewest",
        [
            "Fewest errors",
            "Menos errores",
            "Wenigste Fehler",
            "Moins d'erreurs",
            "Meno errori",
            "エラーが最少",
            "错误最少",
            "Menos erros",
        ],
    ),
    (
        "none",
        [
            "No proven finding with any profile",
            "Ningún hallazgo probado con ningún perfil",
            "Kein nachgewiesener Befund bei einem Profil",
            "Aucun constat prouvé avec aucun profil",
            "Nessun rilevamento accertato con alcun profilo",
            "どのプロファイルでも確定した指摘はありません",
            "任何配置文件均无已证实的发现",
            "Nenhuma constatação comprovada com nenhum perfil",
        ],
    ),
];

/// A word of the comparison in a language; an unknown key gives an empty text.
fn word(key: &str, lang: Lang) -> &'static str {
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, texts)| texts[lang.column()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{CLEARANCE, DRILL_SMALL, TRACK_NARROW};
    use crate::report::Finding;

    fn finding(code: &'static str, severity: Severity, place: &str) -> Finding {
        Finding::new(1, "x", code, severity, place.to_owned())
    }

    fn run(label: &str, verified: bool, findings: Vec<Finding>) -> ProfileRun {
        let mut report = Report::new("check");
        report.confirmed = findings;
        report.finish();
        ProfileRun {
            label: label.to_owned(),
            verified,
            report,
        }
    }

    fn sample() -> Vec<ProfileRun> {
        vec![
            run(
                "alpha",
                false,
                vec![
                    finding(TRACK_NARROW, Severity::Error, "a"),
                    finding(TRACK_NARROW, Severity::Error, "b"),
                    finding(CLEARANCE, Severity::Warning, "c"),
                ],
            ),
            run(
                "beta",
                true,
                vec![
                    finding(TRACK_NARROW, Severity::Error, "a"),
                    finding(DRILL_SMALL, Severity::Info, "d"),
                ],
            ),
            run("gamma", false, Vec::new()),
        ]
    }

    #[test]
    fn each_profile_has_its_counts_and_whether_it_passes() {
        let c = compare_profiles(&sample());
        let rows: Vec<(&str, bool, bool, usize, usize, usize)> = c
            .profiles
            .iter()
            .map(|p| {
                (
                    p.label.as_str(),
                    p.verified,
                    p.passes,
                    p.errors,
                    p.warnings,
                    p.information,
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("alpha", false, false, 2, 1, 0),
                ("beta", true, false, 1, 0, 1),
                ("gamma", false, true, 0, 0, 0),
            ]
        );
        assert!(c.any_passes());
        assert_eq!(c.fewest_errors(), ["gamma"]);
    }

    #[test]
    fn the_codes_are_counted_for_each_profile_and_listed_in_order() {
        let c = compare_profiles(&sample());
        let rows: Vec<(&str, Vec<usize>)> =
            c.codes.iter().map(|r| (r.code, r.counts.clone())).collect();
        // PHOENIX-CHECK-001, then -003, then -005
        assert_eq!(
            rows,
            vec![
                (TRACK_NARROW, vec![2, 1, 0]),
                (DRILL_SMALL, vec![0, 1, 0]),
                (CLEARANCE, vec![1, 0, 0]),
            ]
        );
    }

    #[test]
    fn a_code_is_only_on_some_when_a_profile_finds_it_and_another_does_not() {
        let c = compare_profiles(&sample());
        let some = c.only_on_some();
        // every code of the sample is missing from the third profile
        assert_eq!(some.len(), 3);
        let narrow = some.iter().find(|(code, _)| *code == TRACK_NARROW).unwrap();
        assert_eq!(narrow.1, ["alpha", "beta"]);
        // a code that every profile finds is not in the list
        let all = compare_profiles(&[
            run("a", false, vec![finding(CLEARANCE, Severity::Error, "x")]),
            run("b", false, vec![finding(CLEARANCE, Severity::Error, "y")]),
        ]);
        assert!(all.only_on_some().is_empty());
        assert!(!all.any_passes());
        assert_eq!(all.fewest_errors(), ["a", "b"]);
    }

    #[test]
    fn the_text_names_the_profiles_the_codes_and_the_fewest_errors_in_the_language() {
        let c = compare_profiles(&sample());
        let en = c.to_text(Lang::En);
        assert!(en.starts_with("Profiles compared: 3\n"), "{en}");
        assert!(
            en.contains("- alpha (verified: no): passes: no | errors: 2 | warnings: 1 | information: 0 | not judged: 0\n"),
            "{en}"
        );
        assert!(en.contains("- beta (verified: yes)"), "{en}");
        assert!(
            en.contains(
                "PHOENIX-CHECK-001 Track narrower than the minimum: alpha 2 | beta 1 | gamma 0"
            ),
            "{en}"
        );
        assert!(en.contains("Only on some of the profiles:\n"), "{en}");
        assert!(en.ends_with("Fewest errors: gamma\n"), "{en}");
        let pt = c.to_text(Lang::Pt);
        assert!(pt.contains("Perfis comparados: 3"), "{pt}");
        assert!(pt.contains("Menos erros: gamma"), "{pt}");
        let ja = c.to_text(Lang::Ja);
        assert!(ja.contains("比較したプロファイル"), "{ja}");
        // no finding at all
        let none = compare_profiles(&[run("a", false, Vec::new())]).to_text(Lang::En);
        assert!(
            none.contains("No proven finding with any profile\n"),
            "{none}"
        );
    }

    #[test]
    fn the_json_is_the_same_every_time_and_has_the_counts_by_profile() {
        let c = compare_profiles(&sample());
        assert_eq!(c.to_json(), compare_profiles(&sample()).to_json());
        let v: Value = serde_json::from_str(&c.to_json()).unwrap();
        assert_eq!(v["schema"], "phoenix-dfm-compare/1");
        assert_eq!(v["any_passes"], true);
        assert_eq!(v["profiles"][0]["errors"], 2);
        assert_eq!(v["profiles"][1]["verified"], true);
        let narrow = v["codes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["code"] == TRACK_NARROW)
            .unwrap();
        assert_eq!(narrow["counts"]["alpha"], 2);
        assert_eq!(narrow["counts"]["gamma"], 0);
        assert_eq!(v["fewest_errors"][0], "gamma");
        assert_eq!(v["only_on_some"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn every_word_is_in_the_eight_languages_and_the_other_languages_differ_from_english() {
        for (key, texts) in WORDS {
            for (i, text) in texts.iter().enumerate() {
                assert!(!text.is_empty(), "{key} column {i}");
            }
            for lang in Lang::ALL {
                assert_eq!(word(key, lang), texts[lang.column()], "{key}");
            }
            if !matches!(
                *key,
                "no" | "warnings" | "information" | "errors" | "verified"
            ) {
                assert_ne!(texts[1], texts[0], "{key} es");
                assert_ne!(texts[7], texts[0], "{key} pt");
            }
            assert!(!texts[5].is_ascii(), "{key} ja is in Japanese");
            assert!(!texts[6].is_ascii(), "{key} zh is in Chinese");
        }
        assert_eq!(word("nothing like it", Lang::En), "");
    }

    #[test]
    fn the_order_of_the_runs_is_kept_whatever_the_names() {
        let c = compare_profiles(&[
            run("zeta", false, Vec::new()),
            run("alpha", false, Vec::new()),
        ]);
        let labels: Vec<&str> = c.profiles.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["zeta", "alpha"]);
        assert_eq!(c.fewest_errors(), ["zeta", "alpha"]);
    }
}
