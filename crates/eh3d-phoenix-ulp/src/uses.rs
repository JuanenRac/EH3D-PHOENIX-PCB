// SPDX-License-Identifier: GPL-3.0-or-later
//! What a program uses: its directives, the kinds of statement, the types, the functions it calls,
//! the members of objects it reads and the named values it names, each with how many times and
//! where it first appears. A static pass over the syntax tree; nothing is run.
//!
//! A name the program defines itself (a function, a variable, a parameter, the variable of a walk
//! over the objects) is not a use of the language and is left out.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Decl, Expr, ExprKind, Init, Pos, Program, Stmt, StmtKind};
use crate::support::UseKind;

/// One name used, with its count and the place of its first appearance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// What kind of name.
    pub kind: UseKind,
    /// The name.
    pub name: String,
    /// How many times it appears.
    pub count: usize,
    /// Where it first appears.
    pub first: Pos,
}

#[derive(Default)]
struct Collector {
    seen: BTreeMap<(UseKind, String), (usize, Pos)>,
    defined: BTreeSet<String>,
    functions: BTreeSet<String>,
}

/// True for a name written as a named value: capitals, digits and underscores.
fn looks_like_constant(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && name.len() > 1
}

impl Collector {
    fn note(&mut self, kind: UseKind, name: &str, pos: Pos) {
        let slot = self.seen.entry((kind, name.to_owned())).or_insert((0, pos));
        slot.0 += 1;
        if pos < slot.1 {
            slot.1 = pos;
        }
    }

    fn collect_definitions(&mut self, program: &Program) {
        for f in &program.functions {
            self.functions.insert(f.name.clone());
            for p in &f.params {
                self.defined.insert(p.name.clone());
            }
            self.defined_in_list(&f.body);
        }
        self.defined_in_list(&program.body);
    }

    fn defined_in_list(&mut self, list: &[Stmt]) {
        for s in list {
            self.defined_in(s);
        }
    }

    fn defined_in(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Decl(d) => self.defined_decl(d),
            StmtKind::Loop { call, body } => {
                self.defined_by_walk(call);
                self.defined_in_list(body);
            }
            StmtKind::Block(body) | StmtKind::Switch { body, .. } => self.defined_in_list(body),
            StmtKind::If {
                then, otherwise, ..
            } => {
                self.defined_in(then);
                if let Some(o) = otherwise {
                    self.defined_in(o);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Do { body, .. } => self.defined_in(body),
            StmtKind::For { init, body, .. } => {
                if let Some(i) = init {
                    self.defined_in(i);
                }
                self.defined_in(body);
            }
            _ => {}
        }
    }

    /// The variables of a walk are declared by the walk itself.
    fn defined_by_walk(&mut self, call: &Expr) {
        let call = match &call.kind {
            ExprKind::Assign { value, .. } => value,
            _ => call,
        };
        if let ExprKind::Call { args, .. } = &call.kind {
            for a in args {
                if let ExprKind::Name(n) = &a.kind {
                    self.defined.insert(n.clone());
                }
            }
        }
    }

    fn defined_decl(&mut self, d: &Decl) {
        for n in &d.names {
            self.defined.insert(n.name.clone());
            if let Some(Init::Block { call, body }) = &n.init {
                self.defined_by_walk(call);
                self.defined_in_list(body);
            }
        }
    }

    fn type_use(&mut self, ty: &str, pos: Pos) {
        if ty != "void" {
            self.note(UseKind::Type, ty, pos);
        }
    }

    fn decl(&mut self, d: &Decl, pos: Pos) {
        self.type_use(&d.ty, pos);
        for n in &d.names {
            if let Some(size) = &n.size {
                self.expr(size, false);
            }
            match &n.init {
                Some(Init::Value(e)) => self.expr(e, false),
                Some(Init::List(items)) => items.iter().for_each(|e| self.expr(e, false)),
                Some(Init::Block { call, body }) => {
                    self.note(UseKind::Statement, "loop", call.pos);
                    self.expr(call, false);
                    self.list(body);
                }
                None => {}
            }
        }
    }

    fn list(&mut self, list: &[Stmt]) {
        for s in list {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        let pos = stmt.pos;
        let word = match &stmt.kind {
            StmtKind::Empty => None,
            StmtKind::Decl(_) => Some("declaration"),
            StmtKind::Expr(_) => Some("expression"),
            StmtKind::Loop { .. } => Some("loop"),
            StmtKind::Block(_) => Some("block"),
            StmtKind::If { .. } => Some("if"),
            StmtKind::While { .. } => Some("while"),
            StmtKind::Do { .. } => Some("do"),
            StmtKind::For { .. } => Some("for"),
            StmtKind::Switch { .. } => Some("switch"),
            StmtKind::Case(_) => Some("case"),
            StmtKind::Default => Some("default"),
            StmtKind::Break => Some("break"),
            StmtKind::Continue => Some("continue"),
            StmtKind::Return(_) => Some("return"),
        };
        if let Some(w) = word {
            self.note(UseKind::Statement, w, pos);
        }
        match &stmt.kind {
            StmtKind::Decl(d) => self.decl(d, pos),
            StmtKind::Expr(e) | StmtKind::Case(e) => self.expr(e, false),
            StmtKind::Loop { call, body } => {
                self.expr(call, false);
                self.list(body);
            }
            StmtKind::Block(body) => self.list(body),
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond, false);
                self.stmt(then);
                if let Some(o) = otherwise {
                    self.stmt(o);
                }
            }
            StmtKind::While { cond, body } | StmtKind::Do { body, cond } => {
                self.expr(cond, false);
                self.stmt(body);
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                if let Some(i) = init {
                    self.stmt(i);
                }
                if let Some(c) = cond {
                    self.expr(c, false);
                }
                if let Some(s) = step {
                    self.expr(s, false);
                }
                self.stmt(body);
            }
            StmtKind::Switch { value, body } => {
                self.expr(value, false);
                self.list(body);
            }
            StmtKind::Return(Some(e)) => self.expr(e, false),
            _ => {}
        }
    }

