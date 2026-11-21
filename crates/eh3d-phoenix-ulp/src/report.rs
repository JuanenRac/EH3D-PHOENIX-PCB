// SPDX-License-Identifier: GPL-3.0-or-later
//! The compatibility report of a program: what it uses, what the list of allowed names says about
//! each use, the problems the reader found, and a verdict (`covered`, `partly covered` or `not
//! covered`). It reads the program and runs nothing; the evaluator is not written, so the verdict
//! says what the list would let through, not what runs today.
//!
//! The report is the same for the same program and the same list: the uses are ordered by kind and
//! name, the notes by place, and the JSON has its keys in a fixed order.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use eh3d_phoenix_rules::Lang;

use crate::codes::{title, Diag, ITEM_DENIED, ITEM_LIMITED, ITEM_PENDING, LIST_CUT, NAME_UNKNOWN};
use crate::parser::parse;
use crate::settings::UlpSettings;
use crate::support::{Status, Support, UseKind};
use crate::uses::collect;

/// The name of the format of the JSON of a report.
pub const FORMAT: &str = "phoenix-ulp-report/1";

/// What the report says about a program as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every name it uses is allowed, or allowed with limits.
    Covered,
    /// Some name is not decided yet or is not on the list, and none is refused.
    Partly,
    /// A name is refused by the sandbox, or the program cannot be read.
    NotCovered,
}

impl Verdict {
    /// The word of the JSON.
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Covered => "covered",
            Verdict::Partly => "partly_covered",
            Verdict::NotCovered => "not_covered",
        }
    }
}

/// One name the program uses, with what the list says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Used {
    /// What kind of name.
    pub kind: UseKind,
    /// The name.
    pub name: String,
    /// How many times it appears.
    pub count: usize,
    /// The line of its first appearance.
    pub line: usize,
    /// The column of its first appearance.
    pub col: usize,
    /// What the list says; `None` when the list does not know the name.
    pub status: Option<Status>,
}

/// The report of one program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The name the program is shown by (a file name).
    pub name: String,
    /// The size of the program, in bytes.
    pub bytes: usize,
    /// The SHA-256 of the program, in hexadecimal.
    pub sha256: String,
    /// The verdict.
    pub verdict: Verdict,
    /// The statements of the program, the ones inside blocks included.
    pub statements: usize,
    /// The functions the program defines.
    pub functions: usize,
    /// What the program uses.
    pub uses: Vec<Used>,
    /// The problems the reader found, in the order of the program.
    pub problems: Vec<Diag>,
    /// How many more problems there were than are listed.
    pub problems_dropped: usize,
    /// The limit that stopped the reading, when one did.
    pub stopped: Option<Diag>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Reads a program and reports on it with a list of allowed names.
pub fn analyse(name: &str, source: &str, settings: &UlpSettings, support: &Support) -> Report {
    let mut report = Report {
        name: name.to_owned(),
        bytes: source.len(),
        sha256: sha256_hex(source.as_bytes()),
        verdict: Verdict::Covered,
        statements: 0,
        functions: 0,
        uses: Vec::new(),
        problems: Vec::new(),
        problems_dropped: 0,
        stopped: None,
    };
    match parse(source, settings) {
        Ok(parsed) => {
            report.statements = parsed.program.statement_count();
            report.functions = parsed.program.functions.len();
            report.uses = collect(&parsed.program)
                .into_iter()
                .map(|s| Used {
                    status: support.lookup(s.kind, &s.name),
                    kind: s.kind,
                    name: s.name,
                    count: s.count,
                    line: s.first.line,
                    col: s.first.col,
                })
                .collect();
            report.problems_dropped = parsed.problems.dropped;
            report.problems = parsed.problems.list;
        }
        Err(stopped) => report.stopped = Some(stopped),
    }
    let refused = report.uses.iter().any(|u| u.status == Some(Status::Denied));
    report.verdict = if report.stopped.is_some() || !report.problems.is_empty() || refused {
        Verdict::NotCovered
    } else if report
        .uses
        .iter()
        .any(|u| matches!(u.status, None | Some(Status::Pending)))
    {
        Verdict::Partly
    } else {
        Verdict::Covered
    };
    report
}

