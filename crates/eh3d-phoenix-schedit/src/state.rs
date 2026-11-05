// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of the schematic editor: what the person has chosen and is doing, and the schematic
//! that results. The window passes on pointer events and keys; every decision is taken here.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, History, OpError};
use eh3d_phoenix_erc::{self as erc, ErcConfig};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{
    orthogonal_path, Applied, Clip, EditError, EditOp, Editor, Id, Item, PinKind, Schematic,
    Symbol, SymbolDef,
};
use eh3d_phoenix_sim::SimSettings;
use eh3d_phoenix_sync::{ForwardPackage, SyncSettings};

use crate::hit::{hit, in_rect, snap_target};
use crate::palette::{Palette, PaletteEntry};
use crate::settings::SchEditSettings;
use crate::transfer::{export_netlist, plan_transfer, NetlistExport};

/// The tool that a click uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Pick, select by rectangle and drag.
    Select,
    /// Draw wires with horizontal and vertical pieces.
    Wire,
    /// Put a net label with the pending name.
    Label,
    /// Put a power port with the pending name.
    PowerPort,
    /// Put the labels of the bus named by the pending text (`D[0..7]`).
    Bus,
    /// Place the symbol chosen in the palette.
    Place,
}

/// A drag of the selection in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drag {
    /// What is being moved.
    pub items: Vec<Item>,
    /// Where the drag began (on the grid).
    pub from: Point,
    /// Where the pointer is now (on the grid).
    pub to: Point,
}

impl Drag {
    /// The offset of the drag in nanometres.
    pub fn offset(&self) -> (i64, i64) {
        (
            self.to.x_nm.get() - self.from.x_nm.get(),
            self.to.y_nm.get() - self.from.y_nm.get(),
        )
    }
}

/// The schematic editor.
pub struct SchEdit {
    /// The settings.
    pub settings: SchEditSettings,
    /// The tool.
    pub tool: Tool,
    /// What is selected.
    pub selection: BTreeSet<Item>,
    /// What was copied last.
    pub clip: Option<Clip>,
    /// The drag in progress.
    pub drag: Option<Drag>,
    /// The rectangle being drawn to select: its two corners.
    pub marquee: Option<(Point, Point)>,
    /// The start of the wire in progress.
    pub wire_start: Option<Point>,
    /// Whether the wire in progress goes along x first when it turns a corner.
    pub horizontal_first: bool,
    /// The symbols that can be placed.
    pub palette: Palette,
    /// The name of the symbol of the palette that the place tool places.
    pub place_name: Option<String>,
    /// The name that the label, power port and bus tools use.
    pub pending_name: String,
    /// The settings of the electrical rule check.
    pub erc_config: ErcConfig,
    /// The result of the electrical rule check.
    pub erc: erc::Report,
    /// Why the last operation was refused, until the next one works.
    pub error: Option<EditError>,
    /// The package of the transfer to the board that is ready to be looked at.
    pub preview: Option<ForwardPackage>,
    editor: Editor,
    sheet: Option<Id>,
    dirty: bool,
    revision: u64,
}

fn items_of(s: &Schematic, ids: &[Id]) -> BTreeSet<Item> {
    let mut out = BTreeSet::new();
    for id in ids {
        if s.parts.contains_key(id) {
            out.insert(Item::Part(*id));
        } else if s.wires.contains_key(id) {
            out.insert(Item::Wire(*id));
        } else if s.labels.contains_key(id) {
            out.insert(Item::Label(*id));
        } else if s.power_ports.contains_key(id) {
            out.insert(Item::PowerPort(*id));
        }
    }
    out
}

fn exists(s: &Schematic, item: &Item) -> bool {
    match item {
        Item::Part(id) => s.parts.contains_key(id),
        Item::Wire(id) => s.wires.contains_key(id),
        Item::Label(id) => s.labels.contains_key(id),
        Item::PowerPort(id) => s.power_ports.contains_key(id),
        Item::Junction(id) => s.junctions.contains_key(id),
    }
}

/// What the editor compares of a pin: its number, its place and its type.
type PinFacts = (String, Option<Point>, Option<PinKind>);

fn pin_order(t: &PinFacts) -> (String, Option<(i64, i64)>, Option<PinKind>) {
    (t.0.clone(), t.1.map(|p| (p.x_nm.get(), p.y_nm.get())), t.2)
}

