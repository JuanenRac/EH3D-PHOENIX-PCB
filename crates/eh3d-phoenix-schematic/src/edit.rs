//! Editing the drawn schematic with operations that can be undone.
//!
//! * an [`EditOp`] is one change the person (or a file of operations) asks for: place a symbol, move,
//!   turn or mirror parts, delete, copy and paste, set a value or a reference, draw an orthogonal
//!   wire, put a net label, a power port or the labels of a bus;
//! * the [`Editor`] applies an operation **all or nothing**: it works on a copy, and the schematic
//!   changes only when the operation, the tidying of the drawing and the new net list all worked;
//! * after an operation that touches the wiring the drawing is tidied (a wire that ends in the middle
//!   of another wire, or a pin or a label in the middle of a wire, splits that wire so they are
//!   joined; the junction dots are put where three or more pieces meet) and the nets are made again
//!   from what the drawing joins ([`crate::nets`]);
//! * the steps are kept as differences by map, so undo and redo put back exactly what was there;
//! * every new id comes from an [`IdSource`], which only depends on the ids the schematic already has
//!   and on the number of ids asked for, so the same schematic and the same operations always give
//!   the same bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use eh3d_phoenix_model::{Nm, Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::marks::MAX_NAME_CHARS;
use crate::model::{Id, PinKind, Schematic};

/// An operation that was refused; the schematic is as it was.
pub const REFUSED: &str = "PHOENIX-SCH-010";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[(REFUSED, "Edit refused")];

/// The most operations of one batch, and the most steps kept for undo.
pub const MAX_BATCH: usize = 10_000;
/// How deep batches may be nested.
pub(crate) const MAX_DEPTH: usize = 8;
/// The most steps the history keeps by default.
pub const DEFAULT_HISTORY: usize = 1000;

/// The time stamp of the ids that the editor makes.
const STAMP_MS: u64 = 1_800_000_000_000;

