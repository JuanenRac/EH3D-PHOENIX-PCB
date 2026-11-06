// SPDX-License-Identifier: GPL-3.0-or-later
//! The interpreter of the scripts of the second version.
//!
//! A run works on a copy of the document. Time is counted in steps (one for every statement and
//! every part of an expression), memory in cells, output in bytes, loops in turns and the document
//! in operations, all from `config/script.cfg`, so a script that does not end is stopped at the
//! same place on every computer. A run is all or nothing: the first error ends it, and the
//! outcome then holds no document. A run that ends well holds the result and one batch that does
//! the whole script, which is one step of the history.

use std::collections::BTreeMap;
use std::path::PathBuf;

use eh3d_phoenix_board::{Board, Operation};
use eh3d_phoenix_libedit::{Library, Operation as LibOp};
use eh3d_phoenix_schematic::{EditOp as SchOp, Schematic};

use crate::codes::Diag;
use crate::exec::RunContext;
use crate::v2::codes::{
    ARGUMENTS, ARITHMETIC, INDEX, ITERATION_LIMIT, MEMORY_LIMIT, OUTPUT_LIMIT, REDEFINED,
    STEP_LIMIT, TARGET, TYPE_MISMATCH, UNDEFINED, UNKNOWN_FUNCTION,
};
use crate::v2::funcs::{find, targets_text};
use crate::v2::syntax::{BinOp, Expr, ExprKind, Pos, Program, Stmt, StmtKind};
use crate::v2::value::{Obj, Value, MAX_LIST_DEPTH};
use crate::v2::world::{BoardWorld, LibWorld, SchWorld, World};

/// The document a script runs on.
#[derive(Debug, Clone, Copy)]
pub enum Doc<'d> {
    /// None: only the functions that need no document can be called.
    None,
    /// A board.
    Board(&'d Board),
    /// A schematic.
    Schematic(&'d Schematic),
    /// A library.
    Library(&'d Library),
}

/// A file a script asked to write. It is written only when the caller applies the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingFile {
    /// The path the script wrote, relative to the data folder.
    pub relative: String,
    /// The path on disk.
    pub path: PathBuf,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// What a run changed, as one batch of the operations of the model.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Nothing was changed.
    None,
    /// A board: the batch that does the script and the batch that undoes it.
    Board {
        /// The operations of the script, as one batch.
        batch: Box<Operation>,
        /// The inverse operations, in reverse order, as one batch.
        undo: Box<Operation>,
    },
    /// A board that an operation replaced as a whole (a panel): there is no batch.
    Replaced,
    /// A schematic: the operations of the script, as one batch of the editor.
    Schematic {
        /// The batch.
        batch: Box<SchOp>,
    },
    /// A library: the operations of the script, in order.
    Library {
        /// The operations.
        ops: Vec<LibOp>,
    },
}

/// The result of a run.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// True when the whole script was carried out.
    pub ok: bool,
    /// Why the run stopped (one diagnostic), or nothing.
    pub diagnostics: Vec<Diag>,
    /// The lines that `print` wrote, up to the stop when the run failed.
    pub output: Vec<String>,
    /// The steps taken.
    pub steps: u64,
    /// The operations made on the document.
    pub operations: usize,
    /// The board after the script, when the run ended well on a board.
    pub board: Option<Board>,
    /// The schematic after the script, when the run ended well on a schematic.
    pub schematic: Option<Schematic>,
    /// The library after the script, when the run ended well on a library.
    pub library: Option<Library>,
    /// The batch of the operations.
    pub plan: Plan,
    /// The files the script asked to write.
    pub writes: Vec<PendingFile>,
}

enum Flow {
    Next,
    Break,
    Continue,
}

/// The state of a run.
pub(crate) struct Interp<'a> {
    /// The context of the run.
    pub ctx: &'a RunContext<'a>,
    /// The document.
    pub world: World,
    scopes: Vec<BTreeMap<String, Value>>,
    steps: u64,
    out: Vec<String>,
    out_bytes: usize,
    /// The files the script asked to write.
    pub writes: Vec<PendingFile>,
    /// The data files the script used, by path, to count them.
    pub data_files: Vec<String>,
}

