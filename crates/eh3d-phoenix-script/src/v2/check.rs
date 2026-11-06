// SPDX-License-Identifier: GPL-3.0-or-later
//! The static check of a script of the second version, without a document and without running
//! anything: the syntax, that every variable is defined before it is used, that every function
//! exists, takes that many arguments and applies to the kind of document the header names, and an
//! estimate of the steps the script can take.
//!
//! The estimate is an upper bound by the limits of the settings: a loop whose count is a number the
//! script writes (`for i in 0..4`, or a name that is bound once to such a number and never
//! changed) is counted exactly; any other loop is counted at `max_loop_iterations` turns, and the
//! report then says that the bound comes from the limits.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_rules::Lang;
use serde_json::json;

use crate::codes::Diag;
use crate::settings::ScriptSettings;
use crate::v2::codes::{ARGUMENTS, ESTIMATE, REDEFINED, TARGET, UNDEFINED, UNKNOWN_FUNCTION};
use crate::v2::funcs::find;
use crate::v2::syntax::{parse_program, BinOp, Expr, ExprKind, Program, Stmt, StmtKind, Target};
use crate::v2::value::Value;
use crate::v2::world::{BOARD, LIBRARY, SCHEMATIC};

/// The most errors one check reports.
const MAX_ERRORS: usize = 50;

/// What the check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// The errors, in the order of the script; the script cannot run while there is one.
    pub errors: Vec<Diag>,
    /// What is worth a look but does not stop the script.
    pub warnings: Vec<Diag>,
    /// The kind of document the header names.
    pub target: Option<Target>,
    /// The statements, counting those inside blocks.
    pub statements: usize,
    /// The calls of functions.
    pub calls: usize,
    /// The calls of functions that change the document.
    pub operations: usize,
    /// The estimate of the steps, never more than the limit of the settings can allow.
    pub estimated_steps: u64,
    /// True when every loop has a count that the script writes; false when some bound is the limit
    /// of the settings.
    pub bounded: bool,
    /// The functions called and how many times, by name.
    pub functions: Vec<(String, usize)>,
}

impl CheckReport {
    /// True when there is no error.
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// The report as text, with the titles in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        if self.ok() {
            out.push_str("check: ok\n");
        } else {
            out.push_str(&format!("check: {} error(s)\n", self.errors.len()));
        }
        out.push_str(&format!(
            "target: {}\n",
            self.target.map_or("any", Target::word)
        ));
        out.push_str(&format!(
            "statements: {}; calls: {} (changing the document: {})\n",
            self.statements, self.calls, self.operations
        ));
        out.push_str(&format!(
            "estimated steps: {} ({})\n",
            self.estimated_steps,
            if self.bounded {
                "exact count of every loop"
            } else {
                "an upper bound: some loop is counted at the limit of the settings"
            }
        ));
        if !self.functions.is_empty() {
            let used: Vec<String> = self
                .functions
                .iter()
                .map(|(name, n)| format!("{name} x{n}"))
                .collect();
            out.push_str(&format!("functions: {}\n", used.join(", ")));
        }
        for d in &self.warnings {
            out.push_str(&format!("warning: {}\n", d.message(lang)));
        }
        for d in &self.errors {
            out.push_str(&format!("error: {}\n", d.message(lang)));
        }
        out
    }

    /// The report as JSON (`phoenix-script-check/2`).
    pub fn to_json(&self) -> String {
        let place = |d: &Diag| {
            json!({
                "code": d.code,
                "line": d.line,
                "column": d.col,
                "detail": d.detail,
            })
        };
        let value = json!({
            "schema": "phoenix-script-check/2",
            "ok": self.ok(),
            "target": self.target.map(Target::word),
            "statements": self.statements,
            "calls": self.calls,
            "operations": self.operations,
            "estimated_steps": self.estimated_steps,
            "bounded": self.bounded,
            "functions": self
                .functions
                .iter()
                .map(|(name, n)| json!({ "name": name, "calls": n }))
                .collect::<Vec<_>>(),
            "warnings": self.warnings.iter().map(place).collect::<Vec<_>>(),
            "errors": self.errors.iter().map(place).collect::<Vec<_>>(),
        });
        serde_json::to_string_pretty(&value).unwrap_or_default() + "\n"
    }
}

