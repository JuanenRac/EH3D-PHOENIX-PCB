// SPDX-License-Identifier: GPL-3.0-or-later
//! `#include`: the program of another file put before the program that names it. Only files of the
//! one folder for files are read (`file_dir`), by a relative name inside it, with the same refusals
//! as every other file of a run. Each file is taken once however many times it is named, so two
//! files that name each other end; how deep the files may go, how many they may be and how many
//! bytes they may add are bounded.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::ast::{Directive, Pos, Program};
use crate::codes::{Diag, RUN_DEPTH_LIMIT, RUN_HAS_PROBLEMS, RUN_MEMORY_LIMIT, RUN_NOT_ALLOWED};
use crate::parser::parse;
use crate::runenv;
use crate::settings::UlpSettings;

struct Expansion<'a> {
    settings: &'a UlpSettings,
    taken: BTreeSet<PathBuf>,
    bytes: usize,
    out: Program,
}

fn at(pos: Pos, code: &'static str, detail: String) -> Diag {
    Diag::at(code, pos.line, pos.col, detail)
}

impl Expansion<'_> {
    fn file(&mut self, directive: &Directive, depth: usize) -> Result<(), Diag> {
        let pos = directive.pos;
        let Some(name) = directive.args.first() else {
            return Err(at(
                pos,
                RUN_NOT_ALLOWED,
                "`#include` without a name".to_owned(),
            ));
        };
        if depth >= self.settings.max_include_depth {
            return Err(at(
                pos,
                RUN_DEPTH_LIMIT,
                format!(
                    "more than {} files inside each other",
                    self.settings.max_include_depth
                ),
            ));
        }
        let refused = |(code, detail): (&'static str, String)| at(pos, code, detail);
        let root = runenv::root(&self.settings.file_dir).map_err(refused)?;
        let path = runenv::inside(&root, name).map_err(refused)?;
        runenv::confine(&root, &path).map_err(refused)?;
        if !self.taken.insert(path.clone()) {
            return Ok(());
        }
        if self.taken.len() > self.settings.max_files {
            return Err(at(
                pos,
                RUN_MEMORY_LIMIT,
                format!("more than {} files", self.settings.max_files),
            ));
        }
        let source =
            crate::read_text_limited(&path, self.settings.max_file_bytes).map_err(|e| {
                at(
                    pos,
                    e.code,
                    format!("`#include` of `{name}` cannot be read"),
                )
            })?;
        self.bytes = self.bytes.saturating_add(source.len());
        if self.bytes > self.settings.max_source_bytes {
            return Err(at(
                pos,
                RUN_MEMORY_LIMIT,
                format!(
                    "the included files are more than {} bytes",
                    self.settings.max_source_bytes
                ),
            ));
        }
        let parsed = parse(&source, self.settings)?;
        if let Some(first) = parsed.problems.list.first() {
            return Err(at(
                pos,
                RUN_HAS_PROBLEMS,
                format!(
                    "`{name}` has {} problems; the first is {} at line {}",
                    parsed.problems.total(),
                    first.code,
                    first.line
                ),
            ));
        }
        self.add(parsed.program, depth + 1)
    }

    /// The files that a program names first, then its own functions and statements.
    fn add(&mut self, program: Program, depth: usize) -> Result<(), Diag> {
        for directive in program.directives.iter().filter(|d| d.name == "include") {
            self.file(directive, depth)?;
        }
        self.out.functions.extend(program.functions);
        self.out.body.extend(program.body);
        Ok(())
    }
}

/// True when a program names another file.
pub(crate) fn needed(program: &Program) -> bool {
    program.directives.iter().any(|d| d.name == "include")
}

/// The program with the files it names put before it. The places in an error of a run are those
/// of the file the statement came from.
pub(crate) fn expand(program: &Program, settings: &UlpSettings) -> Result<Program, Diag> {
    let mut expansion = Expansion {
        settings,
        taken: BTreeSet::new(),
        bytes: 0,
        out: Program::default(),
    };
    expansion.add(program.clone(), 0)?;
    let mut out = expansion.out;
    out.directives = program
        .directives
        .iter()
        .filter(|d| d.name != "include")
        .cloned()
        .collect();
    let statements = out.statement_count();
    if statements > settings.max_statements {
        return Err(Diag::new(
            RUN_MEMORY_LIMIT,
            format!("{statements} statements with the included files"),
        ));
    }
    Ok(out)
}
