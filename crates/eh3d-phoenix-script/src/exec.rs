// SPDX-License-Identifier: GPL-3.0-or-later
//! Running a script on a board or on a schematic.
//!
//! A run works on a copy of the document. Every change goes through a reversible operation of the
//! board model, so the run ends with one batch that does the whole script and one batch that undoes
//! it: applying the first to the original gives the result and applying the second to the result
//! gives the original, byte for byte. A script that has a syntax problem, an unknown command or, by
//! default, a pending command is not started at all; an error in the middle of a run drops the copy
//! and the original is left as it was. Nothing is written by a run: a `WRITE` is only noted, and
//! [`commit_writes`] writes it when the caller decides to.
//!
//! The identities of new objects come from a generator seeded by the settings and are never in use,
//! so the same script on the same board always gives the same board.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use eh3d_phoenix_board::{Board, Id, LayerKind, Operation, Segment, SilkItem, Track, Via};
use eh3d_phoenix_erc::{check as erc_check, ErcConfig};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, Nm, Point, StepClock, Udeg};
use eh3d_phoenix_route::RouteSettings;
use eh3d_phoenix_rules::{check_board, Lang, RuleValues};
use eh3d_phoenix_schematic::Schematic;

use crate::actions::{
    interpret, Action, ChangeWhat, DisplayOp, GridAction, GroupAction, LayerRef, Pt,
};
use crate::codes::{
    Diag, ARGUMENT_INVALID, C_CURVE, C_KNOWN_PENDING, C_LAYER_CREATE, C_MIRROR, C_PICK,
    C_SET_OPTION, FILE_FAILED, ID_EXHAUSTED, LAYER_KIND, LAYER_UNKNOWN, LIMIT_EXCEEDED, NESTING,
    NUMBER_INVALID, OBJECT_NOT_FOUND, OPERATION_REFUSED, PATH_REFUSED, PENDING_FORM,
    ROTATION_INVALID, UNKNOWN_COMMAND, WRONG_TARGET,
};
use crate::files;
use crate::parse::{parse_script, Parsed};
use crate::settings::ScriptSettings;
use crate::values::{Len, Unit};

/// What a run needs besides the document and the script.
#[derive(Debug, Clone)]
pub struct RunContext<'a> {
    /// The settings.
    pub settings: &'a ScriptSettings,
    /// The values `DRC` checks the board against; every rule is off in the default.
    pub rules: RuleValues,
    /// The settings of the ratsnest.
    pub route: RouteSettings,
    /// The language of the messages of `DRC`, `ERC` and `RATSNEST`.
    pub lang: Lang,
    /// The folder that relative paths of `SCRIPT`, `OPEN` and `WRITE` start from; without it every
    /// file command is refused.
    pub base_dir: Option<&'a Path>,
    /// The parts base that `assign_part` of a script of the second version looks parts up in.
    pub parts: Option<&'a eh3d_phoenix_parts::PartsDb>,
}

impl<'a> RunContext<'a> {
    /// A context with the default rules, the default ratsnest settings, English and no folder.
    pub fn new(settings: &'a ScriptSettings) -> RunContext<'a> {
        RunContext {
            settings,
            rules: RuleValues::default(),
            route: RouteSettings::default(),
            lang: Lang::En,
            base_dir: None,
            parts: None,
        }
    }
}

/// A file that a `WRITE` asked for: it is written only by [`commit_writes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingWrite {
    /// The place, below the folder of the script.
    pub path: PathBuf,
    /// The canonical bytes of the board as it was at the `WRITE`.
    pub bytes: Vec<u8>,
}

/// The state of a run at its end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// The unit of numbers without a unit.
    pub unit: Unit,
    /// The grid, in nanometres.
    pub grid_nm: i64,
    /// Whether the grid is on.
    pub grid_on: bool,
    /// The layer new objects go to.
    pub layer: Option<Id>,
    /// The width of wires and strokes, in nanometres.
    pub width_nm: i64,
    /// The drill of vias, in nanometres.
    pub drill_nm: i64,
    /// The height of texts, in nanometres.
    pub size_nm: i64,
    /// Every layer is shown.
    pub display_all: bool,
    /// The layers shown by name, in capitals.
    pub display: Vec<String>,
    /// The number of selected objects.
    pub selected: usize,
}

/// The result of a run.
#[derive(Debug, Clone)]
pub struct Run {
    /// True when the whole script was carried out.
    pub ok: bool,
    /// The board after the script (a board target that ended well).
    pub board: Option<Board>,
    /// The batch that turns the original board into the result.
    pub batch: Option<Operation>,
    /// The batch that turns the result back into the original; none after an `OPEN`, which
    /// replaces the board.
    pub undo: Option<Operation>,
    /// Why the run stopped or did not start, in order.
    pub diagnostics: Vec<Diag>,
    /// Pending commands that were skipped.
    pub warnings: Vec<Diag>,
    /// The text of `DRC`, `ERC` and `RATSNEST`.
    pub messages: Vec<String>,
    /// The commands carried out.
    pub executed: usize,
    /// The commands skipped because they are pending.
    pub skipped: usize,
    /// The number of operations made.
    pub operations: usize,
    /// How many checks (`DRC`, `ERC`) proved an error.
    pub failed_checks: usize,
    /// The files a `WRITE` asked for.
    pub writes: Vec<PendingWrite>,
    /// The state at the end.
    pub summary: Option<Summary>,
}