/// Why an operation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// A part that the schematic does not have.
    NoSuchPart(Id),
    /// A sheet that the schematic does not have.
    NoSuchSheet(Id),
    /// A symbol that is neither in the schematic nor defined by the operation.
    NoSuchSymbol(String),
    /// A wire, label, power port, junction or part that the schematic does not have.
    NoSuchItem(Id),
    /// A reference designator that another part has.
    DuplicateRefdes(String),
    /// A name, reference or value that is empty, too long or has control characters.
    BadText(&'static str),
    /// A wire that is not a path of axis-parallel pieces of two or more different points.
    BadWire(&'static str),
    /// A text that is not the name of a bus such as `D[0..7]`.
    BadBus(String),
    /// A symbol definition that cannot be used.
    BadSymbol(&'static str),
    /// A coordinate or an angle left its range.
    OutOfRange,
    /// Something that was needed is not there (nothing selected, nothing copied).
    BadSelection(&'static str),
    /// More operations or nesting than allowed.
    TooMany,
    /// The supply of ids failed.
    Ids(String),
}

impl EditError {
    /// The stable code of the error.
    pub fn code(&self) -> &'static str {
        REFUSED
    }
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{REFUSED} ")?;
        match self {
            EditError::NoSuchPart(id) => write!(f, "there is no part {id}"),
            EditError::NoSuchSheet(id) => write!(f, "there is no sheet {id}"),
            EditError::NoSuchSymbol(name) => write!(f, "there is no symbol {name}"),
            EditError::NoSuchItem(id) => write!(f, "there is nothing with the id {id}"),
            EditError::DuplicateRefdes(r) => write!(f, "the reference {r} is already used"),
            EditError::BadText(what) => {
                write!(f, "{what} is empty, too long or has control characters")
            }
            EditError::BadWire(why) => write!(f, "wire refused: {why}"),
            EditError::BadBus(text) => write!(f, "{text} is not a bus such as D[0..7]"),
            EditError::BadSymbol(why) => write!(f, "symbol refused: {why}"),
            EditError::OutOfRange => write!(f, "a coordinate or an angle is out of range"),
            EditError::BadSelection(why) => write!(f, "nothing to do: {why}"),
            EditError::TooMany => write!(f, "too many operations"),
            EditError::Ids(why) => write!(f, "no id could be made: {why}"),
        }
    }
}

impl std::error::Error for EditError {}

/// The supply of new ids: it knows every id the schematic uses and hands out ids that are not among
/// them, always in the same order.
#[derive(Debug, Clone)]
pub struct IdSource {
    taken: BTreeSet<Id>,
    next: u64,
}

impl IdSource {
    /// A supply that avoids every id of `s`.
    pub fn for_schematic(s: &Schematic) -> IdSource {
        let mut taken: BTreeSet<Id> = BTreeSet::new();
        taken.extend(s.symbols.keys());
        for symbol in s.symbols.values() {
            taken.extend(symbol.pins.keys());
        }
        taken.extend(s.parts.keys());
        taken.extend(s.nets.keys());
        taken.extend(s.sheets.keys());
        taken.extend(s.wires.keys());
        taken.extend(s.labels.keys());
        taken.extend(s.junctions.keys());
        taken.extend(s.power_ports.keys());
        taken.extend(s.sheet_blocks.keys());
        taken.extend(s.hier_pins.keys());
        taken.extend(s.sheet_ports.keys());
        taken.extend(s.buses.keys());
        taken.extend(s.bus_entries.keys());
        taken.extend(s.bus_aliases.keys());
        IdSource { taken, next: 0 }
    }

    /// The next id that is not taken.
    pub fn fresh(&mut self) -> Result<Id, EditError> {
        loop {
            self.next += 1;
            let counter = (self.next & 0x0fff) as u16;
            let id = Id::from_parts(STAMP_MS, counter, self.next)
                .map_err(|e| EditError::Ids(e.to_string()))?;
            if self.taken.insert(id) {
                return Ok(id);
            }
        }
    }

    /// Marks ids as used, for example those of a board that the schematic is linked to.
    pub fn avoid(&mut self, ids: impl IntoIterator<Item = Id>) {
        self.taken.extend(ids);
    }
}

/// A pin of a symbol that an operation defines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinDef {
    /// The number or name printed on the symbol.
    pub number: String,
    /// Where the pin is, relative to the origin of the symbol.
    pub at: Point,
    /// The electrical type, when it is known.
    #[serde(default)]
    pub kind: Option<PinKind>,
}

/// A symbol that an operation defines when the schematic does not have it yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolDef {
    /// The name.
    pub name: String,
    /// The pins.
    pub pins: Vec<PinDef>,
}

/// Something of the drawing that an operation acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    /// A placed part.
    Part(Id),
    /// A wire.
    Wire(Id),
    /// A net label.
    Label(Id),
    /// A power port.
    PowerPort(Id),
    /// A junction dot.
    Junction(Id),
}

/// A name placed at a point (a net label or a power port) in a clip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipMark {
    /// Where it was.
    pub at: Point,
    /// The name.
    pub name: String,
}

/// A part in a clip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipPart {
    /// The reference it had.
    pub refdes: String,
    /// The value.
    pub value: String,
    /// The symbol, by id.
    pub symbol: Id,
    /// The footprint it had.
    #[serde(default)]
    pub footprint: Option<Id>,
    /// Where it was.
    pub at: Point,
    /// How it was turned.
    pub rotation: Udeg,
    /// Whether it was mirrored.
    #[serde(default)]
    pub mirrored: bool,
}

/// What was copied: the parts, wires, labels and power ports, with the positions they had.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    /// The parts.
    #[serde(default)]
    pub parts: Vec<ClipPart>,
    /// The wires, as paths.
    #[serde(default)]
    pub wires: Vec<Vec<Point>>,
    /// The net labels.
    #[serde(default)]
    pub labels: Vec<ClipMark>,
    /// The power ports.
    #[serde(default)]
    pub power_ports: Vec<ClipMark>,
}

