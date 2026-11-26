// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing records of footprints and symbols. Lengths of the file are in tenths of a
//! micrometre; sizes of pads, texts, lines and drills are stored as halves. The model holds
//! nanometres.
//!
//! | first byte | object |
//! |---|---|
//! | `0x21` | polygon, followed by its vertices (line records) |
//! | `0x22`, `0x24` | line or arc |
//! | `0x25` | circle |
//! | `0x26` | rectangle |
//! | `0x28` | hole |
//! | `0x2A` | through-hole pad |
//! | `0x2B` | surface-mount pad |
//! | `0x2C` | pin |
//! | `0x31` | text |
//! | `0x32` | the rest of the content of a text (version 3) |
//! | `0x43` | drawing frame |

use eh3d_phoenix_eagle::Element;
use eh3d_phoenix_libedit::model::{Circle, Hole, Item, Pad, PadShape, Pin, Rect, Smd, Text, Wire};
use eh3d_phoenix_libedit::write::fmt_mm;

use crate::codes;
use crate::records::{i32_at, u16_at, u32_at, Record};
use crate::report::Report;

/// Polygon.
pub const POLYGON: u8 = 0x21;
/// Line or arc.
pub const WIRE: u8 = 0x22;
/// Arc of the second kind.
pub const ARC: u8 = 0x24;
/// Circle.
pub const CIRCLE: u8 = 0x25;
/// Rectangle.
pub const RECTANGLE: u8 = 0x26;
/// Hole.
pub const HOLE: u8 = 0x28;
/// Through-hole pad.
pub const PAD: u8 = 0x2A;
/// Surface-mount pad.
pub const SMD: u8 = 0x2B;
/// Pin.
pub const PIN: u8 = 0x2C;
/// Text.
pub const TEXT: u8 = 0x31;
/// Continuation of the content of a text.
pub const TEXT_MORE: u8 = 0x32;
/// Drawing frame.
pub const FRAME: u8 = 0x43;

/// Nanometres in one unit of the file.
const UNIT: i64 = 100;
/// The spacing of the hatch of a polygon that the file of version 6 leaves out.
const DEFAULT_SPACING: i64 = 1_270_000;

fn length(value: i32) -> i64 {
    i64::from(value) * UNIT
}

/// A size that the file holds as its half.
fn doubled(half: u32) -> i64 {
    i64::from(half) * 2 * UNIT
}

/// A signed number of 24 bits.
fn s24(low: u8, middle: u8, high: u8) -> i32 {
    let v = i32::from(low) | i32::from(middle) << 8 | i32::from(high) << 16;
    if v & 0x0080_0000 != 0 {
        v - 0x0100_0000
    } else {
        v
    }
}

/// The rotation as the XML writes it, from the 16 bits of the file: 4096 steps in a turn, bit
/// 12 mirrors and bit 14 spins. The second value is false when other bits are set.
pub fn rotation(bits: u16) -> (String, bool) {
    let known = bits & !0x5FFF == 0;
    let tenths = (u32::from(bits & 0x0FFF) * 3600 + 2048) / 4096;
    let mirrored = bits & 0x1000 != 0;
    let spin = bits & 0x4000 != 0;
    if tenths == 0 && !mirrored && !spin {
        return (String::new(), known);
    }
    let mut out = String::new();
    if spin {
        out.push('S');
    }
    if mirrored {
        out.push('M');
    }
    out.push('R');
    out.push_str(&(tenths / 10).to_string());
    if tenths % 10 != 0 {
        out.push('.');
        out.push_str(&(tenths % 10).to_string());
    }
    (out, known)
}

fn rot(bits: u16, place: &str, report: &mut Report) -> String {
    let (text, known) = rotation(bits);
    if !known {
        report.note(codes::VALUE, "unknown bits of a rotation", place);
    }
    text
}

fn pair(name: &str, value: impl Into<String>) -> (String, String) {
    (name.to_owned(), value.into())
}

