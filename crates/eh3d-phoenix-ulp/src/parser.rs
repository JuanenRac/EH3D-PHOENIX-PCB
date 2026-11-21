// SPDX-License-Identifier: GPL-3.0-or-later
//! The parser: the tokens of a program made into a syntax tree by recursive descent.
//!
//! It is bounded: the nesting of blocks, parentheses and expressions stops at `max_depth` (the
//! parser calls itself once for each level), the chains of operators and of `.`, `[]` and `()` at
//! eight times that, and the statements at `max_statements`; a program that goes beyond a limit is
//! an error and nothing of it is kept. A mistake in the shape of the program is a problem with its
//! line and column, and the parser goes on from the next `;` or `}`, so one reading shows several
//! problems. Nothing is run and no name is looked up.

use crate::ast::{
    Decl, Declarator, Directive, Expr, ExprKind, Func, Init, Param, Pos, Program, Stmt, StmtKind,
};
use crate::codes::{
    Diag, DECLARATION_INVALID, DIRECTIVE_INVALID, DIRECTIVE_UNKNOWN, EXPECTED, TOO_DEEP,
    TOO_MANY_STATEMENTS, UNEXPECTED,
};
use crate::lexer::{lex, Kind, Problems, Token};
use crate::settings::UlpSettings;

/// A program read: the tree and the problems found on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// What could be read of the program; a part with a problem is left out.
    pub program: Program,
    /// The problems of the characters and of the shape, in the order of the program.
    pub problems: Problems,
}

/// The words that have a meaning of their own and cannot be used as names.
pub const KEYWORDS: [&str; 15] = [
    "break", "case", "char", "continue", "default", "do", "else", "for", "if", "int", "real",
    "return", "string", "switch", "while",
];

const ASSIGN: [&str; 11] = [
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=",
];

/// Reads a program. Only a program that is too large, has too many tokens or statements or is
/// nested too deep is an error; every other problem is kept in the result.
pub fn parse(source: &str, settings: &UlpSettings) -> Result<Parsed, Diag> {
    let lexed = lex(source, settings)?;
    let mut parser = Parser {
        tokens: lexed.tokens,
        at: 0,
        depth: 0,
        statements: 0,
        problems: lexed.problems,
        settings: settings.clone(),
    };
    let program = parser.program()?;
    // the problems of the characters come first from the lexer: put all of them in the order of
    // the program
    parser.problems.list.sort_by_key(|d| (d.line, d.col));
    Ok(Parsed {
        program,
        problems: parser.problems,
    })
}

/// Why a rule of the parser stopped: a problem that was kept (the caller skips ahead), or an
/// error that ends the reading.
enum Stop {
    Recover,
    Fatal(Diag),
}

type Step<T> = Result<T, Stop>;

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    depth: usize,
    statements: usize,
    problems: Problems,
    settings: UlpSettings,
}

fn prec(op: &str) -> Option<u8> {
    Some(match op {
        "||" => 1,
        "&&" => 2,
        "|" => 3,
        "^" => 4,
        "&" => 5,
        "==" | "!=" => 6,
        "<" | ">" | "<=" | ">=" => 7,
        "<<" | ">>" => 8,
        "+" | "-" => 9,
        "*" | "/" | "%" => 10,
        _ => return None,
    })
}

/// True for what may be followed by a block: a call (`board(B) {`), a name that stands for a
/// layout of a dialog (`dlgHBoxLayout {`), or the assignment of a call (`n = dlgDialog("t") {`).
fn opens_a_block(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Call { .. } | ExprKind::Name(_) => true,
        ExprKind::Assign { value, .. } => matches!(value.kind, ExprKind::Call { .. }),
        _ => false,
    }
}

