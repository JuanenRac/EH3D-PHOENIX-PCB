// SPDX-License-Identifier: GPL-3.0-or-later
//! Boards of the binary form, read into the XML tree of an EAGLE board file.
//!
//! ```text
//! preamble            settings 0x11, grid 0x12, layers 0x13 (as in a library)
//! board 0x1B          u16 at 2: records of the plain drawing; u32 at 12: libraries;
//!                     u32 at 16: elements; u32 at 20: signals
//!   libraries         0x19 (u32 at 4: records that follow) and their footprints 0x1E
//!   plain             the drawing records of a library, and 0x42 for an attribute of the board
//!   elements          0x2E (u16 at 2: records that follow): 0x2F name and value, then 0x34,
//!                     0x35 and 0x41, the places of the name, the value and other attributes
//!   signals           0x1C (u16 at 2: records that follow): contacts 0x3E, vias 0x29,
//!                     polygons 0x21 and wires
//! after the strings   the design rules and the net classes
//! ```
//!
//! The records are on disk in that order. The result is the same tree a board of version 6 or
//! later has, so everything that reads an XML board reads it too.

use std::collections::BTreeSet;

use eh3d_phoenix_eagle::{write_document, Element};
use eh3d_phoenix_libedit::model::{Item, Text};
use eh3d_phoenix_libedit::write::{fmt_mm, item_element};

use crate::codes::{self, BinError};
use crate::items;
use crate::records::{self, i32_at, u16_at, u32_at, Container, Limits, Record};
use crate::report::Report;
use crate::rules;
use crate::text::decode;
use crate::walk::{broken, RawDrawing, RawLayer, Walk, PACKAGE};

/// The root of a board.
pub const BOARD: u8 = 0x1B;
/// The root of a schematic.
pub const SCHEMATIC: u8 = 0x14;
/// The root of a library.
pub const LIBRARY_ROOT: u8 = 0x15;
const LIBRARY_HEADER: u8 = 0x19;
const SIGNAL: u8 = 0x1C;
const VIA: u8 = 0x29;
const ELEMENT: u8 = 0x2E;
const ELEMENT_NAMES: u8 = 0x2F;
const SMASHED_NAME: u8 = 0x34;
const SMASHED_VALUE: u8 = 0x35;
const CONTACT: u8 = 0x3E;
const ATTRIBUTE: u8 = 0x41;
const BOARD_ATTRIBUTE: u8 = 0x42;
/// The layer of the connections that are not routed.
const AIRWIRES: u8 = 19;
/// The spacing of the hatch of a polygon that the XML leaves out.
const DEFAULT_SPACING: i64 = 1_270_000;

/// A board read from the binary form: the text of an XML board file and what was not read.
#[derive(Debug, Clone)]
pub struct BoardConversion {
    /// The board as the text of an EAGLE XML board file.
    pub xml: String,
    /// What was read and what was not.
    pub report: Report,
}

struct Library {
    name: String,
    description: String,
    packages: Vec<RawDrawing>,
}

fn element(name: &str, attrs: Vec<(&str, String)>, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs: attrs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        text: String::new(),
        children,
    }
}

fn mm(units: i32) -> String {
    fmt_mm(i64::from(units) * 100)
}

/// A size that the file holds as its half, in millimetres.
fn twice(half: u16) -> String {
    fmt_mm(i64::from(half) * 200)
}

/// The layers a via or an unrouted connection goes through, as `1-16`.
fn extent(byte: u8) -> String {
    format!("{}-{}", (byte & 0x0F) + 1, (byte >> 4) + 1)
}

/// The first byte of the record that follows the preamble, which says what the file holds.
pub fn root_of(file: &Container) -> Option<u8> {
    let preamble = usize::from(u16_at(file.records.first()?, 2));
    file.records.get(1 + preamble).map(|r| r[0])
}

/// Reads the bytes of a binary board of version 3, 4 or 5.
pub fn read_binary_board(bytes: &[u8], limits: &Limits) -> Result<BoardConversion, BinError> {
    let file = records::split(bytes, limits)?;
    match root_of(&file) {
        Some(BOARD) => {}
        Some(SCHEMATIC) => {
            return Err(BinError::new(
                codes::NOT_BOARD,
                "it is a schematic, and binary schematics are not read",
            ))
        }
        Some(LIBRARY_ROOT) => return Err(BinError::new(codes::NOT_BOARD, "it is a library")),
        other => {
            return Err(BinError::new(
                codes::NOT_BOARD,
                format!(
                    "the object after the layers is 0x{:02X}",
                    other.unwrap_or(0)
                ),
            ))
        }
    }
    let mut report = Report {
        version: format!("{}.{}", file.version.0, file.version.1),
        ..Report::default()
    };
    if file.strings_cut {
        report.note(codes::STRINGS, "the long strings are cut short", "");
    }
    let tree = convert(&file, limits, &mut report)?;
    Ok(BoardConversion {
        xml: write_document(&tree),
        report,
    })
}

