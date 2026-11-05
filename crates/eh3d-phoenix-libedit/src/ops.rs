// SPDX-License-Identifier: GPL-3.0-or-later
//! The operations that edit a library, and the history that undoes and redoes them.
//!
//! * every operation is checked before it changes anything, and a list of operations is applied
//!   all or nothing: the first one that fails leaves the library as it was and says which;
//! * the history keeps a copy of the whole library for every step; a copy costs a list of
//!   pointers, because the footprints, symbols and device sets are shared and only the ones an
//!   operation touches are copied;
//! * the operations are plain data and read from the JSON of `phoenix libedit --apply`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::codes::{
    LibError, IN_USE, NAME_INVALID, NAME_USED, NOT_FOUND, OPERATIONS_INVALID, VALUE_INVALID,
};
use crate::geom::auto_diameter;
use crate::model::{
    Circle, Connect, Device, DeviceSet, Gate, Item, Library, Package, PadShape, Rect, Symbol,
    LAYER_BOTTOM, LAYER_B_CREAM, LAYER_B_STOP, LAYER_T_CREAM, LAYER_T_STOP,
};

/// Coordinates stay strictly inside this many nanometres, as in the model of the design.
pub const COORD_LIMIT: i64 = (1 << 30) - 1;
/// The longest name.
pub const MAX_NAME: usize = 127;

/// Whether an operation is about a footprint or about a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// A footprint.
    Package,
    /// A symbol.
    Symbol,
}

/// The footprint or the symbol an item operation is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// Footprint or symbol.
    pub kind: TargetKind,
    /// Its name.
    pub name: String,
}

impl Target {
    /// A footprint.
    pub fn package(name: &str) -> Target {
        Target {
            kind: TargetKind::Package,
            name: name.to_owned(),
        }
    }

    /// A symbol.
    pub fn symbol(name: &str) -> Target {
        Target {
            kind: TargetKind::Symbol,
            name: name.to_owned(),
        }
    }
}

/// The opening of a pad that an operation sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpeningKind {
    /// The solder mask.
    Mask,
    /// The solder paste.
    Paste,
}

/// One edit of a library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Makes an empty footprint.
    CreatePackage {
        /// Its name.
        name: String,
    },
    /// Renames a footprint and the devices that use it.
    RenamePackage {
        /// The name now.
        from: String,
        /// The new name.
        to: String,
    },
    /// Deletes a footprint that no device uses.
    DeletePackage {
        /// Its name.
        name: String,
    },
    /// Makes an empty symbol.
    CreateSymbol {
        /// Its name.
        name: String,
    },
    /// Renames a symbol and the gates that use it.
    RenameSymbol {
        /// The name now.
        from: String,
        /// The new name.
        to: String,
    },
    /// Deletes a symbol that no gate uses.
    DeleteSymbol {
        /// Its name.
        name: String,
    },
    /// Makes an empty device set.
    CreateDeviceSet {
        /// Its name.
        name: String,
        /// The prefix of its reference designators.
        #[serde(default)]
        prefix: String,
    },
    /// Renames a device set.
    RenameDeviceSet {
        /// The name now.
        from: String,
        /// The new name.
        to: String,
    },
    /// Deletes a device set.
    DeleteDeviceSet {
        /// Its name.
        name: String,
    },
    /// Sets the prefix of a device set.
    SetPrefix {
        /// The device set.
        set: String,
        /// The new prefix.
        prefix: String,
    },
    /// Adds a drawing item to a footprint or a symbol.
    AddItem {
        /// Where.
        target: Target,
        /// What.
        item: Item,
        /// The place in the list; at the end when it is not given.
        #[serde(default)]
        at: Option<usize>,
    },
    /// Removes a drawing item.
    RemoveItem {
        /// Where.
        target: Target,
        /// The place of the item in the list.
        index: usize,
    },
    /// Replaces a drawing item by another (this is how a property is edited).
    ReplaceItem {
        /// Where.
        target: Target,
        /// The place of the item in the list.
        index: usize,
        /// The new item.
        item: Item,
    },
    /// Moves a drawing item.
    MoveItem {
        /// Where.
        target: Target,
        /// The place of the item in the list.
        index: usize,
        /// The shift in x, in nanometres.
        dx: i64,
        /// The shift in y, in nanometres.
        dy: i64,
    },
    /// Moves the origin to a point: every item moves the other way.
    ShiftOrigin {
        /// Where.
        target: Target,
        /// The x of the new origin, in the coordinates now.
        dx: i64,
        /// The y of the new origin, in the coordinates now.
        dy: i64,
    },
    /// Gives a pad an opening of the solder mask or the paste that is larger (or smaller) than
    /// its copper by `expansion`, drawn as a shape on the layer of the openings; without an
    /// expansion the opening is removed and the pad is left to the automatic one.
    SetOpening {
        /// The footprint.
        package: String,
        /// The name of the pad.
        pad: String,
        /// Mask or paste.
        kind: OpeningKind,
        /// The expansion on every side, in nanometres.
        #[serde(default)]
        expansion: Option<i64>,
    },
    /// Adds a gate to a device set.
    AddGate {
        /// The device set.
        set: String,
        /// The name of the gate.
        name: String,
        /// The symbol it uses.
        symbol: String,
        /// Its place in the editor of the device.
        #[serde(default)]
        x: i64,
        /// Its place in the editor of the device.
        #[serde(default)]
        y: i64,
    },
    /// Removes a gate and its connections.
    RemoveGate {
        /// The device set.
        set: String,
        /// The name of the gate.
        name: String,
    },
    /// Adds a device to a device set.
    AddDevice {
        /// The device set.
        set: String,
        /// The name of the variant, which may be empty.
        name: String,
        /// The footprint, when it has one.
        #[serde(default)]
        package: Option<String>,
    },
    /// Removes a device.
    RemoveDevice {
        /// The device set.
        set: String,
        /// The name of the variant.
        name: String,
    },
    /// Links a device to a footprint, or to none.
    LinkDevice {
        /// The device set.
        set: String,
        /// The name of the variant.
        device: String,
        /// The footprint, or none to unlink it.
        #[serde(default)]
        package: Option<String>,
    },
    /// Connects a pin of a gate to a pad (or several, separated by spaces) of the footprint.
    Connect {
        /// The device set.
        set: String,
        /// The name of the variant.
        device: String,
        /// The gate.
        gate: String,
        /// The pin of the symbol of the gate.
        pin: String,
        /// The pad.
        pad: String,
    },
    /// Removes the connection of a pin.
    Disconnect {
        /// The device set.
        set: String,
        /// The name of the variant.
        device: String,
        /// The gate.
        gate: String,
        /// The pin.
        pin: String,
    },
    /// Connects every pin to the pad of the same name, when the footprint has one.
    AutoConnect {
        /// The device set.
        set: String,
        /// The name of the variant.
        device: String,
    },
}

