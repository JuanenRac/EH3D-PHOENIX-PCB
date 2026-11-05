// SPDX-License-Identifier: GPL-3.0-or-later
//! An EAGLE library in memory: footprints (packages), symbols and device sets, with the drawing
//! items of the first two as plain integers (nanometres, and micro-degrees for the curve of an
//! arc). Whatever the model does not understand is kept as the XML it came from, so that saving a
//! library does not lose it: the parts of the file outside the three sections stay in the
//! skeleton, an item the model cannot read exactly is kept as [`Item::Other`], and the attributes
//! of an item that the model does not use are kept in its `extra` list.
//!
//! The footprints, symbols and device sets are shared (`Arc`), so a copy of a whole library, which
//! is what the history of undo keeps, costs a list of pointers and not a copy of the drawing.

use std::sync::Arc;

use eh3d_phoenix_eagle::Element;
use serde::{Deserialize, Serialize};

/// The layer of the top copper.
pub const LAYER_TOP: u32 = 1;
/// The layer of the bottom copper.
pub const LAYER_BOTTOM: u32 = 16;
/// The layer of the top silkscreen of the part.
pub const LAYER_T_PLACE: u32 = 21;
/// The layer of the bottom silkscreen of the part.
pub const LAYER_B_PLACE: u32 = 22;
/// The layer of the text of the name.
pub const LAYER_T_NAMES: u32 = 25;
/// The layer of the text of the value.
pub const LAYER_T_VALUES: u32 = 27;
/// The layer of the top solder mask openings.
pub const LAYER_T_STOP: u32 = 29;
/// The layer of the bottom solder mask openings.
pub const LAYER_B_STOP: u32 = 30;
/// The layer of the top solder paste.
pub const LAYER_T_CREAM: u32 = 31;
/// The layer of the bottom solder paste.
pub const LAYER_B_CREAM: u32 = 32;
/// The layer of the keep-out of the top side (the courtyard).
pub const LAYER_T_KEEPOUT: u32 = 39;
/// The layer of the documentation of the top side (the assembly outline).
pub const LAYER_T_DOCU: u32 = 51;
/// The layer of the body of a symbol.
pub const LAYER_SYMBOLS: u32 = 94;
/// The layer of the name of a symbol.
pub const LAYER_NAMES: u32 = 95;
/// The layer of the value of a symbol.
pub const LAYER_VALUES: u32 = 96;

/// The shape of a through-hole pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadShape {
    /// A circle.
    #[default]
    Round,
    /// A square.
    Square,
    /// An octagon.
    Octagon,
    /// An oval, twice as long as it is wide.
    Long,
    /// An oval with the hole off the centre.
    Offset,
}

impl PadShape {
    /// The keyword of the file.
    pub fn keyword(self) -> &'static str {
        match self {
            PadShape::Round => "round",
            PadShape::Square => "square",
            PadShape::Octagon => "octagon",
            PadShape::Long => "long",
            PadShape::Offset => "offset",
        }
    }

    /// The shape of a keyword of the file.
    pub fn from_keyword(text: &str) -> Option<PadShape> {
        Some(match text {
            "round" => PadShape::Round,
            "square" => PadShape::Square,
            "octagon" => PadShape::Octagon,
            "long" => PadShape::Long,
            "offset" => PadShape::Offset,
            _ => return None,
        })
    }
}

/// A through-hole pad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pad {
    /// The name.
    pub name: String,
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The drill diameter.
    pub drill: i64,
    /// The diameter of the copper; zero lets the design rules decide.
    pub diameter: i64,
    /// The shape.
    pub shape: PadShape,
    /// The rotation as the file writes it (`R90`, `MR0`); empty for none.
    pub rot: String,
    /// True when the solder mask is opened automatically.
    pub stop: bool,
    /// True when the pad has thermals in a pour.
    pub thermals: bool,
    /// True for the first pad, which the file may mark.
    pub first: bool,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

impl Default for Pad {
    fn default() -> Self {
        Pad {
            name: String::new(),
            x: 0,
            y: 0,
            drill: 0,
            diameter: 0,
            shape: PadShape::Round,
            rot: String::new(),
            stop: true,
            thermals: true,
            first: false,
            extra: Vec::new(),
        }
    }
}

/// A surface-mount pad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Smd {
    /// The name.
    pub name: String,
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The width.
    pub dx: i64,
    /// The height.
    pub dy: i64,
    /// The copper layer, 1 for the top and 16 for the bottom.
    pub layer: u32,
    /// How round the corners are, 0 to 100.
    pub roundness: u32,
    /// The rotation as the file writes it.
    pub rot: String,
    /// True when the solder mask is opened automatically.
    pub stop: bool,
    /// True when the pad has thermals in a pour.
    pub thermals: bool,
    /// True when solder paste is printed automatically.
    pub cream: bool,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