impl Report {
    /// The notes about the names: one for each name that is refused, pending, not on the list or
    /// allowed with limits, in the order of the program.
    pub fn notes(&self) -> Vec<Diag> {
        let mut notes: Vec<Diag> = self
            .uses
            .iter()
            .filter_map(|u| {
                let code = match u.status {
                    Some(Status::Allowed) => return None,
                    Some(Status::Limited) => ITEM_LIMITED,
                    Some(Status::Pending) => ITEM_PENDING,
                    Some(Status::Denied) => ITEM_DENIED,
                    None => NAME_UNKNOWN,
                };
                Some(Diag::at(
                    code,
                    u.line,
                    u.col,
                    format!("{} {}", u.kind.word(), u.name),
                ))
            })
            .collect();
        notes.sort_by(|a, b| (a.line, a.col, a.code).cmp(&(b.line, b.col, b.code)));
        notes
    }

    /// How many names there are with each status, in the order allowed, limited, pending, denied
    /// and not on the list.
    pub fn tally(&self) -> [usize; 5] {
        let mut t = [0; 5];
        for u in &self.uses {
            let slot = match u.status {
                Some(Status::Allowed) => 0,
                Some(Status::Limited) => 1,
                Some(Status::Pending) => 2,
                Some(Status::Denied) => 3,
                None => 4,
            };
            t[slot] += 1;
        }
        t
    }
}

#[derive(Clone, Copy)]
enum Word {
    Title,
    Program,
    Size,
    Verdict,
    Covered,
    Partly,
    NotCovered,
    NothingRun,
    Counts,
    Uses,
    Line,
    Problems,
    Notes,
    Stopped,
    Directive,
    Statement,
    Type,
    Function,
    Member,
    Constant,
    Allowed,
    Limited,
    Pending,
    Denied,
    Unknown,
}