impl Operation {
    /// A short sentence that names the operation, for the history and the dry run.
    pub fn label(&self) -> String {
        match self {
            Operation::CreatePackage { name } => format!("create footprint {name}"),
            Operation::RenamePackage { from, to } => format!("rename footprint {from} to {to}"),
            Operation::DeletePackage { name } => format!("delete footprint {name}"),
            Operation::CreateSymbol { name } => format!("create symbol {name}"),
            Operation::RenameSymbol { from, to } => format!("rename symbol {from} to {to}"),
            Operation::DeleteSymbol { name } => format!("delete symbol {name}"),
            Operation::CreateDeviceSet { name, .. } => format!("create device set {name}"),
            Operation::RenameDeviceSet { from, to } => {
                format!("rename device set {from} to {to}")
            }
            Operation::DeleteDeviceSet { name } => format!("delete device set {name}"),
            Operation::SetPrefix { set, prefix } => format!("set prefix of {set} to {prefix}"),
            Operation::AddItem { target, item, .. } => {
                format!("add {} to {}", item.kind(), target.name)
            }
            Operation::RemoveItem { target, index } => {
                format!("remove item {index} of {}", target.name)
            }
            Operation::ReplaceItem { target, index, .. } => {
                format!("change item {index} of {}", target.name)
            }
            Operation::MoveItem { target, index, .. } => {
                format!("move item {index} of {}", target.name)
            }
            Operation::ShiftOrigin { target, .. } => format!("move the origin of {}", target.name),
            Operation::SetOpening { package, pad, .. } => {
                format!("set an opening of pad {pad} of {package}")
            }
            Operation::AddGate { set, name, .. } => format!("add gate {name} to {set}"),
            Operation::RemoveGate { set, name } => format!("remove gate {name} of {set}"),
            Operation::AddDevice { set, name, .. } => format!("add device {name} to {set}"),
            Operation::RemoveDevice { set, name } => format!("remove device {name} of {set}"),
            Operation::LinkDevice { set, device, .. } => {
                format!("link device {device} of {set}")
            }
            Operation::Connect { set, pin, pad, .. } => {
                format!("connect pin {pin} to pad {pad} in {set}")
            }
            Operation::Disconnect { set, pin, .. } => format!("disconnect pin {pin} in {set}"),
            Operation::AutoConnect { set, device } => {
                format!("connect the pins of {device} of {set} to the pads of the same name")
            }
        }
    }
}

fn err(code: &'static str, detail: impl Into<String>) -> LibError {
    LibError::new(code, detail)
}

/// Checks a name of a footprint, a symbol, a device set, a gate, a pad or a pin: not empty, not
/// longer than [`MAX_NAME`], no white space or control characters and none of `<>&"`.
pub fn check_name(name: &str) -> Result<(), LibError> {
    if name.is_empty() {
        return Err(err(NAME_INVALID, "the name is empty"));
    }
    check_name_or_empty(name)
}

/// Like [`check_name`] but the empty name is valid (the variant of a device may have none).
pub fn check_name_or_empty(name: &str) -> Result<(), LibError> {
    if name.chars().count() > MAX_NAME {
        return Err(err(NAME_INVALID, format!("{name:?} is too long")));
    }
    if name
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || "<>&\"".contains(c))
    {
        return Err(err(NAME_INVALID, format!("{name:?}")));
    }
    Ok(())
}

/// True for a rotation as the file writes it: empty, or `R`, `MR`, `SR` followed by degrees.
pub fn valid_rot(rot: &str) -> bool {
    if rot.is_empty() {
        return true;
    }
    let rest = rot.trim_start_matches(['S', 'M']);
    if rot.len() - rest.len() > 2 {
        return false;
    }
    match rest.strip_prefix('R') {
        Some(degrees) => degrees.parse::<f64>().is_ok_and(f64::is_finite),
        None => false,
    }
}

fn in_range(label: &str, value: i64) -> Result<(), LibError> {
    if value.abs() > COORD_LIMIT {
        return Err(err(
            VALUE_INVALID,
            format!("{label} {value} is out of the range of the design"),
        ));
    }
    Ok(())
}

fn positive(label: &str, value: i64) -> Result<(), LibError> {
    in_range(label, value)?;
    if value <= 0 {
        return Err(err(VALUE_INVALID, format!("{label} must be above zero")));
    }
    Ok(())
}

fn not_negative(label: &str, value: i64) -> Result<(), LibError> {
    in_range(label, value)?;
    if value < 0 {
        return Err(err(VALUE_INVALID, format!("{label} must not be negative")));
    }
    Ok(())
}

fn one_of(label: &str, value: &str, allowed: &[&str]) -> Result<(), LibError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(err(
            VALUE_INVALID,
            format!("{label} {value:?} is not one of the keywords"),
        ))
    }
}

fn rot_ok(rot: &str) -> Result<(), LibError> {
    if valid_rot(rot) {
        Ok(())
    } else {
        Err(err(VALUE_INVALID, format!("rotation {rot:?}")))
    }
}

fn layer_ok(layer: u32) -> Result<(), LibError> {
    if layer == 0 || layer > 255 {
        return Err(err(VALUE_INVALID, format!("layer {layer}")));
    }
    Ok(())
}

/// Checks that an item may be put in a footprint or a symbol, and that its values are valid.
pub fn validate_item(kind: TargetKind, item: &Item) -> Result<(), LibError> {
    let in_package = kind == TargetKind::Package;
    match item {
        Item::Pad(p) => {
            if !in_package {
                return Err(err(VALUE_INVALID, "a symbol has no pads"));
            }
            check_name(&p.name)?;
            in_range("x", p.x)?;
            in_range("y", p.y)?;
            positive("drill", p.drill)?;
            not_negative("diameter", p.diameter)?;
            rot_ok(&p.rot)
        }
        Item::Smd(p) => {
            if !in_package {
                return Err(err(VALUE_INVALID, "a symbol has no pads"));
            }
            check_name(&p.name)?;
            in_range("x", p.x)?;
            in_range("y", p.y)?;
            positive("dx", p.dx)?;
            positive("dy", p.dy)?;
            layer_ok(p.layer)?;
            if p.roundness > 100 {
                return Err(err(VALUE_INVALID, "roundness is above 100"));
            }
            rot_ok(&p.rot)
        }
        Item::Hole(h) => {
            if !in_package {
                return Err(err(VALUE_INVALID, "a symbol has no holes"));
            }
            in_range("x", h.x)?;
            in_range("y", h.y)?;
            positive("drill", h.drill)
        }
        Item::Pin(p) => {
            if in_package {
                return Err(err(VALUE_INVALID, "a footprint has no pins"));
            }
            check_name(&p.name)?;
            in_range("x", p.x)?;
            in_range("y", p.y)?;
            one_of("length", &p.length, &["point", "short", "middle", "long"])?;
            one_of(
                "direction",
                &p.direction,
                &["nc", "in", "out", "io", "oc", "pwr", "pas", "hiz", "sup"],
            )?;
            one_of("function", &p.function, &["none", "dot", "clk", "dotclk"])?;
            one_of("visible", &p.visible, &["off", "pad", "pin", "both"])?;
            if p.swaplevel.parse::<u8>().is_err() {
                return Err(err(
                    VALUE_INVALID,
                    "the swap level is not a number from 0 to 255",
                ));
            }
            rot_ok(&p.rot)
        }
        Item::Wire(w) => {
            in_range("x1", w.x1)?;
            in_range("y1", w.y1)?;
            in_range("x2", w.x2)?;
            in_range("y2", w.y2)?;
            not_negative("width", w.width)?;
            layer_ok(w.layer)?;
            if w.curve.abs() >= 360_000_000 {
                return Err(err(
                    VALUE_INVALID,
                    "the angle of an arc is below 360 degrees",
                ));
            }
            if w.curve != 0 && (w.x1, w.y1) == (w.x2, w.y2) {
                return Err(err(VALUE_INVALID, "the ends of an arc differ"));
            }
            Ok(())
        }
        Item::Rect(r) => {
            in_range("x1", r.x1)?;
            in_range("y1", r.y1)?;
            in_range("x2", r.x2)?;
            in_range("y2", r.y2)?;
            layer_ok(r.layer)?;
            rot_ok(&r.rot)
        }
        Item::Circle(c) => {
            in_range("x", c.x)?;
            in_range("y", c.y)?;
            positive("radius", c.radius)?;
            not_negative("width", c.width)?;
            layer_ok(c.layer)
        }
        Item::Text(t) => {
            in_range("x", t.x)?;
            in_range("y", t.y)?;
            positive("size", t.size)?;
            layer_ok(t.layer)?;
            rot_ok(&t.rot)?;
            if t.content.is_empty() || t.content != t.content.trim() {
                return Err(err(
                    VALUE_INVALID,
                    "a text is not empty and has no space at its ends",
                ));
            }
            Ok(())
        }
        Item::Other(_) => Err(err(
            VALUE_INVALID,
            "an item of another kind cannot be added",
        )),
    }
}

