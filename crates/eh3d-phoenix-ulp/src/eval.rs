// SPDX-License-Identifier: GPL-3.0-or-later
//! The evaluator of the user programs of the owner's list: it runs the **pure** part of the
//! language (arithmetic, texts, arrays, functions the program defines, `if`, `for`, `while`, `do`,
//! `switch`) and the **read-only walks** over the design (`board(B)`, `B.elements(E)`, ...), with a
//! budget of steps, of memory, of nesting and of output.
//!
//! What it does not do is as much the point as what it does:
//!
//! * no file is opened, read or written, no program is started, nothing is asked of the network,
//!   the clock or the environment: a name that would need any of this is an error with the code
//!   `PHOENIX-ULP-034` (the list says it is denied or allowed with limits, and the evaluator
//!   gives no limit that opens anything);
//! * the model is never changed: the world holds shared references, and an assignment to a member
//!   of an object is the error `PHOENIX-ULP-038`;
//! * a name that is on the list but is not written here yet is the error `PHOENIX-ULP-035`, never a
//!   made-up result;
//! * the text a program prints (`printf`) is collected and returned; the caller decides what to do
//!   with it. The same program, the same model and the same limits give the same bytes.
//!
//! The run is **not** the sandbox of the decision record: it is a run in the process of the caller,
//! confined by what the evaluator can do and by its limits, not by the operating system. The
//! sandbox in a process of its own is not written. The run is done on a thread with a stack made
//! for the depth that the settings allow.

use std::collections::BTreeMap;

use crate::ast::{Decl, Declarator, Expr, ExprKind, Func, Init, Pos, Program, Stmt, StmtKind};
use crate::builtins::{self, format_c, real};
use crate::codes::{
    Diag, RUN_ARGUMENTS, RUN_DEPTH_LIMIT, RUN_DIVISION, RUN_INDEX, RUN_MEMORY_LIMIT, RUN_NAME,
    RUN_NOT_ALLOWED, RUN_NOT_WRITTEN, RUN_NO_MODEL, RUN_OUTPUT_LIMIT, RUN_RANGE, RUN_READ_ONLY,
    RUN_STEP_LIMIT, RUN_TYPE,
};
use crate::runenv::RunEnv;
use crate::settings::UlpSettings;
use crate::support::{Status, Support, UseKind};
use crate::value::{Ty, Value};
use crate::world::{Fault, World, WALKS};

mod io;
pub use io::{
    is_dialog_name, Control, Dialog, DLG_ACTIONS, DLG_CONTROLS, DLG_LAYOUTS, IO_FUNCTIONS,
};

/// What a run gives back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// What the program printed.
    pub output: String,
    /// The code the program ended with: 0 unless it called `exit` with another number.
    pub exit_code: i64,
    /// The steps the run took.
    pub steps: usize,
    /// The names of the files the run wrote, in the order it first wrote them.
    pub files: Vec<String>,
    /// The dialogs and message boxes the program built, as a model: nothing was shown.
    pub dialogs: Vec<Dialog>,
}

enum Halt {
    Err(Diag),
    Exit(i64),
}

type R<T> = Result<T, Halt>;

enum Flow {
    Next,
    Break,
    Continue,
    Return(Value),
}

struct Slot {
    name: String,
    ty: Ty,
    array: bool,
    numeric: bool,
    value: Value,
    size: usize,
}

#[derive(Clone, Copy)]
struct Place {
    slot: usize,
    index: Option<usize>,
}

/// The pieces of the machine that do not change during a run.
struct Machine<'a> {
    settings: &'a UlpSettings,
    support: &'a Support,
    world: World<'a>,
    funcs: BTreeMap<&'a str, &'a Func>,
    stack: Vec<Slot>,
    floor: usize,
    globals: BTreeMap<String, usize>,
    live: usize,
    steps: usize,
    depth: usize,
    out: String,
    env: &'a RunEnv,
    outfile: Option<io::OutFile>,
    file_error: i64,
    files_opened: usize,
    written: Vec<String>,
    dialogs: Vec<Dialog>,
    dialog: Option<usize>,
    dialog_depth: usize,
}

fn err(pos: Pos, code: &'static str, detail: impl Into<String>) -> Halt {
    Halt::Err(Diag::at(code, pos.line, pos.col, detail))
}

fn fault(pos: Pos, f: Fault) -> Halt {
    err(pos, f.0, f.1)
}

/// The named values of the language that the evaluator knows.
fn constant(name: &str) -> Option<Value> {
    Some(match name {
        "PI" => Value::Real(std::f64::consts::PI),
        "INT_MAX" => Value::Int(i64::MAX),
        "INT_MIN" => Value::Int(i64::MIN),
        "CHAR_MAX" => Value::Int(255),
        "CHAR_MIN" => Value::Int(0),
        "REAL_MAX" => Value::Real(f64::MAX),
        "REAL_MIN" => Value::Real(f64::MIN_POSITIVE),
        "REAL_EPSILON" => Value::Real(f64::EPSILON),
        "EOF" => Value::Int(-1),
        "EXIT_SUCCESS" => Value::Int(0),
        "EXIT_FAILURE" => Value::Int(1),
        // the level of the language the evaluator follows, for the programs that ask for it
        "EAGLE_VERSION" => Value::Int(9),
        "EAGLE_RELEASE" => Value::Int(5),
        "EAGLE_SIGNATURE" => Value::Str("EH3D PHOENIX ULP evaluator".to_owned()),
        _ => return None,
    })
}

