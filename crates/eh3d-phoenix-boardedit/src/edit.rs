// SPDX-License-Identifier: GPL-3.0-or-later
//! The board editor: a board with its rules, the locks, the history of the operations and the
//! design rule check after each one. An operation is planned, tried on a copy of the board, checked
//! by the rules and only then kept; one that the board refuses, or that adds a rule error (unless
//! it is forced), leaves everything as it was.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, Id, Operation, Step};
use eh3d_phoenix_lengths::{measure_nets, LengthSettings};
use eh3d_phoenix_route::{diff, findings, ratsnest, RatLine, RuleContext};
use eh3d_phoenix_rules::{title, Finding, Lang, Severity};
use serde_json::{json, Value};

use crate::codes::{BoardEditError, OPERATION_REFUSED, REJECTED_BY_DRC, TUNE_IMPOSSIBLE};
use crate::ids::IdSupply;
use crate::ops::EditOp;
use crate::pairs::PairConfig;
use crate::plan::{plan, Env};
use crate::settings::BoardEditSettings;

fn error_count(list: &[Finding]) -> usize {
    list.iter()
        .filter(|f| f.severity == Severity::Error)
        .count()
}

/// What the design rules say changed because of an operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrcReport {
    /// The rule errors before.
    pub errors_before: usize,
    /// The rule errors after.
    pub errors_after: usize,
    /// The findings the operation added.
    pub added: Vec<Finding>,
    /// The findings the operation removed.
    pub removed: Vec<Finding>,
}

impl DrcReport {
    /// The report of the difference between two lists of findings.
    pub fn new(before: &[Finding], after: &[Finding]) -> DrcReport {
        let changes = diff(before, after);
        DrcReport {
            errors_before: error_count(before),
            errors_after: error_count(after),
            added: changes.added,
            removed: changes.removed,
        }
    }

    /// True when the operation added a finding that is an error.
    pub fn adds_error(&self) -> bool {
        self.added.iter().any(|f| f.severity == Severity::Error)
    }

    /// The report as text, with the titles of the rules in `lang`.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {} -> {}\n",
            word(Word::RuleErrors, lang),
            self.errors_before,
            self.errors_after
        );
        for (label, list) in [(Word::Added, &self.added), (Word::Removed, &self.removed)] {
            if list.is_empty() {
                continue;
            }
            out.push_str(&format!("{}: {}\n", word(label, lang), list.len()));
            for f in list {
                out.push_str(&finding_line(f, lang));
                out.push('\n');
            }
        }
        out
    }

    /// The report as a JSON value.
    pub fn to_value(&self) -> Value {
        let list = |items: &[Finding]| -> Vec<Value> {
            items
                .iter()
                .map(|f| {
                    json!({
                        "code": f.code,
                        "severity": f.severity.as_str(),
                        "location": f.location,
                        "at_nm": f.at_nm,
                        "detail": f.detail,
                    })
                })
                .collect()
        };
        json!({
            "errors_before": self.errors_before,
            "errors_after": self.errors_after,
            "added": list(&self.added),
            "removed": list(&self.removed),
        })
    }
}

fn finding_line(f: &Finding, lang: Lang) -> String {
    let mut line = format!(
        "  [{}] {} {}: {}",
        f.severity.as_str(),
        f.code,
        title(f.code, lang).unwrap_or(""),
        f.location
    );
    if let Some([x, y]) = f.at_nm {
        line.push_str(&format!(" | at ({x}, {y}) nm"));
    }
    if !f.detail.is_empty() {
        line.push_str(&format!(" | {}", f.detail));
    }
    line
}

#[derive(Clone, Copy)]
enum Word {
    RuleErrors,
    Added,
    Removed,
}

/// The words of the report in the eight languages.
fn word(word: Word, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    let row: [&str; 8] = match word {
        Word::RuleErrors => [
            "rule errors",
            "errores de reglas",
            "Regelfehler",
            "erreurs de règles",
            "errori di regole",
            "規則エラー",
            "规则错误",
            "erros de regras",
        ],
        Word::Added => [
            "added by the operations",
            "añadidos por las operaciones",
            "von den Operationen hinzugefügt",
            "ajoutés par les opérations",
            "aggiunti dalle operazioni",
            "操作で追加",
            "由操作增加",
            "acrescentados pelas operações",
        ],
        Word::Removed => [
            "removed by the operations",
            "quitados por las operaciones",
            "von den Operationen entfernt",
            "retirés par les opérations",
            "rimossi dalle operazioni",
            "操作で解消",
            "由操作消除",
            "removidos pelas operações",
        ],
    };
    row[column]
}