/// Moves an item by `dx` and `dy` nanometres.
pub fn translate(item: &mut Item, dx: i64, dy: i64) -> Result<(), LibError> {
    let mv = |x: &mut i64, d: i64| -> Result<(), LibError> {
        let v = x.checked_add(d).unwrap_or(i64::MAX);
        in_range("coordinate", v)?;
        *x = v;
        Ok(())
    };
    match item {
        Item::Pad(p) => {
            mv(&mut p.x, dx)?;
            mv(&mut p.y, dy)
        }
        Item::Smd(p) => {
            mv(&mut p.x, dx)?;
            mv(&mut p.y, dy)
        }
        Item::Hole(p) => {
            mv(&mut p.x, dx)?;
            mv(&mut p.y, dy)
        }
        Item::Pin(p) => {
            mv(&mut p.x, dx)?;
            mv(&mut p.y, dy)
        }
        Item::Circle(p) => {
            mv(&mut p.x, dx)?;
            mv(&mut p.y, dy)
        }
        Item::Text(p) => {
            mv(&mut p.x, dx)?;
            mv(&mut p.y, dy)
        }
        Item::Wire(p) => {
            mv(&mut p.x1, dx)?;
            mv(&mut p.x2, dx)?;
            mv(&mut p.y1, dy)?;
            mv(&mut p.y2, dy)
        }
        Item::Rect(p) => {
            mv(&mut p.x1, dx)?;
            mv(&mut p.x2, dx)?;
            mv(&mut p.y1, dy)?;
            mv(&mut p.y2, dy)
        }
        Item::Other(_) => Err(err(
            VALUE_INVALID,
            "an item of another kind cannot be moved",
        )),
    }
}

fn items_of<'a>(lib: &'a mut Library, target: &Target) -> Result<&'a mut Vec<Item>, LibError> {
    let missing = || {
        err(
            NOT_FOUND,
            format!("{} {}", kind_word(target.kind), target.name),
        )
    };
    match target.kind {
        TargetKind::Package => {
            let i = lib.package_index(&target.name).ok_or_else(missing)?;
            Ok(&mut Arc::make_mut(&mut lib.packages[i]).items)
        }
        TargetKind::Symbol => {
            let i = lib.symbol_index(&target.name).ok_or_else(missing)?;
            Ok(&mut Arc::make_mut(&mut lib.symbols[i]).items)
        }
    }
}

fn kind_word(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::Package => "footprint",
        TargetKind::Symbol => "symbol",
    }
}

fn set_of<'a>(lib: &'a mut Library, name: &str) -> Result<&'a mut DeviceSet, LibError> {
    let i = lib
        .deviceset_index(name)
        .ok_or_else(|| err(NOT_FOUND, format!("device set {name}")))?;
    Ok(Arc::make_mut(&mut lib.devicesets[i]))
}

fn device_of<'a>(set: &'a mut DeviceSet, device: &str) -> Result<&'a mut Device, LibError> {
    let set_name = set.name.clone();
    set.devices
        .iter_mut()
        .find(|d| d.name == device)
        .ok_or_else(|| err(NOT_FOUND, format!("device {device:?} of {set_name}")))
}

fn index_error(target: &Target, index: usize) -> LibError {
    err(
        NOT_FOUND,
        format!("item {index} of {} {}", kind_word(target.kind), target.name),
    )
}

/// Renames the uses of a footprint in the devices.
fn rename_package_uses(lib: &mut Library, from: &str, to: &str) {
    for set in lib.devicesets.iter_mut() {
        if set
            .devices
            .iter()
            .any(|d| d.package.as_deref() == Some(from))
        {
            let set = Arc::make_mut(set);
            for d in set.devices.iter_mut() {
                if d.package.as_deref() == Some(from) {
                    d.package = Some(to.to_owned());
                }
            }
        }
    }
}

/// Renames the uses of a symbol in the gates.
fn rename_symbol_uses(lib: &mut Library, from: &str, to: &str) {
    for set in lib.devicesets.iter_mut() {
        if set.gates.iter().any(|g| g.symbol == from) {
            let set = Arc::make_mut(set);
            for g in set.gates.iter_mut() {
                if g.symbol == from {
                    g.symbol = to.to_owned();
                }
            }
        }
    }
}

fn used_by_devices(lib: &Library, package: &str) -> Vec<String> {
    lib.devicesets
        .iter()
        .filter(|s| {
            s.devices
                .iter()
                .any(|d| d.package.as_deref() == Some(package))
        })
        .map(|s| s.name.clone())
        .collect()
}

fn used_by_gates(lib: &Library, symbol: &str) -> Vec<String> {
    lib.devicesets
        .iter()
        .filter(|s| s.gates.iter().any(|g| g.symbol == symbol))
        .map(|s| s.name.clone())
        .collect()
}

/// The layers of the opening of a pad: the one for its copper side, or both for a pad with a
/// hole.
fn opening_layers(pad: &Item, kind: OpeningKind) -> Result<Vec<u32>, LibError> {
    match (pad, kind) {
        (Item::Smd(s), OpeningKind::Mask) => Ok(vec![if s.layer == LAYER_BOTTOM {
            LAYER_B_STOP
        } else {
            LAYER_T_STOP
        }]),
        (Item::Smd(s), OpeningKind::Paste) => Ok(vec![if s.layer == LAYER_BOTTOM {
            LAYER_B_CREAM
        } else {
            LAYER_T_CREAM
        }]),
        (Item::Pad(_), OpeningKind::Mask) => Ok(vec![LAYER_T_STOP, LAYER_B_STOP]),
        (Item::Pad(_), OpeningKind::Paste) => {
            Err(err(VALUE_INVALID, "a through-hole pad has no solder paste"))
        }
        _ => Err(err(NOT_FOUND, "the item is not a pad")),
    }
}