/// A line or an arc, with the attributes the model keeps beside it.
pub fn wire(r: &Record, place: &str, report: &mut Report) -> Wire {
    let x1 = s24(r[4], r[5], r[6]);
    let y1 = s24(r[8], r[9], r[10]);
    let x2 = s24(r[12], r[13], r[14]);
    let y2 = s24(r[16], r[17], r[18]);
    let flags = r[22];
    let curve = arc_angle(r, (x1, y1, x2, y2), place, report);
    let mut extra = Vec::new();
    match flags & 0x03 {
        1 => extra.push(pair("style", "longdash")),
        2 => extra.push(pair("style", "shortdash")),
        3 => extra.push(pair("style", "dashdot")),
        _ => {}
    }
    if curve != 0 && flags & 0x10 != 0 {
        extra.push(pair("cap", "flat"));
    }
    Wire {
        layer: u32::from(r[3]),
        x1: length(x1),
        y1: length(y1),
        x2: length(x2),
        y2: length(y2),
        width: doubled(u32::from(u16_at(r, 20))),
        curve,
        extra,
    }
}

/// The angle of an arc in micro-degrees; zero for a straight line.
///
/// Byte 23 says the kind: quarter and half circles store only their ends; the general arc
/// stores one coordinate of its centre in the high bytes of the first three coordinates (the x
/// when the chord is steeper than it is wide, else the y), and the other coordinate and the angle
/// follow from the ends. Bit 5 of byte 22 says counterclockwise.
fn arc_angle(
    r: &Record,
    (x1, y1, x2, y2): (i32, i32, i32, i32),
    place: &str,
    report: &mut Report,
) -> i64 {
    let sign: i64 = if r[22] & 0x20 != 0 { 1 } else { -1 };
    let code = r[23];
    match (r[0], code) {
        (WIRE, 0) => return 0,
        (WIRE, 0x78..=0x7B) | (ARC, 1..=4) => return sign * 90_000_000,
        (WIRE, 0x7C..=0x7F) | (ARC, 5..=8) => return sign * 180_000_000,
        (WIRE, 0x81) | (ARC, 0) => {}
        _ => {
            report.note(
                codes::VALUE,
                "unknown kind of arc (read as straight)",
                place,
            );
            return 0;
        }
    }
    let stored = f64::from(s24(r[7], r[11], r[15]));
    let (fx1, fy1, fx2, fy2) = (f64::from(x1), f64::from(y1), f64::from(x2), f64::from(y2));
    let (dx, dy) = (fx2 - fx1, fy2 - fy1);
    let (mx, my) = ((fx1 + fx2) / 2.0, (fy1 + fy2) / 2.0);
    if x1 == x2 && y1 == y2 {
        report.note(codes::VALUE, "arc with no length (read as straight)", place);
        return 0;
    }
    let (cx, cy) = if dx.abs() < dy.abs() {
        (stored, my - (stored - mx) * dx / dy)
    } else {
        (mx - (stored - my) * dy / dx, stored)
    };
    let start = (fy1 - cy).atan2(fx1 - cx);
    let end = (fy2 - cy).atan2(fx2 - cx);
    let mut sweep = (end - start).to_degrees().rem_euclid(360.0);
    if sign < 0 {
        sweep -= 360.0;
    }
    let micro = (sweep * 1e6).round() as i64;
    if micro == 0 || micro.abs() >= 360_000_000 {
        report.note(codes::VALUE, "arc with no angle (read as straight)", place);
        return 0;
    }
    report.note(
        codes::NOT_READ,
        "arc angle computed from the stored centre",
        place,
    );
    micro
}

/// A circle.
pub fn circle(r: &Record) -> Circle {
    Circle {
        layer: u32::from(r[3]),
        x: length(i32_at(r, 4)),
        y: length(i32_at(r, 8)),
        radius: i64::from(u32_at(r, 12)) * UNIT,
        width: doubled(u32_at(r, 20)),
        extra: Vec::new(),
    }
}

/// A rectangle; its corners are put in order.
pub fn rectangle(r: &Record, place: &str, report: &mut Report) -> Rect {
    let (x1, y1) = (i32_at(r, 4), i32_at(r, 8));
    let (x2, y2) = (i32_at(r, 12), i32_at(r, 16));
    Rect {
        layer: u32::from(r[3]),
        x1: length(x1.min(x2)),
        y1: length(y1.min(y2)),
        x2: length(x1.max(x2)),
        y2: length(y1.max(y2)),
        rot: rot(u16_at(r, 20), place, report),
        extra: Vec::new(),
    }
}