/// True when the symbol has the pins of the definition: the same numbers, places and types.
fn same_symbol(s: &Schematic, symbol: &Symbol, def: &SymbolDef) -> bool {
    if symbol.pins.len() != def.pins.len() {
        return false;
    }
    let mut have: Vec<PinFacts> = symbol
        .pins
        .iter()
        .map(|(id, pin)| (pin.number.clone(), pin.at, s.pin_kinds.get(id).copied()))
        .collect();
    let mut want: Vec<PinFacts> = def
        .pins
        .iter()
        .map(|p| (p.number.clone(), Some(p.at), p.kind))
        .collect();
    have.sort_by_key(pin_order);
    want.sort_by_key(pin_order);
    have == want
}

impl SchEdit {
    /// An editor on a schematic. The palette starts with the symbols the schematic already has and
    /// the electrical rule check is run at once.
    pub fn new(schematic: Schematic, settings: SchEditSettings, erc_config: ErcConfig) -> SchEdit {
        let sheet = schematic.sheets.keys().next().copied();
        let mut palette = Palette::new();
        palette.add_schematic("schematic", &schematic);
        let report = erc::check(&schematic, &erc_config);
        let editor = Editor::with_limit(schematic, settings.history_limit);
        let horizontal_first = settings.wire_horizontal_first;
        SchEdit {
            settings,
            tool: Tool::Select,
            selection: BTreeSet::new(),
            clip: None,
            drag: None,
            marquee: None,
            wire_start: None,
            horizontal_first,
            palette,
            place_name: None,
            pending_name: String::new(),
            erc_config,
            erc: report,
            error: None,
            preview: None,
            editor,
            sheet,
            dirty: false,
            revision: 0,
        }
    }

    /// The schematic as it is now.
    pub fn schematic(&self) -> &Schematic {
        self.editor.schematic()
    }

    /// Gives the schematic back.
    pub fn into_schematic(self) -> Schematic {
        self.editor.into_schematic()
    }

    /// A number that grows with every change of the schematic (also with an undo or a redo): the
    /// window compares it with the one it saw to know when to pass the schematic on.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True when there are changes that were not saved.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Says that the schematic was saved.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// True when there is a step to undo.
    pub fn can_undo(&self) -> bool {
        self.editor.can_undo()
    }

    /// True when there is a step to redo.
    pub fn can_redo(&self) -> bool {
        self.editor.can_redo()
    }

    /// The sheet that is drawn on, when the schematic has one.
    pub fn sheet(&self) -> Option<Id> {
        self.sheet
            .filter(|id| self.editor.schematic().sheets.contains_key(id))
            .or_else(|| self.editor.schematic().sheets.keys().next().copied())
    }

    /// Chooses the sheet to draw on; false when there is no such sheet.
    pub fn set_sheet(&mut self, sheet: Id) -> bool {
        if self.editor.schematic().sheets.contains_key(&sheet) {
            self.sheet = Some(sheet);
            self.selection.clear();
            self.cancel();
            true
        } else {
            false
        }
    }

    /// The sheet to draw on, adding the sheet of the settings when the schematic has none.
    pub fn ensure_sheet(&mut self) -> Result<Id, EditError> {
        if let Some(sheet) = self.sheet() {
            return Ok(sheet);
        }
        let width = eh3d_phoenix_model::Nm::new(self.settings.new_sheet_width_nm)
            .map_err(|_| EditError::OutOfRange)?;
        let height = eh3d_phoenix_model::Nm::new(self.settings.new_sheet_height_nm)
            .map_err(|_| EditError::OutOfRange)?;
        let made = self.run(&EditOp::AddSheet {
            name: self.settings.new_sheet_name.clone(),
            width,
            height,
        })?;
        let sheet = made
            .created
            .first()
            .copied()
            .ok_or(EditError::BadSelection("the sheet was not made"))?;
        self.sheet = Some(sheet);
        Ok(sheet)
    }

    /// Applies an operation. A refusal is kept in `error` and the schematic is as it was; a success
    /// runs the electrical rule check (when the settings ask for it) and drops from the selection
    /// what is gone.
    pub fn run(&mut self, op: &EditOp) -> Result<Applied, EditError> {
        match self.editor.apply(op) {
            Ok(applied) => {
                self.error = None;
                self.dirty = true;
                self.after_change();
                Ok(applied)
            }
            Err(e) => {
                self.error = Some(e.clone());
                Err(e)
            }
        }
    }

    fn after_change(&mut self) {
        self.revision += 1;
        let s = self.editor.schematic();
        self.selection.retain(|item| exists(s, item));
        self.preview = None;
        if self.settings.live_erc {
            self.run_erc();
        }
    }

    /// Runs the electrical rule check on the schematic as it is.
    pub fn run_erc(&mut self) {
        self.erc = erc::check(self.editor.schematic(), &self.erc_config);
    }