fn target_bit(t: Target) -> u8 {
    match t {
        Target::Board => BOARD,
        Target::Schematic => SCHEMATIC,
        Target::Library => LIBRARY,
    }
}

struct Walker {
    target: Option<Target>,
    scopes: Vec<BTreeSet<String>>,
    errors: Vec<Diag>,
    calls: usize,
    operations: usize,
    statements: usize,
    usage: BTreeMap<String, usize>,
}

impl Walker {
    fn error(&mut self, d: Diag) {
        if self.errors.len() < MAX_ERRORS {
            self.errors.push(d);
        }
    }

    fn defined(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.contains(name))
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Lit(_) => {}
            ExprKind::Var(name) => {
                if !self.defined(name) {
                    self.error(Diag::new(UNDEFINED, e.pos.line, e.pos.col, name.clone()));
                }
            }
            ExprKind::List(items) => items.iter().for_each(|x| self.expr(x)),
            ExprKind::Neg(x) | ExprKind::Not(x) | ExprKind::Field(x, _) => self.expr(x),
            ExprKind::Bin(_, a, b) | ExprKind::Index(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Call(name, args) => {
                self.calls += 1;
                *self.usage.entry(name.clone()).or_insert(0) += 1;
                match find(name) {
                    None => self.error(Diag::new(
                        UNKNOWN_FUNCTION,
                        e.pos.line,
                        e.pos.col,
                        name.clone(),
                    )),
                    Some(def) => {
                        if def.op {
                            self.operations += 1;
                        }
                        if args.len() < def.min || args.len() > def.max {
                            self.error(Diag::new(
                                ARGUMENTS,
                                e.pos.line,
                                e.pos.col,
                                format!(
                                    "{name} takes {} to {} arguments, not {}",
                                    def.min,
                                    def.max,
                                    args.len()
                                ),
                            ));
                        }
                        if let Some(t) = self.target {
                            if def.targets != 0 && def.targets & target_bit(t) == 0 {
                                self.error(Diag::new(
                                    TARGET,
                                    e.pos.line,
                                    e.pos.col,
                                    format!("{name} does not apply to a {}", t.word()),
                                ));
                            }
                        }
                    }
                }
                args.iter().for_each(|x| self.expr(x));
            }
        }
    }

    fn block(&mut self, body: &[Stmt], bind: Option<&str>) {
        let mut scope = BTreeSet::new();
        if let Some(name) = bind {
            scope.insert(name.to_owned());
        }
        self.scopes.push(scope);
        for s in body {
            self.stmt(s);
        }
        self.scopes.pop();
    }

    fn stmt(&mut self, s: &Stmt) {
        self.statements += 1;
        match &s.kind {
            StmtKind::Let(name, e) => {
                self.expr(e);
                let taken = self.scopes.last().is_some_and(|sc| sc.contains(name));
                if taken {
                    self.error(Diag::new(REDEFINED, s.pos.line, s.pos.col, name.clone()));
                } else if let Some(sc) = self.scopes.last_mut() {
                    sc.insert(name.clone());
                }
            }
            StmtKind::Assign(name, e) => {
                self.expr(e);
                if !self.defined(name) {
                    self.error(Diag::new(UNDEFINED, s.pos.line, s.pos.col, name.clone()));
                }
            }
            StmtKind::If(arms, otherwise) => {
                for (cond, body) in arms {
                    self.expr(cond);
                    self.block(body, None);
                }
                if let Some(body) = otherwise {
                    self.block(body, None);
                }
            }
            StmtKind::For {
                var,
                from,
                to,
                body,
            } => {
                self.expr(from);
                if let Some(end) = to {
                    self.expr(end);
                }
                self.block(body, Some(var));
            }
            StmtKind::While(cond, body) => {
                self.expr(cond);
                self.block(body, None);
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::Expr(e) => self.expr(e),
        }
    }
}

