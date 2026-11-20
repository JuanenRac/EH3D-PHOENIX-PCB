// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of the server mode as text, with its words in the eight languages, and as JSON, the
//! same every time. A dry run reports the request that would have been sent; a run that sent it
//! reports what the server said.

use eh3d_phoenix_rules::Lang;
use eh3d_serverclient::json::quote;

use crate::codes::{Note, Severity};
use crate::session::{Outcome, Probe};

/// The format name of the JSON report.
pub const FORMAT: &str = "phoenix-server/1";

/// What a run of the command made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerReport {
    /// The kind of report, as the station names it (`rules_check`, `drill_report`...).
    pub label: String,
    /// Whether the request was sent.
    pub sent: bool,
    /// The text of the request.
    pub request: String,
    /// How many items (tracks, holes, parts...) the request holds.
    pub items: usize,
    /// What was left out, approximated or worth a warning.
    pub notes: Vec<Note>,
    /// What the server said; none for a dry run.
    pub outcome: Option<Outcome>,
}

impl ServerReport {
    /// True when the server said the job ended well: its verdict is `pass`.
    pub fn passes(&self) -> bool {
        self.outcome.as_ref().is_some_and(Outcome::passes)
    }

    /// True when the command should exit with 1: the server's verdict is not a pass.
    pub fn has_errors(&self) -> bool {
        self.outcome.as_ref().is_some_and(|o| !o.passes())
    }
}

/// The words of the text report, by key, in the eight languages in the order of the tables.
const WORDS: &[(&str, [&str; 8])] = &[
    (
        "title",
        [
            "Server report",
            "Informe del servidor",
            "Serverbericht",
            "Rapport du serveur",
            "Rapporto del server",
            "サーバーレポート",
            "服务器报告",
            "Relatório do servidor",
        ],
    ),
    (
        "dry_run",
        [
            "dry run, nothing was sent",
            "simulacro, no se envió nada",
            "Probelauf, nichts wurde gesendet",
            "essai à blanc, rien n'a été envoyé",
            "prova a vuoto, non è stato inviato nulla",
            "ドライラン、何も送信していません",
            "试运行，未发送任何内容",
            "simulacro, nada foi enviado",
        ],
    ),
    (
        "sent",
        [
            "sent to the server",
            "enviado al servidor",
            "an den Server gesendet",
            "envoyé au serveur",
            "inviato al server",
            "サーバーへ送信済み",
            "已发送到服务器",
            "enviado para o servidor",
        ],
    ),
    (
        "items",
        [
            "Items",
            "Elementos",
            "Elemente",
            "Éléments",
            "Elementi",
            "項目",
            "项目",
            "Itens",
        ],
    ),
    (
        "job",
        [
            "Job",
            "Trabajo",
            "Auftrag",
            "Travail",
            "Lavoro",
            "ジョブ",
            "作业",
            "Trabalho",
        ],
    ),
    (
        "state",
        [
            "State",
            "Estado",
            "Zustand",
            "État",
            "Stato",
            "状態",
            "状态",
            "Situação",
        ],
    ),
    (
        "verdict",
        [
            "Verdict",
            "Veredicto",
            "Ergebnis",
            "Conclusion",
            "Esito",
            "判定",
            "结论",
            "Parecer",
        ],
    ),
    (
        "pass",
        [
            "pass",
            "correcto",
            "bestanden",
            "réussi",
            "superato",
            "合格",
            "通过",
            "aprovado",
        ],
    ),
    (
        "fail",
        [
            "fail",
            "fallo",
            "durchgefallen",
            "échec",
            "fallito",
            "不合格",
            "未通过",
            "reprovado",
        ],
    ),
    (
        "errors",
        [
            "Errors",
            "Errores",
            "Fehler",
            "Erreurs",
            "Errori",
            "エラー",
            "错误",
            "Erros",
        ],
    ),
    (
        "warnings",
        [
            "Warnings",
            "Avisos",
            "Warnungen",
            "Avertissements",
            "Avvisi",
            "警告",
            "告警",
            "Alertas",
        ],
    ),
    (
        "problems",
        [
            "Problems",
            "Problemas",
            "Probleme",
            "Problèmes",
            "Problemi",
            "問題",
            "问题",
            "Questões",
        ],
    ),
    (
        "more",
        [
            "more not listed",
            "más sin listar",
            "weitere nicht aufgeführt",
            "autres non listés",
            "altri non elencati",
            "他は省略",
            "其余未列出",
            "outros não listados",
        ],
    ),
    (
        "request",
        [
            "Request",
            "Petición",
            "Anfrage",
            "Requête",
            "Richiesta",
            "リクエスト",
            "请求",
            "Pedido",
        ],
    ),
    (
        "health",
        [
            "Server health",
            "Estado del servidor",
            "Zustand des Servers",
            "Santé du serveur",
            "Stato del server",
            "サーバーの状態",
            "服务器状态",
            "Saúde do servidor",
        ],
    ),
    (
        "notes",
        [
            "Notes",
            "Notas",
            "Hinweise",
            "Remarques",
            "Note",
            "注記",
            "备注",
            "Observações",
        ],
    ),
];

