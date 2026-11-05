// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading an EAGLE library file (`.lbr`) into the model. A value the model cannot hold exactly
//! (a number with more than six decimals, an exponent, a missing coordinate) makes the whole item
//! stay as the XML it came from, so that nothing is rounded or invented.

use std::sync::Arc;

use eh3d_phoenix_eagle::xml::parse;
use eh3d_phoenix_eagle::{Element, Limits};
use eh3d_phoenix_model::Nm;

use crate::codes::{LibError, FILE_UNUSABLE};
use crate::model::{
    Circle, Connect, Device, DeviceSet, Gate, Hole, Item, Library, Package, Pad, PadShape, Pin,
    Rect, Smd, Symbol, Text, Wire,
};

/// A length or an angle as an exact integer of millionths; `None` when the text has more
/// decimals than that or is not a plain decimal.
pub(crate) fn exact_millionths(text: &str) -> Option<i64> {
    match Nm::parse_mm(text.trim()) {
        Ok((value, true)) => Some(value.get()),
        _ => None,
    }
}

/// The attributes of an element, read one by one; what is not read is kept as extra.
struct Attrs<'a> {
    element: &'a Element,
    used: Vec<&'static str>,
    ok: bool,
}

impl<'a> Attrs<'a> {
    fn new(element: &'a Element) -> Attrs<'a> {
        Attrs {
            element,
            used: Vec::new(),
            ok: true,
        }
    }

    fn text(&mut self, name: &'static str, default: &str) -> String {
        self.used.push(name);
        self.element.attr(name).unwrap_or(default).to_owned()
    }

    fn required_text(&mut self, name: &'static str) -> String {
        self.used.push(name);
        match self.element.attr(name) {
            Some(v) => v.to_owned(),
            None => {
                self.ok = false;
                String::new()
            }
        }
    }

    fn num(&mut self, name: &'static str, required: bool) -> i64 {
        self.used.push(name);
        match self.element.attr(name) {
            None => {
                if required {
                    self.ok = false;
                }
                0
            }
            Some(text) => match exact_millionths(text) {
                Some(v) => v,
                None => {
                    self.ok = false;
                    0
                }
            },
        }
    }

    fn layer(&mut self, name: &'static str, default: u32) -> u32 {
        self.used.push(name);
        match self.element.attr(name) {
            None => default,
            Some(text) => match text.trim().parse::<u32>() {
                Ok(v) => v,
                Err(_) => {
                    self.ok = false;
                    default
                }
            },
        }
    }

    fn percent(&mut self, name: &'static str) -> u32 {
        self.used.push(name);
        match self.element.attr(name) {
            None => 0,
            Some(text) => match text.trim().parse::<u32>() {
                Ok(v) if v <= 100 => v,
                _ => {
                    self.ok = false;
                    0
                }
            },
        }
    }

    fn flag(&mut self, name: &'static str, default: bool) -> bool {
        self.used.push(name);
        match self.element.attr(name) {
            None => default,
            Some("yes") => true,
            Some("no") => false,
            Some(_) => {
                self.ok = false;
                default
            }
        }
    }

    fn extra(&self) -> Vec<(String, String)> {
        self.element
            .attrs
            .iter()
            .filter(|(k, _)| !self.used.contains(&k.as_str()))
            .cloned()
            .collect()
    }
}

/// An item from its element; the element itself when the model cannot hold it exactly. The
/// second value is true in that case.
fn read_item(e: &Element) -> (Item, bool) {
    let mut a = Attrs::new(e);
    let plain = e.children.is_empty();
    let item = match e.name.as_str() {
        "pad" => {
            let name = a.required_text("name");
            let x = a.num("x", true);
            let y = a.num("y", true);
            let drill = a.num("drill", true);
            let diameter = a.num("diameter", false);
            let shape_text = a.text("shape", "round");
            let shape = PadShape::from_keyword(&shape_text);
            let rot = a.text("rot", "");
            let stop = a.flag("stop", true);
            let thermals = a.flag("thermals", true);
            let first = a.flag("first", false);
            let extra = a.extra();
            shape.map(|shape| {
                Item::Pad(Pad {
                    name,
                    x,
                    y,
                    drill,
                    diameter,
                    shape,
                    rot,
                    stop,
                    thermals,
                    first,
                    extra,
                })
            })
        }
        "smd" => {
            let name = a.required_text("name");
            let x = a.num("x", true);
            let y = a.num("y", true);
            let dx = a.num("dx", true);
            let dy = a.num("dy", true);
            let layer = a.layer("layer", 1);
            let roundness = a.percent("roundness");
            let rot = a.text("rot", "");
            let stop = a.flag("stop", true);
            let thermals = a.flag("thermals", true);
            let cream = a.flag("cream", true);
            Some(Item::Smd(Smd {
                name,
                x,
                y,
                dx,
                dy,
                layer,
                roundness,
                rot,
                stop,
                thermals,
                cream,
                extra: a.extra(),
            }))
        }
        "wire" => {
            let x1 = a.num("x1", true);
            let y1 = a.num("y1", true);
            let x2 = a.num("x2", true);
            let y2 = a.num("y2", true);
            let width = a.num("width", true);
            let layer = a.layer("layer", 0);
            let curve = a.num("curve", false);
            Some(Item::Wire(Wire {
                layer,
                x1,
                y1,
                x2,
                y2,
                width,
                curve,
                extra: a.extra(),
            }))
        }
        "rectangle" => {
            let x1 = a.num("x1", true);
            let y1 = a.num("y1", true);
            let x2 = a.num("x2", true);
            let y2 = a.num("y2", true);
            let layer = a.layer("layer", 0);
            let rot = a.text("rot", "");
            Some(Item::Rect(Rect {
                layer,
                x1,
                y1,
                x2,
                y2,
                rot,
                extra: a.extra(),
            }))
        }
        "circle" => {
            let x = a.num("x", true);
            let y = a.num("y", true);
            let radius = a.num("radius", true);
            let width = a.num("width", true);
            let layer = a.layer("layer", 0);
            Some(Item::Circle(Circle {
                layer,
                x,
                y,
                radius,
                width,
                extra: a.extra(),
            }))
        }
        "hole" => {
            let x = a.num("x", true);
            let y = a.num("y", true);
            let drill = a.num("drill", true);
            Some(Item::Hole(Hole {
                x,
                y,
                drill,
                extra: a.extra(),
            }))
        }
        "pin" => {
            let name = a.required_text("name");
            let x = a.num("x", true);
            let y = a.num("y", true);
            let length = a.text("length", "long");
            let direction = a.text("direction", "io");
            let function = a.text("function", "none");
            let visible = a.text("visible", "both");
            let swaplevel = a.text("swaplevel", "0");
            let rot = a.text("rot", "");
            Some(Item::Pin(Pin {
                name,
                x,
                y,
                length,
                direction,
                function,
                visible,
                swaplevel,
                rot,
                extra: a.extra(),
            }))
        }
        "text" => {
            let x = a.num("x", true);
            let y = a.num("y", true);
            let size = a.num("size", true);
            let layer = a.layer("layer", 0);
            let rot = a.text("rot", "");
            Some(Item::Text(Text {
                layer,
                x,
                y,
                size,
                rot,
                content: e.text.clone(),
                extra: a.extra(),
            }))
        }
        _ => None,
    };
    match item {
        Some(item) if a.ok && plain => (item, false),
        _ => (Item::Other(e.clone()), true),
    }
}

fn read_items(children: &[Element], kept_raw: &mut usize) -> (Vec<Element>, Vec<Item>) {
    let mut description = Vec::new();
    let mut items = Vec::new();
    for c in children {
        if c.name == "description" {
            description.push(c.clone());
        } else {
            let (item, raw) = read_item(c);
            if raw {
                *kept_raw += 1;
            }
            items.push(item);
        }
    }
    (description, items)
}

fn name_and_extra(e: &Element) -> (String, Vec<(String, String)>) {
    let name = e.attr("name").unwrap_or("").to_owned();
    let extra = e
        .attrs
        .iter()
        .filter(|(k, _)| k != "name")
        .cloned()
        .collect();
    (name, extra)
}

fn read_package(e: &Element, kept_raw: &mut usize) -> Package {
    let (name, extra) = name_and_extra(e);
    let (description, items) = read_items(&e.children, kept_raw);
    Package {
        name,
        description,
        items,
        extra,
    }
}

fn read_symbol(e: &Element, kept_raw: &mut usize) -> Symbol {
    let (name, extra) = name_and_extra(e);
    let (description, items) = read_items(&e.children, kept_raw);
    Symbol {
        name,
        description,
        items,
        extra,
    }
}

fn read_gate(e: &Element) -> Gate {
    let mut a = Attrs::new(e);
    let name = a.text("name", "");
    let symbol = a.text("symbol", "");
    let x = a.num("x", false);
    let y = a.num("y", false);
    Gate {
        name,
        symbol,
        x,
        y,
        extra: a.extra(),
    }
}

fn read_device(e: &Element) -> Device {
    let mut a = Attrs::new(e);
    let name = a.text("name", "");
    let package = e
        .attr("package")
        .filter(|p| !p.is_empty())
        .map(str::to_owned);
    a.used.push("package");
    let mut connects = Vec::new();
    let mut rest = Vec::new();
    for c in &e.children {
        if c.name == "connects" {
            for k in c.children_named("connect") {
                let mut b = Attrs::new(k);
                let gate = b.text("gate", "");
                let pin = b.text("pin", "");
                let pad = b.text("pad", "");
                connects.push(Connect {
                    gate,
                    pin,
                    pad,
                    extra: b.extra(),
                });
            }
        } else {
            rest.push(c.clone());
        }
    }
    Device {
        name,
        package,
        connects,
        rest,
        extra: a.extra(),
    }
}

fn read_deviceset(e: &Element) -> DeviceSet {
    let mut a = Attrs::new(e);
    let name = a.text("name", "");
    let prefix = a.text("prefix", "");
    let uservalue = a.flag("uservalue", false);
    let mut set = DeviceSet {
        name,
        prefix,
        uservalue,
        extra: a.extra(),
        ..DeviceSet::default()
    };
    for c in &e.children {
        match c.name.as_str() {
            "description" => set.description.push(c.clone()),
            "gates" => set.gates.extend(c.children_named("gate").map(read_gate)),
            "devices" => set
                .devices
                .extend(c.children_named("device").map(read_device)),
            _ => set.rest.push(c.clone()),
        }
    }
    set
}

/// Turns a tree of a file into a library.
pub fn from_tree(mut root: Element) -> Result<Library, LibError> {
    let unusable = |what: &str| LibError::new(FILE_UNUSABLE, what.to_owned());
    if root.name != "eagle" {
        return Err(unusable("the root element is not eagle"));
    }
    let Some(drawing) = root.children.iter_mut().find(|c| c.name == "drawing") else {
        return Err(unusable("there is no drawing"));
    };
    if drawing.child("board").is_some() {
        return Err(unusable("the file is a board, not a library"));
    }
    if drawing.child("schematic").is_some() {
        return Err(unusable("the file is a schematic, not a library"));
    }
    let Some(library) = drawing.children.iter_mut().find(|c| c.name == "library") else {
        return Err(unusable("the drawing has no library"));
    };
    let mut package_elements = Vec::new();
    let mut symbol_elements = Vec::new();
    let mut set_elements = Vec::new();
    for section in library.children.iter_mut() {
        match section.name.as_str() {
            "packages" => package_elements.append(&mut section.children),
            "symbols" => symbol_elements.append(&mut section.children),
            "devicesets" => set_elements.append(&mut section.children),
            _ => {}
        }
    }
    let mut kept_raw = 0;
    let packages = package_elements
        .iter()
        .filter(|e| e.name == "package")
        .map(|e| Arc::new(read_package(e, &mut kept_raw)))
        .collect();
    let symbols = symbol_elements
        .iter()
        .filter(|e| e.name == "symbol")
        .map(|e| Arc::new(read_symbol(e, &mut kept_raw)))
        .collect();
    let devicesets = set_elements
        .iter()
        .filter(|e| e.name == "deviceset")
        .map(|e| Arc::new(read_deviceset(e)))
        .collect();
    Ok(Library {
        skeleton: root,
        packages,
        symbols,
        devicesets,
        kept_raw,
    })
}

/// Reads the bytes of a library file.
pub fn read_library(bytes: &[u8], limits: &Limits) -> Result<Library, LibError> {
    let unusable = |what: String| LibError::new(FILE_UNUSABLE, what);
    if bytes.first() == Some(&0x10) {
        return Err(unusable(
            "the file starts like an EAGLE binary file, which this editor does not read".to_owned(),
        ));
    }
    let text: std::borrow::Cow<'_, str> = match std::str::from_utf8(bytes) {
        Ok(t) => t.into(),
        Err(_) => bytes.iter().map(|&b| b as char).collect::<String>().into(),
    };
    let root = parse(&text, limits).map_err(|e| unusable(e.to_string()))?;
    from_tree(root)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small library written by hand: a resistor footprint with two pads and silkscreen, a
    /// two-pin symbol, a device set that joins them and a polygon the model keeps as it is.
    pub(crate) const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<settings>
<setting alwaysvectorfont="no"/>
</settings>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="21" name="tPlace" color="7" fill="1" visible="yes" active="yes"/>
</layers>
<library>
<description>Test library</description>
<packages>
<package name="R0603">
<description>Chip resistor</description>
<smd name="1" x="-0.8" y="0" dx="0.8" dy="0.9" layer="1"/>
<smd name="2" x="0.8" y="0" dx="0.8" dy="0.9" layer="1" roundness="25"/>
<wire x1="-0.3" y1="0.7" x2="0.3" y2="0.7" width="0.15" layer="21"/>
<wire x1="-0.3" y1="-0.7" x2="0.3" y2="-0.7" width="0.15" layer="21" curve="-45"/>
<text x="0" y="1.2" size="1" layer="25" align="bottom-center">&gt;NAME</text>
<text x="0" y="-1.2" size="1" layer="27">&gt;VALUE</text>
<polygon width="0.1" layer="39">
<vertex x="0" y="0"/>
<vertex x="1" y="0"/>
<vertex x="1" y="1"/>
</polygon>
</package>
<package name="HDR1X2">
<pad name="1" x="0" y="0" drill="1" diameter="1.7" shape="square"/>
<pad name="2" x="2.54" y="0" drill="1" shape="long" rot="R90"/>
</package>
</packages>
<symbols>
<symbol name="R">
<wire x1="-1.27" y1="0.508" x2="1.27" y2="0.508" width="0.254" layer="94"/>
<pin name="1" x="-5.08" y="0" visible="off" length="short" direction="pas" swaplevel="1"/>
<pin name="2" x="5.08" y="0" visible="off" length="short" direction="pas" swaplevel="1" rot="R180"/>
<text x="0" y="1" size="1.778" layer="95">&gt;NAME</text>
</symbol>
</symbols>
<devicesets>
<deviceset name="R" prefix="R" uservalue="yes">
<description>Resistor</description>
<gates>
<gate name="G$1" symbol="R" x="0" y="0"/>
</gates>
<devices>
<device name="0603" package="R0603">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
<technologies>
<technology name=""/>
</technologies>
</device>
<device name="" package="">
</device>
</devices>
</deviceset>
</devicesets>
</library>
</drawing>
</eagle>
"#;

