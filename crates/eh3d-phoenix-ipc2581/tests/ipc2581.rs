// SPDX-License-Identifier: GPL-3.0-or-later
//! The IPC-2581 file of a board written by the test itself, read back with the XML reader of the
//! repository: the file is well formed, it gives the same tree again, and its geometry is the one
//! of the board.

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    Track, Via, Zone,
};
use eh3d_phoenix_eagle::xml::{parse, Element, Limits};
use eh3d_phoenix_ipc2581::codes::{LEFT_OUT, LIMIT_EXCEEDED, NO_OUTLINE};
use eh3d_phoenix_ipc2581::{export, Ipc2581, Ipc2581Settings};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

const MM: i64 = 1_000_000;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn layer(name: &str, kind: LayerKind, side: Option<Side>) -> Layer {
    Layer {
        name: name.into(),
        kind,
        side,
    }
}

fn pad(
    number: &str,
    shape: PadShape,
    at: Point,
    drill: Option<i64>,
    plated: bool,
    layers: &[Id],
) -> Pad {
    Pad {
        number: number.into(),
        shape,
        at,
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers: layers.to_vec(),
        roundness: 0,
        plated,
    }
}

fn place(
    b: &mut Board,
    n: u32,
    (refdes, value): (&str, &str),
    fp: u32,
    (x, y): (i64, i64),
    side: Side,
) {
    b.components.insert(
        id(n),
        Component {
            refdes: refdes.into(),
            value: value.into(),
            symbol: None,
            footprint: id(fp),
            at: pt(x * MM, y * MM),
            rotation: Udeg::ZERO,
            side,
        },
    );
}