fn at(pos: Pos, code: &'static str, detail: impl Into<String>) -> Diag {
    Diag::new(code, pos.line, pos.col, detail)
}

fn want_int(v: Value, pos: Pos, what: &str) -> Result<i64, Diag> {
    match v {
        Value::Int(n) => Ok(n),
        other => Err(at(
            pos,
            TYPE_MISMATCH,
            format!("{what} must be an int, not {}", other.type_name()),
        )),
    }
}

fn want_bool(v: Value, pos: Pos, what: &str) -> Result<bool, Diag> {
    match v {
        Value::Bool(b) => Ok(b),
        other => Err(at(
            pos,
            TYPE_MISMATCH,
            format!("{what} must be a bool, not {}", other.type_name()),
        )),
    }
}

impl<'a> Interp<'a> {
    fn tick(&mut self, pos: Pos) -> Result<(), Diag> {
        self.charge(1, pos)
    }

    fn charge(&mut self, n: usize, pos: Pos) -> Result<(), Diag> {
        self.steps = self
            .steps
            .saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
        if self.steps > self.ctx.settings.max_steps {
            return Err(at(
                pos,
                STEP_LIMIT,
                format!("more than {} steps", self.ctx.settings.max_steps),
            ));
        }
        Ok(())
    }

    /// Writes a line of output.
    pub fn emit(&mut self, line: String) -> Result<(), Diag> {
        self.out_bytes = self.out_bytes.saturating_add(line.len() + 1);
        if self.out_bytes > self.ctx.settings.max_output_bytes {
            return Err(Diag::plain(
                OUTPUT_LIMIT,
                format!(
                    "more than {} bytes of output",
                    self.ctx.settings.max_output_bytes
                ),
            ));
        }
        self.out.push(line);
        Ok(())
    }

    /// The board, or an error when the document is not one.
    pub fn board(&mut self) -> Result<&mut BoardWorld, Diag> {
        match &mut self.world {
            World::Board(b) => Ok(b),
            other => Err(Diag::plain(
                TARGET,
                format!("this needs a board, the document is {}", other.name()),
            )),
        }
    }

    /// The schematic, or an error when the document is not one.
    pub fn sch(&mut self) -> Result<&mut SchWorld, Diag> {
        match &mut self.world {
            World::Schematic(s) => Ok(s),
            other => Err(Diag::plain(
                TARGET,
                format!("this needs a schematic, the document is {}", other.name()),
            )),
        }
    }

    /// The library, or an error when the document is not one.
    pub fn lib(&mut self) -> Result<&mut LibWorld, Diag> {
        match &mut self.world {
            World::Library(l) => Ok(l),
            other => Err(Diag::plain(
                TARGET,
                format!("this needs a library, the document is {}", other.name()),
            )),
        }
    }

    /// A short name of an object, for the output.
    pub fn describe(&self, o: &Obj) -> String {
        match &self.world {
            World::Board(b) => b.describe(o),
            World::Schematic(s) => s.describe(o),
            _ => format!("{}?", o.kind.word()),
        }
    }

