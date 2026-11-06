// SPDX-License-Identifier: GPL-3.0-or-later
//! The document a script of the second version works on: a copy of a board, a schematic or a
//! library, the operations made so far and the questions a script can ask about it.
//!
//! Every change goes through the reversible operations of the model (the board's `Operation`, the
//! schematic's `EditOp`, the library's `Operation`), one at a time on the copy, so an operation
//! that the model refuses stops the script and nothing of it is kept. The operations that were
//! made are kept in order, and they make one batch: the whole script is one step of the history.

use eh3d_phoenix_board::{Board, Id, Operation, Segment};
use eh3d_phoenix_libedit::{Library, Operation as LibOp};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_schematic::{EditOp as SchOp, Editor, Schematic};

use crate::codes::Diag;
use crate::exec::{used_ids, Ids};
use crate::settings::ScriptSettings;
use crate::v2::codes::{NOT_FOUND, REFUSED, TARGET, TYPE_MISMATCH};
use crate::v2::value::{glob, isqrt, Obj, ObjKind, Value};

/// The documents a function can apply to, as bits.
pub const BOARD: u8 = 1;
/// The schematic bit.
pub const SCHEMATIC: u8 = 2;
/// The library bit.
pub const LIBRARY: u8 = 4;

/// The document of a run.
pub enum World {
    /// No document: only the functions that do not need one can be called.
    None,
    /// A board.
    Board(Box<BoardWorld>),
    /// A schematic.
    Schematic(Box<SchWorld>),
    /// A library.
    Library(Box<LibWorld>),
}

impl World {
    /// The bit of the kind of document, 0 for none.
    pub fn mask(&self) -> u8 {
        match self {
            World::None => 0,
            World::Board(_) => BOARD,
            World::Schematic(_) => SCHEMATIC,
            World::Library(_) => LIBRARY,
        }
    }

    /// The name of the kind of document, as the messages use it.
    pub fn name(&self) -> &'static str {
        match self {
            World::None => "no document",
            World::Board(_) => "a board",
            World::Schematic(_) => "a schematic",
            World::Library(_) => "a library",
        }
    }
}

/// A board being edited.
pub struct BoardWorld {
    /// The copy of the board, as it is now.
    pub board: Board,
    /// The operations made, in order.
    pub ops: Vec<Operation>,
    /// The inverse of each operation, in the same order.
    pub inverses: Vec<Operation>,
    /// The supply of identities.
    pub ids: Ids,
    /// True after an operation that replaces the whole board (the panel): the run then has no
    /// batch that undoes it.
    pub replaced: bool,
    max_ops: usize,
}

impl BoardWorld {
    /// A world on a copy of the board.
    pub fn new(board: &Board, settings: &ScriptSettings) -> BoardWorld {
        BoardWorld {
            ids: Ids::new(settings, used_ids(board)),
            board: board.clone(),
            ops: Vec::new(),
            inverses: Vec::new(),
            replaced: false,
            max_ops: settings.max_operations,
        }
    }

    /// Applies one operation to the copy and keeps it.
    pub fn push(&mut self, op: Operation) -> Result<(), Diag> {
        if self.ops.len() >= self.max_ops {
            return Err(Diag::plain(
                crate::codes::LIMIT_EXCEEDED,
                format!("more than {} operations in one run", self.max_ops),
            ));
        }
        let inverse = self
            .board
            .apply(&op)
            .map_err(|e| Diag::plain(REFUSED, e.to_string()))?;
        self.ops.push(op);
        self.inverses.push(inverse);
        Ok(())
    }

    /// Replaces the board by another one (a panel). The run then has no undo batch.
    pub fn replace(&mut self, board: Board) {
        self.ids.used = used_ids(&board);
        self.board = board;
        self.replaced = true;
    }

    /// The component with a reference designator.
    pub fn component_named(&self, refdes: &str) -> Option<Id> {
        self.board
            .components
            .iter()
            .find(|(_, c)| c.refdes == refdes)
            .map(|(id, _)| *id)
    }

