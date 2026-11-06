// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of EAGLE board files on a small file written by hand: the layers, the outline,
//! the packages with their pads, the elements with their turn and side, the signals with their
//! tracks, vias and polygons, the silkscreen, and what is counted as not read.

use eh3d_phoenix_board::{Id, PadShape, Segment, Side, SilkItem};
use eh3d_phoenix_eagle_board::{read_board, ReadBoard, NOT_READ};
use eh3d_phoenix_model::ObjectId;

const FILE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="16" name="Bottom" color="1" fill="1" visible="yes" active="yes"/>
<layer number="20" name="Dimension" color="15" fill="1" visible="yes" active="yes"/>
<layer number="21" name="tPlace" color="7" fill="1" visible="yes" active="yes"/>
<layer number="91" name="Nets" color="2" fill="1" visible="yes" active="yes"/>
</layers>
<board>
<plain>
<wire x1="0" y1="0" x2="20" y2="0" width="0.1" layer="20"/>
<wire x1="20" y1="0" x2="20" y2="10" width="0.1" layer="20"/>
<wire x1="20" y1="10" x2="0" y2="10" width="0.1" layer="20"/>
<wire x1="0" y1="10" x2="0" y2="0" width="0.1" layer="20"/>
<wire x1="1" y1="1" x2="3" y2="1" width="0.2" layer="21"/>
<wire x1="3" y1="1" x2="3" y2="3" width="0.2" layer="21"/>
<text x="1" y="5" size="1" layer="25">hello</text>
</plain>
<libraries>
<library name="lib">
<packages>
<package name="R0603">
<smd name="1" x="-0.8" y="0" dx="0.8" dy="0.9" layer="1"/>
<smd name="2" x="0.8" y="0" dx="0.8" dy="0.9" layer="1" roundness="100"/>
<pad name="3" x="0" y="2" drill="0.8" diameter="1.6" shape="square"/>
<pad name="4" x="0" y="4" drill="0.8" diameter="1.6" shape="long"/>
<wire x1="-1" y1="-1" x2="1" y2="-1" width="0.1" layer="21"/>
</package>
</packages>
</library>
</libraries>
<elements>
<element name="R1" library="lib" package="R0603" value="10k" x="5" y="5" rot="R90"/>
<element name="R2" library="lib" package="R0603" value="4k7" x="12" y="5" rot="MR45.5"/>
<element name="X9" library="lib" package="NOPE" value="" x="1" y="1"/>
</elements>
<signals>
<signal name="GND">
<contactref element="R1" pad="1"/>
<contactref element="R2" pad="2"/>
<contactref element="R9" pad="1"/>
<wire x1="5" y1="5" x2="10" y2="5" width="0.25" layer="1"/>
<wire x1="10" y1="5" x2="10" y2="8" width="0.25" layer="1"/>
<wire x1="1" y1="1" x2="2" y2="2" width="0.25" layer="1"/>
<via x="10" y="8" extent="1-16" drill="0.4" diameter="0.8"/>
<polygon width="0.2" layer="16" isolate="0.3">
<vertex x="1" y="1"/>
<vertex x="9" y="1"/>
<vertex x="9" y="9"/>
</polygon>
<rectangle x1="0" y1="0" x2="1" y2="1" layer="1"/>
</signal>
</signals>
</board>
</drawing>
</eagle>
"#;