    /// The value as the output writes it.
    pub fn render(&self, v: &Value) -> String {
        match v {
            Value::Nil => "nil".to_owned(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Str(s) => s.clone(),
            Value::List(items) => {
                let parts: Vec<String> = items.iter().map(|x| self.render(x)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Obj(o) => self.describe(o),
        }
    }

    /// A field of an object of the document.
    pub fn field_value(&self, v: &Value, name: &str) -> Result<Value, Diag> {
        let Value::Obj(o) = v else {
            return Err(Diag::plain(
                TYPE_MISMATCH,
                format!("only an object has fields, not {}", v.type_name()),
            ));
        };
        match &self.world {
            World::Board(b) => b.field(o, name),
            World::Schematic(s) => s.field(o, name),
            other => Err(Diag::plain(
                TARGET,
                format!(
                    "an object needs a board or a schematic, the document is {}",
                    other.name()
                ),
            )),
        }
    }

    /// Checks the size of a value that was just made, and charges the work of making it.
    pub fn check_value(&mut self, v: &Value, pos: Pos) -> Result<(), Diag> {
        let s = self.ctx.settings;
        let too_big = |what: String| at(pos, MEMORY_LIMIT, what);
        if v.depth() > MAX_LIST_DEPTH {
            return Err(too_big(format!(
                "lists nested deeper than {MAX_LIST_DEPTH}"
            )));
        }
        match v {
            Value::List(items) if items.len() > s.max_list_len => {
                return Err(too_big(format!(
                    "a list of more than {} items",
                    s.max_list_len
                )))
            }
            Value::Str(t) if t.len() > s.max_text_bytes => {
                return Err(too_big(format!(
                    "a text of more than {} bytes",
                    s.max_text_bytes
                )))
            }
            _ => {}
        }
        let cells = v.cells();
        if cells > s.max_value_cells {
            return Err(too_big(format!(
                "a value of more than {} cells",
                s.max_value_cells
            )));
        }
        self.charge(cells / 64, pos)
    }

    fn check_memory(&mut self, pos: Pos) -> Result<(), Diag> {
        let total: usize = self
            .scopes
            .iter()
            .flat_map(|s| s.values())
            .map(Value::cells)
            .sum();
        if total > self.ctx.settings.max_value_cells {
            return Err(at(
                pos,
                MEMORY_LIMIT,
                format!(
                    "the variables hold {total} cells, the limit is {}",
                    self.ctx.settings.max_value_cells
                ),
            ));
        }
        self.charge(total / 64, pos)
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    fn binary(&mut self, op: BinOp, a: Value, b: Value, pos: Pos) -> Result<Value, Diag> {
        let overflow = || at(pos, ARITHMETIC, "the result does not fit 64 bits");
        let mismatch = |a: &Value, b: &Value| {
            at(
                pos,
                TYPE_MISMATCH,
                format!(
                    "this operator does not take {} and {}",
                    a.type_name(),
                    b.type_name()
                ),
            )
        };
        match op {
            BinOp::Eq => Ok(Value::Bool(a == b)),
            BinOp::Ne => Ok(Value::Bool(a != b)),
            BinOp::Add => match (&a, &b) {
                (Value::Int(x), Value::Int(y)) => {
                    x.checked_add(*y).map(Value::Int).ok_or_else(overflow)
                }
                (Value::Str(x), Value::Str(y)) => {
                    if x.len() + y.len() > self.ctx.settings.max_text_bytes {
                        return Err(at(
                            pos,
                            MEMORY_LIMIT,
                            format!(
                                "a text of more than {} bytes",
                                self.ctx.settings.max_text_bytes
                            ),
                        ));
                    }
                    Ok(Value::Str(format!("{x}{y}")))
                }
                _ => Err(mismatch(&a, &b)),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => match (&a, &b) {
                (Value::Int(x), Value::Int(y)) => {
                    let r = match op {
                        BinOp::Sub => x.checked_sub(*y),
                        BinOp::Mul => x.checked_mul(*y),
                        BinOp::Div => {
                            if *y == 0 {
                                return Err(at(pos, ARITHMETIC, "division by zero"));
                            }
                            x.checked_div(*y)
                        }
                        _ => {
                            if *y == 0 {
                                return Err(at(pos, ARITHMETIC, "division by zero"));
                            }
                            x.checked_rem(*y)
                        }
                    };
                    r.map(Value::Int).ok_or_else(overflow)
                }
                _ => Err(mismatch(&a, &b)),
            },
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let order = match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    (Value::Str(x), Value::Str(y)) => x.cmp(y),
                    _ => return Err(mismatch(&a, &b)),
                };
                Ok(Value::Bool(match op {
                    BinOp::Lt => order.is_lt(),
                    BinOp::Le => order.is_le(),
                    BinOp::Gt => order.is_gt(),
                    _ => order.is_ge(),
                }))
            }
            BinOp::And | BinOp::Or => Err(at(pos, TYPE_MISMATCH, "and/or are evaluated lazily")),
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Diag> {
        self.tick(e.pos)?;
        match &e.kind {
            ExprKind::Lit(v) => Ok(v.clone()),
            ExprKind::Var(name) => self
                .lookup(name)
                .cloned()
                .ok_or_else(|| at(e.pos, UNDEFINED, name.clone())),
            ExprKind::List(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval(item)?);
                }
                let list = Value::List(values);
                self.check_value(&list, e.pos)?;
                Ok(list)
            }
            ExprKind::Neg(inner) => {
                let v = self.eval(inner)?;
                let n = want_int(v, e.pos, "the operand of -")?;
                n.checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| at(e.pos, ARITHMETIC, "the result does not fit 64 bits"))
            }
            ExprKind::Not(inner) => {
                let v = self.eval(inner)?;
                Ok(Value::Bool(!want_bool(v, e.pos, "the operand of not")?))
            }
            ExprKind::Bin(BinOp::And, l, r) => {
                let left = want_bool(self.eval(l)?, l.pos, "the operand of and")?;
                if !left {
                    return Ok(Value::Bool(false));
                }
                let right = want_bool(self.eval(r)?, r.pos, "the operand of and")?;
                Ok(Value::Bool(right))
            }
            ExprKind::Bin(BinOp::Or, l, r) => {
                let left = want_bool(self.eval(l)?, l.pos, "the operand of or")?;
                if left {
                    return Ok(Value::Bool(true));
                }
                let right = want_bool(self.eval(r)?, r.pos, "the operand of or")?;
                Ok(Value::Bool(right))
            }
            ExprKind::Bin(op, l, r) => {
                let a = self.eval(l)?;
                let b = self.eval(r)?;
                let v = self.binary(*op, a, b, e.pos)?;
                if matches!(v, Value::Str(_)) {
                    self.check_value(&v, e.pos)?;
                }
                Ok(v)
            }
            ExprKind::Index(target, index) => {
                let t = self.eval(target)?;
                let i = want_int(self.eval(index)?, index.pos, "an index")?;
                let out_of_range = |len: usize| {
                    at(
                        e.pos,
                        INDEX,
                        format!("index {i} in a value of length {len}"),
                    )
                };
                match &t {
                    Value::List(items) => usize::try_from(i)
                        .ok()
                        .and_then(|k| items.get(k))
                        .cloned()
                        .ok_or_else(|| out_of_range(items.len())),
                    Value::Str(s) => usize::try_from(i)
                        .ok()
                        .and_then(|k| s.chars().nth(k))
                        .map(|c| Value::Str(c.to_string()))
                        .ok_or_else(|| out_of_range(s.chars().count())),
                    other => Err(at(
                        e.pos,
                        TYPE_MISMATCH,
                        format!("a {} cannot be indexed", other.type_name()),
                    )),
                }
            }
            ExprKind::Field(target, name) => {
                let t = self.eval(target)?;
                self.field_value(&t, name)
                    .map_err(|d| d.moved(e.pos.line, e.pos.col))
            }
            ExprKind::Call(name, args) => {
                let Some(def) = find(name) else {
                    return Err(at(e.pos, UNKNOWN_FUNCTION, name.clone()));
                };
                if args.len() < def.min || args.len() > def.max {
                    let want = if def.min == def.max {
                        def.min.to_string()
                    } else {
                        format!("{} to {}", def.min, def.max)
                    };
                    return Err(at(
                        e.pos,
                        ARGUMENTS,
                        format!("{name} takes {want}, not {}", args.len()),
                    ));
                }
                if def.targets != 0 && def.targets & self.world.mask() == 0 {
                    return Err(at(
                        e.pos,
                        TARGET,
                        format!(
                            "{name} applies to {} and the document is {}",
                            targets_text(def.targets),
                            self.world.name()
                        ),
                    ));
                }
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval(a)?);
                }
                let result =
                    (def.run)(self, &values).map_err(|d| d.moved(e.pos.line, e.pos.col))?;
                self.check_value(&result, e.pos)?;
                Ok(result)
            }
        }
    }