/// A word of the report in a language; the key itself for one that is not in the table.
pub fn word(key: &'static str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(key, |(_, row)| row[column])
}

/// The report as text in a language. At most `max_listed` problems are listed.
pub fn report_text(report: &ServerReport, max_listed: usize, lang: Lang) -> String {
    let mut out = format!(
        "{}: {} ({})\n",
        word("title", lang),
        report.label,
        if report.sent {
            word("sent", lang)
        } else {
            word("dry_run", lang)
        }
    );
    out.push_str(&format!("{}: {}\n", word("items", lang), report.items));
    if let Some(o) = &report.outcome {
        out.push_str(&format!(
            "{}: {} ({}: {})\n",
            word("job", lang),
            o.job_id,
            word("state", lang),
            o.state
        ));
        if !o.message.is_empty() {
            out.push_str(&format!("  {}\n", o.message));
        }
        if !o.verdict.is_empty() {
            let verdict = match o.verdict.as_str() {
                "pass" => word("pass", lang),
                "fail" => word("fail", lang),
                other => other,
            };
            out.push_str(&format!(
                "{}: {}   {}: {}   {}: {}\n",
                word("verdict", lang),
                verdict,
                word("errors", lang),
                o.errors,
                word("warnings", lang),
                o.warnings
            ));
        }
        for (name, text) in &o.figures {
            out.push_str(&format!("  {name}: {text}\n"));
        }
        if !o.problems.is_empty() {
            out.push_str(&format!("{}:\n", word("problems", lang)));
        }
        for p in o.problems.iter().take(max_listed) {
            out.push_str(&format!(
                "  {}: {} [{}]: {}\n",
                p.severity, p.rule, p.item, p.message
            ));
        }
        if o.problems.len() > max_listed {
            out.push_str(&format!(
                "  ... {} {}\n",
                o.problems.len() - max_listed,
                word("more", lang)
            ));
        }
    } else {
        out.push_str(&format!("{}:\n{}\n", word("request", lang), report.request));
    }
    if !report.notes.is_empty() {
        out.push_str(&format!("{}:\n", word("notes", lang)));
        for note in &report.notes {
            out.push_str(&format!("  {}\n", note.line(lang)));
        }
    }
    out
}

/// What the server said of itself, as text in a language.
pub fn probe_text(probe: &Probe, lang: Lang) -> String {
    format!(
        "{}: {} {}\n{}: {}\n  api_revision: {}\n",
        word("health", lang),
        probe.name,
        probe.version,
        word("state", lang),
        probe.status,
        probe.api_revision
    )
}

/// What the server said of itself, as JSON of the format `phoenix-server/1`, the same every time.
pub fn probe_json(probe: &Probe) -> String {
    format!(
        "{{\"format\":{},\"health\":{{\"name\":{},\"version\":{},\"status\":{},\"api_revision\":{}}}}}\n",
        quote(FORMAT),
        quote(&probe.name),
        quote(&probe.version),
        quote(&probe.status),
        probe.api_revision
    )
}

fn note_json(note: &Note) -> String {
    format!(
        "{{\"code\":{},\"severity\":{},\"detail\":{}}}",
        quote(note.code),
        quote(note.severity.as_str()),
        quote(&note.detail)
    )
}

