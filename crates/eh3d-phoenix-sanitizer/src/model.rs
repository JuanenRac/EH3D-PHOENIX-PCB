// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing items of a library, read from the XML tree into plain integers (nanometres and
//! micro-degrees) so that they can be hashed and compared. An item whose coordinates are missing
//! or cannot be read is skipped and counted; nothing here panics on a damaged file.

use std::collections::BTreeSet;

use eh3d_phoenix_eagle::Element;
use eh3d_phoenix_model::{Nm, Udeg};

/// One drawing item of a footprint or a symbol. Lengths are nanometres, angles micro-degrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prim {
    /// A through-hole pad or a surface-mount pad.
    Pad {
        /// The pad name.
        name: String,
        /// True for a through-hole pad.
        thru: bool,
        /// The centre.
        x: i64,
        /// The centre.
        y: i64,
        /// The width (the diameter of a through-hole pad).
        w: i64,
        /// The height (the diameter of a through-hole pad).
        h: i64,
        /// The drill, zero for a surface-mount pad.
        drill: i64,
        /// The shape keyword, with the roundness of a surface-mount pad.
        shape: String,
        /// The layer of a surface-mount pad, zero for a through-hole pad.
        layer: u32,
        /// The rotation.
        rot: i64,
    },
    /// A line or an arc.
    Wire {
        /// The layer number.
        layer: u32,
        /// First end.
        x1: i64,
        /// First end.
        y1: i64,
        /// Second end.
        x2: i64,
        /// Second end.
        y2: i64,
        /// The width.
        width: i64,
        /// The angle of the arc, zero for a line.
        curve: i64,
    },
    /// A rectangle, turned about its centre.
    Rect {
        /// The layer number.
        layer: u32,
        /// One corner.
        x1: i64,
        /// One corner.
        y1: i64,
        /// The opposite corner.
        x2: i64,
        /// The opposite corner.
        y2: i64,
        /// The rotation.
        rot: i64,
    },
    /// A circle.
    Circle {
        /// The layer number.
        layer: u32,
        /// The centre.
        x: i64,
        /// The centre.
        y: i64,
        /// The radius.
        r: i64,
        /// The line width, zero for a filled circle.
        width: i64,
    },
    /// A polygon: the corners in order, each with the angle of the arc that leaves it.
    Poly {
        /// The layer number.
        layer: u32,
        /// The line width.
        width: i64,
        /// The corners: x, y and the curve to the next corner.
        verts: Vec<(i64, i64, i64)>,
    },
    /// A hole that is not a pad.
    Hole {
        /// The centre.
        x: i64,
        /// The centre.
        y: i64,
        /// The drill.
        drill: i64,
    },
    /// A text.
    Text {
        /// The layer number.
        layer: u32,
        /// The position.
        x: i64,
        /// The position.
        y: i64,
        /// The height.
        size: i64,
        /// The rotation.
        rot: i64,
        /// The text itself.
        content: String,
    },
    /// A pin of a symbol.
    Pin {
        /// The pin name.
        name: String,
        /// The connection point.
        x: i64,
        /// The connection point.
        y: i64,
        /// The rotation.
        rot: i64,
        /// The length keyword (`point`, `short`, `middle`, `long`).
        length: String,
        /// The electrical direction keyword.
        direction: String,
        /// The function keyword (`none`, `dot`, `clk`, `dotclk`).
        function: String,
        /// The visibility keyword of the name and the pad.
        visible: String,
        /// The swap level.
        swaplevel: String,
    },
}

/// A footprint or a symbol of a library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The name in its library.
    pub name: String,
    /// True when the item has a description.
    pub has_description: bool,
    /// The drawing items.
    pub prims: Vec<Prim>,
}

/// One device of a device set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceData {
    /// The footprint it uses, if any.
    pub package: Option<String>,
    /// The connections: gate, pin and pad.
    pub connects: Vec<(String, String, String)>,
}

/// A device set of a library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSetData {
    /// The name in its library.
    pub name: String,
    /// The reference prefix.
    pub prefix: String,
    /// True when the device set has a description.
    pub has_description: bool,
    /// The gates: gate name and the symbol it uses.
    pub gates: Vec<(String, String)>,
    /// The devices.
    pub devices: Vec<DeviceData>,
}

