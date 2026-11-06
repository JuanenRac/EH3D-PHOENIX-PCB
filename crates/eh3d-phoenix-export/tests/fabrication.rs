//! The full set of manufacturing files: profiles, Gerber X2 attributes, mask and paste openings,
//! plated and not plated drill files with slots, the placement file and the bill of materials, the
//! manifest, the folder, the package and the check that reads the files back.

use std::collections::BTreeMap;

use eh3d_parser_gerber::Aperture;
use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadShape, Segment, Side, Track,
    Via, Zone,
};
use eh3d_phoenix_export::assembly::parse_csv;
use eh3d_phoenix_export::profile::{BottomRotation, FiducialScope, ObjectAttribute, X2Level};
use eh3d_phoenix_export::x2::derived_project_id;
use eh3d_phoenix_export::{
    build, codes, export_package_reproducible, export_package_with, verify, ExportOptions, FabSet,
    FileRole, Profile, ProfileSet,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

fn id(n: u16) -> Id {
    ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
}
fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}
fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}

fn pad(number: &str, shape: PadShape, at: Point, drill: Option<i64>, layers: Vec<Id>) -> Pad {
    Pad {
        number: number.into(),
        shape,
        at,
        rotation: Udeg::ZERO,
        drill: drill.map(nm),
        layers,
        roundn,
        roundness: 0,
        plated: true,
        ess: 0,
        plated: true,
    }
}

fn component(
    refdes: &str,
    value: &str,
    footprint: Id,
    at: Point,
    rot: i64,
    side: Side,
) -> Component {
    Component {
        refdes: refdes.into(),
        value: value.into(),
        symbol: None,
        footprint,
        at,
        rotation: Udeg::new(rot).unwrap(),
        side,
    }
}

/// A four-layer-name board (two copper, mask, paste, silkscreen, outline) with two footprints: "SOIC"
/// has a plated round pad, a rectangular SMD pad, a plated oblong pad and a mounting hole; "0603"
/// has one SMD pad. R1 is on top, R2 is on the bottom turned by 30 degrees, C10 and C2 are on top.
fn fixture() -> Board {
    let (top, bottom, mask, paste, silk, outline) = (id(1), id(2), id(3), id(4), id(5), id(6));
    let net = id(7);
    let mut b = Board {
        name: "Fab board".into(),
        outline: vec![
            pt(0, 0),
            pt(30_000_000, 0),
            pt(30_000_000, 20_000_000),
            pt(0, 20_000_000),
        ],
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    for (l, name, kind, side) in [
        (top, "Top copper", LayerKind::Copper, None),
        (bottom, "Bottom copper", LayerKind::Copper, None),
        (mask, "Top mask", LayerKind::SolderMask, Some(Side::Top)),
        (paste, "Top paste", LayerKind::Paste, Some(Side::Top)),
        (silk, "Top silk", LayerKind::Silkscreen, Some(Side::Top)),
        (outline, "Outline", LayerKind::Outline, None),
    ] {
        b.layers.insert(
            l,
            Layer {
                name: name.into(),
                kind,
                side,
            },
        );
    }
    b.nets.insert(net, Net { name: "N".into() });
    let (soic, small) = (id(10), id(11));
    let mut pads = BTreeMap::new();
    pads.insert(
        id(20),
        pad(
            "1",
            PadShape::Circle {
                diameter: nm(1_600_000),
            },
            pt(0, 0),
            Some(800_000),
            vec![top, bottom, mask],
        ),
    );
    pads.insert(
        id(21),
        pad(
            "2",
            PadShape::Rect {
                width: nm(1_000_000),
                height: nm(2_000_000),
            },
            pt(5_000_000, 0),
            None,
            vec![top, mask, paste],
        ),
    );
    pads.insert(
        id(22),
        pad(
            "3",
            PadShape::Oblong {
                width: nm(3_000_000),
                height: nm(1_500_000),
            },
            pt(0, 4_000_000),
            Some(1_000_000),
            vec![top, bottom, mask],
        ),
    );
    pads.insert(
        id(23),
        pad(
            "MH",
            PadShape::Circle {
                diameter: nm(3_200_000),
            },
            pt(-3_000_000, 0),
            Some(3_200_000),
            vec![mask],
        ),
    );
    b.footprints.insert(
        soic,
        Footprint {
            name: "SOIC".into(),
            pads,
        },
    );
    let mut small_pads = BTreeMap::new();
    small_pads.insert(
        id(24),
        pad(
            "1",
            PadShape::Rect {
                width: nm(800_000),
                height: nm(900_000),
            },
            pt(0, 0),
            None,
            vec![top, mask, paste],
        ),
    );
    b.footprints.insert(
        small,
        Footprint {
            name: "0603".into(),
            pads: small_pads,
        },
    );
    for (cid, c) in [
        (
            id(30),
            component("R1", "10k", soic, pt(10_000_000, 10_000_000), 0, Side::Top),
        ),
        (
            id(31),
            component(
                "R2",
                "10k",
                soic,
                pt(20_000_000, 10_000_000),
                30_000_000,
                Side::Bottom,
            ),
        ),
        (
            id(32),
            component(
                "C10",
                "100n",
                small,
                pt(5_000_000, 15_000_000),
                90_000_000,
                Side::Top,
            ),
        ),
        (
            id(33),
            component("C2", "100n", small, pt(8_000_000, 15_000_000), 0, Side::Top),
        ),
    ] {
        b.components.insert(cid, c);
    }
    b.tracks.insert(
        id(40),
        Track {
            net,
            layer: top,
            width: nm(250_000),
            path: vec![
                Segment::Line {
                    start: pt(5_000_000, 12_000_000),
                    end: pt(10_000_000, 12_000_000),
                },
                Segment::Arc {
                    start: pt(10_000_000, 12_000_000),
                    end: pt(12_000_000, 14_000_000),
                    sweep: Udeg::new(90_000_000).unwrap(),
                },
            ],
        },
    );
    for (vid, x) in [(id(41), 12_000_000), (id(42), 14_000_000)] {
        b.vias.insert(
            vid,
            Via {
                net,
                at: pt(x, 17_000_000),
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
    b
}

fn file<'a>(set: &'a FabSet, path: &str) -> &'a str {
    set.files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no file {path}"))
        .text()
}

fn codes_of(set: &FabSet) -> Vec<String> {
    set.diagnostics
        .iter()
        .filter_map(|d| d.code.as_deref().map(str::to_owned))
        .collect()
}

#[test]
fn the_layer_files_carry_the_x2_attributes() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    let version = env!("CARGO_PKG_VERSION");
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(top.contains(&format!(
        "%TF.GenerationSoftware,EH3D,PHOENIX-PCB,{version}*%"
    )));
    assert!(top.contains("%TF.FileFunction,Copper,L1,Top*%"));
    assert!(top.contains("%TF.FilePolarity,Positive*%"));
    assert!(file(&set, "layers/01_bottom_copper.gbr").contains("%TF.FileFunction,Copper,L2,Bot*%"));
    let mask = file(&set, "layers/03_top_mask.gbr");
    assert!(mask.contains("%TF.FileFunction,Soldermask,Top*%"));
    assert!(mask.contains("%TF.FilePolarity,Negative*%"));
    let paste = file(&set, "layers/04_top_paste.gbr");
    assert!(paste.contains("%TF.FileFunction,Paste,Top*%"));
    assert!(paste.contains("%TF.FilePolarity,Positive*%"));
    assert!(file(&set, "layers/05_top_silk.gbr").contains("%TF.FileFunction,Legend,Top*%"));
    let outline = file(&set, "layers/06_outline.gbr");
    assert!(outline.contains("%TF.FileFunction,Profile,NP*%"));
    assert!(!outline.contains("FilePolarity"));
    // the attributes are read back as file attributes
    let read = eh3d_parser_gerber::parse(top);
    let names: Vec<&str> = read.attributes.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        vec![".GenerationSoftware", ".FileFunction", ".FilePolarity"]
    );
}