    /// The net with a name.
    pub fn net_named(&self, name: &str) -> Option<Id> {
        self.board
            .nets
            .iter()
            .find(|(_, n)| n.name == name)
            .map(|(id, _)| *id)
    }

    /// The net with a name, made when the board has none.
    pub fn net_or_new(&mut self, name: &str) -> Result<Id, Diag> {
        if let Some(id) = self.net_named(name) {
            return Ok(id);
        }
        if name.is_empty() {
            return Err(Diag::plain(TYPE_MISMATCH, "a net needs a name"));
        }
        let id = self.ids.fresh()?;
        self.push(Operation::AddNet {
            id,
            name: name.to_owned(),
        })?;
        Ok(id)
    }

    /// The layer with a name, without regard to case.
    pub fn layer_named(&self, name: &str) -> Result<Id, Diag> {
        self.board
            .layers
            .iter()
            .find(|(_, l)| l.name.eq_ignore_ascii_case(name))
            .map(|(id, _)| *id)
            .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the board has no layer {name}")))
    }

    /// The first and the last copper layer.
    pub fn outer_copper(&self) -> Result<(Id, Id), Diag> {
        let stack = &self.board.copper_stack;
        if let (Some(first), Some(last)) = (stack.first(), stack.last()) {
            if stack.len() >= 2 {
                return Ok((*first, *last));
            }
        }
        let copper: Vec<Id> = self
            .board
            .layers
            .iter()
            .filter(|(_, l)| l.kind == eh3d_phoenix_board::LayerKind::Copper)
            .map(|(id, _)| *id)
            .collect();
        match (copper.first(), copper.last()) {
            (Some(first), Some(last)) if copper.len() >= 2 => Ok((*first, *last)),
            _ => Err(Diag::plain(
                NOT_FOUND,
                "the board has fewer than two copper layers",
            )),
        }
    }

    fn net_name(&self, id: &Id) -> &str {
        self.board.nets.get(id).map_or("", |n| n.name.as_str())
    }

    /// The objects of a kind whose name matches a pattern, in the order of their identities: the
    /// designator of a component, the net of a track, a via or a zone, the name of a net.
    pub fn select(&self, kind: ObjKind, pattern: &str) -> Vec<Value> {
        let b = &self.board;
        let make = |id: &Id| Value::Obj(Obj { kind, id: *id });
        match kind {
            ObjKind::Component => b
                .components
                .iter()
                .filter(|(_, c)| glob(pattern, &c.refdes))
                .map(|(id, _)| make(id))
                .collect(),
            ObjKind::Track => b
                .tracks
                .iter()
                .filter(|(_, t)| glob(pattern, self.net_name(&t.net)))
                .map(|(id, _)| make(id))
                .collect(),
            ObjKind::Via => b
                .vias
                .iter()
                .filter(|(_, v)| glob(pattern, self.net_name(&v.net)))
                .map(|(id, _)| make(id))
                .collect(),
            ObjKind::Zone => b
                .zones
                .iter()
                .filter(|(_, z)| glob(pattern, self.net_name(&z.net)))
                .map(|(id, _)| make(id))
                .collect(),
            ObjKind::Net => b
                .nets
                .iter()
                .filter(|(_, n)| glob(pattern, &n.name))
                .map(|(id, _)| make(id))
                .collect(),
            ObjKind::Part => Vec::new(),
        }
    }

    /// True when the object is still in the board.
    pub fn exists(&self, o: &Obj) -> bool {
        let b = &self.board;
        match o.kind {
            ObjKind::Component => b.components.contains_key(&o.id),
            ObjKind::Track => b.tracks.contains_key(&o.id),
            ObjKind::Via => b.vias.contains_key(&o.id),
            ObjKind::Zone => b.zones.contains_key(&o.id),
            ObjKind::Net => b.nets.contains_key(&o.id),
            ObjKind::Part => false,
        }
    }