impl Clip {
    /// Copies the given items; things that are not there are left out, and so are junction dots
    /// (the pasted drawing makes its own).
    pub fn copy(s: &Schematic, items: &[Item]) -> Clip {
        let mut clip = Clip::default();
        let wanted: BTreeSet<Item> = items.iter().copied().collect();
        for item in wanted {
            match item {
                Item::Part(id) => {
                    if let Some(part) = s.parts.get(&id) {
                        if let Some(place) = part.placement {
                            clip.parts.push(ClipPart {
                                refdes: part.refdes.clone(),
                                value: part.value.clone(),
                                symbol: part.symbol,
                                footprint: part.footprint,
                                at: place.at,
                                rotation: place.rotation,
                                mirrored: s.mirrored.contains(&id),
                            });
                        }
                    }
                }
                Item::Wire(id) => {
                    if let Some(wire) = s.wires.get(&id) {
                        clip.wires.push(wire.path.clone());
                    }
                }
                Item::Label(id) => {
                    if let Some(label) = s.labels.get(&id) {
                        clip.labels.push(ClipMark {
                            at: label.at,
                            name: label.name.clone(),
                        });
                    }
                }
                Item::PowerPort(id) => {
                    if let Some(port) = s.power_ports.get(&id) {
                        clip.power_ports.push(ClipMark {
                            at: port.at,
                            name: port.name.clone(),
                        });
                    }
                }
                Item::Junction(_) => {}
            }
        }
        clip
    }

    /// True when nothing was copied.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
            && self.wires.is_empty()
            && self.labels.is_empty()
            && self.power_ports.is_empty()
    }

    /// The corner with the smallest coordinates of everything in the clip, when there is anything.
    pub fn corner(&self) -> Option<Point> {
        let mut points: Vec<Point> = Vec::new();
        points.extend(self.parts.iter().map(|p| p.at));
        points.extend(self.wires.iter().flatten().copied());
        points.extend(self.labels.iter().map(|m| m.at));
        points.extend(self.power_ports.iter().map(|m| m.at));
        let x = points.iter().map(|p| p.x_nm.get()).min()?;
        let y = points.iter().map(|p| p.y_nm.get()).min()?;
        Point::from_nm(x, y).ok()
    }
}

