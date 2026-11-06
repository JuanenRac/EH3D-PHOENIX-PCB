//! What the operations of the hierarchy and of the buses do to a schematic (see [`crate::edit`]).
//! Everything here works on a copy: when it returns an error the caller throws the copy away.

use std::collections::BTreeSet;

use eh3d_phoenix_model::Point;

use crate::bus::{expand_spec, Bus, BusAlias, BusEntry};
use crate::edit::{clean_text, EditError, EditOp, IdSource, Item};
use crate::hier::{HierPin, SheetBlock, SheetPort};
use crate::model::{Id, Schematic};

fn check_sheet(s: &Schematic, sheet: Id) -> Result<(), EditError> {
    if s.sheets.contains_key(&sheet) {
        Ok(())
    } else {
        Err(EditError::NoSuchSheet(sheet))
    }
}

fn shift(p: Point, dx: i64, dy: i64) -> Result<Point, EditError> {
    let x = p.x_nm.get().checked_add(dx).ok_or(EditError::OutOfRange)?;
    let y = p.y_nm.get().checked_add(dy).ok_or(EditError::OutOfRange)?;
    Point::from_nm(x, y).map_err(|_| EditError::OutOfRange)
}

/// The name of a sheet symbol: a clean text without a slash, which would break the names of the
/// nets, and not used by another symbol of the same sheet.
fn block_name(
    s: &Schematic,
    sheet: Id,
    name: &str,
    except: Option<Id>,
) -> Result<String, EditError> {
    let name = clean_text(name, false, "the name of the sheet symbol")?;
    if name.contains('/') {
        return Err(EditError::BadText("the name of the sheet symbol"));
    }
    let used = s
        .sheet_blocks
        .iter()
        .any(|(id, b)| Some(*id) != except && b.sheet == sheet && b.name == name);
    if used {
        return Err(EditError::BadSelection(
            "another sheet symbol of the sheet has that name",
        ));
    }
    Ok(name)
}

/// Whether the sheet `from` shows the sheet `target`, directly or through other sheets.
fn reaches(s: &Schematic, from: Id, target: Id) -> bool {
    let mut seen: BTreeSet<Id> = BTreeSet::new();
    let mut todo = vec![from];
    while let Some(sheet) = todo.pop() {
        if !seen.insert(sheet) {
            continue;
        }
        for block in s.sheet_blocks.values().filter(|b| b.sheet == sheet) {
            if block.child == target {
                return true;
            }
            todo.push(block.child);
        }
    }
    false
}

fn remove_sheet(s: &mut Schematic, sheet: Id) -> Result<(), EditError> {
    check_sheet(s, sheet)?;
    if s.sheet_blocks.values().any(|b| b.child == sheet) {
        return Err(EditError::BadSelection(
            "a sheet symbol still shows the sheet",
        ));
    }
    let blocks: BTreeSet<Id> = s
        .sheet_blocks
        .iter()
        .filter(|(_, b)| b.sheet == sheet)
        .map(|(id, _)| *id)
        .collect();
    s.sheet_blocks.retain(|id, _| !blocks.contains(id));
    s.hier_pins.retain(|_, p| !blocks.contains(&p.block));
    s.sheet_ports.retain(|_, p| p.sheet != sheet);
    let parts: BTreeSet<Id> = s
        .parts
        .iter()
        .filter(|(_, p)| p.placement.is_some_and(|place| place.sheet == sheet))
        .map(|(id, _)| *id)
        .collect();
    s.parts.retain(|id, _| !parts.contains(id));
    s.mirrored.retain(|id| !parts.contains(id));
    s.wires.retain(|_, w| w.sheet != sheet);
    s.labels.retain(|_, l| l.sheet != sheet);
    s.junctions.retain(|_, j| j.sheet != sheet);
    s.power_ports.retain(|_, p| p.sheet != sheet);
    let buses: BTreeSet<Id> = s
        .buses
        .iter()
        .filter(|(_, b)| b.sheet == sheet)
        .map(|(id, _)| *id)
        .collect();
    s.buses.retain(|id, _| !buses.contains(id));
    s.bus_entries.retain(|_, e| !buses.contains(&e.bus));
    s.sheets.remove(&sheet);
    Ok(())
}