/// A 20 x 10 mm board with a 2 x 2 mm cutout, the layers Top, Bottom, Silk and Notes, two
/// resistors (one on each side), a connector with a plated pad and a mounting hole, a track with
/// a quarter arc, a via and a zone with a cutout.
fn board() -> Board {
    let (top, bottom, silk, notes) = (id(1), id(2), id(3), id(4));
    let mut b = Board {
        name: "demo".into(),
        outline: vec![
            pt(0, 0),
            pt(20 * MM, 0),
            pt(20 * MM, 10 * MM),
            pt(0, 10 * MM),
        ],
        outline_cutouts: vec![vec![
            pt(2 * MM, 2 * MM),
            pt(4 * MM, 2 * MM),
            pt(4 * MM, 4 * MM),
            pt(2 * MM, 4 * MM),
        ]],
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    b.layers.insert(top, layer("Top", LayerKind::Copper, None));
    b.layers
        .insert(bottom, layer("Bottom", LayerKind::Copper, None));
    b.layers
        .insert(silk, layer("Silk", LayerKind::Silkscreen, Some(Side::Top)));
    b.layers
        .insert(notes, layer("Notes", LayerKind::Documentation, None));
    b.nets.insert(id(5), Net { name: "GND".into() });
    b.nets.insert(id(6), Net { name: "SIG".into() });
    let rect = PadShape::Rect {
        width: nm(800_000),
        height: nm(600_000),
    };
    let mut resistor = Footprint {
        name: "R_0603".into(),
        pads: Default::default(),
    };
    resistor
        .pads
        .insert(id(11), pad("1", rect, pt(-800_000, 0), None, true, &[top]));
    resistor
        .pads
        .insert(id(12), pad("2", rect, pt(800_000, 0), None, true, &[top]));
    b.footprints.insert(id(10), resistor);
    let mut header = Footprint {
        name: "HDR".into(),
        pads: Default::default(),
    };
    let round = PadShape::Circle {
        diameter: nm(1_600_000),
    };
    header.pads.insert(
        id(14),
        pad("1", round, pt(0, 0), Some(900_000), true, &[top, bottom]),
    );
    let hole = PadShape::Circle {
        diameter: nm(3 * MM),
    };
    header.pads.insert(
        id(15),
        pad(
            "MH",
            hole,
            pt(3 * MM, 0),
            Some(3 * MM),
            false,
            &[top, bottom],
        ),
    );
    b.footprints.insert(id(13), header);
    place(&mut b, 20, ("R1", "10k"), 10, (5, 5), Side::Top);
    place(&mut b, 21, ("R2", "10k"), 10, (15, 5), Side::Bottom);
    place(&mut b, 22, ("J1", ""), 13, (10, 8), Side::Top);
    for (component, pad_id, net) in [
        (20, 11, 6),
        (20, 12, 5),
        (21, 11, 6),
        (21, 12, 5),
        (22, 14, 5),
    ] {
        b.pad_nets.push(PadNet {
            component: id(component),
            pad: id(pad_id),
            net: id(net),
        });
    }
    b.pad_nets.sort_by_key(PadNet::pad_ref);
    b.tracks.insert(
        id(30),
        Track {
            net: id(6),
            layer: top,
            width: nm(250_000),
            path: vec![
                Segment::Line {
                    start: pt(5 * MM, 5 * MM),
                    end: pt(10 * MM, 5 * MM),
                },
                Segment::Arc {
                    start: pt(10 * MM, 5 * MM),
                    end: pt(11 * MM, 6 * MM),
                    sweep: Udeg::new(90_000_000).unwrap(),
                },
            ],
        },
    );
    b.vias.insert(
        id(31),
        Via {
            net: id(6),
            at: pt(8 * MM, 8 * MM),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: top,
            to_layer: bottom,
        },
    );
    b.zones.insert(
        id(32),
        Zone {
            net: id(5),
            layer: bottom,
            outline: vec![
                pt(MM, MM),
                pt(19 * MM, MM),
                pt(19 * MM, 9 * MM),
                pt(MM, 9 * MM),
            ],
            clearance: nm(200_000),
            cutouts: vec![vec![
                pt(10 * MM, 2 * MM),
                pt(12 * MM, 2 * MM),
                pt(12 * MM, 4 * MM),
                pt(10 * MM, 4 * MM),
            ]],
        },
    );
    b
}

fn file(settings: &Ipc2581Settings) -> (Ipc2581, Element) {
    let out = export(&board(), settings).unwrap();
    let root = parse(&out.xml, &Limits::default()).expect("the file is well formed XML");
    (out, root)
}

fn names<'a>(e: &'a Element, child: &'a str) -> Vec<&'a str> {
    e.children_named(child)
        .map(|c| c.attr("name").unwrap_or(""))
        .collect()
}

fn step(root: &Element) -> &Element {
    root.child("Ecad")
        .unwrap()
        .child("CadData")
        .unwrap()
        .child("Step")
        .unwrap()
}

#[test]
fn the_file_is_well_formed_and_gives_the_same_tree_when_read_and_written_again() {
    let (out, root) = file(&Ipc2581Settings::default());
    assert!(
        out.xml.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- EH3D writer profile: phoenix-ipc2581/1:default -->\n<IPC-2581 "
        ),
        "{}",
        &out.xml[..120]
    );
    assert_eq!(root.name, "IPC-2581");
    assert_eq!(root.attr("revision"), Some("C"));
    assert_eq!(root.attr("xmlns"), Some("http://webstds.ipc.org/2581"));
    // the declaration and the comment that names the writer profile come before the root element
    let body = out.xml.splitn(3, '\n').nth(2).unwrap();
    assert_eq!(root.to_xml(), body);
    // the same board gives the same bytes
    assert_eq!(
        out.xml,
        export(&board(), &Ipc2581Settings::default()).unwrap().xml
    );
    assert_eq!(out.summary.bytes, out.xml.len());
}

#[test]
fn the_sections_come_in_the_order_of_the_standard() {
    let (_, root) = file(&Ipc2581Settings::default());
    let order: Vec<&str> = root.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        order,
        vec!["Content", "LogisticHeader", "HistoryRecord", "Bom", "Ecad"]
    );
    let history = root.child("HistoryRecord").unwrap();
    assert_eq!(history.attr("origination"), Some("1970-01-01T00:00:00"));
    assert_eq!(history.attr("software"), Some("EH3D-PHOENIX-PCB"));
}

#[test]
fn the_content_names_the_step_the_layers_and_the_shapes() {
    let (_, root) = file(&Ipc2581Settings::default());
    let content = root.child("Content").unwrap();
    assert_eq!(content.child("StepRef").unwrap().attr("name"), Some("demo"));
    // the documentation layer is not in the file
    assert_eq!(names(content, "LayerRef"), vec!["Top", "Bottom", "Silk"]);
    assert_eq!(content.child("BomRef").unwrap().attr("name"), Some("Bom"));
    let dictionary = content.child("DictionaryStandard").unwrap();
    assert_eq!(dictionary.attr("units"), Some("MILLIMETER"));
    let ids: Vec<&str> = dictionary
        .children_named("EntryStandard")
        .map(|e| e.attr("id").unwrap())
        .collect();
    assert_eq!(ids, vec!["C_0.6", "C_1.6", "C_3", "R_0.8x0.6"]);
    let circle = dictionary.children[1].child("Circle").unwrap();
    assert_eq!(circle.attr("diameter"), Some("1.6"));
}