    fn exec_block(&mut self, body: &[Stmt], bind: Option<(&str, Value)>) -> Result<Flow, Diag> {
        let mut scope = BTreeMap::new();
        if let Some((name, value)) = bind {
            scope.insert(name.to_owned(), value);
        }
        self.scopes.push(scope);
        let mut flow = Flow::Next;
        let mut failure = None;
        for stmt in body {
            match self.exec_stmt(stmt) {
                Ok(Flow::Next) => {}
                Ok(other) => {
                    flow = other;
                    break;
                }
                Err(d) => {
                    failure = Some(d);
                    break;
                }
            }
        }
        self.scopes.pop();
        match failure {
            Some(d) => Err(d),
            None => Ok(flow),
        }
    }

    fn turn(&self, turns: &mut u64, pos: Pos) -> Result<(), Diag> {
        *turns += 1;
        if *turns > self.ctx.settings.max_loop_iterations {
            return Err(at(
                pos,
                ITERATION_LIMIT,
                format!("more than {} turns", self.ctx.settings.max_loop_iterations),
            ));
        }
        Ok(())
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, Diag> {
        self.tick(stmt.pos)?;
        match &stmt.kind {
            StmtKind::Let(name, e) => {
                let v = self.eval(e)?;
                let taken = self.scopes.last().is_some_and(|s| s.contains_key(name));
                if taken {
                    return Err(at(stmt.pos, REDEFINED, name.clone()));
                }
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.clone(), v);
                }
                self.check_memory(stmt.pos)?;
                Ok(Flow::Next)
            }
            StmtKind::Assign(name, e) => {
                let v = self.eval(e)?;
                let mut done = false;
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(slot) = scope.get_mut(name) {
                        *slot = v;
                        done = true;
                        break;
                    }
                }
                if !done {
                    return Err(at(stmt.pos, UNDEFINED, name.clone()));
                }
                self.check_memory(stmt.pos)?;
                Ok(Flow::Next)
            }
            StmtKind::If(arms, otherwise) => {
                for (cond, body) in arms {
                    let c = want_bool(self.eval(cond)?, cond.pos, "a condition")?;
                    if c {
                        return self.exec_block(body, None);
                    }
                }
                match otherwise {
                    Some(body) => self.exec_block(body, None),
                    None => Ok(Flow::Next),
                }
            }
            StmtKind::For {
                var,
                from,
                to,
                body,
            } => {
                let mut turns = 0u64;
                if let Some(end) = to {
                    let first = want_int(self.eval(from)?, from.pos, "the start of a range")?;
                    let last = want_int(self.eval(end)?, end.pos, "the end of a range")?;
                    let mut i = first;
                    while i < last {
                        self.turn(&mut turns, stmt.pos)?;
                        match self.exec_block(body, Some((var.as_str(), Value::Int(i))))? {
                            Flow::Break => break,
                            Flow::Next | Flow::Continue => {}
                        }
                        i = i.saturating_add(1);
                        if i == i64::MAX {
                            break;
                        }
                    }
                } else {
                    let items = match self.eval(from)? {
                        Value::List(items) => items,
                        other => {
                            return Err(at(
                                from.pos,
                                TYPE_MISMATCH,
                                format!(
                                    "a loop goes over a range or a list, not {}",
                                    other.type_name()
                                ),
                            ))
                        }
                    };
                    for item in items {
                        self.turn(&mut turns, stmt.pos)?;
                        match self.exec_block(body, Some((var.as_str(), item)))? {
                            Flow::Break => break,
                            Flow::Next | Flow::Continue => {}
                        }
                    }
                }
                Ok(Flow::Next)
            }
            StmtKind::While(cond, body) => {
                let mut turns = 0u64;
                loop {
                    let c = want_bool(self.eval(cond)?, cond.pos, "a condition")?;
                    if !c {
                        break;
                    }
                    self.turn(&mut turns, stmt.pos)?;
                    match self.exec_block(body, None)? {
                        Flow::Break => break,
                        Flow::Next | Flow::Continue => {}
                    }
                }
                Ok(Flow::Next)
            }
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(Flow::Next)
            }
        }
    }

    fn operations(&self) -> usize {
        match &self.world {
            World::None => 0,
            World::Board(b) => b.ops.len(),
            World::Schematic(s) => s.ops.len(),
            World::Library(l) => l.ops.len(),
        }
    }

    fn finish(self, result: Result<(), Diag>) -> Outcome {
        let operations = self.operations();
        let mut outcome = Outcome {
            ok: result.is_ok(),
            diagnostics: result.err().into_iter().collect(),
            output: self.out,
            steps: self.steps,
            operations,
            board: None,
            schematic: None,
            library: None,
            plan: Plan::None,
            writes: Vec::new(),
        };
        if !outcome.ok {
            return outcome;
        }
        outcome.writes = self.writes;
        match self.world {
            World::None => {}
            World::Board(b) => {
                let b = *b;
                outcome.plan = if b.replaced {
                    Plan::Replaced
                } else if b.ops.is_empty() {
                    Plan::None
                } else {
                    let mut back = b.inverses;
                    back.reverse();
                    Plan::Board {
                        batch: Box::new(Operation::Batch { ops: b.ops }),
                        undo: Box::new(Operation::Batch { ops: back }),
                    }
                };
                outcome.board = Some(b.board);
            }
            World::Schematic(s) => {
                let s = *s;
                if !s.ops.is_empty() {
                    outcome.plan = Plan::Schematic {
                        batch: Box::new(SchOp::Batch { ops: s.ops }),
                    };
                }
                outcome.schematic = Some(s.editor.into_schematic());
            }
            World::Library(l) => {
                let l = *l;
                if !l.ops.is_empty() {
                    outcome.plan = Plan::Library { ops: l.ops };
                }
                outcome.library = Some(l.lib);
            }
        }
        outcome
    }
}