fn move_to_sheet(
    s: &mut Schematic,
    items: &[Item],
    sheet: Id,
    dx: i64,
    dy: i64,
) -> Result<(), EditError> {
    check_sheet(s, sheet)?;
    let wanted: BTreeSet<Item> = items.iter().copied().collect();
    for item in wanted {
        match item {
            Item::Part(id) => {
                let part = s.parts.get_mut(&id).ok_or(EditError::NoSuchPart(id))?;
                let place = part
                    .placement
                    .as_mut()
                    .ok_or(EditError::BadSelection("the part is not placed on a sheet"))?;
                place.sheet = sheet;
                place.at = shift(place.at, dx, dy)?;
            }
            Item::Wire(id) => {
                let wire = s.wires.get_mut(&id).ok_or(EditError::NoSuchItem(id))?;
                wire.sheet = sheet;
                for p in wire.path.iter_mut() {
                    *p = shift(*p, dx, dy)?;
                }
            }
            Item::Label(id) => {
                let label = s.labels.get_mut(&id).ok_or(EditError::NoSuchItem(id))?;
                label.sheet = sheet;
                label.at = shift(label.at, dx, dy)?;
            }
            Item::PowerPort(id) => {
                let port = s
                    .power_ports
                    .get_mut(&id)
                    .ok_or(EditError::NoSuchItem(id))?;
                port.sheet = sheet;
                port.at = shift(port.at, dx, dy)?;
            }
            Item::Junction(id) => {
                let junction = s.junctions.get_mut(&id).ok_or(EditError::NoSuchItem(id))?;
                junction.sheet = sheet;
                junction.at = shift(junction.at, dx, dy)?;
            }
        }
    }
    Ok(())
}

fn set_alias(
    s: &mut Schematic,
    ids: &mut IdSource,
    name: &str,
    members: &[String],
) -> Result<(), EditError> {
    let name = clean_text(name, false, "the name of the alias")?;
    let mut items = Vec::new();
    for member in members {
        items.push(clean_text(member, false, "an item of the alias")?);
    }
    let mut map = s.alias_map();
    map.insert(name.clone(), items.clone());
    expand_spec(&name, &map).map_err(|_| EditError::BadBus(name.clone()))?;
    let existing = s
        .bus_aliases
        .iter()
        .find(|(_, a)| a.name == name)
        .map(|(id, _)| *id);
    let id = match existing {
        Some(id) => id,
        None => ids.fresh()?,
    };
    s.bus_aliases.insert(
        id,
        BusAlias {
            name,
            members: items,
        },
    );
    Ok(())
}