/// The shape of the opening of a pad, grown by `expansion` on every side.
fn opening_item(pad: &Item, layer: u32, expansion: i64) -> Result<Item, LibError> {
    let rect = |x: i64, y: i64, w: i64, h: i64, rot: &str| -> Result<Item, LibError> {
        if w <= 0 || h <= 0 {
            return Err(err(VALUE_INVALID, "the opening would have no size"));
        }
        let (x1, y1) = (x - w / 2, y - h / 2);
        Ok(Item::Rect(Rect {
            layer,
            x1,
            y1,
            x2: x1 + w,
            y2: y1 + h,
            rot: rot.to_owned(),
            extra: Vec::new(),
        }))
    };
    match pad {
        Item::Smd(s) => rect(s.x, s.y, s.dx + 2 * expansion, s.dy + 2 * expansion, &s.rot),
        Item::Pad(p) => {
            let d = if p.diameter > 0 {
                p.diameter
            } else {
                auto_diameter(p.drill)
            };
            match p.shape {
                PadShape::Round | PadShape::Octagon => {
                    let diameter = d + 2 * expansion;
                    if diameter <= 0 {
                        return Err(err(VALUE_INVALID, "the opening would have no size"));
                    }
                    Ok(Item::Circle(Circle {
                        layer,
                        x: p.x,
                        y: p.y,
                        radius: diameter / 2,
                        width: 0,
                        extra: Vec::new(),
                    }))
                }
                PadShape::Square => rect(p.x, p.y, d + 2 * expansion, d + 2 * expansion, &p.rot),
                PadShape::Long | PadShape::Offset => {
                    rect(p.x, p.y, 2 * d + 2 * expansion, d + 2 * expansion, &p.rot)
                }
            }
        }
        _ => Err(err(NOT_FOUND, "the item is not a pad")),
    }
}

/// True when an item is a rectangle or a circle centred on the point.
fn centred_on(item: &Item, x: i64, y: i64) -> bool {
    match item {
        Item::Rect(r) => (r.x1 + r.x2).div_euclid(2) == x && (r.y1 + r.y2).div_euclid(2) == y,
        Item::Circle(c) => c.x == x && c.y == y,
        _ => false,
    }
}

fn set_opening(
    lib: &mut Library,
    package: &str,
    pad: &str,
    kind: OpeningKind,
    expansion: Option<i64>,
) -> Result<(), LibError> {
    let target = Target::package(package);
    let items = items_of(lib, &target)?;
    let index = items
        .iter()
        .position(|i| i.is_pad() && i.name() == Some(pad))
        .ok_or_else(|| err(NOT_FOUND, format!("pad {pad} of {package}")))?;
    let layers = opening_layers(&items[index], kind)?;
    let (x, y) = items[index].pad_centre().unwrap_or((0, 0));
    if let Some(e) = expansion {
        if e.abs() > 5_000_000 {
            return Err(err(VALUE_INVALID, "the expansion is within 5 mm"));
        }
    }
    let mut made = Vec::new();
    if let Some(e) = expansion {
        for layer in &layers {
            made.push(opening_item(&items[index], *layer, e)?);
        }
    }
    items.retain(|i| !(i.layer().is_some_and(|l| layers.contains(&l)) && centred_on(i, x, y)));
    let index = items
        .iter()
        .position(|i| i.is_pad() && i.name() == Some(pad))
        .ok_or_else(|| err(NOT_FOUND, format!("pad {pad} of {package}")))?;
    let automatic = expansion.is_none();
    match (&mut items[index], kind) {
        (Item::Smd(s), OpeningKind::Mask) => s.stop = automatic,
        (Item::Smd(s), OpeningKind::Paste) => s.cream = automatic,
        (Item::Pad(p), OpeningKind::Mask) => p.stop = automatic,
        _ => {}
    }
    items.extend(made);
    Ok(())
}

fn connect_pin(
    lib: &mut Library,
    set_name: &str,
    device: &str,
    gate: &str,
    pin: &str,
    pad: &str,
) -> Result<(), LibError> {
    let set_ref = lib
        .deviceset(set_name)
        .ok_or_else(|| err(NOT_FOUND, format!("device set {set_name}")))?;
    let gate_ref = set_ref
        .gates
        .iter()
        .find(|g| g.name == gate)
        .ok_or_else(|| err(NOT_FOUND, format!("gate {gate} of {set_name}")))?;
    let symbol = lib
        .symbol(&gate_ref.symbol)
        .ok_or_else(|| err(NOT_FOUND, format!("symbol {}", gate_ref.symbol)))?;
    if !symbol.pins().any(|(_, p)| p.name == pin) {
        return Err(err(
            NOT_FOUND,
            format!("pin {pin} of symbol {}", symbol.name),
        ));
    }
    let dev = set_ref
        .devices
        .iter()
        .find(|d| d.name == device)
        .ok_or_else(|| err(NOT_FOUND, format!("device {device:?} of {set_name}")))?;
    let package_name = dev
        .package
        .as_deref()
        .ok_or_else(|| err(VALUE_INVALID, format!("device {device:?} has no footprint")))?;
    let package = lib
        .package(package_name)
        .ok_or_else(|| err(NOT_FOUND, format!("footprint {package_name}")))?;
    let pads: Vec<&str> = pad.split_whitespace().collect();
    if pads.is_empty() {
        return Err(err(VALUE_INVALID, "no pad is given"));
    }
    for p in &pads {
        if package.pad_index(p).is_none() {
            return Err(err(NOT_FOUND, format!("pad {p} of {package_name}")));
        }
    }
    let joined = pads.join(" ");
    let set = set_of(lib, set_name)?;
    let dev = device_of(set, device)?;
    put_connect(dev, gate, pin, &joined);
    Ok(())
}

fn put_connect(dev: &mut Device, gate: &str, pin: &str, pad: &str) {
    if let Some(c) = dev
        .connects
        .iter_mut()
        .find(|c| c.gate == gate && c.pin == pin)
    {
        c.pad = pad.to_owned();
    } else {
        dev.connects.push(Connect {
            gate: gate.to_owned(),
            pin: pin.to_owned(),
            pad: pad.to_owned(),
            extra: Vec::new(),
        });
    }
}

fn auto_connect(lib: &mut Library, set_name: &str, device: &str) -> Result<(), LibError> {
    let set_ref = lib
        .deviceset(set_name)
        .ok_or_else(|| err(NOT_FOUND, format!("device set {set_name}")))?;
    let dev = set_ref
        .devices
        .iter()
        .find(|d| d.name == device)
        .ok_or_else(|| err(NOT_FOUND, format!("device {device:?} of {set_name}")))?;
    let package_name = dev
        .package
        .as_deref()
        .ok_or_else(|| err(VALUE_INVALID, format!("device {device:?} has no footprint")))?;
    let package = lib
        .package(package_name)
        .ok_or_else(|| err(NOT_FOUND, format!("footprint {package_name}")))?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for gate in &set_ref.gates {
        let Some(symbol) = lib.symbol(&gate.symbol) else {
            return Err(err(NOT_FOUND, format!("symbol {}", gate.symbol)));
        };
        for (_, pin) in symbol.pins() {
            if package.pad_index(&pin.name).is_some() {
                pairs.push((gate.name.clone(), pin.name.clone()));
            }
        }
    }
    let set = set_of(lib, set_name)?;
    let dev = device_of(set, device)?;
    for (gate, pin) in pairs {
        put_connect(dev, &gate, &pin, &pin);
    }
    Ok(())
}