#[test]
fn mask_openings_grow_and_paste_openings_shrink_by_the_profile() {
    let f = fixture();
    let mut profile = Profile::default();
    profile.gerber.mask_expansion_nm = 50_000;
    profile.gerber.paste_shrink_nm = 100_000;
    let set = build(&f, &profile).unwrap();
    let has_rect = |text: &str, w: f64, h: f64| {
        let read = eh3d_parser_gerber::parse(text);
        assert!(read.is_clean(), "{:?}", read.diagnostics);
        read.apertures.values().any(|a| {
            matches!(a, Aperture::Rectangle { x, y, .. }
                if (x - w).abs() < 1e-9 && (y - h).abs() < 1e-9)
        })
    };
    // the 1.0 x 2.0 mm pad: 0.05 mm more per side in the mask, 0.1 mm less per side in the paste
    assert!(has_rect(file(&set, "layers/03_top_mask.gbr"), 1.1, 2.1));
    assert!(has_rect(file(&set, "layers/04_top_paste.gbr"), 0.8, 1.8));
    assert!(has_rect(file(&set, "layers/02_top_copper.gbr"), 1.0, 2.0));
}

#[test]
fn the_decimals_of_the_profile_change_the_coordinates() {
    let f = fixture();
    let mut profile = Profile::default();
    profile.gerber.decimal_digits = 3;
    let set = build(&f, &profile).unwrap();
    let text = file(&set, "layers/02_top_copper.gbr");
    assert!(text.contains("%FSLAX43Y43*%"));
    assert!(text.contains("X10000Y12000D01*"), "{text}");
    assert!(verify(&f, &profile, &set).is_empty());
}

