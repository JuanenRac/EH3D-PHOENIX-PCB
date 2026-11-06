// SPDX-License-Identifier: GPL-3.0-or-later
//! The functions that ask the document questions: selecting objects by designator, net, layer,
//! rectangle and kind, reading their fields, sorting them, measuring distances and lengths and
//! counting the findings of the rule check.

use eh3d_phoenix_board::Side;
use eh3d_phoenix_erc::{check as erc_check, ErcConfig};
use eh3d_phoenix_rules::{check_board, Severity};

use crate::codes::Diag;
use crate::v2::codes::{ARITHMETIC, NOT_FOUND, REFUSED, TYPE_MISMATCH};
use crate::v2::funcs::{def, int, list, text, FuncDef};
use crate::v2::interp::Interp;
use crate::v2::value::{glob, isqrt, natural_key, Obj, ObjKind, Value};
use crate::v2::world::{World, BOARD, SCHEMATIC};

type R = Result<Value, Diag>;

fn count(n: usize) -> R {
    i64::try_from(n)
        .map(Value::Int)
        .map_err(|_| Diag::plain(ARITHMETIC, "the count does not fit 64 bits"))
}

/// The objects of a list; a list with anything else is a type error.
pub(crate) fn objects(args: &[Value], i: usize) -> Result<Vec<Obj>, Diag> {
    list(args, i)?
        .iter()
        .map(|v| match v {
            Value::Obj(o) => Ok(*o),
            other => Err(Diag::plain(
                TYPE_MISMATCH,
                format!(
                    "argument {} must hold objects, not {}",
                    i + 1,
                    other.type_name()
                ),
            )),
        })
        .collect()
}

fn as_values(objs: impl IntoIterator<Item = Obj>) -> Value {
    Value::List(objs.into_iter().map(Value::Obj).collect())
}

fn f_select(i: &mut Interp<'_>, a: &[Value]) -> R {
    let word = text(a, 0)?;
    let kind = ObjKind::from_word(word).ok_or_else(|| {
        Diag::plain(
            TYPE_MISMATCH,
            format!("{word:?} is not component, track, via, zone, net or part"),
        )
    })?;
    let pattern = text(a, 1)?;
    match &i.world {
        World::Board(b) => Ok(Value::List(b.select(kind, pattern))),
        World::Schematic(s) => Ok(Value::List(s.select(kind, pattern))),
        _ => Err(Diag::plain(NOT_FOUND, "there is no board or schematic")),
    }
}

fn f_field(i: &mut Interp<'_>, a: &[Value]) -> R {
    i.field_value(&a[0], text(a, 1)?)
}

fn f_exists(i: &mut Interp<'_>, a: &[Value]) -> R {
    let Value::Obj(o) = &a[0] else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(match &i.world {
        World::Board(b) => b.exists(o),
        World::Schematic(s) => s.exists(o),
        _ => false,
    }))
}

fn f_names(i: &mut Interp<'_>, a: &[Value]) -> R {
    let objs = objects(a, 0)?;
    Ok(Value::List(
        objs.iter().map(|o| Value::Str(i.describe(o))).collect(),
    ))
}

fn f_filter_kind(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let word = text(a, 1)?;
    let kind = ObjKind::from_word(word)
        .ok_or_else(|| Diag::plain(TYPE_MISMATCH, format!("{word:?} is not a kind of object")))?;
    Ok(as_values(
        objects(a, 0)?.into_iter().filter(|o| o.kind == kind),
    ))
}

fn f_filter_net(i: &mut Interp<'_>, a: &[Value]) -> R {
    let objs = objects(a, 0)?;
    let pattern = text(a, 1)?;
    let w = i.board()?;
    let b = &w.board;
    let name = |id: &eh3d_phoenix_board::Id| b.nets.get(id).map_or("", |n| n.name.as_str());
    let kept = objs.into_iter().filter(|o| match o.kind {
        ObjKind::Track => b
            .tracks
            .get(&o.id)
            .is_some_and(|t| glob(pattern, name(&t.net))),
        ObjKind::Via => b
            .vias
            .get(&o.id)
            .is_some_and(|v| glob(pattern, name(&v.net))),
        ObjKind::Zone => b
            .zones
            .get(&o.id)
            .is_some_and(|z| glob(pattern, name(&z.net))),
        ObjKind::Net => b.nets.get(&o.id).is_some_and(|n| glob(pattern, &n.name)),
        ObjKind::Component => b
            .pad_nets
            .iter()
            .any(|p| p.component == o.id && glob(pattern, name(&p.net))),
        ObjKind::Part => false,
    });
    Ok(as_values(kept))
}