fn collect_names(
    body: &[Stmt],
    defs: &mut BTreeMap<String, usize>,
    assigned: &mut BTreeSet<String>,
) {
    for s in body {
        match &s.kind {
            StmtKind::Let(name, _) => *defs.entry(name.clone()).or_insert(0) += 1,
            StmtKind::Assign(name, _) => {
                assigned.insert(name.clone());
            }
            StmtKind::If(arms, otherwise) => {
                for (_, b) in arms {
                    collect_names(b, defs, assigned);
                }
                if let Some(b) = otherwise {
                    collect_names(b, defs, assigned);
                }
            }
            StmtKind::For { var, body, .. } => {
                *defs.entry(var.clone()).or_insert(0) += 1;
                collect_names(body, defs, assigned);
            }
            StmtKind::While(_, b) => collect_names(b, defs, assigned),
            StmtKind::Break | StmtKind::Continue | StmtKind::Expr(_) => {}
        }
    }
}

fn const_value(e: &Expr, consts: &BTreeMap<String, i64>) -> Option<i64> {
    match &e.kind {
        ExprKind::Lit(Value::Int(n)) => Some(*n),
        ExprKind::Var(name) => consts.get(name).copied(),
        ExprKind::Neg(x) => const_value(x, consts)?.checked_neg(),
        ExprKind::Bin(op, a, b) => {
            let (x, y) = (const_value(a, consts)?, const_value(b, consts)?);
            match op {
                BinOp::Add => x.checked_add(y),
                BinOp::Sub => x.checked_sub(y),
                BinOp::Mul => x.checked_mul(y),
                _ => None,
            }
        }
        _ => None,
    }
}

struct Estimate<'a> {
    settings: &'a ScriptSettings,
    consts: BTreeMap<String, i64>,
    bounded: bool,
}

impl Estimate<'_> {
    fn expr(&self, e: &Expr) -> u64 {
        let inner: u64 = match &e.kind {
            ExprKind::Lit(_) | ExprKind::Var(_) => 0,
            ExprKind::List(items) => items
                .iter()
                .map(|x| self.expr(x))
                .fold(0, u64::saturating_add),
            ExprKind::Call(_, args) => args
                .iter()
                .map(|x| self.expr(x))
                .fold(0, u64::saturating_add),
            ExprKind::Neg(x) | ExprKind::Not(x) | ExprKind::Field(x, _) => self.expr(x),
            ExprKind::Bin(_, a, b) | ExprKind::Index(a, b) => {
                self.expr(a).saturating_add(self.expr(b))
            }
        };
        inner.saturating_add(1)
    }

    fn block(&mut self, body: &[Stmt]) -> u64 {
        let mut total = 0u64;
        for s in body {
            total = total.saturating_add(self.stmt(s));
        }
        total
    }

    fn turns(&mut self, from: &Expr, to: Option<&Expr>) -> u64 {
        let limit = self.settings.max_loop_iterations;
        let known = match to {
            Some(end) => match (
                const_value(from, &self.consts),
                const_value(end, &self.consts),
            ) {
                (Some(a), Some(b)) => {
                    Some(u64::try_from((i128::from(b) - i128::from(a)).max(0)).unwrap_or(u64::MAX))
                }
                _ => None,
            },
            None => match &from.kind {
                ExprKind::List(items) => Some(items.len() as u64),
                _ => None,
            },
        };
        match known {
            Some(n) => n.min(limit),
            None => {
                self.bounded = false;
                limit
            }
        }
    }

    fn stmt(&mut self, s: &Stmt) -> u64 {
        match &s.kind {
            StmtKind::Let(_, e) | StmtKind::Assign(_, e) | StmtKind::Expr(e) => {
                self.expr(e).saturating_add(1)
            }
            StmtKind::Break | StmtKind::Continue => 1,
            StmtKind::If(arms, otherwise) => {
                let mut conds = 0u64;
                let mut widest = 0u64;
                for (cond, body) in arms {
                    conds = conds.saturating_add(self.expr(cond));
                    widest = widest.max(self.block(body));
                }
                if let Some(body) = otherwise {
                    widest = widest.max(self.block(body));
                }
                1u64.saturating_add(conds).saturating_add(widest)
            }
            StmtKind::For { from, to, body, .. } => {
                let turns = self.turns(from, to.as_ref());
                let head = self
                    .expr(from)
                    .saturating_add(to.as_ref().map_or(0, |end| self.expr(end)));
                let each = self.block(body).saturating_add(1);
                1u64.saturating_add(head)
                    .saturating_add(turns.saturating_mul(each))
            }
            StmtKind::While(cond, body) => {
                self.bounded = false;
                let turns = self.settings.max_loop_iterations;
                let each = self
                    .expr(cond)
                    .saturating_add(self.block(body))
                    .saturating_add(1);
                1u64.saturating_add(self.expr(cond))
                    .saturating_add(turns.saturating_mul(each))
            }
        }
    }
}