/// What an operation did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    /// The ids of the parts that were made.
    pub created: Vec<Id>,
    /// Things to tell the person.
    pub notes: Vec<String>,
    /// What the rules say changed.
    pub drc: DrcReport,
    /// False when the operation left the board as it was (a lock, or a net already at its length).
    pub changed: bool,
    /// True when the operation was forced and adds a rule error.
    pub forced: bool,
    /// The copper length of the tuned net before and after, in nanometres.
    pub length_nm: Option<(i64, i64)>,
}

/// The copper length of a net, in nanometres.
fn net_length(board: &Board, net: Id) -> Option<i64> {
    let (lengths, _) = measure_nets(board, &LengthSettings::default(), None).ok()?;
    lengths
        .into_iter()
        .find(|n| n.net == net)
        .map(|n| n.total_nm)
}

/// The board editor.
#[derive(Debug, Clone)]
pub struct BoardEdit {
    board: Board,
    ctx: RuleContext,
    /// The settings; the window changes the grid and the checks.
    pub settings: BoardEditSettings,
    ids: IdSupply,
    done: Vec<Step>,
    undone: Vec<Step>,
    findings: Vec<Finding>,
    locked: BTreeSet<Id>,
    pairs: PairConfig,
    revision: u64,
}

impl BoardEdit {
    /// An editor on a board, under rules.
    pub fn new(
        board: Board,
        ctx: RuleContext,
        settings: BoardEditSettings,
    ) -> Result<BoardEdit, BoardEditError> {
        let found = if settings.live_drc {
            findings(&board, &ctx)
        } else {
            Vec::new()
        };
        Ok(BoardEdit {
            board,
            ctx,
            settings,
            ids: IdSupply::new()?,
            done: Vec::new(),
            undone: Vec::new(),
            findings: found,
            locked: BTreeSet::new(),
            pairs: PairConfig::default(),
            revision: 0,
        })
    }

    /// The editor with the differential pairs and the settings of the pairs that the pair
    /// operations use.
    pub fn with_pairs(mut self, pairs: PairConfig) -> BoardEdit {
        self.pairs = pairs;
        self
    }

    /// The differential pairs and their settings.
    pub fn pair_config(&self) -> &PairConfig {
        &self.pairs
    }

    /// Changes the differential pairs and their settings.
    pub fn set_pair_config(&mut self, pairs: PairConfig) {
        self.pairs = pairs;
    }

    /// The board as it is now.
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// The board, when the editor is done with it.
    pub fn into_board(self) -> Board {
        self.board
    }

    /// The rules the board is edited under.
    pub fn context(&self) -> &RuleContext {
        &self.ctx
    }

    /// The findings of the rules on the board as it is now (none when the check is off).
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// The components that are locked.
    pub fn locked(&self) -> &BTreeSet<Id> {
        &self.locked
    }

    /// True when a component is locked.
    pub fn is_locked(&self, id: Id) -> bool {
        self.locked.contains(&id)
    }

    /// A number that grows with every change of the board, to tell a window when to draw again.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True when there is a step to undo.
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    /// True when there is a step to redo.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// How many steps are done.
    pub fn steps(&self) -> usize {
        self.done.len()
    }

    /// The connections still to be routed (the pads that the schematic put on a net and no copper
    /// joins yet). Empty when the ratsnest cannot be made.
    pub fn ratsnest(&self) -> Vec<RatLine> {
        ratsnest(&self.board, &self.settings.route_settings()).unwrap_or_default()
    }

    /// The copper length of a net now, in nanometres.
    pub fn net_length_nm(&self, net: Id) -> Option<i64> {
        net_length(&self.board, net)
    }

    fn recheck(&mut self) {
        self.revision += 1;
        if self.settings.live_drc {
            self.findings = findings(&self.board, &self.ctx);
        }
    }

    /// Undoes the last step; false when there is none.
    pub fn undo(&mut self) -> bool {
        let Some(step) = self.done.pop() else {
            return false;
        };
        if self.board.apply(&step.inverse).is_err() {
            self.done.push(step);
            return false;
        }
        self.undone.push(step);
        self.recheck();
        true
    }

