// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading invented binary boards: the XML they become is read by the reader of XML boards, and
//! damaged files give an error or a report, never a panic.

#[path = "builder/board.rs"]
mod builder;

use builder::BoardFile;
use eh3d_phoenix_board::{Board, Component};
use eh3d_phoenix_eagle_board::read_board;
use eh3d_phoenix_eaglebin::{codes, read_binary_board, read_binary_library, Limits};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, StepClock};
use eh3d_phoenix_rules::RuleValues;

fn board_of(xml: &str) -> Board {
    let mut generator = IdGenerator::new(StepClock::new(1_000, 1), DeterministicRandom::new(1));
    let mut new_id = move || generator.generate().unwrap();
    read_board(xml, &mut new_id).unwrap().board
}

fn component<'a>(board: &'a Board, refdes: &str) -> &'a Component {
    board
        .components
        .values()
        .find(|c| c.refdes == refdes)
        .unwrap_or_else(|| panic!("{refdes} is not there"))
}

#[test]
fn a_binary_board_becomes_an_xml_board_that_is_read_as_one() {
    for version in [(3, 55), (4, 16), (5, 11)] {
        let read = read_binary_board(&BoardFile::new(version).bytes(), &Limits::default()).unwrap();
        let report = &read.report;
        assert_eq!(report.version, format!("{}.{}", version.0, version.1));
        for (kind, count) in [
            ("library", 1),
            ("footprint", 1),
            ("element", 2),
            ("signal", 1),
            ("contactref", 2),
            ("via", 1),
            ("airwire", 1),
            ("polygon", 1),
            ("class", 2),
            ("attribute", 1),
        ] {
            assert_eq!(report.read_of(kind), count, "{version:?} {kind}");
        }
        assert_eq!(report.noted(codes::STRINGS), 0, "{:?}", report.notes);
        assert_eq!(report.losses(), 0, "{:?}", report.notes);

        let board = board_of(&read.xml);
        assert_eq!(board.components.len(), 2);
        let r1 = component(&board, "R1");
        assert_eq!((r1.value.as_str(), r1.at.x_nm.get()), ("10k", 5_000_000));
        let r2 = component(&board, "R2");
        assert_eq!(r2.value, "4k7 precision resistor");
        assert_eq!(r2.rotation.get(), 90_000_000);
        assert!(board.nets.values().any(|n| n.name == "SUPPLY_5V"));
        assert_eq!(board.pad_nets.len(), 2);
        assert_eq!(board.vias.len(), 1);
        assert_eq!(board.zones.len(), 1);
        assert!(!board.tracks.is_empty());
        assert!(board.outline.len() >= 4, "{:?}", board.outline);
    }
}

#[test]
fn the_xml_keeps_the_rules_the_classes_and_the_details_of_the_objects() {
    let read = read_binary_board(&BoardFile::new((4, 16)).bytes(), &Limits::default()).unwrap();
    let xml = &read.xml;
    for wanted in [
        "<attribute name=\"AUTHOR\" value=\"test\"",
        "<class number=\"1\" name=\"power\" width=\"1.016\" drill=\"1.016\"",
        "<clearance class=\"1\" value=\"0.508\"",
        "name=\"NAME\"",
        "name=\"MPN\"",
        "extent=\"1-16\"",
        "pour=\"hatch\"",
        "rank=\"2\"",
        "isolate=\"0.254\"",
        "spacing=\"0.6096\"",
        "smashed=\"yes\"",
        "<param name=\"msWidth\" value=\"0.254mm\"",
        "<param name=\"rvPadTop\" value=\"0.25\"",
    ] {
        assert!(xml.contains(wanted), "{wanted} is not in {xml}");
    }
    assert!(
        !xml.contains("name=\"empty\""),
        "the empty library stays out"
    );
    let (rules, _) = RuleValues::from_eagle_board(xml).unwrap();
    assert_eq!(rules.min_track_width.get(), 254_000);
    assert_eq!(rules.min_drill.get(), 609_600);
    assert_eq!(rules.min_clearance.get(), 203_200);
}

#[test]
fn the_kinds_of_file_are_told_apart() {
    let mut file = BoardFile::new((4, 16));
    for root in [0x14u8, 0x15, 0x42] {
        file.root = root;
        let error = read_binary_board(&file.bytes(), &Limits::default()).unwrap_err();
        assert_eq!(error.code, codes::NOT_BOARD, "0x{root:02X}");
    }
    file.root = 0x1B;
    let error = read_binary_library(&file.bytes(), &Limits::default()).unwrap_err();
    assert_eq!(error.code, codes::NOT_LIBRARY);
    let error = read_binary_board(b"<eagle/>", &Limits::default()).unwrap_err();
    assert_eq!(error.code, codes::NOT_BINARY);
}

#[test]
fn a_damaged_board_never_panics() {
    let whole = BoardFile::new((4, 16)).bytes();
    for length in 0..whole.len() {
        let _ = read_binary_board(&whole[..length], &Limits::default());
    }
    // a fixed seed: the same bytes are changed on every run
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for version in [(3, 55), (5, 11)] {
        let whole = BoardFile::new(version).bytes();
        for _ in 0..4_000 {
            let mut bytes = whole.clone();
            for _ in 0..3 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let at = (seed % bytes.len() as u64) as usize;
                bytes[at] = (seed >> 32) as u8;
            }
            if let Ok(read) = read_binary_board(&bytes, &Limits::default()) {
                assert!(read.xml.starts_with("<?xml"));
            }
        }
    }
}

#[test]
fn the_limit_of_items_is_applied() {
    let limits = Limits {
        max_items: 3,
        ..Limits::default()
    };
    let error = read_binary_board(&BoardFile::new((4, 16)).bytes(), &limits).unwrap_err();
    assert_eq!(error.code, codes::LIMIT);
}