/// The named values the evaluator knows.
pub const CONSTANTS: &[&str] = &[
    "PI",
    "INT_MAX",
    "INT_MIN",
    "CHAR_MAX",
    "CHAR_MIN",
    "REAL_MAX",
    "REAL_MIN",
    "REAL_EPSILON",
    "EOF",
    "EXIT_SUCCESS",
    "EXIT_FAILURE",
    "EAGLE_VERSION",
    "EAGLE_RELEASE",
    "EAGLE_SIGNATURE",
    "argc",
    "argv",
];

/// The names of the functions of the run that are not pure: they use the output or the variables.
pub const RUN_FUNCTIONS: &[&str] = &["printf", "exit", "sprintf", "strsplit"];

fn parse_int(text: &str) -> Option<i64> {
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => i64::from_str_radix(hex, 16).ok(),
        None => text.parse::<i64>().ok(),
    }
}

impl<'a> Machine<'a> {
    fn tick(&mut self, pos: Pos) -> R<()> {
        self.steps += 1;
        if self.steps > self.settings.max_steps {
            return Err(err(
                pos,
                RUN_STEP_LIMIT,
                format!("more than {} steps", self.settings.max_steps),
            ));
        }
        Ok(())
    }

    fn enter(&mut self, pos: Pos) -> R<()> {
        self.depth += 1;
        if self.depth > self.settings.max_eval_depth {
            return Err(err(
                pos,
                RUN_DEPTH_LIMIT,
                format!("more than {} levels", self.settings.max_eval_depth),
            ));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn charge(&mut self, pos: Pos, bytes: usize) -> R<()> {
        self.live += bytes;
        if self.live > self.settings.max_memory_bytes {
            return Err(err(
                pos,
                RUN_MEMORY_LIMIT,
                format!("more than {} bytes", self.settings.max_memory_bytes),
            ));
        }
        Ok(())
    }

    fn push(&mut self, pos: Pos, name: &str, ty: Ty, array: bool, value: Value) -> R<usize> {
        let size = value.size() + name.len() + 32;
        self.charge(pos, size)?;
        self.stack.push(Slot {
            name: name.to_owned(),
            ty,
            array,
            numeric: false,
            value,
            size,
        });
        Ok(self.stack.len() - 1)
    }

    fn pop_to(&mut self, len: usize) {
        while self.stack.len() > len {
            if let Some(slot) = self.stack.pop() {
                self.live = self.live.saturating_sub(slot.size);
            }
        }
    }

    fn find(&self, name: &str) -> Option<usize> {
        (self.floor..self.stack.len())
            .rev()
            .find(|i| self.stack[*i].name == name)
            .or_else(|| self.globals.get(name).copied())
    }

    fn coerce(&self, pos: Pos, ty: Ty, v: Value) -> R<Value> {
        let wrong = |v: &Value| {
            err(
                pos,
                RUN_TYPE,
                format!("a {} cannot be used as a {:?}", v.type_name(), ty),
            )
        };
        match (ty, v) {
            (Ty::Int, Value::Int(i)) => Ok(Value::Int(i)),
            (Ty::Int, Value::Real(r)) => {
                if r.abs() < 9.2e18 {
                    Ok(Value::Int(r as i64))
                } else {
                    Err(err(pos, RUN_RANGE, "the number is out of the range of int"))
                }
            }
            (Ty::Real, Value::Int(i)) => Ok(Value::Real(i as f64)),
            (Ty::Real, Value::Real(r)) => Ok(Value::Real(r)),
            (Ty::Str, Value::Str(s)) => Ok(Value::Str(s)),
            (Ty::Obj, v @ (Value::Obj(_) | Value::Void)) => Ok(v),
            (Ty::Void, Value::Void) => Ok(Value::Void),
            (_, other) => Err(wrong(&other)),
        }
    }

    fn truth(&self, pos: Pos, v: &Value) -> R<bool> {
        match v {
            Value::Int(i) => Ok(*i != 0),
            Value::Real(r) => Ok(*r != 0.0),
            other => Err(err(
                pos,
                RUN_TYPE,
                format!("a {} is not a condition", other.type_name()),
            )),
        }
    }

    // ---- places ---------------------------------------------------------------------------

    fn place(&mut self, e: &'a Expr) -> R<Place> {
        match &e.kind {
            ExprKind::Name(n) => {
                let slot = self
                    .find(n)
                    .ok_or_else(|| err(e.pos, RUN_NAME, format!("`{n}`")))?;
                Ok(Place { slot, index: None })
            }
            ExprKind::Index { base, index } => {
                let ExprKind::Name(n) = &base.kind else {
                    return Err(err(
                        e.pos,
                        RUN_TYPE,
                        "only an array variable can be indexed",
                    ));
                };
                let slot = self
                    .find(n)
                    .ok_or_else(|| err(e.pos, RUN_NAME, format!("`{n}`")))?;
                if !self.stack[slot].array {
                    return Err(err(e.pos, RUN_TYPE, format!("`{n}` is not an array")));
                }
                let i = self.eval(index)?;
                let Value::Int(i) = i else {
                    return Err(err(e.pos, RUN_TYPE, "an index is a whole number"));
                };
                let i = usize::try_from(i)
                    .map_err(|_| err(e.pos, RUN_INDEX, format!("the index {i} is negative")))?;
                Ok(Place {
                    slot,
                    index: Some(i),
                })
            }
            ExprKind::Member { .. } => Err(err(
                e.pos,
                RUN_READ_ONLY,
                "a member of an object cannot be assigned",
            )),
            _ => Err(err(e.pos, RUN_TYPE, "this cannot be assigned to")),
        }
    }

    fn load(&self, pos: Pos, p: Place) -> R<Value> {
        let slot = &self.stack[p.slot];
        match (&slot.value, p.index) {
            (v, None) => Ok(v.clone()),
            (Value::Array(items), Some(i)) => items.get(i).cloned().ok_or_else(|| {
                err(
                    pos,
                    RUN_INDEX,
                    format!("the index {i} of an array of {}", items.len()),
                )
            }),
            _ => Err(err(pos, RUN_TYPE, "this is not an array")),
        }
    }

    fn store(&mut self, pos: Pos, p: Place, v: Value) -> R<()> {
        let (ty, is_array) = (self.stack[p.slot].ty, self.stack[p.slot].array);
        let stored = match p.index {
            None => {
                if is_array {
                    return Err(err(pos, RUN_TYPE, "an array cannot be assigned as a whole"));
                }
                let v = self.coerce(pos, ty, v)?;
                self.stack[p.slot].value = v;
                None
            }
            Some(i) => {
                let v = self.coerce(pos, ty, v)?;
                if i >= self.settings.max_array_elements {
                    return Err(err(
                        pos,
                        RUN_MEMORY_LIMIT,
                        format!("more than {} elements", self.settings.max_array_elements),
                    ));
                }
                Some((i, v))
            }
        };
        if let Some((i, v)) = stored {
            let Value::Array(items) = &mut self.stack[p.slot].value else {
                return Err(err(pos, RUN_TYPE, "this is not an array"));
            };
            while items.len() <= i {
                items.push(ty.default_value());
            }
            items[i] = v;
        }
        self.resize(pos, p.slot)
    }

    /// Recounts the memory of one variable after it changed.
    fn resize(&mut self, pos: Pos, slot: usize) -> R<()> {
        let new = self.stack[slot].value.size() + self.stack[slot].name.len() + 32;
        let old = self.stack[slot].size;
        self.stack[slot].size = new;
        self.live = self.live.saturating_sub(old);
        self.charge(pos, new)
    }

    // ---- declarations ---------------------------------------------------------------------

    fn declare(&mut self, d: &'a Decl, pos: Pos, global: bool) -> R<()> {
        let ty = Ty::of(&d.ty);
        for n in &d.names {
            let slot = self.declarator(ty, n, pos)?;
            if global {
                self.globals.insert(n.name.clone(), slot);
            }
        }
        Ok(())
    }

    fn declarator(&mut self, ty: Ty, n: &'a Declarator, pos: Pos) -> R<usize> {
        let at = n.pos;
        let _ = pos;
        if n.array {
            let mut items = Vec::new();
            if let Some(size) = &n.size {
                let Value::Int(count) = self.eval(size)? else {
                    return Err(err(at, RUN_TYPE, "the size of an array is a whole number"));
                };
                let count = usize::try_from(count)
                    .map_err(|_| err(at, RUN_RANGE, "the size of an array is negative"))?;
                if count > self.settings.max_array_elements {
                    return Err(err(
                        at,
                        RUN_MEMORY_LIMIT,
                        format!("more than {} elements", self.settings.max_array_elements),
                    ));
                }
                items = vec![ty.default_value(); count];
            }
            match &n.init {
                Some(Init::List(list)) => {
                    if list.len() > self.settings.max_array_elements {
                        return Err(err(at, RUN_MEMORY_LIMIT, "too many elements"));
                    }
                    items.clear();
                    for e in list {
                        let v = self.eval(e)?;
                        items.push(self.coerce(e.pos, ty, v)?);
                    }
                }
                Some(_) => return Err(err(at, RUN_TYPE, "an array starts from a list")),
                None => {}
            }
            let slot = self.push(at, &n.name, ty, true, Value::Array(items))?;
            self.stack[slot].numeric = n.numeric;
            return Ok(slot);
        }
        let value = match &n.init {
            None => ty.default_value(),
            Some(Init::Value(e)) => {
                let v = self.eval(e)?;
                self.coerce(e.pos, ty, v)?
            }
            Some(Init::List(_)) => return Err(err(at, RUN_TYPE, "a list starts an array")),
            Some(Init::Block { call, body }) => {
                let v = self.dialog_value(at, call, body)?;
                self.coerce(at, ty, v)?
            }
        };
        self.push(at, &n.name, ty, false, value)
    }

    // ---- statements -----------------------------------------------------------------------

    fn exec_list(&mut self, list: &'a [Stmt]) -> R<Flow> {
        let mark = self.stack.len();
        let mut flow = Flow::Next;
        for s in list {
            flow = self.exec(s)?;
            if !matches!(flow, Flow::Next) {
                break;
            }
        }
        self.pop_to(mark);
        Ok(flow)
    }

    fn exec_scoped(&mut self, s: &'a Stmt) -> R<Flow> {
        let mark = self.stack.len();
        let flow = self.exec(s)?;
        self.pop_to(mark);
        Ok(flow)
    }

    fn exec(&mut self, s: &'a Stmt) -> R<Flow> {
        self.tick(s.pos)?;
        self.enter(s.pos)?;
        let flow = self.exec_inner(s);
        self.leave();
        flow
    }

    fn exec_inner(&mut self, s: &'a Stmt) -> R<Flow> {
        match &s.kind {
            StmtKind::Empty | StmtKind::Case(_) | StmtKind::Default => Ok(Flow::Next),
            StmtKind::Decl(d) => {
                self.declare(d, s.pos, false)?;
                Ok(Flow::Next)
            }
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(Flow::Next)
            }
            StmtKind::Loop { call, body } => self.walk(call, body, s.pos),
            StmtKind::Block(list) => self.exec_list(list),
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                let c = self.eval(cond)?;
                if self.truth(cond.pos, &c)? {
                    self.exec_scoped(then)
                } else if let Some(o) = otherwise {
                    self.exec_scoped(o)
                } else {
                    Ok(Flow::Next)
                }
            }
            StmtKind::While { cond, body } => {
                loop {
                    let c = self.eval(cond)?;
                    if !self.truth(cond.pos, &c)? {
                        break;
                    }
                    match self.exec_scoped(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next | Flow::Continue => {}
                    }
                }
                Ok(Flow::Next)
            }
            StmtKind::Do { body, cond } => {
                loop {
                    match self.exec_scoped(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next | Flow::Continue => {}
                    }
                    let c = self.eval(cond)?;
                    if !self.truth(cond.pos, &c)? {
                        break;
                    }
                }
                Ok(Flow::Next)
            }
            StmtKind::For {
                init,
                cond,
                step,
                body,
            } => {
                let mark = self.stack.len();
                if let Some(i) = init {
                    self.exec(i)?;
                }
                let mut result = Flow::Next;
                loop {
                    self.tick(s.pos)?;
                    if let Some(c) = cond {
                        let v = self.eval(c)?;
                        if !self.truth(c.pos, &v)? {
                            break;
                        }
                    }
                    match self.exec_scoped(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => {
                            result = Flow::Return(v);
                            break;
                        }
                        Flow::Next | Flow::Continue => {}
                    }
                    if let Some(st) = step {
                        self.eval(st)?;
                    }
                }
                self.pop_to(mark);
                Ok(result)
            }
            StmtKind::Switch { value, body } => self.switch(value, body),
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e)?,
                    None => Value::Void,
                };
                Ok(Flow::Return(v))
            }
        }
    }

    fn switch(&mut self, value: &'a Expr, body: &'a [Stmt]) -> R<Flow> {
        let v = self.eval(value)?;
        let mut start = None;
        let mut default = None;
        for (i, s) in body.iter().enumerate() {
            match &s.kind {
                StmtKind::Case(e) => {
                    let c = self.eval(e)?;
                    let same = match (&v, &c) {
                        (Value::Int(a), Value::Int(b)) => a == b,
                        (Value::Str(a), Value::Str(b)) => a == b,
                        (Value::Real(a), Value::Real(b)) => a == b,
                        (Value::Int(a), Value::Real(b)) | (Value::Real(b), Value::Int(a)) => {
                            (*a as f64) == *b
                        }
                        _ => {
                            return Err(err(
                                e.pos,
                                RUN_TYPE,
                                format!(
                                    "a case of a {} against a {}",
                                    c.type_name(),
                                    v.type_name()
                                ),
                            ))
                        }
                    };
                    if same {
                        start = Some(i);
                        break;
                    }
                }
                StmtKind::Default => default = default.or(Some(i)),
                _ => {}
            }
        }
        let Some(start) = start.or(default) else {
            return Ok(Flow::Next);
        };
        let mark = self.stack.len();
        let mut result = Flow::Next;
        for s in &body[start..] {
            match self.exec(s)? {
                Flow::Next => {}
                Flow::Break => break,
                other => {
                    result = other;
                    break;
                }
            }
        }
        self.pop_to(mark);
        Ok(result)
    }

    /// A call followed by a block: the walks over the design. Output files and dialogs are not
    /// run.
    fn walk(&mut self, call: &'a Expr, body: &'a [Stmt], pos: Pos) -> R<Flow> {
        if let Some(flow) = self.io_block(call, body)? {
            return Ok(flow);
        }
        let ExprKind::Call { callee, args } = &call.kind else {
            return Err(err(
                pos,
                RUN_NOT_WRITTEN,
                "a block after something that is not a call",
            ));
        };
        let var = |i: usize| -> R<&'a str> {
            match args.get(i).map(|a| &a.kind) {
                Some(ExprKind::Name(n)) => Ok(n.as_str()),
                _ => Err(err(
                    call.pos,
                    RUN_ARGUMENTS,
                    "a walk needs the name of a variable",
                )),
            }
        };
        match &callee.kind {
            ExprKind::Name(name) if name == "board" || name == "schematic" || name == "library" => {
                let root = self
                    .world
                    .root(name)
                    .map_err(|f| fault(call.pos, f))?
                    .ok_or_else(|| {
                        err(
                            call.pos,
                            RUN_NO_MODEL,
                            format!("{name}(...) needs a {name}"),
                        )
                    })?;
                self.bind_and_run(var(0)?, vec![root], body, call.pos)
            }
            ExprKind::Member { base, name } if WALKS.contains(&name.as_str()) => {
                let b = self.eval(base)?;
                let Value::Obj(obj) = b else {
                    return Err(err(
                        base.pos,
                        RUN_TYPE,
                        format!("a walk over a {}, which is not an object", b.type_name()),
                    ));
                };
                let items = self
                    .world
                    .walk(&obj, name)
                    .map_err(|f| fault(call.pos, f))?;
                self.bind_and_run(var(0)?, items, body, call.pos)
            }
            ExprKind::Name(name) if name == "sheet" => {
                let items = self
                    .world
                    .walk(&crate::world::Obj::Schematic, "sheets")
                    .map_err(|f| fault(call.pos, f))?;
                if self.world.schematic.is_none() {
                    return Err(err(call.pos, RUN_NO_MODEL, "sheet(...) needs a schematic"));
                }
                self.bind_and_run(var(0)?, items, body, call.pos)
            }
            ExprKind::Name(name) => Err(self.unavailable(call.pos, UseKind::Function, name)),
            ExprKind::Member { name, .. } => {
                Err(err(call.pos, RUN_NOT_WRITTEN, format!("the walk {name}")))
            }
            _ => Err(err(call.pos, RUN_TYPE, "this cannot be called")),
        }
    }

    fn bind_and_run(
        &mut self,
        var: &str,
        items: Vec<crate::world::Obj>,
        body: &'a [Stmt],
        pos: Pos,
    ) -> R<Flow> {
        let mark = self.stack.len();
        let slot = self.push(pos, var, Ty::Obj, false, Value::Void)?;
        let mut result = Flow::Next;
        for item in items {
            self.tick(pos)?;
            self.stack[slot].value = Value::Obj(item);
            match self.exec_list(body)? {
                Flow::Break => break,
                Flow::Return(v) => {
                    result = Flow::Return(v);
                    break;
                }
                Flow::Next | Flow::Continue => {}
            }
        }
        self.pop_to(mark);
        Ok(result)
    }

    /// The error for a name that the evaluator does not run, by what the list says about it.
    fn unavailable(&self, pos: Pos, kind: UseKind, name: &str) -> Halt {
        match self.support.lookup(kind, name) {
            Some(Status::Denied) => err(
                pos,
                RUN_NOT_ALLOWED,
                format!("`{name}` is denied by the list of names"),
            ),
            Some(Status::Limited) => err(
                pos,
                RUN_NOT_ALLOWED,
                format!("`{name}` needs a file or a folder, and the evaluator opens none"),
            ),
            Some(Status::Pending) => err(
                pos,
                RUN_NOT_WRITTEN,
                format!("`{name}` is pending in the list of names"),
            ),
            Some(Status::Allowed) => err(
                pos,
                RUN_NOT_WRITTEN,
                format!("`{name}` is allowed by the list but is not written in the evaluator"),
            ),
            None => err(
                pos,
                RUN_NAME,
                format!("`{name}` is not defined and is not on the list of names"),
            ),
        }
    }

    // ---- expressions ----------------------------------------------------------------------

    fn eval(&mut self, e: &'a Expr) -> R<Value> {
        self.tick(e.pos)?;
        self.enter(e.pos)?;
        let v = self.eval_inner(e);
        self.leave();
        v
    }

    fn eval_inner(&mut self, e: &'a Expr) -> R<Value> {
        let pos = e.pos;
        match &e.kind {
            ExprKind::Int(t) => parse_int(t).map(Value::Int).ok_or_else(|| {
                err(
                    pos,
                    RUN_RANGE,
                    format!("the number {t} is out of the range of int"),
                )
            }),
            ExprKind::Real(t) => t
                .parse::<f64>()
                .map_err(|_| err(pos, RUN_RANGE, format!("the number {t} cannot be read")))
                .and_then(|r| real(r).map_err(|f| fault(pos, f))),
            ExprKind::Char(t) => Ok(Value::Int(t.chars().next().map_or(0, |c| c as i64))),
            ExprKind::Str(t) => Ok(Value::Str(t.clone())),
            ExprKind::Name(n) => match self.find(n) {
                Some(slot) => Ok(self.stack[slot].value.clone()),
                None => constant(n).ok_or_else(|| err(pos, RUN_NAME, format!("`{n}`"))),
            },
            ExprKind::Index { base, index } => {
                if let ExprKind::Name(n) = &base.kind {
                    if let Some(slot) = self.find(n) {
                        if matches!(self.stack[slot].value, Value::Str(_)) {
                            let i = self.eval(index)?;
                            let Value::Int(i) = i else {
                                return Err(err(pos, RUN_TYPE, "an index is a whole number"));
                            };
                            let Value::Str(s) = &self.stack[slot].value else {
                                return Err(err(pos, RUN_TYPE, "this is not a text"));
                            };
                            return usize::try_from(i)
                                .ok()
                                .and_then(|i| s.chars().nth(i))
                                .map(|c| Value::Int(c as i64))
                                .ok_or_else(|| {
                                    err(pos, RUN_INDEX, format!("the index {i} of a text"))
                                });
                        }
                    }
                }
                let p = self.place(e)?;
                self.load(pos, p)
            }
            ExprKind::Member { base, name } => {
                let b = self.eval(base)?;
                match b {
                    Value::Obj(obj) => self.world.member(&obj, name).map_err(|f| fault(pos, f)),
                    other => Err(err(
                        pos,
                        RUN_TYPE,
                        format!("the member {name} of a {}", other.type_name()),
                    )),
                }
            }
            ExprKind::Call { callee, args } => self.call(pos, callee, args),
            ExprKind::Unary { op, operand } => match op.as_str() {
                "++" | "--" => {
                    let p = self.place(operand)?;
                    let cur = self.load(pos, p)?;
                    let next =
                        self.binary(pos, if op == "++" { "+" } else { "-" }, cur, Value::Int(1))?;
                    self.store(pos, p, next)?;
                    self.load(pos, p)
                }
                _ => {
                    let v = self.eval(operand)?;
                    match (op.as_str(), v) {
                        ("-", Value::Int(i)) => i.checked_neg().map(Value::Int).ok_or_else(|| {
                            err(pos, RUN_RANGE, "the result is out of the range of int")
                        }),
                        ("-", Value::Real(r)) => real(-r).map_err(|f| fault(pos, f)),
                        ("!", v) => Ok(Value::Int(i64::from(!self.truth(pos, &v)?))),
                        ("~", Value::Int(i)) => Ok(Value::Int(!i)),
                        (op, v) => Err(err(
                            pos,
                            RUN_TYPE,
                            format!("the operator {op} on a {}", v.type_name()),
                        )),
                    }
                }
            },
            ExprKind::Postfix { op, operand } => {
                let p = self.place(operand)?;
                let cur = self.load(pos, p)?;
                let next = self.binary(
                    pos,
                    if op == "++" { "+" } else { "-" },
                    cur.clone(),
                    Value::Int(1),
                )?;
                self.store(pos, p, next)?;
                Ok(cur)
            }
            ExprKind::Binary { op, left, right } => match op.as_str() {
                "," => {
                    self.eval(left)?;
                    self.eval(right)
                }
                "&&" => {
                    let l = self.eval(left)?;
                    if !self.truth(left.pos, &l)? {
                        return Ok(Value::Int(0));
                    }
                    let r = self.eval(right)?;
                    Ok(Value::Int(i64::from(self.truth(right.pos, &r)?)))
                }
                "||" => {
                    let l = self.eval(left)?;
                    if self.truth(left.pos, &l)? {
                        return Ok(Value::Int(1));
                    }
                    let r = self.eval(right)?;
                    Ok(Value::Int(i64::from(self.truth(right.pos, &r)?)))
                }
                _ => {
                    let l = self.eval(left)?;
                    let r = self.eval(right)?;
                    self.binary(pos, op, l, r)
                }
            },
            ExprKind::Assign { op, target, value } => {
                let p = self.place(target)?;
                let v = self.eval(value)?;
                let v = if op == "=" {
                    v
                } else {
                    let cur = self.load(pos, p)?;
                    self.binary(pos, &op[..op.len() - 1], cur, v)?
                };
                self.store(pos, p, v)?;
                self.load(pos, p)
            }
            ExprKind::Cond {
                cond,
                then,
                otherwise,
            } => {
                let c = self.eval(cond)?;
                if self.truth(cond.pos, &c)? {
                    self.eval(then)
                } else {
                    self.eval(otherwise)
                }
            }
        }
    }

    fn binary(&mut self, pos: Pos, op: &str, l: Value, r: Value) -> R<Value> {
        let range = || err(pos, RUN_RANGE, "the result is out of the range of int");
        let bad = |l: &Value, r: &Value| {
            err(
                pos,
                RUN_TYPE,
                format!(
                    "the operator {op} on a {} and a {}",
                    l.type_name(),
                    r.type_name()
                ),
            )
        };
        let flag = |b: bool| Ok(Value::Int(i64::from(b)));
        match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => {
                let (a, b) = (*a, *b);
                match op {
                    "+" => a.checked_add(b).map(Value::Int).ok_or_else(range),
                    "-" => a.checked_sub(b).map(Value::Int).ok_or_else(range),
                    "*" => a.checked_mul(b).map(Value::Int).ok_or_else(range),
                    "/" | "%" => {
                        if b == 0 {
                            return Err(err(pos, RUN_DIVISION, "a whole number divided by zero"));
                        }
                        let v = if op == "/" {
                            a.checked_div(b)
                        } else {
                            a.checked_rem(b)
                        };
                        v.map(Value::Int).ok_or_else(range)
                    }
                    "<<" | ">>" => {
                        if !(0..64).contains(&b) {
                            return Err(err(pos, RUN_RANGE, "a shift by a number outside 0 to 63"));
                        }
                        if op == "<<" {
                            a.checked_shl(b as u32).map(Value::Int).ok_or_else(range)
                        } else {
                            Ok(Value::Int(a >> b))
                        }
                    }
                    "&" => Ok(Value::Int(a & b)),
                    "|" => Ok(Value::Int(a | b)),
                    "^" => Ok(Value::Int(a ^ b)),
                    "==" => flag(a == b),
                    "!=" => flag(a != b),
                    "<" => flag(a < b),
                    ">" => flag(a > b),
                    "<=" => flag(a <= b),
                    ">=" => flag(a >= b),
                    _ => Err(bad(&l, &r)),
                }
            }
            (Value::Int(_) | Value::Real(_), Value::Int(_) | Value::Real(_)) => {
                let a = match &l {
                    Value::Int(i) => *i as f64,
                    Value::Real(x) => *x,
                    _ => 0.0,
                };
                let b = match &r {
                    Value::Int(i) => *i as f64,
                    Value::Real(x) => *x,
                    _ => 0.0,
                };
                match op {
                    "+" => real(a + b).map_err(|f| fault(pos, f)),
                    "-" => real(a - b).map_err(|f| fault(pos, f)),
                    "*" => real(a * b).map_err(|f| fault(pos, f)),
                    "/" => {
                        if b == 0.0 {
                            return Err(err(pos, RUN_DIVISION, "a number divided by zero"));
                        }
                        real(a / b).map_err(|f| fault(pos, f))
                    }
                    "==" => flag(a == b),
                    "!=" => flag(a != b),
                    "<" => flag(a < b),
                    ">" => flag(a > b),
                    "<=" => flag(a <= b),
                    ">=" => flag(a >= b),
                    _ => Err(bad(&l, &r)),
                }
            }
            (Value::Str(a), Value::Str(b)) => match op {
                "+" => {
                    let joined = a.len() + b.len();
                    if joined > self.settings.max_memory_bytes {
                        return Err(err(pos, RUN_MEMORY_LIMIT, "a text larger than the memory"));
                    }
                    Ok(Value::Str(format!("{a}{b}")))
                }
                "==" => flag(a == b),
                "!=" => flag(a != b),
                "<" => flag(a < b),
                ">" => flag(a > b),
                "<=" => flag(a <= b),
                ">=" => flag(a >= b),
                _ => Err(bad(&l, &r)),
            },
            (Value::Str(a), Value::Int(c)) if op == "+" => {
                let ch = u32::try_from(*c)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| err(pos, RUN_RANGE, "the code of a character is not valid"))?;
                Ok(Value::Str(format!("{a}{ch}")))
            }
            (Value::Obj(a), Value::Obj(b)) if op == "==" => flag(a == b),
            (Value::Obj(a), Value::Obj(b)) if op == "!=" => flag(a != b),
            _ => Err(bad(&l, &r)),
        }
    }

    // ---- calls ----------------------------------------------------------------------------

    fn call(&mut self, pos: Pos, callee: &'a Expr, args: &'a [Expr]) -> R<Value> {
        let ExprKind::Name(name) = &callee.kind else {
            return Err(err(
                pos,
                RUN_NOT_WRITTEN,
                "a call of a member outside a walk",
            ));
        };
        if let Some(f) = self.funcs.get(name.as_str()).copied() {
            return self.call_user(pos, f, args);
        }
        match name.as_str() {
            "sprintf" => return self.sprintf(pos, args),
            "strsplit" => return self.strsplit(pos, args),
            _ => {}
        }
        if let Some(v) = self.io_call(pos, name, args)? {
            return Ok(v);
        }
        let mut values = Vec::with_capacity(args.len());
        for a in args {
            values.push(self.eval(a)?);
        }
        match name.as_str() {
            "printf" => {
                let Some(Value::Str(f)) = values.first() else {
                    return Err(err(pos, RUN_ARGUMENTS, "printf needs a format text"));
                };
                let text = format_c(f, &values[1..]).map_err(|f| fault(pos, f))?;
                self.emit(pos, &text)?;
                Ok(Value::Int(text.len() as i64))
            }
            "exit" => match values.as_slice() {
                [] => Err(Halt::Exit(0)),
                [Value::Int(code)] => Err(Halt::Exit(*code)),
                [Value::Str(_)] => Err(err(
                    pos,
                    RUN_NOT_ALLOWED,
                    "`exit` with a command of the operating system is denied",
                )),
                _ => Err(err(pos, RUN_ARGUMENTS, "exit takes a whole number")),
            },
            other => match builtins::pure(other, &values).map_err(|f| fault(pos, f))? {
                Some(v) => Ok(v),
                None => Err(self.unavailable(pos, UseKind::Function, other)),
            },
        }
    }

    fn call_user(&mut self, pos: Pos, f: &'a Func, args: &'a [Expr]) -> R<Value> {
        if args.len() != f.params.len() {
            return Err(err(
                pos,
                RUN_ARGUMENTS,
                format!(
                    "`{}` takes {} arguments, not {}",
                    f.name,
                    f.params.len(),
                    args.len()
                ),
            ));
        }
        let mut values = Vec::with_capacity(args.len());
        for (a, p) in args.iter().zip(&f.params) {
            let v = self.eval(a)?;
            let ty = Ty::of(&p.ty);
            let v = if p.array {
                match v {
                    Value::Array(_) => v,
                    other => {
                        return Err(err(
                            a.pos,
                            RUN_TYPE,
                            format!("`{}` needs an array, not a {}", p.name, other.type_name()),
                        ))
                    }
                }
            } else {
                self.coerce(a.pos, ty, v)?
            };
            values.push(v);
        }
        self.enter(pos)?;
        let saved_floor = self.floor;
        let mark = self.stack.len();
        self.floor = mark;
        for (v, p) in values.into_iter().zip(&f.params) {
            self.push(pos, &p.name, Ty::of(&p.ty), p.array, v)?;
        }
        let flow = self.exec_list(&f.body);
        self.pop_to(mark);
        self.floor = saved_floor;
        self.leave();
        let ret = Ty::of(&f.ret);
        match flow? {
            Flow::Return(v) => {
                if ret == Ty::Void {
                    Ok(Value::Void)
                } else {
                    self.coerce(pos, ret, v)
                }
            }
            _ => Ok(ret.default_value()),
        }
    }

    fn sprintf(&mut self, pos: Pos, args: &'a [Expr]) -> R<Value> {
        if args.len() < 2 {
            return Err(err(
                pos,
                RUN_ARGUMENTS,
                "sprintf needs a text variable and a format",
            ));
        }
        let place = self.place(&args[0])?;
        if self.stack[place.slot].ty != Ty::Str {
            return Err(err(pos, RUN_TYPE, "sprintf writes into a text variable"));
        }
        let mut values = Vec::with_capacity(args.len() - 1);
        for a in &args[1..] {
            values.push(self.eval(a)?);
        }
        let Value::Str(f) = &values[0] else {
            return Err(err(pos, RUN_ARGUMENTS, "sprintf needs a format text"));
        };
        let text = format_c(f, &values[1..]).map_err(|f| fault(pos, f))?;
        let len = text.len() as i64;
        self.store(pos, place, Value::Str(text))?;
        Ok(Value::Int(len))
    }

    fn strsplit(&mut self, pos: Pos, args: &'a [Expr]) -> R<Value> {
        if args.len() != 3 {
            return Err(err(
                pos,
                RUN_ARGUMENTS,
                "strsplit takes an array, a text and a separator",
            ));
        }
        let place = self.place(&args[0])?;
        if !self.stack[place.slot].array
            || self.stack[place.slot].ty != Ty::Str
            || place.index.is_some()
        {
            return Err(err(pos, RUN_TYPE, "strsplit fills an array of texts"));
        }
        let (Value::Str(text), Value::Str(sep)) = (self.eval(&args[1])?, self.eval(&args[2])?)
        else {
            return Err(err(pos, RUN_TYPE, "strsplit splits a text at a text"));
        };
        let parts: Vec<Value> = if sep.is_empty() {
            vec![Value::Str(text)]
        } else {
            text.split(sep.as_str())
                .map(|p| Value::Str(p.to_owned()))
                .collect()
        };
        if parts.len() > self.settings.max_array_elements {
            return Err(err(pos, RUN_MEMORY_LIMIT, "too many elements"));
        }
        let count = parts.len() as i64;
        self.stack[place.slot].value = Value::Array(parts);
        self.resize(pos, place.slot)?;
        Ok(Value::Int(count))
    }

    // ---- the run --------------------------------------------------------------------------

    fn run(&mut self, program: &'a Program) -> R<()> {
        let here = Pos::default();
        let argc = self.env.argv.len() as i64;
        let slot = self.push(here, "argc", Ty::Int, false, Value::Int(argc))?;
        self.globals.insert("argc".to_owned(), slot);
        let words: Vec<Value> = self.env.argv.iter().cloned().map(Value::Str).collect();
        let slot = self.push(here, "argv", Ty::Str, true, Value::Array(words))?;
        self.globals.insert("argv".to_owned(), slot);
        for d in &program.directives {
            if d.name == "include" {
                return Err(err(
                    d.pos,
                    RUN_NOT_ALLOWED,
                    "`#include` reads a file, and the evaluator opens none",
                ));
            }
        }
        for s in &program.body {
            self.tick(s.pos)?;
            if let StmtKind::Decl(d) = &s.kind {
                self.declare(d, s.pos, true)?;
                continue;
            }
            match self.exec(s)? {
                Flow::Next => {}
                Flow::Return(_) => break,
                Flow::Break | Flow::Continue => {}
            }
        }
        Ok(())
    }
}