impl Default for Smd {
    fn default() -> Self {
        Smd {
            name: String::new(),
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            layer: LAYER_TOP,
            roundness: 0,
            rot: String::new(),
            stop: true,
            thermals: true,
            cream: true,
            extra: Vec::new(),
        }
    }
}

/// A line or an arc.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Wire {
    /// The layer.
    pub layer: u32,
    /// The first end.
    pub x1: i64,
    /// The first end.
    pub y1: i64,
    /// The second end.
    pub x2: i64,
    /// The second end.
    pub y2: i64,
    /// The line width.
    pub width: i64,
    /// The angle of the arc in micro-degrees, zero for a straight line.
    pub curve: i64,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// A filled rectangle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Rect {
    /// The layer.
    pub layer: u32,
    /// One corner.
    pub x1: i64,
    /// One corner.
    pub y1: i64,
    /// The opposite corner.
    pub x2: i64,
    /// The opposite corner.
    pub y2: i64,
    /// The rotation as the file writes it.
    pub rot: String,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// A circle, filled when its width is zero.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Circle {
    /// The layer.
    pub layer: u32,
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The radius.
    pub radius: i64,
    /// The line width.
    pub width: i64,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// A text.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Text {
    /// The layer.
    pub layer: u32,
    /// The position.
    pub x: i64,
    /// The position.
    pub y: i64,
    /// The height.
    pub size: i64,
    /// The rotation as the file writes it.
    pub rot: String,
    /// The text; `>NAME` and `>VALUE` stand for the name and the value of the part.
    pub content: String,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// A hole that is not a pad.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hole {
    /// The centre.
    pub x: i64,
    /// The centre.
    pub y: i64,
    /// The drill diameter.
    pub drill: i64,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// A pin of a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pin {
    /// The name. The number of the pad it reaches is given by the connections of each device.
    pub name: String,
    /// The connection point.
    pub x: i64,
    /// The connection point.
    pub y: i64,
    /// `point`, `short`, `middle` or `long`.
    pub length: String,
    /// The electrical type: `nc`, `in`, `out`, `io`, `oc`, `pwr`, `pas`, `hiz` or `sup`.
    pub direction: String,
    /// `none`, `dot`, `clk` or `dotclk`.
    pub function: String,
    /// `off`, `pad`, `pin` or `both`.
    pub visible: String,
    /// The swap level.
    pub swaplevel: String,
    /// The rotation as the file writes it, which gives the orientation.
    pub rot: String,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

impl Default for Pin {
    fn default() -> Self {
        Pin {
            name: String::new(),
            x: 0,
            y: 0,
            length: "long".to_owned(),
            direction: "io".to_owned(),
            function: "none".to_owned(),
            visible: "both".to_owned(),
            swaplevel: "0".to_owned(),
            rot: String::new(),
            extra: Vec::new(),
        }
    }
}

/// One drawing item of a footprint or a symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    /// A through-hole pad.
    Pad(Pad),
    /// A surface-mount pad.
    Smd(Smd),
    /// A line or an arc.
    Wire(Wire),
    /// A rectangle.
    Rect(Rect),
    /// A circle.
    Circle(Circle),
    /// A text.
    Text(Text),
    /// A hole.
    Hole(Hole),
    /// A pin of a symbol.
    Pin(Pin),
    /// An element that the model does not read exactly (a polygon, a dimension, a value that
    /// does not fit a nanometre); kept as it came, and not editable.
    #[serde(skip)]
    Other(Element),
}

impl Item {
    /// The kind of the item as a word.
    pub fn kind(&self) -> &'static str {
        match self {
            Item::Pad(_) => "pad",
            Item::Smd(_) => "smd",
            Item::Wire(w) => {
                if w.curve == 0 {
                    "line"
                } else {
                    "arc"
                }
            }
            Item::Rect(_) => "rectangle",
            Item::Circle(_) => "circle",
            Item::Text(_) => "text",
            Item::Hole(_) => "hole",
            Item::Pin(_) => "pin",
            Item::Other(_) => "other",
        }
    }

    /// The name of a pad or a pin; `None` for the other items.
    pub fn name(&self) -> Option<&str> {
        match self {
            Item::Pad(p) => Some(&p.name),
            Item::Smd(p) => Some(&p.name),
            Item::Pin(p) => Some(&p.name),
            _ => None,
        }
    }

    /// The layer, when the item has one.
    pub fn layer(&self) -> Option<u32> {
        match self {
            Item::Smd(i) => Some(i.layer),
            Item::Wire(i) => Some(i.layer),
            Item::Rect(i) => Some(i.layer),
            Item::Circle(i) => Some(i.layer),
            Item::Text(i) => Some(i.layer),
            _ => None,
        }
    }

    /// True for a pad of either kind.
    pub fn is_pad(&self) -> bool {
        matches!(self, Item::Pad(_) | Item::Smd(_))
    }

    /// The centre of a pad of either kind.
    pub fn pad_centre(&self) -> Option<(i64, i64)> {
        match self {
            Item::Pad(p) => Some((p.x, p.y)),
            Item::Smd(p) => Some((p.x, p.y)),
            _ => None,
        }
    }
}

