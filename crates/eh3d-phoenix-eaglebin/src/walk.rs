// SPDX-License-Identifier: GPL-3.0-or-later
//! The tree of the records of a library.
//!
//! ```text
//! header 0x10          u16 at 2: records of the preamble (settings 0x11, grid 0x12, layers 0x13)
//! library 0x15         u32 at 4, 8, 12: records of the three sections, each with its header
//!   device sets 0x17   u32 at 4: records that follow in the section; u16 at 8: how many items
//!     0x37             u16 at 2: gates; u16 at 4: records of the devices
//!       0x36 + 0x3C    a device and its connection records (u16 at 2: how many)
//!       0x2D           the gates
//!     0x36             version 3: a device with its gates 0x2D and connections 0x3C
//!   symbols 0x18
//!     0x1D             u16 at 2: drawing records
//!   footprints 0x19
//!     0x1E             u16 at 2: drawing records
//! ```
//!
//! Every count is checked against the records that are left, so a damaged file is refused and
//! never read past its end.

use eh3d_phoenix_libedit::model::Item;

use crate::codes::{self, BinError};
use crate::items;
use crate::records::{i32_at, u16_at, u32_at, Container, Limits, Record};
use crate::report::Report;
use crate::text::{decode, technologies, Strings};

const SETTINGS: u8 = 0x11;
const GRID: u8 = 0x12;
const LAYER: u8 = 0x13;
const LIBRARY: u8 = 0x15;
const SETS: u8 = 0x17;
const SYMBOLS: u8 = 0x18;
const PACKAGES: u8 = 0x19;
const SYMBOL: u8 = 0x1D;
const PACKAGE: u8 = 0x1E;
const GATE: u8 = 0x2D;
const DEVICE: u8 = 0x36;
const SET: u8 = 0x37;
const CONNECTIONS: u8 = 0x3C;

/// A layer of the file.
#[derive(Debug, Clone, Default)]
pub struct RawLayer {
    /// The number.
    pub number: u8,
    /// The name.
    pub name: String,
    /// The colour.
    pub color: u8,
    /// The fill pattern.
    pub fill: u8,
    /// The flags: visible and active.
    pub flags: u8,
}

/// A gate, with the number of its symbol (from 1).
#[derive(Debug, Clone, Default)]
pub struct RawGate {
    /// The name.
    pub name: String,
    /// The place.
    pub x: i32,
    /// The place.
    pub y: i32,
    /// The number of the symbol, from 1.
    pub symbol: usize,
    /// When the gate is added.
    pub addlevel: u8,
    /// The swap level.
    pub swaplevel: u8,
}

/// A device, with the number of its footprint (from 1, 0 for none).
#[derive(Debug, Clone, Default)]
pub struct RawDevice {
    /// The name of the variant.
    pub name: String,
    /// The number of the footprint, from 1; 0 for none.
    pub package: usize,
    /// The technologies.
    pub technologies: Vec<String>,
    /// The connection records.
    pub connections: Vec<Record>,
}

/// A device set.
#[derive(Debug, Clone, Default)]
pub struct RawSet {
    /// The name.
    pub name: String,
    /// The prefix of the parts.
    pub prefix: String,
    /// The description.
    pub description: String,
    /// True when the value is typed by the user.
    pub uservalue: bool,
    /// True when the connections have the form of version 3.
    pub old_connections: bool,
    /// The gates.
    pub gates: Vec<RawGate>,
    /// The devices.
    pub devices: Vec<RawDevice>,
}

/// A footprint or a symbol.
#[derive(Debug, Clone, Default)]
pub struct RawDrawing {
    /// The name.
    pub name: String,
    /// The description (footprints only).
    pub description: String,
    /// The drawing items in file order.
    pub items: Vec<Item>,
}

/// What the records of a library hold, before references are resolved.
#[derive(Debug, Clone, Default)]
pub struct RawLibrary {
    /// The layers.
    pub layers: Vec<RawLayer>,
    /// The description of the library.
    pub description: String,
    /// The device sets.
    pub sets: Vec<RawSet>,
    /// The symbols.
    pub symbols: Vec<RawDrawing>,
    /// The footprints.
    pub packages: Vec<RawDrawing>,
}

fn broken(at: usize, what: &str) -> BinError {
    BinError::new(codes::STRUCTURE, format!("record {at}: {what}"))
}