fn is_basic_type(name: &str) -> bool {
    matches!(name, "char" | "int" | "real" | "string")
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn peek_at(&self, ahead: usize) -> &Token {
        &self.tokens[(self.at + ahead).min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if token.kind != Kind::Eof {
            self.at += 1;
        }
        token
    }

    fn pos(&self) -> Pos {
        let t = self.peek();
        Pos {
            line: t.line,
            col: t.col,
        }
    }

    fn at_end(&self) -> bool {
        self.peek().kind == Kind::Eof
    }

    /// Keeps a problem and asks the caller to skip ahead.
    fn fail<T>(&mut self, code: &'static str, at: &Token, detail: String) -> Step<T> {
        self.problems.push(Diag::at(code, at.line, at.col, detail));
        Err(Stop::Recover)
    }

    fn unexpected<T>(&mut self) -> Step<T> {
        let t = self.peek().clone();
        let detail = t.describe();
        self.fail(UNEXPECTED, &t, detail)
    }

    fn expect(&mut self, mark: &str) -> Step<Token> {
        if self.peek().is(mark) {
            return Ok(self.advance());
        }
        let t = self.peek().clone();
        let detail = format!("`{mark}` is needed, found {}", t.describe());
        self.fail(EXPECTED, &t, detail)
    }

    fn expect_name(&mut self, what: &str) -> Step<Token> {
        let t = self.peek().clone();
        if t.kind == Kind::Ident && !KEYWORDS.contains(&t.text.as_str()) {
            return Ok(self.advance());
        }
        let detail = format!("{what} is needed, found {}", t.describe());
        self.fail(EXPECTED, &t, detail)
    }

    fn enter(&mut self) -> Step<()> {
        self.depth += 1;
        if self.depth > self.settings.max_depth {
            let t = self.peek().clone();
            return Err(Stop::Fatal(Diag::at(
                TOO_DEEP,
                t.line,
                t.col,
                format!("the limit is {}", self.settings.max_depth),
            )));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// The longest chain of operators, calls, members or commas in one expression.
    fn chain_limit(&self) -> usize {
        self.settings.max_depth * 8
    }

    fn chain_check(&self, links: usize) -> Step<()> {
        if links > self.chain_limit() {
            let t = self.peek();
            return Err(Stop::Fatal(Diag::at(
                TOO_DEEP,
                t.line,
                t.col,
                format!("a chain of more than {} links", self.chain_limit()),
            )));
        }
        Ok(())
    }

    /// Skips to the end of what went wrong: after the next `;`, or before the `}` that closes the
    /// block that is being read. At least one token is taken unless the program is over.
    fn synchronize(&mut self, in_block: bool) {
        let start = self.at;
        let mut open = 0usize;
        while !self.at_end() {
            let t = self.peek();
            if t.is(";") && open == 0 {
                self.advance();
                break;
            }
            if t.is("{") {
                open += 1;
            } else if t.is("}") {
                if open == 0 {
                    if in_block {
                        break;
                    }
                    self.advance();
                    break;
                }
                open -= 1;
            }
            self.advance();
        }
        if self.at == start && !self.at_end() {
            self.advance();
        }
    }

    fn count_statement(&mut self) -> Step<()> {
        self.statements += 1;
        if self.statements > self.settings.max_statements {
            let t = self.peek();
            return Err(Stop::Fatal(Diag::at(
                TOO_MANY_STATEMENTS,
                t.line,
                t.col,
                format!("the limit is {}", self.settings.max_statements),
            )));
        }
        Ok(())
    }

    fn program(&mut self) -> Result<Program, Diag> {
        let mut program = Program::default();
        while !self.at_end() {
            let (before, depth) = (self.at, self.depth);
            let step = self.top_level(&mut program);
            match step {
                Ok(()) => {}
                Err(Stop::Recover) => {
                    self.depth = depth;
                    self.synchronize(false);
                }
                Err(Stop::Fatal(d)) => return Err(d),
            }
            if self.at == before && !self.at_end() {
                self.advance();
            }
        }
        Ok(program)
    }

    fn top_level(&mut self, program: &mut Program) -> Step<()> {
        let t = self.peek().clone();
        if t.kind == Kind::Directive {
            if let Some(d) = self.directive() {
                program.directives.push(d);
            }
            return Ok(());
        }
        if self.starts_declaration()
            && self.peek_at(1).kind == Kind::Ident
            && self.peek_at(2).is("(")
        {
            let f = self.function()?;
            program.functions.push(f);
            return Ok(());
        }
        let s = self.statement()?;
        program.body.push(s);
        Ok(())
    }

    fn directive(&mut self) -> Option<Directive> {
        let t = self.advance();
        let pos = Pos {
            line: t.line,
            col: t.col,
        };
        let mut args = Vec::new();
        match t.text.as_str() {
            "include" => {
                if self.peek().kind == Kind::Str {
                    args.push(self.advance().text);
                } else {
                    self.problems.push(Diag::at(
                        DIRECTIVE_INVALID,
                        t.line,
                        t.col,
                        "`#include` needs the name of a file in quotes or in angle brackets",
                    ));
                }
            }
            "usage" => loop {
                if self.peek().kind != Kind::Str {
                    let at = self.peek().clone();
                    self.problems.push(Diag::at(
                        DIRECTIVE_INVALID,
                        at.line,
                        at.col,
                        "`#usage` needs texts in quotes",
                    ));
                    break;
                }
                args.push(self.advance().text);
                if self.peek().is(",") {
                    self.advance();
                } else {
                    break;
                }
            },
            "require" => {
                if matches!(self.peek().kind, Kind::Int | Kind::Real) {
                    args.push(self.advance().text);
                } else {
                    self.problems.push(Diag::at(
                        DIRECTIVE_INVALID,
                        t.line,
                        t.col,
                        "`#require` needs a version number",
                    ));
                }
            }
            other => {
                self.problems.push(Diag::at(
                    DIRECTIVE_UNKNOWN,
                    t.line,
                    t.col,
                    format!("`#{other}`"),
                ));
                // the rest of the line belongs to the directive
                while !self.at_end() && self.peek().line == t.line {
                    self.advance();
                }
                return None;
            }
        }
        Some(Directive {
            name: t.text,
            args,
            pos,
        })
    }

    /// True when the next token begins a declaration: a basic type, or a name that starts with
    /// `UL_` followed by a name.
    fn starts_declaration(&self) -> bool {
        let t = self.peek();
        if t.kind != Kind::Ident {
            return false;
        }
        if is_basic_type(&t.text) || t.text == "void" {
            return true;
        }
        t.text.starts_with("UL_") && self.peek_at(1).kind == Kind::Ident
    }

    fn function(&mut self) -> Step<Func> {
        self.count_statement()?;
        let pos = self.pos();
        let ret = self.advance().text;
        let name = self.advance().text;
        self.expect("(")?;
        let mut params = Vec::new();
        if !self.peek().is(")") {
            loop {
                if !self.starts_declaration() {
                    let t = self.peek().clone();
                    let detail = format!("a type is needed, found {}", t.describe());
                    return self.fail(DECLARATION_INVALID, &t, detail);
                }
                let ty = self.advance().text;
                let n = self.expect_name("the name of the parameter")?;
                let mut array = false;
                if self.peek().is("[") {
                    self.advance();
                    self.expect("]")?;
                    array = true;
                }
                params.push(Param {
                    ty,
                    name: n.text,
                    array,
                });
                if self.peek().is(",") {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(")")?;
        let body = self.block()?;
        Ok(Func {
            ret,
            name,
            params,
            body,
            pos,
        })
    }

    /// `{ statements }`; a statement with a problem is skipped and the block goes on.
    fn block(&mut self) -> Step<Vec<Stmt>> {
        self.expect("{")?;
        let mut list = Vec::new();
        loop {
            if self.peek().is("}") {
                self.advance();
                break;
            }
            if self.at_end() {
                let t = self.peek().clone();
                self.problems.push(Diag::at(
                    EXPECTED,
                    t.line,
                    t.col,
                    "`}` is needed before the end of the program",
                ));
                break;
            }
            let (before, depth) = (self.at, self.depth);
            match self.statement() {
                Ok(s) => list.push(s),
                Err(Stop::Recover) => {
                    self.depth = depth;
                    self.synchronize(true);
                }
                Err(Stop::Fatal(d)) => return Err(Stop::Fatal(d)),
            }
            if self.at == before && !self.at_end() {
                self.advance();
            }
        }
        Ok(list)
    }

    fn statement(&mut self) -> Step<Stmt> {
        self.count_statement()?;
        self.enter()?;
        let r = self.statement_inner();
        self.leave();
        r
    }

    fn statement_inner(&mut self) -> Step<Stmt> {
        let pos = self.pos();
        let t = self.peek().clone();
        let kind = if t.is(";") {
            self.advance();
            StmtKind::Empty
        } else if t.is("{") {
            StmtKind::Block(self.block()?)
        } else if t.kind == Kind::Ident {
            match t.text.as_str() {
                "if" => self.if_statement()?,
                "while" => {
                    self.advance();
                    let cond = self.condition()?;
                    let body = Box::new(self.statement()?);
                    StmtKind::While { cond, body }
                }
                "do" => {
                    self.advance();
                    let body = Box::new(self.statement()?);
                    if !self.peek().is_word("while") {
                        let at = self.peek().clone();
                        let detail = format!("`while` is needed, found {}", at.describe());
                        return self.fail(EXPECTED, &at, detail);
                    }
                    self.advance();
                    let cond = self.condition()?;
                    self.expect(";")?;
                    StmtKind::Do { body, cond }
                }
                "for" => self.for_statement()?,
                "switch" => {
                    self.advance();
                    let value = self.condition()?;
                    let body = self.block()?;
                    StmtKind::Switch { value, body }
                }
                "case" => {
                    self.advance();
                    let value = self.conditional()?;
                    self.expect(":")?;
                    StmtKind::Case(value)
                }
                "default" => {
                    self.advance();
                    self.expect(":")?;
                    StmtKind::Default
                }
                "break" => {
                    self.advance();
                    self.expect(";")?;
                    StmtKind::Break
                }
                "continue" => {
                    self.advance();
                    self.expect(";")?;
                    StmtKind::Continue
                }
                "return" => {
                    self.advance();
                    let value = if self.peek().is(";") {
                        None
                    } else {
                        Some(self.expression()?)
                    };
                    self.expect(";")?;
                    StmtKind::Return(value)
                }
                "else" => return self.unexpected(),
                _ if self.starts_declaration() => StmtKind::Decl(self.declaration()?),
                _ => self.expression_statement()?,
            }
        } else {
            self.expression_statement()?
        };
        Ok(Stmt { kind, pos })
    }

    fn condition(&mut self) -> Step<Expr> {
        self.expect("(")?;
        let e = self.expression()?;
        self.expect(")")?;
        Ok(e)
    }

    fn if_statement(&mut self) -> Step<StmtKind> {
        self.advance();
        let cond = self.condition()?;
        let then = Box::new(self.statement()?);
        let otherwise = if self.peek().is_word("else") {
            self.advance();
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        Ok(StmtKind::If {
            cond,
            then,
            otherwise,
        })
    }

    fn for_statement(&mut self) -> Step<StmtKind> {
        self.advance();
        self.expect("(")?;
        let init = if self.peek().is(";") {
            self.advance();
            None
        } else if self.starts_declaration() {
            let pos = self.pos();
            let d = self.declaration()?;
            Some(Box::new(Stmt {
                kind: StmtKind::Decl(d),
                pos,
            }))
        } else {
            let pos = self.pos();
            let e = self.expression()?;
            self.expect(";")?;
            Some(Box::new(Stmt {
                kind: StmtKind::Expr(e),
                pos,
            }))
        };
        let cond = if self.peek().is(";") {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(";")?;
        let step = if self.peek().is(")") {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(")")?;
        let body = Box::new(self.statement()?);
        Ok(StmtKind::For {
            init,
            cond,
            step,
            body,
        })
    }

    /// `type name, name[] = {...}, name = value;`
    fn declaration(&mut self) -> Step<Decl> {
        let ty = self.advance().text;
        let mut names = Vec::new();
        loop {
            let pos = self.pos();
            let name = self.expect_name("the name that is declared")?;
            let mut array = false;
            let mut size = None;
            if self.peek().is("[") {
                self.advance();
                array = true;
                if !self.peek().is("]") {
                    size = Some(self.expression()?);
                }
                self.expect("]")?;
            }
            let mut init = None;
            if self.peek().is("=") {
                self.advance();
                if self.peek().is("{") {
                    self.advance();
                    self.enter()?;
                    let mut items = Vec::new();
                    while !self.peek().is("}") {
                        items.push(self.assignment()?);
                        self.chain_check(items.len())?;
                        if self.peek().is(",") {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.leave();
                    self.expect("}")?;
                    init = Some(Init::List(items));
                } else {
                    let value = self.assignment()?;
                    if self.peek().is("{") && matches!(value.kind, ExprKind::Call { .. }) {
                        let body = self.block()?;
                        init = Some(Init::Block { call: value, body });
                    } else {
                        init = Some(Init::Value(value));
                    }
                }
            }
            names.push(Declarator {
                name: name.text,
                array,
                size,
                init,
                pos,
            });
            self.chain_check(names.len())?;
            if self.peek().is(",") {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(";")?;
        Ok(Decl { ty, names })
    }

    /// An expression and `;`, or a call and the block that follows it.
    fn expression_statement(&mut self) -> Step<StmtKind> {
        let e = self.expression()?;
        if self.peek().is("{") && opens_a_block(&e) {
            let body = self.block()?;
            if self.peek().is(";") {
                self.advance();
            }
            return Ok(StmtKind::Loop { call: e, body });
        }
        self.expect(";")?;
        Ok(StmtKind::Expr(e))
    }

    fn expression(&mut self) -> Step<Expr> {
        let mut left = self.assignment()?;
        let mut links = 0usize;
        while self.peek().is(",") {
            let pos = left.pos;
            self.advance();
            let right = self.assignment()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op: ",".to_owned(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
                pos,
            };
            links += 1;
            self.chain_check(links)?;
        }
        Ok(left)
    }

    fn assignment(&mut self) -> Step<Expr> {
        self.enter()?;
        let r = self.assignment_inner();
        self.leave();
        r
    }

    fn assignment_inner(&mut self) -> Step<Expr> {
        let target = self.conditional()?;
        let t = self.peek();
        if t.kind == Kind::Punct && ASSIGN.contains(&t.text.as_str()) {
            let op = self.advance().text;
            let value = self.assignment()?;
            let pos = target.pos;
            return Ok(Expr {
                kind: ExprKind::Assign {
                    op,
                    target: Box::new(target),
                    value: Box::new(value),
                },
                pos,
            });
        }
        Ok(target)
    }

    fn conditional(&mut self) -> Step<Expr> {
        let cond = self.binary(1)?;
        if !self.peek().is("?") {
            return Ok(cond);
        }
        self.advance();
        let then = self.assignment()?;
        self.expect(":")?;
        let otherwise = self.assignment()?;
        let pos = cond.pos;
        Ok(Expr {
            kind: ExprKind::Cond {
                cond: Box::new(cond),
                then: Box::new(then),
                otherwise: Box::new(otherwise),
            },
            pos,
        })
    }

    fn binary(&mut self, min: u8) -> Step<Expr> {
        let mut left = self.unary()?;
        let mut links = 0usize;
        loop {
            let t = self.peek();
            let Some(p) = (t.kind == Kind::Punct).then(|| prec(&t.text)).flatten() else {
                break;
            };
            if p < min {
                break;
            }
            let op = self.advance().text;
            let right = self.binary(p + 1)?;
            let pos = left.pos;
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                pos,
            };
            links += 1;
            self.chain_check(links)?;
        }
        Ok(left)
    }

    fn unary(&mut self) -> Step<Expr> {
        let t = self.peek().clone();
        if t.kind == Kind::Punct && matches!(t.text.as_str(), "-" | "+" | "!" | "~" | "++" | "--") {
            self.advance();
            self.enter()?;
            let operand = self.unary();
            self.leave();
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: t.text,
                    operand: Box::new(operand?),
                },
                pos: Pos {
                    line: t.line,
                    col: t.col,
                },
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Step<Expr> {
        let mut e = self.primary()?;
        let mut links = 0usize;
        loop {
            let t = self.peek().clone();
            let pos = e.pos;
            if t.is("(") {
                if !matches!(e.kind, ExprKind::Name(_) | ExprKind::Member { .. }) {
                    return self.unexpected();
                }
                self.advance();
                self.enter()?;
                let mut args = Vec::new();
                if !self.peek().is(")") {
                    loop {
                        args.push(self.assignment()?);
                        self.chain_check(args.len())?;
                        if self.peek().is(",") {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.leave();
                self.expect(")")?;
                e = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(e),
                        args,
                    },
                    pos,
                };
            } else if t.is("[") {
                self.advance();
                self.enter()?;
                let index = self.expression();
                self.leave();
                let index = index?;
                self.expect("]")?;
                e = Expr {
                    kind: ExprKind::Index {
                        base: Box::new(e),
                        index: Box::new(index),
                    },
                    pos,
                };
            } else if t.is(".") {
                self.advance();
                let name = self.expect_name("the name of a member")?;
                e = Expr {
                    kind: ExprKind::Member {
                        base: Box::new(e),
                        name: name.text,
                    },
                    pos,
                };
            } else if t.is("++") || t.is("--") {
                self.advance();
                e = Expr {
                    kind: ExprKind::Postfix {
                        op: t.text,
                        operand: Box::new(e),
                    },
                    pos,
                };
            } else {
                break;
            }
            links += 1;
            self.chain_check(links)?;
        }
        Ok(e)
    }

    fn primary(&mut self) -> Step<Expr> {
        let t = self.peek().clone();
        let pos = Pos {
            line: t.line,
            col: t.col,
        };
        let kind = match t.kind {
            Kind::Int => {
                self.advance();
                ExprKind::Int(t.text)
            }
            Kind::Real => {
                self.advance();
                ExprKind::Real(t.text)
            }
            Kind::Char => {
                self.advance();
                ExprKind::Char(t.text)
            }
            Kind::Str => {
                self.advance();
                let mut text = t.text;
                while self.peek().kind == Kind::Str {
                    text.push_str(&self.advance().text);
                }
                ExprKind::Str(text)
            }
            Kind::Ident if !KEYWORDS.contains(&t.text.as_str()) => {
                self.advance();
                ExprKind::Name(t.text)
            }
            Kind::Punct if t.text == "(" => {
                self.advance();
                self.enter()?;
                let inner = self.expression();
                self.leave();
                let inner = inner?;
                self.expect(")")?;
                return Ok(inner);
            }
            _ => return self.unexpected(),
        };
        Ok(Expr { kind, pos })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{
        CHAR_INVALID, COMMENT_OPEN, DIRECTIVE_INVALID, NUMBER_INVALID, SOURCE_TOO_LARGE,
        STRING_OPEN, TOO_MANY_TOKENS,
    };

    fn ok(text: &str) -> Program {
        let parsed = parse(text, &UlpSettings::default()).unwrap();
        assert!(
            parsed.problems.list.is_empty(),
            "{text}: {:?}",
            parsed.problems.list
        );
        parsed.program
    }

    fn problems(text: &str) -> Vec<Diag> {
        parse(text, &UlpSettings::default()).unwrap().problems.list
    }

    fn fatal(text: &str, settings: &UlpSettings) -> Diag {
        parse(text, settings).unwrap_err()
    }

    #[test]
    fn a_small_program_is_read_whole() {
        let p = ok("#usage \"Lists the parts\"\n\
                    #require 6.0\n\
                    int n = 0;\n\
                    real total = 0.0;\n\
                    string names[];\n\
                    int twice(int v) { return v * 2; }\n\
                    board(B) {\n\
                      B.elements(E) {\n\
                        n++;\n\
                        total += E.x / 1000.0;\n\
                      }\n\
                    }\n\
                    output(\"parts.txt\", \"wt\") {\n\
                      printf(\"%d parts\\n\", twice(n));\n\
                    }\n");
        assert_eq!(p.directives.len(), 2);
        assert_eq!(p.directives[0].name, "usage");
        assert_eq!(p.directives[1].args, ["6.0"]);
        assert_eq!(p.functions.len(), 1);
        assert_eq!(p.functions[0].name, "twice");
        assert_eq!(p.body.len(), 5);
        assert!(matches!(p.body[3].kind, StmtKind::Loop { .. }));
        assert!(matches!(p.body[4].kind, StmtKind::Loop { .. }));
    }

    #[test]
    fn every_kind_of_statement_is_read() {
        let p = ok("int f(int a, real b[]) {\n\
                      int i, j = 1, k[3] = {1, 2, 3};\n\
                      if (a) i = 1; else if (b[0] > 2) i = 2; else { i = 3; }\n\
                      while (i < 10) i++;\n\
                      do { i--; } while (i > 0);\n\
                      for (int n = 0; n < 3; n++) { if (n == 1) continue; if (n == 2) break; }\n\
                      for (;;) break;\n\
                      switch (a) { case 1: i = 1; break; case 2 + 1: i = 3; break; default: i = 0; }\n\
                      ;\n\
                      return i > 0 ? i : -i;\n\
                    }\n");
        assert_eq!(p.functions[0].params.len(), 2);
        assert!(p.functions[0].params[1].array);
        // the count is the function, its statements and the ones inside them
        assert!(p.statement_count() >= 20, "{}", p.statement_count());
    }

    #[test]
    fn expressions_follow_the_precedence_of_c() {
        let p = ok("x = 1 + 2 * 3 == 7 && 4 < 5 || !0;");
        let StmtKind::Expr(e) = &p.body[0].kind else {
            panic!("not an expression");
        };
        let ExprKind::Assign { value, .. } = &e.kind else {
            panic!("not an assignment");
        };
        // the top operator is `||`, the lowest
        assert!(matches!(&value.kind, ExprKind::Binary { op, .. } if op == "||"));
        let q = ok("y = a - b - c;");
        let StmtKind::Expr(e) = &q.body[0].kind else {
            panic!()
        };
        let ExprKind::Assign { value, .. } = &e.kind else {
            panic!()
        };
        // `a - b - c` is `(a - b) - c`
        let ExprKind::Binary { left, .. } = &value.kind else {
            panic!()
        };
        assert!(matches!(&left.kind, ExprKind::Binary { op, .. } if op == "-"));
    }

    #[test]
    fn members_indexes_calls_and_adjacent_texts_are_read() {
        let p = ok("s = P.name + a[i + 1].text + sprintf(\"%d\" \"%s\", n, \"x\");");
        assert_eq!(p.body.len(), 1);
        let q = ok("UL_WIRE w; w.x1 = 0; board(B) { B.wires(W) { w = W; } }");
        assert_eq!(q.body.len(), 3);
    }

    #[test]
    fn a_missing_semicolon_is_a_problem_with_its_place() {
        let d = problems("int a = 1\nint b = 2;");
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].code, d[0].line), (EXPECTED, 2));
    }

    #[test]
    fn the_reading_goes_on_after_a_problem_and_finds_the_next_one() {
        let d = problems("int a = ;\nint b = 2;\nint c = (3;\nint d = 4;\nx = 5 +;\n");
        let lines: Vec<usize> = d.iter().map(|p| p.line).collect();
        assert_eq!(lines, [1, 3, 5]);
        let p = parse("int a = ;\nint b = 2;\n", &UlpSettings::default()).unwrap();
        assert_eq!(p.program.body.len(), 1, "the good declaration is kept");
    }

    #[test]
    fn a_block_that_is_not_closed_is_one_problem() {
        let d = problems("void f() {\n  int a;\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, EXPECTED);
        let stray = problems("int a; }\nint b;");
        assert_eq!(stray.len(), 1);
        assert_eq!(stray[0].code, UNEXPECTED);
    }

    #[test]
    fn an_else_without_an_if_and_a_keyword_used_as_a_name_are_problems() {
        assert_eq!(problems("else x = 1;")[0].code, UNEXPECTED);
        assert_eq!(problems("int while = 1;")[0].code, EXPECTED);
        assert_eq!(problems("x = if;")[0].code, UNEXPECTED);
    }

    #[test]
    fn a_function_inside_a_block_is_a_problem_and_the_declaration_is_kept_apart() {
        let d = problems("void f() { int g(int a) { return a; } }");
        assert!(!d.is_empty());
        let e = problems("int f(a) { return 1; }");
        assert_eq!(e[0].code, DECLARATION_INVALID);
    }

    #[test]
    fn the_directives_are_checked() {
        assert!(
            problems("#include \"x.ulp\"\n#include <y.ulp>\n#usage \"a\", \"b\"\n#require 7")
                .is_empty()
        );
        assert_eq!(problems("#include\nint a;")[0].code, DIRECTIVE_INVALID);
        assert_eq!(problems("#usage 5\n")[0].code, DIRECTIVE_INVALID);
        assert_eq!(problems("#require \"x\"\n")[0].code, DIRECTIVE_INVALID);
        let d = problems("#define A 1\nint b;");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, DIRECTIVE_UNKNOWN);
    }

    #[test]
    fn the_problems_of_the_lexer_come_with_those_of_the_parser() {
        let d = problems("int a = 1 @;\ns = \"open;\n/* never");
        let codes: Vec<&str> = d.iter().map(|p| p.code).collect();
        assert!(codes.contains(&CHAR_INVALID));
        assert!(codes.contains(&STRING_OPEN));
        assert!(codes.contains(&COMMENT_OPEN));
        let n = problems("x = 12abc;");
        assert_eq!(n[0].code, NUMBER_INVALID);
    }

    #[test]
    fn the_nesting_is_bounded() {
        let s = UlpSettings {
            max_depth: 10,
            ..UlpSettings::default()
        };
        let deep_blocks = format!("{}{}", "{".repeat(11), "}".repeat(11));
        assert_eq!(fatal(&deep_blocks, &s).code, TOO_DEEP);
        let fine = format!("{}{}", "{".repeat(8), "}".repeat(8));
        assert!(parse(&fine, &s).is_ok());
        let deep_parens = format!("x = {}1{};", "(".repeat(12), ")".repeat(12));
        assert_eq!(fatal(&deep_parens, &s).code, TOO_DEEP);
        let deep_unary = format!("x = {}1;", "-".repeat(40));
        assert_eq!(fatal(&deep_unary, &s).code, TOO_DEEP);
    }

    #[test]
    fn a_long_chain_is_bounded_too() {
        let s = UlpSettings {
            max_depth: 10,
            ..UlpSettings::default()
        };
        let sum = format!("x = 1{};", " + 1".repeat(100));
        assert_eq!(fatal(&sum, &s).code, TOO_DEEP);
        let members = format!("x = a{};", ".b".repeat(100));
        assert_eq!(fatal(&members, &s).code, TOO_DEEP);
        let short = format!("x = 1{};", " + 1".repeat(60));
        assert!(parse(&short, &s).is_ok());
    }

    #[test]
    fn the_statements_are_counted() {
        let s = UlpSettings {
            max_statements: 5,
            ..UlpSettings::default()
        };
        assert!(parse(";;;;;", &s).is_ok());
        let e = fatal(";;;;;;", &s);
        assert_eq!((e.code, e.line), (TOO_MANY_STATEMENTS, 1));
    }

    #[test]
    fn the_limits_of_the_lexer_reach_the_parser() {
        let s = UlpSettings {
            max_tokens: 5,
            max_source_bytes: 100,
            ..UlpSettings::default()
        };
        assert_eq!(fatal("a b c d e f g", &s).code, TOO_MANY_TOKENS);
        assert_eq!(fatal(&"a".repeat(101), &s).code, SOURCE_TOO_LARGE);
    }

    #[test]
    fn the_empty_program_and_the_one_with_only_comments_are_valid() {
        assert_eq!(ok("").statement_count(), 0);
        assert_eq!(ok("// nothing\n/* at all */\n").statement_count(), 0);
    }

    #[test]
    fn the_problems_are_in_the_order_of_the_program_whoever_found_them() {
        let d = problems("int a = ;\nint b = 2\nstring s = \"open;\nx = 3 @ 4;\n");
        let at: Vec<(usize, usize)> = d.iter().map(|p| (p.line, p.col)).collect();
        let mut sorted = at.clone();
        sorted.sort_unstable();
        assert_eq!(at, sorted);
        assert_eq!(d[0].code, UNEXPECTED);
        assert!(d.iter().any(|p| p.code == STRING_OPEN));
        assert!(d.iter().any(|p| p.code == CHAR_INVALID));
    }

    #[test]
    fn a_declaration_may_be_given_the_value_of_a_dialog() {
        let p = ok("int ok = dlgDialog(\"T\") { dlgLabel(\"x\"); dlgPushButton(\"+OK\") { dlgAccept(); } };\nint n = 1;");
        assert_eq!(p.body.len(), 2);
        assert!(p.statement_count() >= 5, "{}", p.statement_count());
        let q = ok("n = dlgDialog(\"T\") { };");
        assert!(matches!(q.body[0].kind, StmtKind::Loop { .. }));
    }

    #[test]
    fn a_dialog_block_ending_in_a_semicolon_is_one_statement() {
        let p = ok("dlgDialog(\"Title\") { dlgHBoxLayout { dlgLabel(\"x\"); } };\nint a;");
        assert_eq!(p.body.len(), 2);
    }
}