/// The report as JSON, the same every time. A dry run holds the request; a run that sent it holds
/// the job, the verdict and the problems (at most `max_listed`).
pub fn report_json(report: &ServerReport, max_listed: usize) -> String {
    let notes: Vec<String> = report.notes.iter().map(note_json).collect();
    let mut out = format!(
        "{{\"format\":{},\"kind\":{},\"sent\":{},\"items\":{},\"notes\":[{}]",
        quote(FORMAT),
        quote(&report.label),
        report.sent,
        report.items,
        notes.join(",")
    );
    match &report.outcome {
        None => out.push_str(&format!(",\"request\":{}", report.request)),
        Some(o) => {
            let problems: Vec<String> = o
                .problems
                .iter()
                .take(max_listed)
                .map(|p| {
                    format!(
                        "{{\"severity\":{},\"rule\":{},\"item\":{},\"message\":{}}}",
                        quote(&p.severity),
                        quote(&p.rule),
                        quote(&p.item),
                        quote(&p.message)
                    )
                })
                .collect();
            let figures: Vec<String> = o
                .figures
                .iter()
                .map(|(k, v)| format!("{}:{}", quote(k), quote(v)))
                .collect();
            out.push_str(&format!(
                ",\"job\":{{\"id\":{},\"state\":{},\"message\":{}}},\"verdict\":{},\"errors\":{},\"warnings\":{},\"figures\":{{{}}},\"problems\":[{}],\"problems_cut\":{}",
                quote(&o.job_id),
                quote(&o.state),
                quote(&o.message),
                quote(&o.verdict),
                o.errors,
                o.warnings,
                figures.join(","),
                problems.join(","),
                o.problems.len() > max_listed
            ));
        }
    }
    out.push_str("}\n");
    out
}