struct Walk<'a> {
    records: &'a [Record],
    strings: Strings<'a>,
    old: bool,
    items: usize,
    limits: Limits,
    report: &'a mut Report,
}

impl Walk<'_> {
    fn record(&self, at: usize) -> Result<Record, BinError> {
        self.records
            .get(at)
            .copied()
            .ok_or_else(|| broken(at, "past the end of the records"))
    }

    /// Counts one object against the limit.
    fn one_more(&mut self) -> Result<(), BinError> {
        self.items += 1;
        if self.items > self.limits.max_items {
            return Err(BinError::new(codes::LIMIT, "too many items"));
        }
        Ok(())
    }

    fn string(&mut self, r: &Record, start: usize, end: usize, place: &str) -> String {
        let bytes = self.strings.field(r, start, end);
        decode(&bytes, place, self.report)
    }

    fn preamble(&mut self, count: usize, out: &mut RawLibrary) -> Result<(), BinError> {
        for at in 1..=count {
            let r = self.record(at)?;
            match r[0] {
                LAYER => {
                    let name = self.string(&r, 15, 24, "layer");
                    out.layers.push(RawLayer {
                        number: r[3],
                        name,
                        color: r[6],
                        fill: r[5],
                        flags: r[2],
                    });
                }
                SETTINGS | GRID => {
                    self.report
                        .note(codes::NOT_READ, "settings and grid of the editor", "");
                }
                other => self.report.note(
                    codes::SKIPPED,
                    &format!("record type 0x{other:02X} before the library"),
                    "",
                ),
            }
        }
        Ok(())
    }

    /// The records `first..end` of a section, its header at `first - 1`, and how many items it
    /// says it has.
    fn section(&mut self, at: usize, size: usize, kind: u8) -> Result<(Record, usize), BinError> {
        let header = self.record(at)?;
        if header[0] != kind {
            return Err(broken(at, "not the header of the expected section"));
        }
        if u32_at(&header, 4) as usize != size - 1 {
            return Err(broken(at, "the section has another size in its header"));
        }
        Ok((header, usize::from(u16_at(&header, 8))))
    }

    fn gate(&mut self, at: usize, set: &str) -> Result<RawGate, BinError> {
        let r = self.record(at)?;
        if r[0] != GATE {
            return Err(broken(at, "a gate was expected"));
        }
        self.one_more()?;
        let name = self.string(&r, 16, 24, set);
        Ok(RawGate {
            name,
            x: i32_at(&r, 4),
            y: i32_at(&r, 8),
            symbol: usize::from(u16_at(&r, 14)),
            addlevel: r[12],
            swaplevel: r[13],
        })
    }

    /// A device set of version 4 or 5; returns it and the record after it.
    fn set(&mut self, at: usize, end: usize) -> Result<(RawSet, usize), BinError> {
        let r = self.record(at)?;
        self.one_more()?;
        let gates = usize::from(u16_at(&r, 2));
        let device_records = usize::from(u16_at(&r, 4));
        let after = at + 1 + device_records + gates;
        if after > end {
            return Err(broken(at, "the device set is longer than its section"));
        }
        let name = {
            let bytes = self.strings.field(&r, 18, 24);
            decode(&bytes, "device set", self.report)
        };
        let description = self.string(&r, 13, 18, &name);
        let prefix = self.string(&r, 8, 13, &name);
        let mut set = RawSet {
            name,
            prefix,
            description,
            uservalue: r[6] & 1 != 0,
            ..RawSet::default()
        };
        let devices_end = at + 1 + device_records;
        let mut k = at + 1;
        while k < devices_end {
            let d = self.record(k)?;
            if d[0] != DEVICE {
                return Err(broken(k, "a device was expected"));
            }
            self.one_more()?;
            let count = usize::from(u16_at(&d, 2));
            if k + 1 + count > devices_end {
                return Err(broken(k, "the connections are longer than the devices"));
            }
            let mut device = {
                let name = self.string(&d, 19, 24, &set.name);
                let list = self.strings.field(&d, 6, 19);
                RawDevice {
                    name,
                    package: usize::from(u16_at(&d, 4)),
                    technologies: technologies(&list, &set.name, self.report),
                    connections: Vec::new(),
                }
            };
            for c in k + 1..k + 1 + count {
                let c_record = self.record(c)?;
                if c_record[0] != CONNECTIONS {
                    return Err(broken(c, "connections were expected"));
                }
                device.connections.push(c_record);
            }
            set.devices.push(device);
            k += 1 + count;
        }
        for g in devices_end..after {
            let gate = self.gate(g, &set.name)?;
            set.gates.push(gate);
        }
        Ok((set, after))
    }

    /// A device of version 3: one footprint, its gates and connections.
    fn old_set(&mut self, at: usize, end: usize) -> Result<(RawSet, usize), BinError> {
        let r = self.record(at)?;
        self.one_more()?;
        let children = usize::from(u16_at(&r, 2));
        let after = at + 1 + children;
        if after > end {
            return Err(broken(at, "the device is longer than its section"));
        }
        let name = {
            let bytes = self.strings.field(&r, 14, 24);
            decode(&bytes, "device", self.report)
        };
        let prefix = self.string(&r, 11, 14, &name);
        self.report.note(
            codes::NOT_READ,
            "user value flag of a device of version 3 (read as off)",
            &name,
        );
        let mut set = RawSet {
            name,
            prefix,
            old_connections: true,
            ..RawSet::default()
        };
        let mut device = RawDevice {
            package: usize::from(u16_at(&r, 8)),
            technologies: vec![String::new()],
            ..RawDevice::default()
        };
        for k in at + 1..after {
            let child = self.record(k)?;
            match child[0] {
                GATE => {
                    let gate = self.gate(k, &set.name)?;
                    set.gates.push(gate);
                }
                CONNECTIONS => device.connections.push(child),
                other => self.report.note(
                    codes::SKIPPED,
                    &format!("record type 0x{other:02X} in a device"),
                    &set.name,
                ),
            }
        }
        set.devices.push(device);
        Ok((set, after))
    }

    /// The drawing records `from..to` of a footprint or a symbol called `place`.
    fn drawing_items(
        &mut self,
        from: usize,
        to: usize,
        place: &str,
    ) -> Result<Vec<Item>, BinError> {
        let mut out = Vec::new();
        let mut at = from;
        while at < to {
            let r = self.record(at)?;
            at += 1;
            self.one_more()?;
            let report = &mut *self.report;
            match r[0] {
                items::WIRE | items::ARC => {
                    let w = items::wire(&r, place, report);
                    report.count(if w.curve == 0 { "wire" } else { "arc" });
                    out.push(Item::Wire(w));
                }
                items::CIRCLE => {
                    report.count("circle");
                    out.push(Item::Circle(items::circle(&r)));
                }
                items::RECTANGLE => {
                    report.count("rectangle");
                    out.push(Item::Rect(items::rectangle(&r, place, report)));
                }
                items::HOLE => {
                    report.count("hole");
                    out.push(Item::Hole(items::hole(&r)));
                }
                items::PAD => {
                    let name = self.string(&r, 19, 24, place);
                    self.report.count("pad");
                    out.push(Item::Pad(items::pad(&r, name, place, self.report)));
                }
                items::SMD => {
                    let name = self.string(&r, 19, 24, place);
                    self.report.count("smd");
                    out.push(Item::Smd(items::smd(&r, name, place, self.report)));
                }
                items::PIN => {
                    let name = self.string(&r, 14, 24, place);
                    self.report.count("pin");
                    out.push(Item::Pin(items::pin(&r, name, place, self.report)));
                }
                items::TEXT => {
                    let mut bytes = self.strings.field(&r, 18, 24);
                    // version 3 puts a longer content in the records that follow
                    if bytes.is_empty() {
                        while at < to {
                            let more = self.record(at)?;
                            if more[0] != items::TEXT_MORE {
                                break;
                            }
                            self.one_more()?;
                            bytes.extend(more[2..].iter().copied().take_while(|&b| b != 0));
                            at += 1;
                        }
                    }
                    let content = decode(&bytes, place, self.report);
                    self.report.count("text");
                    out.push(Item::Text(items::text(&r, content, place, self.report)));
                }
                items::POLYGON => {
                    let n = usize::from(u16_at(&r, 2));
                    if at + n > to {
                        return Err(broken(at - 1, "the polygon is longer than its drawing"));
                    }
                    let vertices = &self.records[at..at + n];
                    at += n;
                    self.items += n;
                    self.report.count("polygon");
                    out.push(items::polygon(&r, vertices, place, self.report));
                }
                items::FRAME => {
                    report.count("frame");
                    out.push(items::frame(&r, place, report));
                }
                other => report.note(
                    codes::SKIPPED,
                    &format!("record type 0x{other:02X} in a drawing"),
                    place,
                ),
            }
        }
        Ok(out)
    }

    /// The footprints or symbols of a section whose records are `from..end`.
    fn drawings(&mut self, from: usize, end: usize, kind: u8) -> Result<Vec<RawDrawing>, BinError> {
        let mut out = Vec::new();
        let mut at = from;
        while at < end {
            let r = self.record(at)?;
            if r[0] != kind {
                return Err(broken(at, "a footprint or a symbol was expected"));
            }
            self.one_more()?;
            let after = at + 1 + usize::from(u16_at(&r, 2));
            if after > end {
                return Err(broken(at, "the drawing is longer than its section"));
            }
            let mut drawing = RawDrawing::default();
            if kind == PACKAGE {
                drawing.name = self.string(&r, 18, 24, "footprint");
                drawing.description = self.string(&r, 13, 18, &drawing.name);
            } else {
                drawing.name = self.string(&r, 16, 24, "symbol");
            }
            drawing.items = self.drawing_items(at + 1, after, &drawing.name)?;
            out.push(drawing);
            at = after;
        }
        Ok(out)
    }

    fn count_check(&mut self, announced: usize, found: usize, what: &str) {
        if announced != found {
            self.report.note(
                codes::VALUE,
                &format!("number of {what} in the header of the section"),
                "",
            );
        }
    }
}