/// One operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditOp {
    /// Adds an empty sheet.
    AddSheet {
        /// The name.
        name: String,
        /// The width of the drawing area.
        width: Nm,
        /// The height of the drawing area.
        height: Nm,
    },
    /// Places a symbol as a new part. `symbol` is the id or the name of a symbol of the schematic;
    /// when there is none and `define` is given, the symbol is defined first.
    PlaceSymbol {
        /// The id or the name of the symbol.
        symbol: String,
        /// The definition to use when the schematic does not have the symbol.
        #[serde(default)]
        define: Option<SymbolDef>,
        /// The sheet.
        sheet: Id,
        /// The origin of the symbol.
        at: Point,
        /// The rotation, a multiple of 90 degrees (zero when not given).
        #[serde(default)]
        rotation: Option<Udeg>,
        /// True to place it mirrored.
        #[serde(default)]
        mirrored: bool,
        /// The reference; when not given, the next free one of the letter of the symbol.
        #[serde(default)]
        refdes: Option<String>,
        /// The value (empty when not given).
        #[serde(default)]
        value: Option<String>,
    },
    /// Moves items by an offset. Wires that end on a pin of a moved part follow the pin (and bend
    /// to stay orthogonal) unless `detach` is true.
    MoveItems {
        /// What to move.
        items: Vec<Item>,
        /// The offset along x, in nanometres.
        dx: i64,
        /// The offset along y, in nanometres.
        dy: i64,
        /// True to leave the wires where they are.
        #[serde(default)]
        detach: bool,
    },
    /// Turns parts about their own origin by quarter turns (positive: the direction of growing
    /// angles).
    RotateParts {
        /// The parts.
        parts: Vec<Id>,
        /// How many quarter turns; may be negative.
        quarter_turns: i32,
    },
    /// Mirrors parts left to right.
    MirrorParts {
        /// The parts.
        parts: Vec<Id>,
    },
    /// Deletes items.
    Delete {
        /// What to delete.
        items: Vec<Item>,
    },
    /// Pastes a clip, moved by an offset, as new items; the references are made unique.
    Paste {
        /// What was copied.
        clip: Clip,
        /// The sheet to paste on.
        sheet: Id,
        /// The offset along x, in nanometres.
        dx: i64,
        /// The offset along y, in nanometres.
        dy: i64,
    },
    /// Sets the value of a part.
    SetValue {
        /// The part.
        part: Id,
        /// The value.
        value: String,
    },
    /// Sets the reference of a part.
    SetRefdes {
        /// The part.
        part: Id,
        /// The reference.
        refdes: String,
    },
    /// Draws a wire: a path of pieces that are horizontal or vertical. A wire that ends on the
    /// middle of another wire joins it (a T).
    AddWire {
        /// The sheet.
        sheet: Id,
        /// The points.
        path: Vec<Point>,
    },
    /// Puts a net label.
    AddLabel {
        /// The sheet.
        sheet: Id,
        /// Where.
        at: Point,
        /// The name of the net.
        name: String,
    },
    /// Puts a power port.
    AddPowerPort {
        /// The sheet.
        sheet: Id,
        /// Where.
        at: Point,
        /// The name of the supply net.
        name: String,
    },
    /// Puts the labels of a bus such as `D[0..7]`: one label per member, the first at `at`, each
    /// next one an offset further (2.54 mm down when both offsets are zero).
    AddBus {
        /// The sheet.
        sheet: Id,
        /// Where the first label goes.
        at: Point,
        /// The bus, for example `D[0..7]`.
        spec: String,
        /// The offset along x between two labels, in nanometres.
        #[serde(default)]
        dx: i64,
        /// The offset along y between two labels, in nanometres.
        #[serde(default)]
        dy: i64,
    },
    /// Renames a net label or a power port.
    RenameMark {
        /// The label or power port.
        id: Id,
        /// The new name.
        name: String,
    },
    /// Removes a sheet with everything drawn on it (parts, wires, labels, power ports, junctions,
    /// the sheet symbols on it with their pins, its ports and its buses). Refused while a sheet
    /// symbol shows the sheet.
    RemoveSheet {
        /// The sheet.
        sheet: Id,
    },
    /// Gives a sheet another name.
    RenameSheet {
        /// The sheet.
        sheet: Id,
        /// The new name.
        name: String,
    },
    /// Draws a sheet symbol on `sheet` that shows the sheet `child`. The same `child` may be shown
    /// by several symbols with different names: each one is an instance.
    AddSheetBlock {
        /// The sheet the symbol is drawn on.
        sheet: Id,
        /// The sheet it shows.
        child: Id,
        /// The name of the instance.
        name: String,
        /// The corner with the smallest coordinates.
        at: Point,
        /// The width of the block.
        width: Nm,
        /// The height of the block.
        height: Nm,
    },
    /// Removes a sheet symbol and its pins.
    RemoveSheetBlock {
        /// The sheet symbol.
        block: Id,
    },
    /// Gives a sheet symbol another name.
    RenameSheetBlock {
        /// The sheet symbol.
        block: Id,
        /// The new name.
        name: String,
    },
    /// Puts a pin on a sheet symbol.
    AddHierPin {
        /// The sheet symbol.
        block: Id,
        /// The name, which is the name of the port it joins.
        name: String,
        /// The direction.
        dir: crate::hier::PortDir,
        /// Where it is, on the sheet of the symbol.
        at: Point,
    },
    /// Removes the pin of a sheet symbol.
    RemoveHierPin {
        /// The pin.
        pin: Id,
    },
    /// Puts a port on a sheet.
    AddSheetPort {
        /// The sheet.
        sheet: Id,
        /// The name, which is the name of the pin it joins.
        name: String,
        /// The direction.
        dir: crate::hier::PortDir,
        /// Where it is.
        at: Point,
    },
    /// Removes the port of a sheet.
    RemoveSheetPort {
        /// The port.
        port: Id,
    },
    /// Moves items to another sheet, shifted by an offset; the nets are made again.
    MoveToSheet {
        /// What to move.
        items: Vec<Item>,
        /// The sheet they go to.
        sheet: Id,
        /// The offset along x, in nanometres.
        dx: i64,
        /// The offset along y, in nanometres.
        dy: i64,
    },
    /// Places a bus on a sheet: `D[0..7]`, `USB{DP DM}` or the name of an alias.
    PlaceBus {
        /// The sheet.
        sheet: Id,
        /// The anchor.
        at: Point,
        /// The name of the bus.
        spec: String,
    },
    /// Removes a bus and its entries.
    RemoveBus {
        /// The bus.
        bus: Id,
    },
    /// Takes a member out of a bus at a point: the net there takes the name of the member.
    AddBusEntry {
        /// The bus.
        bus: Id,
        /// The member (`D3`, `USB.DP`).
        member: String,
        /// Where the member leaves the bus.
        at: Point,
    },
    /// Removes a bus entry.
    RemoveBusEntry {
        /// The entry.
        entry: Id,
    },
    /// Gives a name to a group of bus members, or replaces the group of that name.
    SetBusAlias {
        /// The name of the alias.
        name: String,
        /// The items: plain names or vectors.
        members: Vec<String>,
    },
    /// Removes an alias.
    RemoveBusAlias {
        /// The name of the alias.
        name: String,
    },
    /// Several operations that are one step: all or none.
    Batch {
        /// The operations, in order.
        ops: Vec<EditOp>,
    },
}

