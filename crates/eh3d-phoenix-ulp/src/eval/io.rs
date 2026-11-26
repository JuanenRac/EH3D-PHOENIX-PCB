// SPDX-License-Identifier: GPL-3.0-or-later
//! The part of the evaluator that touches the world outside the model: the files of the one
//! folder the settings allow (`output`, `fileread`, `fileglob`, `filesize`, `filetime`), the clock
//! (`time`, `t2string`, ...) and the dialogs (`dlgDialog` and its controls), which are kept as a
//! model in memory and answer without a window.
//!
//! * A file is named with a relative name inside `file_dir`; a name that leaves it, a link and a
//!   folder that does not exist are refused. `output` collects what the block prints and writes
//!   it only when the block ends without an error.
//! * The clock is the one of the caller (`RunEnv::now`) or, when none was given, the machine's.
//!   Dates are in UTC.
//! * A dialog runs its layout, records every control with the nesting it has, does not run the
//!   actions of the buttons and answers as if the user had accepted it: `dlgDialog` is 1,
//!   `dlgMessageBox` is 0, the first button. The values of the variables are not changed.

use std::io::Read;
use std::path::PathBuf;

use super::*;
use crate::runenv::{self, civil, glob_match, t2string};

/// The controls of a dialog that stand alone, with the first argument they show.
pub const DLG_CONTROLS: &[&str] = &[
    "dlgLabel",
    "dlgStringEdit",
    "dlgIntEdit",
    "dlgRealEdit",
    "dlgTextEdit",
    "dlgTextView",
    "dlgCheckBox",
    "dlgRadioButton",
    "dlgComboBox",
    "dlgListBox",
    "dlgListView",
    "dlgPushButton",
    "dlgSpacing",
    "dlgStretch",
];

/// The controls that hold others.
pub const DLG_LAYOUTS: &[&str] = &[
    "dlgHBoxLayout",
    "dlgVBoxLayout",
    "dlgGridLayout",
    "dlgGroup",
    "dlgTabWidget",
    "dlgTabPage",
    "dlgCell",
];

/// The calls that act on a dialog and do nothing without a window.
pub const DLG_ACTIONS: &[&str] = &[
    "dlgAccept",
    "dlgReject",
    "dlgRedisplay",
    "dlgReset",
    "dlgSelectionChanged",
];

/// The names of the functions of this module that the evaluator runs.
pub const IO_FUNCTIONS: &[&str] = &[
    "output",
    "library",
    "time",
    "t2string",
    "t2year",
    "t2month",
    "t2day",
    "t2hour",
    "t2minute",
    "t2second",
    "t2dayofweek",
    "sort",
    "fileread",
    "fileglob",
    "filesize",
    "filetime",
    "fileerror",
    "status",
    "language",
    "country",
    "dlgDialog",
    "dlgMessageBox",
    "dlgFileOpen",
    "dlgFileSave",
    "dlgDirectory",
];

/// Whether the evaluator runs a function of a dialog by this name.
pub fn is_dialog_name(name: &str) -> bool {
    DLG_CONTROLS.contains(&name)
        || DLG_LAYOUTS.contains(&name)
        || DLG_ACTIONS.contains(&name)
        || name == "dlgDialog"
        || name == "dlgMessageBox"
        || matches!(name, "dlgFileOpen" | "dlgFileSave" | "dlgDirectory")
}

/// One control of a dialog as the program built it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    /// The name of the call: `dlgLabel`, `dlgGroup`, ...
    pub kind: String,
    /// What the control shows: its text, or the value of the variable it edits.
    pub text: String,
    /// How deep it stands inside the layouts of the dialog (the dialog itself is 0).
    pub depth: usize,
}

/// A dialog as the program built it, or a message box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    /// The title (the text of a message box).
    pub title: String,
    /// The controls, in the order the program made them.
    pub controls: Vec<Control>,
}

