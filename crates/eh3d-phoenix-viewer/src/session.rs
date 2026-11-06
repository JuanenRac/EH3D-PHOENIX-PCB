// SPDX-License-Identifier: GPL-3.0-or-later
//! The documents that the window edits, without a screen: a board with its rules, its history of
//! edits, the findings of the rules and the ratsnest, and a schematic with its electrical rule
//! check and the synchronisation with a board. Every change goes through the reversible operations
//! of the board crates, so every edit can be undone and redone, and a file that was read is never
//! written over: a document is saved under another name, and a copy of the file saved before is
//! kept when this window replaces it.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_board::{Board, History, Id, Operation, Step};
use eh3d_phoenix_eagle_board::write_board;
use eh3d_phoenix_erc::{self as erc, ErcConfig};
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use eh3d_phoenix_route::edit as route_edit;
use eh3d_phoenix_route::{
    copper_layers, diff, findings as rule_findings, ratsnest, route, LiveDiff, RatLine, RoutePlan,
    RouteRequest, RouteSettings, RuleContext,
};
use eh3d_phoenix_rules::rulefile::MAX_RULE_FILE_BYTES;
use eh3d_phoenix_rules::{Finding, Lang, RuleFile, Severity};
use eh3d_phoenix_schematic::Schematic;
use eh3d_phoenix_sync as sync;
use eh3d_phoenix_sync::{BackwardPackage, ForwardPackage, SchematicHistory, SyncSettings};

use crate::codes::{
    ViewError, CHECK_FILE_UNUSABLE, EDIT_FAILED, FILE_NOT_RECOGNISED, SAVE_FAILED, SAVE_REFUSED,
};
use crate::load::{read_document, Document};
use crate::prim::P;
use crate::settings::ViewerSettings;

const SUPPLY_UNIX_MS: u64 = 1_800_000_000_000;
const SUPPLY_RANDOM: u64 = 0x4544_4954_0000_0000;

/// A supply of ids for the objects that an edit creates. The ids are the same on every run, and
/// the board is asked whether an id is free before it is used.
#[derive(Debug, Clone)]
pub struct IdSupply {
    counter: u64,
    last: Id,
}

impl IdSupply {
    /// A new supply.
    pub fn new() -> Result<IdSupply, ViewError> {
        let last = ObjectId::from_parts(SUPPLY_UNIX_MS, 0, SUPPLY_RANDOM)
            .map_err(|e| ViewError::new(EDIT_FAILED, e.to_string()))?;
        Ok(IdSupply { counter: 0, last })
    }

    /// The next id of the supply.
    pub fn next_id(&mut self) -> Id {
        self.counter += 1;
        let n = self.counter;
        let made = ObjectId::from_parts(
            SUPPLY_UNIX_MS,
            (n & 0xfff) as u16,
            SUPPLY_RANDOM.wrapping_add(n),
        );
        if let Ok(id) = made {
            self.last = id;
        }
        self.last
    }

    /// The next id that no object of the board has.
    pub fn unused(&mut self, board: &Board) -> Id {
        let mut id = self.next_id();
        for _ in 0..1024 {
            let taken = board.tracks.contains_key(&id)
                || board.vias.contains_key(&id)
                || board.zones.contains_key(&id)
                || board.components.contains_key(&id)
                || board.nets.contains_key(&id)
                || board.footprints.contains_key(&id)
                || board.layers.contains_key(&id);
            if !taken {
                break;
            }
            id = self.next_id();
        }
        id
    }
}

/// A connection still to be routed, as a line of the drawing in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatSegment {
    /// One end.
    pub a: P,
    /// The other end.
    pub b: P,
    /// The index of the net in the scene.
    pub net: u32,
}

/// A point of the model as a point of the drawing, in millimetres.
pub(crate) fn mm(p: Point) -> P {
    P::new(
        p.x_nm.get() as f64 / 1_000_000.0,
        p.y_nm.get() as f64 / 1_000_000.0,
    )
}

fn segments_of(board: &Board, lines: &[RatLine]) -> Vec<RatSegment> {
    let index: BTreeMap<Id, u32> = board
        .nets
        .keys()
        .enumerate()
        .map(|(i, id)| (*id, i as u32))
        .collect();
    lines
        .iter()
        .filter_map(|line| {
            index.get(&line.net).map(|net| RatSegment {
                a: mm(line.from_at),
                b: mm(line.to_at),
                net: *net,
            })
        })
        .collect()
}

fn edit_failed(error: impl ToString) -> ViewError {
    ViewError::new(EDIT_FAILED, error.to_string())
}

/// What a route did to the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteOutcome {
    /// True when the track reaches the second element; false when it stopped in front of an
    /// obstacle.
    pub complete: bool,
    /// How many tracks the route added.
    pub tracks: usize,
    /// How many vias the route added.
    pub vias: usize,
    /// The findings of the rules that the route added and removed.
    pub diff: LiveDiff,
}

/// Where a file went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// The file written.
    pub path: PathBuf,
    /// The copy of the file that was there before, when this window replaced a file it had saved.
    pub backup: Option<PathBuf>,
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Writes a file under a name that is not the input. The input is never written; a file that is
/// there already is replaced only when `owned` says this window saved it, and then a copy of it is
/// kept next to it with `suffix` added to its name.
pub fn write_checked(
    target: &Path,
    bytes: &[u8],
    input: Option<&Path>,
    owned: Option<&Path>,
    suffix: &str,
) -> Result<Saved, ViewError> {
    if input.is_some_and(|input| same_file(target, input)) {
        return Err(ViewError::new(
            SAVE_REFUSED,
            "the file that was opened is never written over: save under another name",
        ));
    }
    let failed =
        |e: std::io::Error| ViewError::new(SAVE_FAILED, format!("{}: {e}", target.display()));
    let mut backup = None;
    if target.exists() {
        if !owned.is_some_and(|owned| same_file(owned, target)) {
            return Err(ViewError::new(
                SAVE_REFUSED,
                format!(
                    "{} exists and was not saved by this window",
                    target.display()
                ),
            ));
        }
        let mut name = target.as_os_str().to_os_string();
        name.push(suffix);
        let copy = PathBuf::from(name);
        fs::copy(target, &copy).map_err(failed)?;
        backup = Some(copy);
        fs::write(target, bytes).map_err(failed)?;
    } else {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .map_err(failed)?;
        file.write_all(bytes).map_err(failed)?;
    }
    Ok(Saved {
        path: target.to_path_buf(),
        backup,
    })
}

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(wanted))
}