/// What an applied operation made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Applied {
    /// The ids of the new symbols, parts, wires, labels, power ports and sheets, in order.
    pub created: Vec<Id>,
}

type Delta<T> = Vec<(Id, Option<T>, Option<T>)>;

fn delta<T: Clone + PartialEq>(a: &BTreeMap<Id, T>, b: &BTreeMap<Id, T>) -> Delta<T> {
    let mut out = Vec::new();
    for (id, x) in a {
        match b.get(id) {
            Some(y) if y == x => {}
            other => out.push((*id, Some(x.clone()), other.cloned())),
        }
    }
    for (id, y) in b {
        if !a.contains_key(id) {
            out.push((*id, None, Some(y.clone())));
        }
    }
    out
}

fn put<T: Clone>(map: &mut BTreeMap<Id, T>, d: &Delta<T>, forward: bool) {
    for (id, before, after) in d {
        let target = if forward { after } else { before };
        match target {
            Some(v) => {
                map.insert(*id, v.clone());
            }
            None => {
                map.remove(id);
            }
        }
    }
}

/// The differences between two states of a schematic, by map.
#[derive(Debug, Clone, Default)]
struct Change {
    symbols: Delta<crate::model::Symbol>,
    parts: Delta<crate::model::Part>,
    nets: Delta<crate::model::SchematicNet>,
    sheets: Delta<crate::draw::Sheet>,
    wires: Delta<crate::draw::Wire>,
    pin_kinds: Delta<PinKind>,
    labels: Delta<crate::marks::NetLabel>,
    junctions: Delta<crate::marks::Junction>,
    power_ports: Delta<crate::marks::PowerPort>,
    sheet_blocks: Delta<crate::hier::SheetBlock>,
    hier_pins: Delta<crate::hier::HierPin>,
    sheet_ports: Delta<crate::hier::SheetPort>,
    buses: Delta<crate::bus::Bus>,
    bus_entries: Delta<crate::bus::BusEntry>,
    bus_aliases: Delta<crate::bus::BusAlias>,
    mirrored: Vec<(Id, bool)>,
}