/// True when any note of the report is a warning or an error.
pub fn has_serious_note(report: &ServerReport) -> bool {
    report.notes.iter().any(|n| n.severity >= Severity::Warning)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{DRY_RUN, PARTIAL_REQUEST};
    use crate::session::Problem;
    use eh3d_serverclient::Json;

    fn outcome() -> Outcome {
        Outcome {
            job_id: "w-7".into(),
            state: "done".into(),
            message: "".into(),
            verdict: "fail".into(),
            errors: 2,
            warnings: 1,
            problems: vec![
                Problem {
                    severity: "error".into(),
                    rule: "track_width".into(),
                    item: "GND top".into(),
                    message: "GND top is 0.1 mm wide".into(),
                },
                Problem {
                    severity: "error".into(),
                    rule: "drill_small".into(),
                    item: "via \"A\"".into(),
                    message: "too small".into(),
                },
                Problem {
                    severity: "warning".into(),
                    rule: "drill_large".into(),
                    item: "J1.1".into(),
                    message: "7 mm".into(),
                },
            ],
            figures: vec![("hits".into(), "12".into())],
            polls: 3,
            kind: "rules_check".into(),
        }
    }

    fn report(sent: bool) -> ServerReport {
        ServerReport {
            label: "rules_check".into(),
            sent,
            request: "{\"kind\":\"rules_check\"}".into(),
            items: 5,
            notes: vec![Note::new(PARTIAL_REQUEST, Severity::Info, "arcs by chord")],
            outcome: sent.then(outcome),
        }
    }

    #[test]
    fn every_word_has_a_text_in_each_language_that_is_not_the_english_one() {
        for (key, row) in WORDS {
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{key} column {column}");
                if column > 0 {
                    assert_ne!(*text, row[0], "{key} column {column}");
                }
            }
        }
        assert_eq!(word("verdict", Lang::Fr), "Conclusion");
        assert_eq!(word("pass", Lang::Pt), "aprovado");
        assert_eq!(word("unknown", Lang::En), "unknown");
    }

    #[test]
    fn a_dry_run_shows_the_request_and_says_nothing_was_sent() {
        let text = report_text(&report(false), 10, Lang::En);
        assert!(text.starts_with("Server report: rules_check (dry run, nothing was sent)\n"));
        assert!(text.contains("Items: 5"));
        assert!(text.contains("Request:\n{\"kind\":\"rules_check\"}"));
        assert!(text.contains("info: PHOENIX-SERVER-011"));
        assert!(!report(false).has_errors() && !report(false).passes());
    }

    #[test]
    fn a_run_that_sent_the_request_shows_the_job_the_verdict_and_the_problems() {
        let r = report(true);
        let text = report_text(&r, 10, Lang::En);
        assert!(text.contains("Job: w-7 (State: done)"));
        assert!(text.contains("Verdict: fail   Errors: 2   Warnings: 1"));
        assert!(text.contains("  hits: 12"));
        assert!(text.contains("error: track_width [GND top]: GND top is 0.1 mm wide"));
        assert!(r.has_errors() && !r.passes());
        assert!(!text.contains("more not listed"));
    }

    #[test]
    fn the_list_of_problems_is_cut_at_the_limit_and_says_how_many_are_left() {
        let text = report_text(&report(true), 1, Lang::En);
        assert!(text.contains("track_width"));
        assert!(!text.contains("drill_small"));
        assert!(text.contains("... 2 more not listed"));
        let json = report_json(&report(true), 1);
        let j = Json::parse(&json).unwrap();
        assert_eq!(j.get("problems").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(j.get("problems_cut").and_then(Json::as_bool), Some(true));
    }

    #[test]
    fn the_text_is_in_the_language_asked_for_in_all_eight() {
        let r = report(true);
        let mut seen = Vec::new();
        for lang in Lang::ALL {
            let text = report_text(&r, 10, lang);
            assert!(text.contains(word("title", lang)));
            assert!(text.contains(word("fail", lang)));
            assert!(text.contains(crate::codes::title(PARTIAL_REQUEST, lang)));
            seen.push(text);
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn the_json_of_a_dry_run_holds_the_request_as_it_is() {
        let json = report_json(&report(false), 10);
        let j = Json::parse(&json).unwrap();
        assert_eq!(j.get("format").and_then(Json::as_str), Some(FORMAT));
        assert_eq!(j.get("sent").and_then(Json::as_bool), Some(false));
        assert_eq!(
            j.get("request").unwrap().get("kind").and_then(Json::as_str),
            Some("rules_check")
        );
        assert!(j.get("job").is_none());
        let notes = j.get("notes").unwrap().as_array().unwrap();
        assert_eq!(
            notes[0].get("code").and_then(Json::as_str),
            Some(PARTIAL_REQUEST)
        );
    }

    #[test]
    fn the_json_of_a_run_holds_the_job_the_verdict_and_escapes_the_names() {
        let json = report_json(&report(true), 10);
        let j = Json::parse(&json).unwrap();
        assert_eq!(
            j.get("job").unwrap().get("id").and_then(Json::as_str),
            Some("w-7")
        );
        assert_eq!(j.get("verdict").and_then(Json::as_str), Some("fail"));
        assert_eq!(j.get("errors").and_then(Json::as_u64), Some(2));
        let problems = j.get("problems").unwrap().as_array().unwrap();
        assert_eq!(problems.len(), 3);
        assert_eq!(
            problems[1].get("item").and_then(Json::as_str),
            Some("via \"A\"")
        );
        assert_eq!(j.get("problems_cut").and_then(Json::as_bool), Some(false));
        assert_eq!(
            j.get("figures").unwrap().get("hits").and_then(Json::as_str),
            Some("12")
        );
        // the same report gives the same bytes
        assert_eq!(json, report_json(&report(true), 10));
    }

    fn probe() -> Probe {
        Probe {
            name: "eh3d-server".into(),
            version: "0.9.1".into(),
            status: "ok".into(),
            api_revision: 1,
        }
    }

    #[test]
    fn the_health_of_the_server_is_told_in_each_language_and_as_json() {
        let en = probe_text(&probe(), Lang::En);
        assert_eq!(
            en,
            "Server health: eh3d-server 0.9.1\nState: ok\n  api_revision: 1\n"
        );
        let mut seen: Vec<String> = Lang::ALL.iter().map(|l| probe_text(&probe(), *l)).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 8);
        let j = Json::parse(&probe_json(&probe())).unwrap();
        assert_eq!(j.get("format").and_then(Json::as_str), Some(FORMAT));
        let h = j.get("health").unwrap();
        assert_eq!(h.get("version").and_then(Json::as_str), Some("0.9.1"));
        assert_eq!(h.get("api_revision").and_then(Json::as_u64), Some(1));
    }

    #[test]
    fn a_passing_report_exits_clean_and_a_serious_note_is_found() {
        let mut r = report(true);
        if let Some(o) = r.outcome.as_mut() {
            o.verdict = "pass".into();
            o.errors = 0;
            o.warnings = 0;
        }
        assert!(r.passes() && !r.has_errors());
        assert!(!has_serious_note(&r));
        r.notes.push(Note::new(DRY_RUN, Severity::Warning, ""));
        assert!(has_serious_note(&r));
    }
}
