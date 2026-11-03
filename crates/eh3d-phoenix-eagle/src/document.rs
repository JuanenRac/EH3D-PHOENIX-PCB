//! Reads an EAGLE XML file into a summary, with the diagnostics of what could not be confirmed.
//!
//! The summary is a count and a list of names, not the design model: it is what the model readers
//! are compared with, and it already tells libraries, boards and schematics apart. Nothing is
//! lost silently: an element the reader does not interpret is counted under its name in
//! [`DocumentSummary::uninterpreted`] and reported once with `PHOENIX-EAGLE-009`.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Diagnostic, Severity};

use crate::codes;
use crate::xml::{self, Element, Limits, XmlError};

/// What the file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentKind {
    /// A component library.
    Library,
    /// A board.
    Board,
    /// A schematic.
    Schematic,
}

impl DocumentKind {
    /// A stable lower-case name.
    pub fn name(self) -> &'static str {
        match self {
            DocumentKind::Library => "library",
            DocumentKind::Board => "board",
            DocumentKind::Schematic => "schematic",
        }
    }
}

/// One package of a library with the number of each kind of drawing item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageSummary {
    /// The package name.
    pub name: String,
    /// Through-hole pads.
    pub pads: usize,
    /// Surface-mount pads.
    pub smds: usize,
    /// Wires and arcs.
    pub wires: usize,
    /// Everything else that is drawn (circles, rectangles, polygons, texts, holes).
    pub other_items: usize,
}

/// One symbol of a library.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolSummary {
    /// The symbol name.
    pub name: String,
    /// Its pins.
    pub pins: usize,
}

/// One device set of a library.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceSetSummary {
    /// The device set name.
    pub name: String,
    /// The reference prefix, such as `R` or `IC`.
    pub prefix: String,
    /// The gates (each uses a symbol).
    pub gates: usize,
    /// The devices (each uses a package) and how many connections they hold in all.
    pub devices: usize,
    /// The pad connections of all devices.
    pub connects: usize,
}

/// The summary of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSummary {
    /// Library, board or schematic.
    pub kind: DocumentKind,
    /// The declared version, such as `9.6.2`.
    pub version: String,
    /// The layers declared.
    pub layers: usize,
    /// Packages (libraries only; empty otherwise).
    pub packages: Vec<PackageSummary>,
    /// Symbols (libraries only).
    pub symbols: Vec<SymbolSummary>,
    /// Device sets (libraries only).
    pub device_sets: Vec<DeviceSetSummary>,
    /// Placed parts: board elements or schematic parts.
    pub parts: usize,
    /// Signals of a board, or nets of a schematic.
    pub nets: usize,
    /// Wires and vias inside the signals of a board, or net segments of a schematic.
    pub routed_items: usize,
    /// Sheets of a schematic.
    pub sheets: usize,
    /// Libraries embedded in a board or schematic.
    pub embedded_libraries: usize,
    /// Elements that were counted but not interpreted, by name.
    pub uninterpreted: BTreeMap<String, usize>,
}

/// The result of reading: the summary when the file is an EAGLE document, and every diagnostic.
#[derive(Debug, Clone)]
pub struct Reading {
    /// The summary; `None` when the file could not be read as an EAGLE document at all.
    pub summary: Option<DocumentSummary>,
    /// What was found, in order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Reading {
    fn fail(code: &'static str, detail: &str) -> Reading {
        Reading {
            summary: None,
            diagnostics: vec![codes::diagnostic(code, Severity::Error, detail)],
        }
    }

    /// True when a summary exists and nothing worse than information was found.
    pub fn is_clean(&self) -> bool {
        self.summary.is_some()
            && self
                .diagnostics
                .iter()
                .all(|d| d.severity == Severity::Info)
    }
}

/// The supported major versions.
pub const SUPPORTED_MAJOR: std::ops::RangeInclusive<u32> = 6..=9;