    /// A short name of the object for the output.
    pub fn describe(&self, o: &Obj) -> String {
        let b = &self.board;
        match o.kind {
            ObjKind::Component => b
                .components
                .get(&o.id)
                .map_or_else(|| "component?".to_owned(), |c| c.refdes.clone()),
            ObjKind::Net => self.net_name(&o.id).to_owned(),
            ObjKind::Track => b.tracks.get(&o.id).map_or_else(
                || "track?".to_owned(),
                |t| format!("track({})", self.net_name(&t.net)),
            ),
            ObjKind::Via => b.vias.get(&o.id).map_or_else(
                || "via?".to_owned(),
                |v| format!("via({})", self.net_name(&v.net)),
            ),
            ObjKind::Zone => b.zones.get(&o.id).map_or_else(
                || "zone?".to_owned(),
                |z| format!("zone({})", self.net_name(&z.net)),
            ),
            ObjKind::Part => "part?".to_owned(),
        }
    }

    /// The place of a component or a via.
    pub fn position(&self, o: &Obj) -> Option<Point> {
        match o.kind {
            ObjKind::Component => self.board.components.get(&o.id).map(|c| c.at),
            ObjKind::Via => self.board.vias.get(&o.id).map(|v| v.at),
            _ => None,
        }
    }

    /// The copper length of a net, in nanometres: its tracks, an arc counted by its chord.
    pub fn net_length(&self, net: &Id) -> i64 {
        self.board
            .tracks
            .values()
            .filter(|t| t.net == *net)
            .map(|t| track_length(&t.path))
            .fold(0i64, i64::saturating_add)
    }

    /// A field of an object.
    pub fn field(&self, o: &Obj, name: &str) -> Result<Value, Diag> {
        let b = &self.board;
        let missing = || {
            Diag::plain(
                NOT_FOUND,
                format!("the {} has no field {name}", o.kind.word()),
            )
        };
        let gone = || {
            Diag::plain(
                NOT_FOUND,
                format!("the {} is not in the board", o.kind.word()),
            )
        };
        match o.kind {
            ObjKind::Component => {
                let c = b.components.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "refdes" => Value::Str(c.refdes.clone()),
                    "value" => Value::Str(c.value.clone()),
                    "footprint" => Value::Str(
                        b.footprints
                            .get(&c.footprint)
                            .map_or(String::new(), |f| f.name.clone()),
                    ),
                    "x" => Value::Int(c.at.x_nm.get()),
                    "y" => Value::Int(c.at.y_nm.get()),
                    "rot" => Value::Int(c.rotation.get()),
                    "side" => Value::Str(
                        match c.side {
                            eh3d_phoenix_board::Side::Top => "top",
                            eh3d_phoenix_board::Side::Bottom => "bottom",
                        }
                        .to_owned(),
                    ),
                    "pads" => Value::Int(
                        b.footprints
                            .get(&c.footprint)
                            .map_or(0, |f| i64::try_from(f.pads.len()).unwrap_or(0)),
                    ),
                    _ => return Err(missing()),
                })
            }
            ObjKind::Track => {
                let t = b.tracks.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "net" => Value::Str(self.net_name(&t.net).to_owned()),
                    "layer" => Value::Str(
                        b.layers
                            .get(&t.layer)
                            .map_or(String::new(), |l| l.name.clone()),
                    ),
                    "width" => Value::Int(t.width.get()),
                    "length" => Value::Int(track_length(&t.path)),
                    "segments" => Value::Int(i64::try_from(t.path.len()).unwrap_or(0)),
                    _ => return Err(missing()),
                })
            }
            ObjKind::Via => {
                let v = b.vias.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "net" => Value::Str(self.net_name(&v.net).to_owned()),
                    "x" => Value::Int(v.at.x_nm.get()),
                    "y" => Value::Int(v.at.y_nm.get()),
                    "drill" => Value::Int(v.drill.get()),
                    "diameter" => Value::Int(v.diameter.get()),
                    _ => return Err(missing()),
                })
            }
            ObjKind::Zone => {
                let z = b.zones.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "net" => Value::Str(self.net_name(&z.net).to_owned()),
                    "layer" => Value::Str(
                        b.layers
                            .get(&z.layer)
                            .map_or(String::new(), |l| l.name.clone()),
                    ),
                    "clearance" => Value::Int(z.clearance.get()),
                    "points" => Value::Int(i64::try_from(z.outline.len()).unwrap_or(0)),
                    _ => return Err(missing()),
                })
            }
            ObjKind::Net => {
                let n = b.nets.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "name" => Value::Str(n.name.clone()),
                    "tracks" => Value::Int(
                        i64::try_from(b.tracks.values().filter(|t| t.net == o.id).count())
                            .unwrap_or(0),
                    ),
                    "vias" => Value::Int(
                        i64::try_from(b.vias.values().filter(|v| v.net == o.id).count())
                            .unwrap_or(0),
                    ),
                    "pads" => Value::Int(
                        i64::try_from(b.pad_nets.iter().filter(|p| p.net == o.id).count())
                            .unwrap_or(0),
                    ),
                    "length" => Value::Int(self.net_length(&o.id)),
                    _ => return Err(missing()),
                })
            }
            ObjKind::Part => Err(Diag::plain(TARGET, "a part belongs to a schematic")),
        }
    }
}