fn f_filter_layer(i: &mut Interp<'_>, a: &[Value]) -> R {
    let objs = objects(a, 0)?;
    let pattern = text(a, 1)?.to_ascii_lowercase();
    let w = i.board()?;
    let b = &w.board;
    let layer = |id: &eh3d_phoenix_board::Id| {
        b.layers
            .get(id)
            .is_some_and(|l| glob(&pattern, &l.name.to_ascii_lowercase()))
    };
    let kept = objs.into_iter().filter(|o| match o.kind {
        ObjKind::Track => b.tracks.get(&o.id).is_some_and(|t| layer(&t.layer)),
        ObjKind::Zone => b.zones.get(&o.id).is_some_and(|z| layer(&z.layer)),
        ObjKind::Via => b
            .vias
            .get(&o.id)
            .is_some_and(|v| layer(&v.from_layer) || layer(&v.to_layer)),
        ObjKind::Component => b.components.get(&o.id).is_some_and(|c| {
            let side = match c.side {
                Side::Top => "top",
                Side::Bottom => "bottom",
            };
            glob(&pattern, side)
        }),
        _ => false,
    });
    Ok(as_values(kept))
}

fn f_filter_value(i: &mut Interp<'_>, a: &[Value]) -> R {
    let objs = objects(a, 0)?;
    let pattern = text(a, 1)?;
    let kept: Vec<Obj> = match &i.world {
        World::Board(w) => objs
            .into_iter()
            .filter(|o| {
                w.board
                    .components
                    .get(&o.id)
                    .is_some_and(|c| o.kind == ObjKind::Component && glob(pattern, &c.value))
            })
            .collect(),
        World::Schematic(w) => objs
            .into_iter()
            .filter(|o| {
                w.schematic()
                    .parts
                    .get(&o.id)
                    .is_some_and(|p| o.kind == ObjKind::Part && glob(pattern, &p.value))
            })
            .collect(),
        _ => Vec::new(),
    };
    Ok(as_values(kept))
}

fn f_filter_rect(i: &mut Interp<'_>, a: &[Value]) -> R {
    let objs = objects(a, 0)?;
    let (x1, y1, x2, y2) = (int(a, 1)?, int(a, 2)?, int(a, 3)?, int(a, 4)?);
    let (lx, hx) = (x1.min(x2), x1.max(x2));
    let (ly, hy) = (y1.min(y2), y1.max(y2));
    let inside = |p: eh3d_phoenix_model::Point| {
        (lx..=hx).contains(&p.x_nm.get()) && (ly..=hy).contains(&p.y_nm.get())
    };
    let mut kept = Vec::new();
    for o in objs {
        let hit =
            match &i.world {
                World::Board(w) => match o.kind {
                    ObjKind::Track => w.board.tracks.get(&o.id).is_some_and(|t| {
                        t.path.iter().any(|s| inside(s.start()) || inside(s.end()))
                    }),
                    ObjKind::Zone => w
                        .board
                        .zones
                        .get(&o.id)
                        .is_some_and(|z| z.outline.iter().any(|p| inside(*p))),
                    _ => w.position(&o).is_some_and(inside),
                },
                World::Schematic(w) => w.position(&o).is_some_and(inside),
                _ => false,
            };
        if hit {
            kept.push(o);
        }
    }
    Ok(as_values(kept))
}

fn f_sort_refdes(i: &mut Interp<'_>, a: &[Value]) -> R {
    let mut objs = objects(a, 0)?;
    objs.sort_by_cached_key(|o| natural_key(&i.describe(o)));
    Ok(as_values(objs))
}

fn f_sort_position(i: &mut Interp<'_>, a: &[Value]) -> R {
    let mut objs = objects(a, 0)?;
    let order = text(a, 1)?;
    if !matches!(order, "xy" | "yx") {
        return Err(Diag::plain(
            TYPE_MISMATCH,
            "the order is xy (by x, then y) or yx (by y, then x)",
        ));
    }
    objs.sort_by_cached_key(|o| {
        let (x, y) = match &i.world {
            World::Board(w) => w.position(o),
            World::Schematic(w) => w.position(o),
            _ => None,
        }
        .map_or((0, 0), |p| (p.x_nm.get(), p.y_nm.get()));
        let key = if order == "xy" { (x, y) } else { (y, x) };
        (key, natural_key(&i.describe(o)))
    });
    Ok(as_values(objs))
}