/// A footprint.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Package {
    /// The name.
    pub name: String,
    /// The description elements, kept as they came.
    pub description: Vec<Element>,
    /// The drawing items in file order.
    pub items: Vec<Item>,
    /// The attributes of the file that the model does not use.
    pub extra: Vec<(String, String)>,
}

impl Package {
    /// The pads of either kind, with their place in the list of items.
    pub fn pads(&self) -> impl Iterator<Item = (usize, &Item)> {
        self.items.iter().enumerate().filter(|(_, i)| i.is_pad())
    }

    /// The place of the pad called `name`.
    pub fn pad_index(&self, name: &str) -> Option<usize> {
        self.pads()
            .find(|(_, i)| i.name() == Some(name))
            .map(|(n, _)| n)
    }
}

/// A symbol.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Symbol {
    /// The name.
    pub name: String,
    /// The description elements, kept as they came.
    pub description: Vec<Element>,
    /// The drawing items in file order.
    pub items: Vec<Item>,
    /// The attributes of the file that the model does not use.
    pub extra: Vec<(String, String)>,
}

impl Symbol {
    /// The pins, with their place in the list of items.
    pub fn pins(&self) -> impl Iterator<Item = (usize, &Pin)> {
        self.items.iter().enumerate().filter_map(|(n, i)| match i {
            Item::Pin(p) => Some((n, p)),
            _ => None,
        })
    }
}

/// A gate of a device set: a symbol placed in the device.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Gate {
    /// The name.
    pub name: String,
    /// The symbol it uses.
    pub symbol: String,
    /// Where it is placed in the editor of the device.
    pub x: i64,
    /// Where it is placed in the editor of the device.
    pub y: i64,
    /// The attributes of the file that the model does not use.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// The connection of a pin of a gate to a pad.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Connect {
    /// The gate.
    pub gate: String,
    /// The pin of the symbol of the gate.
    pub pin: String,
    /// The pad of the footprint of the device (several pads are written separated by spaces).
    pub pad: String,
    /// The attributes of the file that the model does not use (`route`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

/// A device: a footprint and the connections of the pins to its pads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Device {
    /// The name (the variant).
    pub name: String,
    /// The footprint, when it has one.
    pub package: Option<String>,
    /// The connections.
    pub connects: Vec<Connect>,
    /// The elements after the connections (technologies), kept as they came.
    pub rest: Vec<Element>,
    /// The attributes of the file that the model does not use.
    pub extra: Vec<(String, String)>,
}

/// A device set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceSet {
    /// The name.
    pub name: String,
    /// The prefix of the reference designators.
    pub prefix: String,
    /// True when the value of the part is typed by the user.
    pub uservalue: bool,
    /// The description elements, kept as they came.
    pub description: Vec<Element>,
    /// The gates.
    pub gates: Vec<Gate>,
    /// The devices.
    pub devices: Vec<Device>,
    /// The elements after the devices, kept as they came.
    pub rest: Vec<Element>,
    /// The attributes of the file that the model does not use.
    pub extra: Vec<(String, String)>,
}

/// A library.
#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    /// The whole file as a tree, in which the three sections of the library are empty: what the
    /// model does not read stays here and is written back.
    pub skeleton: Element,
    /// The footprints.
    pub packages: Vec<Arc<Package>>,
    /// The symbols.
    pub symbols: Vec<Arc<Symbol>>,
    /// The device sets.
    pub devicesets: Vec<Arc<DeviceSet>>,
    /// How many items were kept as the XML they came from because the model could not read them
    /// exactly.
    pub kept_raw: usize,
}

/// The kind of thing a name belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A footprint.
    Package,
    /// A symbol.
    Symbol,
    /// A device set.
    DeviceSet,
}

impl Kind {
    /// The word for the kind in a report.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Package => "package",
            Kind::Symbol => "symbol",
            Kind::DeviceSet => "deviceset",
        }
    }
}

impl Library {
    /// An empty library with the skeleton of a file of EAGLE 9.
    pub fn empty() -> Library {
        let skeleton = el(
            "eagle",
            &[("version", "9.6.2")],
            vec![el(
                "drawing",
                &[],
                vec![
                    el("settings", &[], Vec::new()),
                    el("layers", &[], default_layers()),
                    el("library", &[], Vec::new()),
                ],
            )],
        );
        Library {
            skeleton,
            packages: Vec::new(),
            symbols: Vec::new(),
            devicesets: Vec::new(),
            kept_raw: 0,
        }
    }

