//! What each operation of [`crate::edit`] does to a schematic, and the tidying of the drawing that
//! follows an operation that touches the wiring. Everything here works on a copy: when it returns
//! an error the caller throws the copy away.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_model::{Point, Udeg};

use crate::draw::{inside_piece, key, turn, Placement, Sheet, Wire};
use crate::edit::{
    clean_text, Clip, EditError, EditOp, IdSource, Item, SymbolDef, MAX_BATCH, MAX_DEPTH,
};
use crate::marks::{bus_members, Junction, NetLabel, PowerPort};
use crate::model::{Id, Part, Pin, Schematic, Symbol};

/// The most points of one wire and the most pins of one symbol that an operation accepts.
const MAX_POINTS: usize = 10_000;

/// The default distance between the labels of a bus, in nanometres (2.54 mm).
const BUS_STEP_NM: i64 = 2_540_000;

type PinTargets = BTreeMap<(Id, (i64, i64)), Point>;

/// The items an operation names, by kind.
#[derive(Default)]
struct Selection {
    parts: BTreeSet<Id>,
    wires: BTreeSet<Id>,
    labels: BTreeSet<Id>,
    ports: BTreeSet<Id>,
    junctions: BTreeSet<Id>,
}

fn select(s: &Schematic, items: &[Item]) -> Result<Selection, EditError> {
    let mut sel = Selection::default();
    for item in items {
        match item {
            Item::Part(id) => {
                if !s.parts.contains_key(id) {
                    return Err(EditError::NoSuchPart(*id));
                }
                sel.parts.insert(*id);
            }
            Item::Wire(id) => {
                if !s.wires.contains_key(id) {
                    return Err(EditError::NoSuchItem(*id));
                }
                sel.wires.insert(*id);
            }
            Item::Label(id) => {
                if !s.labels.contains_key(id) {
                    return Err(EditError::NoSuchItem(*id));
                }
                sel.labels.insert(*id);
            }
            Item::PowerPort(id) => {
                if !s.power_ports.contains_key(id) {
                    return Err(EditError::NoSuchItem(*id));
                }
                sel.ports.insert(*id);
            }
            Item::Junction(id) => {
                if !s.junctions.contains_key(id) {
                    return Err(EditError::NoSuchItem(*id));
                }
                sel.junctions.insert(*id);
            }
        }
    }
    Ok(sel)
}

fn shift(p: Point, dx: i64, dy: i64) -> Result<Point, EditError> {
    let x = p.x_nm.get().checked_add(dx).ok_or(EditError::OutOfRange)?;
    let y = p.y_nm.get().checked_add(dy).ok_or(EditError::OutOfRange)?;
    Point::from_nm(x, y).map_err(|_| EditError::OutOfRange)
}

/// Where the pins of a placed part are: the pin and the point. Nothing for a part that is not
/// placed or whose symbol is gone; an error when a pin would leave the coordinate range or the
/// rotation is not a multiple of 90 degrees.
pub(crate) fn pin_points(s: &Schematic, part_id: Id) -> Result<Vec<(Id, Point)>, EditError> {
    let part = s
        .parts
        .get(&part_id)
        .ok_or(EditError::NoSuchPart(part_id))?;
    let Some(place) = part.placement else {
        return Ok(Vec::new());
    };
    let Some(symbol) = s.symbols.get(&part.symbol) else {
        return Ok(Vec::new());
    };
    let mirrored = s.mirrored.contains(&part_id);
    let mut out = Vec::new();
    for (pin_id, pin) in &symbol.pins {
        let Some(offset) = pin.at else {
            continue;
        };
        let ox = if mirrored {
            offset
                .x_nm
                .get()
                .checked_neg()
                .ok_or(EditError::OutOfRange)?
        } else {
            offset.x_nm.get()
        };
        let (dx, dy) = turn(ox, offset.y_nm.get(), place.rotation).ok_or(EditError::OutOfRange)?;
        out.push((*pin_id, shift(place.at, dx, dy)?));
    }
    Ok(out)
}

fn refdes_used(s: &Schematic, refdes: &str, except: Option<Id>) -> bool {
    s.parts
        .iter()
        .any(|(id, p)| Some(*id) != except && p.refdes == refdes)
}

fn next_refdes(s: &Schematic, prefix: &str) -> String {
    let mut n = 1usize;
    loop {
        let candidate = format!("{prefix}{n}");
        if !refdes_used(s, &candidate, None) {
            return candidate;
        }
        n += 1;
    }
}