fn position_of(i: &Interp<'_>, v: &Value) -> Result<eh3d_phoenix_model::Point, Diag> {
    let Value::Obj(o) = v else {
        return Err(Diag::plain(
            TYPE_MISMATCH,
            format!("a place needs an object, not {}", v.type_name()),
        ));
    };
    match &i.world {
        World::Board(w) => w.position(o),
        World::Schematic(w) => w.position(o),
        _ => None,
    }
    .ok_or_else(|| Diag::plain(NOT_FOUND, "the object has no place"))
}

fn f_distance(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (p, q) = (position_of(i, &a[0])?, position_of(i, &a[1])?);
    Ok(Value::Int(
        i64::try_from(isqrt(p.distance_squared(q))).unwrap_or(i64::MAX),
    ))
}

fn f_nearest(i: &mut Interp<'_>, a: &[Value]) -> R {
    let from = position_of(i, &a[0])?;
    let mut best: Option<(i128, Obj)> = None;
    for o in objects(a, 1)? {
        if a[0] == Value::Obj(o) {
            continue;
        }
        let p = position_of(i, &Value::Obj(o))?;
        let d = from.distance_squared(p);
        let better = match best {
            None => true,
            Some((best_distance, _)) => d < best_distance,
        };
        if better {
            best = Some((d, o));
        }
    }
    Ok(best.map_or(Value::Nil, |(_, o)| Value::Obj(o)))
}

fn f_pad_nets(i: &mut Interp<'_>, a: &[Value]) -> R {
    let Value::Obj(o) = &a[0] else {
        return Err(Diag::plain(TYPE_MISMATCH, "pad_nets takes a component"));
    };
    let w = i.board()?;
    let mut names: Vec<String> = w
        .board
        .pad_nets
        .iter()
        .filter(|p| p.component == o.id)
        .filter_map(|p| w.board.nets.get(&p.net).map(|n| n.name.clone()))
        .collect();
    names.sort();
    names.dedup();
    Ok(Value::List(names.into_iter().map(Value::Str).collect()))
}

fn f_net_length(i: &mut Interp<'_>, a: &[Value]) -> R {
    let name = text(a, 0)?;
    let w = i.board()?;
    let id = w
        .net_named(name)
        .ok_or_else(|| Diag::plain(NOT_FOUND, format!("the board has no net {name}")))?;
    Ok(Value::Int(w.net_length(&id)))
}

fn f_outline_bounds(i: &mut Interp<'_>, _a: &[Value]) -> R {
    let w = i.board()?;
    let xs: Vec<i64> = w.board.outline.iter().map(|p| p.x_nm.get()).collect();
    let ys: Vec<i64> = w.board.outline.iter().map(|p| p.y_nm.get()).collect();
    let (Some(x0), Some(x1), Some(y0), Some(y1)) = (
        xs.iter().min(),
        xs.iter().max(),
        ys.iter().min(),
        ys.iter().max(),
    ) else {
        return Err(Diag::plain(NOT_FOUND, "the board has no outline"));
    };
    Ok(Value::List(vec![
        Value::Int(*x0),
        Value::Int(*y0),
        Value::Int(*x1),
        Value::Int(*y1),
    ]))
}

fn f_layers(i: &mut Interp<'_>, _a: &[Value]) -> R {
    let w = i.board()?;
    Ok(Value::List(
        w.board
            .layers
            .values()
            .map(|l| Value::Str(l.name.clone()))
            .collect(),
    ))
}

fn f_drc_count(i: &mut Interp<'_>, _a: &[Value]) -> R {
    let ctx = i.ctx;
    let w = i.board()?;
    count(check_board(&w.board, &ctx.rules).confirmed.len())
}

fn f_drc_errors(i: &mut Interp<'_>, _a: &[Value]) -> R {
    let ctx = i.ctx;
    let w = i.board()?;
    count(check_board(&w.board, &ctx.rules).count(Severity::Error))
}