/// Checks a script that was read.
pub fn check_program(program: &Program, settings: &ScriptSettings) -> CheckReport {
    let mut walker = Walker {
        target: program.target,
        scopes: vec![BTreeSet::new()],
        errors: Vec::new(),
        calls: 0,
        operations: 0,
        statements: 0,
        usage: BTreeMap::new(),
    };
    for s in &program.body {
        walker.stmt(s);
    }
    let mut defs = BTreeMap::new();
    let mut assigned = BTreeSet::new();
    collect_names(&program.body, &mut defs, &mut assigned);
    let mut consts: BTreeMap<String, i64> = BTreeMap::new();
    for s in &program.body {
        if let StmtKind::Let(name, e) = &s.kind {
            if defs.get(name) == Some(&1) && !assigned.contains(name) {
                if let Some(n) = const_value(e, &consts) {
                    consts.insert(name.clone(), n);
                }
            }
        }
    }
    let mut estimate = Estimate {
        settings,
        consts,
        bounded: true,
    };
    let estimated = estimate.block(&program.body);
    let mut warnings = Vec::new();
    if estimated > settings.max_steps {
        warnings.push(Diag::plain(
            ESTIMATE,
            format!(
                "about {estimated} steps against a limit of {}",
                settings.max_steps
            ),
        ));
    }
    CheckReport {
        errors: walker.errors,
        warnings,
        target: program.target,
        statements: walker.statements,
        calls: walker.calls,
        operations: walker.operations,
        estimated_steps: estimated,
        bounded: estimate.bounded,
        functions: walker.usage.into_iter().collect(),
    }
}