fn word(w: Word, lang: Lang) -> &'static str {
    let row: [&str; 8] = match w {
        Word::Title => [
            "Compatibility report of a user program",
            "Informe de compatibilidad de un programa de usuario",
            "Kompatibilitätsbericht eines Benutzerprogramms",
            "Rapport de compatibilité d'un programme utilisateur",
            "Rapporto di compatibilità di un programma utente",
            "ユーザープログラムの互換性レポート",
            "用户程序兼容性报告",
            "Relatório de compatibilidade de um programa de utilizador",
        ],
        Word::Program => [
            "Program", "Programa", "Programm", "Programme", "Programma", "プログラム", "程序",
            "Programa",
        ],
        Word::Size => [
            "Size in bytes",
            "Tamaño en bytes",
            "Größe in Bytes",
            "Taille en octets",
            "Dimensione in byte",
            "サイズ（バイト）",
            "大小（字节）",
            "Tamanho em bytes",
        ],
        Word::Verdict => [
            "Verdict", "Veredicto", "Urteil", "Verdict", "Verdetto", "判定", "结论", "Veredicto",
        ],
        Word::Covered => [
            "covered", "cubierto", "abgedeckt", "couvert", "coperto", "対応済み", "已覆盖",
            "coberto",
        ],
        Word::Partly => [
            "partly covered",
            "cubierto en parte",
            "teilweise abgedeckt",
            "couvert en partie",
            "coperto in parte",
            "一部対応",
            "部分覆盖",
            "coberto em parte",
        ],
        Word::NotCovered => [
            "not covered",
            "no cubierto",
            "nicht abgedeckt",
            "non couvert",
            "non coperto",
            "未対応",
            "未覆盖",
            "não coberto",
        ],
        Word::NothingRun => [
            "Nothing was run: the report only reads the program. The evaluator is not written yet, so the verdict says what the list of allowed names would let through.",
            "No se ejecutó nada: el informe solo lee el programa. El evaluador todavía no está escrito, así que el veredicto dice lo que dejaría pasar la lista de nombres permitidos.",
            "Es wurde nichts ausgeführt: Der Bericht liest das Programm nur. Der Auswerter ist noch nicht geschrieben, das Urteil sagt also, was die Liste der erlaubten Namen durchlassen würde.",
            "Rien n'a été exécuté : le rapport ne fait que lire le programme. L'évaluateur n'est pas encore écrit, le verdict dit donc ce que laisserait passer la liste des noms autorisés.",
            "Non è stato eseguito nulla: il rapporto si limita a leggere il programma. Il valutatore non è ancora scritto, quindi il verdetto dice ciò che l'elenco dei nomi consentiti lascerebbe passare.",
            "何も実行していません。レポートはプログラムを読むだけです。評価器はまだ書かれていないため、判定は許可名一覧が通すものを示します。",
            "未运行任何内容：报告只读取程序。求值器尚未编写，因此结论只说明允许名称列表会放行什么。",
            "Nada foi executado: o relatório apenas lê o programa. O avaliador ainda não está escrito, pelo que o veredicto diz o que a lista de nomes permitidos deixaria passar.",
        ],
        Word::Counts => [
            "Statements and functions",
            "Sentencias y funciones",
            "Anweisungen und Funktionen",
            "Instructions et fonctions",
            "Istruzioni e funzioni",
            "文と関数",
            "语句与函数",
            "Instruções e funções",
        ],
        Word::Uses => [
            "What the program uses",
            "Lo que usa el programa",
            "Was das Programm verwendet",
            "Ce que le programme utilise",
            "Ciò che il programma usa",
            "プログラムが使うもの",
            "程序使用的内容",
            "O que o programa usa",
        ],
        Word::Line => [
            "line", "línea", "Zeile", "ligne", "riga", "行", "行", "linha",
        ],
        Word::Problems => [
            "Problems of the reading",
            "Problemas de la lectura",
            "Probleme beim Lesen",
            "Problèmes de la lecture",
            "Problemi della lettura",
            "読み取りの問題",
            "读取问题",
            "Problemas da leitura",
        ],
        Word::Notes => [
            "Notes about the names",
            "Notas sobre los nombres",
            "Hinweise zu den Namen",
            "Remarques sur les noms",
            "Note sui nomi",
            "名前に関する注記",
            "名称说明",
            "Notas sobre os nomes",
        ],
        Word::Stopped => [
            "The reading stopped",
            "La lectura se detuvo",
            "Das Lesen wurde abgebrochen",
            "La lecture s'est arrêtée",
            "La lettura si è interrotta",
            "読み取りが中断されました",
            "读取已停止",
            "A leitura parou",
        ],
        Word::Directive => [
            "directive", "directiva", "Direktive", "directive", "direttiva", "ディレクティブ",
            "指令", "diretiva",
        ],
        Word::Statement => [
            "statement", "sentencia", "Anweisung", "instruction", "istruzione", "文", "语句",
            "instrução",
        ],
        Word::Type => [
            "type", "tipo", "Typ", "type", "tipo", "型", "类型", "tipo",
        ],
        Word::Function => [
            "function", "función", "Funktion", "fonction", "funzione", "関数", "函数", "função",
        ],
        Word::Member => [
            "member", "miembro", "Member", "membre", "membro", "メンバー", "成员", "membro",
        ],
        Word::Constant => [
            "value", "valor", "Wert", "valeur", "valore", "値", "值", "valor",
        ],
        Word::Allowed => [
            "allowed", "permitido", "erlaubt", "autorisé", "consentito", "許可", "允许",
            "permitido",
        ],
        Word::Limited => [
            "with limits",
            "con límites",
            "mit Grenzen",
            "avec limites",
            "con limiti",
            "制限付き",
            "有限制",
            "com limites",
        ],
        Word::Pending => [
            "pending", "pendiente", "ausstehend", "en attente", "in sospeso", "保留", "待定",
            "pendente",
        ],
        Word::Denied => [
            "refused", "rechazado", "abgelehnt", "refusé", "rifiutato", "拒否", "拒绝", "recusado",
        ],
        Word::Unknown => [
            "not on the list",
            "no está en la lista",
            "nicht in der Liste",
            "pas dans la liste",
            "non nell'elenco",
            "一覧にない",
            "不在列表中",
            "não está na lista",
        ],
    };
    let column = match lang {
        Lang::En => 0,
        Lang::Es => 1,
        Lang::De => 2,
        Lang::Fr => 3,
        Lang::It => 4,
        Lang::Ja => 5,
        Lang::Zh => 6,
        Lang::Pt => 7,
    };
    row[column]
}

fn kind_word(kind: UseKind, lang: Lang) -> &'static str {
    word(
        match kind {
            UseKind::Directive => Word::Directive,
            UseKind::Statement => Word::Statement,
            UseKind::Type => Word::Type,
            UseKind::Function => Word::Function,
            UseKind::Member => Word::Member,
            UseKind::Constant => Word::Constant,
        },
        lang,
    )
}

fn status_word(status: Option<Status>, lang: Lang) -> &'static str {
    word(
        match status {
            Some(Status::Allowed) => Word::Allowed,
            Some(Status::Limited) => Word::Limited,
            Some(Status::Pending) => Word::Pending,
            Some(Status::Denied) => Word::Denied,
            None => Word::Unknown,
        },
        lang,
    )
}

fn verdict_word(verdict: Verdict, lang: Lang) -> &'static str {
    word(
        match verdict {
            Verdict::Covered => Word::Covered,
            Verdict::Partly => Word::Partly,
            Verdict::NotCovered => Word::NotCovered,
        },
        lang,
    )
}

