// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of EAGLE board files on a small file written by hand: the layers, the outline,
//! the packages with their pads, the elements with their turn and side, the signals with their
//! tracks, vias and polygons, the silkscreen, and what is counted as not read.

use eh3d_phoenix_board::{DimensionKind, Id, LayerKind, PadShape, Segment, Side, SilkItem};
use eh3d_phoenix_eagle_board::{
    read_board, read_board_with_limits, write_board, EagleLimits, ReadBoard, NOT_READ,
};
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

fn read_with(text: &str, limits: &EagleLimits) -> ReadBoard {
    read_board_with_limits(text, &mut ids(), limits).unwrap()
}

fn messages(read: &ReadBoard) -> Vec<&str> {
    read.diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect()
}

#[test]
fn rings_of_the_outline_layer_over_the_limit_are_counted_and_not_tested_for_openings() {
    // three squares of 1 mm inside the board of 20 by 10 mm, each of four wires
    let square = |x: i32, y: i32| {
        format!(
            "<wire x1=\"{x}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" width=\"0.1\" layer=\"20\"/>\
             <wire x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{}\" width=\"0.1\" layer=\"20\"/>\
             <wire x1=\"{}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\" width=\"0.1\" layer=\"20\"/>\
             <wire x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{y}\" width=\"0.1\" layer=\"20\"/>",
            x + 1,
            x + 1,
            x + 1,
            y + 1,
            x + 1,
            y + 1,
            y + 1,
            y + 1,
        )
    };
    let anchor = "<wire x1=\"0\" y1=\"10\" x2=\"0\" y2=\"0\" width=\"0.1\" layer=\"20\"/>";
    assert!(FILE.contains(anchor));
    let text = FILE.replace(
        anchor,
        &format!("{anchor}{}{}{}", square(2, 2), square(5, 2), square(8, 2)),
    );
    let all = read_with(&text, &EagleLimits::default());
    assert_eq!(all.board.outline.len(), 4);
    assert_eq!(all.board.outline_cutouts.len(), 3);
    assert!(!messages(&all)
        .iter()
        .any(|m| m.contains("max_outline_rings")));
    // the outline and one opening are kept, the other two rings are left over
    let tight = EagleLimits {
        max_outline_rings: 2,
        ..EagleLimits::default()
    };
    let r = read_with(&text, &tight);
    assert_eq!(r.board.outline.len(), 4);
    assert_eq!(r.board.outline_cutouts.len(), 1);
    assert!(messages(&r)
        .iter()
        .any(|m| m.contains("2 outline ring (over max_outline_rings) element(s)")));
}