/// Everything the sanitizer needs from one library.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LibraryData {
    /// The footprints.
    pub packages: Vec<Item>,
    /// The symbols.
    pub symbols: Vec<Item>,
    /// The device sets.
    pub devicesets: Vec<DeviceSetData>,
    /// Drawing items and items that were skipped for want of a usable number or name.
    pub skipped: usize,
    /// Names used twice for the same kind of item: the later one is left out.
    pub duplicate_names: usize,
}

/// A length in nanometres: `None` when the attribute is absent, `Err` when it cannot be read.
fn opt_len(e: &Element, attr: &str) -> Result<Option<i64>, ()> {
    match e.attr(attr) {
        None => Ok(None),
        Some(text) => Nm::parse_mm(text)
            .map(|(n, _)| Some(n.get()))
            .map_err(|_| ()),
    }
}

fn req_len(e: &Element, attr: &str) -> Result<i64, ()> {
    opt_len(e, attr)?.ok_or(())
}

fn len_or_zero(e: &Element, attr: &str) -> Result<i64, ()> {
    Ok(opt_len(e, attr)?.unwrap_or(0))
}

/// An angle in micro-degrees in `[0, 360)`: `45`, `-90.5`.
fn angle_value(text: &str) -> Result<i64, ()> {
    Udeg::parse_deg(text)
        .map(|(a, _)| a.normalised().get())
        .map_err(|_| ())
}

/// A rotation such as `R90`, `MR180` or `SR45`: the angle, in `[0, 360)`.
fn rotation(e: &Element) -> Result<i64, ()> {
    match e.attr("rot") {
        None => Ok(0),
        Some(text) => {
            let digits = text.trim_start_matches(['M', 'S', 'R']);
            if digits.is_empty() {
                Ok(0)
            } else {
                angle_value(digits)
            }
        }
    }
}

fn curve_value(e: &Element) -> Result<i64, ()> {
    match e.attr("curve") {
        None => Ok(0),
        Some(text) => Udeg::parse_deg(text).map(|(a, _)| a.get()).map_err(|_| ()),
    }
}

fn layer_of(e: &Element) -> u32 {
    e.attr("layer").and_then(|l| l.parse().ok()).unwrap_or(0)
}

fn text_attr(e: &Element, attr: &str) -> String {
    e.attr(attr).unwrap_or("").to_owned()
}

fn read_prim(e: &Element, symbol: bool) -> Result<Option<Prim>, ()> {
    let prim = match e.name.as_str() {
        "pad" if !symbol => Prim::Pad {
            name: text_attr(e, "name"),
            thru: true,
            x: req_len(e, "x")?,
            y: req_len(e, "y")?,
            w: len_or_zero(e, "diameter")?,
            h: len_or_zero(e, "diameter")?,
            drill: req_len(e, "drill")?,
            shape: e.attr("shape").unwrap_or("round").to_owned(),
            layer: 0,
            rot: rotation(e)?,
        },
        "smd" if !symbol => Prim::Pad {
            name: text_attr(e, "name"),
            thru: false,
            x: req_len(e, "x")?,
            y: req_len(e, "y")?,
            w: req_len(e, "dx")?,
            h: req_len(e, "dy")?,
            drill: 0,
            shape: format!("smd{}", e.attr("roundness").unwrap_or("0")),
            layer: layer_of(e),
            rot: rotation(e)?,
        },
        "wire" => Prim::Wire {
            layer: layer_of(e),
            x1: req_len(e, "x1")?,
            y1: req_len(e, "y1")?,
            x2: req_len(e, "x2")?,
            y2: req_len(e, "y2")?,
            width: len_or_zero(e, "width")?,
            curve: curve_value(e)?,
        },
        "rectangle" => Prim::Rect {
            layer: layer_of(e),
            x1: req_len(e, "x1")?,
            y1: req_len(e, "y1")?,
            x2: req_len(e, "x2")?,
            y2: req_len(e, "y2")?,
            rot: rotation(e)?,
        },
        "circle" => Prim::Circle {
            layer: layer_of(e),
            x: req_len(e, "x")?,
            y: req_len(e, "y")?,
            r: req_len(e, "radius")?,
            width: len_or_zero(e, "width")?,
        },
        "polygon" => {
            let mut verts = Vec::new();
            for v in e.children_named("vertex") {
                verts.push((req_len(v, "x")?, req_len(v, "y")?, curve_value(v)?));
            }
            Prim::Poly {
                layer: layer_of(e),
                width: len_or_zero(e, "width")?,
                verts,
            }
        }
        "hole" if !symbol => Prim::Hole {
            x: req_len(e, "x")?,
            y: req_len(e, "y")?,
            drill: req_len(e, "drill")?,
        },
        "text" => Prim::Text {
            layer: layer_of(e),
            x: req_len(e, "x")?,
            y: req_len(e, "y")?,
            size: len_or_zero(e, "size")?,
            rot: rotation(e)?,
            content: e.text.trim().to_owned(),
        },
        "pin" if symbol => Prim::Pin {
            name: text_attr(e, "name"),
            x: req_len(e, "x")?,
            y: req_len(e, "y")?,
            rot: rotation(e)?,
            length: e.attr("length").unwrap_or("long").to_owned(),
            direction: e.attr("direction").unwrap_or("io").to_owned(),
            function: e.attr("function").unwrap_or("none").to_owned(),
            visible: e.attr("visible").unwrap_or("both").to_owned(),
            swaplevel: e.attr("swaplevel").unwrap_or("0").to_owned(),
        },
        _ => return Ok(None),
    };
    Ok(Some(prim))
}