/// The length of a path in nanometres; an arc is counted by its chord.
pub fn track_length(path: &[Segment]) -> i64 {
    path.iter()
        .map(|s| {
            let (a, b) = (s.start(), s.end());
            i64::try_from(isqrt(a.distance_squared(b))).unwrap_or(i64::MAX)
        })
        .fold(0i64, i64::saturating_add)
}

/// A schematic being edited.
pub struct SchWorld {
    /// The editor on the copy of the schematic.
    pub editor: Editor,
    /// The operations made, in order.
    pub ops: Vec<SchOp>,
    max_ops: usize,
}

impl SchWorld {
    /// A world on a copy of the schematic.
    pub fn new(schematic: &Schematic, settings: &ScriptSettings) -> SchWorld {
        SchWorld {
            editor: Editor::new(schematic.clone()),
            ops: Vec::new(),
            max_ops: settings.max_operations,
        }
    }

    /// The schematic as it is now.
    pub fn schematic(&self) -> &Schematic {
        self.editor.schematic()
    }

    /// Applies one operation to the copy and keeps it. Returns the identities of what it made.
    pub fn push(&mut self, op: SchOp) -> Result<Vec<Id>, Diag> {
        if self.ops.len() >= self.max_ops {
            return Err(Diag::plain(
                crate::codes::LIMIT_EXCEEDED,
                format!("more than {} operations in one run", self.max_ops),
            ));
        }
        let applied = self
            .editor
            .apply(&op)
            .map_err(|e| Diag::plain(REFUSED, format!("{e:?}")))?;
        self.ops.push(op);
        Ok(applied.created)
    }

    /// The part with a reference designator.
    pub fn part_named(&self, refdes: &str) -> Option<Id> {
        self.schematic()
            .parts
            .iter()
            .find(|(_, p)| p.refdes == refdes)
            .map(|(id, _)| *id)
    }