/// A file the program is writing: the text is kept until the block ends.
pub(super) struct OutFile {
    name: String,
    path: PathBuf,
    append: bool,
    text: String,
}

/// The largest number of dialogs one run may build.
const MAX_DIALOGS: usize = 1000;

fn show(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Real(r) => r.to_string(),
        Value::Str(s) => s.clone(),
        Value::Array(a) => format!("{} items", a.len()),
        Value::Obj(o) => o.kind().to_owned(),
        Value::Void => String::new(),
    }
}

impl<'a> Machine<'a> {
    fn file_path(&mut self, pos: Pos, name: &str) -> R<PathBuf> {
        let root = runenv::root(&self.settings.file_dir).map_err(|f| fault(pos, f))?;
        let path = runenv::inside(&root, name).map_err(|f| fault(pos, f))?;
        runenv::confine(&root, &path).map_err(|f| fault(pos, f))?;
        Ok(path)
    }

    fn count_file(&mut self, pos: Pos) -> R<()> {
        self.files_opened += 1;
        if self.files_opened > self.settings.max_files {
            return Err(err(
                pos,
                RUN_MEMORY_LIMIT,
                format!("more than {} files", self.settings.max_files),
            ));
        }
        Ok(())
    }

    fn text_arg(&mut self, pos: Pos, args: &'a [Expr], i: usize, what: &str) -> R<String> {
        match args.get(i) {
            Some(a) => match self.eval(a)? {
                Value::Str(s) => Ok(s),
                other => Err(err(
                    pos,
                    RUN_TYPE,
                    format!("{what} is a text, not a {}", other.type_name()),
                )),
            },
            None => Err(err(pos, RUN_ARGUMENTS, format!("{what} is missing"))),
        }
    }

    fn int_arg(&mut self, pos: Pos, args: &'a [Expr], i: usize, what: &str) -> R<i64> {
        match args.get(i) {
            Some(a) => match self.eval(a)? {
                Value::Int(n) => Ok(n),
                other => Err(err(
                    pos,
                    RUN_TYPE,
                    format!("{what} is a whole number, not a {}", other.type_name()),
                )),
            },
            None => Err(err(pos, RUN_ARGUMENTS, format!("{what} is missing"))),
        }
    }