/// The bytes of a board for a file name: EAGLE XML for `.brd`, the JSON of the model otherwise.
fn board_bytes(board: &Board, target: &Path) -> Result<Vec<u8>, ViewError> {
    if has_extension(target, "brd") {
        return Ok(write_board(board).text.into_bytes());
    }
    board
        .to_bytes()
        .map_err(|e| ViewError::new(SAVE_FAILED, e.to_string()))
}

/// A board that is being edited.
pub struct BoardSession {
    /// The board as it is now.
    pub board: Board,
    /// The rules it is edited under.
    pub ctx: RuleContext,
    /// The settings of the router and of the editing.
    pub settings: RouteSettings,
    /// The findings of the rules on the board as it is now.
    pub findings: Vec<Finding>,
    /// What the last edit (or undo, or redo) added to the findings and removed from them.
    pub last_diff: LiveDiff,
    /// The connections still to be routed.
    pub rat: Vec<RatLine>,
    /// The same connections as lines of the drawing.
    pub rat_segments: Vec<RatSegment>,
    /// Why the ratsnest is empty, when it could not be made.
    pub rat_error: Option<String>,
    /// The file that this session saved last.
    pub last_saved: Option<PathBuf>,
    history: History,
    redo_count: usize,
    serial: u64,
    saved_serial: u64,
    ids: IdSupply,
}

impl BoardSession {
    /// A session on a board under rules.
    pub fn new(
        board: Board,
        ctx: RuleContext,
        settings: RouteSettings,
    ) -> Result<BoardSession, ViewError> {
        let mut session = BoardSession {
            board,
            ctx,
            settings,
            findings: Vec::new(),
            last_diff: LiveDiff::default(),
            rat: Vec::new(),
            rat_segments: Vec::new(),
            rat_error: None,
            last_saved: None,
            history: History::new(),
            redo_count: 0,
            serial: 0,
            saved_serial: 0,
            ids: IdSupply::new()?,
        };
        session.findings = rule_findings(&session.board, &session.ctx);
        session.refresh_ratsnest();
        Ok(session)
    }

    fn refresh_ratsnest(&mut self) {
        match ratsnest(&self.board, &self.settings) {
            Ok(lines) => {
                self.rat = lines;
                self.rat_error = None;
            }
            Err(error) => {
                self.rat.clear();
                self.rat_error = Some(error.to_string());
            }
        }
        self.rat_segments = segments_of(&self.board, &self.rat);
    }

    fn after_history_step(&mut self) {
        let after = rule_findings(&self.board, &self.ctx);
        self.last_diff = diff(&self.findings, &after);
        self.findings = after;
        self.serial += 1;
        self.refresh_ratsnest();
    }

    /// Applies an operation, all or nothing, and returns the findings it added and removed. The
    /// operation is tried on a copy first: when the board refuses it, or when it adds a rule error
    /// and the settings ask for strict edits, the board stays as it was and nothing is recorded.
    pub fn apply(&mut self, op: Operation) -> Result<LiveDiff, ViewError> {
        let mut trial = self.board.clone();
        trial.apply(&op).map_err(edit_failed)?;
        let after = rule_findings(&trial, &self.ctx);
        let changes = diff(&self.findings, &after);
        if self.settings.strict_edits {
            if let Some(first) = changes.added.iter().find(|f| f.severity == Severity::Error) {
                return Err(ViewError::new(
                    EDIT_FAILED,
                    format!("the edit adds {} at {}", first.code, first.location),
                ));
            }
        }
        self.history
            .execute(&mut self.board, op)
            .map_err(edit_failed)?;
        self.redo_count = 0;
        self.serial += 1;
        self.findings = after;
        self.last_diff = changes.clone();
        self.refresh_ratsnest();
        Ok(changes)
    }

