// SPDX-License-Identifier: GPL-3.0-or-later
//! The operations of the schematic (move, turn, mirror, rename, set a value, remove, place a part,
//! draw a wire, put a label, renumber) and of the library (create, rename and delete footprints,
//! symbols and device sets, add pads). Each is one operation of the editor of the model.

use eh3d_phoenix_libedit::model::{Pad, Smd};
use eh3d_phoenix_libedit::ops::Target;
use eh3d_phoenix_libedit::{Item as LibItem, Kind, Operation as LibOp};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_schematic::{EditOp as SchOp, Id, Item};

use crate::codes::Diag;
use crate::v2::codes::{ARITHMETIC, NOT_FOUND, TYPE_MISMATCH};
use crate::v2::funcs::{def, int, quarter_turns, text, FuncDef};
use crate::v2::interp::Interp;
use crate::v2::value::{glob, Obj, ObjKind, Value};
use crate::v2::world::{SchWorld, LIBRARY, SCHEMATIC};

type R = Result<Value, Diag>;

fn point(x: i64, y: i64) -> Result<Point, Diag> {
    Point::from_nm(x, y)
        .map_err(|_| Diag::plain(ARITHMETIC, "the place is outside the range of the model"))
}

fn part_id(w: &SchWorld, v: &Value) -> Result<Id, Diag> {
    match v {
        Value::Obj(o) if o.kind == ObjKind::Part => {
            if w.exists(o) {
                Ok(o.id)
            } else {
                Err(Diag::plain(NOT_FOUND, "the part is not in the schematic"))
            }
        }
        Value::Str(name) => w
            .part_named(name)
            .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the schematic has no part {name}"))),
        other => Err(Diag::plain(
            TYPE_MISMATCH,
            format!(
                "a part or its designator was expected, not {}",
                other.type_name()
            ),
        )),
    }
}

fn first_sheet(w: &SchWorld) -> Result<Id, Diag> {
    w.schematic()
        .sheets
        .keys()
        .next()
        .copied()
        .ok_or_else(|| Diag::plain(NOT_FOUND, "the schematic has no sheet"))
}

pub(crate) fn sch_move_to(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (x, y) = (int(a, 1)?, int(a, 2)?);
    let w = i.sch()?;
    let id = part_id(w, &a[0])?;
    let at = w
        .position(&Obj {
            kind: ObjKind::Part,
            id,
        })
        .ok_or_else(|| Diag::plain(NOT_FOUND, "the part is not drawn"))?;
    let over = || Diag::plain(ARITHMETIC, "the offset does not fit 64 bits");
    let dx = x.checked_sub(at.x_nm.get()).ok_or_else(over)?;
    let dy = y.checked_sub(at.y_nm.get()).ok_or_else(over)?;
    w.push(SchOp::MoveItems {
        items: vec![Item::Part(id)],
        dx,
        dy,
        detach: false,
    })?;
    Ok(Value::Nil)
}

pub(crate) fn sch_move_by(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (dx, dy) = (int(a, 1)?, int(a, 2)?);
    let w = i.sch()?;
    let id = part_id(w, &a[0])?;
    w.push(SchOp::MoveItems {
        items: vec![Item::Part(id)],
        dx,
        dy,
        detach: false,
    })?;
    Ok(Value::Nil)
}

pub(crate) fn sch_rotate_by(i: &mut Interp<'_>, a: &[Value]) -> R {
    let turns = quarter_turns(a, 1)?;
    let w = i.sch()?;
    let id = part_id(w, &a[0])?;
    w.push(SchOp::RotateParts {
        parts: vec![id],
        quarter_turns: turns,
    })?;
    Ok(Value::Nil)
}

pub(crate) fn sch_mirror(i: &mut Interp<'_>, a: &[Value]) -> R {
    let w = i.sch()?;
    let id = part_id(w, &a[0])?;
    w.push(SchOp::MirrorParts { parts: vec![id] })?;
    Ok(Value::Nil)
}

pub(crate) fn sch_rename(i: &mut Interp<'_>, a: &[Value]) -> R {
    let new = text(a, 1)?.to_owned();
    let w = i.sch()?;
    let id = part_id(w, &a[0])?;
    w.push(SchOp::SetRefdes {
        part: id,
        refdes: new,
    })?;
    Ok(Value::Nil)
}

pub(crate) fn sch_set_value(i: &mut Interp<'_>, target: &Value, value: &str) -> Result<(), Diag> {
    let w = i.sch()?;
    let id = part_id(w, target)?;
    w.push(SchOp::SetValue {
        part: id,
        value: value.to_owned(),
    })?;
    Ok(())
}

pub(crate) fn sch_set_value_fn(i: &mut Interp<'_>, a: &[Value]) -> R {
    let value = text(a, 1)?.to_owned();
    sch_set_value(i, &a[0], &value)?;
    Ok(Value::Nil)
}

