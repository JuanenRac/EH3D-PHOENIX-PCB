// SPDX-License-Identifier: GPL-3.0-or-later
//! The compatibility report of scripts.
//!
//! For each command of each script the report says whether it is `equivalent`, `adapted` (with the
//! difference) or `pending` (with the cause). It is computed from the commands alone, without
//! running anything and without a board, with the same [`classify`] function that the run uses, so
//! the report and a run never disagree about which forms of a command are understood. It is given as
//! text, as JSON and as one self-contained HTML page, in the eight languages of the suite; a folder
//! of scripts gets one line per script and the totals.
//!
//! The percentages are whole tenths of a percent (`1000` is all of them). A script without commands
//! counts as 100.0 % so that an empty file does not lower the rate of a folder; the rate of a folder
//! is computed over all its commands and not as the mean of the rates of its scripts.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::actions::{classify, Status};
use crate::codes::{column, title, Diag, FILE_FAILED, LIMIT_EXCEEDED, REGISTRY, REPORT_LIMIT};
use crate::files;
use crate::parse::parse_script;
use crate::settings::ScriptSettings;

/// One command of a script and how well it is covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The line of the command.
    pub line: usize,
    /// The column of the command.
    pub col: usize,
    /// The name in capitals, or `?` for text that could not be read as a command.
    pub command: String,
    /// The status.
    pub status: Status,
    /// The code of the cause or of the difference; none for an equivalent command.
    pub code: Option<&'static str>,
    /// What is specific to the case, in English.
    pub detail: String,
}

/// The report of one script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptReport {
    /// The path, relative to the folder that was analysed, with `/` between the parts.
    pub path: String,
    /// The commands, in the order of the script.
    pub entries: Vec<Entry>,
}

/// The part of a total, in tenths of a percent; an empty total is 1000.
pub fn tenths(part: usize, total: usize) -> u32 {
    if total == 0 {
        1000
    } else {
        u32::try_from(part.saturating_mul(1000) / total).unwrap_or(1000)
    }
}

/// A number of tenths as text: `1000` is `100.0`.
pub fn percent_text(tenths: u32) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

impl ScriptReport {
    /// How many commands have a status.
    pub fn count(&self, status: Status) -> usize {
        self.entries.iter().filter(|e| e.status == status).count()
    }

    /// The number of commands.
    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// The share of equivalent commands, in tenths of a percent.
    pub fn equivalent_tenths(&self) -> u32 {
        tenths(self.count(Status::Equivalent), self.total())
    }

    /// The share of commands that are equivalent or adapted, in tenths of a percent.
    pub fn covered_tenths(&self) -> u32 {
        tenths(
            self.count(Status::Equivalent) + self.count(Status::Adapted),
            self.total(),
        )
    }
}

/// Analyses the text of a script. `path` is only the name shown in the report.
pub fn analyse_text(path: &str, text: &str, settings: &ScriptSettings) -> ScriptReport {
    let parsed = parse_script(text, settings);
    let mut entries: Vec<Entry> = parsed
        .issues
        .iter()
        .map(|issue| Entry {
            line: issue.line,
            col: issue.col,
            command: "?".to_owned(),
            status: Status::Pending,
            code: Some(issue.code),
            detail: issue.detail.clone(),
        })
        .collect();
    for cmd in &parsed.commands {
        let (status, note) = classify(cmd, settings);
        entries.push(Entry {
            line: cmd.line,
            col: cmd.col,
            command: cmd.name.clone(),
            status,
            code: note.as_ref().map(|n| n.code),
            detail: note.map(|n| n.detail).unwrap_or_default(),
        });
    }
    entries.sort_by_key(|e| (e.line, e.col));
    ScriptReport {
        path: path.to_owned(),
        entries,
    }
}

/// The report of a script file or of a folder of scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    /// The scripts, in the order of their paths.
    pub scripts: Vec<ScriptReport>,
    /// Set when the walk stopped at a limit of the settings.
    pub limit: Option<Diag>,
}

