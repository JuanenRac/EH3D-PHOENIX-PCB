// SPDX-License-Identifier: GPL-3.0-or-later
//! The operations of the board: place, move, turn, mirror, rename, set a value, remove, draw
//! tracks, vias and zones, join a pad to a net, renumber, panelize and assign a part of the base.
//! Each one is made of the reversible operations of the board model.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{
    Component, Id, LayerKind, Operation, PadRef, Segment, Side, Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_panel::{compose, plan, BoardShape, PanelSettings};
use eh3d_phoenix_rules::Lang;

use crate::codes::Diag;
use crate::v2::codes::{ARITHMETIC, NOT_FOUND, PARTS_BASE, REFUSED, TYPE_MISMATCH};
use crate::v2::funcs::{def, int, list, text, FuncDef};
use crate::v2::funcs_doc::objects;
use crate::v2::interp::Interp;
use crate::v2::value::{Obj, ObjKind, Value};
use crate::v2::world::{BoardWorld, World, BOARD, SCHEMATIC};

type R = Result<Value, Diag>;

fn point(x: i64, y: i64) -> Result<Point, Diag> {
    Point::from_nm(x, y)
        .map_err(|_| Diag::plain(ARITHMETIC, "the place is outside the range of the model"))
}

fn nm(value: i64, what: &str) -> Result<Nm, Diag> {
    Nm::new(value).map_err(|_| Diag::plain(ARITHMETIC, format!("{what} is outside the range")))
}

fn angle(udeg: i64) -> Result<Udeg, Diag> {
    Udeg::new(udeg.rem_euclid(360_000_000))
        .map_err(|_| Diag::plain(ARITHMETIC, "the angle is outside the range"))
}

fn component_id(w: &BoardWorld, v: &Value) -> Result<Id, Diag> {
    match v {
        Value::Obj(o) if o.kind == ObjKind::Component => {
            if w.board.components.contains_key(&o.id) {
                Ok(o.id)
            } else {
                Err(Diag::plain(NOT_FOUND, "the component is not in the board"))
            }
        }
        Value::Str(name) => w
            .component_named(name)
            .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the board has no component {name}"))),
        other => Err(Diag::plain(
            TYPE_MISMATCH,
            format!(
                "a component or its designator was expected, not {}",
                other.type_name()
            ),
        )),
    }
}

fn component_of(w: &BoardWorld, id: Id) -> Result<&Component, Diag> {
    w.board
        .components
        .get(&id)
        .ok_or_else(|| Diag::plain(NOT_FOUND, "the component is not in the board"))
}

fn points(args: &[Value], index: usize, least: usize) -> Result<Vec<Point>, Diag> {
    let items = list(args, index)?;
    if items.len() % 2 != 0 || items.len() < least * 2 {
        return Err(Diag::plain(
            TYPE_MISMATCH,
            format!(
                "argument {} must be a flat list of x, y pairs with at least {least} points",
                index + 1
            ),
        ));
    }
    let mut out = Vec::with_capacity(items.len() / 2);
    for (x, y) in items.iter().step_by(2).zip(items.iter().skip(1).step_by(2)) {
        let (Value::Int(x), Value::Int(y)) = (x, y) else {
            return Err(Diag::plain(TYPE_MISMATCH, "the points are integers"));
        };
        out.push(point(*x, *y)?);
    }
    Ok(out)
}

fn set_refdes(w: &mut BoardWorld, id: Id, refdes: &str) -> Result<(), Diag> {
    let c = component_of(w, id)?;
    let op = Operation::SetComponentFields {
        id,
        refdes: refdes.to_owned(),
        value: c.value.clone(),
        footprint: c.footprint,
    };
    w.push(op)
}

fn obj(kind: ObjKind, id: Id) -> Value {
    Value::Obj(Obj { kind, id })
}

fn board_move_to(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (x, y) = (int(a, 1)?, int(a, 2)?);
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let rotation = component_of(w, id)?.rotation;
    w.push(Operation::MoveComponent {
        id,
        at: point(x, y)?,
        rotation,
    })?;
    Ok(Value::Nil)
}

fn board_move_by(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (dx, dy) = (int(a, 1)?, int(a, 2)?);
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let c = component_of(w, id)?;
    let (rotation, at) = (c.rotation, c.at);
    let sum = |p: Nm, d: i64| {
        p.get()
            .checked_add(d)
            .ok_or_else(|| Diag::plain(ARITHMETIC, "the place does not fit 64 bits"))
    };
    let (x, y) = (sum(at.x_nm, dx)?, sum(at.y_nm, dy)?);
    w.push(Operation::MoveComponent {
        id,
        at: point(x, y)?,
        rotation,
    })?;
    Ok(Value::Nil)
}