/// The report as text in a language.
pub fn report_text(report: &Report, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!("{}\n", word(Word::Title, lang)));
    out.push_str(&format!("{}: {}\n", word(Word::Program, lang), report.name));
    out.push_str(&format!("{}: {}\n", word(Word::Size, lang), report.bytes));
    out.push_str(&format!("SHA-256: {}\n", report.sha256));
    out.push_str(&format!(
        "{}: {}\n",
        word(Word::Verdict, lang),
        verdict_word(report.verdict, lang)
    ));
    out.push_str(&format!("{}\n", word(Word::NothingRun, lang)));
    out.push_str(&format!(
        "{}: {} / {}\n",
        word(Word::Counts, lang),
        report.statements,
        report.functions
    ));
    if let Some(stopped) = &report.stopped {
        out.push_str(&format!(
            "\n{}:\n  {}\n",
            word(Word::Stopped, lang),
            stopped.message(lang)
        ));
    }
    if !report.uses.is_empty() {
        out.push_str(&format!("\n{}:\n", word(Word::Uses, lang)));
        for u in &report.uses {
            out.push_str(&format!(
                "  {}  {}  x{}  {} {}  {}\n",
                kind_word(u.kind, lang),
                u.name,
                u.count,
                word(Word::Line, lang),
                u.line,
                status_word(u.status, lang)
            ));
        }
    }
    if !report.problems.is_empty() {
        out.push_str(&format!("\n{}:\n", word(Word::Problems, lang)));
        for p in &report.problems {
            out.push_str(&format!("  {}\n", p.message(lang)));
        }
        if report.problems_dropped > 0 {
            out.push_str(&format!(
                "  {}: +{}\n",
                Diag::new(LIST_CUT, "").message(lang),
                report.problems_dropped
            ));
        }
    }
    let notes = report.notes();
    if !notes.is_empty() {
        out.push_str(&format!("\n{}:\n", word(Word::Notes, lang)));
        for n in &notes {
            out.push_str(&format!("  {}\n", n.message(lang)));
        }
    }
    out
}

fn diag_json(d: &Diag) -> Value {
    json!({
        "code": d.code,
        "title": title(d.code, Lang::En),
        "line": d.line,
        "col": d.col,
        "detail": d.detail,
    })
}