/// The sums over a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    /// The number of scripts.
    pub scripts: usize,
    /// The number of commands.
    pub commands: usize,
    /// The equivalent commands.
    pub equivalent: usize,
    /// The adapted commands.
    pub adapted: usize,
    /// The pending commands.
    pub pending: usize,
}

impl Totals {
    /// The share of equivalent commands, in tenths of a percent.
    pub fn equivalent_tenths(&self) -> u32 {
        tenths(self.equivalent, self.commands)
    }

    /// The share of equivalent or adapted commands, in tenths of a percent.
    pub fn covered_tenths(&self) -> u32 {
        tenths(self.equivalent + self.adapted, self.commands)
    }
}

impl Folder {
    /// The sums over all the scripts.
    pub fn totals(&self) -> Totals {
        let mut t = Totals {
            scripts: self.scripts.len(),
            commands: 0,
            equivalent: 0,
            adapted: 0,
            pending: 0,
        };
        for s in &self.scripts {
            t.commands += s.total();
            t.equivalent += s.count(Status::Equivalent);
            t.adapted += s.count(Status::Adapted);
            t.pending += s.count(Status::Pending);
        }
        t
    }

    /// How many pending commands each cause has, by code.
    pub fn causes(&self) -> BTreeMap<&'static str, usize> {
        let mut out = BTreeMap::new();
        for e in self.scripts.iter().flat_map(|s| s.entries.iter()) {
            if let (Status::Pending, Some(code)) = (e.status, e.code) {
                *out.entry(code).or_insert(0) += 1;
            }
        }
        out
    }
}

fn is_script(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("scr"))
}

fn read_script(display: String, path: &Path, settings: &ScriptSettings) -> ScriptReport {
    let failed = |code: &'static str, detail: String| ScriptReport {
        path: display.clone(),
        entries: vec![Entry {
            line: 1,
            col: 1,
            command: "?".to_owned(),
            status: Status::Pending,
            code: Some(code),
            detail,
        }],
    };
    let length = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return failed(FILE_FAILED, e.to_string()),
    };
    if length > settings.max_script_bytes as u64 {
        return failed(
            LIMIT_EXCEEDED,
            format!(
                "the script is larger than {} bytes",
                settings.max_script_bytes
            ),
        );
    }
    match files::read_limited(path, settings.max_script_bytes as u64) {
        Ok(bytes) => analyse_text(&display, &String::from_utf8_lossy(&bytes), settings),
        Err(d) => failed(d.code, d.detail),
    }
}

fn walk(
    dir: &Path,
    prefix: &str,
    depth: usize,
    settings: &ScriptSettings,
    found: &mut Vec<(String, std::path::PathBuf)>,
    limit: &mut Option<Diag>,
) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<(String, std::fs::FileType, std::path::PathBuf)> = Vec::new();
    for entry in read.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        entries.push((
            entry.file_name().to_string_lossy().into_owned(),
            kind,
            entry.path(),
        ));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, kind, path) in entries {
        if limit.is_some() {
            return;
        }
        if kind.is_symlink() {
            continue;
        }
        let shown = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if kind.is_dir() {
            if depth >= settings.max_walk_depth {
                *limit = Some(Diag::plain(
                    REPORT_LIMIT,
                    format!("folders deeper than {}", settings.max_walk_depth),
                ));
                return;
            }
            walk(&path, &shown, depth + 1, settings, found, limit);
        } else if kind.is_file() && is_script(&path) {
            if found.len() >= settings.max_files {
                *limit = Some(Diag::plain(
                    REPORT_LIMIT,
                    format!("more than {} scripts", settings.max_files),
                ));
                return;
            }
            found.push((shown, path));
        }
    }
}

/// Analyses one script file or every `.scr` file below a folder. Links are not followed.
pub fn analyse_path(path: &Path, settings: &ScriptSettings) -> Result<Folder, Diag> {
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| Diag::plain(FILE_FAILED, format!("{}: {e}", path.display())))?;
    if meta.is_file() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "script".to_owned());
        let report = read_script(name, path, settings);
        return Ok(Folder {
            scripts: vec![report],
            limit: None,
        });
    }
    if !meta.is_dir() {
        return Err(Diag::plain(
            FILE_FAILED,
            format!("{} is not a file or a folder", path.display()),
        ));
    }
    let mut found = Vec::new();
    let mut limit = None;
    walk(path, "", 0, settings, &mut found, &mut limit);
    found.sort_by(|a, b| a.0.cmp(&b.0));
    let scripts = found
        .into_iter()
        .map(|(shown, file)| read_script(shown, &file, settings))
        .collect();
    Ok(Folder { scripts, limit })
}