pub(crate) fn sch_remove(i: &mut Interp<'_>, a: &[Value]) -> R {
    let w = i.sch()?;
    let id = part_id(w, &a[0])?;
    w.push(SchOp::Delete {
        items: vec![Item::Part(id)],
    })?;
    Ok(Value::Nil)
}

pub(crate) fn sch_renumber(i: &mut Interp<'_>, objs: &[Obj], prefix: &str, start: i64) -> R {
    let w = i.sch()?;
    let mut changes: Vec<(Id, String)> = Vec::new();
    for (k, o) in objs.iter().enumerate() {
        if o.kind != ObjKind::Part || !w.exists(o) {
            return Err(Diag::plain(
                TYPE_MISMATCH,
                "renumber takes parts of the schematic",
            ));
        }
        let n = i64::try_from(k)
            .ok()
            .and_then(|k| start.checked_add(k))
            .ok_or_else(|| Diag::plain(ARITHMETIC, "the number does not fit 64 bits"))?;
        let wanted = format!("{prefix}{n}");
        let current = w
            .schematic()
            .parts
            .get(&o.id)
            .map_or("", |p| p.refdes.as_str());
        if wanted != current {
            changes.push((o.id, wanted));
        }
    }
    for (k, (id, _)) in changes.iter().enumerate() {
        w.push(SchOp::SetRefdes {
            part: *id,
            refdes: format!("TMP_RENUMBER_{k}"),
        })?;
    }
    for (id, wanted) in changes {
        w.push(SchOp::SetRefdes {
            part: id,
            refdes: wanted,
        })?;
    }
    i64::try_from(objs.len())
        .map(Value::Int)
        .map_err(|_| Diag::plain(ARITHMETIC, "the count does not fit 64 bits"))
}

fn f_place_part(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (refdes, symbol) = (text(a, 0)?.to_owned(), text(a, 1)?.to_owned());
    let (x, y) = (int(a, 2)?, int(a, 3)?);
    let value = if a.len() > 4 {
        Some(text(a, 4)?.to_owned())
    } else {
        None
    };
    let w = i.sch()?;
    let sheet = first_sheet(w)?;
    let made = w.push(SchOp::PlaceSymbol {
        symbol,
        define: None,
        sheet,
        at: point(x, y)?,
        rotation: None,
        mirrored: false,
        refdes: Some(refdes),
        value,
    })?;
    let id = made
        .last()
        .copied()
        .ok_or_else(|| Diag::plain(NOT_FOUND, "the editor made no part"))?;
    Ok(Value::Obj(Obj {
        kind: ObjKind::Part,
        id,
    }))
}

fn f_draw_wire(i: &mut Interp<'_>, a: &[Value]) -> R {
    let items = crate::v2::funcs::list(a, 0)?;
    if items.len() & 1 == 1 || items.len() < 4 {
        return Err(Diag::plain(
            TYPE_MISMATCH,
            "a wire is a flat list of x, y pairs with at least two points",
        ));
    }
    let mut path = Vec::new();
    for (x, y) in items.iter().step_by(2).zip(items.iter().skip(1).step_by(2)) {
        let (Value::Int(x), Value::Int(y)) = (x, y) else {
            return Err(Diag::plain(TYPE_MISMATCH, "the points are integers"));
        };
        path.push(point(*x, *y)?);
    }
    let w = i.sch()?;
    let sheet = first_sheet(w)?;
    w.push(SchOp::AddWire { sheet, path })?;
    Ok(Value::Nil)
}

fn f_label(i: &mut Interp<'_>, a: &[Value]) -> R {
    let name = text(a, 0)?.to_owned();
    let (x, y) = (int(a, 1)?, int(a, 2)?);
    let w = i.sch()?;
    let sheet = first_sheet(w)?;
    w.push(SchOp::AddLabel {
        sheet,
        at: point(x, y)?,
        name,
    })?;
    Ok(Value::Nil)
}

fn lib_kind(word: &str) -> Result<Kind, Diag> {
    match word {
        "package" => Ok(Kind::Package),
        "symbol" => Ok(Kind::Symbol),
        "deviceset" => Ok(Kind::DeviceSet),
        other => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("{other:?} is not package, symbol or deviceset"),
        )),
    }
}

fn f_lib_names(i: &mut Interp<'_>, a: &[Value]) -> R {
    let kind = lib_kind(text(a, 0)?)?;
    let pattern = text(a, 1)?.to_owned();
    let w = i.lib()?;
    Ok(Value::List(
        w.lib
            .names(kind)
            .into_iter()
            .filter(|n| glob(&pattern, n))
            .map(Value::Str)
            .collect(),
    ))
}