/// Applies one operation to a library. On an error the library may be half changed, so callers
/// apply to a copy ([`apply_all`] does).
pub fn apply_one(lib: &mut Library, op: &Operation) -> Result<(), LibError> {
    match op {
        Operation::CreatePackage { name } => {
            check_name(name)?;
            if lib.package_index(name).is_some() {
                return Err(err(NAME_USED, format!("footprint {name}")));
            }
            lib.packages.push(Arc::new(Package {
                name: name.clone(),
                ..Package::default()
            }));
        }
        Operation::RenamePackage { from, to } => {
            let i = lib
                .package_index(from)
                .ok_or_else(|| err(NOT_FOUND, format!("footprint {from}")))?;
            if from != to {
                check_name(to)?;
                if lib.package_index(to).is_some() {
                    return Err(err(NAME_USED, format!("footprint {to}")));
                }
                Arc::make_mut(&mut lib.packages[i]).name = to.clone();
                rename_package_uses(lib, from, to);
            }
        }
        Operation::DeletePackage { name } => {
            let i = lib
                .package_index(name)
                .ok_or_else(|| err(NOT_FOUND, format!("footprint {name}")))?;
            let users = used_by_devices(lib, name);
            if !users.is_empty() {
                return Err(err(
                    IN_USE,
                    format!("footprint {name} in {}", users.join(", ")),
                ));
            }
            lib.packages.remove(i);
        }
        Operation::CreateSymbol { name } => {
            check_name(name)?;
            if lib.symbol_index(name).is_some() {
                return Err(err(NAME_USED, format!("symbol {name}")));
            }
            lib.symbols.push(Arc::new(Symbol {
                name: name.clone(),
                ..Symbol::default()
            }));
        }
        Operation::RenameSymbol { from, to } => {
            let i = lib
                .symbol_index(from)
                .ok_or_else(|| err(NOT_FOUND, format!("symbol {from}")))?;
            if from != to {
                check_name(to)?;
                if lib.symbol_index(to).is_some() {
                    return Err(err(NAME_USED, format!("symbol {to}")));
                }
                Arc::make_mut(&mut lib.symbols[i]).name = to.clone();
                rename_symbol_uses(lib, from, to);
            }
        }
        Operation::DeleteSymbol { name } => {
            let i = lib
                .symbol_index(name)
                .ok_or_else(|| err(NOT_FOUND, format!("symbol {name}")))?;
            let users = used_by_gates(lib, name);
            if !users.is_empty() {
                return Err(err(
                    IN_USE,
                    format!("symbol {name} in {}", users.join(", ")),
                ));
            }
            lib.symbols.remove(i);
        }
        Operation::CreateDeviceSet { name, prefix } => {
            check_name(name)?;
            check_name_or_empty(prefix)?;
            if lib.deviceset_index(name).is_some() {
                return Err(err(NAME_USED, format!("device set {name}")));
            }
            lib.devicesets.push(Arc::new(DeviceSet {
                name: name.clone(),
                prefix: prefix.clone(),
                ..DeviceSet::default()
            }));
        }
        Operation::RenameDeviceSet { from, to } => {
            let i = lib
                .deviceset_index(from)
                .ok_or_else(|| err(NOT_FOUND, format!("device set {from}")))?;
            if from != to {
                check_name(to)?;
                if lib.deviceset_index(to).is_some() {
                    return Err(err(NAME_USED, format!("device set {to}")));
                }
                Arc::make_mut(&mut lib.devicesets[i]).name = to.clone();
            }
        }
        Operation::DeleteDeviceSet { name } => {
            let i = lib
                .deviceset_index(name)
                .ok_or_else(|| err(NOT_FOUND, format!("device set {name}")))?;
            lib.devicesets.remove(i);
        }
        Operation::SetPrefix { set, prefix } => {
            check_name_or_empty(prefix)?;
            set_of(lib, set)?.prefix = prefix.clone();
        }
        Operation::AddItem { target, item, at } => {
            validate_item(target.kind, item)?;
            let items = items_of(lib, target)?;
            let place = at.unwrap_or(items.len());
            if place > items.len() {
                return Err(index_error(target, place));
            }
            items.insert(place, item.clone());
        }
        Operation::RemoveItem { target, index } => {
            let items = items_of(lib, target)?;
            if *index >= items.len() {
                return Err(index_error(target, *index));
            }
            items.remove(*index);
        }
        Operation::ReplaceItem {
            target,
            index,
            item,
        } => {
            validate_item(target.kind, item)?;
            let items = items_of(lib, target)?;
            if *index >= items.len() {
                return Err(index_error(target, *index));
            }
            items[*index] = item.clone();
        }
        Operation::MoveItem {
            target,
            index,
            dx,
            dy,
        } => {
            let items = items_of(lib, target)?;
            if *index >= items.len() {
                return Err(index_error(target, *index));
            }
            translate(&mut items[*index], *dx, *dy)?;
        }
        Operation::ShiftOrigin { target, dx, dy } => {
            in_range("dx", *dx)?;
            in_range("dy", *dy)?;
            let items = items_of(lib, target)?;
            for item in items.iter_mut() {
                translate(item, -*dx, -*dy)?;
            }
        }
        Operation::SetOpening {
            package,
            pad,
            kind,
            expansion,
        } => set_opening(lib, package, pad, *kind, *expansion)?,
        Operation::AddGate {
            set,
            name,
            symbol,
            x,
            y,
        } => {
            check_name(name)?;
            in_range("x", *x)?;
            in_range("y", *y)?;
            if lib.symbol_index(symbol).is_none() {
                return Err(err(NOT_FOUND, format!("symbol {symbol}")));
            }
            let s = set_of(lib, set)?;
            if s.gates.iter().any(|g| g.name == *name) {
                return Err(err(NAME_USED, format!("gate {name} of {set}")));
            }
            s.gates.push(Gate {
                name: name.clone(),
                symbol: symbol.clone(),
                x: *x,
                y: *y,
                extra: Vec::new(),
            });
        }
        Operation::RemoveGate { set, name } => {
            let s = set_of(lib, set)?;
            let before = s.gates.len();
            s.gates.retain(|g| g.name != *name);
            if s.gates.len() == before {
                return Err(err(NOT_FOUND, format!("gate {name} of {set}")));
            }
            for d in s.devices.iter_mut() {
                d.connects.retain(|c| c.gate != *name);
            }
        }
        Operation::AddDevice { set, name, package } => {
            check_name_or_empty(name)?;
            if let Some(p) = package {
                if lib.package_index(p).is_none() {
                    return Err(err(NOT_FOUND, format!("footprint {p}")));
                }
            }
            let s = set_of(lib, set)?;
            if s.devices.iter().any(|d| d.name == *name) {
                return Err(err(NAME_USED, format!("device {name:?} of {set}")));
            }
            s.devices.push(Device {
                name: name.clone(),
                package: package.clone(),
                ..Device::default()
            });
        }
        Operation::RemoveDevice { set, name } => {
            let s = set_of(lib, set)?;
            let before = s.devices.len();
            s.devices.retain(|d| d.name != *name);
            if s.devices.len() == before {
                return Err(err(NOT_FOUND, format!("device {name:?} of {set}")));
            }
        }
        Operation::LinkDevice {
            set,
            device,
            package,
        } => {
            if let Some(p) = package {
                if lib.package_index(p).is_none() {
                    return Err(err(NOT_FOUND, format!("footprint {p}")));
                }
            }
            let s = set_of(lib, set)?;
            device_of(s, device)?.package = package.clone();
        }
        Operation::Connect {
            set,
            device,
            gate,
            pin,
            pad,
        } => connect_pin(lib, set, device, gate, pin, pad)?,
        Operation::Disconnect {
            set,
            device,
            gate,
            pin,
        } => {
            let s = set_of(lib, set)?;
            let d = device_of(s, device)?;
            let before = d.connects.len();
            d.connects.retain(|c| !(c.gate == *gate && c.pin == *pin));
            if d.connects.len() == before {
                return Err(err(
                    NOT_FOUND,
                    format!("connection of pin {pin} of gate {gate}"),
                ));
            }
        }
        Operation::AutoConnect { set, device } => auto_connect(lib, set, device)?,
    }
    Ok(())
}