#[derive(Clone, Copy)]
enum Label {
    Title,
    Intro,
    Script,
    Commands,
    Equivalent,
    Adapted,
    Pending,
    EquivalentRate,
    CoveredRate,
    Total,
    Position,
    Command,
    Status,
    Cause,
    Causes,
    Count,
    NoScripts,
    SEquivalent,
    SAdapted,
    SPending,
}

const LABELS: [[&str; 8]; 20] = [
    [
        "Script compatibility report",
        "Informe de compatibilidad de scripts",
        "Kompatibilitätsbericht der Skripte",
        "Rapport de compatibilité des scripts",
        "Rapporto di compatibilità degli script",
        "スクリプト互換性レポート",
        "脚本兼容性报告",
        "Relatório de compatibilidade dos scripts",
    ],
    [
        "Computed without running any script. Each command is equivalent, adapted (with the difference) or pending (with the cause).",
        "Calculado sin ejecutar ningún script. Cada orden es equivalente, adaptada (con la diferencia) o pendiente (con la causa).",
        "Berechnet, ohne ein Skript auszuführen. Jeder Befehl ist gleichwertig, angepasst (mit dem Unterschied) oder ausstehend (mit der Ursache).",
        "Calculé sans exécuter aucun script. Chaque commande est équivalente, adaptée (avec la différence) ou en attente (avec la cause).",
        "Calcolato senza eseguire alcuno script. Ogni comando è equivalente, adattato (con la differenza) o in sospeso (con la causa).",
        "スクリプトを実行せずに算出しました。各コマンドは、同等、調整済み（差異あり）、保留（原因あり）のいずれかです。",
        "未运行任何脚本即计算得出。每条命令为等价、已调整（附差异）或待处理（附原因）。",
        "Calculado sem executar nenhum script. Cada comando é equivalente, adaptado (com a diferença) ou pendente (com a causa).",
    ],
    [
        "Script", "Script", "Skript", "Script", "Script", "スクリプト", "脚本", "Script",
    ],
    [
        "Commands", "Órdenes", "Befehle", "Commandes", "Comandi", "コマンド数", "命令数",
        "Comandos",
    ],
    [
        "Equivalent", "Equivalentes", "Gleichwertig", "Équivalentes", "Equivalenti", "同等",
        "等价", "Equivalentes",
    ],
    [
        "Adapted", "Adaptadas", "Angepasst", "Adaptées", "Adattati", "調整済み", "已调整",
        "Adaptados",
    ],
    [
        "Pending", "Pendientes", "Ausstehend", "En attente", "In sospeso", "保留", "待处理",
        "Pendentes",
    ],
    [
        "Equivalent %",
        "% equivalente",
        "% gleichwertig",
        "% équivalentes",
        "% equivalenti",
        "同等の割合",
        "等价比例",
        "% equivalentes",
    ],
    [
        "Equivalent or adapted %",
        "% equivalente o adaptado",
        "% gleichwertig oder angepasst",
        "% équivalentes ou adaptées",
        "% equivalenti o adattati",
        "同等または調整済みの割合",
        "等价或已调整比例",
        "% equivalentes ou adaptados",
    ],
    [
        "Total", "Total", "Gesamt", "Total", "Totale", "合計", "总计", "Total",
    ],
    [
        "Line:column",
        "Línea:columna",
        "Zeile:Spalte",
        "Ligne:colonne",
        "Riga:colonna",
        "行:列",
        "行:列",
        "Linha:coluna",
    ],
    [
        "Command", "Orden", "Befehl", "Commande", "Comando", "コマンド", "命令", "Comando",
    ],
    [
        "Status", "Estado", "Status", "Statut", "Stato", "状態", "状态", "Estado",
    ],
    [
        "Cause or difference",
        "Causa o diferencia",
        "Ursache oder Unterschied",
        "Cause ou différence",
        "Causa o differenza",
        "原因または差異",
        "原因或差异",
        "Causa ou diferença",
    ],
    [
        "Causes of the pending commands",
        "Causas de las órdenes pendientes",
        "Ursachen der ausstehenden Befehle",
        "Causes des commandes en attente",
        "Cause dei comandi in sospeso",
        "保留コマンドの原因",
        "待处理命令的原因",
        "Causas dos comandos pendentes",
    ],
    [
        "Count", "Cantidad", "Anzahl", "Nombre", "Numero", "件数", "数量", "Quantidade",
    ],
    [
        "No script was found",
        "No se encontró ningún script",
        "Es wurde kein Skript gefunden",
        "Aucun script n'a été trouvé",
        "Non è stato trovato alcuno script",
        "スクリプトが見つかりません",
        "未找到脚本",
        "Não foi encontrado nenhum script",
    ],
    [
        "equivalent", "equivalente", "gleichwertig", "équivalente", "equivalente", "同等",
        "等价", "equivalente",
    ],
    [
        "adapted", "adaptada", "angepasst", "adaptée", "adattato", "調整済み", "已调整",
        "adaptado",
    ],
    [
        "pending", "pendiente", "ausstehend", "en attente", "in sospeso", "保留", "待处理",
        "pendente",
    ],
];

