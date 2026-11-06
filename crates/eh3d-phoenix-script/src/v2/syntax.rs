// SPDX-License-Identifier: GPL-3.0-or-later
//! The text of a script of the second version: the header, the tokens, the syntax tree and the
//! reader that makes the tree. Every error names its line and column.
//!
//! ```text
//! phoenix-script/2 board          # the header: the version and, optionally, the kind of document
//! let n = 4                       # a statement ends at a line break or at ;
//! for i in 0..n {                 # a loop over a range (half open) or over a list
//!     if i % 2 == 0 { print("even", i) } else { print("odd", i) }
//! }
//! ```

use crate::codes::{Diag, LIMIT_EXCEEDED};
use crate::settings::ScriptSettings;
use crate::v2::codes::{HEADER, NESTING_LIMIT, SYNTAX};
use crate::v2::value::Value;
use crate::values::{Len, Unit};

/// The version string of the language, as the header writes it.
pub const API: &str = "phoenix-script/2";

/// The kind of document a script is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A board.
    Board,
    /// A schematic.
    Schematic,
    /// A library of footprints, symbols and devices.
    Library,
}

impl Target {
    /// The word of the header.
    pub fn word(self) -> &'static str {
        match self {
            Target::Board => "board",
            Target::Schematic => "schematic",
            Target::Library => "library",
        }
    }

    /// The target a word names.
    pub fn from_word(word: &str) -> Option<Target> {
        [Target::Board, Target::Schematic, Target::Library]
            .into_iter()
            .find(|t| t.word() == word)
    }
}

/// A place in the text of a script: line and column from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    /// The line.
    pub line: usize,
    /// The column, in characters.
    pub col: usize,
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`: numbers added, texts joined.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`: integer division towards zero.
    Div,
    /// `%`: the remainder of that division.
    Rem,
    /// `==`.
    Eq,
    /// `!=`.
    Ne,
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `>=`.
    Ge,
    /// `and`: the right side is evaluated only when the left is true.
    And,
    /// `or`: the right side is evaluated only when the left is false.
    Or,
}

/// An expression with its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Where it starts.
    pub pos: Pos,
    /// What it is.
    pub kind: ExprKind,
}

/// The forms of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A constant.
    Lit(Value),
    /// A variable.
    Var(String),
    /// A list written out: `[a, b]`.
    List(Vec<Expr>),
    /// `-x`.
    Neg(Box<Expr>),
    /// `not x`.
    Not(Box<Expr>),
    /// `a op b`.
    Bin(BinOp, Box<Expr>, Box<Expr>),
    /// `name(args)`.
    Call(String, Vec<Expr>),
    /// `a[i]`.
    Index(Box<Expr>, Box<Expr>),
    /// `a.name`.
    Field(Box<Expr>, String),
}

/// A statement with its place.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// Where it starts.
    pub pos: Pos,
    /// What it is.
    pub kind: StmtKind,
}

/// The forms of a statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `let name = expr`.
    Let(String, Expr),
    /// `name = expr`.
    Assign(String, Expr),
    /// `if c { } else if d { } else { }`: the arms and the last block.
    If(Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    /// `for v in a..b { }` (with `to`) or `for v in list { }` (without).
    For {
        /// The variable of the loop.
        var: String,
        /// The first number of the range, or the list.
        from: Expr,
        /// The end of the range, which is not reached.
        to: Option<Expr>,
        /// The body.
        body: Vec<Stmt>,
    },
    /// `while c { }`.
    While(Expr, Vec<Stmt>),
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// An expression whose value is dropped, usually a call.
    Expr(Expr),
}

/// A script that was read.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// The kind of document the header names, if it names one.
    pub target: Option<Target>,
    /// The statements.
    pub body: Vec<Stmt>,
}

/// The version of the language a text is written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    /// The command language of the old workflow: no header, or `phoenix-script/1`.
    First,
    /// The language of this module.
    Second,
    /// A header that names another version.
    Other(String),
}

/// The line of the header: the first line that is not blank and not a comment.
fn header_line(text: &str) -> Option<(usize, &str)> {
    text.lines().enumerate().find_map(|(i, raw)| {
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            None
        } else {
            Some((i, t))
        }
    })
}