#[test]
fn drills_go_to_a_plated_and_a_not_plated_file() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    // plated: 2 vias, and per component with the SOIC footprint a round and an oblong pad
    let plated = eh3d_parser_excellon::parse(file(&set, "layers/drill_pth.drl"));
    assert!(plated.is_clean(), "{:?}", plated.diagnostics);
    assert_eq!(plated.drills.len(), 2 + 2 * 2);
    assert!(plated.slots.is_empty());
    let diameters: Vec<f64> = plated.tools.values().map(|t| t.diameter).collect();
    assert_eq!(diameters.len(), 3);
    assert!(diameters.windows(2).all(|w| w[0] < w[1]), "{diameters:?}");
    assert!((diameters[0] - 0.3).abs() < 1e-9);
    // not plated: the mounting hole of the two components
    let not_plated = eh3d_parser_excellon::parse(file(&set, "layers/drill_npth.drl"));
    assert_eq!(not_plated.drills.len(), 2);
    assert!((not_plated.tools[&1].diameter - 3.2).abs() < 1e-9);
    let text = file(&set, "layers/drill_pth.drl");
    assert!(text.contains("TF.FileFunction,Plated,1,2,PTH"));
    assert!(text.contains("TF.GenerationSoftware,EH3D,PHOENIX-PCB,"));
    assert!(text.contains("METRIC"));
    assert!(file(&set, "layers/drill_npth.drl").contains("TF.FileFunction,NonPlated,1,2,NPTH"));
    assert!(codes_of(&set).contains(&codes::SLOT_AS_HOLE.to_string()));
}

#[test]
fn an_oblong_pad_is_a_slot_when_the_profile_asks_for_it() {
    let f = fixture();
    let mut profile = Profile::default();
    profile.drill.oblong_as_slot = true;
    let set = build(&f, &profile).unwrap();
    let text = file(&set, "layers/drill_pth.drl");
    assert!(text.contains("G85"));
    let plated = eh3d_parser_excellon::parse(text);
    assert!(plated.is_clean(), "{:?}", plated.diagnostics);
    // R1 is on a right angle and gives a slot; R2 is turned by 30 degrees and stays a round hole
    assert_eq!(plated.slots.len(), 1);
    assert_eq!(plated.drills.len(), 2 + 2 + 1);
    let slot = plated.slots[0];
    // the pad is 3.0 x 1.5 mm with a 1.0 mm drill: a slot 2.0 mm long, along X, at (10, 14)
    assert!((slot.from.y - 14.0).abs() < 1e-9 && (slot.to.y - 14.0).abs() < 1e-9);
    assert!((slot.to.x - slot.from.x - 1.0).abs() < 1e-9);
    assert!(codes_of(&set).contains(&codes::SLOT_ROTATION.to_string()));
    assert!(verify(&f, &profile, &set).is_empty());
}

#[test]
fn the_fixed_style_declares_its_format() {
    let f = fixture();
    let mut profile = Profile::default();
    profile.drill.style = eh3d_phoenix_export::profile::DrillStyle::Fixed;
    let set = build(&f, &profile).unwrap();
    let text = file(&set, "layers/drill_pth.drl");
    assert!(text.contains("FILE_FORMAT=3:3"), "{text}");
    assert!(verify(&f, &profile, &set).is_empty());
}

fn placement_rows(set: &FabSet, path: &str) -> Vec<Vec<String>> {
    parse_csv(file(set, path), ',')
}

#[test]
fn the_placement_file_orders_the_parts_and_applies_the_bottom_rule() {
    let f = fixture();
    let rows_for = |rule: BottomRotation| {
        let mut profile = Profile::default();
        profile.placement.bottom_rotation = rule;
        let set = build(&f, &profile).unwrap();
        placement_rows(&set, "layers/placement.csv")
    };
    let keep = rows_for(BottomRotation::Keep);
    assert_eq!(
        keep[0],
        vec!["Ref", "Value", "Package", "X_mm", "Y_mm", "Rotation", "Side"]
    );
    let refs: Vec<&str> = keep.iter().skip(1).map(|r| r[0].as_str()).collect();
    assert_eq!(refs, vec!["C2", "C10", "R1", "R2"]);
    assert_eq!(
        keep[1],
        vec!["C2", "100n", "0603", "8.0000", "15.0000", "0.00", "top"]
    );
    assert_eq!(keep[2][5], "90.00");
    let r2 = |rows: &[Vec<String>]| rows[4].clone();
    assert_eq!(
        r2(&keep),
        vec!["R2", "10k", "SOIC", "20.0000", "10.0000", "30.00", "bottom"]
    );
    assert_eq!(r2(&rows_for(BottomRotation::Mirror))[5], "330.00");
    assert_eq!(r2(&rows_for(BottomRotation::HalfTurn))[5], "210.00");
    // the top parts do not change with the rule
    assert_eq!(rows_for(BottomRotation::HalfTurn)[3][5], "0.00");
}

