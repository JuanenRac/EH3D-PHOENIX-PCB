// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading invented binary libraries: every kind of record, the references, the report, and
//! damaged files.

mod builder;

use builder::{drawing, old_pad, sample, sample_3, File, R};
use eh3d_phoenix_eaglebin::codes;
use eh3d_phoenix_eaglebin::{read_binary_library, BinError, Conversion, Limits};
use eh3d_phoenix_libedit::model::{Item, PadShape};
use eh3d_phoenix_libedit::write::verify_round_trip;

fn read(bytes: &[u8]) -> Result<Conversion, BinError> {
    read_binary_library(bytes, &Limits::default())
}

fn code_of(bytes: &[u8]) -> &'static str {
    read(bytes).unwrap_err().code
}

fn extra<'a>(list: &'a [(String, String)], name: &str) -> Option<&'a str> {
    list.iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

#[test]
fn the_drawings_of_footprints_are_read_into_the_model() {
    let read = read(&sample().bytes()).unwrap();
    let lib = &read.library;
    let chip = lib.package("R0603").unwrap();
    assert_eq!(chip.description[0].text, "Chip resistor");
    assert_eq!(chip.items.len(), 9);
    match &chip.items[0] {
        Item::Smd(s) => {
            assert_eq!(
                (s.name.as_str(), s.x, s.dx, s.dy),
                ("1", -800_000, 300_000, 1_500_000)
            );
            assert_eq!((s.layer, s.roundness, s.stop, s.cream), (1, 25, true, true));
        }
        other => panic!("{other:?}"),
    }
    match (&chip.items[2], &chip.items[3], &chip.items[4]) {
        (Item::Wire(quarter), Item::Wire(general), Item::Wire(other)) => {
            assert_eq!((quarter.curve, quarter.width), (90_000_000, 127_000));
            assert_eq!(extra(&quarter.extra, "cap"), Some("flat"));
            assert_eq!(general.curve, 126_869_898);
            assert_eq!(
                (general.x1, general.y1, general.x2, general.y2),
                (500_000, 0, -300_000, 400_000)
            );
            assert_eq!(other.curve, -106_260_205);
            assert_eq!((other.x1, other.y2), (290_000, -400_000));
            assert_eq!(extra(&other.extra, "style"), Some("shortdash"));
            assert_eq!(extra(&other.extra, "cap"), None);
        }
        items => panic!("{items:?}"),
    }
    match (
        &chip.items[5],
        &chip.items[6],
        &chip.items[7],
        &chip.items[8],
    ) {
        (Item::Circle(c), Item::Rect(r), Item::Hole(h), Item::Text(t)) => {
            assert_eq!((c.x, c.radius, c.width), (2_000_000, 317_500, 254_000));
            assert_eq!(
                (r.x1, r.y1, r.x2, r.y2, r.rot.as_str()),
                (873_000, 1_625_600, 1_127_000, 2_565_400, "R90")
            );
            assert_eq!((h.x, h.y, h.drill), (100_000, -100_000, 1_000_000));
            assert_eq!(
                (t.content.as_str(), t.size, t.rot.as_str(), t.layer),
                (">VALUE of the part", 1_000_000, "MR90", 25)
            );
            assert_eq!(extra(&t.extra, "font"), Some("vector"));
            assert_eq!(extra(&t.extra, "ratio"), None);
        }
        items => panic!("{items:?}"),
    }
}

