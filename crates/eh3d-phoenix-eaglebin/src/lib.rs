// SPDX-License-Identifier: GPL-3.0-or-later
//! Reader of the binary libraries and boards of EAGLE versions 3, 4 and 5 (the versions before
//! the XML files). It only reads: a library becomes the library model of the library editor,
//! which can be saved as an XML library; a board becomes the text of an XML board file. Both
//! come with a report of everything that was not read as it is. Schematics are not read.
//!
//! The format was worked out from the bytes of library files and is described in
//! `docs/EAGLE_BINARY.md`; no program was disassembled and no code of another reader was used.
//!
//! * [`records`]: the records of 24 bytes and the long strings, with limits;
//! * [`walk`]: the tree of records: layers, device sets, symbols and footprints;
//! * [`items`]: the drawing records;
//! * [`resolve`]: the references between device sets, symbols and footprints;
//! * [`board`] and [`rules`]: boards, their design rules and net classes;
//! * [`report`] and [`codes`]: what was not read, with the codes `PHOENIX-EAGLEBIN-<NNN>`.
//!
//! A damaged or hostile file gives an error with its code, never a panic or a read past its end.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod board;
pub mod codes;
pub mod items;
pub mod records;
pub mod report;
pub mod resolve;
pub mod rules;
pub mod text;
pub mod walk;

use std::collections::BTreeSet;
use std::sync::Arc;

use eh3d_phoenix_eagle::Element;
use eh3d_phoenix_libedit::codes::{LibError, FILE_UNUSABLE};
use eh3d_phoenix_libedit::model::{Library, Package, Symbol};
use eh3d_phoenix_libedit::LibSettings;

pub use board::{read_binary_board, BoardConversion};
pub use codes::BinError;
pub use records::{looks_binary, Limits};
pub use report::Report;

/// A library read from a binary file, with the report of the read.
#[derive(Debug, Clone)]
pub struct Conversion {
    /// The library.
    pub library: Library,
    /// What was read and what was not.
    pub report: Report,
}

fn element(name: &str, attrs: Vec<(String, String)>, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs,
        text: String::new(),
        children,
    }
}

fn yes_no(value: bool) -> String {
    if value { "yes" } else { "no" }.to_owned()
}

pub(crate) fn layer_element(layer: &walk::RawLayer) -> Element {
    let attrs = [
        ("number", layer.number.to_string()),
        ("name", layer.name.clone()),
        ("color", layer.color.to_string()),
        ("fill", layer.fill.to_string()),
        ("visible", yes_no(layer.flags & 0x0C != 0)),
        ("active", yes_no(layer.flags & 0x02 != 0)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    element("layer", attrs, Vec::new())
}

/// The XML tree around the three sections: the layers and the description of the file.
fn skeleton(layers: &[walk::RawLayer], description: &str) -> Element {
    let mut library = Vec::new();
    if !description.is_empty() {
        let mut d = element("description", Vec::new(), Vec::new());
        d.text = description.to_owned();
        library.push(d);
    }
    element(
        "eagle",
        vec![("version".to_owned(), "9.6.2".to_owned())],
        vec![element(
            "drawing",
            Vec::new(),
            vec![
                element("settings", Vec::new(), Vec::new()),
                element(
                    "layers",
                    Vec::new(),
                    layers.iter().map(layer_element).collect(),
                ),
                element("library", Vec::new(), library),
            ],
        )],
    )
}

fn note_repeated<'a>(names: impl Iterator<Item = &'a str>, subject: &str, report: &mut Report) {
    let mut seen = BTreeSet::new();
    for name in names {
        if !seen.insert(name) {
            report.note(codes::REFERENCE, subject, name);
        }
    }
}

/// Reads the bytes of an EAGLE library of either form, XML or binary, within the limits of the
/// settings of the library editor. A binary file also gives the report of its conversion.
pub fn read_any_library(
    bytes: &[u8],
    settings: &LibSettings,
) -> Result<(Library, Option<Report>), LibError> {
    if looks_binary(bytes) {
        let limits = Limits {
            max_bytes: usize::try_from(settings.max_file_bytes).unwrap_or(usize::MAX),
            ..Limits::default()
        };
        let read = read_binary_library(bytes, &limits)
            .map_err(|e| LibError::new(FILE_UNUSABLE, e.to_string()))?;
        return Ok((read.library, Some(read.report)));
    }
    eh3d_phoenix_libedit::read::read_library(bytes, &settings.limits())
        .map(|library| (library, None))
}

/// Reads the bytes of a binary library of version 3, 4 or 5.
pub fn read_binary_library(bytes: &[u8], limits: &Limits) -> Result<Conversion, BinError> {
    let file = records::split(bytes, limits)?;
    let mut report = Report {
        version: format!("{}.{}", file.version.0, file.version.1),
        ..Report::default()
    };
    if file.strings_cut {
        report.note(codes::STRINGS, "the long strings are cut short", "");
    }
    if file.extra_bytes > 0 {
        report.note(codes::NOT_READ, "bytes after the end of the file", "");
    }
    let raw = walk::walk(&file, limits, &mut report)?;

    let packages: Vec<Arc<Package>> = raw
        .packages
        .into_iter()
        .map(|p| {
            report.count("footprint");
            let description = if p.description.is_empty() {
                Vec::new()
            } else {
                report.count("description");
                let mut d = element("description", Vec::new(), Vec::new());
                d.text = p.description;
                vec![d]
            };
            Arc::new(Package {
                name: p.name,
                description,
                items: p.items,
                extra: Vec::new(),
            })
        })
        .collect();
    let symbols: Vec<Arc<Symbol>> = raw
        .symbols
        .into_iter()
        .map(|s| {
            report.count("symbol");
            Arc::new(Symbol {
                name: s.name,
                description: Vec::new(),
                items: s.items,
                extra: Vec::new(),
            })
        })
        .collect();
    let mut budget = resolve::Budget(limits.max_work);
    let mut devicesets = Vec::new();
    for s in &raw.sets {
        let set = resolve::device_set(s, &symbols, &packages, &mut budget, &mut report)?;
        devicesets.push(Arc::new(set));
    }
    for _ in &raw.layers {
        report.count("layer");
    }
    if !raw.description.is_empty() {
        report.count("description");
    }
    note_repeated(
        packages.iter().map(|p| p.name.as_str()),
        "footprint name used twice",
        &mut report,
    );
    note_repeated(
        symbols.iter().map(|p| p.name.as_str()),
        "symbol name used twice",
        &mut report,
    );
    note_repeated(
        devicesets.iter().map(|p| p.name.as_str()),
        "device set name used twice",
        &mut report,
    );
    // polygons and frames stay as XML elements, as in a library read from XML
    let kept_raw = packages
        .iter()
        .map(|p| &p.items)
        .chain(symbols.iter().map(|s| &s.items))
        .flatten()
        .filter(|i| matches!(i, eh3d_phoenix_libedit::Item::Other(_)))
        .count();
    let library = Library {
        skeleton: skeleton(&raw.layers, &raw.description),
        packages,
        symbols,
        devicesets,
        kept_raw,
    };
    Ok(Conversion { library, report })
}