/// Applies a list of operations to a copy of the library and returns the copy, or the place of
/// the first operation that failed and its error; the library given is never changed.
pub fn apply_all(lib: &Library, ops: &[Operation]) -> Result<Library, (usize, LibError)> {
    let mut work = lib.clone();
    for (i, op) in ops.iter().enumerate() {
        apply_one(&mut work, op).map_err(|e| (i, e))?;
    }
    Ok(work)
}

/// Reads the JSON of a list of operations: an array, or an object with an `operations` array.
pub fn parse_operations(text: &str) -> Result<Vec<Operation>, LibError> {
    let bad = |what: String| err(OPERATIONS_INVALID, what);
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
    let list = match value {
        serde_json::Value::Array(_) => value,
        serde_json::Value::Object(mut map) => map
            .remove("operations")
            .ok_or_else(|| bad("an object needs an \"operations\" array".to_owned()))?,
        _ => return Err(bad("an array of operations is needed".to_owned())),
    };
    serde_json::from_value(list).map_err(|e| bad(e.to_string()))
}

/// The undo and redo of the edits of a library.
#[derive(Debug, Clone)]
pub struct History {
    current: Library,
    revision: u64,
    saved_revision: u64,
    next_revision: u64,
    past: Vec<(String, Library, u64)>,
    future: Vec<(String, Library, u64)>,
    limit: usize,
}