fn f_lib_create(i: &mut Interp<'_>, a: &[Value]) -> R {
    let kind = lib_kind(text(a, 0)?)?;
    let name = text(a, 1)?.to_owned();
    let op = match kind {
        Kind::Package => LibOp::CreatePackage { name },
        Kind::Symbol => LibOp::CreateSymbol { name },
        Kind::DeviceSet => LibOp::CreateDeviceSet {
            name,
            prefix: String::new(),
        },
    };
    i.lib()?.push(op)?;
    Ok(Value::Nil)
}

fn f_lib_rename(i: &mut Interp<'_>, a: &[Value]) -> R {
    let kind = lib_kind(text(a, 0)?)?;
    let (from, to) = (text(a, 1)?.to_owned(), text(a, 2)?.to_owned());
    let op = match kind {
        Kind::Package => LibOp::RenamePackage { from, to },
        Kind::Symbol => LibOp::RenameSymbol { from, to },
        Kind::DeviceSet => LibOp::RenameDeviceSet { from, to },
    };
    i.lib()?.push(op)?;
    Ok(Value::Nil)
}

fn f_lib_delete(i: &mut Interp<'_>, a: &[Value]) -> R {
    let kind = lib_kind(text(a, 0)?)?;
    let name = text(a, 1)?.to_owned();
    let op = match kind {
        Kind::Package => LibOp::DeletePackage { name },
        Kind::Symbol => LibOp::DeleteSymbol { name },
        Kind::DeviceSet => LibOp::DeleteDeviceSet { name },
    };
    i.lib()?.push(op)?;
    Ok(Value::Nil)
}

fn f_lib_add_pad(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (package, name) = (text(a, 0)?, text(a, 1)?.to_owned());
    let (x, y, drill, diameter) = (int(a, 2)?, int(a, 3)?, int(a, 4)?, int(a, 5)?);
    let target = Target::package(package);
    i.lib()?.push(LibOp::AddItem {
        target,
        item: LibItem::Pad(Pad {
            name,
            x,
            y,
            drill,
            diameter,
            ..Pad::default()
        }),
        at: None,
    })?;
    Ok(Value::Nil)
}

fn f_lib_add_smd(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (package, name) = (text(a, 0)?, text(a, 1)?.to_owned());
    let (x, y, dx, dy) = (int(a, 2)?, int(a, 3)?, int(a, 4)?, int(a, 5)?);
    let target = Target::package(package);
    i.lib()?.push(LibOp::AddItem {
        target,
        item: LibItem::Smd(Smd {
            name,
            x,
            y,
            dx,
            dy,
            ..Smd::default()
        }),
        at: None,
    })?;
    Ok(Value::Nil)
}

fn f_lib_pad_count(i: &mut Interp<'_>, a: &[Value]) -> R {
    let package = text(a, 0)?.to_owned();
    let w = i.lib()?;
    let p = w
        .lib
        .package(&package)
        .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the library has no footprint {package}")))?;
    i64::try_from(p.pads().count())
        .map(Value::Int)
        .map_err(|_| Diag::plain(ARITHMETIC, "the count does not fit 64 bits"))
}

/// The operations of the schematic that the board does not share, and those of the library.
pub(crate) static TABLE: &[FuncDef] = &[
    def("place_part", 4, 5, SCHEMATIC, true, "place_part(refdes, symbol, x, y[, value])", "Places a part of a symbol that the schematic has on its first sheet; returns it.", f_place_part),
    def("draw_wire", 1, 1, SCHEMATIC, true, "draw_wire(points)", "Draws a wire of horizontal and vertical pieces through points given as a flat list on the first sheet.", f_draw_wire),
    def("label", 3, 3, SCHEMATIC, true, "label(name, x, y)", "Puts a net label on the first sheet.", f_label),
    def("lib_names", 2, 2, LIBRARY, false, "lib_names(kind, pattern)", "The names of the footprints (package), symbols (symbol) or device sets (deviceset) that match the pattern.", f_lib_names),
    def("lib_create", 2, 2, LIBRARY, true, "lib_create(kind, name)", "Creates an empty footprint, symbol or device set.", f_lib_create),
    def("lib_rename", 3, 3, LIBRARY, true, "lib_rename(kind, from, to)", "Renames a footprint, a symbol or a device set.", f_lib_rename),
    def("lib_delete", 2, 2, LIBRARY, true, "lib_delete(kind, name)", "Deletes a footprint, a symbol or a device set.", f_lib_delete),
    def("lib_add_pad", 6, 6, LIBRARY, true, "lib_add_pad(footprint, name, x, y, drill, diameter)", "Adds a through-hole pad to a footprint.", f_lib_add_pad),
    def("lib_add_smd", 6, 6, LIBRARY, true, "lib_add_smd(footprint, name, x, y, width, height)", "Adds a surface-mount pad to a footprint.", f_lib_add_smd),
    def("lib_pad_count", 1, 1, LIBRARY, false, "lib_pad_count(footprint)", "The number of pads of a footprint.", f_lib_pad_count),
];