    pub(crate) fn sample() -> Library {
        read_library(SAMPLE.as_bytes(), &Limits::default()).unwrap()
    }

    #[test]
    fn the_sample_is_read_into_footprints_symbols_and_device_sets() {
        let lib = sample();
        assert_eq!(lib.names(crate::model::Kind::Package), ["R0603", "HDR1X2"]);
        assert_eq!(lib.names(crate::model::Kind::Symbol), ["R"]);
        assert_eq!(lib.names(crate::model::Kind::DeviceSet), ["R"]);
        let r = lib.package("R0603").unwrap();
        assert_eq!(r.description.len(), 1);
        assert_eq!(r.items.len(), 7);
        match &r.items[1] {
            Item::Smd(s) => {
                assert_eq!(
                    (s.x, s.dx, s.dy, s.roundness),
                    (800_000, 800_000, 900_000, 25)
                );
            }
            other => panic!("{other:?}"),
        }
        match &r.items[3] {
            Item::Wire(w) => assert_eq!(w.curve, -45_000_000),
            other => panic!("{other:?}"),
        }
        match &r.items[4] {
            Item::Text(t) => {
                assert_eq!(t.content, ">NAME");
                assert_eq!(
                    t.extra,
                    vec![("align".to_owned(), "bottom-center".to_owned())]
                );
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(r.items[6], Item::Other(_)));
        assert_eq!(lib.kept_raw, 1);
    }

    #[test]
    fn pads_keep_their_shape_and_flags() {
        let lib = sample();
        let h = lib.package("HDR1X2").unwrap();
        match (&h.items[0], &h.items[1]) {
            (Item::Pad(a), Item::Pad(b)) => {
                assert_eq!(
                    (a.shape, a.drill, a.diameter),
                    (PadShape::Square, 1_000_000, 1_700_000)
                );
                assert_eq!(
                    (b.shape, b.diameter, b.rot.as_str()),
                    (PadShape::Long, 0, "R90")
                );
                assert!(a.stop && a.thermals && !a.first);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn symbols_and_device_sets_are_read_with_their_connections() {
        let lib = sample();
        let s = lib.symbol("R").unwrap();
        let pins: Vec<_> = s.pins().map(|(_, p)| p.name.clone()).collect();
        assert_eq!(pins, ["1", "2"]);
        let first = s.pins().next().unwrap().1;
        assert_eq!(
            (
                first.length.as_str(),
                first.direction.as_str(),
                first.visible.as_str()
            ),
            ("short", "pas", "off")
        );
        let set = lib.deviceset("R").unwrap();
        assert!(set.uservalue);
        assert_eq!(set.gates.len(), 1);
        assert_eq!(set.devices.len(), 2);
        assert_eq!(set.devices[0].connects.len(), 2);
        assert_eq!(set.devices[0].package.as_deref(), Some("R0603"));
        assert_eq!(set.devices[0].rest.len(), 1);
    }

    #[test]
    fn a_number_with_too_many_decimals_keeps_the_item_as_it_came() {
        let text = SAMPLE.replace(
            "x=\"0.8\" y=\"0\" dx=\"0.8\"",
            "x=\"0.8000001\" y=\"0\" dx=\"0.8\"",
        );
        let lib = read_library(text.as_bytes(), &Limits::default()).unwrap();
        let r = lib.package("R0603").unwrap();
        assert!(matches!(r.items[1], Item::Other(_)));
        assert_eq!(lib.kept_raw, 2);
    }

    #[test]
    fn files_that_are_not_libraries_are_refused_with_the_code() {
        let board =
            "<?xml version=\"1.0\"?><eagle version=\"9.6.2\"><drawing><board/></drawing></eagle>";
        let e = read_library(board.as_bytes(), &Limits::default()).unwrap_err();
        assert_eq!(e.code, FILE_UNUSABLE);
        assert!(e.detail.contains("board"));
        let e = read_library(b"<other/>", &Limits::default()).unwrap_err();
        assert_eq!(e.code, FILE_UNUSABLE);
        let e = read_library(&[0x10, 0x80, 0x00], &Limits::default()).unwrap_err();
        assert!(e.detail.contains("binary"));
        let e = read_library(b"<eagle><drawing/></eagle>", &Limits::default()).unwrap_err();
        assert!(e.detail.contains("no library"));
    }

    #[test]
    fn exact_millionths_refuses_what_it_would_round() {
        assert_eq!(exact_millionths("1.27"), Some(1_270_000));
        assert_eq!(exact_millionths(" -0.05 "), Some(-50_000));
        assert_eq!(exact_millionths("0.1234567"), None);
        assert_eq!(exact_millionths("abc"), None);
    }
}