struct Sections {
    libraries: (usize, usize),
    plain: (usize, usize),
    elements: (usize, usize),
    signals: (usize, usize),
}

/// The four parts of the board whose record is at `at`.
fn sections(file: &Container, at: usize) -> Result<Sections, BinError> {
    let root = file.records[at];
    let libraries = u32_at(&root, 12) as usize;
    let elements = u32_at(&root, 16) as usize;
    let signals = u32_at(&root, 20) as usize;
    let after = file.records.len() - at - 1;
    // the count of the plain drawing has 16 bits: what is left over tells the rest
    let plain = libraries
        .checked_add(elements)
        .and_then(|n| n.checked_add(signals))
        .and_then(|n| after.checked_sub(n))
        .filter(|n| n & 0xFFFF == usize::from(u16_at(&root, 2)))
        .ok_or_else(|| broken(at, "the parts of the board do not add up to the records"))?;
    let a = at + 1;
    let b = a + libraries;
    let c = b + plain;
    let d = c + elements;
    Ok(Sections {
        libraries: (a, b),
        plain: (b, c),
        elements: (c, d),
        signals: (d, d + signals),
    })
}

fn convert(file: &Container, limits: &Limits, report: &mut Report) -> Result<Element, BinError> {
    let preamble = usize::from(u16_at(&file.records[0], 2));
    let parts = sections(file, 1 + preamble)?;
    let mut w = Walk::new(file, limits, report);
    let mut layers: Vec<RawLayer> = Vec::new();
    w.preamble(preamble, &mut layers)?;
    for _ in &layers {
        w.report.count("layer");
    }
    let libraries = read_libraries(&mut w, parts.libraries)?;
    let (plain, attributes) = read_plain(&mut w, parts.plain)?;
    let (elements, placed) = read_elements(&mut w, parts.elements, &libraries)?;
    let signals = read_signals(&mut w, parts.signals, &placed)?;
    w.strings_check();
    let design = rules::read(&file.tail, w.report);
    let library_elements = libraries
        .iter()
        .filter(|l| !l.packages.is_empty())
        .map(library_element)
        .collect();
    let mut board = vec![
        element("plain", Vec::new(), plain),
        element("libraries", Vec::new(), library_elements),
        element("attributes", Vec::new(), attributes),
        element("classes", Vec::new(), design.classes),
    ];
    board.extend(design.rules);
    board.push(element("elements", Vec::new(), elements));
    board.push(element("signals", Vec::new(), signals));
    let layer_elements = layers.iter().map(crate::layer_element).collect();
    Ok(element(
        "eagle",
        vec![("version", "9.6.2".to_owned())],
        vec![element(
            "drawing",
            Vec::new(),
            vec![
                element("settings", Vec::new(), Vec::new()),
                element("layers", Vec::new(), layer_elements),
                element("board", Vec::new(), board),
            ],
        )],
    ))
}

/// A name that is not taken yet in `taken`, adding `@n` when it is.
fn unique(name: String, taken: &mut BTreeSet<String>, what: &str, report: &mut Report) -> String {
    if taken.insert(name.clone()) {
        return name;
    }
    report.note(codes::REFERENCE, what, &name);
    let mut n = 2usize;
    loop {
        let other = format!("{name}@{n}");
        if taken.insert(other.clone()) {
            return other;
        }
        n += 1;
    }
}

fn read_libraries(w: &mut Walk, (from, to): (usize, usize)) -> Result<Vec<Library>, BinError> {
    let mut out = Vec::new();
    let mut names = BTreeSet::new();
    let mut at = from;
    while at < to {
        let r = w.record(at)?;
        if r[0] != LIBRARY_HEADER {
            return Err(broken(at, "a library of the board was expected"));
        }
        w.one_more()?;
        let end = (at + 1)
            .checked_add(u32_at(&r, 4) as usize)
            .filter(|&end| end <= to)
            .ok_or_else(|| broken(at, "the library is longer than its part"))?;
        let name = w.string(&r, 16, 24, "library");
        let description = w.string(&r, 10, 16, &name);
        let mut packages = w.drawings(at + 1, end, PACKAGE)?;
        let name = unique(name, &mut names, "library name used twice", w.report);
        let mut taken = BTreeSet::new();
        for p in &mut packages {
            let own = std::mem::take(&mut p.name);
            p.name = unique(own, &mut taken, "footprint name used twice", w.report);
            w.report.count("footprint");
        }
        if !packages.is_empty() {
            w.report.count("library");
        }
        out.push(Library {
            name,
            description,
            packages,
        });
        at = end;
    }
    Ok(out)
}