    fn expr(&mut self, expr: &Expr, called: bool) {
        let pos = expr.pos;
        match &expr.kind {
            ExprKind::Int(_) | ExprKind::Real(_) | ExprKind::Char(_) | ExprKind::Str(_) => {}
            ExprKind::Name(name) => {
                if called {
                    if !self.functions.contains(name) {
                        self.note(UseKind::Function, name, pos);
                    }
                } else if !self.defined.contains(name) && !self.functions.contains(name) {
                    self.note_value(name, pos);
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee, true);
                args.iter().for_each(|a| self.expr(a, false));
            }
            ExprKind::Index { base, index } => {
                self.expr(base, false);
                self.expr(index, false);
            }
            ExprKind::Member { base, name } => {
                self.expr(base, false);
                self.note(UseKind::Member, name, pos);
            }
            ExprKind::Unary { operand, .. } | ExprKind::Postfix { operand, .. } => {
                self.expr(operand, false);
            }
            ExprKind::Binary { left, right, .. } => {
                self.expr(left, false);
                self.expr(right, false);
            }
            ExprKind::Assign { target, value, .. } => {
                self.expr(target, false);
                self.expr(value, false);
            }
            ExprKind::Cond {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond, false);
                self.expr(then, false);
                self.expr(otherwise, false);
            }
        }
    }

    /// A name that is not called and that the program did not define: a named value when it is
    /// written like one (`UL_WIRE`, `PI`) or is one of the two lower-case values (`argc`, `argv`).
    fn note_value(&mut self, name: &str, pos: Pos) {
        if looks_like_constant(name) || name == "argc" || name == "argv" {
            self.note(UseKind::Constant, name, pos);
        }
    }
}