/// Reads and checks the text of a script. A text that cannot be read is a report with that error.
pub fn check_text(text: &str, settings: &ScriptSettings) -> CheckReport {
    match parse_program(text, settings) {
        Ok(program) => check_program(&program, settings),
        Err(d) => CheckReport {
            errors: vec![d],
            warnings: Vec::new(),
            target: None,
            statements: 0,
            calls: 0,
            operations: 0,
            estimated_steps: 0,
            bounded: true,
            functions: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::codes::SYNTAX;

    fn check(body: &str) -> CheckReport {
        check_text(
            &format!("phoenix-script/2 board\n{body}"),
            &ScriptSettings::default(),
        )
    }

    fn codes(r: &CheckReport) -> Vec<&'static str> {
        r.errors.iter().map(|d| d.code).collect()
    }

    #[test]
    fn a_good_script_is_counted() {
        let r = check("let n = 3\nfor i in 0..n {\n  print(i)\n}\nmove_to(\"R1\", 0, 0)");
        assert!(r.ok(), "{:?}", r.errors);
        assert_eq!(r.target, Some(Target::Board));
        assert_eq!(r.statements, 4);
        assert_eq!(r.calls, 2);
        assert_eq!(r.operations, 1);
        assert!(r.bounded);
        assert_eq!(
            r.functions,
            vec![("move_to".to_owned(), 1), ("print".to_owned(), 1)]
        );
    }

    #[test]
    fn the_names_the_functions_and_the_documents_are_checked() {
        assert_eq!(codes(&check("print(x)")), vec![UNDEFINED]);
        assert_eq!(codes(&check("x = 1")), vec![UNDEFINED]);
        assert_eq!(codes(&check("let a = 1\nlet a = 2")), vec![REDEFINED]);
        assert_eq!(codes(&check("nothing(1)")), vec![UNKNOWN_FUNCTION]);
        assert_eq!(codes(&check("len()")), vec![ARGUMENTS]);
        assert_eq!(codes(&check("len(1, 2)")), vec![ARGUMENTS]);
        assert_eq!(
            codes(&check("lib_create(\"package\", \"A\")")),
            vec![TARGET]
        );
        assert_eq!(
            codes(&check("for i in 0..2 { print(i) }\nprint(i)")),
            vec![UNDEFINED]
        );
        let both = check("print(a)\nprint(b)");
        assert_eq!(both.errors.len(), 2);
        assert_eq!(both.errors[1].line, 3);
    }

    #[test]
    fn a_syntax_error_is_the_only_error() {
        let r = check("let = 1");
        assert_eq!(codes(&r), vec![SYNTAX]);
        assert_eq!((r.errors[0].line, r.errors[0].col), (2, 5));
        assert!(!r.ok());
    }

    #[test]
    fn the_estimate_is_exact_for_written_counts_and_a_bound_for_the_rest() {
        let exact = check("let rows = 2\nlet cols = 3\nfor r in 0..rows {\n for c in 0..cols {\n  print(r, c)\n }\n}");
        assert!(exact.bounded);
        // two lets of 2 steps; the outer loop is 1 + head 2 + 2 turns of (inner + 1); the inner loop
        // is 1 + head 2 + 3 turns of (the print of 4 steps + 1)
        let inner = 1 + 2 + 3 * (4 + 1);
        assert_eq!(exact.estimated_steps, 2 + 2 + (1 + 2 + 2 * (inner + 1)));
        assert_eq!(exact.estimated_steps, 45);
        let open = check("for x in select(\"component\", \"*\") { print(x) }");
        assert!(!open.bounded);
        assert!(open.estimated_steps > 10_000);
        assert!(open.warnings.is_empty());
    }

    #[test]
    fn an_estimate_over_the_limit_is_a_warning() {
        let small = ScriptSettings {
            max_steps: 100,
            ..ScriptSettings::default()
        };
        let r = check_text("phoenix-script/2\nwhile true { }", &small);
        assert!(r.ok());
        assert_eq!(r.warnings.len(), 1);
        assert_eq!(r.warnings[0].code, ESTIMATE);
        assert!(!r.bounded);
    }

    #[test]
    fn a_name_that_is_assigned_is_not_a_constant() {
        let r = check("let n = 2\nn = 5\nfor i in 0..n { print(i) }");
        assert!(!r.bounded);
    }

    #[test]
    fn the_report_is_written_as_text_and_json() {
        let r = check("print(missing)");
        let text = r.to_text(Lang::En);
        assert!(text.starts_with("check: 1 error(s)\n"));
        assert!(text.contains("PHOENIX-SCRIPT-045: Variable is not defined [2:7]: missing"));
        let pt = r.to_text(Lang::Pt);
        assert!(pt.contains("A variável não está definida"));
        let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(json["schema"], "phoenix-script-check/2");
        assert_eq!(json["ok"], false);
        assert_eq!(json["errors"][0]["code"], "PHOENIX-SCRIPT-045");
        assert_eq!(json["errors"][0]["line"], 2);
        assert_eq!(json["target"], "board");
    }
}