#[test]
fn restrict_areas_over_the_check_budget_are_counted_and_not_given_to_a_pour() {
    // the pour of FILE has 3 vertices on the bottom layer; the restrict area (layer 42) has 3
    // points inside it, so it costs 3 * 3 = 9 tests
    let area = "<polygon width=\"0\" layer=\"42\"><vertex x=\"6\" y=\"2\"/><vertex x=\"8\" y=\"2\"/><vertex x=\"8\" y=\"3\"/></polygon>\n<text x=\"1\" y=\"5\"";
    let text = FILE.replace("<text x=\"1\" y=\"5\"", area);
    let cutouts = |limits: &EagleLimits| {
        let r = read_with(&text, limits);
        let n = r
            .board
            .zones
            .values()
            .next()
            .expect("the pour")
            .cutouts
            .len();
        (n, messages(&r).join("|"))
    };
    let budget = |max_restrict_checks| EagleLimits {
        max_restrict_checks,
        ..EagleLimits::default()
    };
    let (n, said) = cutouts(&EagleLimits::default());
    assert_eq!(n, 1);
    assert!(!said.contains("max_restrict_checks"));
    assert_eq!(cutouts(&budget(9)).0, 1);
    let (n, said) = cutouts(&budget(8));
    assert_eq!(n, 0);
    assert!(said.contains("1 polygon (over max_restrict_checks) element(s)"));
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

/// A board with a layer for the documentation and dimensions of every kind that is read and some
/// that is not.
const DIMENSIONS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="16" name="Bottom" color="1" fill="1" visible="yes" active="yes"/>
<layer number="47" name="Measures" color="7" fill="1" visible="yes" active="yes"/>
<layer number="51" name="tDocu" color="7" fill="1" visible="yes" active="yes"/>
</layers>
<board>
<plain>
<dimension x1="0" y1="0" x2="3" y2="4" x3="-0.8" y3="0.6" textsize="2" textratio="10" layer="47"/>
<dimension x1="1" y1="1" x2="13.5" y2="4" x3="7" y3="3" textsize="1.2" layer="51" dtype="horizontal"/>
<dimension x1="30" y1="1" x2="30" y2="9" x3="32.5" y3="5" layer="47" dtype="vertical"/>
<dimension x1="0" y1="0" x2="5" y2="0" x3="2" y3="2" layer="47" dtype="radius"/>
<dimension x1="0" y1="0" x2="5" y2="0" x3="2" y3="2" layer="1"/>
<dimension x1="4" y1="4" x2="4" y2="4" x3="2" y3="2" layer="47"/>
<dimension x1="0" y1="0" x2="5" layer="47"/>
</plain>
<libraries/>
<elements/>
<signals/>
</board>
</drawing>
</eagle>
"#;

#[test]
fn the_parallel_horizontal_and_vertical_dimensions_are_read_and_the_others_are_counted() {
    let read = read(DIMENSIONS);
    let b = &read.board;
    assert_eq!(b.dimensions.len(), 3);
    let of = |kind: DimensionKind| b.dimensions.values().find(|d| d.kind == kind).unwrap();
    // a parallel one: 3-4-5, the line through (-0.8, 0.6) is one millimetre to the left
    let aligned = of(DimensionKind::Aligned);
    assert_eq!(aligned.length_nm(), mm(5.0));
    assert_eq!(aligned.offset.get(), mm(1.0));
    assert_eq!(aligned.text_height.get(), mm(2.0));
    // the ratio of the stroke to the text is a tenth
    assert_eq!(aligned.stroke.get(), mm(0.2));
    let horizontal = of(DimensionKind::Horizontal);
    assert_eq!(horizontal.length_nm(), mm(12.5));
    assert_eq!(horizontal.offset.get(), mm(2.0));
    assert_eq!(horizontal.text_height.get(), mm(1.2));
    // the ratio left out is 8 per cent
    assert_eq!(horizontal.stroke.get(), mm(0.096));
    let vertical = of(DimensionKind::Vertical);
    assert_eq!(vertical.length_nm(), mm(8.0));
    assert_eq!(vertical.offset.get(), mm(2.5));
    // the size of text that the file leaves out is the usual one of EAGLE
    assert_eq!(vertical.text_height.get(), 1_778_000);
    // the layers 47 and 51 hold them, both for documentation
    for d in b.dimensions.values() {
        assert_eq!(b.layers[&d.layer].kind, LayerKind::Documentation);
    }
    assert_eq!(b.layers[&aligned.layer].name, "Measures");
    assert_eq!(b.layers[&horizontal.layer].name, "tDocu");
    assert!(b.validate().is_empty(), "{:?}", b.validate());
    // the radius, the one on a copper layer, the one that measures nothing and the one with a
    // point missing are counted
    assert!(
        read.diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some(NOT_READ)
                && d.message.ends_with("4 dimension element(s)")),
        "{:?}",
        read.diagnostics
    );
}

#[test]
fn the_dimensions_of_a_board_are_written_and_read_back() {
    let read_first = read(DIMENSIONS).board;
    let written = write_board(&read_first);
    assert!(written.text.contains("<dimension "), "{}", written.text);
    let again = read(&written.text).board;
    let key = |d: &eh3d_phoenix_board::Dimension| {
        (
            d.kind.as_str(),
            (d.from.x_nm.get(), d.from.y_nm.get()),
            (d.to.x_nm.get(), d.to.y_nm.get()),
            d.offset.get(),
            d.text_height.get(),
            d.stroke.get(),
        )
    };
    let mut first: Vec<_> = read_first.dimensions.values().map(key).collect();
    let mut second: Vec<_> = again.dimensions.values().map(key).collect();
    first.sort();
    second.sort();
    assert_eq!(first, second);
    assert_eq!(again.dimensions.len(), 3);
    // the same board gives the same file
    assert_eq!(written.text, write_board(&read_first).text);
}