    /// Undoes the last edit; false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        if !self.history.undo(&mut self.board) {
            return false;
        }
        self.redo_count += 1;
        self.after_history_step();
        true
    }

    /// Redoes the last undone edit; false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        if !self.history.redo(&mut self.board) {
            return false;
        }
        self.redo_count = self.redo_count.saturating_sub(1);
        self.after_history_step();
        true
    }

    /// True when there is an edit to undo.
    pub fn can_undo(&self) -> bool {
        !self.history.done().is_empty()
    }

    /// True when there is an undone edit to redo.
    pub fn can_redo(&self) -> bool {
        self.redo_count > 0
    }

    /// Puts an edited board in the place of the one that is open (the board editor keeps its own
    /// steps for undo and redo): the rules are checked again and what changed is kept for the
    /// panel, the ratsnest is made again, the steps of this session are dropped and the document
    /// counts as changed.
    pub fn replace_board(&mut self, board: Board) {
        self.board = board;
        self.history = History::new();
        self.redo_count = 0;
        self.serial += 1;
        let before = std::mem::take(&mut self.findings);
        self.findings = rule_findings(&self.board, &self.ctx);
        self.last_diff = diff(&before, &self.findings);
        self.refresh_ratsnest();
    }

    /// How many edits are done.
    pub fn edits(&self) -> usize {
        self.history.done().len()
    }

    /// The edits that are done, oldest first.
    pub fn steps_done(&self) -> &[Step] {
        self.history.done()
    }

    /// The edits that were undone and can be redone, the next one to redo last.
    pub fn steps_undone(&self) -> &[Step] {
        self.history.undone()
    }

    /// True when the board has changed since it was opened or saved.
    pub fn dirty(&self) -> bool {
        self.serial != self.saved_serial
    }

    /// Moves a component to a place, keeping its rotation.
    pub fn move_component(&mut self, id: Id, at: Point) -> Result<LiveDiff, ViewError> {
        let op = route_edit::move_component(&self.board, id, at).map_err(edit_failed)?;
        self.apply(op)
    }

    /// Turns a component by an angle in micro-degrees.
    pub fn rotate_component(&mut self, id: Id, delta_udeg: i64) -> Result<LiveDiff, ViewError> {
        let op = route_edit::rotate_component(&self.board, id, delta_udeg).map_err(edit_failed)?;
        self.apply(op)
    }

    /// Deletes a track.
    pub fn delete_track(&mut self, id: Id) -> Result<LiveDiff, ViewError> {
        let op = route_edit::delete_track(&self.board, id).map_err(edit_failed)?;
        self.apply(op)
    }

    /// Moves a vertex of a track.
    pub fn move_vertex(
        &mut self,
        track: Id,
        index: usize,
        to: Point,
    ) -> Result<LiveDiff, ViewError> {
        let op = route_edit::move_vertex(&self.board, track, index, to, self.settings.keep_angles)
            .map_err(edit_failed)?;
        self.apply(op)
    }

    /// Gives a track a width in nanometres.
    pub fn set_track_width(&mut self, id: Id, width_nm: i64) -> Result<LiveDiff, ViewError> {
        let width = Nm::new(width_nm).map_err(edit_failed)?;
        let op = route_edit::set_track_width(&self.board, id, width).map_err(edit_failed)?;
        self.apply(op)
    }

    /// Adds a via of a net at a place, between the first and the last copper layer, sized by the
    /// rules and the net class.
    pub fn add_via(&mut self, net: Id, at: Point) -> Result<LiveDiff, ViewError> {
        let copper = copper_layers(&self.board);
        let (Some(first), Some(last)) = (copper.first().copied(), copper.last().copied()) else {
            return Err(edit_failed("the board has no copper layer"));
        };
        if first == last {
            return Err(edit_failed("a via needs two copper layers"));
        }
        let via = route_edit::via_for_layer_change(
            &self.board,
            &self.ctx,
            &self.settings,
            net,
            at,
            (first, last),
        )
        .map_err(edit_failed)?;
        let id = self.ids.unused(&self.board);
        self.apply(route_edit::add_via(id, via))
    }

    /// Routes between two elements of a net and applies the plan as one edit.
    pub fn route(&mut self, request: &RouteRequest) -> Result<RouteOutcome, ViewError> {
        let plan = {
            let ids = &mut self.ids;
            let board = &self.board;
            let mut new_id = || ids.unused(board);
            route(&self.board, &self.ctx, &self.settings, request, &mut new_id)
                .map_err(edit_failed)?
        };
        let RoutePlan {
            operation,
            complete,
            tracks,
            vias,
            ..
        } = plan;
        let changes = self.apply(operation)?;
        Ok(RouteOutcome {
            complete,
            tracks: tracks.len(),
            vias: vias.len(),
            diff: changes,
        })
    }

    /// Changes the rules the board is judged by; the findings are made again and the edits that
    /// were done stay.
    pub fn set_context(&mut self, ctx: RuleContext) {
        self.ctx = ctx;
        self.findings = rule_findings(&self.board, &self.ctx);
        self.last_diff = LiveDiff::default();
    }

    /// Reads a rule file and judges the board by it.
    pub fn load_rules(&mut self, path: &Path) -> Result<(), ViewError> {
        let unusable = |e: String| ViewError::new(CHECK_FILE_UNUSABLE, e);
        let size = fs::metadata(path)
            .map_err(|e| unusable(format!("{}: {e}", path.display())))?
            .len();
        if size > MAX_RULE_FILE_BYTES as u64 {
            return Err(unusable(format!(
                "the file is {size} bytes, the limit is {MAX_RULE_FILE_BYTES}"
            )));
        }
        let text =
            fs::read_to_string(path).map_err(|e| unusable(format!("{}: {e}", path.display())))?;
        let file = RuleFile::from_json(&text).map_err(|e| unusable(e.to_string()))?;
        self.set_context(RuleContext {
            rules: file.values,
            classes: file.classes,
        });
        Ok(())
    }

    /// Saves the board under another name than the input; see [`write_checked`].
    pub fn save_as(
        &mut self,
        target: &Path,
        input: Option<&Path>,
        suffix: &str,
    ) -> Result<Saved, ViewError> {
        let bytes = board_bytes(&self.board, target)?;
        let saved = write_checked(target, &bytes, input, self.last_saved.as_deref(), suffix)?;
        self.last_saved = Some(target.to_path_buf());
        self.saved_serial = self.serial;
        Ok(saved)
    }
}

/// The board that a schematic is compared with, and the edits made to it.
pub struct Companion {
    /// Where it was read from.
    pub path: PathBuf,
    /// The board as it is now.
    pub board: Board,
    /// The file that this session saved last.
    pub last_saved: Option<PathBuf>,
    history: History,
    serial: u64,
    saved_serial: u64,
}

impl Companion {
    /// True when the board has changed since it was read or saved.
    pub fn dirty(&self) -> bool {
        self.serial != self.saved_serial
    }
}