#[test]
fn the_origin_of_the_placement_can_be_the_corner_of_the_outline() {
    let mut f = fixture();
    f.outline = vec![
        pt(1_000_000, 2_000_000),
        pt(30_000_000, 2_000_000),
        pt(30_000_000, 20_000_000),
    ];
    let mut profile = Profile::default();
    profile.placement.origin = eh3d_phoenix_export::profile::Origin::OutlineMin;
    let set = build(&f, &profile).unwrap();
    let rows = placement_rows(&set, "layers/placement.csv");
    assert_eq!(rows[3][3], "9.0000");
    assert_eq!(rows[3][4], "8.0000");
}

#[test]
fn the_bill_of_materials_groups_by_value_and_package() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    let rows = parse_csv(file(&set, "layers/bom.csv"), ',');
    assert_eq!(
        rows,
        vec![
            vec!["Designators", "Quantity", "Value", "Package"],
            vec!["C2 C10", "2", "100n", "0603"],
            vec!["R1 R2", "2", "10k", "SOIC"],
        ]
    );
}

#[test]
fn a_board_without_components_gives_files_with_only_a_header() {
    let set = build(&Board::default(), &Profile::default()).unwrap();
    assert!(codes_of(&set).contains(&codes::NO_COMPONENTS.to_string()));
    assert_eq!(placement_rows(&set, "layers/placement.csv").len(), 1);
    assert_eq!(placement_rows(&set, "layers/bom.csv").len(), 1);
    assert!(set
        .files
        .iter()
        .all(|f| !matches!(f.role, FileRole::Drill { .. })));
    assert!(verify(&Board::default(), &Profile::default(), &set).is_empty());
}

#[test]
fn the_same_board_gives_the_same_bytes_in_every_form() {
    let profile = Profile::default();
    let a = build(&fixture(), &profile).unwrap();
    let b = build(&fixture(), &profile).unwrap();
    assert_eq!(a.files, b.files);
    assert_eq!(a.manifest().unwrap(), b.manifest().unwrap());
    let pa = export_package_reproducible(&fixture(), &profile).unwrap();
    let pb = export_package_reproducible(&fixture(), &profile).unwrap();
    assert_eq!(
        pa.package.to_bytes().unwrap(),
        pb.package.to_bytes().unwrap()
    );
    // another board gives another identity
    let mut other = fixture();
    other.components.get_mut(&id(30)).unwrap().value = "22k".into();
    let pc = export_package_reproducible(&other, &profile).unwrap();
    assert_ne!(pa.package.metadata.id, pc.package.metadata.id);
}

#[test]
fn the_manifest_lists_every_file_with_its_sum() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&set.manifest().unwrap()).unwrap();
    assert_eq!(manifest["algorithm"], "sha256");
    let files = manifest["files"].as_object().unwrap();
    assert_eq!(files.len(), set.files.len());
    for file in &set.files {
        let entry = &files[file.folder_name()];
        assert_eq!(entry["sha256"], eh3d_container::sha256_hex(&file.bytes));
        assert_eq!(entry["size"], file.bytes.len());
    }
}

#[test]
fn the_folder_holds_the_files_and_the_manifest() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fab");
    set.write_folder(&out).unwrap();
    for file in &set.files {
        assert_eq!(
            std::fs::read(out.join(file.folder_name())).unwrap(),
            file.bytes
        );
    }
    assert_eq!(
        std::fs::read(out.join("manifest.json")).unwrap(),
        set.manifest().unwrap()
    );
    // a path below a file cannot be created
    let blocked = out.join("manifest.json").join("inside");
    let error = set.write_folder(&blocked).unwrap_err();
    assert_eq!(error.code(), codes::OUTPUT_FAILED);
}

#[test]
fn a_file_that_is_already_in_the_folder_is_never_replaced() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let name = set.files.last().unwrap().folder_name().to_owned();
    std::fs::write(dir.path().join(&name), b"mine").unwrap();
    let error = set.write_folder(dir.path()).unwrap_err();
    assert_eq!(error.code(), codes::OUTPUT_FAILED);
    assert_eq!(std::fs::read(dir.path().join(&name)).unwrap(), b"mine");
    // nothing else was written before the refusal
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    // a second write into the folder of a first one is refused too
    let out = dir.path().join("fab");
    set.write_folder(&out).unwrap();
    assert!(set.write_folder(&out).is_err());
}