/// Tells which version of the language a text is written in, from its first significant line.
pub fn version_of(text: &str) -> Version {
    match header_line(text) {
        Some((_, line)) if line.starts_with("phoenix-script/") => {
            let word = line.split_whitespace().next().unwrap_or(line);
            match word {
                "phoenix-script/1" => Version::First,
                API => Version::Second,
                other => Version::Other(other.to_owned()),
            }
        }
        _ => Version::First,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Int(i64),
    Str(String),
    Ident(String),
    Op(&'static str),
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    pos: Pos,
}

fn syntax(pos: Pos, detail: impl Into<String>) -> Diag {
    Diag::new(SYNTAX, pos.line, pos.col, detail)
}

const KEYWORDS: &[&str] = &[
    "let", "if", "else", "for", "in", "while", "break", "continue", "and", "or", "not",
];

/// True when the word is a keyword of the language and so cannot name a variable.
pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word) || matches!(word, "true" | "false" | "nil")
}

fn number(text: &str, pos: Pos) -> Result<i64, Diag> {
    let split = text
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(text.len());
    let (digits, suffix) = text.split_at(split);
    if suffix.is_empty() {
        if digits.contains('.') {
            return Err(syntax(
                pos,
                format!("{text}: a decimal number needs a unit (mm, mil, in, um or deg)"),
            ));
        }
        return digits
            .parse::<i64>()
            .map_err(|_| syntax(pos, format!("{text}: the number does not fit 64 bits")));
    }
    if suffix == "deg" {
        let (angle, _exact) = eh3d_phoenix_model::Udeg::parse_deg(digits)
            .map_err(|_| syntax(pos, format!("{text}: the angle is not valid")))?;
        return Ok(angle.get());
    }
    let unit_text = if suffix == "um" {
        format!("{digits}micron")
    } else {
        text.to_owned()
    };
    let Some(len) = Len::parse(&unit_text) else {
        return Err(syntax(
            pos,
            format!("{text}: the unit is not mm, mil, in, inch, um or deg"),
        ));
    };
    if len.unit == Some(Unit::Grid) {
        return Err(syntax(pos, "the unit grid does not exist in this language"));
    }
    len.to_nm(Unit::Mm, 0)
        .map(eh3d_phoenix_model::Nm::get)
        .ok_or_else(|| syntax(pos, format!("{text}: the length does not fit the model")))
}