/// A package of changes that a dry run made and that has not been applied.
pub enum Preview {
    /// The transfer of the schematic to the board.
    Forward(ForwardPackage),
    /// The annotation of the board back into the schematic.
    Backward(BackwardPackage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Schematic,
    Board,
}

/// A schematic that is being looked at and annotated.
pub struct SchematicSession {
    /// The schematic as it is now.
    pub schematic: Schematic,
    /// The settings of the electrical rule check.
    pub erc_config: ErcConfig,
    /// The result of the electrical rule check.
    pub erc: erc::Report,
    /// The settings of the synchronisation.
    pub sync_settings: SyncSettings,
    /// The board that the schematic is compared with.
    pub companion: Option<Companion>,
    /// The package that a dry run made.
    pub preview: Option<Preview>,
    /// The file that this session saved last.
    pub last_saved: Option<PathBuf>,
    history: SchematicHistory,
    done: Vec<Target>,
    undone: Vec<Target>,
    serial: u64,
    saved_serial: u64,
    ids: IdSupply,
}

impl SchematicSession {
    /// A session on a schematic. The electrical rule check is run at once.
    pub fn new(
        schematic: Schematic,
        erc_config: ErcConfig,
        sync_settings: SyncSettings,
    ) -> Result<SchematicSession, ViewError> {
        let report = erc::check(&schematic, &erc_config);
        Ok(SchematicSession {
            schematic,
            erc_config,
            erc: report,
            sync_settings,
            companion: None,
            preview: None,
            last_saved: None,
            history: SchematicHistory::new(),
            done: Vec::new(),
            undone: Vec::new(),
            serial: 0,
            saved_serial: 0,
            ids: IdSupply::new()?,
        })
    }

    /// Puts a schematic that was edited elsewhere (the schematic editor of the window) in the place
    /// of the one of the session: a package that was ready is thrown away, the annotation steps
    /// that were done are forgotten (they were made for the old schematic), the rule check is run
    /// again and the session counts as changed.
    pub fn replace_schematic(&mut self, schematic: Schematic) {
        self.schematic = schematic;
        self.preview = None;
        self.history = SchematicHistory::new();
        self.done.retain(|t| *t != Target::Schematic);
        self.undone.retain(|t| *t != Target::Schematic);
        self.serial += 1;
        self.run_erc();
    }

    /// Runs the electrical rule check again.
    pub fn run_erc(&mut self) {
        self.erc = erc::check(&self.schematic, &self.erc_config);
    }

    /// Takes a board as the one to compare with.
    pub fn set_companion(&mut self, path: PathBuf, board: Board) {
        self.companion = Some(Companion {
            path,
            board,
            last_saved: None,
            history: History::new(),
            serial: 0,
            saved_serial: 0,
        });
        self.preview = None;
        self.done.retain(|t| *t != Target::Board);
        self.undone.retain(|t| *t != Target::Board);
    }

    /// Reads a board file as the one to compare with.
    pub fn open_companion(
        &mut self,
        path: &Path,
        settings: &ViewerSettings,
    ) -> Result<(), ViewError> {
        let loaded = read_document(path, settings)?;
        match loaded.document {
            Document::Board(board) => {
                self.set_companion(path.to_path_buf(), *board);
                Ok(())
            }
            Document::Schematic(_) => Err(ViewError::new(
                FILE_NOT_RECOGNISED,
                "the file is a schematic, a board is needed",
            )),
        }
    }

    /// The dry run of the transfer of the schematic to the board: nothing is changed, and the
    /// package is kept as the preview.
    pub fn plan_forward(&mut self) -> Result<(), ViewError> {
        let Some(companion) = self.companion.as_ref() else {
            return Err(edit_failed("there is no board to compare with"));
        };
        let ids = &mut self.ids;
        let mut new_id = || ids.unused(&companion.board);
        let package = sync::plan_forward(
            &self.schematic,
            &companion.board,
            &self.sync_settings,
            &mut new_id,
        );
        self.preview = Some(Preview::Forward(package));
        Ok(())
    }

    /// The dry run of the annotation of the board back into the schematic.
    pub fn plan_backward(&mut self) -> Result<(), ViewError> {
        let Some(companion) = self.companion.as_ref() else {
            return Err(edit_failed("there is no board to compare with"));
        };
        let package = sync::plan_backward(&self.schematic, &companion.board, &self.sync_settings);
        self.preview = Some(Preview::Backward(package));
        Ok(())
    }

    /// True when a package is ready and it holds changes.
    pub fn preview_has_changes(&self) -> bool {
        match &self.preview {
            Some(Preview::Forward(p)) => !p.is_empty(),
            Some(Preview::Backward(p)) => !p.is_empty(),
            None => false,
        }
    }

    /// The report of the package that is ready, as text in a language.
    pub fn preview_text(&self, lang: Lang) -> Option<String> {
        match &self.preview {
            Some(Preview::Forward(p)) => Some(p.report().to_text(lang)),
            Some(Preview::Backward(p)) => Some(p.report().to_text(lang)),
            None => None,
        }
    }

    /// Throws the package that is ready away.
    pub fn discard_preview(&mut self) {
        self.preview = None;
    }

    /// Applies the package that is ready, as one step that can be undone.
    pub fn apply_preview(&mut self) -> Result<(), ViewError> {
        let Some(preview) = self.preview.take() else {
            return Err(edit_failed("there is no package to apply"));
        };
        match preview {
            Preview::Forward(package) => {
                let Some(companion) = self.companion.as_mut() else {
                    return Err(edit_failed("there is no board to compare with"));
                };
                package
                    .apply(&mut companion.board, &mut companion.history)
                    .map_err(edit_failed)?;
                if !package.is_empty() {
                    companion.serial += 1;
                    self.done.push(Target::Board);
                }
            }
            Preview::Backward(package) => {
                package
                    .apply(&mut self.schematic, &mut self.history)
                    .map_err(edit_failed)?;
                if !package.is_empty() {
                    self.serial += 1;
                    self.done.push(Target::Schematic);
                }
            }
        }
        self.undone.clear();
        self.run_erc();
        Ok(())
    }

    /// Undoes the last package that was applied, to the schematic or to the board; false when
    /// there is none.
    pub fn undo(&mut self) -> bool {
        let Some(target) = self.done.pop() else {
            return false;
        };
        let undone = match target {
            Target::Schematic => self.history.undo(&mut self.schematic),
            Target::Board => self
                .companion
                .as_mut()
                .is_some_and(|c| c.history.undo(&mut c.board)),
        };
        if !undone {
            self.done.push(target);
            return false;
        }
        self.after_step(target);
        self.undone.push(target);
        true
    }