/// Walks the records of a library.
pub fn walk(
    file: &Container,
    limits: &Limits,
    report: &mut Report,
) -> Result<RawLibrary, BinError> {
    let header = file
        .records
        .first()
        .copied()
        .ok_or_else(|| broken(0, "no header"))?;
    let preamble = usize::from(u16_at(&header, 2));
    let library_at = 1 + preamble;
    let mut w = Walk {
        records: &file.records,
        strings: Strings::new(&file.strings),
        old: file.version.0 < 4,
        items: 0,
        limits: *limits,
        report,
    };
    let mut out = RawLibrary::default();
    w.preamble(preamble, &mut out)?;
    let library = w.record(library_at)?;
    if library[0] != LIBRARY {
        return Err(BinError::new(
            codes::NOT_LIBRARY,
            format!("the object after the layers is 0x{:02X}", library[0]),
        ));
    }
    let sizes = [
        u32_at(&library, 4) as usize,
        u32_at(&library, 8) as usize,
        u32_at(&library, 12) as usize,
    ];
    let total = sizes
        .iter()
        .try_fold(library_at + 1, |sum, &n| sum.checked_add(n));
    if total != Some(file.records.len()) {
        return Err(broken(
            library_at,
            "the sections do not add up to the records",
        ));
    }
    // the name of the library, which the XML does not keep
    w.strings.field(&library, 16, 24);
    let mut at = library_at + 1;
    for (size, kind) in sizes.into_iter().zip([SETS, SYMBOLS, PACKAGES]) {
        if size == 0 {
            continue;
        }
        let end = at + size;
        let (head, announced) = w.section(at, size, kind)?;
        w.strings.field(&head, 16, 24);
        if kind == PACKAGES {
            out.description = w.string(&head, 10, 16, "library");
        }
        let found = match kind {
            SETS => {
                let mut k = at + 1;
                while k < end {
                    let r = w.record(k)?;
                    let (set, next) = match r[0] {
                        SET => w.set(k, end)?,
                        DEVICE if w.old => w.old_set(k, end)?,
                        _ => return Err(broken(k, "a device set was expected")),
                    };
                    out.sets.push(set);
                    k = next;
                }
                out.sets.len()
            }
            SYMBOLS => {
                out.symbols = w.drawings(at + 1, end, SYMBOL)?;
                out.symbols.len()
            }
            _ => {
                out.packages = w.drawings(at + 1, end, PACKAGE)?;
                out.packages.len()
            }
        };
        w.count_check(announced, found, "objects");
        at = end;
    }
    let (missing, left) = (w.strings.missing, w.strings.left());
    w.report.note_many(
        codes::STRINGS,
        "fields without their long string (names may be wrong)",
        "",
        missing,
    );
    w.report.note_many(
        codes::STRINGS,
        "long strings that no field asked for (names may be wrong)",
        "",
        left,
    );
    Ok(out)
}