/// Runs one operation of the hierarchy or of the buses on `s`. `wiring` is set when the operation
/// may change what the drawing joins.
pub(crate) fn run(
    s: &mut Schematic,
    ids: &mut IdSource,
    op: &EditOp,
    wiring: &mut bool,
) -> Result<(), EditError> {
    match op {
        EditOp::RemoveSheet { sheet } => {
            remove_sheet(s, *sheet)?;
            *wiring = true;
        }
        EditOp::RenameSheet { sheet, name } => {
            let name = clean_text(name, false, "the name of the sheet")?;
            let entry = s
                .sheets
                .get_mut(sheet)
                .ok_or(EditError::NoSuchSheet(*sheet))?;
            entry.name = name;
        }
        EditOp::AddSheetBlock {
            sheet,
            child,
            name,
            at,
            width,
            height,
        } => {
            check_sheet(s, *sheet)?;
            check_sheet(s, *child)?;
            if sheet == child || reaches(s, *child, *sheet) {
                return Err(EditError::BadSelection("a sheet cannot show itself"));
            }
            if width.get() <= 0 || height.get() <= 0 {
                return Err(EditError::OutOfRange);
            }
            let name = block_name(s, *sheet, name, None)?;
            let id = ids.fresh()?;
            s.sheet_blocks.insert(
                id,
                SheetBlock {
                    sheet: *sheet,
                    child: *child,
                    name,
                    at: *at,
                    width: *width,
                    height: *height,
                },
            );
        }
        EditOp::RemoveSheetBlock { block } => {
            if s.sheet_blocks.remove(block).is_none() {
                return Err(EditError::NoSuchItem(*block));
            }
            s.hier_pins.retain(|_, p| p.block != *block);
            *wiring = true;
        }
        EditOp::RenameSheetBlock { block, name } => {
            let sheet = s
                .sheet_blocks
                .get(block)
                .map(|b| b.sheet)
                .ok_or(EditError::NoSuchItem(*block))?;
            let name = block_name(s, sheet, name, Some(*block))?;
            if let Some(b) = s.sheet_blocks.get_mut(block) {
                b.name = name;
            }
            *wiring = true;
        }
        EditOp::AddHierPin {
            block,
            name,
            dir,
            at,
        } => {
            if !s.sheet_blocks.contains_key(block) {
                return Err(EditError::NoSuchItem(*block));
            }
            let name = clean_text(name, false, "the name of a pin")?;
            if s.hier_pins
                .values()
                .any(|p| p.block == *block && p.name == name)
            {
                return Err(EditError::BadSelection(
                    "the sheet symbol already has a pin of that name",
                ));
            }
            let id = ids.fresh()?;
            s.hier_pins.insert(
                id,
                HierPin {
                    block: *block,
                    name,
                    dir: *dir,
                    at: *at,
                },
            );
            *wiring = true;
        }
        EditOp::RemoveHierPin { pin } => {
            if s.hier_pins.remove(pin).is_none() {
                return Err(EditError::NoSuchItem(*pin));
            }
            *wiring = true;
        }
        EditOp::AddSheetPort {
            sheet,
            name,
            dir,
            at,
        } => {
            check_sheet(s, *sheet)?;
            let name = clean_text(name, false, "the name of a port")?;
            if s.sheet_ports
                .values()
                .any(|p| p.sheet == *sheet && p.name == name)
            {
                return Err(EditError::BadSelection(
                    "the sheet already has a port of that name",
                ));
            }
            let id = ids.fresh()?;
            s.sheet_ports.insert(
                id,
                SheetPort {
                    sheet: *sheet,
                    name,
                    dir: *dir,
                    at: *at,
                },
            );
            *wiring = true;
        }
        EditOp::RemoveSheetPort { port } => {
            if s.sheet_ports.remove(port).is_none() {
                return Err(EditError::NoSuchItem(*port));
            }
            *wiring = true;
        }
        EditOp::MoveToSheet {
            items,
            sheet,
            dx,
            dy,
        } => {
            move_to_sheet(s, items, *sheet, *dx, *dy)?;
            *wiring = true;
        }
        EditOp::PlaceBus { sheet, at, spec } => {
            check_sheet(s, *sheet)?;
            let spec = clean_text(spec, false, "the name of the bus")?;
            expand_spec(&spec, &s.alias_map()).map_err(|_| EditError::BadBus(spec.clone()))?;
            let id = ids.fresh()?;
            s.buses.insert(
                id,
                Bus {
                    sheet: *sheet,
                    at: *at,
                    spec,
                },
            );
        }
        EditOp::RemoveBus { bus } => {
            if s.buses.remove(bus).is_none() {
                return Err(EditError::NoSuchItem(*bus));
            }
            s.bus_entries.retain(|_, e| e.bus != *bus);
            *wiring = true;
        }
        EditOp::AddBusEntry { bus, member, at } => {
            if !s.buses.contains_key(bus) {
                return Err(EditError::NoSuchItem(*bus));
            }
            let member = clean_text(member, false, "the name of a member")?;
            let names = s
                .bus_member_names(*bus)
                .map_err(|_| EditError::BadBus(member.clone()))?;
            if !names.contains(&member) {
                return Err(EditError::BadBus(member));
            }
            let id = ids.fresh()?;
            s.bus_entries.insert(
                id,
                BusEntry {
                    bus: *bus,
                    at: *at,
                    member,
                },
            );
            *wiring = true;
        }
        EditOp::RemoveBusEntry { entry } => {
            if s.bus_entries.remove(entry).is_none() {
                return Err(EditError::NoSuchItem(*entry));
            }
            *wiring = true;
        }
        EditOp::SetBusAlias { name, members } => {
            set_alias(s, ids, name, members)?;
            *wiring = true;
        }
        EditOp::RemoveBusAlias { name } => {
            let found = s
                .bus_aliases
                .iter()
                .find(|(_, a)| a.name == *name)
                .map(|(id, _)| *id);
            match found {
                Some(id) => {
                    s.bus_aliases.remove(&id);
                }
                None => return Err(EditError::BadBus(name.clone())),
            }
            *wiring = true;
        }
        _ => {}
    }
    Ok(())
}