fn f_rotate_to(i: &mut Interp<'_>, a: &[Value]) -> R {
    let turned = int(a, 1)?;
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let at = component_of(w, id)?.at;
    w.push(Operation::MoveComponent {
        id,
        at,
        rotation: angle(turned)?,
    })?;
    Ok(Value::Nil)
}

fn board_rotate_by(i: &mut Interp<'_>, a: &[Value]) -> R {
    let delta = int(a, 1)?;
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let c = component_of(w, id)?;
    let (at, current) = (c.at, c.rotation.get());
    let turned = current
        .checked_add(delta)
        .ok_or_else(|| Diag::plain(ARITHMETIC, "the angle does not fit 64 bits"))?;
    w.push(Operation::MoveComponent {
        id,
        at,
        rotation: angle(turned)?,
    })?;
    Ok(Value::Nil)
}

fn board_mirror(i: &mut Interp<'_>, a: &[Value]) -> R {
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let c = component_of(w, id)?.clone();
    let pads: Vec<(Id, Id)> = w
        .board
        .pad_nets
        .iter()
        .filter(|p| p.component == id)
        .map(|p| (p.pad, p.net))
        .collect();
    let flipped = Component {
        side: match c.side {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        },
        rotation: angle(360_000_000 - c.rotation.get())?,
        ..c
    };
    w.push(Operation::RemoveComponent { id })?;
    w.push(Operation::AddComponent {
        id,
        component: flipped,
    })?;
    for (pad, net) in pads {
        w.push(Operation::SetPadNet {
            pad: PadRef { component: id, pad },
            net: Some(net),
        })?;
    }
    Ok(Value::Nil)
}

fn board_rename(i: &mut Interp<'_>, a: &[Value]) -> R {
    let new = text(a, 1)?;
    if new.is_empty() {
        return Err(Diag::plain(TYPE_MISMATCH, "the new name is empty"));
    }
    let w = i.board()?;
    let net_target = match &a[0] {
        Value::Obj(o) if o.kind == ObjKind::Net => Some(o.id),
        Value::Str(name) if w.component_named(name).is_none() => w.net_named(name),
        _ => None,
    };
    if let Some(id) = net_target {
        if w.net_named(new).is_some() {
            return Err(Diag::plain(
                REFUSED,
                format!("a net is already named {new}"),
            ));
        }
        w.push(Operation::RenameNet {
            id,
            name: new.to_owned(),
        })?;
        return Ok(Value::Nil);
    }
    let id = component_id(w, &a[0])?;
    if w.component_named(new).is_some() {
        return Err(Diag::plain(
            REFUSED,
            format!("a component is already named {new}"),
        ));
    }
    set_refdes(w, id, new)?;
    Ok(Value::Nil)
}

fn board_set_value(i: &mut Interp<'_>, a: &[Value]) -> R {
    let value = text(a, 1)?.to_owned();
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let c = component_of(w, id)?;
    let op = Operation::SetComponentFields {
        id,
        refdes: c.refdes.clone(),
        value,
        footprint: c.footprint,
    };
    w.push(op)?;
    Ok(Value::Nil)
}

fn board_remove(i: &mut Interp<'_>, a: &[Value]) -> R {
    let w = i.board()?;
    let op = match &a[0] {
        Value::Obj(o) if w.exists(o) => match o.kind {
            ObjKind::Component => Operation::RemoveComponent { id: o.id },
            ObjKind::Track => Operation::RemoveTrack { id: o.id },
            ObjKind::Via => Operation::RemoveVia { id: o.id },
            ObjKind::Zone => Operation::RemoveZone { id: o.id },
            _ => {
                return Err(Diag::plain(
                    TYPE_MISMATCH,
                    "remove takes a component, a track, a via or a zone",
                ))
            }
        },
        Value::Obj(_) => return Err(Diag::plain(NOT_FOUND, "the object is not in the board")),
        other => Operation::RemoveComponent {
            id: component_id(w, other)?,
        },
    };
    w.push(op)?;
    Ok(Value::Nil)
}