fn label(l: Label, lang: Lang) -> &'static str {
    LABELS[l as usize][column(lang)]
}

pub(crate) fn status_word(status: Status, lang: Lang) -> &'static str {
    match status {
        Status::Equivalent => label(Label::SEquivalent, lang),
        Status::Adapted => label(Label::SAdapted, lang),
        Status::Pending => label(Label::SPending, lang),
    }
}

fn english_title(code: &str) -> &'static str {
    REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| t)
}

impl Folder {
    /// The report as text.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{}", label(Label::Title, lang));
        let _ = writeln!(out, "{}", label(Label::Intro, lang));
        if self.scripts.is_empty() {
            let _ = writeln!(out, "{}", label(Label::NoScripts, lang));
            return out;
        }
        let width = self
            .scripts
            .iter()
            .map(|s| s.path.chars().count())
            .max()
            .unwrap_or(0)
            .max(label(Label::Script, lang).chars().count());
        let _ = writeln!(
            out,
            "{:<width$}  {:>8}  {:>10}  {:>8}  {:>8}  {:>8}  {:>8}",
            label(Label::Script, lang),
            "n",
            label(Label::Equivalent, lang),
            label(Label::Adapted, lang),
            label(Label::Pending, lang),
            "eq.%",
            "eq+ad.%",
        );
        for s in &self.scripts {
            let _ = writeln!(
                out,
                "{:<width$}  {:>8}  {:>10}  {:>8}  {:>8}  {:>8}  {:>8}",
                s.path,
                s.total(),
                s.count(Status::Equivalent),
                s.count(Status::Adapted),
                s.count(Status::Pending),
                percent_text(s.equivalent_tenths()),
                percent_text(s.covered_tenths()),
            );
        }
        let t = self.totals();
        let _ = writeln!(
            out,
            "{:<width$}  {:>8}  {:>10}  {:>8}  {:>8}  {:>8}  {:>8}",
            label(Label::Total, lang),
            t.commands,
            t.equivalent,
            t.adapted,
            t.pending,
            percent_text(t.equivalent_tenths()),
            percent_text(t.covered_tenths()),
        );
        let causes = self.causes();
        if !causes.is_empty() {
            let _ = writeln!(out, "\n{}", label(Label::Causes, lang));
            for (code, count) in &causes {
                let _ = writeln!(out, "  {code} {}: {count}", title(code, lang));
            }
        }
        for s in &self.scripts {
            let _ = writeln!(out, "\n{}", s.path);
            for e in &s.entries {
                let mut line = format!(
                    "  {}:{} {} {}",
                    e.line,
                    e.col,
                    e.command,
                    status_word(e.status, lang)
                );
                if let Some(code) = e.code {
                    let _ = write!(line, " {code} {}", title(code, lang));
                }
                if !e.detail.is_empty() {
                    let _ = write!(line, " ({})", e.detail);
                }
                let _ = writeln!(out, "{line}");
            }
        }
        if let Some(limit) = &self.limit {
            let _ = writeln!(out, "\n{}", limit.message(lang));
        }
        out
    }

    /// The report as JSON (`phoenix-script-report/1`). The details are in English and a title is the
    /// English one; a program shows the title of its language from the code.
    pub fn to_json(&self) -> String {
        let t = self.totals();
        let scripts: Vec<Value> = self
            .scripts
            .iter()
            .map(|s| {
                let entries: Vec<Value> = s
                    .entries
                    .iter()
                    .map(|e| {
                        json!({
                            "line": e.line,
                            "column": e.col,
                            "command": e.command,
                            "status": e.status.key(),
                            "code": e.code,
                            "title": e.code.map(english_title),
                            "detail": e.detail,
                        })
                    })
                    .collect();
                json!({
                    "path": s.path,
                    "commands": s.total(),
                    "equivalent": s.count(Status::Equivalent),
                    "adapted": s.count(Status::Adapted),
                    "pending": s.count(Status::Pending),
                    "equivalent_tenths": s.equivalent_tenths(),
                    "covered_tenths": s.covered_tenths(),
                    "entries": entries,
                })
            })
            .collect();
        let causes: Vec<Value> = self
            .causes()
            .iter()
            .map(|(code, count)| {
                json!({ "code": code, "title": english_title(code), "count": count })
            })
            .collect();
        let value = json!({
            "schema": "phoenix-script-report/1",
            "totals": {
                "scripts": t.scripts,
                "commands": t.commands,
                "equivalent": t.equivalent,
                "adapted": t.adapted,
                "pending": t.pending,
                "equivalent_tenths": t.equivalent_tenths(),
                "covered_tenths": t.covered_tenths(),
            },
            "causes": causes,
            "scripts": scripts,
            "limit": self.limit.as_ref().map(|d| d.code),
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }

    /// The report as one HTML page with its style inside, no script and no external file.
    pub fn to_html(&self, lang: Lang) -> String {
        let t = self.totals();
        let mut out = String::new();
        let _ = write!(
            out,
            "<!DOCTYPE html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{}</title><style>\
:root{{--bg:#fff;--fg:#1b1f24;--muted:#59636e;--line:#d0d7de;--ok:#1a7f37;--mid:#9a6700;--bad:#cf222e}}\
@media (prefers-color-scheme:dark){{:root{{--bg:#0d1117;--fg:#e6edf3;--muted:#9198a1;--line:#30363d;--ok:#3fb950;--mid:#d29922;--bad:#f85149}}}}\
body{{margin:0;padding:16px;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,sans-serif}}\
main{{max-width:1100px;margin:auto}}table{{border-collapse:collapse;width:100%;margin:12px 0}}\
th,td{{border-bottom:1px solid var(--line);padding:4px 8px;text-align:left;vertical-align:top}}\
td.n,th.n{{text-align:right}}.muted{{color:var(--muted)}}\
.equivalent{{color:var(--ok)}}.adapted{{color:var(--mid)}}.pending{{color:var(--bad)}}\
.bar{{display:inline-block;height:8px;background:var(--ok);border-radius:4px}}\
@media (max-width:640px){{body{{font-size:14px}}th,td{{padding:3px 4px}}}}\
</style></head><body><main>\n<h1>{}</h1>\n<p class=\"muted\">{}</p>\n",
            lang_tag(lang),
            escape(label(Label::Title, lang)),
            escape(label(Label::Title, lang)),
            escape(label(Label::Intro, lang)),
        );
        if self.scripts.is_empty() {
            let _ = write!(
                out,
                "<p>{}</p>\n</main></body></html>\n",
                escape(label(Label::NoScripts, lang))
            );
            return out;
        }
        let _ = writeln!(
            out,
            "<table><thead><tr><th>{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th></tr></thead><tbody>\n",
            escape(label(Label::Script, lang)),
            escape(label(Label::Commands, lang)),
            escape(label(Label::Equivalent, lang)),
            escape(label(Label::Adapted, lang)),
            escape(label(Label::Pending, lang)),
            escape(label(Label::EquivalentRate, lang)),
            escape(label(Label::CoveredRate, lang)),
        );
        for s in &self.scripts {
            let _ = writeln!(
                out,
                "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td></tr>\n",
                escape(&s.path),
                s.total(),
                s.count(Status::Equivalent),
                s.count(Status::Adapted),
                s.count(Status::Pending),
                percent_text(s.equivalent_tenths()),
                percent_text(s.covered_tenths()),
            );
        }
        let _ = write!(
            out,
            "<tr><th>{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{}</th><th class=\"n\">{} <span class=\"bar\" style=\"width:{}px\"></span></th><th class=\"n\">{}</th></tr>\n</tbody></table>\n",
            escape(label(Label::Total, lang)),
            t.commands,
            t.equivalent,
            t.adapted,
            t.pending,
            percent_text(t.equivalent_tenths()),
            t.equivalent_tenths() / 10,
            percent_text(t.covered_tenths()),
        );
        let causes = self.causes();
        if !causes.is_empty() {
            let _ = write!(
                out,
                "<h2>{}</h2>\n<table><thead><tr><th>{}</th><th>{}</th><th class=\"n\">{}</th></tr></thead><tbody>\n",
                escape(label(Label::Causes, lang)),
                "Code",
                escape(label(Label::Cause, lang)),
                escape(label(Label::Count, lang)),
            );
            for (code, count) in &causes {
                let _ = writeln!(
                    out,
                    "<tr><td>{code}</td><td>{}</td><td class=\"n\">{count}</td></tr>\n",
                    escape(title(code, lang)),
                );
            }
            out.push_str("</tbody></table>\n");
        }
        for s in &self.scripts {
            let _ = write!(
                out,
                "<h2>{}</h2>\n<table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>\n",
                escape(&s.path),
                escape(label(Label::Position, lang)),
                escape(label(Label::Command, lang)),
                escape(label(Label::Status, lang)),
                escape(label(Label::Cause, lang)),
            );
            for e in &s.entries {
                let cause = match e.code {
                    Some(code) => format!("{code} {}", title(code, lang)),
                    None => String::new(),
                };
                let detail = if e.detail.is_empty() {
                    String::new()
                } else {
                    format!(" <span class=\"muted\">({})</span>", escape(&e.detail))
                };
                let _ = writeln!(
                    out,
                    "<tr><td>{}:{}</td><td>{}</td><td class=\"{}\">{}</td><td>{}{}</td></tr>\n",
                    e.line,
                    e.col,
                    escape(&e.command),
                    e.status.key(),
                    escape(status_word(e.status, lang)),
                    escape(&cause),
                    detail,
                );
            }
            out.push_str("</tbody></table>\n");
        }
        if let Some(limit) = &self.limit {
            let _ = writeln!(out, "<p>{}</p>", escape(&limit.message(lang)));
        }
        out.push_str("</main></body></html>\n");
        out
    }
}

fn lang_tag(lang: Lang) -> &'static str {
    ["en", "es", "de", "fr", "it", "ja", "zh", "pt"][column(lang)]
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{
        ARGUMENT_INVALID, A_ENGINE, A_FILES, A_SILK, C_PICK, SYNTAX_STRING, UNKNOWN_COMMAND,
    };

    fn analyse(text: &str) -> ScriptReport {
        analyse_text("t.scr", text, &ScriptSettings::default())
    }

    #[test]
    fn each_command_gets_a_status_and_a_cause() {
        let r = analyse(
            "GRID mm 0.5\nLAYER Top\nWIRE 'GND' 0.3 (0 0) (1 1)\nCIRCLE (0 0) (1 0)\nDRC\nMOVE (1 1) (2 2)\nFROBNICATE\n",
        );
        let got: Vec<(&str, Status, Option<&str>)> = r
            .entries
            .iter()
            .map(|e| (e.command.as_str(), e.status, e.code))
            .collect();
        assert_eq!(
            got,
            vec![
                ("GRID", Status::Equivalent, None),
                ("LAYER", Status::Equivalent, None),
                ("WIRE", Status::Equivalent, None),
                ("CIRCLE", Status::Adapted, Some(A_SILK)),
                ("DRC", Status::Adapted, Some(A_ENGINE)),
                ("MOVE", Status::Pending, Some(C_PICK)),
                ("FROBNICATE", Status::Pending, Some(UNKNOWN_COMMAND)),
            ]
        );
        assert_eq!(r.total(), 7);
        assert_eq!(r.count(Status::Equivalent), 3);
        assert_eq!(r.count(Status::Adapted), 2);
        assert_eq!(r.count(Status::Pending), 2);
        assert_eq!(r.equivalent_tenths(), 428);
        assert_eq!(r.covered_tenths(), 714);
        assert_eq!(percent_text(r.equivalent_tenths()), "42.8");
    }

    #[test]
    fn text_that_cannot_be_read_is_pending_at_its_place() {
        let r = analyse("GRID mm\nTEXT 'oops\nLAYER Top");
        assert_eq!(r.total(), 3);
        let bad = &r.entries[1];
        assert_eq!((bad.line, bad.col), (2, 6));
        assert_eq!(bad.status, Status::Pending);
        assert_eq!(bad.code, Some(SYNTAX_STRING));
        assert_eq!(bad.command, "?");
    }

    #[test]
    fn the_rate_of_an_empty_script_is_all() {
        let r = analyse("# nothing\n");
        assert_eq!(r.total(), 0);
        assert_eq!(r.equivalent_tenths(), 1000);
        assert_eq!(percent_text(1000), "100.0");
        assert_eq!(tenths(1, 3), 333);
        assert_eq!(tenths(0, 5), 0);
    }

    #[test]
    fn the_report_never_runs_anything_and_is_the_same_every_time() {
        let text = "WRITE 'out/x.json'\nOPEN 'y.json'\nSCRIPT 'z.scr'\nCHANGE WIDTH 0.2 (1 1)";
        let a = analyse(text);
        let b = analyse(text);
        assert_eq!(a, b);
        assert_eq!(a.entries[0].code, Some(A_FILES));
        assert_eq!(a.entries[3].code, Some(C_PICK));
    }

    #[test]
    fn a_folder_is_walked_in_order_with_totals_and_causes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.scr"), "GRID mm\nMOVE (1 1) (2 2)\n").unwrap();
        std::fs::write(dir.path().join("a.SCR"), "LAYER Top\nDRC\n").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("c.scr"), "BOGUS\nBOGUS\n").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "GRID mm").unwrap();
        let folder = analyse_path(dir.path(), &ScriptSettings::default()).unwrap();
        let names: Vec<&str> = folder.scripts.iter().map(|s| s.path.as_str()).collect();
        assert_eq!(names, vec!["a.SCR", "b.scr", "sub/c.scr"]);
        let t = folder.totals();
        assert_eq!(
            (t.scripts, t.commands, t.equivalent, t.adapted, t.pending),
            (3, 6, 2, 1, 3)
        );
        assert_eq!(t.equivalent_tenths(), 333);
        let causes = folder.causes();
        assert_eq!(causes.get(C_PICK), Some(&1));
        assert_eq!(causes.get(UNKNOWN_COMMAND), Some(&2));
        assert!(folder.limit.is_none());
    }

    #[test]
    fn a_single_file_is_named_without_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("one.scr");
        std::fs::write(&file, "GRID mm\n").unwrap();
        let folder = analyse_path(&file, &ScriptSettings::default()).unwrap();
        assert_eq!(folder.scripts[0].path, "one.scr");
        assert!(analyse_path(&dir.path().join("none"), &ScriptSettings::default()).is_err());
    }

    #[test]
    fn the_limits_of_the_walk_are_noted_and_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.scr", "b.scr", "c.scr"] {
            std::fs::write(dir.path().join(name), "GRID mm\n").unwrap();
        }
        let s = ScriptSettings {
            max_files: 2,
            ..ScriptSettings::default()
        };
        let folder = analyse_path(dir.path(), &s).unwrap();
        assert_eq!(folder.scripts.len(), 2);
        assert_eq!(folder.limit.as_ref().map(|d| d.code), Some(REPORT_LIMIT));

        let deep = dir.path().join("d1").join("d2");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("x.scr"), "GRID mm\n").unwrap();
        let s = ScriptSettings {
            max_walk_depth: 1,
            ..ScriptSettings::default()
        };
        let folder = analyse_path(dir.path(), &s).unwrap();
        assert_eq!(folder.limit.as_ref().map(|d| d.code), Some(REPORT_LIMIT));

        let s = ScriptSettings {
            max_script_bytes: 4,
            ..ScriptSettings::default()
        };
        let folder = analyse_path(dir.path(), &s).unwrap();
        assert!(folder
            .scripts
            .iter()
            .all(|r| r.entries[0].code == Some(LIMIT_EXCEEDED)));
    }

    #[test]
    fn the_three_forms_say_the_same_thing() {
        let r = analyse("GRID mm\nCIRCLE (0 0) (1 0)\nROUTE\nWIRE (0 0)\n");
        let folder = Folder {
            scripts: vec![r],
            limit: None,
        };
        let text = folder.to_text(Lang::En);
        assert!(text.contains("Script compatibility report"));
        assert!(text.contains("2:1 CIRCLE adapted PHOENIX-SCRIPT-035"));
        assert!(text.contains("3:1 ROUTE pending PHOENIX-SCRIPT-027"));
        assert!(text.contains(ARGUMENT_INVALID));
        let json: Value = serde_json::from_str(&folder.to_json()).unwrap();
        assert_eq!(json["schema"], "phoenix-script-report/1");
        assert_eq!(json["totals"]["commands"], 4);
        assert_eq!(json["totals"]["equivalent"], 1);
        assert_eq!(json["totals"]["equivalent_tenths"], 250);
        assert_eq!(json["scripts"][0]["entries"][1]["status"], "adapted");
        assert_eq!(
            json["scripts"][0]["entries"][1]["code"],
            "PHOENIX-SCRIPT-035"
        );
        assert_eq!(json["scripts"][0]["entries"][0]["code"], Value::Null);
        assert_eq!(json["causes"].as_array().map(Vec::len), Some(2));
        let html = folder.to_html(Lang::Pt);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("lang=\"pt\""));
        assert!(html.contains("Relatório de compatibilidade dos scripts"));
        assert!(html.contains("PHOENIX-SCRIPT-035"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("http://") && !html.contains("https://"));
    }

    #[test]
    fn the_html_escapes_what_a_script_wrote() {
        let r = analyse("<b>&\"x 1");
        let folder = Folder {
            scripts: vec![ScriptReport {
                path: "a<b>.scr".to_owned(),
                ..r
            }],
            limit: None,
        };
        let html = folder.to_html(Lang::En);
        assert!(html.contains("a&lt;b&gt;.scr"));
        assert!(html.contains("&lt;B&gt;&amp;&quot;X"));
        assert!(!html.contains("<B>&"));
    }

    #[test]
    fn every_language_has_every_label() {
        for row in LABELS.iter() {
            for text in row {
                assert!(!text.is_empty());
            }
        }
        for lang in Lang::ALL {
            let folder = Folder {
                scripts: vec![analyse("GRID mm\nROUTE")],
                limit: None,
            };
            assert!(folder.to_text(lang).contains("PHOENIX-SCRIPT-027"));
            assert!(folder.to_html(lang).contains(lang_tag(lang)));
        }
        assert_eq!(Label::SPending as usize, LABELS.len() - 1);
    }

    #[test]
    fn an_empty_folder_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let folder = analyse_path(dir.path(), &ScriptSettings::default()).unwrap();
        assert!(folder.scripts.is_empty());
        assert!(folder.to_text(Lang::En).contains("No script was found"));
        assert!(folder.to_html(Lang::En).contains("No script was found"));
        assert_eq!(folder.totals().equivalent_tenths(), 1000);
    }
}