/// A hole.
pub fn hole(r: &Record) -> Hole {
    Hole {
        x: length(i32_at(r, 4)),
        y: length(i32_at(r, 8)),
        drill: doubled(u32_at(r, 12)),
        extra: Vec::new(),
    }
}

/// The flags of a pad of either kind: bit 0 no opening of the mask, bit 1 no paste, bit 2 no
/// thermals, bit 3 first pad. `known` are the bits that mean something for this kind.
fn pad_flags(flags: u8, known: u8, place: &str, report: &mut Report) {
    if flags & !known != 0 {
        report.note(codes::VALUE, "unknown flags of a pad", place);
    }
    if flags & 0x04 != 0 {
        report.note(codes::VALUE, "thermals flag of a pad (inferred)", place);
    }
}

/// The angle and the flags of a pad of either kind; both are zero in version 3.
#[derive(Debug, Clone, Copy, Default)]
pub struct Turn {
    /// The angle, as [`rotation`] reads it.
    pub angle: u16,
    /// The flags.
    pub flags: u8,
}

/// A through-hole pad.
pub fn pad(r: &Record, turn: Turn, name: String, place: &str, report: &mut Report) -> Pad {
    let mut angle = turn.angle;
    let shape = match r[2] {
        0 => PadShape::Square,
        1 => PadShape::Round,
        2 => PadShape::Octagon,
        3 => PadShape::Long,
        // a long pad that stands upright
        4 => {
            angle = (angle & 0xF000) | (((angle & 0x0FFF) + 1024) % 4096);
            PadShape::Long
        }
        _ => {
            report.note(
                codes::VALUE,
                "unknown shape of a pad (read as round)",
                place,
            );
            PadShape::Round
        }
    };
    let flags = turn.flags;
    pad_flags(flags, 0x0D, place, report);
    Pad {
        name,
        x: length(i32_at(r, 4)),
        y: length(i32_at(r, 8)),
        drill: doubled(u32::from(u16_at(r, 12))),
        diameter: doubled(u32::from(u16_at(r, 14))),
        shape,
        rot: rot(angle, place, report),
        stop: flags & 0x01 == 0,
        thermals: flags & 0x04 == 0,
        first: flags & 0x08 != 0,
        extra: Vec::new(),
    }
}

/// A surface-mount pad.
pub fn smd(r: &Record, turn: Turn, name: String, place: &str, report: &mut Report) -> Smd {
    let roundness = if r[2] <= 100 {
        u32::from(r[2])
    } else {
        report.note(codes::VALUE, "roundness over 100 (read as zero)", place);
        0
    };
    let flags = turn.flags;
    pad_flags(flags, 0x07, place, report);
    Smd {
        name,
        x: length(i32_at(r, 4)),
        y: length(i32_at(r, 8)),
        dx: doubled(u32::from(u16_at(r, 12))),
        dy: doubled(u32::from(u16_at(r, 14))),
        layer: u32::from(r[3]),
        roundness,
        rot: rot(turn.angle, place, report),
        stop: flags & 0x01 == 0,
        thermals: flags & 0x04 == 0,
        cream: flags & 0x02 == 0,
        extra: Vec::new(),
    }
}

/// The electrical types of a pin, by their number in the file.
const DIRECTIONS: [&str; 9] = ["nc", "in", "out", "io", "oc", "pwr", "pas", "hiz", "sup"];
const LENGTHS: [&str; 4] = ["point", "short", "middle", "long"];
const FUNCTIONS: [&str; 4] = ["none", "dot", "clk", "dotclk"];
const VISIBLE: [&str; 4] = ["off", "pad", "pin", "both"];