    /// Redoes the last package that was undone; false when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(target) = self.undone.pop() else {
            return false;
        };
        let redone = match target {
            Target::Schematic => self.history.redo(&mut self.schematic),
            Target::Board => self
                .companion
                .as_mut()
                .is_some_and(|c| c.history.redo(&mut c.board)),
        };
        if !redone {
            self.undone.push(target);
            return false;
        }
        self.after_step(target);
        self.done.push(target);
        true
    }

    fn after_step(&mut self, target: Target) {
        match target {
            Target::Schematic => self.serial += 1,
            Target::Board => {
                if let Some(c) = self.companion.as_mut() {
                    c.serial += 1;
                }
            }
        }
        self.preview = None;
        self.run_erc();
    }

    /// True when there is a package to undo.
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    /// True when there is a package to redo.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// The packages that are done, oldest first: true for one applied to the board and false for
    /// one applied to the schematic.
    pub fn steps_done(&self) -> Vec<bool> {
        self.done.iter().map(|t| *t == Target::Board).collect()
    }

    /// The packages that were undone, the next one to redo last, in the same way.
    pub fn steps_undone(&self) -> Vec<bool> {
        self.undone.iter().map(|t| *t == Target::Board).collect()
    }

    /// True when the schematic or the board has changed since it was opened or saved.
    pub fn dirty(&self) -> bool {
        self.serial != self.saved_serial
            || self
                .companion
                .as_ref()
                .is_some_and(|c| c.serial != c.saved_serial)
    }

    /// Saves the schematic as the JSON of the model under another name than the input.
    pub fn save_schematic_as(
        &mut self,
        target: &Path,
        input: Option<&Path>,
        suffix: &str,
    ) -> Result<Saved, ViewError> {
        if has_extension(target, "sch") {
            return Err(ViewError::new(
                SAVE_FAILED,
                "a schematic is saved as the JSON of the model, not as an EAGLE file",
            ));
        }
        let bytes = self
            .schematic
            .to_bytes()
            .map_err(|e| ViewError::new(SAVE_FAILED, e.to_string()))?;
        let saved = write_checked(target, &bytes, input, self.last_saved.as_deref(), suffix)?;
        self.last_saved = Some(target.to_path_buf());
        self.saved_serial = self.serial;
        Ok(saved)
    }

    /// Saves the board that the schematic is compared with under another name than the file it
    /// was read from.
    pub fn save_board_as(&mut self, target: &Path, suffix: &str) -> Result<Saved, ViewError> {
        let Some(companion) = self.companion.as_mut() else {
            return Err(edit_failed("there is no board to compare with"));
        };
        let bytes = board_bytes(&companion.board, target)?;
        let saved = write_checked(
            target,
            &bytes,
            Some(companion.path.as_path()),
            companion.last_saved.as_deref(),
            suffix,
        )?;
        companion.last_saved = Some(target.to_path_buf());
        companion.saved_serial = companion.serial;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use eh3d_phoenix_rules::RuleValues;

    fn point(x: f64, y: f64) -> Point {
        sample::point(x, y)
    }

    fn relaxed() -> RouteSettings {
        RouteSettings {
            strict_edits: false,
            ..RouteSettings::default()
        }
    }

    fn session() -> BoardSession {
        BoardSession::new(sample::board(), RuleContext::default(), relaxed()).unwrap()
    }

    #[test]
    fn the_supply_gives_ids_that_differ_and_that_the_board_does_not_use() {
        let mut supply = IdSupply::new().unwrap();
        let first = supply.next_id();
        let second = supply.next_id();
        assert_ne!(first, second);
        let mut again = IdSupply::new().unwrap();
        assert_eq!(again.next_id(), first);
        // an id that the board uses is skipped
        let mut board = sample::board();
        let mut probe = IdSupply::new().unwrap();
        let wanted = probe.next_id();
        board.nets.insert(
            wanted,
            eh3d_phoenix_board::Net {
                name: "X".to_string(),
            },
        );
        let mut fresh = IdSupply::new().unwrap();
        let got = fresh.unused(&board);
        assert_ne!(got, wanted);
        assert!(!board.nets.contains_key(&got));
    }

    #[test]
    fn a_new_session_has_findings_and_a_ratsnest_that_match_the_crates() {
        let s = session();
        assert_eq!(s.findings, rule_findings(&s.board, &s.ctx));
        let lines = ratsnest(&s.board, &s.settings).unwrap();
        assert_eq!(s.rat, lines);
        assert_eq!(s.rat_segments.len(), s.rat.len());
        assert!(s.rat_error.is_none());
        assert!(!s.dirty() && !s.can_undo() && !s.can_redo());
        assert_eq!(s.edits(), 0);
        // the nets of the sample are GND (0) and VCC (1) and every segment names one of them
        assert!(s.rat_segments.iter().all(|seg| seg.net < 2));
    }

    #[test]
    fn moving_a_component_changes_the_board_and_undo_and_redo_bring_it_back() {
        let mut s = session();
        let original = s.board.clone();
        let id = sample::id(30);
        s.move_component(id, point(12.0, 10.0)).unwrap();
        assert_eq!(s.board.components[&id].at, point(12.0, 10.0));
        assert!(s.dirty() && s.can_undo() && !s.can_redo());
        assert_eq!(s.edits(), 1);
        let moved = s.board.clone();
        assert!(s.undo());
        assert_eq!(s.board, original);
        assert!(s.can_redo() && !s.can_undo());
        assert!(s.redo());
        assert_eq!(s.board, moved);
        assert!(!s.can_redo());
        assert!(!s.redo());
        // an edit after an undo clears what could be redone
        assert!(s.undo());
        s.rotate_component(id, 90_000_000).unwrap();
        assert!(!s.can_redo());
        assert_eq!(s.board.components[&id].rotation.get(), 90_000_000);
    }

    #[test]
    fn the_ratsnest_is_made_again_after_each_edit() {
        let mut s = session();
        let before = s.rat_segments.clone();
        s.move_component(sample::id(30), point(12.0, 10.0)).unwrap();
        assert_eq!(s.rat, ratsnest(&s.board, &s.settings).unwrap());
        assert_eq!(s.rat_segments.len(), s.rat.len());
        s.undo();
        assert_eq!(s.rat_segments, before);
    }

    #[test]
    fn deleting_a_track_and_changing_a_width_are_edits_that_can_be_undone() {
        let mut s = session();
        let track = sample::id(40);
        s.set_track_width(track, 400_000).unwrap();
        assert_eq!(s.board.tracks[&track].width.get(), 400_000);
        s.delete_track(track).unwrap();
        assert!(!s.board.tracks.contains_key(&track));
        assert!(s.undo());
        assert!(s.board.tracks.contains_key(&track));
        assert_eq!(s.board.tracks[&track].width.get(), 400_000);
        assert!(s.undo());
        assert_eq!(s.board.tracks[&track].width.get(), 250_000);
        // a track that does not exist is an error with the code of the edits
        let error = s.delete_track(sample::id(99)).unwrap_err();
        assert_eq!(error.code, EDIT_FAILED);
        assert_eq!(s.edits(), 0);
    }

    #[test]
    fn a_vertex_moves_and_a_piece_that_would_shrink_or_leave_the_angles_is_refused() {
        let track = sample::id(40);
        // the track is the line from (5, 5) to (9, 5): the vertex 1 is its end
        let mut s = session();
        // the piece would run at another angle than 0, 45 and 90 degrees
        assert_eq!(
            s.move_vertex(track, 1, point(9.0, 8.0)).unwrap_err().code,
            EDIT_FAILED
        );
        // the piece would shrink to a point
        assert_eq!(
            s.move_vertex(track, 1, point(5.0, 5.0)).unwrap_err().code,
            EDIT_FAILED
        );
        // a vertex that the track does not have
        assert_eq!(
            s.move_vertex(track, 7, point(9.0, 9.0)).unwrap_err().code,
            EDIT_FAILED
        );
        assert_eq!(s.edits(), 0);
        // along a diagonal the angle is kept
        s.move_vertex(track, 1, point(9.0, 9.0)).unwrap();
        assert_eq!(s.board.tracks[&track].path[0].end(), point(9.0, 9.0));
        assert!(s.undo());
        // without the check of the angles any place is allowed
        let loose = RouteSettings {
            keep_angles: false,
            ..relaxed()
        };
        let mut s = BoardSession::new(sample::board(), RuleContext::default(), loose).unwrap();
        s.move_vertex(track, 1, point(9.0, 8.0)).unwrap();
        let piece = s.board.tracks[&track].path[0];
        assert_eq!(piece.end(), point(9.0, 8.0));
        assert_eq!(piece.start(), point(5.0, 5.0));
        assert!(s.undo());
        assert_eq!(s.board.tracks[&track].path[0].end(), point(9.0, 5.0));
    }

    #[test]
    fn a_via_is_added_between_the_outer_layers_for_the_net() {
        let mut s = session();
        let gnd = sample::id(10);
        let vias = s.board.vias.len();
        s.add_via(gnd, point(20.0, 5.0)).unwrap();
        assert_eq!(s.board.vias.len(), vias + 1);
        let added = s
            .board
            .vias
            .values()
            .find(|v| v.at == point(20.0, 5.0))
            .unwrap();
        assert_eq!(added.net, gnd);
        assert_eq!(added.from_layer, sample::id(1));
        assert_eq!(added.to_layer, sample::id(2));
        assert!(s.undo());
        assert_eq!(s.board.vias.len(), vias);
        // a net that does not exist is an error
        assert_eq!(
            s.add_via(sample::id(98), point(1.0, 1.0)).unwrap_err().code,
            EDIT_FAILED
        );
    }

    #[test]
    fn a_strict_session_refuses_an_edit_that_adds_a_rule_error_and_a_relaxed_one_reports_it() {
        let ctx = RuleContext {
            rules: RuleValues {
                min_track_width: Nm::new(200_000).unwrap(),
                ..RuleValues::default()
            },
            classes: Default::default(),
        };
        let track = sample::id(40);
        let mut strict =
            BoardSession::new(sample::board(), ctx.clone(), RouteSettings::default()).unwrap();
        let error = strict.set_track_width(track, 100_000).unwrap_err();
        assert_eq!(error.code, EDIT_FAILED);
        assert_eq!(strict.board.tracks[&track].width.get(), 250_000);
        assert_eq!(strict.edits(), 0);
        assert!(!strict.dirty());
        let mut loose = BoardSession::new(sample::board(), ctx, relaxed()).unwrap();
        let changes = loose.set_track_width(track, 100_000).unwrap();
        assert!(changes.adds_error());
        assert_eq!(loose.last_diff, changes);
        // widening it again resolves what was added
        loose.set_track_width(track, 250_000).unwrap();
        assert!(!loose.last_diff.removed.is_empty());
        assert!(!loose.last_diff.adds_error());
    }

    #[test]
    fn the_diff_after_an_undo_is_the_reverse_of_the_edit() {
        let ctx = RuleContext {
            rules: RuleValues {
                min_track_width: Nm::new(200_000).unwrap(),
                ..RuleValues::default()
            },
            classes: Default::default(),
        };
        let mut s = BoardSession::new(sample::board(), ctx, relaxed()).unwrap();
        let edit = s.set_track_width(sample::id(40), 100_000).unwrap();
        assert!(s.undo());
        assert_eq!(s.last_diff.removed, edit.added);
        assert!(s.redo());
        assert_eq!(s.last_diff.added, edit.added);
    }

    #[test]
    fn routing_joins_two_elements_of_a_net_as_one_edit() {
        use eh3d_phoenix_board::PadRef;
        use eh3d_phoenix_route::{Element, Mode};
        let mut s = session();
        let before = s.rat.len();
        let request = RouteRequest {
            from: Element::Pad(PadRef {
                component: sample::id(30),
                pad: sample::id(21),
            }),
            to: Element::Track {
                id: sample::id(40),
                end: true,
            },
            layer: None,
            mode: Mode::Around,
        };
        let outcome = s.route(&request).unwrap();
        assert!(outcome.complete);
        assert!(outcome.tracks >= 1);
        assert_eq!(s.edits(), 1);
        assert!(s.rat.len() < before, "{} < {before}", s.rat.len());
        assert!(s.undo());
        assert_eq!(s.rat.len(), before);
        // two elements of different nets cannot be joined
        let pad_vcc = RouteRequest {
            from: Element::Pad(PadRef {
                component: sample::id(30),
                pad: sample::id(22),
            }),
            to: Element::Via(sample::id(42)),
            layer: None,
            mode: Mode::Stop,
        };
        assert_eq!(s.route(&pad_vcc).unwrap_err().code, EDIT_FAILED);
    }

    #[test]
    fn a_rule_file_replaces_the_rules_and_keeps_the_edits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.json");
        let rules = RuleValues {
            min_track_width: Nm::new(300_000).unwrap(),
            ..RuleValues::default()
        };
        std::fs::write(&path, rules.to_json("mine")).unwrap();
        let mut s = session();
        s.move_component(sample::id(30), point(12.0, 10.0)).unwrap();
        let before = s.findings.len();
        s.load_rules(&path).unwrap();
        assert_eq!(s.ctx.rules.min_track_width.get(), 300_000);
        assert_eq!(s.edits(), 1);
        assert_eq!(s.findings, rule_findings(&s.board, &s.ctx));
        assert!(s.findings.len() >= before);
        assert!(s.last_diff.added.is_empty() && s.last_diff.removed.is_empty());
        let missing = dir.path().join("nothing.json");
        assert_eq!(
            s.load_rules(&missing).unwrap_err().code,
            CHECK_FILE_UNUSABLE
        );
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(s.load_rules(&path).unwrap_err().code, CHECK_FILE_UNUSABLE);
        assert_eq!(s.ctx.rules.min_track_width.get(), 300_000);
    }

    #[test]
    fn saving_never_writes_over_the_input_and_keeps_a_copy_of_what_it_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("board.json");
        let original = sample::board().to_bytes().unwrap();
        std::fs::write(&input, &original).unwrap();
        let mut s = session();
        s.move_component(sample::id(30), point(12.0, 10.0)).unwrap();
        assert!(s.dirty());
        // the input is refused, with a path spelt another way too
        let error = s.save_as(&input, Some(&input), ".bak").unwrap_err();
        assert_eq!(error.code, SAVE_REFUSED);
        let roundabout = dir.path().join(".").join("board.json");
        assert_eq!(
            s.save_as(&roundabout, Some(&input), ".bak")
                .unwrap_err()
                .code,
            SAVE_REFUSED
        );
        assert_eq!(std::fs::read(&input).unwrap(), original);
        assert!(s.dirty());
        // a new name works and the board saved is the edited one
        let target = dir.path().join("edited.json");
        let saved = s.save_as(&target, Some(&input), ".bak").unwrap();
        assert_eq!(saved.backup, None);
        assert!(!s.dirty());
        let first = std::fs::read(&target).unwrap();
        assert_eq!(first, s.board.to_bytes().unwrap());
        assert_eq!(std::fs::read(&input).unwrap(), original);
        // a second save replaces it and keeps the first as the copy
        s.move_component(sample::id(30), point(13.0, 10.0)).unwrap();
        let again = s.save_as(&target, Some(&input), ".bak").unwrap();
        let copy = again.backup.unwrap();
        assert_eq!(copy, dir.path().join("edited.json.bak"));
        assert_eq!(std::fs::read(&copy).unwrap(), first);
        assert_ne!(std::fs::read(&target).unwrap(), first);
        // a file that exists and was not saved by this session is not replaced
        let foreign = dir.path().join("other.json");
        std::fs::write(&foreign, b"mine").unwrap();
        assert_eq!(
            s.save_as(&foreign, Some(&input), ".bak").unwrap_err().code,
            SAVE_REFUSED
        );
        assert_eq!(std::fs::read(&foreign).unwrap(), b"mine");
        // a folder that does not exist is a failure to write
        let nowhere = dir.path().join("none").join("x.json");
        assert_eq!(
            s.save_as(&nowhere, Some(&input), ".bak").unwrap_err().code,
            SAVE_FAILED
        );
    }

    #[test]
    fn a_board_saved_with_the_brd_extension_is_eagle_text_that_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("edited.brd");
        let mut s = session();
        s.save_as(&target, None, ".bak").unwrap();
        let text = std::fs::read_to_string(&target).unwrap();
        assert!(text.trim_start().starts_with('<'));
        let loaded = crate::load::parse_document("brd", &text).unwrap();
        assert!(matches!(loaded.document, Document::Board(_)));
    }

    fn linked_board() -> Board {
        // the component R1 is made from the part R1 and holds another value
        let mut board = sample::board();
        let component = board.components.get_mut(&sample::id(30)).unwrap();
        component.symbol = Some(sample::sch_id(10));
        component.value = "22k".to_string();
        board
    }

    fn schematic_session() -> SchematicSession {
        SchematicSession::new(
            sample::schematic(),
            ErcConfig::default(),
            SyncSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn the_rule_check_of_a_schematic_is_the_one_of_the_crate_and_can_be_run_again() {
        let mut s = schematic_session();
        assert_eq!(s.erc, erc::check(&s.schematic, &ErcConfig::default()));
        s.erc = erc::Report::default();
        s.run_erc();
        assert_eq!(s.erc, erc::check(&s.schematic, &ErcConfig::default()));
        assert!(!s.dirty() && !s.can_undo() && !s.can_redo());
    }

    #[test]
    fn a_dry_run_needs_a_board_and_changes_nothing() {
        let mut s = schematic_session();
        assert_eq!(s.plan_forward().unwrap_err().code, EDIT_FAILED);
        assert_eq!(s.plan_backward().unwrap_err().code, EDIT_FAILED);
        assert!(s.apply_preview().is_err());
        s.set_companion(PathBuf::from("board.json"), linked_board());
        let board_before = s.companion.as_ref().unwrap().board.clone();
        let schematic_before = s.schematic.clone();
        s.plan_forward().unwrap();
        assert!(s.preview_has_changes());
        let text = s.preview_text(Lang::En).unwrap();
        assert!(text.contains("PHOENIX-SYNC-"), "{text}");
        assert_eq!(s.companion.as_ref().unwrap().board, board_before);
        assert_eq!(s.schematic, schematic_before);
        assert!(!s.dirty());
        s.discard_preview();
        assert!(s.preview_text(Lang::En).is_none());
        assert!(!s.preview_has_changes());
    }

    #[test]
    fn the_transfer_to_the_board_is_applied_as_one_step_and_undone_and_redone() {
        let mut s = schematic_session();
        s.set_companion(PathBuf::from("board.json"), linked_board());
        let component = sample::id(30);
        s.plan_forward().unwrap();
        s.apply_preview().unwrap();
        assert!(s.preview.is_none());
        assert_eq!(
            s.companion.as_ref().unwrap().board.components[&component].value,
            "10k"
        );
        assert!(s.dirty() && s.can_undo() && !s.can_redo());
        assert!(s.undo());
        assert_eq!(
            s.companion.as_ref().unwrap().board.components[&component].value,
            "22k"
        );
        assert!(s.can_redo());
        assert!(s.redo());
        assert_eq!(
            s.companion.as_ref().unwrap().board.components[&component].value,
            "10k"
        );
        // once the board agrees a second dry run finds nothing to change
        s.plan_forward().unwrap();
        assert!(!s.preview_has_changes());
        assert!(s.preview_text(Lang::En).is_some());
    }

    #[test]
    fn the_annotation_back_writes_the_board_into_the_schematic_and_can_be_undone() {
        let mut s = schematic_session();
        s.set_companion(PathBuf::from("board.json"), linked_board());
        let part = sample::sch_id(10);
        s.plan_backward().unwrap();
        assert!(s.preview_has_changes());
        s.apply_preview().unwrap();
        assert_eq!(s.schematic.parts[&part].value, "22k");
        assert!(s.dirty() && s.can_undo());
        assert!(s.undo());
        assert_eq!(s.schematic.parts[&part].value, "10k");
        assert!(s.redo());
        assert_eq!(s.schematic.parts[&part].value, "22k");
        // the rule check was run again after each step
        assert_eq!(s.erc, erc::check(&s.schematic, &ErcConfig::default()));
    }

    #[test]
    fn a_board_file_is_opened_as_the_companion_and_a_schematic_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let board_path = dir.path().join("board.json");
        std::fs::write(&board_path, sample::board().to_bytes().unwrap()).unwrap();
        let schematic_path = dir.path().join("schematic.json");
        std::fs::write(&schematic_path, sample::schematic().to_bytes().unwrap()).unwrap();
        let settings = ViewerSettings::default();
        let mut s = schematic_session();
        s.open_companion(&board_path, &settings).unwrap();
        assert_eq!(s.companion.as_ref().unwrap().path, board_path);
        assert_eq!(
            s.open_companion(&schematic_path, &settings)
                .unwrap_err()
                .code,
            FILE_NOT_RECOGNISED
        );
        assert_eq!(
            s.open_companion(&dir.path().join("none.json"), &settings)
                .unwrap_err()
                .code,
            crate::codes::FILE_UNUSABLE
        );
        // the board is the one that was read
        assert_eq!(s.companion.as_ref().unwrap().board, sample::board());
    }

    #[test]
    fn the_schematic_and_the_companion_board_are_saved_under_other_names() {
        let dir = tempfile::tempdir().unwrap();
        let board_path = dir.path().join("board.json");
        std::fs::write(&board_path, linked_board().to_bytes().unwrap()).unwrap();
        let schematic_path = dir.path().join("schematic.json");
        std::fs::write(&schematic_path, sample::schematic().to_bytes().unwrap()).unwrap();
        let mut s = schematic_session();
        s.open_companion(&board_path, &ViewerSettings::default())
            .unwrap();
        s.plan_forward().unwrap();
        s.apply_preview().unwrap();
        s.plan_backward().unwrap();
        s.apply_preview().unwrap();
        assert!(s.dirty());
        assert_eq!(
            s.save_board_as(&board_path, ".bak").unwrap_err().code,
            SAVE_REFUSED
        );
        let new_board = dir.path().join("board_new.json");
        s.save_board_as(&new_board, ".bak").unwrap();
        let new_schematic = dir.path().join("schematic_new.json");
        assert_eq!(
            s.save_schematic_as(&schematic_path, Some(&schematic_path), ".bak")
                .unwrap_err()
                .code,
            SAVE_REFUSED
        );
        assert_eq!(
            s.save_schematic_as(&dir.path().join("x.sch"), None, ".bak")
                .unwrap_err()
                .code,
            SAVE_FAILED
        );
        s.save_schematic_as(&new_schematic, Some(&schematic_path), ".bak")
            .unwrap();
        assert!(!s.dirty());
        let board = Board::from_bytes(&std::fs::read(&new_board).unwrap()).unwrap();
        assert_eq!(board, s.companion.as_ref().unwrap().board);
        let schematic = Schematic::from_bytes(&std::fs::read(&new_schematic).unwrap()).unwrap();
        assert_eq!(schematic, s.schematic);
        assert_eq!(
            std::fs::read(&schematic_path).unwrap(),
            sample::schematic().to_bytes().unwrap()
        );
    }
}