/// Reads `bytes` as an EAGLE XML file.
pub fn read_bytes(bytes: &[u8], limits: &Limits) -> Reading {
    let mut diagnostics = Vec::new();
    let text: std::borrow::Cow<'_, str> = match std::str::from_utf8(bytes) {
        Ok(t) => t.into(),
        Err(_) => {
            diagnostics.push(codes::diagnostic(
                codes::ENCODING_FALLBACK,
                Severity::Warning,
                "",
            ));
            bytes.iter().map(|&b| b as char).collect::<String>().into()
        }
    };
    let root = match xml::parse(&text, limits) {
        Ok(r) => r,
        Err(XmlError::Limit(what)) => return Reading::fail(codes::LIMIT_EXCEEDED, what),
        Err(e @ XmlError::Malformed { .. }) => {
            return Reading::fail(codes::NOT_XML, &e.to_string())
        }
    };
    if root.name != "eagle" {
        return Reading::fail(codes::NOT_EAGLE, "the root element is not eagle");
    }
    let Some(drawing) = root.child("drawing") else {
        return Reading::fail(codes::NOT_EAGLE, "there is no drawing");
    };
    let version = root.attr("version").unwrap_or("").to_owned();
    let major = version
        .split('.')
        .next()
        .and_then(|m| m.parse::<u32>().ok());
    if !major.is_some_and(|m| SUPPORTED_MAJOR.contains(&m)) {
        diagnostics.push(codes::diagnostic(
            codes::VERSION_UNSUPPORTED,
            Severity::Warning,
            &format!("version {version:?}"),
        ));
    }
    let layers = drawing
        .child("layers")
        .map_or(0, |l| l.children_named("layer").count());
    let mut summary = DocumentSummary {
        kind: DocumentKind::Library,
        version,
        layers,
        packages: Vec::new(),
        symbols: Vec::new(),
        device_sets: Vec::new(),
        parts: 0,
        nets: 0,
        routed_items: 0,
        sheets: 0,
        embedded_libraries: 0,
        uninterpreted: BTreeMap::new(),
    };
    if let Some(board) = drawing.child("board") {
        summary.kind = DocumentKind::Board;
        read_board(board, &mut summary);
    } else if let Some(schematic) = drawing.child("schematic") {
        summary.kind = DocumentKind::Schematic;
        read_schematic(schematic, &mut summary);
    } else if let Some(library) = drawing.child("library") {
        read_library(library, &mut summary, &mut diagnostics);
    } else {
        return Reading::fail(
            codes::NOT_EAGLE,
            "the drawing has no library, board or schematic",
        );
    }
    for (name, n) in &summary.uninterpreted {
        diagnostics.push(codes::diagnostic(
            codes::ELEMENT_UNKNOWN,
            Severity::Info,
            &format!("{name} x{n}"),
        ));
    }
    Reading {
        summary: Some(summary),
        diagnostics,
    }
}

fn count_embedded(libraries: Option<&Element>) -> usize {
    libraries.map_or(0, |l| l.children_named("library").count())
}

fn note_unknown(parent: &Element, known: &[&str], summary: &mut DocumentSummary) {
    for c in &parent.children {
        if !known.contains(&c.name.as_str()) {
            *summary.uninterpreted.entry(c.name.clone()).or_insert(0) += 1;
        }
    }
}

fn read_board(board: &Element, summary: &mut DocumentSummary) {
    const KNOWN: &[&str] = &[
        "plain",
        "libraries",
        "attributes",
        "variantdefs",
        "classes",
        "designrules",
        "autorouter",
        "elements",
        "signals",
        "errors",
    ];
    note_unknown(board, KNOWN, summary);
    summary.embedded_libraries = count_embedded(board.child("libraries"));
    summary.parts = board
        .child("elements")
        .map_or(0, |e| e.children_named("element").count());
    if let Some(signals) = board.child("signals") {
        summary.nets = signals.children_named("signal").count();
        summary.routed_items = signals
            .children_named("signal")
            .flat_map(|s| s.children.iter())
            .filter(|c| c.name == "wire" || c.name == "via")
            .count();
    }
}

fn read_schematic(schematic: &Element, summary: &mut DocumentSummary) {
    const KNOWN: &[&str] = &[
        "description",
        "libraries",
        "attributes",
        "variantdefs",
        "classes",
        "parts",
        "sheets",
        "errors",
    ];
    note_unknown(schematic, KNOWN, summary);
    summary.embedded_libraries = count_embedded(schematic.child("libraries"));
    summary.parts = schematic
        .child("parts")
        .map_or(0, |p| p.children_named("part").count());
    if let Some(sheets) = schematic.child("sheets") {
        for sheet in sheets.children_named("sheet") {
            summary.sheets += 1;
            if let Some(nets) = sheet.child("nets") {
                summary.nets += nets.children_named("net").count();
                summary.routed_items += nets
                    .children_named("net")
                    .flat_map(|n| n.children_named("segment"))
                    .count();
            }
        }
    }
}