/// A pin.
pub fn pin(r: &Record, name: String, place: &str, report: &mut Report) -> Pin {
    let kind = r[12];
    let direction = match DIRECTIONS.get(usize::from(kind & 0x0F)) {
        Some(d) => *d,
        None => {
            report.note(
                codes::VALUE,
                "unknown direction of a pin (read as io)",
                place,
            );
            "io"
        }
    };
    if r[2] & 0x3C != 0 {
        report.note(codes::VALUE, "unknown flags of a pin", place);
    }
    let turn = match kind >> 6 {
        1 => "R90",
        2 => "R180",
        3 => "R270",
        _ => "",
    };
    Pin {
        name,
        x: length(i32_at(r, 4)),
        y: length(i32_at(r, 8)),
        length: LENGTHS[usize::from((kind >> 4) & 3)].to_owned(),
        direction: direction.to_owned(),
        function: FUNCTIONS[usize::from(r[2] & 3)].to_owned(),
        visible: VISIBLE[usize::from(r[2] >> 6)].to_owned(),
        swaplevel: r[13].to_string(),
        rot: turn.to_owned(),
        extra: Vec::new(),
    }
}

/// A text.
pub fn text(r: &Record, content: String, place: &str, report: &mut Report) -> Text {
    let mut extra = Vec::new();
    match r[2] {
        0 => extra.push(pair("font", "vector")),
        1 => {}
        2 => extra.push(pair("font", "fixed")),
        _ => report.note(codes::VALUE, "unknown font of a text", place),
    }
    let ratio = (r[14] >> 2) & 0x1F;
    if ratio != 8 {
        extra.push(pair("ratio", ratio.to_string()));
    }
    Text {
        layer: u32::from(r[3]),
        x: length(i32_at(r, 4)),
        y: length(i32_at(r, 8)),
        size: doubled(u32::from(u16_at(r, 12))),
        rot: rot(u16_at(r, 16), place, report),
        content,
        extra,
    }
}

fn element(name: &str, attrs: Vec<(String, String)>, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs,
        text: String::new(),
        children,
    }
}

/// A polygon with its vertices. The model keeps polygons as XML elements.
pub fn polygon(head: &Record, vertices: &[Record], place: &str, report: &mut Report) -> Item {
    let mut attrs = vec![
        pair("width", fmt_mm(doubled(u32::from(u16_at(head, 12))))),
        pair("layer", head[18].to_string()),
    ];
    let spacing = doubled(u32::from(u16_at(head, 14)));
    if spacing != DEFAULT_SPACING && spacing != 0 {
        attrs.push(pair("spacing", fmt_mm(spacing)));
    }
    let isolate = doubled(u32::from(u16_at(head, 16)));
    if isolate != 0 {
        attrs.push(pair("isolate", fmt_mm(isolate)));
    }
    if head[19] & !0x8E != 0 {
        report.note(
            codes::VALUE,
            "pour, thermals or orphans flags of a polygon (defaults used)",
            place,
        );
    }
    let mut children = Vec::new();
    for v in vertices {
        if v[0] != WIRE && v[0] != ARC {
            report.note(codes::SKIPPED, "record inside a polygon", place);
            continue;
        }
        let side = wire(v, place, report);
        let mut a = vec![pair("x", fmt_mm(side.x1)), pair("y", fmt_mm(side.y1))];
        if side.curve != 0 {
            a.push(pair("curve", fmt_mm(side.curve)));
        }
        children.push(element("vertex", a, Vec::new()));
    }
    Item::Other(element("polygon", attrs, children))
}

/// A drawing frame. The model keeps frames as XML elements.
pub fn frame(r: &Record, place: &str, report: &mut Report) -> Item {
    let mut attrs = vec![
        pair("x1", fmt_mm(length(i32_at(r, 4)))),
        pair("y1", fmt_mm(length(i32_at(r, 8)))),
        pair("x2", fmt_mm(length(i32_at(r, 12)))),
        pair("y2", fmt_mm(length(i32_at(r, 16)))),
        pair("columns", r[20].to_string()),
        pair("rows", r[21].to_string()),
        pair("layer", r[3].to_string()),
    ];
    match r[22] {
        0 => {
            for side in ["left", "top", "right", "bottom"] {
                attrs.push(pair(&format!("border-{side}"), "no"));
            }
        }
        0x0F => {}
        _ => report.note(
            codes::VALUE,
            "borders of a frame (all four are shown)",
            place,
        ),
    }
    Item::Other(element("frame", attrs, Vec::new()))
}