/// The letters of a reference without its number (`R12` gives `R`); `U` when there are none.
fn refdes_prefix(refdes: &str) -> String {
    let letters = refdes.trim().trim_end_matches(|c: char| c.is_ascii_digit());
    if letters.is_empty() || letters.chars().any(char::is_control) {
        "U".to_owned()
    } else {
        letters.to_owned()
    }
}

/// The reference prefix a new part of a symbol gets: its first letter in capitals.
fn symbol_prefix(name: &str) -> String {
    name.chars()
        .find(char::is_ascii_alphabetic)
        .map_or_else(|| "U".to_owned(), |c| c.to_ascii_uppercase().to_string())
}

fn define_symbol(s: &mut Schematic, ids: &mut IdSource, def: &SymbolDef) -> Result<Id, EditError> {
    let name = clean_text(&def.name, false, "the name of the symbol")?;
    if def.pins.is_empty() || def.pins.len() > MAX_POINTS {
        return Err(EditError::BadSymbol("a symbol needs 1 to 10000 pins"));
    }
    let mut numbers: BTreeSet<String> = BTreeSet::new();
    let mut pins: BTreeMap<Id, Pin> = BTreeMap::new();
    let mut kinds = Vec::new();
    for p in &def.pins {
        let number = clean_text(&p.number, false, "the number of a pin")?;
        if !numbers.insert(number.clone()) {
            return Err(EditError::BadSymbol("two pins have the same number"));
        }
        let pin_id = ids.fresh()?;
        pins.insert(
            pin_id,
            Pin {
                number,
                at: Some(p.at),
            },
        );
        if let Some(kind) = p.kind {
            kinds.push((pin_id, kind));
        }
    }
    let id = ids.fresh()?;
    s.symbols.insert(id, Symbol { name, pins });
    s.pin_kinds.extend(kinds);
    Ok(id)
}

fn resolve_symbol(
    s: &mut Schematic,
    ids: &mut IdSource,
    spec: &str,
    define: Option<&SymbolDef>,
) -> Result<Id, EditError> {
    let spec = spec.trim();
    if let Ok(id) = spec.parse::<Id>() {
        if s.symbols.contains_key(&id) {
            return Ok(id);
        }
    }
    if let Some((id, _)) = s.symbols.iter().find(|(_, symbol)| symbol.name == spec) {
        return Ok(*id);
    }
    match define {
        Some(def) => define_symbol(s, ids, def),
        None => Err(EditError::NoSuchSymbol(spec.to_owned())),
    }
}

fn check_sheet(s: &Schematic, sheet: Id) -> Result<(), EditError> {
    if s.sheets.contains_key(&sheet) {
        Ok(())
    } else {
        Err(EditError::NoSuchSheet(sheet))
    }
}

fn check_path(path: &[Point]) -> Result<(), EditError> {
    if path.len() < 2 {
        return Err(EditError::BadWire("a wire needs two points"));
    }
    if path.len() > MAX_POINTS {
        return Err(EditError::BadWire("a wire has too many points"));
    }
    for w in path.windows(2) {
        if w[0] == w[1] {
            return Err(EditError::BadWire("two equal points in a row"));
        }
        if w[0].x_nm != w[1].x_nm && w[0].y_nm != w[1].y_nm {
            return Err(EditError::BadWire("a piece is not horizontal or vertical"));
        }
    }
    Ok(())
}