fn read_library(library: &Element, summary: &mut DocumentSummary, diags: &mut Vec<Diagnostic>) {
    const KNOWN: &[&str] = &[
        "description",
        "packages",
        "symbols",
        "devicesets",
        "packages3d",
    ];
    note_unknown(library, KNOWN, summary);
    if let Some(packages) = library.child("packages") {
        for package in packages.children_named("package") {
            let mut p = PackageSummary {
                name: package.attr("name").unwrap_or("").to_owned(),
                ..PackageSummary::default()
            };
            for item in &package.children {
                match item.name.as_str() {
                    "pad" => p.pads += 1,
                    "smd" => p.smds += 1,
                    "wire" => p.wires += 1,
                    "description" => {}
                    _ => p.other_items += 1,
                }
            }
            summary.packages.push(p);
        }
    }
    if let Some(symbols) = library.child("symbols") {
        for symbol in symbols.children_named("symbol") {
            summary.symbols.push(SymbolSummary {
                name: symbol.attr("name").unwrap_or("").to_owned(),
                pins: symbol.children_named("pin").count(),
            });
        }
    }
    let package_names: BTreeSet<&str> = summary.packages.iter().map(|p| p.name.as_str()).collect();
    let symbol_names: BTreeSet<&str> = summary.symbols.iter().map(|s| s.name.as_str()).collect();
    let mut sets = Vec::new();
    if let Some(device_sets) = library.child("devicesets") {
        for set in device_sets.children_named("deviceset") {
            let name = set.attr("name").unwrap_or("").to_owned();
            let mut d = DeviceSetSummary {
                name: name.clone(),
                prefix: set.attr("prefix").unwrap_or("").to_owned(),
                ..DeviceSetSummary::default()
            };
            if let Some(gates) = set.child("gates") {
                for gate in gates.children_named("gate") {
                    d.gates += 1;
                    let symbol = gate.attr("symbol").unwrap_or("");
                    if !symbol_names.contains(symbol) {
                        diags.push(codes::diagnostic(
                            codes::SYMBOL_MISSING,
                            Severity::Warning,
                            &format!("{name}: {symbol}"),
                        ));
                    }
                }
            }
            if let Some(devices) = set.child("devices") {
                for device in devices.children_named("device") {
                    d.devices += 1;
                    let package = device.attr("package");
                    let pad_names = package.and_then(|pk| pad_names_of(library, pk));
                    if let Some(pk) = package {
                        if !package_names.contains(pk) {
                            diags.push(codes::diagnostic(
                                codes::PACKAGE_MISSING,
                                Severity::Warning,
                                &format!("{name}: {pk}"),
                            ));
                        }
                    }
                    if let Some(connects) = device.child("connects") {
                        for connect in connects.children_named("connect") {
                            d.connects += 1;
                            let pad = connect.attr("pad").unwrap_or("");
                            if let Some(pads) = &pad_names {
                                if !pads.contains(pad) {
                                    diags.push(codes::diagnostic(
                                        codes::CONNECT_DANGLES,
                                        Severity::Warning,
                                        &format!("{name}: pad {pad}"),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            sets.push(d);
        }
    }
    summary.device_sets = sets;
}

/// The pad and smd names of the package called `package`, if the library has it.
fn pad_names_of<'a>(library: &'a Element, package: &str) -> Option<BTreeSet<&'a str>> {
    let found = library
        .child("packages")?
        .children_named("package")
        .find(|p| p.attr("name") == Some(package))?;
    Some(
        found
            .children
            .iter()
            .filter(|c| c.name == "pad" || c.name == "smd")
            .filter_map(|c| c.attr("name"))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIB: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="6.5.0"><drawing><settings/><layers><layer number="1" name="Top"/><layer number="16" name="Bottom"/></layers>
<library><packages>
<package name="R0805"><smd name="1" x="0" y="0" dx="1" dy="1" layer="1"/><smd name="2" x="2" y="0" dx="1" dy="1" layer="1"/><wire x1="0" y1="0" x2="1" y2="1" width="0.1" layer="21"/><text x="0" y="0" size="1" layer="25">&gt;NAME</text></package>
</packages>
<symbols><symbol name="R"><pin name="1" x="0" y="0"/><pin name="2" x="5" y="0"/></symbol></symbols>
<devicesets><deviceset name="R" prefix="R"><gates><gate name="G$1" symbol="R" x="0" y="0"/></gates>
<devices><device name="0805" package="R0805"><connects><connect gate="G$1" pin="1" pad="1"/><connect gate="G$1" pin="2" pad="2"/></connects></device></devices></deviceset></devicesets>
</library></drawing></eagle>"#;

    fn read(t: &str) -> Reading {
        read_bytes(t.as_bytes(), &Limits::default())
    }

    #[test]
    fn reads_a_library() {
        let r = read(LIB);
        assert!(r.is_clean(), "{:?}", r.diagnostics);
        let s = r.summary.unwrap();
        assert_eq!(s.kind, DocumentKind::Library);
        assert_eq!(s.version, "6.5.0");
        assert_eq!(s.layers, 2);
        assert_eq!(s.packages.len(), 1);
        assert_eq!(
            (
                s.packages[0].smds,
                s.packages[0].wires,
                s.packages[0].other_items
            ),
            (2, 1, 1)
        );
        assert_eq!(s.symbols[0].pins, 2);
        assert_eq!(
            (
                s.device_sets[0].gates,
                s.device_sets[0].devices,
                s.device_sets[0].connects
            ),
            (1, 1, 2)
        );
    }

    #[test]
    fn a_dangling_connection_a_missing_package_and_a_missing_symbol_are_reported() {
        let t = LIB
            .replace("pad=\"2\"/>", "pad=\"9\"/>")
            .replace("package=\"R0805\"", "package=\"NOPE\"")
            .replace("symbol=\"R\" x", "symbol=\"Q\" x");
        let r = read(&t);
        let codes_found: Vec<&str> = r
            .diagnostics
            .iter()
            .filter_map(|d| d.code.as_deref())
            .collect();
        assert!(codes_found.contains(&codes::SYMBOL_MISSING));
        assert!(codes_found.contains(&codes::PACKAGE_MISSING));
        assert!(!r.is_clean());
        assert!(r.summary.is_some());
    }

    #[test]
    fn a_connection_to_a_missing_pad_is_reported() {
        let t = LIB.replace("pad=\"2\"/>", "pad=\"9\"/>");
        let r = read(&t);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some(codes::CONNECT_DANGLES)));
    }

    #[test]
    fn reads_a_board_and_a_schematic() {
        let brd = r#"<eagle version="9.6.2"><drawing><layers/><board><plain/><libraries><library name="a"/></libraries>
<elements><element name="R1" library="a" package="p" value="1k" x="0" y="0"/></elements>
<signals><signal name="N$1"><contactref element="R1" pad="1"/><wire x1="0" y1="0" x2="1" y2="1" width="0.2" layer="1"/><via x="1" y="1" extent="1-16" drill="0.3"/></signal></signals><weird/></board></drawing></eagle>"#;
        let s = read(brd).summary.unwrap();
        assert_eq!(s.kind, DocumentKind::Board);
        assert_eq!(
            (s.parts, s.nets, s.routed_items, s.embedded_libraries),
            (1, 1, 2, 1)
        );
        assert_eq!(s.uninterpreted.get("weird"), Some(&1));
        let sch = r#"<eagle version="7.7.0"><drawing><schematic><libraries/><parts><part name="R1" library="a" deviceset="R" device="0805"/></parts>
<sheets><sheet><nets><net name="GND" class="0"><segment/><segment/></net></nets></sheet><sheet/></sheets></schematic></drawing></eagle>"#;
        let s = read(sch).summary.unwrap();
        assert_eq!(s.kind, DocumentKind::Schematic);
        assert_eq!((s.parts, s.sheets, s.nets, s.routed_items), (1, 2, 1, 2));
    }

    #[test]
    fn things_that_are_not_eagle_files_give_codes() {
        let code = |t: &str| read(t).diagnostics[0].code.clone().unwrap().to_string();
        assert_eq!(code("<a"), codes::NOT_XML);
        assert_eq!(code("<html/>"), codes::NOT_EAGLE);
        assert_eq!(code("<eagle version=\"9.0.0\"/>"), codes::NOT_EAGLE);
        assert_eq!(
            code("<eagle><drawing><x/></drawing></eagle>"),
            codes::NOT_EAGLE
        );
        assert_eq!(
            code("<eagle><drawing><library/></drawing></eagle>"),
            codes::VERSION_UNSUPPORTED
        );
        assert_eq!(
            code("<eagle version=\"5.0\"><drawing><library/></drawing></eagle>"),
            codes::VERSION_UNSUPPORTED
        );
    }

    #[test]
    fn latin_1_files_are_read_with_a_warning() {
        let mut b =
            b"<eagle version=\"6.0.0\"><drawing><library><packages><package name=\"caf".to_vec();
        b.push(0xE9);
        b.extend_from_slice(b"\"/></packages></library></drawing></eagle>");
        let r = read_bytes(&b, &Limits::default());
        assert!(r.summary.is_some());
        assert_eq!(
            r.diagnostics[0].code.as_deref(),
            Some(codes::ENCODING_FALLBACK)
        );
    }

    #[test]
    fn a_limit_gives_its_own_code() {
        let l = Limits {
            max_nodes: 2,
            ..Limits::default()
        };
        let r = read_bytes(LIB.as_bytes(), &l);
        assert!(r.summary.is_none());
        assert_eq!(
            r.diagnostics[0].code.as_deref(),
            Some(codes::LIMIT_EXCEEDED)
        );
    }
}