fn described(text: &str) -> Vec<Element> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut d = element("description", Vec::new(), Vec::new());
    d.text = text.to_owned();
    vec![d]
}

fn library_element(library: &Library) -> Element {
    let packages = library
        .packages
        .iter()
        .map(|p| {
            let mut children = described(&p.description);
            children.extend(p.items.iter().map(item_element));
            element("package", vec![("name", p.name.clone())], children)
        })
        .collect();
    let mut children = described(&library.description);
    children.push(element("packages", Vec::new(), packages));
    element("library", vec![("name", library.name.clone())], children)
}

/// A polygon of the board. Inside a signal it also has a rank, thermals and orphans.
fn polygon(w: &mut Walk, at: usize, to: usize, signal: bool) -> Result<(Element, usize), BinError> {
    let head = w.record(at)?;
    let count = usize::from(u16_at(&head, 2));
    let end = at + 1 + count;
    if end > to {
        return Err(broken(at, "the polygon is longer than its part"));
    }
    w.one_more()?;
    let flags = head[19];
    let mut attrs = vec![
        ("width", twice(u16_at(&head, 12))),
        ("layer", head[18].to_string()),
    ];
    let spacing = i64::from(u16_at(&head, 14)) * 200;
    if spacing != DEFAULT_SPACING && spacing != 0 {
        attrs.push(("spacing", fmt_mm(spacing)));
    }
    if u16_at(&head, 16) != 0 {
        attrs.push(("isolate", twice(u16_at(&head, 16))));
    }
    if flags & 0x01 != 0 {
        attrs.push(("pour", "hatch".to_owned()));
    }
    if flags & 0x30 != 0 {
        w.report
            .note(codes::VALUE, "unknown flags of a polygon", "");
    }
    if signal {
        let rank = (flags >> 1) & 0x07;
        let mut inferred = false;
        if (2..=6).contains(&rank) {
            attrs.push(("rank", rank.to_string()));
        } else if rank != 1 {
            w.report.note(codes::VALUE, "unknown rank of a polygon", "");
        }
        if flags & 0x80 == 0 {
            attrs.push(("thermals", "no".to_owned()));
            inferred = true;
        }
        if flags & 0x40 != 0 {
            attrs.push(("orphans", "yes".to_owned()));
            inferred = true;
        }
        if inferred {
            w.report.note(
                codes::VALUE,
                "thermals or orphans of a polygon (inferred)",
                "",
            );
        }
    }
    let mut vertices = Vec::new();
    for k in at + 1..end {
        let v = w.record(k)?;
        w.one_more()?;
        if v[0] != items::WIRE && v[0] != items::ARC {
            w.report.note(codes::SKIPPED, "record inside a polygon", "");
            continue;
        }
        let side = items::wire(&v, w.old, "polygon", w.report);
        let mut a = vec![("x", fmt_mm(side.x1)), ("y", fmt_mm(side.y1))];
        if side.curve != 0 {
            a.push(("curve", fmt_mm(side.curve)));
        }
        vertices.push(element("vertex", a, Vec::new()));
    }
    w.report.count("polygon");
    Ok((element("polygon", attrs, vertices), end))
}

/// An attribute of the form `name!value`.
fn named_value(text: &str) -> (&str, &str) {
    text.split_once('!').unwrap_or((text, ""))
}

fn read_plain(
    w: &mut Walk,
    (from, to): (usize, usize),
) -> Result<(Vec<Element>, Vec<Element>), BinError> {
    let mut plain = Vec::new();
    let mut attributes = Vec::new();
    let mut at = from;
    while at < to {
        let r = w.record(at)?;
        match r[0] {
            items::POLYGON => {
                let (p, end) = polygon(w, at, to, false)?;
                plain.push(p);
                at = end;
            }
            BOARD_ATTRIBUTE => {
                w.one_more()?;
                let text = w.string(&r, 7, 24, "attribute of the board");
                let (name, value) = named_value(&text);
                plain_attribute(&mut attributes, name, value);
                w.report.count("attribute");
                at += 1;
            }
            _ => {
                // version 3 puts the rest of a long text in the records that follow it
                let mut end = at + 1;
                while r[0] == items::TEXT && end < to && w.record(end)?[0] == items::TEXT_MORE {
                    end += 1;
                }
                let read = w.drawing_items(at, end, "plain")?;
                plain.extend(read.iter().map(item_element));
                at = end;
            }
        }
    }
    Ok((plain, attributes))
}