    /// Undoes the last step.
    pub fn undo(&mut self) -> bool {
        let done = self.editor.undo();
        if done {
            self.dirty = true;
            self.after_change();
        }
        done
    }

    /// Redoes the step that was undone last.
    pub fn redo(&mut self) -> bool {
        let done = self.editor.redo();
        if done {
            self.dirty = true;
            self.after_change();
        }
        done
    }

    /// Where a click on `at` lands: on a pin or wire point nearby, else on the grid.
    pub fn snap(&self, at: Point) -> Point {
        match self.sheet() {
            Some(sheet) => snap_target(
                self.editor.schematic(),
                sheet,
                at,
                self.settings.grid_nm,
                self.settings.snap,
                self.settings.pick_tolerance_nm,
            ),
            None => at,
        }
    }

    /// Chooses a tool and drops what the previous one had in progress.
    pub fn set_tool(&mut self, tool: Tool) {
        self.cancel();
        self.tool = tool;
    }

    /// Chooses a symbol of the palette for the place tool and switches to it; false when the
    /// palette has no such symbol.
    pub fn choose_symbol(&mut self, name: &str) -> bool {
        if self.palette.find(name).is_none() {
            return false;
        }
        self.place_name = Some(name.to_owned());
        self.set_tool(Tool::Place);
        true
    }

    /// Drops the wire, the drag and the rectangle in progress.
    pub fn cancel(&mut self) {
        self.wire_start = None;
        self.drag = None;
        self.marquee = None;
    }

    /// Changes which way the wire in progress turns its corner.
    pub fn flip_corner(&mut self) {
        self.horizontal_first = !self.horizontal_first;
    }

    /// A press of the pointer at `at`. `additive` (the shift key) adds to the selection or takes
    /// from it instead of replacing it.
    pub fn pointer_pressed(&mut self, at: Point, additive: bool) {
        let Ok(sheet) = self.ensure_sheet() else {
            return;
        };
        match self.tool {
            Tool::Select => {
                let under = hit(
                    self.editor.schematic(),
                    sheet,
                    at,
                    self.settings.pick_tolerance_nm,
                    self.settings.part_padding_nm,
                );
                match under {
                    Some(item) => {
                        if additive {
                            if !self.selection.remove(&item) {
                                self.selection.insert(item);
                            }
                        } else if !self.selection.contains(&item) {
                            self.selection.clear();
                            self.selection.insert(item);
                        }
                        if self.selection.contains(&item) {
                            let from = self.snap(at);
                            self.drag = Some(Drag {
                                items: self.selection.iter().copied().collect(),
                                from,
                                to: from,
                            });
                        }
                    }
                    None => {
                        if !additive {
                            self.selection.clear();
                        }
                        self.marquee = Some((at, at));
                    }
                }
            }
            Tool::Wire => self.wire_click(sheet, at),
            Tool::Label => {
                let op = EditOp::AddLabel {
                    sheet,
                    at: self.snap(at),
                    name: self.pending_name.clone(),
                };
                self.put(&op);
            }
            Tool::PowerPort => {
                let op = EditOp::AddPowerPort {
                    sheet,
                    at: self.snap(at),
                    name: self.pending_name.clone(),
                };
                self.put(&op);
            }
            Tool::Bus => {
                let op = EditOp::AddBus {
                    sheet,
                    at: self.snap(at),
                    spec: self.pending_name.clone(),
                    dx: 0,
                    dy: self.settings.bus_step_nm,
                };
                self.put(&op);
            }
            Tool::Place => {
                if let Some(op) = self.place_op(sheet, self.snap(at)) {
                    self.put(&op);
                }
            }
        }
    }

    /// Runs an operation and selects what it made.
    fn put(&mut self, op: &EditOp) {
        if let Ok(applied) = self.run(op) {
            let made = items_of(self.editor.schematic(), &applied.created);
            if !made.is_empty() {
                self.selection = made;
            }
        }
    }

    /// The pointer moved to `at`.
    pub fn pointer_moved(&mut self, at: Point) {
        let snapped = self.snap(at);
        if let Some(drag) = self.drag.as_mut() {
            drag.to = snapped;
        }
        if let Some(marquee) = self.marquee.as_mut() {
            marquee.1 = at;
        }
    }