    /// The calls of this module that are not followed by a block. `None`: not one of them.
    pub(super) fn io_call(&mut self, pos: Pos, name: &str, args: &'a [Expr]) -> R<Option<Value>> {
        let value = match name {
            "time" => {
                if !args.is_empty() {
                    return Err(err(pos, RUN_ARGUMENTS, "time takes no arguments"));
                }
                Value::Int(self.env.clock())
            }
            "t2string" => {
                let t = self.int_arg(pos, args, 0, "the time")?;
                let format = if args.len() > 1 {
                    self.text_arg(pos, args, 1, "the format")?
                } else {
                    "yyyy-MM-dd hh:mm:ss".to_owned()
                };
                Value::Str(t2string(t, &format))
            }
            "t2year" | "t2month" | "t2day" | "t2hour" | "t2minute" | "t2second" | "t2dayofweek" => {
                let c = civil(self.int_arg(pos, args, 0, "the time")?);
                Value::Int(match name {
                    "t2year" => c.year,
                    "t2month" => c.month,
                    "t2day" => c.day,
                    "t2hour" => c.hour,
                    "t2minute" => c.minute,
                    "t2second" => c.second,
                    _ => c.weekday,
                })
            }
            "fileerror" => Value::Int(self.file_error),
            "status" => {
                for a in args {
                    self.eval(a)?;
                }
                Value::Int(0)
            }
            "language" => Value::Str("en".to_owned()),
            // fixed like the language: the clock and the places of the machine are not asked
            "country" => Value::Str("US".to_owned()),
            "filesize" | "filetime" => {
                let file = self.text_arg(pos, args, 0, "the name of the file")?;
                let path = self.file_path(pos, &file)?;
                self.count_file(pos)?;
                match std::fs::metadata(&path) {
                    Ok(m) if m.is_file() => {
                        self.file_error = 0;
                        if name == "filesize" {
                            Value::Int(i64::try_from(m.len()).unwrap_or(i64::MAX))
                        } else {
                            Value::Int(
                                m.modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0)),
                            )
                        }
                    }
                    _ => {
                        self.file_error = 1;
                        Value::Int(-1)
                    }
                }
            }
            "sort" => self.sort(pos, args)?,
            "fileread" => self.fileread(pos, args)?,
            "fileglob" => self.fileglob(pos, args)?,
            "dlgMessageBox" => {
                let text = self.text_arg(pos, args, 0, "the text")?;
                let mut controls = Vec::new();
                for i in 1..args.len() {
                    let b = self.text_arg(pos, args, i, "a button")?;
                    controls.push(Control {
                        kind: "button".to_owned(),
                        text: b,
                        depth: 1,
                    });
                }
                self.record_dialog(
                    pos,
                    Dialog {
                        title: text,
                        controls,
                    },
                )?;
                Value::Int(0)
            }
            "dlgFileOpen" | "dlgFileSave" | "dlgDirectory" => {
                // a window to choose a file is not there: the choice is cancelled
                for a in args {
                    self.eval(a)?;
                }
                Value::Str(String::new())
            }
            n if DLG_ACTIONS.contains(&n) => {
                for a in args {
                    self.eval(a)?;
                }
                Value::Int(0)
            }
            n if DLG_CONTROLS.contains(&n) => {
                let mut first = String::new();
                for (i, a) in args.iter().enumerate() {
                    let v = self.eval(a)?;
                    if i == 0 {
                        first = show(&v);
                    }
                }
                self.control(pos, n, first)?;
                Value::Int(0)
            }
            n if DLG_LAYOUTS.contains(&n) || n == "dlgDialog" || n == "output" => {
                return Err(err(
                    pos,
                    RUN_NOT_WRITTEN,
                    format!("`{n}` holds a block, and here it has none"),
                ))
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    /// `sort(n, key[], other[], ...)`: puts the first `n` elements of the arrays in the order of the
    /// first array, ascending and keeping the order of equal ones. An array of numbers or one
    /// declared `numeric` is ordered as numbers, any other array of texts by its bytes.
    fn sort(&mut self, pos: Pos, args: &'a [Expr]) -> R<Value> {
        if args.len() < 2 {
            return Err(err(
                pos,
                RUN_ARGUMENTS,
                "sort takes a count and one or more arrays",
            ));
        }
        let count = self.int_arg(pos, args, 0, "the count")?;
        let mut slots = Vec::new();
        for a in &args[1..] {
            let place = self.place(a)?;
            if !self.stack[place.slot].array || place.index.is_some() {
                return Err(err(pos, RUN_TYPE, "sort orders arrays"));
            }
            slots.push(place.slot);
        }
        let len = |m: &Self, s: usize| match &m.stack[s].value {
            Value::Array(v) => v.len(),
            _ => 0,
        };
        let n = usize::try_from(count)
            .map_err(|_| err(pos, RUN_RANGE, "the count is negative"))?
            .min(slots.iter().map(|s| len(self, *s)).min().unwrap_or(0));
        let key = slots[0];
        let numeric = self.stack[key].numeric || self.stack[key].ty != Ty::Str;
        let Value::Array(keys) = &self.stack[key].value else {
            return Err(err(pos, RUN_TYPE, "sort orders arrays"));
        };
        let number = |v: &Value| match v {
            Value::Int(i) => *i as f64,
            Value::Real(r) => *r,
            Value::Str(s) => s.trim().parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        };
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|a, b| {
            if numeric {
                number(&keys[*a]).total_cmp(&number(&keys[*b]))
            } else {
                match (&keys[*a], &keys[*b]) {
                    (Value::Str(x), Value::Str(y)) => x.as_bytes().cmp(y.as_bytes()),
                    _ => std::cmp::Ordering::Equal,
                }
            }
        });
        for s in slots {
            if let Value::Array(items) = &mut self.stack[s].value {
                let moved: Vec<Value> = order.iter().map(|i| items[*i].clone()).collect();
                for (i, v) in moved.into_iter().enumerate() {
                    items[i] = v;
                }
            }
        }
        Ok(Value::Int(0))
    }