/// Runs a program that was read without problems.
///
/// A program with problems of reading must not be passed: the caller reports them
/// (`PHOENIX-ULP-039`). The run is done on a thread with a stack for `max_eval_depth` levels.
pub fn run(
    program: &Program,
    settings: &UlpSettings,
    support: &Support,
    world: World<'_>,
) -> Result<Outcome, Diag> {
    run_with(program, settings, support, world, &RunEnv::default())
}

/// Like [`run`], with the arguments, the clock and the folder for files of the caller.
pub fn run_with(
    program: &Program,
    settings: &UlpSettings,
    support: &Support,
    world: World<'_>,
    env: &RunEnv,
) -> Result<Outcome, Diag> {
    let stack_bytes = (1 << 20) + settings.max_eval_depth * (64 << 10);
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("ulp-run".to_owned())
            .stack_size(stack_bytes)
            .spawn_scoped(scope, || run_here(program, settings, support, world, env));
        match handle {
            Ok(h) => match h.join() {
                Ok(result) => result,
                Err(panic) => std::panic::resume_unwind(panic),
            },
            Err(e) => Err(Diag::new(
                RUN_DEPTH_LIMIT,
                format!("no stack for the run: {e}"),
            )),
        }
    })
}

fn run_here(
    program: &Program,
    settings: &UlpSettings,
    support: &Support,
    world: World<'_>,
    env: &RunEnv,
) -> Result<Outcome, Diag> {
    let mut machine = Machine {
        settings,
        support,
        world,
        funcs: program
            .functions
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect(),
        stack: Vec::new(),
        floor: 0,
        globals: BTreeMap::new(),
        live: 0,
        steps: 0,
        depth: 0,
        out: String::new(),
        env,
        outfile: None,
        file_error: 0,
        files_opened: 0,
        written: Vec::new(),
        dialogs: Vec::new(),
        dialog: None,
        dialog_depth: 0,
    };
    let result = machine.run(program);
    let mut exit_code = 0;
    match result {
        Ok(()) => {}
        Err(Halt::Exit(code)) => exit_code = code,
        Err(Halt::Err(d)) => return Err(d),
    }
    Ok(Outcome {
        output: machine.out,
        exit_code,
        steps: machine.steps,
        files: machine.written,
        dialogs: machine.dialogs,
    })
}

/// Reads a program and runs it. The problems of reading stop the run before it starts
/// (`PHOENIX-ULP-039`, with the first problem as the detail).
pub fn run_source(
    source: &str,
    settings: &UlpSettings,
    support: &Support,
    world: World<'_>,
) -> Result<Outcome, Diag> {
    run_source_with(source, settings, support, world, &RunEnv::default())
}

/// Like [`run_source`], with the arguments, the clock and the folder for files of the caller.
pub fn run_source_with(
    source: &str,
    settings: &UlpSettings,
    support: &Support,
    world: World<'_>,
    env: &RunEnv,
) -> Result<Outcome, Diag> {
    let parsed = crate::parser::parse(source, settings)?;
    if let Some(first) = parsed.problems.list.first() {
        return Err(Diag::at(
            crate::codes::RUN_HAS_PROBLEMS,
            first.line,
            first.col,
            format!(
                "{} problems; the first is {}",
                parsed.problems.total(),
                first.code
            ),
        ));
    }
    run_with(&parsed.program, settings, support, world, env)
}