/// A counter that gives the ids of the new objects.
fn ids() -> impl FnMut() -> Id {
    let mut n = 2000u16;
    move || {
        n += 1;
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
}

fn read(text: &str) -> ReadBoard {
    read_board(text, &mut ids()).unwrap()
}

fn mm(v: f64) -> i64 {
    (v * 1_000_000.0).round() as i64
}

#[test]
fn the_layers_are_read_with_their_kind_and_the_copper_stack_by_number() {
    let board = read(FILE).board;
    // 1, 16, 20 and 21 are known; 91 is not read
    assert_eq!(board.layers.len(), 4);
    let names: Vec<&str> = board
        .copper_stack
        .iter()
        .map(|id| board.layers[id].name.as_str())
        .collect();
    assert_eq!(names, vec!["Top", "Bottom"]);
    let silk = board
        .layers
        .values()
        .find(|l| l.name == "tPlace")
        .expect("the silkscreen layer");
    // an odd number is a top layer
    assert_eq!(silk.side, Some(Side::Top));
}

#[test]
fn a_closed_ring_of_wires_on_the_dimension_layer_is_the_outline() {
    let board = read(FILE).board;
    let corners: Vec<(i64, i64)> = board
        .outline
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect();
    assert_eq!(
        corners,
        vec![(0, 0), (mm(20.0), 0), (mm(20.0), mm(10.0)), (0, mm(10.0))]
    );
}

#[test]
fn wires_that_follow_each_other_on_a_silkscreen_layer_are_one_stroke() {
    let board = read(FILE).board;
    assert_eq!(board.silkscreen.len(), 1);
    let SilkItem::Stroke { width, path, .. } = board.silkscreen.values().next().unwrap() else {
        panic!("a stroke");
    };
    assert_eq!(width.get(), mm(0.2));
    assert_eq!(path.len(), 2);
    assert_eq!(path[0].end(), path[1].start());
}

#[test]
fn the_pads_of_a_package_take_their_shape_from_the_file() {
    let board = read(FILE).board;
    assert_eq!(board.footprints.len(), 1);
    let footprint = board.footprints.values().next().unwrap();
    assert_eq!(footprint.name, "R0603");
    assert_eq!(footprint.pads.len(), 4);
    let pad = |number: &str| {
        footprint
            .pads
            .values()
            .find(|p| p.number == number)
            .unwrap()
    };
    // a surface pad with no rounding is a rectangle, on the layer it names
    let one = pad("1");
    assert_eq!(
        one.shape,
        PadShape::Rect {
            width: eh3d_phoenix_model::Nm::new(mm(0.8)).unwrap(),
            height: eh3d_phoenix_model::Nm::new(mm(0.9)).unwrap()
        }
    );
    assert_eq!((one.at.x_nm.get(), one.at.y_nm.get()), (mm(-0.8), 0));
    assert_eq!(one.drill, None);
    assert_eq!(one.layers.len(), 1);
    // fully rounded and not square: an oblong
    assert!(matches!(pad("2").shape, PadShape::Oblong { .. }));
    // a square through-hole pad is a square with its drill, on every copper layer
    let three = pad("3");
    assert!(matches!(three.shape, PadShape::Rect { .. }));
    assert_eq!(three.drill.map(|d| d.get()), Some(mm(0.8)));
    assert_eq!(three.layers.len(), 2);
    // a shape that the model does not have is read as round
    assert!(matches!(pad("4").shape, PadShape::Circle { .. }));
}

#[test]
fn the_elements_take_their_turn_and_side_from_the_rotation_text() {
    let board = read(FILE).board;
    assert_eq!(board.components.len(), 2);
    let part = |refdes: &str| {
        board
            .components
            .values()
            .find(|c| c.refdes == refdes)
            .unwrap()
    };
    let r1 = part("R1");
    assert_eq!(r1.value, "10k");
    assert_eq!(r1.rotation.get(), 90_000_000);
    assert_eq!(r1.side, Side::Top);
    assert_eq!((r1.at.x_nm.get(), r1.at.y_nm.get()), (mm(5.0), mm(5.0)));
    // M means mirrored, which is the bottom side; the angle follows the R
    let r2 = part("R2");
    assert_eq!(r2.rotation.get(), 45_500_000);
    assert_eq!(r2.side, Side::Bottom);
    // both use the one footprint of the file
    assert_eq!(r1.footprint, r2.footprint);
}

#[test]
fn a_signal_gives_a_net_with_its_pads_tracks_vias_and_zones() {
    let board = read(FILE).board;
    assert_eq!(board.nets.len(), 1);
    let (net, gnd) = board.nets.iter().next().unwrap();
    assert_eq!(gnd.name, "GND");
    // R1 pad 1 and R2 pad 2; the reference to R9 names no element
    assert_eq!(board.pad_nets.len(), 2);
    assert!(board.pad_nets.iter().all(|p| p.net == *net));
    // the first two wires join into one track of two pieces, the third is a track of its own
    assert_eq!(board.tracks.len(), 2);
    let mut lengths: Vec<usize> = board.tracks.values().map(|t| t.path.len()).collect();
    lengths.sort_unstable();
    assert_eq!(lengths, vec![1, 2]);
    assert!(board.tracks.values().all(|t| t.width.get() == mm(0.25)));
    // the via runs from the first copper layer to the last
    assert_eq!(board.vias.len(), 1);
    let via = board.vias.values().next().unwrap();
    assert_eq!(via.from_layer, board.copper_stack[0]);
    assert_eq!(via.to_layer, board.copper_stack[1]);
    assert_eq!((via.drill.get(), via.diameter.get()), (mm(0.4), mm(0.8)));
    // the polygon is a zone on the bottom layer with the isolation as its clearance
    assert_eq!(board.zones.len(), 1);
    let zone = board.zones.values().next().unwrap();
    assert_eq!(zone.layer, board.copper_stack[1]);
    assert_eq!(zone.outline.len(), 3);
    assert_eq!(zone.clearance.get(), mm(0.3));
    assert!(board
        .tracks
        .values()
        .flat_map(|t| t.path.iter())
        .all(|s| matches!(s, Segment::Line { .. })));
}

#[test]
fn what_is_not_read_is_counted_by_name_in_order() {
    let read = read(FILE);
    let counted: Vec<String> = read
        .diagnostics
        .iter()
        .filter(|d| d.code.as_deref() == Some(NOT_READ))
        .map(|d| d.message.rsplit(": ").next().unwrap().to_string())
        .collect();
    assert_eq!(
        counted,
        vec![
            "1 contactref element(s)",
            "1 element element(s)",
            "1 layer element(s)",
            "1 rectangle element(s)",
            "1 text element(s)",
            "1 wire element(s)"
        ]
    );
    // the long pad was read as round, and the reader says so
    assert!(read
        .diagnostics
        .iter()
        .any(|d| d.code.as_deref() == Some("PHOENIX-EAGLE-020")
            && d.message.contains("pad 4: the shape long is read as round")));
}

#[test]
fn a_file_that_is_not_a_board_is_refused_with_the_code_of_unreadable() {
    for (text, why) in [
        ("this is not xml", "malformed"),
        ("<other/>", "the root element is not eagle"),
        (
            "<eagle><drawing/></eagle>",
            "there is no board in the drawing",
        ),
        ("<eagle/>", "there is no board in the drawing"),
    ] {
        let error = read_board(text, &mut ids()).unwrap_err();
        assert_eq!(error.code.as_deref(), Some("PHOENIX-EAGLE-023"), "{text}");
        assert!(error.message.contains(why), "{text}: {}", error.message);
    }
}

#[test]
fn an_outline_that_is_not_a_closed_ring_is_left_out() {
    let open = FILE.replace(
        r#"<wire x1="0" y1="10" x2="0" y2="0" width="0.1" layer="20"/>"#,
        "",
    );
    let read = read(&open);
    assert!(read.board.outline.is_empty());
    // the three wires that did not close are counted with the other wires that were not read
    assert!(read
        .diagnostics
        .iter()
        .any(|d| d.message.ends_with("4 wire element(s)")));
}