fn plain_attribute(out: &mut Vec<Element>, name: &str, value: &str) {
    out.push(element(
        "attribute",
        vec![("name", name.to_owned()), ("value", value.to_owned())],
        Vec::new(),
    ));
}

/// The place of the name, the value or another attribute of an element.
fn placed_attribute(w: &mut Walk, r: &Record, place: &str) -> Element {
    let (name, value) = match r[0] {
        SMASHED_NAME => ("NAME".to_owned(), None),
        SMASHED_VALUE => ("VALUE".to_owned(), None),
        _ => {
            let bytes = w.strings.field(r, 18, 24);
            let text = decode(&bytes, place, w.report);
            let (name, value) = named_value(&text);
            // no file with a visible attribute was compared, so it is kept out of sight
            w.report.note(
                codes::NOT_READ,
                "visibility of an attribute (written as not shown)",
                place,
            );
            (name.to_owned(), Some(value.to_owned()))
        }
    };
    let shown: Text = items::text(r, String::new(), w.old, place, w.report);
    let mut out = item_element(&Item::Text(shown));
    out.name = "attribute".to_owned();
    out.text = String::new();
    let mut attrs = vec![("name".to_owned(), name)];
    if let Some(value) = value {
        attrs.push(("value".to_owned(), value));
        attrs.push(("display".to_owned(), "off".to_owned()));
    }
    attrs.append(&mut out.attrs);
    out.attrs = attrs;
    out
}

/// An element as the signals need it: its name and the names of the pads of its footprint.
struct Placed<'a> {
    name: String,
    pads: Vec<&'a str>,
}

type Elements<'a> = (Vec<Element>, Vec<Placed<'a>>);