/// Runs one operation on `s`. `wiring` is set when the operation may change what the drawing joins.
pub(crate) fn run(
    s: &mut Schematic,
    ids: &mut IdSource,
    op: &EditOp,
    wiring: &mut bool,
    depth: usize,
) -> Result<(), EditError> {
    match op {
        EditOp::AddSheet {
            name,
            width,
            height,
        } => {
            let name = clean_text(name, false, "the name of the sheet")?;
            if width.get() <= 0 || height.get() <= 0 {
                return Err(EditError::OutOfRange);
            }
            let id = ids.fresh()?;
            s.sheets.insert(
                id,
                Sheet {
                    name,
                    width: *width,
                    height: *height,
                },
            );
        }
        EditOp::PlaceSymbol {
            symbol,
            define,
            sheet,
            at,
            rotation,
            mirrored,
            refdes,
            value,
        } => {
            check_sheet(s, *sheet)?;
            let symbol_id = resolve_symbol(s, ids, symbol, define.as_ref())?;
            let refdes = match refdes {
                Some(r) => {
                    let r = clean_text(r, false, "the reference")?;
                    if refdes_used(s, &r, None) {
                        return Err(EditError::DuplicateRefdes(r));
                    }
                    r
                }
                None => {
                    let name = s.symbols.get(&symbol_id).map_or("", |y| y.name.as_str());
                    next_refdes(s, &symbol_prefix(name))
                }
            };
            let value = match value {
                Some(v) => clean_text(v, true, "the value")?,
                None => String::new(),
            };
            let part_id = ids.fresh()?;
            s.parts.insert(
                part_id,
                Part {
                    refdes,
                    value,
                    symbol: symbol_id,
                    footprint: None,
                    placement: Some(Placement {
                        sheet: *sheet,
                        at: *at,
                        rotation: rotation.unwrap_or(Udeg::ZERO),
                    }),
                },
            );
            if *mirrored {
                s.mirrored.insert(part_id);
            }
            pin_points(s, part_id)?;
            *wiring = true;
        }
        EditOp::MoveItems {
            items,
            dx,
            dy,
            detach,
        } => {
            move_items(s, items, *dx, *dy, *detach)?;
            *wiring = true;
        }
        EditOp::RotateParts {
            parts,
            quarter_turns,
        } => {
            let turns = i64::from(quarter_turns.rem_euclid(4));
            reposition(s, parts, |s, id| {
                if let Some(place) = s.parts.get_mut(&id).and_then(|p| p.placement.as_mut()) {
                    let turned =
                        (place.rotation.get() + turns * 90_000_000).rem_euclid(360_000_000);
                    place.rotation = Udeg::new(turned).map_err(|_| EditError::OutOfRange)?;
                }
                Ok(())
            })?;
            *wiring = true;
        }
        EditOp::MirrorParts { parts } => {
            reposition(s, parts, |s, id| {
                if !s.mirrored.remove(&id) {
                    s.mirrored.insert(id);
                }
                Ok(())
            })?;
            *wiring = true;
        }
        EditOp::Delete { items } => {
            let sel = select(s, items)?;
            for id in &sel.parts {
                s.parts.remove(id);
                s.mirrored.remove(id);
            }
            for id in &sel.wires {
                s.wires.remove(id);
            }
            for id in &sel.labels {
                s.labels.remove(id);
            }
            for id in &sel.ports {
                s.power_ports.remove(id);
            }
            for id in &sel.junctions {
                s.junctions.remove(id);
            }
            *wiring = true;
        }
        EditOp::Paste {
            clip,
            sheet,
            dx,
            dy,
        } => {
            paste(s, ids, clip, *sheet, *dx, *dy)?;
            *wiring = true;
        }
        EditOp::SetValue { part, value } => {
            let value = clean_text(value, true, "the value")?;
            let part = s.parts.get_mut(part).ok_or(EditError::NoSuchPart(*part))?;
            part.value = value;
        }
        EditOp::SetRefdes { part, refdes } => {
            let refdes = clean_text(refdes, false, "the reference")?;
            if !s.parts.contains_key(part) {
                return Err(EditError::NoSuchPart(*part));
            }
            if refdes_used(s, &refdes, Some(*part)) {
                return Err(EditError::DuplicateRefdes(refdes));
            }
            if let Some(p) = s.parts.get_mut(part) {
                p.refdes = refdes;
            }
        }
        EditOp::AddWire { sheet, path } => {
            check_sheet(s, *sheet)?;
            check_path(path)?;
            let id = ids.fresh()?;
            s.wires.insert(
                id,
                Wire {
                    sheet: *sheet,
                    path: path.clone(),
                },
            );
            *wiring = true;
        }
        EditOp::AddLabel { sheet, at, name } => {
            check_sheet(s, *sheet)?;
            let name = clean_text(name, false, "the name of a label")?;
            let id = ids.fresh()?;
            s.labels.insert(
                id,
                NetLabel {
                    sheet: *sheet,
                    at: *at,
                    name,
                },
            );
            *wiring = true;
        }
        EditOp::AddPowerPort { sheet, at, name } => {
            check_sheet(s, *sheet)?;
            let name = clean_text(name, false, "the name of a power port")?;
            let id = ids.fresh()?;
            s.power_ports.insert(
                id,
                PowerPort {
                    sheet: *sheet,
                    at: *at,
                    name,
                },
            );
            *wiring = true;
        }
        EditOp::AddBus {
            sheet,
            at,
            spec,
            dx,
            dy,
        } => {
            check_sheet(s, *sheet)?;
            let members = bus_members(spec).ok_or_else(|| EditError::BadBus(spec.clone()))?;
            let (dx, dy) = if *dx == 0 && *dy == 0 {
                (0, BUS_STEP_NM)
            } else {
                (*dx, *dy)
            };
            let mut here = *at;
            for member in members {
                let name = clean_text(&member, false, "the name of a bus member")?;
                let id = ids.fresh()?;
                s.labels.insert(
                    id,
                    NetLabel {
                        sheet: *sheet,
                        at: here,
                        name,
                    },
                );
                here = shift(here, dx, dy)?;
            }
            *wiring = true;
        }
        EditOp::RenameMark { id, name } => {
            let name = clean_text(name, false, "the name")?;
            if let Some(label) = s.labels.get_mut(id) {
                label.name = name;
            } else if let Some(port) = s.power_ports.get_mut(id) {
                port.name = name;
            } else {
                return Err(EditError::NoSuchItem(*id));
            }
            *wiring = true;
        }
        EditOp::Batch { ops } => {
            if depth >= MAX_DEPTH || ops.len() > MAX_BATCH {
                return Err(EditError::TooMany);
            }
            for op in ops {
                run(s, ids, op, wiring, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn move_items(
    s: &mut Schematic,
    items: &[Item],
    dx: i64,
    dy: i64,
    detach: bool,
) -> Result<(), EditError> {
    let sel = select(s, items)?;
    let mut remap: PinTargets = BTreeMap::new();
    for pid in &sel.parts {
        let old = pin_points(s, *pid)?;
        let mut sheet = None;
        if let Some(place) = s.parts.get_mut(pid).and_then(|p| p.placement.as_mut()) {
            place.at = shift(place.at, dx, dy)?;
            sheet = Some(place.sheet);
        }
        let new = pin_points(s, *pid)?;
        if let Some(sheet) = sheet {
            for ((_, a), (_, b)) in old.iter().zip(new.iter()) {
                remap.entry((sheet, key(*a))).or_insert(*b);
            }
        }
    }
    for id in &sel.wires {
        if let Some(wire) = s.wires.get_mut(id) {
            for p in wire.path.iter_mut() {
                *p = shift(*p, dx, dy)?;
            }
        }
    }
    for id in &sel.labels {
        if let Some(label) = s.labels.get_mut(id) {
            label.at = shift(label.at, dx, dy)?;
        }
    }
    for id in &sel.ports {
        if let Some(port) = s.power_ports.get_mut(id) {
            port.at = shift(port.at, dx, dy)?;
        }
    }
    for id in &sel.junctions {
        if let Some(junction) = s.junctions.get_mut(id) {
            junction.at = shift(junction.at, dx, dy)?;
        }
    }
    if !detach {
        follow(s, &remap, &sel.wires, &sel.labels, &sel.ports);
    }
    Ok(())
}

/// Changes parts one by one and makes the wires that ended on their pins follow the pins.
fn reposition(
    s: &mut Schematic,
    parts: &[Id],
    mut change: impl FnMut(&mut Schematic, Id) -> Result<(), EditError>,
) -> Result<(), EditError> {
    let set: BTreeSet<Id> = parts.iter().copied().collect();
    let mut remap: PinTargets = BTreeMap::new();
    for pid in set {
        if !s.parts.contains_key(&pid) {
            return Err(EditError::NoSuchPart(pid));
        }
        let old = pin_points(s, pid)?;
        change(s, pid)?;
        let new = pin_points(s, pid)?;
        let sheet = s
            .parts
            .get(&pid)
            .and_then(|p| p.placement)
            .map(|place| place.sheet);
        if let Some(sheet) = sheet {
            for ((_, a), (_, b)) in old.iter().zip(new.iter()) {
                if a != b {
                    remap.entry((sheet, key(*a))).or_insert(*b);
                }
            }
        }
    }
    let none: BTreeSet<Id> = BTreeSet::new();
    follow(s, &remap, &none, &none, &none);
    Ok(())
}

/// Moves the points of wires, labels and power ports that sat where a pin was to where the pin is
/// now. A piece that was horizontal or vertical stays so: a bend point is added where needed.
fn follow(
    s: &mut Schematic,
    remap: &PinTargets,
    skip_wires: &BTreeSet<Id>,
    skip_labels: &BTreeSet<Id>,
    skip_ports: &BTreeSet<Id>,
) {
    if remap.is_empty() {
        return;
    }
    let wire_ids: Vec<Id> = s.wires.keys().copied().collect();
    let mut dead: Vec<Id> = Vec::new();
    for wid in wire_ids {
        if skip_wires.contains(&wid) {
            continue;
        }
        let Some(wire) = s.wires.get_mut(&wid) else {
            continue;
        };
        let moved: Vec<Option<Point>> = wire
            .path
            .iter()
            .map(|p| remap.get(&(wire.sheet, key(*p))).copied())
            .collect();
        if moved.iter().all(Option::is_none) {
            continue;
        }
        let path = restitch(&wire.path, &moved);
        if path.len() < 2 {
            dead.push(wid);
        } else {
            wire.path = path;
        }
    }
    for wid in dead {
        s.wires.remove(&wid);
    }
    for (id, label) in s.labels.iter_mut() {
        if skip_labels.contains(id) {
            continue;
        }
        if let Some(p) = remap.get(&(label.sheet, key(label.at))) {
            label.at = *p;
        }
    }
    for (id, port) in s.power_ports.iter_mut() {
        if skip_ports.contains(id) {
            continue;
        }
        if let Some(p) = remap.get(&(port.sheet, key(port.at))) {
            port.at = *p;
        }
    }
}

/// The path with the points in `moved` replaced; where a horizontal or vertical piece would
/// become slanted a bend point is added, and points that repeat are dropped.
fn restitch(path: &[Point], moved: &[Option<Point>]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(path.len() + 2);
    let mut prev_now: Option<Point> = None;
    for (i, old) in path.iter().enumerate() {
        let now = moved.get(i).copied().flatten().unwrap_or(*old);
        if let (Some(a_now), Some(a_old)) = (prev_now, i.checked_sub(1).and_then(|j| path.get(j))) {
            let a_changed = a_now != *a_old;
            let b_changed = now != *old;
            let slanted = a_now.x_nm != now.x_nm && a_now.y_nm != now.y_nm;
            if (a_changed || b_changed) && slanted {
                let only_a = a_changed && !b_changed;
                if a_old.y_nm == old.y_nm {
                    // the piece was horizontal
                    out.push(if only_a {
                        Point {
                            x_nm: a_now.x_nm,
                            y_nm: now.y_nm,
                        }
                    } else {
                        Point {
                            x_nm: now.x_nm,
                            y_nm: a_now.y_nm,
                        }
                    });
                } else if a_old.x_nm == old.x_nm {
                    // the piece was vertical
                    out.push(if only_a {
                        Point {
                            x_nm: now.x_nm,
                            y_nm: a_now.y_nm,
                        }
                    } else {
                        Point {
                            x_nm: a_now.x_nm,
                            y_nm: now.y_nm,
                        }
                    });
                }
            }
        }
        out.push(now);
        prev_now = Some(now);
    }
    out.dedup();
    out
}

fn paste(
    s: &mut Schematic,
    ids: &mut IdSource,
    clip: &Clip,
    sheet: Id,
    dx: i64,
    dy: i64,
) -> Result<(), EditError> {
    check_sheet(s, sheet)?;
    for cp in &clip.parts {
        if !s.symbols.contains_key(&cp.symbol) {
            return Err(EditError::NoSuchSymbol(cp.symbol.to_string()));
        }
        let at = shift(cp.at, dx, dy)?;
        let id = ids.fresh()?;
        let refdes = next_refdes(s, &refdes_prefix(&cp.refdes));
        let value = clean_text(&cp.value, true, "the value")?;
        s.parts.insert(
            id,
            Part {
                refdes,
                value,
                symbol: cp.symbol,
                footprint: cp.footprint,
                placement: Some(Placement {
                    sheet,
                    at,
                    rotation: cp.rotation,
                }),
            },
        );
        if cp.mirrored {
            s.mirrored.insert(id);
        }
        pin_points(s, id)?;
    }
    for path in &clip.wires {
        let moved: Result<Vec<Point>, EditError> = path.iter().map(|p| shift(*p, dx, dy)).collect();
        let moved = moved?;
        check_path(&moved)?;
        let id = ids.fresh()?;
        s.wires.insert(id, Wire { sheet, path: moved });
    }
    for mark in &clip.labels {
        let name = clean_text(&mark.name, false, "the name of a label")?;
        let at = shift(mark.at, dx, dy)?;
        let id = ids.fresh()?;
        s.labels.insert(id, NetLabel { sheet, at, name });
    }
    for mark in &clip.power_ports {
        let name = clean_text(&mark.name, false, "the name of a power port")?;
        let at = shift(mark.at, dx, dy)?;
        let id = ids.fresh()?;
        s.power_ports.insert(id, PowerPort { sheet, at, name });
    }
    Ok(())
}

/// Tidies the drawing after an operation: a wire gets a point where another wire's point, a pin, a
/// net label or a power port lies in the middle of one of its pieces, so they are joined; and the
/// junction dots are exactly at the points where three or more pieces and pins meet.
pub(crate) fn tidy(s: &mut Schematic, ids: &mut IdSource) -> Result<(), EditError> {
    split_wires(s);
    junctions(s, ids)
}

fn split_wires(s: &mut Schematic) {
    // (sheet, wire that owns the point, point)
    let mut candidates: Vec<(Id, Option<Id>, Point)> = Vec::new();
    for (wid, wire) in &s.wires {
        for p in &wire.path {
            candidates.push((wire.sheet, Some(*wid), *p));
        }
    }
    for (_, sheet, p) in s.pin_positions() {
        candidates.push((sheet, None, p));
    }
    for label in s.labels.values() {
        candidates.push((label.sheet, None, label.at));
    }
    for port in s.power_ports.values() {
        candidates.push((port.sheet, None, port.at));
    }
    let wire_ids: Vec<Id> = s.wires.keys().copied().collect();
    for wid in wire_ids {
        let Some(wire) = s.wires.get(&wid) else {
            continue;
        };
        let mut path: Vec<Point> = Vec::with_capacity(wire.path.len());
        let mut changed = false;
        for (i, a) in wire.path.iter().enumerate() {
            path.push(*a);
            let Some(b) = wire.path.get(i + 1) else {
                continue;
            };
            let mut found: Vec<Point> = candidates
                .iter()
                .filter(|(sheet, owner, p)| {
                    *sheet == wire.sheet && *owner != Some(wid) && inside_piece(*p, *a, *b)
                })
                .map(|(_, _, p)| *p)
                .collect();
            found.sort_by_key(|p| (a.distance_squared(*p), key(*p)));
            found.dedup();
            if !found.is_empty() {
                changed = true;
                path.extend(found);
            }
        }
        if changed {
            if let Some(wire) = s.wires.get_mut(&wid) {
                wire.path = path;
            }
        }
    }
}

fn junctions(s: &mut Schematic, ids: &mut IdSource) -> Result<(), EditError> {
    // how many pieces and pins meet at each point
    let mut arms: BTreeMap<(Id, (i64, i64)), (u32, Point)> = BTreeMap::new();
    for wire in s.wires.values() {
        if !s.sheets.contains_key(&wire.sheet) {
            continue;
        }
        let last = wire.path.len().saturating_sub(1);
        for (i, p) in wire.path.iter().enumerate() {
            let entry = arms.entry((wire.sheet, key(*p))).or_insert((0, *p));
            entry.0 += if i == 0 || i == last { 1 } else { 2 };
        }
    }
    for (_, sheet, p) in s.pin_positions() {
        arms.entry((sheet, key(p))).or_insert((0, p)).0 += 1;
    }
    let need: BTreeMap<(Id, (i64, i64)), Point> = arms
        .into_iter()
        .filter(|(_, (count, _))| *count >= 3)
        .map(|(k, (_, p))| (k, p))
        .collect();
    let mut have: BTreeSet<(Id, (i64, i64))> = BTreeSet::new();
    let mut remove: Vec<Id> = Vec::new();
    for (jid, junction) in &s.junctions {
        let k = (junction.sheet, key(junction.at));
        if need.contains_key(&k) && have.insert(k) {
            continue;
        }
        remove.push(*jid);
    }
    for jid in remove {
        s.junctions.remove(&jid);
    }
    for (k, p) in &need {
        if !have.contains(k) {
            let id = ids.fresh()?;
            s.junctions.insert(id, Junction { sheet: k.0, at: *p });
        }
    }
    Ok(())
}