    /// Redoes the last undone step; false when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(step) = self.undone.pop() else {
            return false;
        };
        if self.board.apply(&step.op).is_err() {
            self.undone.push(step);
            return false;
        }
        self.done.push(step);
        self.recheck();
        true
    }

    /// Applies an operation. On an error nothing changes.
    pub fn apply(&mut self, op: &EditOp) -> Result<Outcome, BoardEditError> {
        self.apply_inner(op, false)
    }

    /// Applies several operations as one step: all or nothing, and the first one that is refused
    /// ends the run with its number (from 1).
    pub fn apply_all(&mut self, ops: &[EditOp]) -> Result<Vec<Outcome>, BoardEditError> {
        let backup = self.clone();
        let mut outcomes = Vec::with_capacity(ops.len());
        for (index, op) in ops.iter().enumerate() {
            match self.apply(op) {
                Ok(outcome) => outcomes.push(outcome),
                Err(e) => {
                    *self = backup;
                    return Err(BoardEditError::new(
                        e.code,
                        format!("operation {} was refused: {}", index + 1, e.detail),
                    ));
                }
            }
        }
        Ok(outcomes)
    }

    fn apply_group(&mut self, ops: &[EditOp], force: bool) -> Result<Outcome, BoardEditError> {
        let backup = self.clone();
        let first = self.done.len();
        let before = self.findings.clone();
        let mut outcome = Outcome::default();
        for (index, op) in ops.iter().enumerate() {
            match self.apply_inner(op, force) {
                Ok(done) => {
                    outcome.changed |= done.changed;
                    outcome.created.extend(done.created);
                    outcome.notes.extend(done.notes);
                }
                Err(e) => {
                    *self = backup;
                    return Err(BoardEditError::new(
                        e.code,
                        format!("operation {} of the group: {}", index + 1, e.detail),
                    ));
                }
            }
        }
        let mut steps = self.done.split_off(first);
        if steps.len() > 1 {
            let op = Operation::Batch {
                ops: steps.iter().map(|s| s.op.clone()).collect(),
            };
            let inverse = Operation::Batch {
                ops: steps.iter().rev().map(|s| s.inverse.clone()).collect(),
            };
            steps = vec![Step { op, inverse }];
        }
        self.done.extend(steps);
        outcome.drc = DrcReport::new(&before, &self.findings);
        outcome.forced = force && outcome.drc.adds_error();
        Ok(outcome)
    }

    fn apply_inner(&mut self, op: &EditOp, force: bool) -> Result<Outcome, BoardEditError> {
        match op {
            EditOp::Forced { with } => return self.apply_inner(with, true),
            EditOp::Group { ops } => return self.apply_group(ops, force),
            EditOp::Lock { id } | EditOp::Unlock { id } => {
                if !self.board.components.contains_key(id) {
                    return Err(BoardEditError::new(
                        OPERATION_REFUSED,
                        format!("no component {id}"),
                    ));
                }
                if matches!(op, EditOp::Lock { .. }) {
                    self.locked.insert(*id);
                } else {
                    self.locked.remove(id);
                }
                return Ok(Outcome::default());
            }
            _ => {}
        }
        let route = self.settings.route_settings();
        let planned = plan(
            op,
            &self.board,
            &Env {
                ctx: &self.ctx,
                route: &route,
                settings: &self.settings,
                locked: &self.locked,
                pairs: &self.pairs,
            },
            &mut self.ids,
        )?;
        let mut first_error: Option<BoardEditError> = None;
        for alternative in &planned.alternatives {
            if matches!(alternative, Operation::Batch { ops } if ops.is_empty()) {
                return Ok(Outcome {
                    created: planned.created.clone(),
                    notes: planned.notes.clone(),
                    ..Outcome::default()
                });
            }
            let mut trial = self.board.clone();
            if let Err(e) = trial.apply(alternative) {
                first_error.get_or_insert(BoardEditError::new(OPERATION_REFUSED, e.to_string()));
                continue;
            }
            let after = if self.settings.live_drc {
                findings(&trial, &self.ctx)
            } else {
                Vec::new()
            };
            let drc = DrcReport::new(&self.findings, &after);
            if !force && self.settings.reject_new_errors {
                if let Some(first) = drc.added.iter().find(|f| f.severity == Severity::Error) {
                    first_error.get_or_insert(BoardEditError::new(
                        REJECTED_BY_DRC,
                        format!("the operation adds {} at {}", first.code, first.location),
                    ));
                    continue;
                }
            }
            let mut length_nm = None;
            if let Some(expect) = &planned.expect {
                let reached = net_length(&trial, expect.net);
                match reached {
                    Some(now) if (now - expect.target_nm).abs() <= expect.tolerance_nm => {
                        length_nm = Some((expect.before_nm, now));
                    }
                    other => {
                        first_error.get_or_insert(BoardEditError::new(
                            TUNE_IMPOSSIBLE,
                            format!(
                                "the net would be {} nm long, the target is {} nm",
                                other.unwrap_or(0),
                                expect.target_nm
                            ),
                        ));
                        continue;
                    }
                }
            }
            let inverse = self
                .board
                .apply(alternative)
                .map_err(|e| BoardEditError::new(OPERATION_REFUSED, e.to_string()))?;
            self.done.push(Step {
                op: alternative.clone(),
                inverse,
            });
            self.undone.clear();
            self.revision += 1;
            if self.settings.live_drc {
                self.findings = after;
            }
            let forced = force && drc.adds_error();
            return Ok(Outcome {
                created: planned.created.clone(),
                notes: planned.notes.clone(),
                drc,
                changed: true,
                forced,
                length_nm,
            });
        }
        Err(first_error.unwrap_or_else(|| {
            BoardEditError::new(OPERATION_REFUSED, "no way to do the operation")
        }))
    }
}