fn read_elements<'a>(
    w: &mut Walk,
    (from, to): (usize, usize),
    libraries: &'a [Library],
) -> Result<Elements<'a>, BinError> {
    let mut out = Vec::new();
    let mut placed = Vec::new();
    let mut at = from;
    while at < to {
        let r = w.record(at)?;
        if r[0] != ELEMENT {
            return Err(broken(at, "an element was expected"));
        }
        w.one_more()?;
        let count = usize::from(u16_at(&r, 2));
        let end = at + 1 + count;
        if count == 0 || end > to {
            return Err(broken(at, "the element does not fit its part"));
        }
        let names = w.record(at + 1)?;
        if names[0] != ELEMENT_NAMES {
            return Err(broken(at + 1, "the name of an element was expected"));
        }
        // the long strings follow the fields from the last to the first
        let value = w.string(&names, 10, 24, "element");
        let name = w.string(&names, 2, 10, "element");
        let library = libraries.get(usize::from(u16_at(&r, 12)).wrapping_sub(1));
        let package =
            library.and_then(|l| l.packages.get(usize::from(u16_at(&r, 14)).wrapping_sub(1)));
        if package.is_none() {
            w.report
                .note(codes::REFERENCE, "element without its footprint", &name);
        }
        let (turn, known) = items::rotation(u16_at(&r, 16));
        if !known {
            w.report
                .note(codes::VALUE, "unknown bits of a rotation", &name);
        }
        let flags = r[18];
        if flags & !0x03 != 0 {
            w.report
                .note(codes::VALUE, "unknown flags of an element", &name);
        }
        let mut attributes = Vec::new();
        for k in at + 2..end {
            let a = w.record(k)?;
            w.one_more()?;
            match a[0] {
                SMASHED_NAME | SMASHED_VALUE | ATTRIBUTE => {
                    attributes.push(placed_attribute(w, &a, &name));
                }
                other => w.report.note(
                    codes::SKIPPED,
                    &format!("record type 0x{other:02X} in an element"),
                    &name,
                ),
            }
        }
        let mut attrs = vec![
            ("name", name.clone()),
            (
                "library",
                library.map(|l| l.name.clone()).unwrap_or_default(),
            ),
            (
                "package",
                package.map(|p| p.name.clone()).unwrap_or_default(),
            ),
            ("value", value),
            ("x", mm(i32_at(&r, 4))),
            ("y", mm(i32_at(&r, 8))),
        ];
        if flags & 0x01 != 0 || !attributes.is_empty() {
            attrs.push(("smashed", "yes".to_owned()));
        }
        if !turn.is_empty() {
            attrs.push(("rot", turn));
        }
        out.push(element("element", attrs, attributes));
        let pads = package
            .map(|p| {
                p.items
                    .iter()
                    .filter_map(|i| match i {
                        Item::Pad(pad) => Some(pad.name.as_str()),
                        Item::Smd(smd) => Some(smd.name.as_str()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        placed.push(Placed { name, pads });
        w.report.count("element");
        at = end;
    }
    Ok((out, placed))
}

fn via(w: &mut Walk, r: &Record) -> Element {
    let mut attrs = vec![
        ("x", mm(i32_at(r, 4))),
        ("y", mm(i32_at(r, 8))),
        ("extent", extent(r[16])),
        ("drill", twice(u16_at(r, 12))),
    ];
    if u16_at(r, 14) != 0 {
        attrs.push(("diameter", twice(u16_at(r, 14))));
    }
    match r[2] {
        0 => attrs.push(("shape", "square".to_owned())),
        1 => {}
        2 => attrs.push(("shape", "octagon".to_owned())),
        _ => w
            .report
            .note(codes::VALUE, "unknown shape of a via (read as round)", ""),
    }
    w.report.count("via");
    element("via", attrs, Vec::new())
}

/// A connection that is not routed: its ends, and the layers it asks for.
fn airwire(w: &mut Walk, r: &Record) -> Element {
    let s24 = |at: usize| {
        let v = i32::from_le_bytes([r[at], r[at + 1], r[at + 2], 0]);
        mm((v << 8) >> 8)
    };
    let mut attrs = vec![
        ("x1", s24(4)),
        ("y1", s24(8)),
        ("x2", s24(12)),
        ("y2", s24(16)),
        ("width", "0".to_owned()),
        ("layer", AIRWIRES.to_string()),
    ];
    if r[22] != 0 {
        attrs.push(("extent", extent(r[22])));
    }
    w.report.count("airwire");
    element("wire", attrs, Vec::new())
}

fn read_signals(
    w: &mut Walk,
    (from, to): (usize, usize),
    placed: &[Placed],
) -> Result<Vec<Element>, BinError> {
    let mut out = Vec::new();
    let mut at = from;
    while at < to {
        let r = w.record(at)?;
        if r[0] != SIGNAL {
            return Err(broken(at, "a signal was expected"));
        }
        w.one_more()?;
        let end = at + 1 + usize::from(u16_at(&r, 2));
        if end > to {
            return Err(broken(at, "the signal is longer than its part"));
        }
        let name = w.string(&r, 16, 24, "signal");
        let mut children = Vec::new();
        let mut k = at + 1;
        while k < end {
            let q = w.record(k)?;
            if q[0] == items::POLYGON {
                let (p, next) = polygon(w, k, end, true)?;
                children.push(p);
                k = next;
                continue;
            }
            k += 1;
            w.one_more()?;
            match q[0] {
                CONTACT => {
                    let part = placed.get(usize::from(u16_at(&q, 4)).wrapping_sub(1));
                    let pad = part.and_then(|p| {
                        p.pads
                            .get(usize::from(u16_at(&q, 6)).wrapping_sub(1))
                            .map(|pad| (p.name.clone(), (*pad).to_owned()))
                    });
                    match pad {
                        Some((element_name, pad)) => {
                            w.report.count("contactref");
                            children.push(element(
                                "contactref",
                                vec![("element", element_name), ("pad", pad)],
                                Vec::new(),
                            ));
                        }
                        None => w.report.note(
                            codes::REFERENCE,
                            "contact of a signal without its pad",
                            &name,
                        ),
                    }
                }
                VIA => children.push(via(w, &q)),
                items::WIRE | items::ARC if q[3] == AIRWIRES => children.push(airwire(w, &q)),
                items::WIRE | items::ARC => {
                    let wire = items::wire(&q, w.old, &name, w.report);
                    w.report.count(if wire.curve == 0 { "wire" } else { "arc" });
                    children.push(item_element(&Item::Wire(wire)));
                }
                other => w.report.note(
                    codes::SKIPPED,
                    &format!("record type 0x{other:02X} in a signal"),
                    &name,
                ),
            }
        }
        let mut attrs = vec![("name", name)];
        if r[13] != 0 {
            attrs.push(("class", r[13].to_string()));
        }
        out.push(element("signal", attrs, children));
        w.report.count("signal");
        at = end;
    }
    Ok(out)
}