#[test]
fn pads_polygons_and_frames_are_read() {
    let read = read(&sample().bytes()).unwrap();
    let header = read.library.package("HDR").unwrap();
    assert_eq!(header.items.len(), 4);
    match (&header.items[0], &header.items[1]) {
        (Item::Pad(a), Item::Pad(b)) => {
            assert_eq!(
                (a.shape, a.drill, a.diameter, a.first),
                (PadShape::Square, 812_800, 1_400_000, true)
            );
            // an upright long pad is a long pad turned by 90 degrees
            assert_eq!(
                (b.name.as_str(), b.shape, b.rot.as_str(), b.diameter),
                ("PAD_TWO", PadShape::Long, "R90", 0)
            );
        }
        items => panic!("{items:?}"),
    }
    match (&header.items[2], &header.items[3]) {
        (Item::Other(polygon), Item::Other(frame)) => {
            assert_eq!(polygon.name, "polygon");
            assert_eq!(polygon.attr("width"), Some("0.1016"));
            assert_eq!(polygon.attr("spacing"), Some("0.508"));
            assert_eq!(polygon.attr("isolate"), None);
            assert_eq!(polygon.children.len(), 3);
            assert_eq!(polygon.children[1].attr("x"), Some("1"));
            assert_eq!(polygon.children[1].attr("curve"), Some("90"));
            assert_eq!(frame.name, "frame");
            assert_eq!(frame.attr("x2"), Some("10"));
            assert_eq!(frame.attr("columns"), Some("4"));
            assert_eq!(frame.attr("border-right"), Some("no"));
            assert_eq!(frame.attr("border-bottom"), Some("no"));
            assert_eq!(frame.attr("border-left"), None);
            assert_eq!(frame.attr("border-top"), None);
        }
        items => panic!("{items:?}"),
    }
    assert_eq!(read.library.kept_raw, 2);
}

#[test]
fn symbols_and_device_sets_are_read_with_their_connections() {
    let read = read(&sample().bytes()).unwrap();
    let lib = &read.library;
    let symbol = lib.symbol("R").unwrap();
    let pins: Vec<_> = symbol.pins().map(|(_, p)| p.clone()).collect();
    assert_eq!(pins.len(), 2);
    assert_eq!(
        (
            pins[0].x,
            pins[0].length.as_str(),
            pins[0].direction.as_str(),
            pins[0].visible.as_str(),
            pins[0].function.as_str(),
            pins[0].rot.as_str()
        ),
        (-5_080_000, "short", "pas", "off", "none", "")
    );
    assert_eq!(
        (
            pins[1].visible.as_str(),
            pins[1].function.as_str(),
            pins[1].rot.as_str()
        ),
        ("both", "dot", "R180")
    );
    let set = lib.deviceset("RES").unwrap();
    assert_eq!((set.prefix.as_str(), set.uservalue), ("R", true));
    assert_eq!(set.description[0].text, "<b>Resistor</b>");
    assert_eq!(set.gates[0].symbol, "R");
    assert_eq!((set.gates[0].x, set.gates[0].y), (-12_700_000, 10_160_000));
    assert_eq!(set.devices.len(), 2);
    let device = &set.devices[0];
    assert_eq!(device.name, "0603");
    assert_eq!(device.package.as_deref(), Some("R0603"));
    let connects: Vec<_> = device
        .connects
        .iter()
        .map(|c| (c.gate.as_str(), c.pin.as_str(), c.pad.as_str()))
        .collect();
    assert_eq!(connects, [("G$1", "1", "2"), ("G$1", "2", "1")]);
    let names: Vec<_> = device.rest[0]
        .children
        .iter()
        .map(|t| t.attr("name").unwrap().to_owned())
        .collect();
    assert_eq!(names, ["", "HC"]);
    assert_eq!(set.devices[1].name, "");
    assert_eq!(set.devices[1].package, None);
}

#[test]
fn the_report_counts_what_was_read_and_the_library_can_be_saved() {
    let read = read(&sample().bytes()).unwrap();
    let r = &read.report;
    assert_eq!(r.version, "4.9");
    for (kind, count) in [
        ("footprint", 2),
        ("symbol", 1),
        ("device set", 1),
        ("device", 2),
        ("gate", 1),
        ("connection", 2),
        ("pad", 2),
        ("smd", 2),
        ("wire", 1),
        ("arc", 3),
        ("circle", 1),
        ("rectangle", 1),
        ("hole", 1),
        ("polygon", 1),
        ("frame", 1),
        ("text", 2),
        ("pin", 2),
        ("layer", 3),
        ("description", 3),
    ] {
        assert_eq!(r.read_of(kind), count, "{kind}");
    }
    assert!(!r.has_losses(), "{:?}", r.notes);
    // the two arcs whose angle is computed, and the settings record
    assert_eq!(r.noted(codes::NOT_READ), 3);
    let text = verify_round_trip(&read.library).unwrap();
    assert!(text.contains("<description>Test library"));
    assert!(text.contains("name=\"tPlace\""));
    assert!(text.contains("curve=\"126.869898\""));
    assert!(text.contains("uservalue=\"yes\""));
    assert!(!text.contains("addlevel"));
}