#[test]
fn the_layers_have_their_function_and_side() {
    let (_, root) = file(&Ipc2581Settings::default());
    let cad = root.child("Ecad").unwrap().child("CadData").unwrap();
    let rows: Vec<(&str, &str, &str)> = cad
        .children_named("Layer")
        .map(|l| {
            (
                l.attr("name").unwrap(),
                l.attr("layerFunction").unwrap(),
                l.attr("side").unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("Top", "CONDUCTOR", "TOP"),
            ("Bottom", "CONDUCTOR", "BOTTOM"),
            ("Silk", "SILKSCREEN", "TOP"),
        ]
    );
}

#[test]
fn the_profile_is_the_outline_and_its_cutouts() {
    let (_, root) = file(&Ipc2581Settings::default());
    let profile = step(&root).child("Profile").unwrap();
    let polygon = profile.child("Polygon").unwrap();
    let corners: Vec<(&str, &str)> = polygon
        .children
        .iter()
        .map(|c| (c.attr("x").unwrap(), c.attr("y").unwrap()))
        .collect();
    // four corners, then the step back to the first
    assert_eq!(
        corners,
        vec![
            ("0", "0"),
            ("20", "0"),
            ("20", "10"),
            ("0", "10"),
            ("0", "0")
        ]
    );
    let cutout = profile.child("Cutout").unwrap();
    assert_eq!(cutout.children.len(), 5);
    assert_eq!(cutout.children[0].name, "PolyBegin");
    assert_eq!(cutout.children[0].attr("x"), Some("2"));
}

#[test]
fn the_padstacks_hold_the_shape_the_drill_the_plating_and_the_layers() {
    let (out, root) = file(&Ipc2581Settings::default());
    let stacks: Vec<&Element> = step(&root).children_named("PadStack").collect();
    assert_eq!(stacks.len(), 4);
    assert_eq!(out.summary.padstacks, 4);
    // by shape: the via (0.6), the header pad (1.6), the mounting hole (3) and the resistor pad
    let via = stacks[0];
    assert_eq!(
        via.child("PadstackHoleDef").unwrap().attr("diameter"),
        Some("0.3")
    );
    assert_eq!(via.children_named("PadstackPadDef").count(), 2);
    let header = stacks[1];
    let hole = header.child("PadstackHoleDef").unwrap();
    assert_eq!(
        (hole.attr("diameter"), hole.attr("platingStatus")),
        (Some("0.9"), Some("PLATED"))
    );
    let layers: Vec<&str> = header
        .children_named("PadstackPadDef")
        .map(|d| d.attr("layerRef").unwrap())
        .collect();
    assert_eq!(layers, vec!["Top", "Bottom"]);
    let mount = stacks[2];
    assert_eq!(
        mount
            .child("PadstackHoleDef")
            .unwrap()
            .attr("platingStatus"),
        Some("NONPLATED")
    );
    assert_eq!(mount.children_named("PadstackPadDef").count(), 0);
    let smd = stacks[3];
    assert!(smd.child("PadstackHoleDef").is_none());
    let pad_def = smd.child("PadstackPadDef").unwrap();
    assert_eq!(
        pad_def.child("StandardPrimitiveRef").unwrap().attr("id"),
        Some("R_0.8x0.6")
    );
}

#[test]
fn the_packages_and_the_placed_parts_are_written() {
    let (_, root) = file(&Ipc2581Settings::default());
    let step = step(&root);
    assert_eq!(names(step, "Package"), vec!["HDR", "R_0603"]);
    let resistor = step.children_named("Package").nth(1).unwrap();
    let pins: Vec<(&str, &str)> = resistor
        .children_named("Pin")
        .map(|p| {
            (
                p.attr("number").unwrap(),
                p.child("Location").unwrap().attr("x").unwrap(),
            )
        })
        .collect();
    assert_eq!(pins, vec![("1", "-0.8"), ("2", "0.8")]);
    // the box of the pads: 0.8 + 0.4 to each side, 0.3 up and down
    let outline = resistor.child("Outline").unwrap().child("Polygon").unwrap();
    assert_eq!(outline.children[0].attr("x"), Some("-1.2"));
    assert_eq!(outline.children[0].attr("y"), Some("-0.3"));
    let parts: Vec<(&str, &str, &str, &str)> = step
        .children_named("Component")
        .map(|c| {
            (
                c.attr("refDes").unwrap(),
                c.attr("layerRef").unwrap(),
                c.attr("mountType").unwrap(),
                c.child("Xform").unwrap().attr("mirror").unwrap(),
            )
        })
        .collect();
    assert_eq!(
        parts,
        vec![
            ("J1", "Top", "THMT", "false"),
            ("R1", "Top", "SMT", "false"),
            ("R2", "Bottom", "SMT", "true"),
        ]
    );
    let r1 = step.children_named("Component").nth(1).unwrap();
    assert_eq!(r1.child("Location").unwrap().attr("x"), Some("5"));
    assert_eq!(r1.child("Location").unwrap().attr("y"), Some("5"));
}

#[test]
fn the_nets_list_their_pins() {
    let (_, root) = file(&Ipc2581Settings::default());
    let nets: Vec<(&str, Vec<String>)> = step(&root)
        .children_named("LogicalNet")
        .map(|n| {
            (
                n.attr("name").unwrap(),
                n.children_named("PinRef")
                    .map(|p| {
                        format!(
                            "{}:{}",
                            p.attr("componentRef").unwrap(),
                            p.attr("pin").unwrap()
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    assert_eq!(nets.len(), 2);
    assert_eq!(
        nets[0],
        (
            "GND",
            vec!["J1:1".to_owned(), "R1:2".to_owned(), "R2:2".to_owned()]
        )
    );
    assert_eq!(nets[1], ("SIG", vec!["R1:1".to_owned(), "R2:1".to_owned()]));
}

#[test]
fn the_copper_layers_hold_pads_vias_tracks_and_zones_grouped_by_net() {
    let (out, root) = file(&Ipc2581Settings::default());
    let features: Vec<&Element> = step(&root).children_named("LayerFeature").collect();
    assert_eq!(features.len(), 2);
    let top = features[0];
    assert_eq!(top.attr("layerRef"), Some("Top"));
    let sets: Vec<&str> = top
        .children_named("Set")
        .map(|s| s.attr("net").unwrap())
        .collect();
    assert_eq!(sets, vec!["GND", "SIG"]);
    let sig = top.children_named("Set").nth(1).unwrap();
    // R1:1 and R2:1, the via, and the track
    assert_eq!(sig.children_named("Pad").count(), 3);
    let line = sig.child("Features").unwrap().child("Polyline").unwrap();
    let steps: Vec<&str> = line.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        steps,
        vec!["PolyBegin", "PolyStepSegment", "PolyStepCurve", "LineDesc"]
    );
    assert_eq!(line.children[0].attr("x"), Some("5"));
    assert_eq!(line.children[1].attr("x"), Some("10"));
    let curve = &line.children[2];
    assert_eq!((curve.attr("x"), curve.attr("y")), (Some("11"), Some("6")));
    assert_eq!(
        (curve.attr("centerX"), curve.attr("centerY")),
        (Some("10"), Some("6"))
    );
    assert_eq!(curve.attr("clockwise"), Some("false"));
    assert_eq!(line.children[3].attr("lineWidth"), Some("0.25"));
    // a pad has its place and its pin
    let gnd = top.children_named("Set").next().unwrap();
    let j1 = gnd
        .children_named("Pad")
        .find(|p| p.child("PinRef").unwrap().attr("componentRef") == Some("J1"))
        .unwrap();
    assert_eq!(j1.child("Location").unwrap().attr("x"), Some("10"));
    assert_eq!(j1.child("Location").unwrap().attr("y"), Some("8"));
    assert_eq!(
        j1.child("StandardPrimitiveRef").unwrap().attr("id"),
        Some("C_1.6")
    );
    // the bottom layer: the connector pad, the via and the zone
    let bottom = features[1];
    assert_eq!(bottom.attr("layerRef"), Some("Bottom"));
    let gnd = bottom.children_named("Set").next().unwrap();
    let contour = gnd.child("Features").unwrap().child("Contour").unwrap();
    assert_eq!(contour.child("Polygon").unwrap().children.len(), 5);
    assert_eq!(contour.child("Cutout").unwrap().children.len(), 5);
    let s = &out.summary;
    assert_eq!(
        (s.layers, s.copper_layers, s.components, s.packages),
        (3, 2, 3, 2)
    );
    assert_eq!((s.pads, s.vias, s.tracks, s.zones, s.nets), (6, 1, 1, 1, 2));
}

#[test]
fn the_bill_of_materials_groups_the_parts_by_value_and_footprint() {
    let (out, root) = file(&Ipc2581Settings::default());
    let bom = root.child("Bom").unwrap();
    assert_eq!(
        bom.child("BomHeader").unwrap().attr("assembly"),
        Some("demo")
    );
    let items: Vec<(&str, &str, &str)> = bom
        .children_named("BomItem")
        .map(|i| {
            (
                i.attr("OEMDesignNumberRef").unwrap(),
                i.attr("quantity").unwrap(),
                i.attr("pinCount").unwrap(),
            )
        })
        .collect();
    assert_eq!(items, vec![("HDR", "1", "2"), ("10k_R_0603", "2", "2")]);
    let resistors = bom.children_named("BomItem").nth(1).unwrap();
    let refs: Vec<(&str, &str)> = resistors
        .children_named("RefDes")
        .map(|r| (r.attr("name").unwrap(), r.attr("layerRef").unwrap()))
        .collect();
    assert_eq!(refs, vec![("R1", "Top"), ("R2", "Bottom")]);
    assert_eq!(out.summary.bom_lines, 2);
}

#[test]
fn what_is_left_out_is_said() {
    let (out, _) = file(&Ipc2581Settings::default());
    let f = out.findings.iter().find(|f| f.code == LEFT_OUT).unwrap();
    for part in [
        "1 holes that are not plated",
        "the copper fill of the zones",
        "the documentation layers",
        "the stack-up",
    ] {
        assert!(f.detail.contains(part), "{}", f.detail);
    }
}

#[test]
fn the_settings_switch_parts_off() {
    let settings = Ipc2581Settings {
        include_bom: false,
        include_tracks: false,
        include_zones: false,
        origination: "2001-02-03T04:05:06".to_owned(),
        ..Ipc2581Settings::default()
    };
    let (out, root) = file(&settings);
    assert!(root.child("Bom").is_none());
    assert!(root.child("Content").unwrap().child("BomRef").is_none());
    let xml = &out.xml;
    assert!(!xml.contains("<Polyline"), "no tracks");
    assert!(!xml.contains("<Contour"), "no zones");
    assert!(xml.contains("origination=\"2001-02-03T04:05:06\""));
    assert_eq!(
        (out.summary.tracks, out.summary.zones, out.summary.bom_lines),
        (0, 0, 0)
    );
    let f = out.findings.iter().find(|f| f.code == LEFT_OUT).unwrap();
    assert!(
        f.detail.contains("the zones") && f.detail.contains("the tracks"),
        "{}",
        f.detail
    );
}

#[test]
fn a_board_without_an_outline_has_no_profile_and_says_so() {
    let mut b = board();
    b.outline.clear();
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    assert!(step(&root).child("Profile").is_none());
    assert!(out.findings.iter().any(|f| f.code == NO_OUTLINE));
}

#[test]
fn a_board_with_too_many_objects_is_refused() {
    let settings = Ipc2581Settings {
        max_items: 4,
        ..Ipc2581Settings::default()
    };
    let e = export(&board(), &settings).unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
}

#[test]
fn names_with_marks_of_xml_are_escaped_and_read_back_the_same() {
    let mut b = board();
    b.name = "a&b <\"c\">".into();
    b.nets.insert(
        id(7),
        Net {
            name: "N<1>&\"x\"".into(),
        },
    );
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    assert_eq!(step(&root).attr("name"), Some("a&b <\"c\">"));
    assert!(names(step(&root), "LogicalNet").contains(&"N<1>&\"x\""));
}

#[test]
fn a_board_without_the_order_of_its_layers_still_gives_a_file() {
    let mut b = board();
    b.copper_stack.clear();
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    assert_eq!(out.summary.copper_layers, 2);
    // the via joins the two layers it starts and ends on
    assert_eq!(step(&root).children_named("LayerFeature").count(), 2);
}

#[test]
fn the_texts_of_the_dimensions_are_written_with_their_height_and_their_turn() {
    use eh3d_phoenix_board::{Dimension, DimensionKind};
    let mut b = board();
    let notes = *b
        .layers
        .iter()
        .find(|(_, l)| l.name == "Notes")
        .map(|(id, _)| id)
        .unwrap();
    let mut add = |n: u32, kind, to| {
        b.dimensions.insert(
            id(n),
            Dimension {
                layer: notes,
                kind,
                from: pt(0, 0),
                to,
                third: None,
                offset: nm(2 * MM),
                text_height: nm(MM),
                stroke: nm(100_000),
            },
        );
    };
    add(900, DimensionKind::Horizontal, pt(10 * MM, 0));
    add(901, DimensionKind::Vertical, pt(0, 8 * MM));
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    assert_eq!(out.summary.dimension_texts, 2);
    let root = parse(&out.xml, &Limits::default()).unwrap();
    let feature = step(&root)
        .children_named("LayerFeature")
        .find(|f| f.attr("layerRef") == Some("Notes"))
        .unwrap();
    let texts: Vec<&_> = feature
        .child("Set")
        .unwrap()
        .children_named("Features")
        .filter(|f| f.child("UserSpecial").is_some())
        .collect();
    assert_eq!(texts.len(), 2);
    let described: Vec<(Option<&str>, Option<&str>, bool)> = texts
        .iter()
        .map(|f| {
            let text = f.child("UserSpecial").unwrap().child("Text").unwrap();
            (
                text.attr("text"),
                text.attr("textHeight"),
                f.child("Xform").is_some(),
            )
        })
        .collect();
    // the horizontal one is read level, the vertical one runs upwards and carries its turn
    assert_eq!(
        described,
        vec![(Some("10"), Some("1"), false), (Some("8"), Some("1"), true)]
    );
    let turned = texts[1].child("Xform").unwrap().attr("rotation");
    assert_eq!(turned, Some("90"));
    // nothing about the texts is left out
    let f = out.findings.iter().find(|f| f.code == LEFT_OUT).unwrap();
    assert!(!f.detail.contains("texts of dimensions"), "{}", f.detail);
    // the height of the settings replaces the height of every dimension
    let tall = Ipc2581Settings {
        dimension_text_height_nm: 2_500_000,
        ..Ipc2581Settings::default()
    };
    let out = export(&b, &tall).unwrap();
    assert!(out.xml.contains("textHeight=\"2.5\""));
    assert!(!out.xml.contains("textHeight=\"1\""));
}

#[test]
fn the_lines_of_the_dimensions_are_written_on_a_document_layer_and_their_texts_when_asked() {
    use eh3d_phoenix_board::{Dimension, DimensionKind};
    let mut b = board();
    let notes = *b
        .layers
        .iter()
        .find(|(_, l)| l.name == "Notes")
        .map(|(id, _)| id)
        .unwrap();
    let mut add = |n: u32, kind, from, to, third, offset: i64| {
        b.dimensions.insert(
            id(n),
            Dimension {
                layer: notes,
                kind,
                from,
                to,
                third,
                offset: nm(offset),
                text_height: nm(MM),
                stroke: nm(100_000),
            },
        );
    };
    add(
        900,
        DimensionKind::Horizontal,
        pt(0, 0),
        pt(10 * MM, 0),
        None,
        2 * MM,
    );
    add(
        901,
        DimensionKind::Angle,
        pt(0, 0),
        pt(10 * MM, 0),
        Some(pt(0, 10 * MM)),
        5 * MM,
    );
    // one that cannot be drawn
    add(
        902,
        DimensionKind::Aligned,
        pt(MM, MM),
        pt(MM, MM),
        None,
        MM,
    );
    let no_texts = Ipc2581Settings {
        include_dimension_texts: false,
        ..Ipc2581Settings::default()
    };
    let out = export(&b, &no_texts).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    // the layer is declared with the function DOCUMENT and holds one set of lines
    let cad = root.child("Ecad").unwrap().child("CadData").unwrap();
    let declared = cad
        .children_named("Layer")
        .find(|l| l.attr("name") == Some("Notes"))
        .unwrap();
    assert_eq!(declared.attr("layerFunction"), Some("DOCUMENT"));
    let feature = step(&root)
        .children_named("LayerFeature")
        .find(|f| f.attr("layerRef") == Some("Notes"))
        .unwrap();
    // 7 lines of the horizontal one and 22 of the angle: every line is a polyline of one piece
    let set = feature.child("Set").unwrap();
    assert_eq!(set.children_named("Features").count(), 7 + 22);
    let width = set
        .child("Features")
        .unwrap()
        .child("Polyline")
        .unwrap()
        .child("LineDesc")
        .unwrap()
        .attr("lineWidth");
    assert_eq!(width, Some("0.1"));
    assert_eq!(out.summary.dimensions, 2);
    assert_eq!(out.summary.dimension_texts, 0);
    assert!(!out.xml.contains("<Text "));
    // what is left out: the two texts and the dimension that cannot be drawn, no longer the layer
    let f = out.findings.iter().find(|f| f.code == LEFT_OUT).unwrap();
    assert!(f.detail.contains("2 texts of dimensions"), "{}", f.detail);
    assert!(
        f.detail.contains("1 dimensions that cannot be drawn"),
        "{}",
        f.detail
    );
    assert!(
        !f.detail.contains("the documentation layers"),
        "{}",
        f.detail
    );
    // the same board gives the same bytes, and the setting switches them off
    assert_eq!(out.xml, export(&b, &no_texts).unwrap().xml);
    let off = Ipc2581Settings {
        include_dimensions: false,
        ..Ipc2581Settings::default()
    };
    let out = export(&b, &off).unwrap();
    assert_eq!(out.summary.dimensions, 0);
    assert!(!out.xml.contains("DOCUMENT"));
    let f = out.findings.iter().find(|f| f.code == LEFT_OUT).unwrap();
    assert!(
        f.detail.contains("the documentation layers"),
        "{}",
        f.detail
    );
}

#[test]
fn the_file_names_its_writer_profile_once_and_the_same_settings_give_the_same_bytes() {
    let (out, _) = file(&Ipc2581Settings::default());
    assert_eq!(eh3d_phoenix_model::writer_profile::count(&out.xml), 1);
    assert_eq!(
        eh3d_phoenix_model::writer_profile::find(&out.xml).as_deref(),
        Some("phoenix-ipc2581/1:default")
    );
    let other = Ipc2581Settings {
        profile: "my-maker_2.0".to_owned(),
        ..Ipc2581Settings::default()
    };
    let (a, _) = file(&other);
    let (b, _) = file(&other);
    assert_eq!(a.xml, b.xml);
    assert_eq!(
        eh3d_phoenix_model::writer_profile::find(&a.xml).as_deref(),
        Some("phoenix-ipc2581/1:my-maker_2.0")
    );
    // nothing but the comment differs from the file with the default profile
    let strip = |text: &str| -> String {
        text.split_inclusive('\n')
            .filter(|l| !l.contains(eh3d_phoenix_model::writer_profile::LABEL))
            .collect()
    };
    assert_eq!(strip(&a.xml), strip(&out.xml));
}

#[test]
fn a_profile_name_that_a_comment_cannot_hold_is_refused_with_the_settings_code() {
    for name in ["", "a b", "a--b", "a*b", "é", &"a".repeat(65)] {
        let settings = Ipc2581Settings {
            profile: name.to_owned(),
            ..Ipc2581Settings::default()
        };
        let error = export(&board(), &settings).unwrap_err();
        assert_eq!(
            error.code,
            eh3d_phoenix_ipc2581::codes::SETTINGS_INVALID,
            "{name:?}"
        );
    }
    let from_text = Ipc2581Settings::parse("profile = my-maker\n").unwrap();
    assert_eq!(from_text.profile, "my-maker");
    assert!(Ipc2581Settings::parse("profile = a b\n").is_err());
}

#[test]
fn an_arc_of_the_outline_is_a_curve_of_the_profile() {
    use eh3d_phoenix_board::OutlineArc;
    let mut b = board();
    // the right side of the board bulges out in a half circle of radius 5 mm about (20, 5)
    b.outline_arcs = vec![OutlineArc {
        edge: 1,
        sweep: Udeg::new(180_000_000).unwrap(),
    }];
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    let polygon = step(&root)
        .child("Profile")
        .unwrap()
        .child("Polygon")
        .unwrap();
    let curves: Vec<&Element> = polygon.children_named("PolyStepCurve").collect();
    assert_eq!(curves.len(), 1);
    for (name, value) in [
        ("x", "20"),
        ("y", "10"),
        ("centerX", "20"),
        ("centerY", "5"),
        ("clockwise", "false"),
    ] {
        assert_eq!(curves[0].attr(name), Some(value), "{name}");
    }
    assert_eq!(polygon.children_named("PolyStepSegment").count(), 3);
    let f = out.findings.iter().find(|f| f.code == LEFT_OUT).unwrap();
    assert!(!f.detail.contains("arcs of the outline"), "{}", f.detail);
}

#[test]
fn footprints_of_one_name_with_other_pads_are_other_packages() {
    let mut b = board();
    // a second R_0603 with its pads further apart, used by R2
    let mut wide = b.footprints[&id(10)].clone();
    wide.pads.get_mut(&id(11)).unwrap().at = pt(-MM, 0);
    wide.pads.get_mut(&id(11)).unwrap().rotation = Udeg::new(30_000_000).unwrap();
    b.footprints.insert(id(16), wide);
    // and a third one like the first, used by R3, which shares its package
    let same = b.footprints[&id(10)].clone();
    b.footprints.insert(id(17), same);
    b.components.get_mut(&id(21)).unwrap().footprint = id(16);
    place(&mut b, 23, ("R3", "10k"), 17, (5, 2), Side::Top);
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    let step = step(&root);
    assert_eq!(names(step, "Package"), vec!["HDR", "R_0603", "R_0603_2"]);
    let package_of = |refdes: &str| {
        step.children_named("Component")
            .find(|c| c.attr("refDes") == Some(refdes))
            .and_then(|c| c.attr("packageRef"))
            .unwrap()
            .to_owned()
    };
    assert_eq!(package_of("R1"), "R_0603");
    assert_eq!(package_of("R2"), "R_0603_2");
    assert_eq!(package_of("R3"), "R_0603");
    // the pin of the second package has its place and its turn
    let package = step
        .children_named("Package")
        .find(|p| p.attr("name") == Some("R_0603_2"))
        .unwrap();
    let pin = package
        .children_named("Pin")
        .find(|p| p.attr("number") == Some("1"))
        .unwrap();
    assert_eq!(pin.child("Location").unwrap().attr("x"), Some("-1"));
    assert_eq!(pin.child("Xform").unwrap().attr("rotation"), Some("30"));
}

#[test]
fn layers_of_one_name_are_told_apart() {
    let mut b = board();
    for l in b.layers.values_mut() {
        if l.name == "Bottom" {
            l.name = "Top".into();
        }
    }
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    let cad = root.child("Ecad").unwrap().child("CadData").unwrap();
    let layers = names(cad, "Layer");
    assert_eq!(&layers[..2], ["Top", "Top_2"]);
    let features: Vec<&str> = step(&root)
        .children_named("LayerFeature")
        .filter_map(|f| f.attr("layerRef"))
        .collect();
    assert_eq!(features, ["Top", "Top_2"]);
}

#[test]
fn the_turn_of_a_bottom_pad_is_taken_off_the_turn_of_its_part() {
    let mut b = board();
    b.footprints
        .get_mut(&id(10))
        .unwrap()
        .pads
        .get_mut(&id(11))
        .unwrap()
        .rotation = Udeg::new(30_000_000).unwrap();
    for c in [20, 21] {
        b.components.get_mut(&id(c)).unwrap().rotation = Udeg::new(90_000_000).unwrap();
    }
    let out = export(&b, &Ipc2581Settings::default()).unwrap();
    let root = parse(&out.xml, &Limits::default()).unwrap();
    let turn_of = |refdes: &str| {
        step(&root)
            .children_named("LayerFeature")
            .flat_map(|f| f.children_named("Set"))
            .flat_map(|s| s.children_named("Pad"))
            .find(|p| {
                p.child("PinRef").is_some_and(|r| {
                    r.attr("componentRef") == Some(refdes) && r.attr("pin") == Some("1")
                })
            })
            .and_then(|p| p.child("Xform"))
            .and_then(|x| x.attr("rotation"))
            .unwrap()
            .to_owned()
    };
    // R1 is on top, R2 on the bottom, where the file mirrors before it turns
    assert_eq!(turn_of("R1"), "120");
    assert_eq!(turn_of("R2"), "60");
}