/// Writes the files a successful run asked for. A file that exists is never replaced.
pub fn commit_writes(run: &Run) -> Result<(), Diag> {
    if !run.ok {
        return Err(Diag::plain(FILE_FAILED, "the run did not end well"));
    }
    for write in &run.writes {
        files::write_new(&write.path, &write.bytes)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Selected {
    Component(Id),
    Track(Id),
    Via(Id),
    Silk(Id),
}

struct State {
    unit: Unit,
    grid_nm: i64,
    grid_on: bool,
    last_grid: (Unit, i64, bool),
    layer: Option<Id>,
    width: Nm,
    drill: Nm,
    via_diameter: Nm,
    size: Nm,
    selection: BTreeSet<Selected>,
    display_all: bool,
    display: BTreeSet<String>,
    last_display: (bool, BTreeSet<String>),
}

fn nm_or_zero(value: i64) -> Nm {
    Nm::new(value).unwrap_or(Nm::ZERO)
}

impl State {
    fn new(s: &ScriptSettings) -> State {
        State {
            unit: s.default_unit,
            grid_nm: s.default_grid_nm,
            grid_on: true,
            last_grid: (s.default_unit, s.default_grid_nm, true),
            layer: None,
            width: nm_or_zero(s.default_width_nm),
            drill: nm_or_zero(s.default_drill_nm),
            via_diameter: nm_or_zero(s.default_via_diameter_nm),
            size: nm_or_zero(s.default_text_height_nm),
            selection: BTreeSet::new(),
            display_all: true,
            display: BTreeSet::new(),
            last_display: (true, BTreeSet::new()),
        }
    }
}

pub(crate) struct Ids {
    generator: IdGenerator<StepClock, DeterministicRandom>,
    pub(crate) used: BTreeSet<Id>,
}

impl Ids {
    pub(crate) fn new(s: &ScriptSettings, used: BTreeSet<Id>) -> Ids {
        Ids {
            generator: IdGenerator::new(
                StepClock::new(s.id_epoch_ms, 1),
                DeterministicRandom::new(s.id_seed),
            ),
            used,
        }
    }

    pub(crate) fn fresh(&mut self) -> Result<Id, Diag> {
        for _ in 0..1000 {
            match self.generator.generate() {
                Ok(id) => {
                    if self.used.insert(id) {
                        return Ok(id);
                    }
                }
                Err(e) => return Err(Diag::plain(ID_EXHAUSTED, e.to_string())),
            }
        }
        Err(Diag::plain(ID_EXHAUSTED, "every identity tried was in use"))
    }
}

/// Every id that the board already uses.
pub(crate) fn used_ids(board: &Board) -> BTreeSet<Id> {
    let mut used: BTreeSet<Id> = BTreeSet::new();
    used.extend(board.layers.keys());
    used.extend(board.nets.keys());
    used.extend(board.footprints.keys());
    for footprint in board.footprints.values() {
        used.extend(footprint.pads.keys());
    }
    used.extend(board.components.keys());
    used.extend(board.tracks.keys());
    used.extend(board.vias.keys());
    used.extend(board.zones.keys());
    used.extend(board.silkscreen.keys());
    used
}

fn is_pending_cause(code: &str) -> bool {
    [
        C_KNOWN_PENDING,
        C_PICK,
        C_CURVE,
        C_LAYER_CREATE,
        C_MIRROR,
        C_SET_OPTION,
        PENDING_FORM,
    ]
    .contains(&code)
}

/// The problems of a script that stop it from starting, and the pending commands that are skipped
/// with a warning (only when the settings say so).
fn precheck(parsed: &Parsed, settings: &ScriptSettings) -> (Vec<Diag>, Vec<Diag>) {
    let mut errors: Vec<Diag> = parsed.issues.clone();
    let mut warnings = Vec::new();
    for cmd in &parsed.commands {
        if let Err(reject) = interpret(cmd, settings) {
            let detail = if reject.code == UNKNOWN_COMMAND {
                reject.detail
            } else {
                format!("{}: {}", cmd.name, reject.detail)
            };
            let diag = Diag::new(reject.code, cmd.line, cmd.col, detail);
            if is_pending_cause(reject.code) && settings.skip_pending {
                warnings.push(diag);
            } else {
                errors.push(diag);
            }
        }
    }
    (errors, warnings)
}

struct Runner<'a> {
    ctx: &'a RunContext<'a>,
    board: Board,
    schematic: Option<&'a Schematic>,
    state: State,
    ids: Ids,
    ops: Vec<Operation>,
    inverses: Vec<Operation>,
    replaced: bool,
    warnings: Vec<Diag>,
    messages: Vec<String>,
    commands_run: usize,
    executed: usize,
    skipped: usize,
    failed_checks: usize,
    writes: Vec<PendingWrite>,
}

fn invalid(what: &str) -> Diag {
    Diag::plain(NUMBER_INVALID, what.to_owned())
}

impl<'a> Runner<'a> {
    fn new(ctx: &'a RunContext<'a>, board: Board, schematic: Option<&'a Schematic>) -> Runner<'a> {
        let used = used_ids(&board);
        Runner {
            ctx,
            board,
            schematic,
            state: State::new(ctx.settings),
            ids: Ids::new(ctx.settings, used),
            ops: Vec::new(),
            inverses: Vec::new(),
            replaced: false,
            warnings: Vec::new(),
            messages: Vec::new(),
            commands_run: 0,
            executed: 0,
            skipped: 0,
            failed_checks: 0,
            writes: Vec::new(),
        }
    }

    fn len(&self, l: &Len) -> Result<Nm, Diag> {
        l.to_nm(self.state.unit, self.state.grid_nm)
            .ok_or_else(|| invalid("the length does not fit the range of the board"))
    }

    fn positive(&self, l: &Len, what: &str) -> Result<Nm, Diag> {
        let n = self.len(l)?;
        if n.get() <= 0 {
            return Err(invalid(&format!("{what} must be positive")));
        }
        Ok(n)
    }

    fn point(&self, p: &Pt) -> Result<Point, Diag> {
        Ok(Point {
            x_nm: self.len(&p.x)?,
            y_nm: self.len(&p.y)?,
        })
    }

    fn layer_id(&self, r: &LayerRef) -> Result<Id, Diag> {
        let name = match r {
            LayerRef::Number(n) => self
                .ctx
                .settings
                .layer_name(*n)
                .map(str::to_owned)
                .ok_or_else(|| {
                    Diag::plain(
                        LAYER_UNKNOWN,
                        format!("layer number {n} is not in the layer table"),
                    )
                })?,
            LayerRef::Name(name) => name.clone(),
        };
        self.board
            .layers
            .iter()
            .find(|(_, l)| l.name.eq_ignore_ascii_case(&name))
            .map(|(id, _)| *id)
            .ok_or_else(|| Diag::plain(LAYER_UNKNOWN, format!("the board has no layer {name}")))
    }

    fn current_layer(&self) -> Result<(Id, LayerKind), Diag> {
        let id = self.state.layer.ok_or_else(|| {
            Diag::plain(ARGUMENT_INVALID, "no layer is selected, use LAYER first")
        })?;
        let kind =
            self.board.layers.get(&id).map(|l| l.kind).ok_or_else(|| {
                Diag::plain(LAYER_UNKNOWN, "the selected layer is not on the board")
            })?;
        Ok((id, kind))
    }

    fn silk_layer(&self, what: &str) -> Result<Id, Diag> {
        let (id, kind) = self.current_layer()?;
        if kind != LayerKind::Silkscreen {
            return Err(Diag::plain(
                LAYER_KIND,
                format!("{what} goes on a silkscreen layer"),
            ));
        }
        Ok(id)
    }

    fn outer_copper(&self) -> Result<(Id, Id), Diag> {
        if let (Some(first), Some(last)) = (
            self.board.copper_stack.first(),
            self.board.copper_stack.last(),
        ) {
            if self.board.copper_stack.len() >= 2 {
                return Ok((*first, *last));
            }
        }
        let copper: Vec<Id> = self
            .board
            .layers
            .iter()
            .filter(|(_, l)| l.kind == LayerKind::Copper)
            .map(|(id, _)| *id)
            .collect();
        match (copper.first(), copper.last()) {
            (Some(first), Some(last)) if copper.len() >= 2 => Ok((*first, *last)),
            _ => Err(Diag::plain(
                LAYER_UNKNOWN,
                "the board has fewer than two copper layers",
            )),
        }
    }

    fn push(&mut self, op: Operation) -> Result<(), Diag> {
        if self.ops.len() >= self.ctx.settings.max_operations {
            return Err(Diag::plain(
                LIMIT_EXCEEDED,
                format!(
                    "more than {} operations in one run",
                    self.ctx.settings.max_operations
                ),
            ));
        }
        let inverse = self
            .board
            .apply(&op)
            .map_err(|e| Diag::plain(OPERATION_REFUSED, e.to_string()))?;
        self.ops.push(op);
        self.inverses.push(inverse);
        Ok(())
    }

    fn component_by_name(&self, name: &str) -> Option<Id> {
        self.board
            .components
            .iter()
            .find(|(_, c)| c.refdes == name)
            .map(|(id, _)| *id)
    }

    fn net_by_name(&self, name: &str) -> Option<Id> {
        self.board
            .nets
            .iter()
            .find(|(_, n)| n.name == name)
            .map(|(id, _)| *id)
    }

    fn net_or_new(&mut self, name: &str) -> Result<Id, Diag> {
        if let Some(id) = self.net_by_name(name) {
            return Ok(id);
        }
        let id = self.ids.fresh()?;
        self.push(Operation::AddNet {
            id,
            name: name.to_owned(),
        })?;
        Ok(id)
    }

    fn stroke_for(&self, height: Nm) -> Result<Nm, Diag> {
        let ratio = self.ctx.settings.text_ratio_percent;
        let value = (height.get().saturating_mul(ratio) + 50) / 100;
        Nm::new(value.max(1)).map_err(|_| invalid("the stroke of the text does not fit the range"))
    }

    /// What a name stands for: a component by its reference designator, or the tracks and vias of
    /// a net by its name.
    fn resolve_name(&self, name: &str) -> Vec<Selected> {
        if let Some(id) = self.component_by_name(name) {
            return vec![Selected::Component(id)];
        }
        let Some(net) = self.net_by_name(name) else {
            return Vec::new();
        };
        let mut out: Vec<Selected> = self
            .board
            .tracks
            .iter()
            .filter(|(_, t)| t.net == net)
            .map(|(id, _)| Selected::Track(*id))
            .collect();
        out.extend(
            self.board
                .vias
                .iter()
                .filter(|(_, v)| v.net == net)
                .map(|(id, _)| Selected::Via(*id)),
        );
        out
    }

    fn targets(&self, names: &[String]) -> Result<Vec<Selected>, Diag> {
        if names.is_empty() {
            return Ok(self.state.selection.iter().copied().collect());
        }
        let mut out = BTreeSet::new();
        for name in names {
            let found = self.resolve_name(name);
            if found.is_empty() {
                return Err(Diag::plain(OBJECT_NOT_FOUND, name.clone()));
            }
            out.extend(found);
        }
        Ok(out.into_iter().collect())
    }

    fn component_targets(&self, names: &[String]) -> Result<Vec<Id>, Diag> {
        if names.is_empty() {
            return Ok(self
                .state
                .selection
                .iter()
                .filter_map(|s| match s {
                    Selected::Component(id) => Some(*id),
                    _ => None,
                })
                .collect());
        }
        let mut out = BTreeSet::new();
        for name in names {
            match self.component_by_name(name) {
                Some(id) => {
                    out.insert(id);
                }
                None => return Err(Diag::plain(OBJECT_NOT_FOUND, name.clone())),
            }
        }
        Ok(out.into_iter().collect())
    }

    fn require_board(&self) -> Result<(), Diag> {
        if self.schematic.is_some() {
            return Err(Diag::plain(
                WRONG_TARGET,
                "this command edits a board and the document is a schematic",
            ));
        }
        Ok(())
    }

    fn execute(&mut self, action: &Action) -> Result<(), Diag> {
        match action {
            Action::Grid(g) => self.grid(g),
            Action::Display(ops) => {
                self.display(ops);
                Ok(())
            }
            Action::Set { .. } => Ok(()),
            Action::Erc => self.erc(),
            Action::Script(_) => Err(Diag::plain(ARGUMENT_INVALID, "SCRIPT is run by the loop")),
            _ => {
                self.require_board()?;
                self.edit(action)
            }
        }
    }

    fn edit(&mut self, action: &Action) -> Result<(), Diag> {
        match action {
            Action::Layer(r) => {
                self.state.layer = Some(self.layer_id(r)?);
                Ok(())
            }
            Action::Change { what, names } => self.change(what, names),
            Action::Wire {
                net, width, points, ..
            } => self.wire(net.as_deref(), width.as_ref(), points),
            Action::Via {
                net, diameter, at, ..
            } => self.via(net.as_deref(), diameter.as_ref(), at),
            Action::Circle {
                width,
                center,
                through,
            } => self.circle(width.as_ref(), center, through),
            Action::Rect { a, b } => self.rect(a, b),
            Action::Text { text, rot, at } => self.text(text, rot.as_ref(), at),
            Action::Move { name, to } => self.move_component(name, to),
            Action::Rotate { rot, names } => self.rotate(rot.angle.get(), rot.absolute, names),
            Action::Delete { names } => self.delete(names),
            Action::Name { old, new } => self.rename(old, new),
            Action::Value { name, value } => self.set_value(name, value),
            Action::Group(g) => self.group(g),
            Action::Ratsnest => self.ratsnest(),
            Action::Drc => self.drc(),
            Action::Write(path) => self.write(path),
            Action::Open(path) => self.open(path),
            _ => Err(Diag::plain(
                ARGUMENT_INVALID,
                "the command was routed to the wrong place",
            )),
        }
    }

    fn grid(&mut self, g: &GridAction) -> Result<(), Diag> {
        let before = (self.state.unit, self.state.grid_nm, self.state.grid_on);
        match g {
            GridAction::Style => return Ok(()),
            GridAction::Last => {
                let (unit, grid, on) = self.state.last_grid;
                self.state.unit = unit;
                self.state.grid_nm = grid;
                self.state.grid_on = on;
            }
            GridAction::Default => {
                let s = self.ctx.settings;
                self.state.unit = s.default_unit;
                self.state.grid_nm = s.default_grid_nm;
                self.state.grid_on = true;
            }
            GridAction::Set { unit, size, on } => {
                if let Some(u) = unit {
                    self.state.unit = *u;
                }
                if let Some(l) = size {
                    if l.unit == Some(Unit::Grid) {
                        return Err(invalid("the grid cannot be given in grid steps"));
                    }
                    self.state.grid_nm = self.positive(l, "the grid")?.get();
                }
                if let Some(on) = on {
                    self.state.grid_on = *on;
                }
            }
        }
        self.state.last_grid = before;
        Ok(())
    }

    fn display(&mut self, ops: &[DisplayOp]) {
        let before = (self.state.display_all, self.state.display.clone());
        for op in ops {
            match op {
                DisplayOp::All => {
                    self.state.display_all = true;
                    self.state.display.clear();
                }
                DisplayOp::None => {
                    self.state.display_all = false;
                    self.state.display.clear();
                }
                DisplayOp::Last => {
                    self.state.display_all = self.state.last_display.0;
                    self.state.display = self.state.last_display.1.clone();
                }
                DisplayOp::Show(name) => {
                    self.state.display.insert(name.to_ascii_uppercase());
                }
                DisplayOp::Hide(name) => {
                    self.state.display.remove(&name.to_ascii_uppercase());
                }
            }
        }
        self.state.last_display = before;
    }

    fn change(&mut self, what: &ChangeWhat, names: &[String]) -> Result<(), Diag> {
        match what {
            ChangeWhat::Width(l) => {
                let width = self.positive(l, "the width")?;
                self.state.width = width;
                for target in self.targets(names)? {
                    match target {
                        Selected::Track(id) => {
                            let Some(mut track) = self.board.tracks.get(&id).cloned() else {
                                continue;
                            };
                            if track.width != width {
                                track.width = width;
                                self.replace_track(id, track)?;
                            }
                        }
                        Selected::Silk(id) => {
                            if let Some(SilkItem::Stroke { layer, path, .. }) =
                                self.board.silkscreen.get(&id).cloned()
                            {
                                let item = SilkItem::Stroke { layer, width, path };
                                self.replace_silk(id, item)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            ChangeWhat::Drill(l) => {
                let drill = self.positive(l, "the drill")?;
                self.state.drill = drill;
                for target in self.targets(names)? {
                    if let Selected::Via(id) = target {
                        let Some(mut via) = self.board.vias.get(&id).cloned() else {
                            continue;
                        };
                        if via.drill != drill {
                            if drill >= via.diameter {
                                return Err(invalid("the drill must be smaller than the via"));
                            }
                            via.drill = drill;
                            self.push(Operation::RemoveVia { id })?;
                            self.push(Operation::AddVia { id, via })?;
                        }
                    }
                }
            }
            ChangeWhat::Size(l) => {
                let size = self.positive(l, "the size")?;
                self.state.size = size;
                let stroke = self.stroke_for(size)?;
                for target in self.targets(names)? {
                    if let Selected::Silk(id) = target {
                        if let Some(SilkItem::Text {
                            layer,
                            at,
                            text,
                            rotation,
                            mirrored,
                            ..
                        }) = self.board.silkscreen.get(&id).cloned()
                        {
                            let item = SilkItem::Text {
                                layer,
                                at,
                                text,
                                height: size,
                                stroke,
                                rotation,
                                mirrored,
                            };
                            self.replace_silk(id, item)?;
                        }
                    }
                }
            }
            ChangeWhat::Layer(r) => {
                let layer = self.layer_id(r)?;
                let kind = self.board.layers.get(&layer).map(|l| l.kind);
                self.state.layer = Some(layer);
                for target in self.targets(names)? {
                    match target {
                        Selected::Track(id) => {
                            if kind != Some(LayerKind::Copper) {
                                return Err(Diag::plain(
                                    LAYER_KIND,
                                    "a track goes on a copper layer",
                                ));
                            }
                            let Some(mut track) = self.board.tracks.get(&id).cloned() else {
                                continue;
                            };
                            if track.layer != layer {
                                track.layer = layer;
                                self.replace_track(id, track)?;
                            }
                        }
                        Selected::Silk(id) => {
                            if kind != Some(LayerKind::Silkscreen) {
                                return Err(Diag::plain(
                                    LAYER_KIND,
                                    "silkscreen goes on a silkscreen layer",
                                ));
                            }
                            let Some(mut item) = self.board.silkscreen.get(&id).cloned() else {
                                continue;
                            };
                            match &mut item {
                                SilkItem::Text { layer: l, .. }
                                | SilkItem::Stroke { layer: l, .. } => *l = layer,
                            }
                            self.replace_silk(id, item)?;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn replace_track(&mut self, id: Id, track: Track) -> Result<(), Diag> {
        self.push(Operation::RemoveTrack { id })?;
        self.push(Operation::AddTrack { id, track })
    }

    fn replace_silk(&mut self, id: Id, item: SilkItem) -> Result<(), Diag> {
        self.push(Operation::RemoveSilk { id })?;
        self.push(Operation::AddSilk { id, item })
    }

    fn wire(&mut self, net: Option<&str>, width: Option<&Len>, points: &[Pt]) -> Result<(), Diag> {
        let (layer, kind) = self.current_layer()?;
        let width = match width {
            Some(l) => self.positive(l, "the width")?,
            None => self.state.width,
        };
        let mut pts = Vec::with_capacity(points.len());
        for p in points {
            pts.push(self.point(p)?);
        }
        let mut path = Vec::new();
        for pair in pts.windows(2) {
            if pair[0] != pair[1] {
                path.push(Segment::Line {
                    start: pair[0],
                    end: pair[1],
                });
            }
        }
        if path.is_empty() {
            return Err(Diag::plain(ARGUMENT_INVALID, "all the points coincide"));
        }
        match kind {
            LayerKind::Copper => {
                let Some(name) = net else {
                    return Err(Diag::plain(
                        ARGUMENT_INVALID,
                        "a wire on a copper layer needs a signal name",
                    ));
                };
                let net_id = self.net_or_new(name)?;
                let id = self.ids.fresh()?;
                self.push(Operation::AddTrack {
                    id,
                    track: Track {
                        net: net_id,
                        layer,
                        width,
                        path,
                    },
                })
            }
            LayerKind::Silkscreen => {
                let id = self.ids.fresh()?;
                self.push(Operation::AddSilk {
                    id,
                    item: SilkItem::Stroke { layer, width, path },
                })
            }
            _ => Err(Diag::plain(
                LAYER_KIND,
                "a wire goes on a copper or a silkscreen layer",
            )),
        }
    }

    fn via(&mut self, net: Option<&str>, diameter: Option<&Len>, at: &Pt) -> Result<(), Diag> {
        let Some(name) = net else {
            return Err(Diag::plain(ARGUMENT_INVALID, "a via needs a signal name"));
        };
        let (from_layer, to_layer) = self.outer_copper()?;
        let diameter = match diameter {
            Some(l) => self.positive(l, "the diameter")?,
            None => self.state.via_diameter,
        };
        let drill = self.state.drill;
        if drill >= diameter {
            return Err(invalid("the drill must be smaller than the via"));
        }
        let at = self.point(at)?;
        let net = self.net_or_new(name)?;
        let id = self.ids.fresh()?;
        self.push(Operation::AddVia {
            id,
            via: Via {
                net,
                at,
                drill,
                diameter,
                from_layer,
                to_layer,
            },
        })
    }

    fn circle(&mut self, width: Option<&Len>, center: &Pt, through: &Pt) -> Result<(), Diag> {
        let layer = self.silk_layer("a circle")?;
        let width = match width {
            Some(l) => self.positive(l, "the width")?,
            None => self.state.width,
        };
        let c = self.point(center)?;
        let p = self.point(through)?;
        if c == p {
            return Err(Diag::plain(ARGUMENT_INVALID, "the circle has no radius"));
        }
        let opposite = Point::from_nm(
            2 * c.x_nm.get() - p.x_nm.get(),
            2 * c.y_nm.get() - p.y_nm.get(),
        )
        .map_err(|_| invalid("the circle does not fit the range of the board"))?;
        let half = Udeg::new(180_000_000).map_err(|_| invalid("the sweep of the circle"))?;
        let path = vec![
            Segment::Arc {
                start: p,
                end: opposite,
                sweep: half,
            },
            Segment::Arc {
                start: opposite,
                end: p,
                sweep: half,
            },
        ];
        let id = self.ids.fresh()?;
        self.push(Operation::AddSilk {
            id,
            item: SilkItem::Stroke { layer, width, path },
        })
    }

    fn rect(&mut self, a: &Pt, b: &Pt) -> Result<(), Diag> {
        let layer = self.silk_layer("a rectangle")?;
        let width = self.state.width;
        let a = self.point(a)?;
        let b = self.point(b)?;
        if a.x_nm == b.x_nm || a.y_nm == b.y_nm {
            return Err(Diag::plain(ARGUMENT_INVALID, "the rectangle has no area"));
        }
        let corners = [
            a,
            Point {
                x_nm: b.x_nm,
                y_nm: a.y_nm,
            },
            b,
            Point {
                x_nm: a.x_nm,
                y_nm: b.y_nm,
            },
            a,
        ];
        let path: Vec<Segment> = corners
            .windows(2)
            .map(|pair| Segment::Line {
                start: pair[0],
                end: pair[1],
            })
            .collect();
        let id = self.ids.fresh()?;
        self.push(Operation::AddSilk {
            id,
            item: SilkItem::Stroke { layer, width, path },
        })
    }

    fn text(&mut self, text: &str, rot: Option<&crate::values::Rot>, at: &Pt) -> Result<(), Diag> {
        let layer = self.silk_layer("a text")?;
        if text.is_empty() {
            return Err(Diag::plain(ARGUMENT_INVALID, "the text is empty"));
        }
        let (angle, mirrored) = match rot {
            Some(r) => (r.angle.get(), r.mirror),
            None => (0, false),
        };
        if angle % 90_000_000 != 0 {
            return Err(Diag::plain(
                ROTATION_INVALID,
                "a text turns by multiples of 90 degrees",
            ));
        }
        let rotation = Udeg::new(angle.rem_euclid(360_000_000))
            .map_err(|_| Diag::plain(ROTATION_INVALID, "the rotation of the text"))?;
        let height = self.state.size;
        let stroke = self.stroke_for(height)?;
        let at = self.point(at)?;
        let id = self.ids.fresh()?;
        self.push(Operation::AddSilk {
            id,
            item: SilkItem::Text {
                layer,
                at,
                text: text.to_owned(),
                height,
                stroke,
                rotation,
                mirrored,
            },
        })
    }

    fn move_component(&mut self, name: &str, to: &Pt) -> Result<(), Diag> {
        let id = self
            .component_by_name(name)
            .ok_or_else(|| Diag::plain(OBJECT_NOT_FOUND, name.to_owned()))?;
        let rotation = self
            .board
            .components
            .get(&id)
            .map(|c| c.rotation)
            .ok_or_else(|| Diag::plain(OBJECT_NOT_FOUND, name.to_owned()))?;
        let at = self.point(to)?;
        self.push(Operation::MoveComponent { id, at, rotation })
    }

    fn rotate(&mut self, angle: i64, absolute: bool, names: &[String]) -> Result<(), Diag> {
        for id in self.component_targets(names)? {
            let Some((at, current)) = self
                .board
                .components
                .get(&id)
                .map(|c| (c.at, c.rotation.get()))
            else {
                continue;
            };
            let turned = if absolute { angle } else { current + angle };
            let rotation = Udeg::new(turned.rem_euclid(360_000_000))
                .map_err(|_| Diag::plain(ROTATION_INVALID, "the rotation of the component"))?;
            self.push(Operation::MoveComponent { id, at, rotation })?;
        }
        Ok(())
    }

    fn delete(&mut self, names: &[String]) -> Result<(), Diag> {
        let targets: Vec<Selected> = if names.is_empty() {
            self.state.selection.iter().copied().collect()
        } else {
            self.component_targets(names)?
                .into_iter()
                .map(Selected::Component)
                .collect()
        };
        for target in targets {
            let op = match target {
                Selected::Component(id) if self.board.components.contains_key(&id) => {
                    Operation::RemoveComponent { id }
                }
                Selected::Track(id) if self.board.tracks.contains_key(&id) => {
                    Operation::RemoveTrack { id }
                }
                Selected::Via(id) if self.board.vias.contains_key(&id) => {
                    Operation::RemoveVia { id }
                }
                Selected::Silk(id) if self.board.silkscreen.contains_key(&id) => {
                    Operation::RemoveSilk { id }
                }
                _ => continue,
            };
            self.push(op)?;
            self.state.selection.remove(&target);
        }
        Ok(())
    }

    fn rename(&mut self, old: &str, new: &str) -> Result<(), Diag> {
        if new.is_empty() {
            return Err(Diag::plain(ARGUMENT_INVALID, "the new name is empty"));
        }
        if let Some(id) = self.component_by_name(old) {
            if self.component_by_name(new).is_some() {
                return Err(Diag::plain(
                    OPERATION_REFUSED,
                    format!("a component is already named {new}"),
                ));
            }
            let Some(c) = self.board.components.get(&id) else {
                return Err(Diag::plain(OBJECT_NOT_FOUND, old.to_owned()));
            };
            let op = Operation::SetComponentFields {
                id,
                refdes: new.to_owned(),
                value: c.value.clone(),
                footprint: c.footprint,
            };
            return self.push(op);
        }
        if let Some(id) = self.net_by_name(old) {
            if self.net_by_name(new).is_some() {
                return Err(Diag::plain(
                    OPERATION_REFUSED,
                    format!("a signal is already named {new}"),
                ));
            }
            return self.push(Operation::RenameNet {
                id,
                name: new.to_owned(),
            });
        }
        Err(Diag::plain(OBJECT_NOT_FOUND, old.to_owned()))
    }

    fn set_value(&mut self, name: &str, value: &str) -> Result<(), Diag> {
        let id = self
            .component_by_name(name)
            .ok_or_else(|| Diag::plain(OBJECT_NOT_FOUND, name.to_owned()))?;
        let Some(c) = self.board.components.get(&id) else {
            return Err(Diag::plain(OBJECT_NOT_FOUND, name.to_owned()));
        };
        let op = Operation::SetComponentFields {
            id,
            refdes: c.refdes.clone(),
            value: value.to_owned(),
            footprint: c.footprint,
        };
        self.push(op)
    }

    fn group(&mut self, g: &GroupAction) -> Result<(), Diag> {
        match g {
            GroupAction::None => self.state.selection.clear(),
            GroupAction::All => {
                let mut all = BTreeSet::new();
                all.extend(
                    self.board
                        .components
                        .keys()
                        .map(|k| Selected::Component(*k)),
                );
                all.extend(self.board.tracks.keys().map(|k| Selected::Track(*k)));
                all.extend(self.board.vias.keys().map(|k| Selected::Via(*k)));
                all.extend(self.board.silkscreen.keys().map(|k| Selected::Silk(*k)));
                self.state.selection = all;
            }
            GroupAction::Names(names) => {
                let found: Vec<Selected> = self.targets(names)?;
                self.state.selection = found.into_iter().collect();
            }
        }
        Ok(())
    }

    fn ratsnest(&mut self) -> Result<(), Diag> {
        let lines = eh3d_phoenix_route::ratsnest(&self.board, &self.ctx.route)
            .map_err(|e| Diag::plain(OPERATION_REFUSED, e.message(self.ctx.lang)))?;
        self.messages
            .push(eh3d_phoenix_route::ratsnest::to_text(&self.board, &lines));
        Ok(())
    }

    fn drc(&mut self) -> Result<(), Diag> {
        let report = check_board(&self.board, &self.ctx.rules);
        if !report.passes() {
            self.failed_checks += 1;
        }
        self.messages.push(report.to_text(self.ctx.lang));
        Ok(())
    }

    fn erc(&mut self) -> Result<(), Diag> {
        let Some(schematic) = self.schematic else {
            return Err(Diag::plain(
                WRONG_TARGET,
                "ERC needs a schematic as the document",
            ));
        };
        let report = erc_check(schematic, &ErcConfig::default());
        if !report.passes() {
            self.failed_checks += 1;
        }
        self.messages.push(report.to_text(self.ctx.lang));
        Ok(())
    }

    fn write(&mut self, relative: &str) -> Result<(), Diag> {
        if !self.ctx.settings.allow_write {
            return Err(Diag::plain(
                PATH_REFUSED,
                "WRITE is not allowed by the settings",
            ));
        }
        let path = files::resolve(self.ctx.base_dir, relative, self.ctx.settings)?;
        if !has_json_extension(&path) {
            return Err(Diag::plain(
                PATH_REFUSED,
                "a board is written to a file with the extension .json",
            ));
        }
        let bytes = self
            .board
            .to_bytes()
            .map_err(|e| Diag::plain(FILE_FAILED, e.to_string()))?;
        self.writes.push(PendingWrite { path, bytes });
        Ok(())
    }

    fn open(&mut self, relative: &str) -> Result<(), Diag> {
        if !self.ctx.settings.allow_open {
            return Err(Diag::plain(
                PATH_REFUSED,
                "OPEN is not allowed by the settings",
            ));
        }
        let path = files::resolve(self.ctx.base_dir, relative, self.ctx.settings)?;
        if !has_json_extension(&path) {
            return Err(Diag::plain(
                PATH_REFUSED,
                "a board is read from a file with the extension .json",
            ));
        }
        let bytes = files::read_limited(&path, self.ctx.settings.max_file_bytes)?;
        let board = Board::from_bytes(&bytes)
            .map_err(|e| Diag::plain(FILE_FAILED, format!("{relative}: {e}")))?;
        self.ids = Ids::new(self.ctx.settings, used_ids(&board));
        self.board = board;
        self.state.selection.clear();
        self.state.layer = None;
        self.replaced = true;
        Ok(())
    }

    fn include(&mut self, relative: &str, depth: usize) -> Result<(), Vec<Diag>> {
        let one = |d: Diag| vec![d];
        let settings = self.ctx.settings;
        if depth >= settings.max_depth {
            return Err(one(Diag::plain(
                NESTING,
                format!(
                    "more than {} scripts inside one another",
                    settings.max_depth
                ),
            )));
        }
        if !settings.allow_script {
            return Err(one(Diag::plain(
                PATH_REFUSED,
                "SCRIPT is not allowed by the settings",
            )));
        }
        let path = files::resolve(self.ctx.base_dir, relative, settings).map_err(one)?;
        let bytes = files::read_limited(&path, settings.max_script_bytes as u64).map_err(one)?;
        let text = String::from_utf8(bytes).map_err(|_| {
            one(Diag::plain(
                FILE_FAILED,
                format!("{relative} is not UTF-8 text"),
            ))
        })?;
        let parsed = parse_script(&text, settings);
        self.exec_all(&parsed, depth + 1).map_err(|diags| {
            diags
                .into_iter()
                .map(|mut d| {
                    d.detail = format!("in {relative}: {}", d.detail);
                    d
                })
                .collect()
        })
    }

    fn exec_all(&mut self, parsed: &Parsed, depth: usize) -> Result<(), Vec<Diag>> {
        let settings = self.ctx.settings;
        let (errors, warnings) = precheck(parsed, settings);
        if !errors.is_empty() {
            return Err(errors);
        }
        self.warnings.extend(warnings);
        for cmd in &parsed.commands {
            if self.commands_run >= settings.max_commands {
                return Err(vec![Diag::new(
                    LIMIT_EXCEEDED,
                    cmd.line,
                    cmd.col,
                    format!("more than {} commands in one run", settings.max_commands),
                )]);
            }
            self.commands_run += 1;
            let Ok(action) = interpret(cmd, settings) else {
                self.skipped += 1;
                continue;
            };
            if let Action::Script(path) = &action {
                self.include(path, depth).map_err(|diags| {
                    diags
                        .into_iter()
                        .map(|d| d.moved(cmd.line, cmd.col))
                        .collect::<Vec<Diag>>()
                })?;
            } else {
                self.execute(&action)
                    .map_err(|d| vec![d.moved(cmd.line, cmd.col)])?;
            }
            self.executed += 1;
        }
        Ok(())
    }

    fn summary(&self) -> Summary {
        Summary {
            unit: self.state.unit,
            grid_nm: self.state.grid_nm,
            grid_on: self.state.grid_on,
            layer: self.state.layer,
            width_nm: self.state.width.get(),
            drill_nm: self.state.drill.get(),
            size_nm: self.state.size.get(),
            display_all: self.state.display_all,
            display: self.state.display.iter().cloned().collect(),
            selected: self.state.selection.len(),
        }
    }

    fn into_run(self, result: Result<(), Vec<Diag>>) -> Run {
        let summary = Some(self.summary());
        match result {
            Err(diagnostics) => Run {
                ok: false,
                board: None,
                batch: None,
                undo: None,
                diagnostics,
                warnings: self.warnings,
                messages: self.messages,
                executed: self.executed,
                skipped: self.skipped,
                operations: self.ops.len(),
                failed_checks: self.failed_checks,
                writes: Vec::new(),
                summary,
            },
            Ok(()) => {
                let operations = self.ops.len();
                let is_board = self.schematic.is_none();
                let (batch, undo) = if self.replaced || !is_board {
                    (None, None)
                } else {
                    let mut back = self.inverses;
                    back.reverse();
                    (
                        Some(Operation::Batch { ops: self.ops }),
                        Some(Operation::Batch { ops: back }),
                    )
                };
                Run {
                    ok: true,
                    board: is_board.then_some(self.board),
                    batch,
                    undo,
                    diagnostics: Vec::new(),
                    warnings: self.warnings,
                    messages: self.messages,
                    executed: self.executed,
                    skipped: self.skipped,
                    operations,
                    failed_checks: self.failed_checks,
                    writes: self.writes,
                    summary,
                }
            }
        }
    }
}

fn has_json_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// Runs a script on a board. The board is not changed: the result is in [`Run::board`].
pub fn run_board(board: &Board, parsed: &Parsed, ctx: &RunContext<'_>) -> Run {
    let mut runner = Runner::new(ctx, board.clone(), None);
    let result = runner.exec_all(parsed, 0);
    runner.into_run(result)
}

/// Runs a script on a schematic. Only the commands that do not edit a board apply (`GRID`,
/// `DISPLAY`, `SET`, `ERC` and `SCRIPT`); the schematic is not changed.
pub fn run_schematic(schematic: &Schematic, parsed: &Parsed, ctx: &RunContext<'_>) -> Run {
    let mut runner = Runner::new(ctx, Board::default(), Some(schematic));
    let result = runner.exec_all(parsed, 0);
    runner.into_run(result)
}