#[test]
fn version_3_devices_and_long_texts_are_read() {
    let read = read(&sample_3().bytes()).unwrap();
    assert_eq!(read.report.version, "3.55");
    let set = read.library.deviceset("TWO-GATES").unwrap();
    assert_eq!(set.prefix, "IC");
    assert_eq!(set.gates[0].symbol, "TWO");
    assert_eq!(extra(&set.gates[0].extra, "addlevel"), Some("always"));
    assert_eq!(extra(&set.gates[1].extra, "addlevel"), Some("must"));
    assert_eq!(set.devices.len(), 1);
    assert_eq!(set.devices[0].package.as_deref(), Some("DIL"));
    let connects: Vec<_> = set.devices[0]
        .connects
        .iter()
        .map(|c| (c.gate.as_str(), c.pin.as_str(), c.pad.as_str()))
        .collect();
    assert_eq!(connects, [("A", "IN", "3"), ("P", "VCC", "1")]);
    let dil = read.library.package("DIL").unwrap();
    match (&dil.items[0], &dil.items[2]) {
        (Item::Smd(a), Item::Pad(b)) => {
            assert_eq!((a.name.as_str(), a.rot.as_str(), a.dx), ("1", "", 300_000));
            // the upright long pad of version 3
            assert_eq!(
                (b.name.as_str(), b.shape, b.rot.as_str(), b.stop),
                ("3", PadShape::Long, "R90", true)
            );
        }
        items => panic!("{items:?}"),
    }
    match &dil.items[3] {
        Item::Text(t) => {
            assert_eq!(t.content, "a text that goes on beyond one record");
            // byte 2 is not the font before version 4
            assert_eq!(extra(&t.extra, "font"), None);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(dil.items.len(), 4);
    assert_eq!(dil.description[0].text, "Dual in line");
    assert!(!read.report.has_losses(), "{:?}", read.report.notes);
    verify_round_trip(&read.library).unwrap();
}

#[test]
fn before_version_4_the_name_of_a_footprint_starts_at_byte_16() {
    let mut f = sample_3();
    // a name that fills the eight bytes, a description of three, and a pad whose name puts a
    // byte with the bit 0x04 where version 4 keeps its flags
    f.packages = vec![drawing(
        R::new(0x1E).text(13, b"abc").text(16, b"AB-SOT23"),
        vec![old_pad(0x2A, 0, b"12D"), old_pad(0x2B, 10, b"34D")],
    )];
    f.strings.clear();
    let read = read(&f.bytes()).unwrap();
    let footprint = read.library.package("AB-SOT23").unwrap();
    assert_eq!(footprint.description[0].text, "abc");
    match (&footprint.items[0], &footprint.items[1]) {
        (Item::Pad(a), Item::Smd(b)) => {
            assert_eq!((a.name.as_str(), a.thermals), ("12D", true));
            assert_eq!((b.name.as_str(), b.thermals), ("34D", true));
        }
        items => panic!("{items:?}"),
    }
}

#[test]
fn each_bit_of_a_frame_turns_one_border_off() {
    for (flags, off) in [
        (0x00, &[][..]),
        (0x01, &["left"][..]),
        (0x02, &["top"][..]),
        (0x04, &["right"][..]),
        (0x08, &["bottom"][..]),
        (0x0F, &["left", "top", "right", "bottom"][..]),
    ] {
        let mut f = sample();
        let last = f.packages[1].len() - 1;
        f.packages[1][last] = f.packages[1][last].b(22, flags);
        let read = read(&f.bytes()).unwrap();
        let header = read.library.package("HDR").unwrap();
        let Item::Other(frame) = &header.items[3] else {
            panic!("{:?}", header.items[3]);
        };
        for side in ["left", "top", "right", "bottom"] {
            let wanted = off.contains(&side).then_some("no");
            assert_eq!(frame.attr(&format!("border-{side}")), wanted, "{flags}");
        }
        // the left and the top bits are only inferred, and the report says so
        assert_eq!(read.report.has_losses(), flags & 0x03 != 0, "{flags}");
    }
}

#[test]
fn what_is_not_read_is_in_the_report() {
    let mut f = sample();
    // an unknown record in the first footprint, a gate of a missing symbol and a long string
    // that nothing asks for
    f.packages[0].push(R::new(0x74));
    let children = f.packages[0].len() as u16 - 1;
    f.packages[0][0] = f.packages[0][0].u16(2, children);
    let last = f.sets[0].len() - 1;
    f.sets[0][last] = f.sets[0][last].u16(14, 9);
    f.strings.push(b"nobody");
    let read = read(&f.bytes()).unwrap();
    let r = &read.report;
    assert_eq!(r.noted(codes::SKIPPED), 2, "{:?}", r.notes);
    assert!(r.noted(codes::REFERENCE) >= 1);
    assert_eq!(r.noted(codes::STRINGS), 1);
    assert!(r.has_losses());
    // connections that do not fit are all left out, never guessed
    assert!(read.library.deviceset("RES").unwrap().devices[0]
        .connects
        .is_empty());
    let missing = {
        let mut f = sample();
        f.strings.pop();
        f
    };
    let report = eh3d_phoenix_eaglebin::read_binary_library(&missing.bytes(), &Limits::default())
        .unwrap()
        .report;
    assert_eq!(report.noted(codes::STRINGS), 1);
}

#[test]
fn files_that_cannot_be_read_have_their_code() {
    assert_eq!(code_of(b"<?xml version=\"1.0\"?>"), codes::NOT_BINARY);
    assert_eq!(code_of(&[0x10, 0x80]), codes::NOT_BINARY);
    assert_eq!(code_of(&File::new((6, 0)).bytes()), codes::VERSION);
    let good = sample().bytes();
    // the records are cut short
    assert_eq!(code_of(&good[..100]), codes::STRUCTURE);
    // another object where the library is expected
    let layers = sample().layers.len();
    let mut board = good.clone();
    board[24 * (1 + layers)] = 0x1B;
    assert_eq!(code_of(&board), codes::NOT_LIBRARY);
    // the sections do not add up
    let mut sizes = good.clone();
    sizes[24 * (1 + layers) + 4] += 1;
    assert_eq!(code_of(&sizes), codes::STRUCTURE);
    // a count of records that the file cannot hold, and one over the limit
    let mut long = good.clone();
    long[4..8].copy_from_slice(&1_000_000u32.to_le_bytes());
    assert_eq!(code_of(&long), codes::STRUCTURE);
    long[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(code_of(&long), codes::LIMIT);
}

#[test]
fn the_limits_are_applied() {
    let bytes = sample().bytes();
    for limits in [
        Limits {
            max_bytes: 100,
            ..Limits::default()
        },
        Limits {
            max_records: 10,
            ..Limits::default()
        },
        Limits {
            max_strings: 2,
            ..Limits::default()
        },
        Limits {
            max_items: 5,
            ..Limits::default()
        },
        Limits {
            max_string_bytes: 4,
            ..Limits::default()
        },
        Limits {
            max_work: 3,
            ..Limits::default()
        },
    ] {
        let e = read_binary_library(&bytes, &limits).unwrap_err();
        assert_eq!(e.code, codes::LIMIT, "{limits:?}");
    }
}

/// A small generator with a fixed seed, so that a failure repeats.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn damaged_and_hostile_files_never_panic() {
    let mut random = Random(0x1234_5678_9ABC_DEF1);
    for base in [sample().bytes(), sample_3().bytes()] {
        for round in 0..3_000 {
            let mut bytes = base.clone();
            for _ in 0..1 + random.below(8) {
                let at = random.below(bytes.len());
                match random.below(4) {
                    0 => bytes[at] = random.next() as u8,
                    1 => bytes[at] = 0xFF,
                    2 => bytes[at] = 0x7F,
                    _ => bytes[at] ^= 1 << random.below(8),
                }
            }
            if round % 5 == 0 {
                bytes.truncate(random.below(bytes.len()) + 1);
            }
            if let Ok(read) = read(&bytes) {
                // whatever was read can be turned into text
                let _ = eh3d_phoenix_libedit::to_text(&read.library);
                let _ = read.report.to_text(eh3d_phoenix_rules::Lang::En);
            }
        }
    }
    for length in 0..600 {
        let mut bytes: Vec<u8> = (0..length).map(|_| random.next() as u8).collect();
        if let Some(first) = bytes.first_mut() {
            *first = 0x10;
        }
        if length > 9 {
            bytes[8] = 4;
            bytes[4..8].copy_from_slice(&((length / 24) as u32).to_le_bytes());
        }
        let _ = read(&bytes);
    }
}