fn f_place(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (refdes, footprint) = (text(a, 0)?.to_owned(), text(a, 1)?);
    let (x, y) = (int(a, 2)?, int(a, 3)?);
    let rotation = if a.len() > 4 { int(a, 4)? } else { 0 };
    let value = if a.len() > 5 {
        text(a, 5)?.to_owned()
    } else {
        String::new()
    };
    let w = i.board()?;
    let footprint_id = w
        .board
        .footprints
        .iter()
        .find(|(_, f)| f.name == footprint)
        .map(|(id, _)| *id)
        .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the board has no footprint {footprint}")))?;
    if w.component_named(&refdes).is_some() {
        return Err(Diag::plain(
            REFUSED,
            format!("a component is already named {refdes}"),
        ));
    }
    let id = w.ids.fresh()?;
    w.push(Operation::AddComponent {
        id,
        component: Component {
            refdes,
            value,
            symbol: None,
            footprint: footprint_id,
            at: point(x, y)?,
            rotation: angle(rotation)?,
            side: Side::Top,
        },
    })?;
    Ok(obj(ObjKind::Component, id))
}

fn f_route(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (net, layer) = (text(a, 0)?.to_owned(), text(a, 1)?.to_owned());
    let width = nm(int(a, 2)?, "the width")?;
    let pts = points(a, 3, 2)?;
    let w = i.board()?;
    let layer_id = w.layer_named(&layer)?;
    if w.board.layers.get(&layer_id).map(|l| l.kind) != Some(LayerKind::Copper) {
        return Err(Diag::plain(
            REFUSED,
            format!("{layer} is not a copper layer"),
        ));
    }
    let path: Vec<Segment> = pts
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .map(|pair| Segment::Line {
            start: pair[0],
            end: pair[1],
        })
        .collect();
    if path.is_empty() {
        return Err(Diag::plain(TYPE_MISMATCH, "all the points coincide"));
    }
    let net_id = w.net_or_new(&net)?;
    let id = w.ids.fresh()?;
    w.push(Operation::AddTrack {
        id,
        track: Track {
            net: net_id,
            layer: layer_id,
            width,
            path,
        },
    })?;
    Ok(obj(ObjKind::Track, id))
}

fn f_via(i: &mut Interp<'_>, a: &[Value]) -> R {
    let net = text(a, 0)?.to_owned();
    let (x, y) = (int(a, 1)?, int(a, 2)?);
    let s = i.ctx.settings;
    let diameter = if a.len() > 3 {
        int(a, 3)?
    } else {
        s.default_via_diameter_nm
    };
    let drill = if a.len() > 4 {
        int(a, 4)?
    } else {
        s.default_drill_nm
    };
    if drill <= 0 || drill >= diameter {
        return Err(Diag::plain(
            TYPE_MISMATCH,
            "the drill must be positive and smaller than the diameter",
        ));
    }
    let w = i.board()?;
    let (from_layer, to_layer) = w.outer_copper()?;
    let net_id = w.net_or_new(&net)?;
    let id = w.ids.fresh()?;
    w.push(Operation::AddVia {
        id,
        via: Via {
            net: net_id,
            at: point(x, y)?,
            drill: nm(drill, "the drill")?,
            diameter: nm(diameter, "the diameter")?,
            from_layer,
            to_layer,
        },
    })?;
    Ok(obj(ObjKind::Via, id))
}

fn f_zone(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (net, layer) = (text(a, 0)?.to_owned(), text(a, 1)?.to_owned());
    let outline = points(a, 2, 3)?;
    let clearance = nm(int(a, 3)?, "the clearance")?;
    let w = i.board()?;
    let layer_id = w.layer_named(&layer)?;
    let net_id = w.net_or_new(&net)?;
    let id = w.ids.fresh()?;
    w.push(Operation::AddZone {
        id,
        zone: Zone {
            net: net_id,
            layer: layer_id,
            outline,
            clearance,
        },
    })?;
    Ok(obj(ObjKind::Zone, id))
}

fn f_connect(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (number, net) = (text(a, 1)?.to_owned(), text(a, 2)?.to_owned());
    let w = i.board()?;
    let id = component_id(w, &a[0])?;
    let footprint = component_of(w, id)?.footprint;
    let pad = w
        .board
        .footprints
        .get(&footprint)
        .and_then(|f| f.pads.iter().find(|(_, p)| p.number == number))
        .map(|(pad_id, _)| *pad_id)
        .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the footprint has no pad {number}")))?;
    let net_id = w.net_or_new(&net)?;
    w.push(Operation::SetPadNet {
        pad: PadRef { component: id, pad },
        net: Some(net_id),
    })?;
    Ok(Value::Nil)
}