    /// The place of the footprint called `name`.
    pub fn package_index(&self, name: &str) -> Option<usize> {
        self.packages.iter().position(|p| p.name == name)
    }

    /// The place of the symbol called `name`.
    pub fn symbol_index(&self, name: &str) -> Option<usize> {
        self.symbols.iter().position(|p| p.name == name)
    }

    /// The place of the device set called `name`.
    pub fn deviceset_index(&self, name: &str) -> Option<usize> {
        self.devicesets.iter().position(|p| p.name == name)
    }

    /// The footprint called `name`.
    pub fn package(&self, name: &str) -> Option<&Package> {
        self.package_index(name).map(|i| self.packages[i].as_ref())
    }

    /// The symbol called `name`.
    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbol_index(name).map(|i| self.symbols[i].as_ref())
    }

    /// The device set called `name`.
    pub fn deviceset(&self, name: &str) -> Option<&DeviceSet> {
        self.deviceset_index(name)
            .map(|i| self.devicesets[i].as_ref())
    }

    /// The names of the footprints, symbols or device sets, in file order.
    pub fn names(&self, kind: Kind) -> Vec<String> {
        match kind {
            Kind::Package => self.packages.iter().map(|p| p.name.clone()).collect(),
            Kind::Symbol => self.symbols.iter().map(|p| p.name.clone()).collect(),
            Kind::DeviceSet => self.devicesets.iter().map(|p| p.name.clone()).collect(),
        }
    }
}

/// An element from a name, attributes and children.
fn el(name: &str, attrs: &[(&str, &str)], children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs: attrs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        text: String::new(),
        children,
    }
}

/// The layers a new library declares: the ones its footprints and symbols draw on.
fn default_layers() -> Vec<Element> {
    const LAYERS: [(&str, &str); 15] = [
        ("1", "Top"),
        ("16", "Bottom"),
        ("21", "tPlace"),
        ("22", "bPlace"),
        ("25", "tNames"),
        ("27", "tValues"),
        ("29", "tStop"),
        ("30", "bStop"),
        ("31", "tCream"),
        ("32", "bCream"),
        ("39", "tKeepout"),
        ("51", "tDocu"),
        ("94", "Symbols"),
        ("95", "Names"),
        ("96", "Values"),
    ];
    LAYERS
        .iter()
        .map(|(number, name)| {
            el(
                "layer",
                &[
                    ("number", *number),
                    ("name", *name),
                    ("color", "1"),
                    ("fill", "1"),
                    ("visible", "yes"),
                    ("active", "yes"),
                ],
                Vec::new(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_have_keywords_that_read_back() {
        for shape in [
            PadShape::Round,
            PadShape::Square,
            PadShape::Octagon,
            PadShape::Long,
            PadShape::Offset,
        ] {
            assert_eq!(PadShape::from_keyword(shape.keyword()), Some(shape));
        }
        assert_eq!(PadShape::from_keyword("hexagon"), None);
    }

    #[test]
    fn an_empty_library_has_a_library_element_and_its_layers() {
        let lib = Library::empty();
        let drawing = lib.skeleton.child("drawing").unwrap();
        assert!(drawing.child("library").is_some());
        assert_eq!(drawing.child("layers").unwrap().children.len(), 15);
        assert!(lib.packages.is_empty() && lib.symbols.is_empty() && lib.devicesets.is_empty());
    }

    #[test]
    fn items_say_their_kind_and_name() {
        let pad = Item::Smd(Smd {
            name: "1".into(),
            ..Smd::default()
        });
        assert_eq!(pad.kind(), "smd");
        assert_eq!(pad.name(), Some("1"));
        assert_eq!(pad.layer(), Some(LAYER_TOP));
        let arc = Item::Wire(Wire {
            curve: 90_000_000,
            ..Wire::default()
        });
        assert_eq!(arc.kind(), "arc");
        assert_eq!(Item::Wire(Wire::default()).kind(), "line");
        assert!(!arc.is_pad());
    }

    #[test]
    fn an_item_reads_from_json_with_the_defaults_of_the_file() {
        let item: Item =
            serde_json::from_str(r#"{"type":"smd","name":"1","dx":900000,"dy":800000}"#).unwrap();
        match item {
            Item::Smd(s) => {
                assert_eq!((s.layer, s.stop, s.cream), (1, true, true));
                assert_eq!((s.dx, s.dy, s.roundness), (900_000, 800_000, 0));
            }
            other => panic!("{other:?}"),
        }
        assert!(serde_json::from_str::<Item>(r#"{"type":"nothing"}"#).is_err());
    }
}