fn read_items(
    parent: Option<&Element>,
    tag: &str,
    symbol: bool,
    out: &mut LibraryData,
) -> Vec<Item> {
    let mut items = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let Some(parent) = parent else {
        return items;
    };
    for element in parent.children_named(tag) {
        let name = element.attr("name").unwrap_or("");
        if name.is_empty() {
            out.skipped += 1;
            continue;
        }
        if !seen.insert(name.to_owned()) {
            out.duplicate_names += 1;
            continue;
        }
        let mut prims = Vec::new();
        for child in &element.children {
            match read_prim(child, symbol) {
                Ok(Some(p)) => prims.push(p),
                Ok(None) => {}
                Err(()) => out.skipped += 1,
            }
        }
        items.push(Item {
            name: name.to_owned(),
            has_description: element
                .child("description")
                .is_some_and(|d| !d.text.trim().is_empty()),
            prims,
        });
    }
    items
}

fn read_device_sets(library: &Element, out: &mut LibraryData) -> Vec<DeviceSetData> {
    let mut sets = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let Some(parent) = library.child("devicesets") else {
        return sets;
    };
    for set in parent.children_named("deviceset") {
        let name = set.attr("name").unwrap_or("");
        if name.is_empty() {
            out.skipped += 1;
            continue;
        }
        if !seen.insert(name.to_owned()) {
            out.duplicate_names += 1;
            continue;
        }
        let gates: Vec<(String, String)> = set
            .child("gates")
            .map(|g| {
                g.children_named("gate")
                    .map(|gate| (text_attr(gate, "name"), text_attr(gate, "symbol")))
                    .collect()
            })
            .unwrap_or_default();
        let mut devices = Vec::new();
        if let Some(list) = set.child("devices") {
            for device in list.children_named("device") {
                let connects: Vec<(String, String, String)> = device
                    .child("connects")
                    .map(|c| {
                        c.children_named("connect")
                            .map(|k| {
                                (
                                    text_attr(k, "gate"),
                                    text_attr(k, "pin"),
                                    text_attr(k, "pad"),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                devices.push(DeviceData {
                    package: device.attr("package").map(str::to_owned),
                    connects,
                });
            }
        }
        sets.push(DeviceSetData {
            name: name.to_owned(),
            prefix: text_attr(set, "prefix"),
            has_description: set
                .child("description")
                .is_some_and(|d| !d.text.trim().is_empty()),
            gates,
            devices,
        });
    }
    sets
}

/// Reads the `library` element of a drawing.
pub fn read_library(library: &Element) -> LibraryData {
    let mut counts = LibraryData::default();
    let packages = read_items(library.child("packages"), "package", false, &mut counts);
    let symbols = read_items(library.child("symbols"), "symbol", true, &mut counts);
    let devicesets = read_device_sets(library, &mut counts);
    LibraryData {
        packages,
        symbols,
        devicesets,
        ..counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_eagle::xml::{parse, Limits};

    fn library_of(text: &str) -> LibraryData {
        let root = parse(text, &Limits::default()).unwrap();
        let drawing = root.child("drawing").unwrap();
        read_library(drawing.child("library").unwrap())
    }

    const LIB: &str = r#"<eagle version="9.6.2"><drawing><library>
<packages>
<package name="R0805"><description>resistor</description>
<smd name="1" x="-0.9" y="0" dx="1.2" dy="1.4" layer="1" rot="R90"/>
<smd name="2" x="0.9" y="0" dx="1.2" dy="1.4" layer="1"/>
<wire x1="-1.5" y1="0.8" x2="1.5" y2="0.8" width="0.1" layer="21" curve="-90"/>
<polygon width="0.1" layer="39"><vertex x="0" y="0"/><vertex x="1" y="0"/><vertex x="1" y="1"/></polygon>
<text x="0" y="1" size="1" layer="25" rot="R180">&gt;NAME</text>
<hole x="0" y="2" drill="0.8"/>
<pad name="3" x="5" y="5" drill="0.8" diameter="1.6" shape="square"/>
</package>
<package name="BAD"><smd name="1" x="abc" y="0" dx="1" dy="1" layer="1"/></package>
<package name="R0805"/>
<package name=""/>
</packages>
<symbols><symbol name="R"><pin name="1" x="0" y="0" length="short" rot="R180"/><rectangle x1="1" y1="-1" x2="3" y2="1" layer="94"/><circle x="0" y="0" radius="0.5" width="0.2" layer="94"/></symbol></symbols>
<devicesets><deviceset name="R" prefix="R"><gates><gate name="G$1" symbol="R" x="0" y="0"/></gates>
<devices><device name="0805" package="R0805"><connects><connect gate="G$1" pin="1" pad="1"/></connects></device></devices></deviceset></devicesets>
</library></drawing></eagle>"#;

    #[test]
    fn reads_footprints_symbols_and_device_sets() {
        let d = library_of(LIB);
        assert_eq!(d.packages.len(), 2);
        let r = &d.packages[0];
        assert_eq!(r.name, "R0805");
        assert!(r.has_description);
        assert_eq!(r.prims.len(), 7);
        assert_eq!(
            r.prims[0],
            Prim::Pad {
                name: "1".into(),
                thru: false,
                x: -900_000,
                y: 0,
                w: 1_200_000,
                h: 1_400_000,
                drill: 0,
                shape: "smd0".into(),
                layer: 1,
                rot: 90_000_000
            }
        );
        assert!(matches!(
            r.prims[2],
            Prim::Wire {
                curve: -90_000_000,
                ..
            }
        ));
        assert!(
            matches!(&r.prims[4], Prim::Text { rot: 180_000_000, content, .. } if content == ">NAME")
        );
        assert_eq!(d.symbols.len(), 1);
        assert_eq!(d.symbols[0].prims.len(), 3);
        assert_eq!(d.devicesets.len(), 1);
        assert_eq!(
            d.devicesets[0].gates,
            vec![("G$1".to_string(), "R".to_string())]
        );
        assert_eq!(d.devicesets[0].devices[0].package.as_deref(), Some("R0805"));
    }

    #[test]
    fn unusable_numbers_and_names_are_counted_not_fatal() {
        let d = library_of(LIB);
        // the smd with "abc", the footprint with no name
        assert_eq!(d.skipped, 2);
        // the second R0805
        assert_eq!(d.duplicate_names, 1);
        assert!(d.packages[1].prims.is_empty());
    }

    #[test]
    fn rotation_keywords_are_read() {
        let root = parse(r#"<a rot="MR270"/>"#, &Limits::default()).unwrap();
        assert_eq!(rotation(&root), Ok(270_000_000));
        let root = parse(r#"<a rot="R-90"/>"#, &Limits::default()).unwrap();
        assert_eq!(rotation(&root), Ok(270_000_000));
        let root = parse(r#"<a rot="Rx"/>"#, &Limits::default()).unwrap();
        assert_eq!(rotation(&root), Err(()));
    }
}