impl History {
    /// A history that starts with `library`, keeping at most `limit` steps of undo.
    pub fn new(library: Library, limit: usize) -> History {
        History {
            current: library,
            revision: 0,
            saved_revision: 0,
            next_revision: 1,
            past: Vec::new(),
            future: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// The library as it is now.
    pub fn library(&self) -> &Library {
        &self.current
    }

    /// Applies the operations all or nothing as one step. On an error nothing changes and the
    /// place of the failing operation is returned with its error.
    pub fn apply(&mut self, ops: &[Operation]) -> Result<(), (usize, LibError)> {
        if ops.is_empty() {
            return Ok(());
        }
        let next = apply_all(&self.current, ops)?;
        let label = if ops.len() == 1 {
            ops[0].label()
        } else {
            format!("{} operations", ops.len())
        };
        let before = std::mem::replace(&mut self.current, next);
        self.past.push((label, before, self.revision));
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        self.future.clear();
        self.revision = self.next_revision;
        self.next_revision += 1;
        Ok(())
    }

    /// True when there is a step to undo.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// True when there is a step to redo.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// The label of the step that undo would take back.
    pub fn undo_label(&self) -> Option<&str> {
        self.past.last().map(|(l, _, _)| l.as_str())
    }

    /// The label of the step that redo would do again.
    pub fn redo_label(&self) -> Option<&str> {
        self.future.last().map(|(l, _, _)| l.as_str())
    }

    /// Takes back the last step; false when there is none.
    pub fn undo(&mut self) -> bool {
        match self.past.pop() {
            Some((label, library, revision)) => {
                let undone = std::mem::replace(&mut self.current, library);
                self.future.push((label, undone, self.revision));
                self.revision = revision;
                true
            }
            None => false,
        }
    }

    /// Does again the last step that was taken back; false when there is none.
    pub fn redo(&mut self) -> bool {
        match self.future.pop() {
            Some((label, library, revision)) => {
                let redone = std::mem::replace(&mut self.current, library);
                self.past.push((label, redone, self.revision));
                self.revision = revision;
                true
            }
            None => false,
        }
    }

    /// Notes that the library as it is now has been saved.
    pub fn mark_saved(&mut self) {
        self.saved_revision = self.revision;
    }

    /// True when the library has changed since it was read or saved.
    pub fn dirty(&self) -> bool {
        self.revision != self.saved_revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Pad, Pin, Smd, Text, Wire};
    use crate::read::tests::sample;

    fn pkg(name: &str) -> Target {
        Target::package(name)
    }

    fn smd(name: &str, x: i64) -> Item {
        Item::Smd(Smd {
            name: name.into(),
            x,
            dx: 800_000,
            dy: 900_000,
            ..Smd::default()
        })
    }

    #[test]
    fn entities_are_created_renamed_and_deleted_and_the_uses_follow() {
        let mut lib = sample();
        apply_one(&mut lib, &Operation::CreatePackage { name: "NEW".into() }).unwrap();
        assert_eq!(lib.names(Kind::Package), ["R0603", "HDR1X2", "NEW"]);
        apply_one(
            &mut lib,
            &Operation::RenamePackage {
                from: "R0603".into(),
                to: "R0603_A".into(),
            },
        )
        .unwrap();
        assert_eq!(
            lib.deviceset("R").unwrap().devices[0].package.as_deref(),
            Some("R0603_A")
        );
        apply_one(
            &mut lib,
            &Operation::RenameSymbol {
                from: "R".into(),
                to: "RES".into(),
            },
        )
        .unwrap();
        assert_eq!(lib.deviceset("R").unwrap().gates[0].symbol, "RES");
        apply_one(&mut lib, &Operation::DeletePackage { name: "NEW".into() }).unwrap();
        assert_eq!(lib.names(Kind::Package), ["R0603_A", "HDR1X2"]);
    }

    #[test]
    fn a_name_that_is_used_or_not_valid_is_refused() {
        let mut lib = sample();
        let e = apply_one(
            &mut lib,
            &Operation::CreatePackage {
                name: "R0603".into(),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, NAME_USED);
        for bad in ["", "a b", "a<b", "tab\t"] {
            let e = apply_one(&mut lib, &Operation::CreateSymbol { name: bad.into() }).unwrap_err();
            assert_eq!(e.code, NAME_INVALID, "{bad:?}");
        }
        let e = apply_one(
            &mut lib,
            &Operation::RenamePackage {
                from: "nothing".into(),
                to: "x".into(),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, NOT_FOUND);
        let e = apply_one(
            &mut lib,
            &Operation::DeletePackage {
                name: "R0603".into(),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, IN_USE);
        let e = apply_one(&mut lib, &Operation::DeleteSymbol { name: "R".into() }).unwrap_err();
        assert_eq!(e.code, IN_USE);
    }

    #[test]
    fn pads_are_added_moved_changed_and_removed() {
        let mut lib = sample();
        let t = pkg("HDR1X2");
        apply_one(
            &mut lib,
            &Operation::AddItem {
                target: t.clone(),
                item: smd("3", 5_080_000),
                at: None,
            },
        )
        .unwrap();
        let p = lib.package("HDR1X2").unwrap();
        assert_eq!(p.items.len(), 3);
        apply_one(
            &mut lib,
            &Operation::MoveItem {
                target: t.clone(),
                index: 2,
                dx: 1_000_000,
                dy: -500_000,
            },
        )
        .unwrap();
        match &lib.package("HDR1X2").unwrap().items[2] {
            Item::Smd(s) => assert_eq!((s.x, s.y), (6_080_000, -500_000)),
            other => panic!("{other:?}"),
        }
        apply_one(
            &mut lib,
            &Operation::ReplaceItem {
                target: t.clone(),
                index: 2,
                item: smd("4", 0),
            },
        )
        .unwrap();
        assert_eq!(lib.package("HDR1X2").unwrap().items[2].name(), Some("4"));
        apply_one(
            &mut lib,
            &Operation::RemoveItem {
                target: t.clone(),
                index: 2,
            },
        )
        .unwrap();
        assert_eq!(lib.package("HDR1X2").unwrap().items.len(), 2);
        let e = apply_one(
            &mut lib,
            &Operation::RemoveItem {
                target: t,
                index: 9,
            },
        )
        .unwrap_err();
        assert_eq!(e.code, NOT_FOUND);
    }

    #[test]
    fn an_item_of_the_wrong_place_or_with_a_bad_value_is_refused() {
        let mut lib = sample();
        let pin = Item::Pin(Pin {
            name: "1".into(),
            ..Pin::default()
        });
        let e = apply_one(
            &mut lib,
            &Operation::AddItem {
                target: pkg("R0603"),
                item: pin.clone(),
                at: None,
            },
        )
        .unwrap_err();
        assert_eq!(e.code, VALUE_INVALID);
        let e = apply_one(
            &mut lib,
            &Operation::AddItem {
                target: Target::symbol("R"),
                item: smd("1", 0),
                at: None,
            },
        )
        .unwrap_err();
        assert_eq!(e.code, VALUE_INVALID);
        let too_far = Item::Pad(Pad {
            name: "1".into(),
            x: 2_000_000_000,
            drill: 1_000_000,
            ..Pad::default()
        });
        assert!(validate_item(TargetKind::Package, &too_far).is_err());
        let no_drill = Item::Pad(Pad {
            name: "1".into(),
            ..Pad::default()
        });
        assert!(validate_item(TargetKind::Package, &no_drill).is_err());
        let spaced = Item::Text(Text {
            content: " x".into(),
            size: 1,
            layer: 25,
            ..Text::default()
        });
        assert!(validate_item(TargetKind::Package, &spaced).is_err());
        let bad_direction = Item::Pin(Pin {
            name: "1".into(),
            direction: "sideways".into(),
            ..Pin::default()
        });
        assert!(validate_item(TargetKind::Symbol, &bad_direction).is_err());
        assert!(validate_item(TargetKind::Symbol, &pin).is_ok());
        let e = apply_one(
            &mut lib,
            &Operation::AddItem {
                target: pkg("R0603"),
                item: smd("9", 0),
                at: Some(99),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, NOT_FOUND);
    }

    #[test]
    fn arcs_need_ends_that_differ_and_a_turn_below_a_full_one() {
        let arc = |x2: i64, curve: i64| {
            Item::Wire(Wire {
                x2,
                layer: 21,
                width: 100_000,
                curve,
                ..Wire::default()
            })
        };
        assert!(validate_item(TargetKind::Package, &arc(1_000_000, 90_000_000)).is_ok());
        assert!(validate_item(TargetKind::Package, &arc(0, 90_000_000)).is_err());
        assert!(validate_item(TargetKind::Package, &arc(1_000_000, 360_000_000)).is_err());
    }

    #[test]
    fn the_origin_moves_every_item_the_other_way() {
        let mut lib = sample();
        apply_one(
            &mut lib,
            &Operation::ShiftOrigin {
                target: pkg("HDR1X2"),
                dx: 2_540_000,
                dy: 0,
            },
        )
        .unwrap();
        let p = lib.package("HDR1X2").unwrap();
        match (&p.items[0], &p.items[1]) {
            (Item::Pad(a), Item::Pad(b)) => assert_eq!((a.x, b.x), (-2_540_000, 0)),
            other => panic!("{other:?}"),
        }
        // the footprint with a polygon cannot be shifted without moving the polygon
        let e = apply_one(
            &mut lib,
            &Operation::ShiftOrigin {
                target: pkg("R0603"),
                dx: 1,
                dy: 0,
            },
        )
        .unwrap_err();
        assert_eq!(e.code, VALUE_INVALID);
    }

    #[test]
    fn a_mask_opening_is_drawn_larger_than_the_pad_and_taken_away_again() {
        let mut lib = sample();
        let set = |expansion| Operation::SetOpening {
            package: "R0603".into(),
            pad: "1".into(),
            kind: OpeningKind::Mask,
            expansion,
        };
        apply_one(&mut lib, &set(Some(50_000))).unwrap();
        let p = lib.package("R0603").unwrap();
        let opening = p
            .items
            .iter()
            .find_map(|i| match i {
                Item::Rect(r) if r.layer == LAYER_T_STOP => Some(r.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            (opening.x1, opening.x2),
            (-800_000 - 450_000, -800_000 + 450_000)
        );
        assert_eq!((opening.y1, opening.y2), (-500_000, 500_000));
        match &p.items[0] {
            Item::Smd(s) => assert!(!s.stop),
            other => panic!("{other:?}"),
        }
        // setting it again replaces the opening instead of adding a second one
        apply_one(&mut lib, &set(Some(100_000))).unwrap();
        let count = lib
            .package("R0603")
            .unwrap()
            .items
            .iter()
            .filter(|i| i.layer() == Some(LAYER_T_STOP))
            .count();
        assert_eq!(count, 1);
        apply_one(&mut lib, &set(None)).unwrap();
        let p = lib.package("R0603").unwrap();
        assert!(p.items.iter().all(|i| i.layer() != Some(LAYER_T_STOP)));
        match &p.items[0] {
            Item::Smd(s) => assert!(s.stop),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_paste_opening_and_a_through_hole_pad_follow_their_own_rules() {
        let mut lib = sample();
        apply_one(
            &mut lib,
            &Operation::SetOpening {
                package: "R0603".into(),
                pad: "2".into(),
                kind: OpeningKind::Paste,
                expansion: Some(-50_000),
            },
        )
        .unwrap();
        let p = lib.package("R0603").unwrap();
        let rect = p
            .items
            .iter()
            .find_map(|i| match i {
                Item::Rect(r) if r.layer == LAYER_T_CREAM => Some(r.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!((rect.x2 - rect.x1, rect.y2 - rect.y1), (700_000, 800_000));
        match &p.items[1] {
            Item::Smd(s) => assert!(!s.cream && s.stop),
            other => panic!("{other:?}"),
        }
        let paste = Operation::SetOpening {
            package: "HDR1X2".into(),
            pad: "1".into(),
            kind: OpeningKind::Paste,
            expansion: Some(0),
        };
        assert_eq!(apply_one(&mut lib, &paste).unwrap_err().code, VALUE_INVALID);
        apply_one(
            &mut lib,
            &Operation::SetOpening {
                package: "HDR1X2".into(),
                pad: "1".into(),
                kind: OpeningKind::Mask,
                expansion: Some(100_000),
            },
        )
        .unwrap();
        let h = lib.package("HDR1X2").unwrap();
        // a square pad of 1.7 mm opens as a square of 1.9 mm, on the top and on the bottom
        let sides: Vec<i64> = h
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Rect(r) => Some(r.x2 - r.x1),
                _ => None,
            })
            .collect();
        assert_eq!(sides, vec![1_900_000, 1_900_000]);
        let too_small = Operation::SetOpening {
            package: "R0603".into(),
            pad: "1".into(),
            kind: OpeningKind::Mask,
            expansion: Some(-600_000),
        };
        assert_eq!(
            apply_one(&mut lib, &too_small).unwrap_err().code,
            VALUE_INVALID
        );
    }

    #[test]
    fn devices_gates_and_connections_are_edited() {
        let mut lib = sample();
        let ops = vec![
            Operation::AddDevice {
                set: "R".into(),
                name: "HDR".into(),
                package: Some("HDR1X2".into()),
            },
            Operation::AutoConnect {
                set: "R".into(),
                device: "HDR".into(),
            },
        ];
        lib = apply_all(&lib, &ops).unwrap();
        let d = &lib.deviceset("R").unwrap().devices[2];
        assert_eq!(d.connects.len(), 2);
        assert_eq!(
            (d.connects[0].pin.as_str(), d.connects[0].pad.as_str()),
            ("1", "1")
        );
        apply_one(
            &mut lib,
            &Operation::Connect {
                set: "R".into(),
                device: "HDR".into(),
                gate: "G$1".into(),
                pin: "1".into(),
                pad: "2".into(),
            },
        )
        .unwrap();
        let d = &lib.deviceset("R").unwrap().devices[2];
        assert_eq!(d.connects.len(), 2);
        assert_eq!(d.connects[0].pad, "2");
        apply_one(
            &mut lib,
            &Operation::Disconnect {
                set: "R".into(),
                device: "HDR".into(),
                gate: "G$1".into(),
                pin: "1".into(),
            },
        )
        .unwrap();
        assert_eq!(lib.deviceset("R").unwrap().devices[2].connects.len(), 1);
        let e = apply_one(
            &mut lib,
            &Operation::Connect {
                set: "R".into(),
                device: "HDR".into(),
                gate: "G$1".into(),
                pin: "1".into(),
                pad: "77".into(),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, NOT_FOUND);
        let e = apply_one(
            &mut lib,
            &Operation::Connect {
                set: "R".into(),
                device: "".into(),
                gate: "G$1".into(),
                pin: "1".into(),
                pad: "1".into(),
            },
        )
        .unwrap_err();
        assert_eq!(e.code, VALUE_INVALID);
        apply_one(
            &mut lib,
            &Operation::RemoveGate {
                set: "R".into(),
                name: "G$1".into(),
            },
        )
        .unwrap();
        let set = lib.deviceset("R").unwrap();
        assert!(set.gates.is_empty());
        assert!(set.devices.iter().all(|d| d.connects.is_empty()));
    }

    #[test]
    fn a_list_of_operations_is_all_or_nothing() {
        let lib = sample();
        let ops = vec![
            Operation::CreatePackage { name: "A".into() },
            Operation::CreatePackage { name: "B".into() },
            Operation::CreatePackage { name: "A".into() },
        ];
        let (place, e) = apply_all(&lib, &ops).unwrap_err();
        assert_eq!((place, e.code), (2, NAME_USED));
        assert_eq!(lib.packages.len(), 2);
        let mut history = History::new(lib.clone(), 10);
        assert!(history.apply(&ops).is_err());
        assert_eq!(history.library(), &lib);
        assert!(!history.can_undo() && !history.dirty());
    }

    #[test]
    fn undo_and_redo_walk_through_the_steps() {
        let mut h = History::new(sample(), 10);
        assert!(!h.undo() && !h.redo());
        h.apply(&[Operation::CreatePackage { name: "A".into() }])
            .unwrap();
        h.apply(&[Operation::CreatePackage { name: "B".into() }])
            .unwrap();
        assert!(h.dirty());
        assert_eq!(h.undo_label(), Some("create footprint B"));
        assert_eq!(h.library().packages.len(), 4);
        assert!(h.undo());
        assert_eq!(h.library().packages.len(), 3);
        assert_eq!(h.redo_label(), Some("create footprint B"));
        assert!(h.redo());
        assert_eq!(h.library().packages.len(), 4);
        // a new step after an undo drops what could have been redone
        assert!(h.undo());
        h.apply(&[Operation::CreateSymbol { name: "S".into() }])
            .unwrap();
        assert!(!h.can_redo());
        assert!(h.undo() && h.undo());
        assert_eq!(h.library(), &sample());
        assert!(!h.dirty());
    }

    #[test]
    fn saving_marks_the_history_clean_until_the_next_change() {
        let mut h = History::new(sample(), 10);
        h.apply(&[Operation::CreatePackage { name: "A".into() }])
            .unwrap();
        assert!(h.dirty());
        h.mark_saved();
        assert!(!h.dirty());
        assert!(h.undo());
        assert!(h.dirty());
        assert!(h.redo());
        assert!(!h.dirty());
    }

    #[test]
    fn the_history_keeps_only_the_steps_of_its_limit() {
        let mut h = History::new(sample(), 2);
        for n in 0..4 {
            h.apply(&[Operation::CreatePackage {
                name: format!("P{n}"),
            }])
            .unwrap();
        }
        assert!(h.undo() && h.undo());
        assert!(!h.undo());
        assert_eq!(h.library().packages.len(), 4);
    }

    #[test]
    fn operations_are_read_from_json() {
        let text = r#"{"operations":[
            {"op":"create_package","name":"NEW"},
            {"op":"add_item","target":{"kind":"package","name":"NEW"},
             "item":{"type":"smd","name":"1","dx":800000,"dy":600000}},
            {"op":"set_opening","package":"NEW","pad":"1","kind":"mask","expansion":50000}
        ]}"#;
        let ops = parse_operations(text).unwrap();
        assert_eq!(ops.len(), 3);
        let lib = apply_all(&sample(), &ops).unwrap();
        assert_eq!(lib.package("NEW").unwrap().items.len(), 2);
        assert!(parse_operations("[]").unwrap().is_empty());
        for bad in ["{}", "3", "[{\"op\":\"fly\"}]", "not json"] {
            assert_eq!(
                parse_operations(bad).unwrap_err().code,
                OPERATIONS_INVALID,
                "{bad}"
            );
        }
    }

    #[test]
    fn rotations_must_be_written_as_the_file_does() {
        for ok in ["", "R0", "R90", "MR180", "SR45", "SMR45", "R-90", "R45.5"] {
            assert!(valid_rot(ok), "{ok}");
        }
        for bad in ["90", "R", "Rx", "XR90", "MMMR0", "R90 "] {
            assert!(!valid_rot(bad), "{bad}");
        }
    }

    #[test]
    fn labels_say_what_the_step_does() {
        let op = Operation::AddItem {
            target: pkg("X"),
            item: smd("1", 0),
            at: None,
        };
        assert_eq!(op.label(), "add smd to X");
        assert_eq!(
            Operation::MoveItem {
                target: pkg("X"),
                index: 3,
                dx: 0,
                dy: 0
            }
            .label(),
            "move item 3 of X"
        );
    }

    #[test]
    fn a_whole_copy_of_the_library_shares_what_an_edit_does_not_touch() {
        let lib = sample();
        let edited = apply_all(&lib, &[Operation::CreateSymbol { name: "S".into() }]).unwrap();
        assert!(Arc::ptr_eq(&lib.packages[0], &edited.packages[0]));
        assert!(Arc::ptr_eq(&lib.devicesets[0], &edited.devicesets[0]));
        assert_eq!(edited.symbols.len(), 2);
    }
}