    /// The pointer was let go at `at`: a drag moves what it carries (the wires that end on the pins
    /// of moved parts follow them), a rectangle selects what it takes in.
    pub fn pointer_released(&mut self, at: Point) {
        self.pointer_moved(at);
        if let Some(drag) = self.drag.take() {
            let (dx, dy) = drag.offset();
            if dx != 0 || dy != 0 {
                let op = EditOp::MoveItems {
                    items: drag.items,
                    dx,
                    dy,
                    detach: false,
                };
                let _ = self.run(&op);
            }
        }
        if let Some((a, b)) = self.marquee.take() {
            if a != b {
                if let Some(sheet) = self.sheet() {
                    let found = in_rect(self.editor.schematic(), sheet, a, b);
                    self.selection.extend(found);
                }
            }
        }
    }

    /// The schematic as it would be if the drag in progress were let go now.
    pub fn drag_preview(&self) -> Option<Schematic> {
        let drag = self.drag.as_ref()?;
        let (dx, dy) = drag.offset();
        if dx == 0 && dy == 0 {
            return None;
        }
        let op = EditOp::MoveItems {
            items: drag.items.clone(),
            dx,
            dy,
            detach: false,
        };
        self.editor.preview(&op).ok()
    }

    fn wire_click(&mut self, sheet: Id, at: Point) {
        let target = self.snap(at);
        match self.wire_start {
            None => self.wire_start = Some(target),
            Some(start) => {
                if start == target {
                    self.wire_start = None;
                    return;
                }
                let path = orthogonal_path(start, target, self.horizontal_first);
                if self.run(&EditOp::AddWire { sheet, path }).is_ok() {
                    self.wire_start = if self.is_joined(sheet, target) {
                        None
                    } else {
                        Some(target)
                    };
                }
            }
        }
    }

    /// True when the point is a pin, a junction, a label or a power port, or a point of more than
    /// one wire: a wire that ends there is finished.
    fn is_joined(&self, sheet: Id, p: Point) -> bool {
        let s = self.editor.schematic();
        let pin = s
            .pin_positions()
            .iter()
            .any(|(_, pin_sheet, at)| *pin_sheet == sheet && *at == p);
        let junction = s.junctions.values().any(|j| j.sheet == sheet && j.at == p);
        let named = s.labels.values().any(|l| l.sheet == sheet && l.at == p)
            || s.power_ports
                .values()
                .any(|l| l.sheet == sheet && l.at == p);
        let wire_points = s
            .wires
            .values()
            .filter(|w| w.sheet == sheet)
            .flat_map(|w| w.path.iter())
            .filter(|q| **q == p)
            .count();
        pin || junction || named || wire_points >= 2
    }

    /// The path the wire in progress would take to the pointer at `at`.
    pub fn wire_preview(&self, at: Point) -> Vec<Point> {
        match self.wire_start {
            Some(start) => orthogonal_path(start, self.snap(at), self.horizontal_first),
            None => Vec::new(),
        }
    }

    /// The operation that places the chosen symbol of the palette at a point. When the schematic
    /// already has a symbol of that name with other pins, the new one is named `library:name`.
    pub fn place_op(&self, sheet: Id, at: Point) -> Option<EditOp> {
        let name = self.place_name.as_ref()?;
        let entry = self.palette.find(name)?;
        let (symbol, define) = self.symbol_spec(entry);
        Some(EditOp::PlaceSymbol {
            symbol,
            define: Some(define),
            sheet,
            at,
            rotation: None,
            mirrored: false,
            refdes: None,
            value: None,
        })
    }

    fn symbol_spec(&self, entry: &PaletteEntry) -> (String, SymbolDef) {
        let s = self.editor.schematic();
        let mut name_taken = false;
        for (id, symbol) in &s.symbols {
            if symbol.name == entry.def.name {
                if same_symbol(s, symbol, &entry.def) {
                    return (id.to_string(), entry.def.clone());
                }
                name_taken = true;
            }
        }
        if name_taken {
            let mut def = entry.def.clone();
            def.name = format!("{}:{}", entry.library, entry.def.name);
            (def.name.clone(), def)
        } else {
            (entry.def.name.clone(), entry.def.clone())
        }
    }