fn f_erc_count(i: &mut Interp<'_>, _a: &[Value]) -> R {
    let w = i.sch()?;
    count(
        erc_check(w.schematic(), &ErcConfig::default())
            .confirmed
            .len(),
    )
}

fn f_ratsnest_count(i: &mut Interp<'_>, _a: &[Value]) -> R {
    let ctx = i.ctx;
    let w = i.board()?;
    let lines = eh3d_phoenix_route::ratsnest(&w.board, &ctx.route)
        .map_err(|e| Diag::plain(REFUSED, e.message(ctx.lang)))?;
    count(lines.len())
}

/// The functions that read the board or the schematic.
pub(crate) static TABLE: &[FuncDef] = &[
    def("select", 2, 2, BOARD | SCHEMATIC, false, "select(kind, pattern)", "The objects of a kind (component, track, via, zone, net, part) whose designator or net matches the pattern.", f_select),
    def("field", 2, 2, BOARD | SCHEMATIC, false, "field(object, name)", "A field of an object, also written object.name: refdes, value, footprint, x, y, rot, side, pads of a component; net, layer, width, length of a track; net, x, y, drill, diameter of a via; name, tracks, vias, pads, length of a net; refdes, value, symbol, x, y, rot, pins of a part.", f_field),
    def("exists", 1, 1, BOARD | SCHEMATIC, false, "exists(object)", "Whether the object is still in the document.", f_exists),
    def("names", 1, 1, BOARD | SCHEMATIC, false, "names(list)", "The designators or net names of a list of objects.", f_names),
    def("filter_kind", 2, 2, BOARD | SCHEMATIC, false, "filter_kind(list, kind)", "The objects of a list that are of a kind.", f_filter_kind),
    def("filter_net", 2, 2, BOARD, false, "filter_net(list, pattern)", "The objects of a list on a net that matches the pattern (a component when one of its pads is on it).", f_filter_net),
    def("filter_layer", 2, 2, BOARD, false, "filter_layer(list, pattern)", "The tracks and zones on a layer that matches the pattern, the vias that start or end on it, the components on that side (top or bottom).", f_filter_layer),
    def("filter_value", 2, 2, BOARD | SCHEMATIC, false, "filter_value(list, pattern)", "The components or parts whose value matches the pattern.", f_filter_value),
    def("filter_rect", 5, 5, BOARD | SCHEMATIC, false, "filter_rect(list, x1, y1, x2, y2)", "The objects with a place inside the rectangle (a track or a zone when one of its points is inside).", f_filter_rect),
    def("sort_refdes", 1, 1, BOARD | SCHEMATIC, false, "sort_refdes(list)", "The objects by name the way a person sorts them (C2 before C10).", f_sort_refdes),
    def("sort_position", 2, 2, BOARD | SCHEMATIC, false, "sort_position(list, order)", "The objects by place: xy (by x, then y) or yx (by y, then x).", f_sort_position),
    def("distance", 2, 2, BOARD | SCHEMATIC, false, "distance(a, b)", "The distance in nanometres between the places of two objects (components, vias or parts), rounded down.", f_distance),
    def("nearest", 2, 2, BOARD | SCHEMATIC, false, "nearest(object, list)", "The object of the list closest to the object (nil for an empty list); the first one on a tie.", f_nearest),
    def("pad_nets", 1, 1, BOARD, false, "pad_nets(component)", "The names of the nets on the pads of a component, sorted.", f_pad_nets),
    def("net_length", 1, 1, BOARD, false, "net_length(name)", "The copper length of a net in nanometres; an arc is counted by its chord.", f_net_length),
    def("outline_bounds", 0, 0, BOARD, false, "outline_bounds()", "The box of the board outline as [x1, y1, x2, y2].", f_outline_bounds),
    def("layers", 0, 0, BOARD, false, "layers()", "The names of the layers of the board.", f_layers),
    def("drc_count", 0, 0, BOARD, false, "drc_count()", "The findings the rule check proves on the board now, with the rules of the run.", f_drc_count),
    def("drc_errors", 0, 0, BOARD, false, "drc_errors()", "The proven findings of the rule check that are errors.", f_drc_errors),
    def("erc_count", 0, 0, SCHEMATIC, false, "erc_count()", "The findings the electrical rule check proves on the schematic now.", f_erc_count),
    def("ratsnest_count", 0, 0, BOARD, false, "ratsnest_count()", "The connections still to be routed.", f_ratsnest_count),
];