    fn fileread(&mut self, pos: Pos, args: &'a [Expr]) -> R<Value> {
        if args.len() != 2 {
            return Err(err(
                pos,
                RUN_ARGUMENTS,
                "fileread takes a text or an array of texts, and a name",
            ));
        }
        let place = self.place(&args[0])?;
        let slot = &self.stack[place.slot];
        if slot.ty != Ty::Str || place.index.is_some() {
            return Err(err(
                pos,
                RUN_TYPE,
                "fileread fills a text or an array of texts",
            ));
        }
        let array = slot.array;
        let file = self.text_arg(pos, args, 1, "the name of the file")?;
        let path = self.file_path(pos, &file)?;
        self.count_file(pos)?;
        let limit = self.settings.max_file_bytes;
        let mut bytes = Vec::new();
        let read = std::fs::File::open(&path)
            .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut bytes));
        if read.is_err() {
            self.file_error = 1;
            return Ok(Value::Int(-1));
        }
        if bytes.len() > limit {
            return Err(err(
                pos,
                RUN_MEMORY_LIMIT,
                format!("the file has more than {limit} bytes"),
            ));
        }
        self.file_error = 0;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if array {
            let mut lines: Vec<Value> = text
                .split('\n')
                .map(|l| Value::Str(l.strip_suffix('\r').unwrap_or(l).to_owned()))
                .collect();
            if text.ends_with('\n') || text.is_empty() {
                lines.pop();
            }
            if lines.len() > self.settings.max_array_elements {
                return Err(err(pos, RUN_MEMORY_LIMIT, "too many lines"));
            }
            let count = lines.len() as i64;
            self.stack[place.slot].value = Value::Array(lines);
            self.resize(pos, place.slot)?;
            Ok(Value::Int(count))
        } else {
            let len = text.len() as i64;
            self.stack[place.slot].value = Value::Str(text);
            self.resize(pos, place.slot)?;
            Ok(Value::Int(len))
        }
    }

    fn fileglob(&mut self, pos: Pos, args: &'a [Expr]) -> R<Value> {
        if args.len() != 2 {
            return Err(err(
                pos,
                RUN_ARGUMENTS,
                "fileglob takes an array of texts and a pattern",
            ));
        }
        let place = self.place(&args[0])?;
        if !self.stack[place.slot].array
            || self.stack[place.slot].ty != Ty::Str
            || place.index.is_some()
        {
            return Err(err(pos, RUN_TYPE, "fileglob fills an array of texts"));
        }
        let pattern = self.text_arg(pos, args, 1, "the pattern")?;
        let (dir, leaf) = match pattern.rsplit_once('/') {
            Some((d, l)) => (d.to_owned(), l.to_owned()),
            None => (String::new(), pattern.clone()),
        };
        let root = runenv::root(&self.settings.file_dir).map_err(|f| fault(pos, f))?;
        let folder = if dir.is_empty() {
            root.clone()
        } else {
            let p = runenv::inside(&root, &dir).map_err(|f| fault(pos, f))?;
            runenv::confine(&root, &p.join("x")).map_err(|f| fault(pos, f))?;
            p
        };
        self.count_file(pos)?;
        let mut names = Vec::new();
        match std::fs::read_dir(&folder) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let Some(n) = entry.file_name().to_str().map(str::to_owned) else {
                        continue;
                    };
                    if glob_match(&leaf, &n) && !entry.file_type().is_ok_and(|t| t.is_symlink()) {
                        names.push(if dir.is_empty() {
                            n
                        } else {
                            format!("{dir}/{n}")
                        });
                    }
                }
                self.file_error = 0;
            }
            Err(_) => self.file_error = 1,
        }
        names.sort();
        if names.len() > self.settings.max_array_elements {
            return Err(err(pos, RUN_MEMORY_LIMIT, "too many files"));
        }
        let count = names.len() as i64;
        self.stack[place.slot].value = Value::Array(names.into_iter().map(Value::Str).collect());
        self.resize(pos, place.slot)?;
        Ok(Value::Int(count))
    }

    /// What `printf` does with a text: into the file that `output` is writing, or into the output
    /// of the run.
    pub(super) fn emit(&mut self, pos: Pos, text: &str) -> R<()> {
        if let Some(f) = self.outfile.as_mut() {
            if f.text.len() + text.len() > self.settings.max_file_bytes {
                return Err(err(
                    pos,
                    RUN_OUTPUT_LIMIT,
                    format!("a file of more than {} bytes", self.settings.max_file_bytes),
                ));
            }
            f.text.push_str(text);
            return Ok(());
        }
        if self.out.len() + text.len() > self.settings.max_output_bytes {
            return Err(err(
                pos,
                RUN_OUTPUT_LIMIT,
                format!("more than {} bytes", self.settings.max_output_bytes),
            ));
        }
        self.out.push_str(text);
        Ok(())
    }

    /// A call followed by a block, when it is one of this module. `None`: it is not.
    pub(super) fn io_block(&mut self, call: &'a Expr, body: &'a [Stmt]) -> R<Option<Flow>> {
        let (name, args): (&str, &[Expr]) = match &call.kind {
            ExprKind::Call { callee, args } => match &callee.kind {
                ExprKind::Name(n) => (n.as_str(), args.as_slice()),
                _ => return Ok(None),
            },
            ExprKind::Name(n) => (n.as_str(), &[]),
            _ => return Ok(None),
        };
        let pos = call.pos;
        match name {
            "output" => self.output_block(pos, args, body).map(Some),
            "dlgDialog" => {
                self.dialog_block(pos, args, body)?;
                Ok(Some(Flow::Next))
            }
            n if DLG_LAYOUTS.contains(&n) => {
                if self.dialog.is_none() {
                    return Err(err(pos, RUN_TYPE, format!("`{n}` outside a dialog")));
                }
                let mut first = String::new();
                for (i, a) in args.iter().enumerate() {
                    let v = self.eval(a)?;
                    if i == 0 {
                        first = show(&v);
                    }
                }
                self.control(pos, n, first)?;
                self.dialog_depth += 1;
                let r = self.exec_list(body);
                self.dialog_depth -= 1;
                r.map(Some)
            }
            n if DLG_CONTROLS.contains(&n) => {
                // a control with the action it runs when used: recorded, never run
                let mut first = String::new();
                for (i, a) in args.iter().enumerate() {
                    let v = self.eval(a)?;
                    if i == 0 {
                        first = show(&v);
                    }
                }
                self.control(pos, n, first)?;
                Ok(Some(Flow::Next))
            }
            _ => Ok(None),
        }
    }

    fn output_block(&mut self, pos: Pos, args: &'a [Expr], body: &'a [Stmt]) -> R<Flow> {
        if self.outfile.is_some() {
            return Err(err(pos, RUN_NOT_ALLOWED, "output blocks do not nest"));
        }
        let name = self.text_arg(pos, args, 0, "the name of the file")?;
        let mode = if args.len() > 1 {
            self.text_arg(pos, args, 1, "the mode")?
        } else {
            "wt".to_owned()
        };
        let append = match mode.as_str() {
            "w" | "wt" | "wb" => false,
            "a" | "at" | "ab" => true,
            _ => {
                return Err(err(
                    pos,
                    RUN_ARGUMENTS,
                    "the mode is w, wt, wb, a, at or ab",
                ))
            }
        };
        let path = self.file_path(pos, &name)?;
        self.count_file(pos)?;
        self.outfile = Some(OutFile {
            name,
            path,
            append,
            text: String::new(),
        });
        let result = self.exec_list(body);
        let file = self.outfile.take();
        match result {
            Ok(flow) => {
                if let Some(f) = file {
                    self.flush(pos, f)?;
                }
                Ok(flow)
            }
            Err(Halt::Exit(code)) => {
                if let Some(f) = file {
                    self.flush(pos, f)?;
                }
                Err(Halt::Exit(code))
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self, pos: Pos, f: OutFile) -> R<()> {
        use std::io::Write;
        if f.append {
            let before = std::fs::metadata(&f.path).map_or(0, |m| m.len());
            if before.saturating_add(f.text.len() as u64) > self.settings.max_file_bytes as u64 {
                return Err(err(
                    pos,
                    RUN_OUTPUT_LIMIT,
                    format!("a file of more than {} bytes", self.settings.max_file_bytes),
                ));
            }
        }
        let done = if f.append {
            std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&f.path)
                .and_then(|mut h| h.write_all(f.text.as_bytes()))
        } else {
            std::fs::write(&f.path, f.text.as_bytes())
        };
        if done.is_ok() {
            self.file_error = 0;
            if !self.written.contains(&f.name) {
                self.written.push(f.name);
            }
        } else {
            self.file_error = 1;
        }
        Ok(())
    }

    fn record_dialog(&mut self, pos: Pos, d: Dialog) -> R<()> {
        if self.dialogs.len() >= MAX_DIALOGS {
            return Err(err(pos, RUN_MEMORY_LIMIT, "too many dialogs"));
        }
        self.dialogs.push(d);
        Ok(())
    }

    fn dialog_block(&mut self, pos: Pos, args: &'a [Expr], body: &'a [Stmt]) -> R<()> {
        if self.dialog.is_some() {
            return Err(err(pos, RUN_NOT_ALLOWED, "dialogs do not nest"));
        }
        let title = if args.is_empty() {
            String::new()
        } else {
            self.text_arg(pos, args, 0, "the title")?
        };
        self.record_dialog(
            pos,
            Dialog {
                title,
                controls: Vec::new(),
            },
        )?;
        self.dialog = Some(self.dialogs.len() - 1);
        self.dialog_depth = 1;
        let r = self.exec_list(body);
        self.dialog = None;
        self.dialog_depth = 0;
        r.map(|_| ())
    }

    /// The value of a dialog that has no window: it was accepted.
    pub(super) fn dialog_value(&mut self, pos: Pos, call: &'a Expr, body: &'a [Stmt]) -> R<Value> {
        let ExprKind::Call { callee, args } = &call.kind else {
            return Err(err(pos, RUN_TYPE, "this cannot be called"));
        };
        match &callee.kind {
            ExprKind::Name(n) if n == "dlgDialog" => {
                self.dialog_block(pos, args, body)?;
                Ok(Value::Int(1))
            }
            ExprKind::Name(n) => Err(self.unavailable(pos, UseKind::Function, n)),
            _ => Err(err(pos, RUN_TYPE, "this cannot be called")),
        }
    }

    fn control(&mut self, pos: Pos, kind: &str, text: String) -> R<()> {
        let Some(i) = self.dialog else {
            return Err(err(pos, RUN_TYPE, format!("`{kind}` outside a dialog")));
        };
        let total: usize = self.dialogs.iter().map(|d| d.controls.len()).sum();
        if total >= self.settings.max_array_elements {
            return Err(err(pos, RUN_MEMORY_LIMIT, "too many controls"));
        }
        let depth = self.dialog_depth;
        self.dialogs[i].controls.push(Control {
            kind: kind.to_owned(),
            text,
            depth,
        });
        Ok(())
    }
}