fn board_renumber(i: &mut Interp<'_>, objs: &[Obj], prefix: &str, start: i64) -> R {
    let w = i.board()?;
    let mut changes: Vec<(Id, String)> = Vec::new();
    for (k, o) in objs.iter().enumerate() {
        if o.kind != ObjKind::Component {
            return Err(Diag::plain(TYPE_MISMATCH, "renumber takes components"));
        }
        let c = component_of(w, o.id)?;
        let n = i64::try_from(k)
            .ok()
            .and_then(|k| start.checked_add(k))
            .ok_or_else(|| Diag::plain(ARITHMETIC, "the number does not fit 64 bits"))?;
        let wanted = format!("{prefix}{n}");
        if wanted != c.refdes {
            changes.push((o.id, wanted));
        }
    }
    let moving: BTreeSet<Id> = changes.iter().map(|(id, _)| *id).collect();
    for (_, wanted) in &changes {
        let clash = w
            .board
            .components
            .iter()
            .any(|(id, c)| !moving.contains(id) && c.refdes == *wanted);
        if clash {
            return Err(Diag::plain(
                REFUSED,
                format!("a component that is not renumbered is already named {wanted}"),
            ));
        }
    }
    for (k, (id, _)) in changes.iter().enumerate() {
        set_refdes(w, *id, &format!("TMP_RENUMBER_{k}"))?;
    }
    for (id, wanted) in &changes {
        set_refdes(w, *id, wanted)?;
    }
    count_value(objs.len())
}

fn count_value(n: usize) -> R {
    i64::try_from(n)
        .map(Value::Int)
        .map_err(|_| Diag::plain(ARITHMETIC, "the count does not fit 64 bits"))
}

fn f_renumber(i: &mut Interp<'_>, a: &[Value]) -> R {
    let objs = objects(a, 0)?;
    let prefix = text(a, 1)?.to_owned();
    let start = int(a, 2)?;
    match i.world.mask() {
        BOARD => board_renumber(i, &objs, &prefix, start),
        SCHEMATIC => crate::v2::funcs_sch::sch_renumber(i, &objs, &prefix, start),
        _ => Err(Diag::plain(NOT_FOUND, "there is no board or schematic")),
    }
}

fn f_panelize(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (cols, rows, gap) = (int(a, 0)?, int(a, 1)?, int(a, 2)?);
    let (Ok(columns), Ok(row_count)) = (u32::try_from(cols), u32::try_from(rows)) else {
        return Err(Diag::plain(ARITHMETIC, "the counts are not 1 to 64"));
    };
    if !(1..=64).contains(&columns) || !(1..=64).contains(&row_count) || gap < 0 {
        return Err(Diag::plain(
            ARITHMETIC,
            "the counts are not 1 to 64 or the gap is negative",
        ));
    }
    let w = i.board()?;
    let refused = |e: eh3d_phoenix_panel::PanelError| Diag::plain(REFUSED, e.message(Lang::En));
    let (shape, _notes) = BoardShape::from_board(&w.board).map_err(refused)?;
    let settings = PanelSettings {
        columns,
        rows: row_count,
        gap_nm: gap,
        copy_suffix: true,
        ..PanelSettings::default()
    };
    let layout = plan(&shape, &settings).map_err(refused)?;
    let panel =
        compose(std::slice::from_ref(&w.board), &layout, &settings, 0, false).map_err(refused)?;
    w.replace(panel);
    count_value(usize::try_from(columns * row_count).unwrap_or(0))
}

/// The value and the footprint that the parts base gives for a part.
pub(crate) fn base_part(i: &Interp<'_>, id: &str) -> Result<(String, String), Diag> {
    let Some(db) = i.ctx.parts else {
        return Err(Diag::plain(
            PARTS_BASE,
            "no parts base was given (--parts FILE)",
        ));
    };
    let index = eh3d_phoenix_parts::Index::build(db);
    let part = index
        .get(id)
        .ok_or_else(|| Diag::plain(PARTS_BASE, format!("the base has no part {id}")))?;
    let value = part.value.trim();
    if value.is_empty() {
        return Err(Diag::plain(
            PARTS_BASE,
            format!("the part {id} has no value"),
        ));
    }
    Ok((value.to_owned(), part.footprint.clone()))
}

fn f_assign_part(i: &mut Interp<'_>, a: &[Value]) -> R {
    let id = text(a, 1)?.to_owned();
    let (value, footprint) = base_part(i, &id)?;
    match i.world.mask() {
        SCHEMATIC => crate::v2::funcs_sch::sch_set_value(i, &a[0], &value)?,
        _ => {
            let w = i.board()?;
            let comp = component_id(w, &a[0])?;
            let c = component_of(w, comp)?;
            let op = Operation::SetComponentFields {
                id: comp,
                refdes: c.refdes.clone(),
                value,
                footprint: c.footprint,
            };
            w.push(op)?;
        }
    }
    Ok(Value::Str(footprint))
}