    /// The objects of a kind whose name matches a pattern: the designator of a part, the name of
    /// a net.
    pub fn select(&self, kind: ObjKind, pattern: &str) -> Vec<Value> {
        let s = self.schematic();
        let make = |id: &Id| Value::Obj(Obj { kind, id: *id });
        match kind {
            ObjKind::Part => s
                .parts
                .iter()
                .filter(|(_, p)| glob(pattern, &p.refdes))
                .map(|(id, _)| make(id))
                .collect(),
            ObjKind::Net => s
                .nets
                .iter()
                .filter(|(_, n)| glob(pattern, &n.name))
                .map(|(id, _)| make(id))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// True when the object is still in the schematic.
    pub fn exists(&self, o: &Obj) -> bool {
        let s = self.schematic();
        match o.kind {
            ObjKind::Part => s.parts.contains_key(&o.id),
            ObjKind::Net => s.nets.contains_key(&o.id),
            _ => false,
        }
    }

    /// A short name of the object for the output.
    pub fn describe(&self, o: &Obj) -> String {
        let s = self.schematic();
        match o.kind {
            ObjKind::Part => s
                .parts
                .get(&o.id)
                .map_or_else(|| "part?".to_owned(), |p| p.refdes.clone()),
            ObjKind::Net => s
                .nets
                .get(&o.id)
                .map_or_else(|| "net?".to_owned(), |n| n.name.clone()),
            other => format!("{}?", other.word()),
        }
    }

    /// The place of a part that is drawn.
    pub fn position(&self, o: &Obj) -> Option<Point> {
        match o.kind {
            ObjKind::Part => self
                .schematic()
                .parts
                .get(&o.id)
                .and_then(|p| p.placement.map(|pl| pl.at)),
            _ => None,
        }
    }

    /// A field of an object.
    pub fn field(&self, o: &Obj, name: &str) -> Result<Value, Diag> {
        let s = self.schematic();
        let missing = || {
            Diag::plain(
                NOT_FOUND,
                format!("the {} has no field {name}", o.kind.word()),
            )
        };
        let gone = || {
            Diag::plain(
                NOT_FOUND,
                format!("the {} is not in the schematic", o.kind.word()),
            )
        };
        match o.kind {
            ObjKind::Part => {
                let p = s.parts.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "refdes" => Value::Str(p.refdes.clone()),
                    "value" => Value::Str(p.value.clone()),
                    "symbol" => Value::Str(
                        s.symbols
                            .get(&p.symbol)
                            .map_or(String::new(), |sym| sym.name.clone()),
                    ),
                    "x" => Value::Int(p.placement.map_or(0, |pl| pl.at.x_nm.get())),
                    "y" => Value::Int(p.placement.map_or(0, |pl| pl.at.y_nm.get())),
                    "rot" => Value::Int(p.placement.map_or(0, |pl| pl.rotation.get())),
                    "pins" => Value::Int(
                        s.symbols
                            .get(&p.symbol)
                            .map_or(0, |sym| i64::try_from(sym.pins.len()).unwrap_or(0)),
                    ),
                    _ => return Err(missing()),
                })
            }
            ObjKind::Net => {
                let n = s.nets.get(&o.id).ok_or_else(gone)?;
                Ok(match name {
                    "name" => Value::Str(n.name.clone()),
                    _ => return Err(missing()),
                })
            }
            _ => Err(Diag::plain(
                TARGET,
                "this kind of object belongs to a board",
            )),
        }
    }
}

/// A library being edited.
pub struct LibWorld {
    /// The copy of the library, as it is now.
    pub lib: Library,
    /// The operations made, in order.
    pub ops: Vec<LibOp>,
    max_ops: usize,
}

impl LibWorld {
    /// A world on a copy of the library.
    pub fn new(lib: &Library, settings: &ScriptSettings) -> LibWorld {
        LibWorld {
            lib: lib.clone(),
            ops: Vec::new(),
            max_ops: settings.max_operations,
        }
    }

    /// Applies one operation to the copy and keeps it.
    pub fn push(&mut self, op: LibOp) -> Result<(), Diag> {
        if self.ops.len() >= self.max_ops {
            return Err(Diag::plain(
                crate::codes::LIMIT_EXCEEDED,
                format!("more than {} operations in one run", self.max_ops),
            ));
        }
        eh3d_phoenix_libedit::ops::apply_one(&mut self.lib, &op)
            .map_err(|e| Diag::plain(REFUSED, e.to_string()))?;
        self.ops.push(op);
        Ok(())
    }
}