/// Everything a program uses, in the order of a report: by kind and then by name.
pub fn collect(program: &Program) -> Vec<Seen> {
    let mut c = Collector::default();
    c.collect_definitions(program);
    for d in &program.directives {
        c.note(UseKind::Directive, &d.name, d.pos);
    }
    for f in &program.functions {
        c.note(UseKind::Statement, "function", f.pos);
        c.type_use(&f.ret, f.pos);
        for p in &f.params {
            c.type_use(&p.ty, f.pos);
        }
        c.list(&f.body);
    }
    c.list(&program.body);
    c.seen
        .into_iter()
        .map(|((kind, name), (count, first))| Seen {
            kind,
            name,
            count,
            first,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::settings::UlpSettings;

    fn seen(text: &str) -> Vec<Seen> {
        let parsed = parse(text, &UlpSettings::default()).unwrap();
        assert!(
            parsed.problems.list.is_empty(),
            "{:?}",
            parsed.problems.list
        );
        collect(&parsed.program)
    }

    fn find<'a>(list: &'a [Seen], kind: UseKind, name: &str) -> Option<&'a Seen> {
        list.iter().find(|s| s.kind == kind && s.name == name)
    }

    #[test]
    fn the_functions_the_members_and_the_types_are_counted_with_their_first_place() {
        let list = seen("string s;\nboard(B) {\n  B.elements(E) {\n    s = sprintf(\"%s\", E.name);\n    s = strupr(s);\n    t = sprintf(\"%d\", E.x);\n  }\n}\n");
        let sp = find(&list, UseKind::Function, "sprintf").unwrap();
        assert_eq!((sp.count, sp.first.line, sp.first.col), (2, 4, 9));
        assert_eq!(find(&list, UseKind::Function, "strupr").unwrap().count, 1);
        assert_eq!(find(&list, UseKind::Function, "board").unwrap().count, 1);
        assert_eq!(find(&list, UseKind::Member, "elements").unwrap().count, 1);
        assert_eq!(find(&list, UseKind::Member, "name").unwrap().count, 1);
        assert_eq!(find(&list, UseKind::Type, "string").unwrap().count, 1);
        assert_eq!(find(&list, UseKind::Statement, "loop").unwrap().count, 2);
    }

    #[test]
    fn a_name_the_program_defines_is_not_a_use_of_the_language() {
        let list = seen(
            "int twice(int v) { return v * 2; }\nint n = twice(4);\nboard(B) { n = B.name; }\n",
        );
        assert!(find(&list, UseKind::Function, "twice").is_none());
        assert!(find(&list, UseKind::Function, "board").is_some());
        assert!(find(&list, UseKind::Statement, "function").is_some());
        assert!(find(&list, UseKind::Statement, "return").is_some());
    }

    #[test]
    fn the_named_values_are_found_and_the_variables_are_not() {
        let list = seen("int a = UL_WIRE;\nreal p = PI;\nint c = argc;\nint k = other;\nint m = LOCAL;\nint LOCAL2 = 1;\nint n = LOCAL2;\n");
        for name in ["UL_WIRE", "PI", "argc", "LOCAL"] {
            assert!(find(&list, UseKind::Constant, name).is_some(), "{name}");
        }
        assert!(find(&list, UseKind::Constant, "other").is_none());
        assert!(find(&list, UseKind::Constant, "LOCAL2").is_none());
    }

    #[test]
    fn the_directives_and_the_object_types_are_listed() {
        let list = seen("#usage \"x\"\n#include \"a.ulp\"\nUL_ELEMENT e;\nint i;\n");
        assert!(find(&list, UseKind::Directive, "usage").is_some());
        assert!(find(&list, UseKind::Directive, "include").is_some());
        assert!(find(&list, UseKind::Type, "UL_ELEMENT").is_some());
        assert!(find(&list, UseKind::Type, "int").is_some());
    }

    #[test]
    fn the_order_is_by_kind_and_then_by_name_and_does_not_change() {
        let a = seen("x = strupr(strlwr(s)); y = abs(1);");
        let names: Vec<&str> = a
            .iter()
            .filter(|s| s.kind == UseKind::Function)
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(names, ["abs", "strlwr", "strupr"]);
        assert_eq!(a, seen("x = strupr(strlwr(s)); y = abs(1);"));
    }

    #[test]
    fn a_program_with_nothing_uses_nothing() {
        assert!(seen("").is_empty());
        assert!(seen("// only a comment").is_empty());
    }
}