fn dispatch(
    i: &mut Interp<'_>,
    a: &[Value],
    board: fn(&mut Interp<'_>, &[Value]) -> R,
    sch: fn(&mut Interp<'_>, &[Value]) -> R,
) -> R {
    if matches!(i.world, World::Schematic(_)) {
        sch(i, a)
    } else {
        board(i, a)
    }
}

fn f_move_to(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(i, a, board_move_to, crate::v2::funcs_sch::sch_move_to)
}

fn f_move_by(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(i, a, board_move_by, crate::v2::funcs_sch::sch_move_by)
}

fn f_rotate_by(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(i, a, board_rotate_by, crate::v2::funcs_sch::sch_rotate_by)
}

fn f_mirror(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(i, a, board_mirror, crate::v2::funcs_sch::sch_mirror)
}

fn f_rename(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(i, a, board_rename, crate::v2::funcs_sch::sch_rename)
}

fn f_set_value(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(
        i,
        a,
        board_set_value,
        crate::v2::funcs_sch::sch_set_value_fn,
    )
}

fn f_remove(i: &mut Interp<'_>, a: &[Value]) -> R {
    dispatch(i, a, board_remove, crate::v2::funcs_sch::sch_remove)
}

/// The operations of the board, and those the board and the schematic share.
pub(crate) static TABLE: &[FuncDef] = &[
    def("move_to", 3, 3, BOARD | SCHEMATIC, true, "move_to(target, x, y)", "Puts a component (or a part of the schematic) at a place; a component keeps its angle.", f_move_to),
    def("move_by", 3, 3, BOARD | SCHEMATIC, true, "move_by(target, dx, dy)", "Moves a component (or a part) by an offset.", f_move_by),
    def("rotate_to", 2, 2, BOARD, true, "rotate_to(component, angle)", "Sets the angle of a component, in micro-degrees (90deg).", f_rotate_to),
    def("rotate_by", 2, 2, BOARD | SCHEMATIC, true, "rotate_by(target, angle)", "Turns a component by an angle; a part of the schematic turns by whole quarter turns only.", f_rotate_by),
    def("mirror", 1, 1, BOARD | SCHEMATIC, true, "mirror(target)", "Puts a component on the other side of the board (its pads keep their nets; tracks do not follow), or mirrors a part.", f_mirror),
    def("rename", 2, 2, BOARD | SCHEMATIC, true, "rename(target, name)", "Renames a component, a part or a net; refused when the name is taken.", f_rename),
    def("set_value", 2, 2, BOARD | SCHEMATIC, true, "set_value(target, text)", "Sets the value of a component or a part.", f_set_value),
    def("remove", 1, 1, BOARD | SCHEMATIC, true, "remove(target)", "Removes a component, a track, a via or a zone (a part of the schematic).", f_remove),
    def("place", 4, 6, BOARD, true, "place(refdes, footprint, x, y[, angle[, value]])", "Places a new component of a footprint that the board has, on the top side; returns it.", f_place),
    def("route", 4, 4, BOARD, true, "route(net, layer, width, points)", "Draws a track of a net on a copper layer through points given as a flat list [x1, y1, x2, y2, ...]; the net is made when missing; returns the track.", f_route),
    def("via", 3, 5, BOARD, true, "via(net, x, y[, diameter[, drill]])", "Adds a via between the outer copper layers; the sizes default to the ones in the settings; returns it.", f_via),
    def("zone", 4, 4, BOARD, true, "zone(net, layer, points, clearance)", "Adds a copper zone of a net with a polygon given as a flat list of points; returns it.", f_zone),
    def("connect", 3, 3, BOARD, true, "connect(component, pad, net)", "Joins the pad with that number of a component to a net (made when missing).", f_connect),
    def("renumber", 3, 3, BOARD | SCHEMATIC, true, "renumber(list, prefix, first)", "Names the objects of a list prefix+first, prefix+first+1, ... in the order of the list, without a clash on the way; returns how many.", f_renumber),
    def("panelize", 3, 3, BOARD, true, "panelize(columns, rows, gap)", "Replaces the board by a matrix of copies with a gap between them, each designator ending with _1, _2, ... (the frame, the tabs and the tooling are the business of phoenix panel, which also makes the files). The run then has no batch that undoes it.", f_panelize),
    def("assign_part", 2, 2, BOARD | SCHEMATIC, true, "assign_part(target, part_id)", "Sets the value of a component or a part from a part of the parts base (--parts); returns the footprint the base names.", f_assign_part),
];