impl Change {
    fn between(a: &Schematic, b: &Schematic) -> Change {
        Change {
            symbols: delta(&a.symbols, &b.symbols),
            parts: delta(&a.parts, &b.parts),
            nets: delta(&a.nets, &b.nets),
            sheets: delta(&a.sheets, &b.sheets),
            wires: delta(&a.wires, &b.wires),
            pin_kinds: delta(&a.pin_kinds, &b.pin_kinds),
            labels: delta(&a.labels, &b.labels),
            junctions: delta(&a.junctions, &b.junctions),
            power_ports: delta(&a.power_ports, &b.power_ports),
            sheet_blocks: delta(&a.sheet_blocks, &b.sheet_blocks),
            hier_pins: delta(&a.hier_pins, &b.hier_pins),
            sheet_ports: delta(&a.sheet_ports, &b.sheet_ports),
            buses: delta(&a.buses, &b.buses),
            bus_entries: delta(&a.bus_entries, &b.bus_entries),
            bus_aliases: delta(&a.bus_aliases, &b.bus_aliases),
            mirrored: a
                .mirrored
                .symmetric_difference(&b.mirrored)
                .map(|id| (*id, b.mirrored.contains(id)))
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.symbols.is_empty()
            && self.parts.is_empty()
            && self.nets.is_empty()
            && self.sheets.is_empty()
            && self.wires.is_empty()
            && self.pin_kinds.is_empty()
            && self.labels.is_empty()
            && self.junctions.is_empty()
            && self.power_ports.is_empty()
            && self.sheet_blocks.is_empty()
            && self.hier_pins.is_empty()
            && self.sheet_ports.is_empty()
            && self.buses.is_empty()
            && self.bus_entries.is_empty()
            && self.bus_aliases.is_empty()
            && self.mirrored.is_empty()
    }

    /// Puts the schematic in the state after (`forward`) or before the change.
    fn apply(&self, s: &mut Schematic, forward: bool) {
        put(&mut s.symbols, &self.symbols, forward);
        put(&mut s.parts, &self.parts, forward);
        put(&mut s.nets, &self.nets, forward);
        put(&mut s.sheets, &self.sheets, forward);
        put(&mut s.wires, &self.wires, forward);
        put(&mut s.pin_kinds, &self.pin_kinds, forward);
        put(&mut s.labels, &self.labels, forward);
        put(&mut s.junctions, &self.junctions, forward);
        put(&mut s.power_ports, &self.power_ports, forward);
        put(&mut s.sheet_blocks, &self.sheet_blocks, forward);
        put(&mut s.hier_pins, &self.hier_pins, forward);
        put(&mut s.sheet_ports, &self.sheet_ports, forward);
        put(&mut s.buses, &self.buses, forward);
        put(&mut s.bus_entries, &self.bus_entries, forward);
        put(&mut s.bus_aliases, &self.bus_aliases, forward);
        for (id, now) in &self.mirrored {
            // `now` is the state after the change
            if *now == forward {
                s.mirrored.insert(*id);
            } else {
                s.mirrored.remove(id);
            }
        }
    }

    /// The ids that the change added to the maps a person can point at.
    fn created(&self) -> Vec<Id> {
        let mut ids: Vec<Id> = Vec::new();
        ids.extend(added(&self.sheets));
        ids.extend(added(&self.symbols));
        ids.extend(added(&self.parts));
        ids.extend(added(&self.wires));
        ids.extend(added(&self.labels));
        ids.extend(added(&self.power_ports));
        ids.extend(added(&self.sheet_blocks));
        ids.extend(added(&self.hier_pins));
        ids.extend(added(&self.sheet_ports));
        ids.extend(added(&self.buses));
        ids.extend(added(&self.bus_entries));
        ids.extend(added(&self.bus_aliases));
        ids
    }
}

fn added<T>(d: &Delta<T>) -> Vec<Id> {
    d.iter()
        .filter(|(_, before, after)| before.is_none() && after.is_some())
        .map(|(id, _, _)| *id)
        .collect()
}

/// A schematic being edited, with the steps that can be undone and redone.
#[derive(Debug, Clone)]
pub struct Editor {
    schematic: Schematic,
    ids: IdSource,
    past: Vec<Change>,
    future: Vec<Change>,
    limit: usize,
}

impl Editor {
    /// An editor on a schematic, with the default depth of history.
    pub fn new(schematic: Schematic) -> Editor {
        Editor::with_limit(schematic, DEFAULT_HISTORY)
    }