#[test]
fn the_package_has_the_layers_and_the_assembly_files() {
    let f = fixture();
    let options = ExportOptions {
        project_id: uuid::Uuid::from_u128(7),
        created: chrono::DateTime::UNIX_EPOCH,
        outline_width: nm(100_000),
    };
    let out = export_package_with(&f, &options, &Profile::default()).unwrap();
    let names: Vec<&str> = out.package.entry_names().collect();
    for expected in [
        "layers/01_bottom_copper.gbr",
        "layers/drill_pth.drl",
        "layers/drill_npth.drl",
        "layers/placement.csv",
        "layers/bom.csv",
    ] {
        assert!(names.contains(&expected), "{expected}");
    }
    // the layers and the drills are named in the metadata, the CSV files are not
    assert_eq!(out.package.metadata.layers.len(), 8);
    let bytes = out.package.to_bytes().unwrap();
    let read = eh3d_common::Package::read(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(read.entry_names().count(), 10);
}

#[test]
fn the_check_finds_nothing_in_a_good_set_and_finds_what_is_lost() {
    let f = fixture();
    let profile = Profile::default();
    let set = build(&f, &profile).unwrap();
    assert!(verify(&f, &profile, &set).is_empty());

    let found = |set: &FabSet| -> Vec<String> {
        verify(&f, &profile, set)
            .iter()
            .filter_map(|d| d.code.as_ref().map(|c| c.to_string()))
            .collect()
    };
    let edit = |set: &mut FabSet, path: &str, change: &dyn Fn(&str) -> String| {
        let file = set.files.iter_mut().find(|f| f.path == path).unwrap();
        file.bytes = change(file.text()).into_bytes();
    };

    // a pad lost from the top copper
    let mut lost = set.clone();
    edit(&mut lost, "layers/02_top_copper.gbr", &|t: &str| {
        let at = t.find("D03*\n").unwrap();
        let start = t[..at].rfind('\n').unwrap() + 1;
        format!("{}{}", &t[..start], &t[at + 5..])
    });
    assert_eq!(found(&lost), vec![codes::VERIFY_MISMATCH]);

    // a zone lost from the bottom copper
    let mut no_zone = set.clone();
    edit(&mut no_zone, "layers/01_bottom_copper.gbr", &|t: &str| {
        let start = t.find("G36*\n").unwrap();
        let end = t.find("G37*\n").unwrap() + 5;
        format!("{}{}", &t[..start], &t[end..])
    });
    assert!(found(&no_zone).iter().any(|c| c == codes::VERIFY_MISMATCH));

    // an outline that is larger than the board
    let mut wide = set.clone();
    edit(&mut wide, "layers/06_outline.gbr", &|t: &str| {
        t.replace("X30000000", "X31000000")
    });
    assert!(found(&wide).iter().any(|c| c == codes::VERIFY_MISMATCH));

    // a drill file lost, a drill file emptied
    let mut no_drill = set.clone();
    no_drill
        .files
        .retain(|f| f.role != FileRole::Drill { plated: true });
    assert_eq!(found(&no_drill), vec![codes::VERIFY_MISMATCH]);
    let mut empty_drill = set.clone();
    edit(&mut empty_drill, "layers/drill_pth.drl", &|_: &str| {
        "garbage".to_owned()
    });
    assert!(!found(&empty_drill).is_empty());

    // a part lost from the placement file and from the bill of materials
    let mut short = set.clone();
    edit(&mut short, "layers/placement.csv", &|t: &str| {
        t.lines()
            .take(3)
            .map(|l| format!("{l}\n"))
            .collect::<String>()
    });
    assert!(found(&short).iter().any(|c| c == codes::VERIFY_MISMATCH));
    let mut thin = set.clone();
    edit(&mut thin, "layers/bom.csv", &|t: &str| {
        t.lines()
            .take(2)
            .map(|l| format!("{l}\n"))
            .collect::<String>()
    });
    assert!(found(&thin).iter().any(|c| c == codes::VERIFY_MISMATCH));
}

#[test]
fn profiles_come_from_the_file_and_errors_have_their_codes() {
    let text = include_str!("../../../config/export_profiles.json");
    let set = ProfileSet::from_json(text).unwrap();
    let jlc = set.get("jlc-like").unwrap();
    let f = fixture();
    let built = build(&f, jlc).unwrap();
    assert!(verify(&f, jlc, &built).is_empty());
    let rows = parse_csv(file(&built, "layers/positions.csv"), ',');
    assert_eq!(
        rows[0],
        vec![
            "Designator",
            "Val",
            "Package",
            "Mid X",
            "Mid Y",
            "Rotation",
            "Layer"
        ]
    );
    // R2: bottom, 30 degrees, a half turn added, written without decimals
    assert_eq!(
        rows[4],
        vec!["R2", "10k", "SOIC", "20.0000", "10.0000", "210", "Bottom"]
    );
    let bom = parse_csv(file(&built, "layers/bom_assembly.csv"), ',');
    assert_eq!(bom[1], vec!["100n", "C2,C10", "0603", "2"]);
    // the oblong pad of R1 is a slot in this profile
    assert!(file(&built, "layers/drill_pth.drl").contains("G85"));

    let missing = set.get("nothing").unwrap_err();
    assert_eq!(missing.code(), codes::PROFILE_NOT_FOUND);
    assert_eq!(
        missing.diagnostic().code.as_deref(),
        Some(codes::PROFILE_NOT_FOUND)
    );
    let mut bad = Profile::default();
    bad.gerber.decimal_digits = 0;
    assert_eq!(build(&f, &bad).unwrap_err().code(), codes::PROFILE_INVALID);
}

fn with_level(level: X2Level) -> Profile {
    let mut profile = Profile::default();
    profile.gerber.x2.level = level;
    profile
}

fn layer_files(set: &FabSet) -> Vec<&str> {
    set.files
        .iter()
        .filter(|f| matches!(f.role, FileRole::Layer { .. }))
        .map(|f| f.text())
        .collect()
}

#[test]
fn the_default_level_writes_what_it_always_wrote() {
    let f = fixture();
    let set = build(&f, &Profile::default()).unwrap();
    for text in layer_files(&set) {
        assert!(!text.contains("%TA"), "{text}");
        assert!(!text.contains("%TO"), "{text}");
        assert!(!text.contains("%TD"), "{text}");
        assert!(!text.contains(".ProjectId"), "{text}");
        assert!(!text.contains(".Part"), "{text}");
    }
    let drill = file(&set, "layers/drill_pth.drl");
    assert!(!drill.contains("TA."));
    assert!(drill.contains("TF.FileFunction,Plated,1,2,PTH\n"));
    // the other levels keep the layers, the order and the geometry of the default level
    let full = build(&f, &with_level(X2Level::Full)).unwrap();
    assert_eq!(full.files.len(), set.files.len());
    for (a, b) in set.files.iter().zip(&full.files) {
        assert_eq!(a.path, b.path);
    }
}

#[test]
fn plain_x1_has_no_attribute_at_all() {
    let f = fixture();
    let profile = with_level(X2Level::X1);
    let set = build(&f, &profile).unwrap();
    for text in layer_files(&set) {
        assert!(!text.contains("%T"), "{text}");
        assert!(text.ends_with("M02*\n"));
    }
    assert!(!file(&set, "layers/drill_pth.drl").contains("#@!"));
    assert!(!file(&set, "layers/drill_npth.drl").contains("#@!"));
    assert!(verify(&f, &profile, &set).is_empty());
}

#[test]
fn the_file_level_adds_the_project_the_coordinates_and_the_part() {
    let f = fixture();
    let profile = with_level(X2Level::File);
    let set = build(&f, &profile).unwrap();
    let id = derived_project_id("Fab board").hyphenated().to_string();
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(
        top.contains(&format!("%TF.ProjectId,Fab board,{id},1*%")),
        "{top}"
    );
    assert!(top.contains("%TF.SameCoordinates,Original*%"));
    assert!(top.contains("%TF.Part,Single*%"));
    assert!(!top.contains("CreationDate"));
    assert!(!top.contains("%TA"));
    assert!(!top.contains("%TO"));
    let names: Vec<String> = eh3d_parser_gerber::parse(top)
        .attributes
        .iter()
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(
        names,
        vec![
            ".GenerationSoftware",
            ".ProjectId",
            ".SameCoordinates",
            ".FileFunction",
            ".FilePolarity",
            ".Part"
        ]
    );
    // the outline has no polarity; the drill file has the same project and no polarity
    assert!(!file(&set, "layers/06_outline.gbr").contains("FilePolarity"));
    let drill = file(&set, "layers/drill_pth.drl");
    assert!(drill.contains(&format!("TF.ProjectId,Fab board,{id},1")));
    assert!(drill.contains("TF.FileFunction,Plated,1,2,PTH,Drill"));
    assert!(drill.contains("TF.Part,Single"));
    assert!(!drill.contains("FilePolarity"));
    assert!(verify(&f, &profile, &set).is_empty());
}

#[test]
fn the_creation_date_only_appears_when_the_profile_gives_it() {
    let f = fixture();
    let mut profile = with_level(X2Level::File);
    let set = build(&f, &profile).unwrap();
    assert!(layer_files(&set)
        .iter()
        .all(|t| !t.contains("CreationDate")));
    profile.gerber.x2.creation_date = Some("2000-01-01T00:00:00+00:00".to_owned());
    profile.gerber.x2.project_id = Some("123e4567-e89b-12d3-a456-426614174000".to_owned());
    let set = build(&f, &profile).unwrap();
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(top.contains("%TF.CreationDate,2000-01-01T00:00:00+00:00*%"));
    assert!(top.contains("123e4567-e89b-12d3-a456-426614174000"));
    assert!(
        file(&set, "layers/drill_pth.drl").contains("TF.CreationDate,2000-01-01T00:00:00+00:00")
    );
}

#[test]
fn the_aperture_level_names_the_function_of_each_aperture() {
    let f = fixture();
    let profile = with_level(X2Level::Aperture);
    let set = build(&f, &profile).unwrap();
    let top = file(&set, "layers/02_top_copper.gbr");
    for function in ["Conductor", "ViaPad", "ComponentPad", "SMDPad,CuDef"] {
        assert!(
            top.contains(&format!("%TA.AperFunction,{function}*%\n%ADD")),
            "{function}"
        );
    }
    assert!(!top.contains("%TO"));
    let outline = file(&set, "layers/06_outline.gbr");
    assert!(outline.contains("%TA.AperFunction,Profile*%\n%ADD10C,0.1*%\n%TD.AperFunction*%\n"));
    // every definition follows the attribute of its function and the attribute is deleted after
    let read = eh3d_parser_gerber::parse(top);
    assert!(read.is_clean(), "{:?}", read.diagnostics);
    let kinds: Vec<&str> = read.attributes.iter().map(|a| a.kind.as_str()).collect();
    assert_eq!(kinds.last(), Some(&"TD"));
    assert_eq!(kinds.iter().filter(|k| **k == "TA").count(), 5);
    // the silkscreen has no function for its strokes
    assert!(!file(&set, "layers/05_top_silk.gbr").contains("%TA"));
    assert!(verify(&f, &profile, &set).is_empty());
}

#[test]
fn the_full_level_puts_the_net_pin_and_component_on_each_object() {
    let f = fixture();
    let profile = with_level(X2Level::Full);
    let set = build(&f, &profile).unwrap();
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(top.contains("%TO.N,N*%\n"), "{top}");
    assert!(top.contains("%TO.N,N/C*%\n"));
    assert!(top.contains("%TO.P,R1,1*%\n%TO.C,R1*%\n"));
    // eight pads are on the top copper, four components
    assert_eq!(top.matches("%TO.P,").count(), 8);
    assert_eq!(top.matches("%TO.C,").count(), 4);
    assert_eq!(top.matches("%TO.N,").count(), 2);
    assert!(top.ends_with("%TD.N*%\n%TD.P*%\n%TD.C*%\nM02*\n"), "{top}");
    let read = eh3d_parser_gerber::parse(top);
    assert!(read.is_clean(), "{:?}", read.diagnostics);
    // the mask file names the pins but not the nets; the outline has no object attribute
    let mask = file(&set, "layers/03_top_mask.gbr");
    assert!(mask.contains("%TO.P,R1,MH*%"));
    assert!(!mask.contains("%TO.N,"));
    assert!(!file(&set, "layers/06_outline.gbr").contains("%TO"));
    // the zone of the bottom copper carries its net
    let bottom = file(&set, "layers/01_bottom_copper.gbr");
    assert!(bottom.contains("%TO.N,N*%\nG36*\n"), "{bottom}");
    assert!(verify(&f, &profile, &set).is_empty());
}

#[test]
fn the_object_attributes_can_be_chosen() {
    let f = fixture();
    let mut profile = with_level(X2Level::Full);
    profile.gerber.x2.object_attributes = vec![ObjectAttribute::Net];
    let set = build(&f, &profile).unwrap();
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(top.contains("%TO.N,"));
    assert!(!top.contains("%TO.P,") && !top.contains("%TO.C,"));
    profile.gerber.x2.object_attributes = vec![ObjectAttribute::Component];
    let set = build(&f, &profile).unwrap();
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(!top.contains("%TO.N,") && !top.contains("%TO.P,"));
    assert!(top.contains("%TO.C,C10*%"));
    profile.gerber.x2.object_attributes = Vec::new();
    let set = build(&f, &profile).unwrap();
    assert!(!file(&set, "layers/02_top_copper.gbr").contains("%TO"));
}

#[test]
fn pads_that_are_fiducials_or_heatsinks_get_their_function() {
    let mut f = fixture();
    let footprint = id(11);
    f.footprints.get_mut(&footprint).unwrap().name = "Fiducial 1mm".into();
    let mut profile = with_level(X2Level::Aperture);
    let set = build(&f, &profile).unwrap();
    assert!(file(&set, "layers/02_top_copper.gbr").contains("%TA.AperFunction,FiducialPad,Local*%"));
    profile.gerber.x2.fiducial_scope = FiducialScope::Global;
    let set = build(&f, &profile).unwrap();
    assert!(
        file(&set, "layers/02_top_copper.gbr").contains("%TA.AperFunction,FiducialPad,Global*%")
    );
    let mut g = fixture();
    let soic = id(10);
    g.footprints
        .get_mut(&soic)
        .unwrap()
        .pads
        .get_mut(&id(21))
        .unwrap()
        .number = "EP".into();
    let set = build(&g, &with_level(X2Level::Aperture)).unwrap();
    assert!(file(&set, "layers/02_top_copper.gbr").contains("%TA.AperFunction,HeatsinkPad*%"));
}

#[test]
fn the_drill_tools_get_a_function_and_equal_diameters_get_two_tools() {
    let mut f = fixture();
    // a via drilled as the plated hole of pad 1
    f.vias.get_mut(&id(41)).unwrap().drill = nm(800_000);
    let profile = with_level(X2Level::Aperture);
    let set = build(&f, &profile).unwrap();
    let pth = file(&set, "layers/drill_pth.drl");
    assert!(
        pth.contains("; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT01C0.3"),
        "{pth}"
    );
    assert!(
        pth.contains("; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT02C0.8"),
        "{pth}"
    );
    assert!(
        pth.contains("; #@! TA.AperFunction,Plated,PTH,ComponentDrill\nT03C0.8"),
        "{pth}"
    );
    assert!(pth.contains("; #@! TD.AperFunction\n%\n"));
    let read = eh3d_parser_excellon::parse(pth);
    assert!(read.is_clean(), "{:?}", read.diagnostics);
    assert_eq!(read.tools.len(), 4);
    let npth = file(&set, "layers/drill_npth.drl");
    assert!(
        npth.contains("; #@! TA.AperFunction,NonPlated,NPTH,MechanicalDrill\nT01C3.2"),
        "{npth}"
    );
    assert!(npth.contains("TF.FileFunction,NonPlated,1,2,NPTH,Drill"));
    assert!(verify(&f, &profile, &set).is_empty());
    // a slot makes the file a mixed one
    let mut slots = with_level(X2Level::Aperture);
    slots.drill.oblong_as_slot = true;
    let set = build(&fixture(), &slots).unwrap();
    assert!(file(&set, "layers/drill_pth.drl").contains("TF.FileFunction,Plated,1,2,PTH,Mixed"));
}

#[test]
fn text_that_a_field_cannot_hold_is_escaped_or_refused() {
    let mut f = fixture();
    f.nets.get_mut(&id(7)).unwrap().name = "A,B".into();
    let mut profile = with_level(X2Level::Full);
    let set = build(&f, &profile).unwrap();
    let top = file(&set, "layers/02_top_copper.gbr");
    assert!(top.contains("%TO.N,A\\u002CB*%"), "{top}");
    assert!(codes_of(&set).contains(&codes::ATTRIBUTE_ESCAPED.to_string()));
    assert!(verify(&f, &profile, &set).is_empty());
    profile.gerber.x2.strict = true;
    let error = build(&f, &profile).unwrap_err();
    assert_eq!(error.code(), codes::ATTRIBUTE_REJECTED);
    assert!(error.to_string().contains("U+002C"), "{error}");
    // a name that needs no escape is fine in strict mode
    assert!(build(&fixture(), &profile).is_ok());
    // below the object level the net names are not written, so nothing is refused
    let mut file_level = with_level(X2Level::File);
    file_level.gerber.x2.strict = true;
    assert!(build(&f, &file_level).is_ok());
}

#[test]
fn every_level_gives_the_same_bytes_every_time() {
    let f = fixture();
    for level in [
        X2Level::Legacy,
        X2Level::X1,
        X2Level::File,
        X2Level::Aperture,
        X2Level::Full,
    ] {
        let profile = with_level(level);
        let a = build(&f, &profile).unwrap();
        let b = build(&f, &profile).unwrap();
        for (x, y) in a.files.iter().zip(&b.files) {
            assert_eq!(x.bytes, y.bytes, "{}", x.path);
        }
        assert_eq!(a.manifest().unwrap(), b.manifest().unwrap());
    }
}

#[test]
fn the_x2_choices_of_a_profile_file_are_checked() {
    let ok = r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "gerber": {"x2": {
        "level": "full", "object_attributes": ["net", "pin"], "strict": true,
        "creation_date": "2000-01-01T00:00:00Z", "project_revision": "B",
        "same_coordinates": "", "fiducial_names": ["fid"], "fiducial_scope": "global",
        "heatsink_pad_numbers": ["EP", "TAB"]}}}]}"#;
    let set = ProfileSet::from_json(ok).unwrap();
    let x2 = &set.get("a").unwrap().gerber.x2;
    assert_eq!(x2.level, X2Level::Full);
    assert!(x2.strict && x2.writes(ObjectAttribute::Pin) && !x2.writes(ObjectAttribute::Component));
    assert_eq!(x2.fiducial_scope, FiducialScope::Global);
    for text in [
        r#"{"level": "everything"}"#,
        r#"{"unknown": 1}"#,
        r#"{"creation_date": "yesterday"}"#,
        r#"{"project_id": "not-a-uuid"}"#,
        r#"{"project_revision": ""}"#,
        r#"{"project_revision": "a,b"}"#,
        r#"{"same_coordinates": "a*b"}"#,
        r#"{"object_attributes": ["net", "net"]}"#,
        r#"{"object_attributes": ["colour"]}"#,
        r#"{"fiducial_names": [" "]}"#,
        r#"{"heatsink_pad_numbers": [""]}"#,
    ] {
        let json = format!(
            r#"{{"format": "phoenix-export/1", "profiles": [{{"name": "a", "gerber": {{"x2": {text}}}}}]}}"#
        );
        let error = ProfileSet::from_json(&json).unwrap_err();
        assert_eq!(error.code(), codes::PROFILE_INVALID, "{text}");
    }
    let shipped =
        ProfileSet::from_json(include_str!("../../../config/export_profiles.json")).unwrap();
    assert_eq!(
        shipped.get("x2-full").unwrap().gerber.x2.level,
        X2Level::Full
    );
    assert_eq!(shipped.get("x1").unwrap().gerber.x2.level, X2Level::X1);
    assert_eq!(
        shipped.get("generic").unwrap().gerber.x2.level,
        X2Level::Legacy
    );
}