    /// The selected parts.
    pub fn selected_parts(&self) -> Vec<Id> {
        self.selection
            .iter()
            .filter_map(|item| match item {
                Item::Part(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Turns the selected parts by quarter turns.
    pub fn rotate_selection(&mut self, quarter_turns: i32) -> Result<Applied, EditError> {
        let parts = self.selected_parts();
        if parts.is_empty() {
            return Err(self.refuse(EditError::BadSelection("no part is selected")));
        }
        self.run(&EditOp::RotateParts {
            parts,
            quarter_turns,
        })
    }

    /// Mirrors the selected parts.
    pub fn mirror_selection(&mut self) -> Result<Applied, EditError> {
        let parts = self.selected_parts();
        if parts.is_empty() {
            return Err(self.refuse(EditError::BadSelection("no part is selected")));
        }
        self.run(&EditOp::MirrorParts { parts })
    }

    /// Deletes the selection.
    pub fn delete_selection(&mut self) -> Result<Applied, EditError> {
        if self.selection.is_empty() {
            return Err(self.refuse(EditError::BadSelection("nothing is selected")));
        }
        let items: Vec<Item> = self.selection.iter().copied().collect();
        let result = self.run(&EditOp::Delete { items });
        if result.is_ok() {
            self.selection.clear();
        }
        result
    }

    /// Copies the selection; false when there was nothing to copy.
    pub fn copy_selection(&mut self) -> bool {
        let items: Vec<Item> = self.selection.iter().copied().collect();
        let clip = Clip::copy(self.editor.schematic(), &items);
        if clip.is_empty() {
            return false;
        }
        self.clip = Some(clip);
        true
    }

    /// Copies the selection and deletes it.
    pub fn cut_selection(&mut self) -> Result<Applied, EditError> {
        if !self.copy_selection() {
            return Err(self.refuse(EditError::BadSelection("nothing is selected")));
        }
        self.delete_selection()
    }

    /// Pastes the clip with its corner on `at` (on the grid), or moved by the offset of the
    /// settings when there is no target. What is pasted becomes the selection.
    pub fn paste(&mut self, at: Option<Point>) -> Result<Applied, EditError> {
        let Some(clip) = self.clip.clone() else {
            return Err(self.refuse(EditError::BadSelection("nothing was copied")));
        };
        let sheet = self.ensure_sheet()?;
        let (dx, dy) = match (at, clip.corner()) {
            (Some(target), Some(corner)) => {
                let target = self.snap(target);
                (
                    target.x_nm.get() - corner.x_nm.get(),
                    target.y_nm.get() - corner.y_nm.get(),
                )
            }
            _ => (self.settings.paste_offset_nm, self.settings.paste_offset_nm),
        };
        let applied = self.run(&EditOp::Paste {
            clip,
            sheet,
            dx,
            dy,
        })?;
        let made = items_of(self.editor.schematic(), &applied.created);
        if !made.is_empty() {
            self.selection = made;
        }
        Ok(applied)
    }

    /// Sets the value of the selected parts.
    pub fn set_value(&mut self, value: &str) -> Result<Applied, EditError> {
        let parts = self.selected_parts();
        if parts.is_empty() {
            return Err(self.refuse(EditError::BadSelection("no part is selected")));
        }
        let ops = parts
            .into_iter()
            .map(|part| EditOp::SetValue {
                part,
                value: value.to_owned(),
            })
            .collect();
        self.run(&EditOp::Batch { ops })
    }

    /// Sets the reference of the selected part (exactly one must be selected).
    pub fn set_refdes(&mut self, refdes: &str) -> Result<Applied, EditError> {
        let parts = self.selected_parts();
        match parts.as_slice() {
            [part] => self.run(&EditOp::SetRefdes {
                part: *part,
                refdes: refdes.to_owned(),
            }),
            _ => Err(self.refuse(EditError::BadSelection("exactly one part must be selected"))),
        }
    }

    fn refuse(&mut self, error: EditError) -> EditError {
        self.error = Some(error.clone());
        error
    }

    /// The dry run of the transfer to a board: the package is kept as the preview.
    pub fn plan_transfer(&mut self, board: &Board, settings: &SyncSettings) {
        self.preview = Some(plan_transfer(self.editor.schematic(), board, settings));
    }

    /// The report of the package that is ready, as text in a language.
    pub fn transfer_text(&self, lang: Lang) -> Option<String> {
        self.preview.as_ref().map(|p| p.report().to_text(lang))
    }

    /// True when a package is ready and it holds changes.
    pub fn transfer_has_changes(&self) -> bool {
        self.preview.as_ref().is_some_and(|p| !p.is_empty())
    }

    /// Applies the package that is ready to the board, as one step of its history. Nothing happens
    /// when no package is ready.
    pub fn apply_transfer(
        &mut self,
        board: &mut Board,
        history: &mut History,
    ) -> Result<(), OpError> {
        let Some(package) = self.preview.take() else {
            return Ok(());
        };
        package.apply(board, history)
    }

    /// The net list of the schematic for the simulator.
    pub fn netlist(&self, settings: &SimSettings) -> NetlistExport {
        export_netlist(self.editor.schematic(), settings)
    }
}