fn lex(src: &str, settings: &ScriptSettings) -> Result<Vec<Token>, Diag> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Token> = Vec::new();
    let (mut i, mut line, mut col) = (0usize, 1usize, 1usize);
    let mut depth = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let pos = Pos { line, col };
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
                col += 1;
            }
            '\n' => {
                if depth == 0 {
                    out.push(Token {
                        tok: Tok::Newline,
                        pos,
                    });
                }
                i += 1;
                line += 1;
                col = 1;
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
            }
            ';' => {
                if depth > 0 {
                    return Err(syntax(pos, "a semicolon inside brackets"));
                }
                out.push(Token {
                    tok: Tok::Newline,
                    pos,
                });
                i += 1;
                col += 1;
            }
            '"' | '\'' => {
                let quote = c;
                let mut text = String::new();
                let mut j = i + 1;
                let mut closed = false;
                while j < chars.len() && chars[j] != '\n' {
                    let d = chars[j];
                    if d == quote {
                        closed = true;
                        j += 1;
                        break;
                    }
                    if d == '\\' && j + 1 < chars.len() {
                        let e = chars[j + 1];
                        text.push(match e {
                            'n' => '\n',
                            't' => '\t',
                            other => other,
                        });
                        j += 2;
                    } else {
                        text.push(d);
                        j += 1;
                    }
                    if text.chars().count() > settings.max_string_chars {
                        return Err(Diag::new(
                            LIMIT_EXCEEDED,
                            pos.line,
                            pos.col,
                            format!(
                                "a text longer than {} characters",
                                settings.max_string_chars
                            ),
                        ));
                    }
                }
                if !closed {
                    return Err(syntax(pos, "a text in quotes is not closed"));
                }
                col += j - i;
                i = j;
                out.push(Token {
                    tok: Tok::Str(text),
                    pos,
                });
            }
            '0'..='9' => {
                let mut j = i;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j + 1 < chars.len() && chars[j] == '.' && chars[j + 1].is_ascii_digit() {
                    j += 1;
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                }
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                let text: String = chars[i..j].iter().collect();
                let value = number(&text, pos)?;
                out.push(Token {
                    tok: Tok::Int(value),
                    pos,
                });
                col += j - i;
                i = j;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut j = i;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let text: String = chars[i..j].iter().collect();
                out.push(Token {
                    tok: Tok::Ident(text),
                    pos,
                });
                col += j - i;
                i = j;
            }
            _ => {
                let next = chars.get(i + 1).copied();
                let two: Option<&'static str> = match (c, next) {
                    ('=', Some('=')) => Some("=="),
                    ('!', Some('=')) => Some("!="),
                    ('<', Some('=')) => Some("<="),
                    ('>', Some('=')) => Some(">="),
                    ('.', Some('.')) => Some(".."),
                    _ => None,
                };
                if let Some(op) = two {
                    out.push(Token {
                        tok: Tok::Op(op),
                        pos,
                    });
                    i += 2;
                    col += 2;
                    continue;
                }
                let one: Option<&'static str> = match c {
                    '+' => Some("+"),
                    '-' => Some("-"),
                    '*' => Some("*"),
                    '/' => Some("/"),
                    '%' => Some("%"),
                    '(' => Some("("),
                    ')' => Some(")"),
                    '[' => Some("["),
                    ']' => Some("]"),
                    '{' => Some("{"),
                    '}' => Some("}"),
                    ',' => Some(","),
                    '.' => Some("."),
                    '=' => Some("="),
                    '<' => Some("<"),
                    '>' => Some(">"),
                    _ => None,
                };
                let Some(op) = one else {
                    return Err(syntax(
                        pos,
                        format!("the character {c:?} is not part of the language"),
                    ));
                };
                match op {
                    "(" | "[" => depth += 1,
                    ")" | "]" => {
                        if depth == 0 {
                            return Err(syntax(pos, "a closing bracket with no opening one"));
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
                out.push(Token {
                    tok: Tok::Op(op),
                    pos,
                });
                i += 1;
                col += 1;
            }
        }
    }
    let end = Pos { line, col };
    if depth > 0 {
        return Err(syntax(end, "a bracket is not closed"));
    }
    out.push(Token {
        tok: Tok::Newline,
        pos: end,
    });
    out.push(Token {
        tok: Tok::Eof,
        pos: end,
    });
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<Token>,
    at: usize,
    settings: &'a ScriptSettings,
    blocks: usize,
    depth: usize,
    loops: usize,
}

