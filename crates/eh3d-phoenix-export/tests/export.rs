//! The manufacturing export: Gerber and Excellon files read back by the parsers of EH3D-CORE-LIBS,
//! the package read back by EH3D-COMMON, and the same bytes for the same board.

use std::collections::BTreeMap;
use std::io::Cursor;

use chrono::{TimeZone, Utc};
use eh3d_common::Package;
use eh3d_parser_gerber::Operation;
use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_export::{codes, export_package, ExportOptions, Exported};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use uuid::Uuid;

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}
fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}
fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

struct Fixture {
    board: Board,
    top: Id,
    bottom: Id,
}

/// Two copper layers, a mask, a silkscreen and an outline; two components of a footprint with a
/// round through-hole pad and a rectangular SMD pad; a track with a line and an arc; two vias; a
/// zone.
fn fixture(second_rotation: i64) -> Fixture {
    let (top, bottom, mask, silk, outline) = (id(1), id(2), id(3), id(4), id(5));
    let net = id(6);
    let mut b = Board {
        name: "Test board".into(),
        outline: vec![
            pt(0, 0),
            pt(30_000_000, 0),
            pt(30_000_000, 20_000_000),
            pt(0, 20_000_000),
        ],
        ..Board::default()
    };
    for (l, name, kind) in [
        (top, "Top copper", LayerKind::Copper),
        (bottom, "Bottom copper", LayerKind::Copper),
        (mask, "Top mask", LayerKind::SolderMask),
        (silk, "Top silk", LayerKind::Silkscreen),
        (outline, "Outline", LayerKind::Outline),
    ] {
        b.layers.insert(
            l,
            Layer {
                name: name.into(),
                kind,
                side: (kind == LayerKind::Silkscreen).then_some(Side::Top),
            },
        );
    }
    b.nets.insert(net, Net { name: "N".into() });
    let fp = id(10);
    let (round, rect) = (id(11), id(12));
    let mut pads = BTreeMap::new();
    pads.insert(
        round,
        Pad {
            number: "1".into(),
            shape: PadShape::Circle {
                diameter: nm(1_600_000),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: Some(nm(800_000)),
            layers: vec![top, bottom, mask],
            roundness: 0,
            plated: true,
        },
    );
    pads.insert(
        rect,
        Pad {
            number: "2".into(),
            shape: PadShape::Rect {
                width: nm(1_000_000),
                height: nm(2_000_000),
            },
            at: pt(5_000_000, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![top, mask],
            roundness: 0,
            plated: true,
        },
    );
    b.footprints.insert(
        fp,
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    for (cid, refdes, x, rot) in [
        (id(20), "R1", 5_000_000, 0),
        (id(21), "R2", 20_000_000, second_rotation),
    ] {
        b.components.insert(
            cid,
            Component {
                refdes: refdes.into(),
                value: "1k".into(),
                symbol: None,
                footprint: fp,
                at: pt(x, 10_000_000),
                rotation: Udeg::new(rot).unwrap(),
                side: Side::Top,
            },
        );
        b.pad_nets.push(PadNet {
            component: cid,
            pad: round,
            net,
        });
    }
    b.pad_nets.sort_by_key(|p| p.pad_ref());
    b.tracks.insert(
        id(30),
        Track {
            net,
            layer: top,
            width: nm(250_000),
            path: vec![
                Segment::Line {
                    start: pt(5_000_000, 10_000_000),
                    end: pt(10_000_000, 10_000_000),
                },
                Segment::Arc {
                    start: pt(10_000_000, 10_000_000),
                    end: pt(12_000_000, 12_000_000),
                    sweep: Udeg::new(90_000_000).unwrap(),
                },
            ],
        },
    );
    for (vid, x) in [(id(40), 12_000_000), (id(41), 14_000_000)] {
        b.vias.insert(
            vid,
            Via {
                net,
                at: pt(x, 12_000_000),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
    }
    b.zones.insert(
        id(50),
        Zone {
            net,
            layer: bottom,
            outline: vec![
                pt(1_000_000, 1_000_000),
                pt(29_000_000, 1_000_000),
                pt(29_000_000, 19_000_000),
            ],
            clearance: nm(200_000),
            cutouts: Vec::new(),
        },
    );
    Fixture {
        board: b,
        top,
        bottom,
    }
}

/// Sets or clears the copper stack of the fixture and exports it.
fn export(f: &mut Fixture, with_stack: bool) -> Exported {
    f.board.copper_stack = if with_stack {
        vec![f.top, f.bottom]
    } else {
        Vec::new()
    };
    let options = ExportOptions {
        project_id: Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0),
        created: Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
        outline_width: nm(100_000),
    };
    export_package(&f.board, &options).unwrap()
}

fn codes_of(diags: &[eh3d_diagnostic::Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter_map(|d| d.code.as_deref().map(str::to_owned))
        .collect()
}

#[test]
fn the_same_board_gives_the_same_package_bytes() {
    let mut f = fixture(0);
    let a = export(&mut f, true);
    let b = export(&mut fixture(0), true);
    assert_eq!(a.package.to_bytes().unwrap(), b.package.to_bytes().unwrap());
}

#[test]
fn the_package_reads_back_with_its_layers_and_integrity() {
    let mut f = fixture(0);
    let exported = export(&mut f, true);
    let bytes = exported.package.to_bytes().unwrap();
    let read = Package::read(Cursor::new(bytes)).unwrap();
    let paths: Vec<&str> = read
        .metadata
        .layers
        .iter()
        .map(|l| l.path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec![
            "layers/01_bottom_copper.gbr",
            "layers/02_top_copper.gbr",
            "layers/03_top_mask.gbr",
            "layers/04_top_silk.gbr",
            "layers/05_outline.gbr",
            "layers/drill_pth.drl"
        ]
    );
    for p in &paths {
        assert!(read.entry(p).is_some(), "{p}");
    }
    assert_eq!(read.metadata.name, "Test board");
}

#[test]
fn the_copper_layer_reads_back_with_every_item() {
    let mut f = fixture(0);
    let exported = export(&mut f, true);
    let text = std::str::from_utf8(exported.package.entry("layers/02_top_copper.gbr").unwrap())
        .unwrap()
        .to_owned();
    let file = eh3d_parser_gerber::parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let count = |pred: fn(&Operation) -> bool| file.commands.iter().filter(|c| pred(&c.op)).count();
    // two round pads, two rectangular pads and two vias are flashed
    assert_eq!(count(|o| matches!(o, Operation::Flash { .. })), 6);
    assert_eq!(count(|o| matches!(o, Operation::Line { .. })), 1);
    assert_eq!(count(|o| matches!(o, Operation::Arc { .. })), 1);
    assert_eq!(count(|o| matches!(o, Operation::RegionStart)), 0);
    assert!(text.contains("Copper,L2,Bot") || text.contains("Copper,L1,Top"));
    // 1.6 mm circles, 0.25 mm and 0.6 mm circles, one rectangle: four apertures
    assert_eq!(file.apertures.len(), 4);
}

#[test]
fn the_zone_is_a_region_and_the_outline_is_four_lines() {
    let mut f = fixture(0);
    let exported = export(&mut f, true);
    let bottom = std::str::from_utf8(
        exported
            .package
            .entry("layers/01_bottom_copper.gbr")
            .unwrap(),
    )
    .unwrap()
    .to_owned();
    let file = eh3d_parser_gerber::parse(&bottom);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    assert_eq!(
        file.commands
            .iter()
            .filter(|c| matches!(c.op, Operation::RegionStart))
            .count(),
        1
    );
    assert!(codes_of(&exported.diagnostics).contains(&codes::ZONE_NOT_POURED.to_string()));
    let outline = std::str::from_utf8(exported.package.entry("layers/05_outline.gbr").unwrap())
        .unwrap()
        .to_owned();
    let file = eh3d_parser_gerber::parse(&outline);
    assert_eq!(
        file.commands
            .iter()
            .filter(|c| matches!(c.op, Operation::Line { .. }))
            .count(),
        4
    );
    assert!(outline.contains("Profile,NP"));
}

#[test]
fn the_drill_file_holds_every_hole_in_order() {
    let mut f = fixture(0);
    let exported = export(&mut f, true);
    let text = std::str::from_utf8(exported.package.entry("layers/drill_pth.drl").unwrap())
        .unwrap()
        .to_owned();
    let file = eh3d_parser_excellon::parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    // two 0.3 mm vias and two 0.8 mm pads
    assert_eq!(file.drills.len(), 4);
    assert_eq!(file.tools.len(), 2);
    assert!((file.tools[&1].diameter - 0.3).abs() < 1e-9);
    assert!((file.tools[&2].diameter - 0.8).abs() < 1e-9);
    let first_tool: Vec<u32> = file.drills.iter().map(|h| h.tool).collect();
    assert_eq!(first_tool, vec![1, 1, 2, 2]);
}

#[test]
fn what_the_files_cannot_say_is_reported() {
    let mut f = fixture(0);
    let without = export(&mut f, false);
    assert!(codes_of(&without.diagnostics).contains(&codes::STACK_ORDER_MISSING.to_string()));
    let top =
        std::str::from_utf8(without.package.entry("layers/02_top_copper.gbr").unwrap()).unwrap();
    assert!(!top.contains("FileFunction"));
    let with = export(&mut f, true);
    assert!(!codes_of(&with.diagnostics).contains(&codes::STACK_ORDER_MISSING.to_string()));
    // a pad rotated by 45 degrees is flashed unrotated and says so
    let mut rotated = fixture(45_000_000);
    let out = export(&mut rotated, true);
    assert!(codes_of(&out.diagnostics).contains(&codes::PAD_ROTATION.to_string()));
    let plain = codes_of(&with.diagnostics);
    assert!(!plain.contains(&codes::PAD_ROTATION.to_string()));
}

#[test]
fn a_right_angle_swaps_the_sides_of_a_rectangular_pad() {
    let mut f = fixture(90_000_000);
    let out = export(&mut f, true);
    let text = std::str::from_utf8(out.package.entry("layers/02_top_copper.gbr").unwrap())
        .unwrap()
        .to_owned();
    let file = eh3d_parser_gerber::parse(&text);
    // the rectangle 1 x 2 mm and the same rectangle turned by 90 degrees (2 x 1 mm)
    let rects = file
        .apertures
        .values()
        .filter(|a| matches!(a, eh3d_parser_gerber::Aperture::Rectangle { .. }))
        .count();
    assert_eq!(rects, 2);
    assert!(!codes_of(&out.diagnostics).contains(&codes::PAD_ROTATION.to_string()));
}

/// The fixture with a text and a stroke on its silkscreen layer.
fn with_silkscreen(f: &mut Fixture, text: &str) {
    let silk = id(4);
    f.board.silkscreen.insert(
        id(60),
        SilkItem::Text {
            layer: silk,
            at: pt(2_000_000, 8_000_000),
            text: text.into(),
            height: nm(1_000_000),
            stroke: nm(120_000),
            rotation: Udeg::ZERO,
            mirrored: false,
        },
    );
    f.board.silkscreen.insert(
        id(61),
        SilkItem::Stroke {
            layer: silk,
            width: nm(150_000),
            path: vec![
                Segment::Line {
                    start: pt(1_000_000, 1_000_000),
                    end: pt(9_000_000, 1_000_000),
                },
                Segment::Arc {
                    start: pt(9_000_000, 1_000_000),
                    end: pt(10_000_000, 2_000_000),
                    sweep: Udeg::new(90_000_000).unwrap(),
                },
            ],
        },
    );
}

#[test]
fn the_silkscreen_layer_is_a_legend_file_with_its_strokes() {
    let mut f = fixture(0);
    with_silkscreen(&mut f, "R1 10K");
    let out = export(&mut f, true);
    let text = std::str::from_utf8(out.package.entry("layers/04_top_silk.gbr").unwrap())
        .unwrap()
        .to_owned();
    assert!(text.contains("%TF.FileFunction,Legend,Top*%"), "{text}");
    let file = eh3d_parser_gerber::parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let lines = file
        .commands
        .iter()
        .filter(|c| matches!(c.op, Operation::Line { .. }))
        .count();
    let arcs = file
        .commands
        .iter()
        .filter(|c| matches!(c.op, Operation::Arc { .. }))
        .count();
    let drawn = f.board.silkscreen[&id(60)].drawn().unwrap();
    // every stroke of the characters, and the line of the stroke item
    assert_eq!(lines, drawn.pieces.len() + 1);
    assert_eq!(arcs, 1);
    // nothing is flashed on the legend layer
    assert_eq!(
        file.commands
            .iter()
            .filter(|c| matches!(c.op, Operation::Flash { .. }))
            .count(),
        0
    );
    // two line widths, so two apertures
    assert_eq!(file.apertures.len(), 2);
    assert!(!codes_of(&out.diagnostics).contains(&codes::SILK_SIDE_MISSING.to_string()));
    // the same board gives the same bytes
    let mut g = fixture(0);
    with_silkscreen(&mut g, "R1 10K");
    assert_eq!(
        out.package.to_bytes().unwrap(),
        export(&mut g, true).package.to_bytes().unwrap()
    );
}

#[test]
fn what_the_legend_cannot_say_is_reported() {
    let mut f = fixture(0);
    with_silkscreen(&mut f, "A\u{3a9}");
    f.board.layers.get_mut(&id(4)).unwrap().side = None;
    let out = export(&mut f, true);
    let found = codes_of(&out.diagnostics);
    assert!(found.contains(&codes::SILK_SIDE_MISSING.to_string()));
    assert!(found.contains(&codes::SILK_NO_GLYPH.to_string()));
    let text = std::str::from_utf8(out.package.entry("layers/04_top_silk.gbr").unwrap()).unwrap();
    assert!(!text.contains("FileFunction"));
    // an item that cannot be drawn is left out and says so
    let mut g = fixture(0);
    with_silkscreen(&mut g, "X");
    if let Some(SilkItem::Text { at, .. }) = g.board.silkscreen.get_mut(&id(60)) {
        *at = pt(Nm::MAX.get(), 0);
    }
    let out = export(&mut g, true);
    assert!(codes_of(&out.diagnostics).contains(&codes::SILK_UNDRAWN.to_string()));
}

#[test]
fn a_board_without_silkscreen_items_gives_an_empty_legend() {
    let mut f = fixture(0);
    let out = export(&mut f, true);
    let text = std::str::from_utf8(out.package.entry("layers/04_top_silk.gbr").unwrap()).unwrap();
    assert!(text.contains("Legend,Top"));
    assert!(eh3d_parser_gerber::parse(text).is_clean());
}