/// The report as JSON, with its keys in a fixed order and the same for the same program.
pub fn report_json(report: &Report) -> String {
    let uses: Vec<Value> = report
        .uses
        .iter()
        .map(|u| {
            json!({
                "kind": u.kind.word(),
                "name": u.name,
                "count": u.count,
                "line": u.line,
                "col": u.col,
                "status": u.status.map_or("unknown", Status::word),
            })
        })
        .collect();
    let value = json!({
        "format": FORMAT,
        "program": report.name,
        "bytes": report.bytes,
        "sha256": report.sha256,
        "verdict": report.verdict.word(),
        "evaluator": "not written",
        "ran": false,
        "statements": report.statements,
        "functions": report.functions,
        "uses": uses,
        "problems": report.problems.iter().map(diag_json).collect::<Vec<_>>(),
        "problems_dropped": report.problems_dropped,
        "notes": report.notes().iter().map(diag_json).collect::<Vec<_>>(),
        "stopped": report.stopped.as_ref().map(diag_json),
    });
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(text: &str) -> Report {
        analyse(
            "test.ulp",
            text,
            &UlpSettings::default(),
            &Support::shipped(),
        )
    }

    #[test]
    fn a_program_with_only_allowed_names_is_covered() {
        let r = report(
            "int n = 0;\nboard(B) { B.elements(E) { n++; } }\nstring s = sprintf(\"%d\", n);\n",
        );
        assert_eq!(r.verdict, Verdict::Covered, "{}", report_text(&r, Lang::En));
        assert!(r.problems.is_empty() && r.stopped.is_none());
        assert_eq!(r.tally()[2..], [0, 0, 0]);
    }

    #[test]
    fn a_name_with_limits_keeps_the_program_covered_and_is_noted() {
        let r = report("output(\"a.txt\", \"wt\") { printf(\"x\\n\"); }\n");
        assert_eq!(r.verdict, Verdict::Covered);
        let notes = r.notes();
        assert!(notes.iter().all(|n| n.code == ITEM_LIMITED));
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn a_dialog_or_a_name_that_is_not_on_the_list_makes_it_partly_covered() {
        let a = report("dlgDialog(\"x\") { dlgLabel(\"y\"); };\n");
        assert_eq!(a.verdict, Verdict::Partly);
        assert!(a.notes().iter().any(|n| n.code == ITEM_PENDING));
        let b = report("int n = mysteryFunction(1);\n");
        assert_eq!(b.verdict, Verdict::Partly);
        assert!(b.notes().iter().any(|n| n.code == NAME_UNKNOWN));
        assert_eq!(b.tally()[4], 1);
    }

    #[test]
    fn a_name_the_sandbox_refuses_makes_it_not_covered() {
        let r = report("system(\"dir\");\n");
        assert_eq!(r.verdict, Verdict::NotCovered);
        assert!(r.notes().iter().any(|n| n.code == ITEM_DENIED));
        // a refused name wins over a pending one
        let both = report("system(\"x\"); dlgDialog(\"y\") { };\n");
        assert_eq!(both.verdict, Verdict::NotCovered);
    }

    #[test]
    fn a_program_that_cannot_be_read_is_not_covered_and_says_why() {
        let r = report("int a = ;\nint b = 2;\n");
        assert_eq!(r.verdict, Verdict::NotCovered);
        assert_eq!(r.problems.len(), 1);
        let text = report_text(&r, Lang::En);
        assert!(text.contains("PHOENIX-ULP-015 [1:9]"), "{text}");
        let small = UlpSettings {
            max_tokens: 3,
            ..UlpSettings::default()
        };
        let stopped = analyse("big.ulp", "a b c d e", &small, &Support::shipped());
        assert_eq!(stopped.verdict, Verdict::NotCovered);
        assert!(stopped.stopped.is_some());
        assert!(report_text(&stopped, Lang::En).contains("PHOENIX-ULP-005"));
    }

    #[test]
    fn the_empty_program_is_covered_because_it_uses_nothing() {
        let r = report("");
        assert_eq!(r.verdict, Verdict::Covered);
        assert!(r.uses.is_empty());
        assert_eq!(r.sha256.len(), 64);
        assert_eq!(
            r.sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_same_program_gives_the_same_text_and_the_same_json() {
        let text = "int f(int a) { return abs(a); }\nboard(B) { B.signals(S) { printf(\"%s\", S.name); } }\n";
        let (a, b) = (report(text), report(text));
        assert_eq!(report_json(&a), report_json(&b));
        for lang in Lang::ALL {
            assert_eq!(report_text(&a, lang), report_text(&b, lang));
        }
    }

    #[test]
    fn the_json_has_the_fields_of_the_format() {
        let r = report("system(\"dir\");\n");
        let v: Value = serde_json::from_str(&report_json(&r)).unwrap();
        assert_eq!(v["format"], FORMAT);
        assert_eq!(v["verdict"], "not_covered");
        assert_eq!(v["evaluator"], "not written");
        assert_eq!(v["ran"], false);
        let system = v["uses"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["name"] == "system")
            .unwrap();
        assert_eq!(system["kind"], "function");
        assert_eq!(system["status"], "denied");
        assert!(v["stopped"].is_null());
        let denied: Vec<&Value> = v["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["code"] == ITEM_DENIED)
            .collect();
        assert_eq!(denied.len(), 1);
        assert_eq!(denied[0]["line"], 1);
    }

    #[test]
    fn the_text_exists_in_every_language_and_none_is_a_copy_of_the_english_one() {
        let r = report("system(\"dir\");\nint a = ;\n");
        let en = report_text(&r, Lang::En);
        for lang in Lang::ALL {
            let t = report_text(&r, lang);
            assert!(t.contains("test.ulp") && t.contains(&r.sha256));
            if lang != Lang::En {
                assert_ne!(t, en, "{lang:?}");
                assert!(!t.contains("Compatibility report"), "{lang:?}");
            }
        }
    }

    #[test]
    fn the_notes_are_in_the_order_of_the_program() {
        let r = report("dlgDialog(\"x\") { };\nsystem(\"y\");\nint n = nothing(2);\n");
        let lines: Vec<usize> = r.notes().iter().map(|n| n.line).collect();
        let mut sorted = lines.clone();
        sorted.sort_unstable();
        assert_eq!(lines, sorted);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn a_list_the_owner_edits_changes_the_verdict() {
        let strict = Support::parse("function sprintf denied\n").unwrap();
        let r = analyse(
            "a.ulp",
            "s = sprintf(\"x\");",
            &UlpSettings::default(),
            &strict,
        );
        assert_eq!(r.verdict, Verdict::NotCovered);
        let empty = Support::parse("").unwrap();
        let r = analyse(
            "a.ulp",
            "s = sprintf(\"x\");",
            &UlpSettings::default(),
            &empty,
        );
        assert_eq!(r.verdict, Verdict::Partly);
    }
}