impl<'a> Parser<'a> {
    fn pos(&self) -> Pos {
        self.toks[self.at].pos
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.at].tok
    }

    fn bump(&mut self) -> Token {
        let token = self.toks[self.at].clone();
        if self.at + 1 < self.toks.len() {
            self.at += 1;
        }
        token
    }

    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if *o == op)
    }

    fn is_word(&self, word: &str) -> bool {
        matches!(self.peek(), Tok::Ident(w) if w == word)
    }

    fn expect_op(&mut self, op: &str) -> Result<Pos, Diag> {
        if self.is_op(op) {
            Ok(self.bump().pos)
        } else {
            Err(syntax(self.pos(), format!("{op} was expected")))
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) {
            self.bump();
        }
    }

    fn ident(&mut self, what: &str) -> Result<String, Diag> {
        let pos = self.pos();
        match self.peek().clone() {
            Tok::Ident(w) if !is_keyword(&w) => {
                self.bump();
                Ok(w)
            }
            _ => Err(syntax(pos, format!("a name was expected for {what}"))),
        }
    }

    fn enter(&mut self, extra: usize) -> Result<(), Diag> {
        self.depth += 1;
        if self.depth + extra > self.settings.max_expr_depth {
            return Err(Diag::new(
                NESTING_LIMIT,
                self.pos().line,
                self.pos().col,
                format!(
                    "an expression nested deeper than {}",
                    self.settings.max_expr_depth
                ),
            ));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn end_of_statement(&mut self) -> Result<(), Diag> {
        match self.peek() {
            Tok::Newline => {
                self.bump();
                Ok(())
            }
            Tok::Eof => Ok(()),
            Tok::Op("}") => Ok(()),
            _ => Err(syntax(self.pos(), "the end of the statement was expected")),
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, Diag> {
        let open = self.expect_op("{")?;
        self.blocks += 1;
        if self.blocks > self.settings.max_block_depth {
            return Err(Diag::new(
                NESTING_LIMIT,
                open.line,
                open.col,
                format!(
                    "blocks nested deeper than {}",
                    self.settings.max_block_depth
                ),
            ));
        }
        let mut body = Vec::new();
        loop {
            self.skip_newlines();
            if self.is_op("}") {
                self.bump();
                break;
            }
            if matches!(self.peek(), Tok::Eof) {
                return Err(syntax(open, "a block is not closed"));
            }
            body.push(self.statement()?);
            if body.len() > self.settings.max_commands {
                return Err(Diag::new(
                    LIMIT_EXCEEDED,
                    open.line,
                    open.col,
                    format!("more than {} statements", self.settings.max_commands),
                ));
            }
        }
        self.blocks -= 1;
        Ok(body)
    }

    fn statement(&mut self) -> Result<Stmt, Diag> {
        let pos = self.pos();
        let word = match self.peek() {
            Tok::Ident(w) => Some(w.clone()),
            _ => None,
        };
        let kind = match word.as_deref() {
            Some("let") => {
                self.bump();
                let name = self.ident("the variable")?;
                self.expect_op("=")?;
                let value = self.expr()?;
                self.end_of_statement()?;
                StmtKind::Let(name, value)
            }
            Some("if") => {
                let kind = self.if_statement()?;
                self.end_of_statement()?;
                kind
            }
            Some("for") => {
                self.bump();
                let var = self.ident("the loop variable")?;
                if !self.is_word("in") {
                    return Err(syntax(self.pos(), "in was expected"));
                }
                self.bump();
                let from = self.expr()?;
                let to = if self.is_op("..") {
                    self.bump();
                    Some(self.expr()?)
                } else {
                    None
                };
                self.loops += 1;
                let body = self.block()?;
                self.loops -= 1;
                self.end_of_statement()?;
                StmtKind::For {
                    var,
                    from,
                    to,
                    body,
                }
            }
            Some("while") => {
                self.bump();
                let cond = self.expr()?;
                self.loops += 1;
                let body = self.block()?;
                self.loops -= 1;
                self.end_of_statement()?;
                StmtKind::While(cond, body)
            }
            Some(w @ ("break" | "continue")) => {
                self.bump();
                if self.loops == 0 {
                    return Err(syntax(pos, format!("{w} outside a loop")));
                }
                self.end_of_statement()?;
                if w == "break" {
                    StmtKind::Break
                } else {
                    StmtKind::Continue
                }
            }
            Some(w)
                if !is_keyword(w)
                    && matches!(self.toks.get(self.at + 1), Some(t) if t.tok == Tok::Op("=")) =>
            {
                let name = w.to_owned();
                self.bump();
                self.bump();
                let value = self.expr()?;
                self.end_of_statement()?;
                StmtKind::Assign(name, value)
            }
            _ => {
                let e = self.expr()?;
                self.end_of_statement()?;
                StmtKind::Expr(e)
            }
        };
        Ok(Stmt { pos, kind })
    }

    fn if_statement(&mut self) -> Result<StmtKind, Diag> {
        let mut arms = Vec::new();
        let mut otherwise = None;
        self.bump();
        let cond = self.expr()?;
        let body = self.block()?;
        arms.push((cond, body));
        while self.is_word("else") {
            self.bump();
            if self.is_word("if") {
                self.bump();
                let cond = self.expr()?;
                let body = self.block()?;
                arms.push((cond, body));
            } else {
                otherwise = Some(self.block()?);
                break;
            }
        }
        Ok(StmtKind::If(arms, otherwise))
    }

    fn expr(&mut self) -> Result<Expr, Diag> {
        self.enter(0)?;
        let r = self.or_expr();
        self.leave();
        r
    }

    fn binary_level(
        &mut self,
        next: fn(&mut Parser<'a>) -> Result<Expr, Diag>,
        table: &[(&str, BinOp)],
    ) -> Result<Expr, Diag> {
        let mut left = next(self)?;
        let mut chain = 0usize;
        loop {
            let found = match self.peek() {
                Tok::Op(o) => table
                    .iter()
                    .find(|entry| entry.0 == *o)
                    .map(|entry| entry.1),
                Tok::Ident(w) => table
                    .iter()
                    .find(|entry| entry.0 == w.as_str())
                    .map(|entry| entry.1),
                _ => None,
            };
            let Some(op) = found else { break };
            let pos = self.pos();
            self.bump();
            chain += 1;
            self.enter(chain)?;
            let right = next(self);
            self.leave();
            let right = right?;
            left = Expr {
                pos,
                kind: ExprKind::Bin(op, Box::new(left), Box::new(right)),
            };
        }
        Ok(left)
    }

    fn or_expr(&mut self) -> Result<Expr, Diag> {
        self.binary_level(Parser::and_expr, &[("or", BinOp::Or)])
    }

    fn and_expr(&mut self) -> Result<Expr, Diag> {
        self.binary_level(Parser::not_expr, &[("and", BinOp::And)])
    }

    fn not_expr(&mut self) -> Result<Expr, Diag> {
        if self.is_word("not") {
            let pos = self.pos();
            self.bump();
            self.enter(0)?;
            let inner = self.not_expr();
            self.leave();
            return Ok(Expr {
                pos,
                kind: ExprKind::Not(Box::new(inner?)),
            });
        }
        self.cmp_expr()
    }

    fn cmp_expr(&mut self) -> Result<Expr, Diag> {
        self.binary_level(
            Parser::add_expr,
            &[
                ("==", BinOp::Eq),
                ("!=", BinOp::Ne),
                ("<=", BinOp::Le),
                (">=", BinOp::Ge),
                ("<", BinOp::Lt),
                (">", BinOp::Gt),
            ],
        )
    }

    fn add_expr(&mut self) -> Result<Expr, Diag> {
        self.binary_level(Parser::mul_expr, &[("+", BinOp::Add), ("-", BinOp::Sub)])
    }

    fn mul_expr(&mut self) -> Result<Expr, Diag> {
        self.binary_level(
            Parser::unary_expr,
            &[("*", BinOp::Mul), ("/", BinOp::Div), ("%", BinOp::Rem)],
        )
    }

    fn unary_expr(&mut self) -> Result<Expr, Diag> {
        if self.is_op("-") {
            let pos = self.pos();
            self.bump();
            self.enter(0)?;
            let inner = self.unary_expr();
            self.leave();
            return Ok(Expr {
                pos,
                kind: ExprKind::Neg(Box::new(inner?)),
            });
        }
        self.postfix_expr()
    }

    fn postfix_expr(&mut self) -> Result<Expr, Diag> {
        let mut e = self.primary()?;
        let mut chain = 0usize;
        loop {
            let pos = self.pos();
            if self.is_op("[") {
                self.bump();
                chain += 1;
                self.enter(chain)?;
                let index = self.expr();
                self.leave();
                let index = index?;
                self.expect_op("]")?;
                e = Expr {
                    pos,
                    kind: ExprKind::Index(Box::new(e), Box::new(index)),
                };
            } else if self.is_op(".") {
                self.bump();
                chain += 1;
                self.enter(chain)?;
                self.leave();
                let name = match self.peek().clone() {
                    Tok::Ident(w) => {
                        self.bump();
                        w
                    }
                    _ => return Err(syntax(self.pos(), "a field name was expected after .")),
                };
                e = Expr {
                    pos,
                    kind: ExprKind::Field(Box::new(e), name),
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn arguments(&mut self, close: &str) -> Result<Vec<Expr>, Diag> {
        let mut items = Vec::new();
        if self.is_op(close) {
            self.bump();
            return Ok(items);
        }
        loop {
            items.push(self.expr()?);
            if items.len() > self.settings.max_args {
                return Err(Diag::new(
                    LIMIT_EXCEEDED,
                    self.pos().line,
                    self.pos().col,
                    format!("more than {} items", self.settings.max_args),
                ));
            }
            if self.is_op(",") {
                self.bump();
                continue;
            }
            self.expect_op(close)?;
            return Ok(items);
        }
    }

    fn primary(&mut self) -> Result<Expr, Diag> {
        let pos = self.pos();
        match self.peek().clone() {
            Tok::Int(n) => {
                self.bump();
                Ok(Expr {
                    pos,
                    kind: ExprKind::Lit(Value::Int(n)),
                })
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Expr {
                    pos,
                    kind: ExprKind::Lit(Value::Str(s)),
                })
            }
            Tok::Op("(") => {
                self.bump();
                self.enter(0)?;
                let inner = self.expr();
                self.leave();
                let inner = inner?;
                self.expect_op(")")?;
                Ok(inner)
            }
            Tok::Op("[") => {
                self.bump();
                let items = self.arguments("]")?;
                Ok(Expr {
                    pos,
                    kind: ExprKind::List(items),
                })
            }
            Tok::Ident(w) => match w.as_str() {
                "true" | "false" => {
                    self.bump();
                    Ok(Expr {
                        pos,
                        kind: ExprKind::Lit(Value::Bool(w == "true")),
                    })
                }
                "nil" => {
                    self.bump();
                    Ok(Expr {
                        pos,
                        kind: ExprKind::Lit(Value::Nil),
                    })
                }
                _ if is_keyword(&w) => Err(syntax(pos, format!("{w} cannot be used here"))),
                _ => {
                    self.bump();
                    if self.is_op("(") {
                        self.bump();
                        let args = self.arguments(")")?;
                        Ok(Expr {
                            pos,
                            kind: ExprKind::Call(w, args),
                        })
                    } else {
                        Ok(Expr {
                            pos,
                            kind: ExprKind::Var(w),
                        })
                    }
                }
            },
            Tok::Newline | Tok::Eof => Err(syntax(pos, "an expression was expected")),
            _ => Err(syntax(pos, "this cannot start an expression")),
        }
    }
}

/// Reads a script of the second version. The first significant line must be the header
/// `phoenix-script/2`, optionally followed by `board`, `schematic` or `library`.
pub fn parse_program(text: &str, settings: &ScriptSettings) -> Result<Program, Diag> {
    if text.len() > settings.max_script_bytes {
        return Err(Diag::plain(
            LIMIT_EXCEEDED,
            format!(
                "the script is larger than {} bytes",
                settings.max_script_bytes
            ),
        ));
    }
    if text.lines().count() > settings.max_lines {
        return Err(Diag::plain(
            LIMIT_EXCEEDED,
            format!("the script has more than {} lines", settings.max_lines),
        ));
    }
    let Some((header_index, header)) = header_line(text) else {
        return Err(Diag::new(
            HEADER,
            1,
            1,
            "the script is empty: the first line must be phoenix-script/2",
        ));
    };
    let mut words = header.split_whitespace();
    let version = words.next().unwrap_or("");
    if version != API {
        return Err(Diag::new(
            HEADER,
            header_index + 1,
            1,
            format!("{version:?} is not {API}"),
        ));
    }
    let mut target = None;
    for word in words {
        if word.starts_with('#') {
            break;
        }
        match Target::from_word(word) {
            Some(t) if target.is_none() => target = Some(t),
            _ => {
                return Err(Diag::new(
                    HEADER,
                    header_index + 1,
                    1,
                    format!("{word:?} is not board, schematic or library, or comes twice"),
                ))
            }
        }
    }
    let body: String = text
        .lines()
        .enumerate()
        .map(|(i, l)| if i == header_index { "" } else { l })
        .collect::<Vec<&str>>()
        .join("\n");
    let toks = lex(&body, settings)?;
    let mut parser = Parser {
        toks,
        at: 0,
        settings,
        blocks: 0,
        depth: 0,
        loops: 0,
    };
    let mut statements = Vec::new();
    loop {
        parser.skip_newlines();
        if matches!(parser.peek(), Tok::Eof) {
            break;
        }
        if parser.is_op("}") {
            return Err(syntax(parser.pos(), "a closing brace with no block"));
        }
        statements.push(parser.statement()?);
        if statements.len() > settings.max_commands {
            return Err(Diag::plain(
                LIMIT_EXCEEDED,
                format!("more than {} statements", settings.max_commands),
            ));
        }
    }
    Ok(Program {
        target,
        body: statements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(body: &str) -> Result<Program, Diag> {
        parse_program(
            &format!("phoenix-script/2\n{body}"),
            &ScriptSettings::default(),
        )
    }

    fn first_error(body: &str) -> Diag {
        read(body).unwrap_err()
    }

    #[test]
    fn the_version_is_told_from_the_first_significant_line() {
        assert_eq!(version_of("GRID mm\n"), Version::First);
        assert_eq!(version_of("# a\n\nphoenix-script/1\nGRID"), Version::First);
        assert_eq!(version_of("# a\nphoenix-script/2 board\n"), Version::Second);
        assert_eq!(
            version_of("phoenix-script/9"),
            Version::Other("phoenix-script/9".to_owned())
        );
        assert_eq!(version_of(""), Version::First);
    }

    #[test]
    fn the_header_names_the_target() {
        let s = ScriptSettings::default();
        let p = parse_program("# note\nphoenix-script/2 schematic # more\nlet a = 1", &s).unwrap();
        assert_eq!(p.target, Some(Target::Schematic));
        assert_eq!(p.body.len(), 1);
        assert_eq!(
            parse_program("phoenix-script/2\n", &s).unwrap().target,
            None
        );
        for bad in [
            "phoenix-script/1\nlet a = 1",
            "let a = 1",
            "phoenix-script/2 board board",
            "phoenix-script/2 chip",
            "",
        ] {
            assert_eq!(parse_program(bad, &s).unwrap_err().code, HEADER, "{bad:?}");
        }
        assert_eq!(parse_program("a\nb", &s).unwrap_err().line, 1);
    }

    #[test]
    fn numbers_with_units_become_integers() {
        let p = read(
            "let a = 0.2mm\nlet b = 8mil\nlet c = 1in\nlet d = 25um\nlet e = 22.5deg\nlet f = 7",
        )
        .unwrap();
        let values: Vec<i64> = p
            .body
            .iter()
            .map(|s| match &s.kind {
                StmtKind::Let(_, e) => match &e.kind {
                    ExprKind::Lit(Value::Int(n)) => *n,
                    other => panic!("{other:?}"),
                },
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            values,
            vec![200_000, 203_200, 25_400_000, 25_000, 22_500_000, 7]
        );
    }

    #[test]
    fn numbers_that_are_not_valid_are_refused_with_their_place() {
        for (body, line, col) in [
            ("let a = 1.5", 2, 9),
            ("let a = 5furlong", 2, 9),
            ("let a = 99999999999999999999", 2, 9),
            ("let a = 2grid", 2, 9),
            ("let a = 1\nlet b = 3k", 3, 9),
        ] {
            let d = first_error(body);
            assert_eq!((d.code, d.line, d.col), (SYNTAX, line, col), "{body}");
        }
    }

    #[test]
    fn statements_and_blocks_are_read() {
        let p = read(
            "let n = 3\nfor i in 0..n {\n  if i == 1 { print(i) } else if i == 2 { print(0) } else { print(9) }\n}\nwhile n > 0 { n = n - 1; continue }\nfor x in [1, 2] { break }",
        )
        .unwrap();
        assert_eq!(p.body.len(), 4);
        assert!(matches!(p.body[1].kind, StmtKind::For { to: Some(_), .. }));
        assert!(matches!(p.body[3].kind, StmtKind::For { to: None, .. }));
        assert!(matches!(&p.body[2].kind, StmtKind::While(_, b) if b.len() == 2));
    }

    #[test]
    fn precedence_and_postfix_forms_are_read() {
        let p = read("let a = 1 + 2 * 3 == 7 and not false or x[0].name").unwrap();
        let StmtKind::Let(_, e) = &p.body[0].kind else {
            panic!()
        };
        let ExprKind::Bin(BinOp::Or, left, right) = &e.kind else {
            panic!("{e:?}")
        };
        assert!(matches!(left.kind, ExprKind::Bin(BinOp::And, _, _)));
        assert!(matches!(right.kind, ExprKind::Field(_, _)));
    }

    #[test]
    fn newlines_inside_brackets_do_not_end_a_statement() {
        let p = read("let a = [\n 1,\n 2\n]\nprint(\n a,\n a\n)").unwrap();
        assert_eq!(p.body.len(), 2);
    }

    #[test]
    fn syntax_errors_name_line_and_column() {
        for (body, line, col) in [
            ("let = 1", 2, 5),
            ("let a 1", 2, 7),
            ("let a = ", 2, 9),
            ("print(1", 2, 8),
            ("for i 0..3 { }", 2, 7),
            ("if true { print(1)", 2, 9),
            ("}", 2, 1),
            ("let a = 1 $", 2, 11),
            ("let a = \"open", 2, 9),
            ("break", 2, 1),
            ("let if = 1", 2, 5),
            ("x = ", 2, 5),
            ("let a = (1 + 2", 2, 15),
        ] {
            let d = first_error(body);
            assert_eq!((d.code, d.line, d.col), (SYNTAX, line, col), "{body}");
        }
    }

    #[test]
    fn else_must_follow_the_closing_brace_on_its_line() {
        assert!(read("if true { }\nelse { }").is_err());
        assert!(read("if true { } else { }").is_ok());
    }

    #[test]
    fn nesting_and_size_limits_are_enforced() {
        let s = ScriptSettings::default();
        let deep = format!(
            "phoenix-script/2\nlet a = {}1{}",
            "(".repeat(40),
            ")".repeat(40)
        );
        assert_eq!(parse_program(&deep, &s).unwrap_err().code, NESTING_LIMIT);
        let chain = format!("phoenix-script/2\nlet a = {}", "1+".repeat(300) + "1");
        assert_eq!(parse_program(&chain, &s).unwrap_err().code, NESTING_LIMIT);
        let blocks = format!(
            "phoenix-script/2\n{}{}",
            "if true {".repeat(20),
            "}".repeat(20)
        );
        assert_eq!(parse_program(&blocks, &s).unwrap_err().code, NESTING_LIMIT);
        let small = ScriptSettings {
            max_script_bytes: 10,
            ..ScriptSettings::default()
        };
        assert_eq!(
            parse_program("phoenix-script/2\nlet a = 1", &small)
                .unwrap_err()
                .code,
            LIMIT_EXCEEDED
        );
        let long = format!("phoenix-script/2\nlet a = \"{}\"", "x".repeat(300));
        assert_eq!(parse_program(&long, &s).unwrap_err().code, LIMIT_EXCEEDED);
    }

    #[test]
    fn a_comment_runs_to_the_end_of_the_line_and_a_semicolon_ends_a_statement() {
        let p = read("let a = 1 # one\nlet b = 2; let c = 3 # three\n# only a comment\n").unwrap();
        assert_eq!(p.body.len(), 3);
    }

    #[test]
    fn texts_take_both_quotes_and_the_usual_escapes() {
        let p = read("let a = 'it\\'s'\nlet b = \"x\\ny\"").unwrap();
        let texts: Vec<String> = p
            .body
            .iter()
            .map(|s| match &s.kind {
                StmtKind::Let(_, e) => match &e.kind {
                    ExprKind::Lit(Value::Str(t)) => t.clone(),
                    other => panic!("{other:?}"),
                },
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(texts, vec!["it's".to_owned(), "x\ny".to_owned()]);
    }
}