    /// An editor that keeps at most `limit` steps (at least one).
    pub fn with_limit(schematic: Schematic, limit: usize) -> Editor {
        let ids = IdSource::for_schematic(&schematic);
        Editor {
            schematic,
            ids,
            past: Vec::new(),
            future: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// The schematic as it is now.
    pub fn schematic(&self) -> &Schematic {
        &self.schematic
    }

    /// Gives the schematic back, without the history.
    pub fn into_schematic(self) -> Schematic {
        self.schematic
    }

    /// Marks ids (for example of the board) that new ids must avoid.
    pub fn avoid_ids(&mut self, ids: impl IntoIterator<Item = Id>) {
        self.ids.avoid(ids);
    }

    /// True when there is a step to undo.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// True when there is a step to redo.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// How many steps can be undone.
    pub fn undo_depth(&self) -> usize {
        self.past.len()
    }

    /// Applies an operation all or nothing. A refused operation leaves the schematic, the history
    /// and the supply of ids as they were.
    pub fn apply(&mut self, op: &EditOp) -> Result<Applied, EditError> {
        let (next, ids) = self.simulate(op)?;
        let change = Change::between(&self.schematic, &next);
        let applied = Applied {
            created: change.created(),
        };
        self.ids = ids;
        if change.is_empty() {
            return Ok(applied);
        }
        self.schematic = next;
        self.future.clear();
        self.past.push(change);
        if self.past.len() > self.limit {
            let extra = self.past.len() - self.limit;
            self.past.drain(..extra);
        }
        Ok(applied)
    }

    /// The schematic that the operation would give, without changing anything.
    pub fn preview(&self, op: &EditOp) -> Result<Schematic, EditError> {
        self.simulate(op).map(|(s, _)| s)
    }

    fn simulate(&self, op: &EditOp) -> Result<(Schematic, IdSource), EditError> {
        let mut work = self.schematic.clone();
        let mut ids = self.ids.clone();
        let before = self.schematic.drawn().groups;
        let mut wiring = false;
        crate::edit_ops::run(&mut work, &mut ids, op, &mut wiring, 0)?;
        if wiring {
            crate::edit_ops::tidy(&mut work, &mut ids)?;
            crate::nets::rebuild_nets(&mut work, &mut ids, &before)?;
        }
        Ok((work, ids))
    }

    /// Undoes the last step; false when there is none.
    pub fn undo(&mut self) -> bool {
        match self.past.pop() {
            Some(change) => {
                change.apply(&mut self.schematic, false);
                self.future.push(change);
                true
            }
            None => false,
        }
    }

    /// Redoes the step that was undone last; false when there is none.
    pub fn redo(&mut self) -> bool {
        match self.future.pop() {
            Some(change) => {
                change.apply(&mut self.schematic, true);
                self.past.push(change);
                true
            }
            None => false,
        }
    }
}

/// Checks a name, reference or value: at most [`MAX_NAME_CHARS`] characters, no control characters,
/// and not empty unless `allow_empty`. Returns the text without the spaces around it.
pub(crate) fn clean_text(
    text: &str,
    allow_empty: bool,
    what: &'static str,
) -> Result<String, EditError> {
    let t = text.trim();
    if (t.is_empty() && !allow_empty)
        || t.chars().count() > MAX_NAME_CHARS
        || t.chars().any(char::is_control)
    {
        return Err(EditError::BadText(what));
    }
    Ok(t.to_owned())
}

/// The path of a wire from one point to another with horizontal and vertical pieces: one piece when
/// the points are on a line, otherwise two with the corner at the end of the horizontal piece when
/// `horizontal_first`, or of the vertical one. Empty when the points are the same.
pub fn orthogonal_path(from: Point, to: Point, horizontal_first: bool) -> Vec<Point> {
    if from == to {
        return Vec::new();
    }
    if from.x_nm == to.x_nm || from.y_nm == to.y_nm {
        return vec![from, to];
    }
    let corner = if horizontal_first {
        Point {
            x_nm: to.x_nm,
            y_nm: from.y_nm,
        }
    } else {
        Point {
            x_nm: from.x_nm,
            y_nm: to.y_nm,
        }
    };
    vec![from, corner, to]
}