/// Runs a script on a copy of a document. The document given is never changed.
pub fn run(program: &Program, doc: Doc<'_>, ctx: &RunContext<'_>) -> Outcome {
    let settings = ctx.settings;
    let world = match doc {
        Doc::None => World::None,
        Doc::Board(b) => World::Board(Box::new(BoardWorld::new(b, settings))),
        Doc::Schematic(s) => World::Schematic(Box::new(SchWorld::new(s, settings))),
        Doc::Library(l) => World::Library(Box::new(LibWorld::new(l, settings))),
    };
    let mut interp = Interp {
        ctx,
        world,
        scopes: Vec::new(),
        steps: 0,
        out: Vec::new(),
        out_bytes: 0,
        writes: Vec::new(),
        data_files: Vec::new(),
    };
    let declared = match (program.target, &interp.world) {
        (Some(t), w) => {
            let bit = match t {
                crate::v2::syntax::Target::Board => crate::v2::world::BOARD,
                crate::v2::syntax::Target::Schematic => crate::v2::world::SCHEMATIC,
                crate::v2::syntax::Target::Library => crate::v2::world::LIBRARY,
            };
            if w.mask() == bit {
                Ok(())
            } else {
                Err(Diag::plain(
                    TARGET,
                    format!(
                        "the script is for a {} and the document is {}",
                        t.word(),
                        w.name()
                    ),
                ))
            }
        }
        (None, _) => Ok(()),
    };
    let result = declared.and_then(|()| interp.exec_block(&program.body, None).map(|_| ()));
    interp.finish(result)
}
