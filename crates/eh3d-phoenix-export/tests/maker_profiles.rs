//! The export profiles of the manufacturers that come with the program: the names of the layer
//! files, the formats and the order of the files, checked on the files of a board of four copper
//! layers that is made here, for every profile.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadShape, Side, Via,
};
use eh3d_phoenix_export::{
    build, codes, maker_names, maker_profile, FabSet, FileRole, Profile, ProfileSet,
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

const COPPER_TOP: u16 = 1;
const COPPER_IN1: u16 = 2;
const COPPER_IN2: u16 = 3;
const COPPER_BOTTOM: u16 = 4;

fn pad(number: &str, at: Point, drill: i64, plated: bool, layers: Vec<Id>) -> Pad {
    Pad {
        number: number.into(),
        shape: PadShape::Circle {
            diameter: nm(drill + 800_000),
        },
        at,
        rotation: Udeg::ZERO,
        drill: Some(nm(drill)),
        layers,
        roundness: 0,
        plated,
    }
}

/// A board with four copper layers (two inner ones), a mask, a paste and a legend layer on each
/// side, the outline and a documentation layer; one part with a plated pad and a mounting hole, and
/// one via.
fn four_layers(name: &str) -> Board {
    let mut b = Board {
        name: name.into(),
        outline: vec![
            pt(0, 0),
            pt(30_000_000, 0),
            pt(30_000_000, 20_000_000),
            pt(0, 20_000_000),
        ],
        copper_stack: vec![
            id(COPPER_TOP),
            id(COPPER_IN1),
            id(COPPER_IN2),
            id(COPPER_BOTTOM),
        ],
        ..Board::default()
    };
    let layers = [
        (COPPER_TOP, "Top", LayerKind::Copper, None),
        (COPPER_IN1, "Inner 1", LayerKind::Copper, None),
        (COPPER_IN2, "Inner 2", LayerKind::Copper, None),
        (COPPER_BOTTOM, "Bottom", LayerKind::Copper, None),
        (10, "Mask top", LayerKind::SolderMask, Some(Side::Top)),
        (11, "Mask bottom", LayerKind::SolderMask, Some(Side::Bottom)),
        (12, "Paste top", LayerKind::Paste, Some(Side::Top)),
        (13, "Paste bottom", LayerKind::Paste, Some(Side::Bottom)),
        (14, "Legend top", LayerKind::Silkscreen, Some(Side::Top)),
        (
            15,
            "Legend bottom",
            LayerKind::Silkscreen,
            Some(Side::Bottom),
        ),
        (16, "Outline", LayerKind::Outline, None),
        (17, "Notes", LayerKind::Documentation, None),
    ];
    for (n, label, kind, side) in layers {
        b.layers.insert(
            id(n),
            Layer {
                name: label.into(),
                kind,
                side,
            },
        );
    }
    b.nets.insert(id(30), Net { name: "N".into() });
    let all_copper = vec![
        id(COPPER_TOP),
        id(COPPER_IN1),
        id(COPPER_IN2),
        id(COPPER_BOTTOM),
    ];
    let mut pads = BTreeMap::new();
    pads.insert(id(40), pad("1", pt(0, 0), 800_000, true, all_copper));
    pads.insert(
        id(41),
        pad("MH", pt(-3_000_000, 0), 3_200_000, false, vec![]),
    );
    b.footprints.insert(
        id(42),
        Footprint {
            name: "PART".into(),
            pads,
        },
    );
    b.components.insert(
        id(43),
        Component {
            refdes: "R1".into(),
            value: "1k".into(),
            symbol: None,
            footprint: id(42),
            at: pt(10_000_000, 10_000_000),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.vias.insert(
        id(44),
        Via {
            net: id(30),
            at: pt(15_000_000, 10_000_000),
            drill: nm(300_000),
            diameter: nm(600_000),
            from_layer: id(COPPER_TOP),
            to_layer: id(COPPER_BOTTOM),
        },
    );
    b
}

/// The layer files of a set, in the order of the set, by their names in a folder.
fn layer_names(set: &FabSet) -> Vec<String> {
    set.files
        .iter()
        .filter(|f| matches!(f.role, FileRole::Layer { .. }))
        .map(|f| f.folder_name().to_owned())
        .collect()
}

fn name_of(set: &FabSet, layer: u16) -> String {
    set.files
        .iter()
        .find(|f| matches!(f.role, FileRole::Layer { layer: l, .. } if l == id(layer)))
        .map(|f| f.folder_name().to_owned())
        .unwrap_or_else(|| panic!("no file for the layer {layer}"))
}

fn drill_names(set: &FabSet) -> Vec<String> {
    set.files
        .iter()
        .filter(|f| matches!(f.role, FileRole::Drill { .. }))
        .map(|f| f.folder_name().to_owned())
        .collect()
}

#[test]
fn the_manufacturers_that_come_with_the_program_are_these() {
    assert_eq!(
        maker_names(),
        [
            "jlcpcb-2l-cam",
            "jlcpcb-4l-cam",
            "jlcpcb-6l-cam",
            "pcbway-cam",
            "oshpark-2l-cam",
            "oshpark-4l-cam",
            "sunstone-2l-cam",
            "elecrow-cam",
            "apc-2l-cam",
            "apc-4l-cam",
            "seeed-2l-cam",
            "seeed-4l-cam",
            "lpkf-2l-cam",
            "advancedcircuits-2l-cam",
            "isocam-cam",
            "jlcpcb-kicad-names",
            "jlcpcb-altium-names",
            "jlcpcb-eagle-names",
            "jlcpcb-thermal-relief",
        ]
    );
    assert!(maker_profile("nobody").is_none());
    for name in maker_names() {
        let profile = maker_profile(&name).unwrap();
        assert_eq!(profile.name, name);
        assert!(!profile.description.is_empty(), "{name}");
        profile.validate().unwrap();
    }
}

#[test]
fn the_four_layer_board_gets_the_names_of_the_jlcpcb_job() {
    let profile = maker_profile("jlcpcb-4l-cam").unwrap();
    let set = build(&four_layers("Maker board"), &profile).unwrap();
    assert_eq!(
        layer_names(&set),
        [
            "maker_board.GKO",
            "maker_board.GTL",
            "maker_board.G1",
            "maker_board.G2",
            "maker_board.GBL",
            "maker_board.GTS",
            "maker_board.GBS",
            "maker_board.GTP",
            "maker_board.GBP",
            "maker_board.GTO",
            "maker_board.GBO",
            "11_notes.gbr",
        ]
    );
    assert_eq!(
        drill_names(&set),
        ["maker_board_drill.XLN", "maker_board_drill_npth.XLN"]
    );
    // the job asks for 3.4 coordinates in millimetres and 3.3 in the drill file
    let top = set
        .files
        .iter()
        .find(|f| f.folder_name() == "maker_board.GTL")
        .unwrap();
    assert!(top.text().contains("%FSLAX44Y44*%"), "{}", top.text());
    let drill = set
        .files
        .iter()
        .find(|f| f.folder_name() == "maker_board_drill.XLN")
        .unwrap();
    assert!(drill.text().contains("FILE_FORMAT=3:3"), "{}", drill.text());
}

#[test]
fn the_jlcpcb_pages_name_the_files_in_three_styles() {
    let board = four_layers("Maker board");
    let kicad = build(&board, &maker_profile("jlcpcb-kicad-names").unwrap()).unwrap();
    assert_eq!(name_of(&kicad, COPPER_TOP), "maker_board-F_Cu.gbr");
    assert_eq!(name_of(&kicad, COPPER_BOTTOM), "maker_board-B_Cu.gbr");
    assert_eq!(name_of(&kicad, 16), "maker_board-Edge_Cuts.gbr");
    assert_eq!(name_of(&kicad, 10), "maker_board-F_Mask.gbr");
    assert_eq!(name_of(&kicad, 14), "maker_board-F_SilkS.gbr");
    assert_eq!(name_of(&kicad, 12), "maker_board-F_Paste.gbr");
    assert_eq!(
        drill_names(&kicad),
        ["maker_board-PTH.drl", "maker_board-NPTH.drl"]
    );
    let altium = build(&board, &maker_profile("jlcpcb-altium-names").unwrap()).unwrap();
    for (layer, name) in [
        (COPPER_TOP, "maker_board.gtl"),
        (COPPER_BOTTOM, "maker_board.gbl"),
        (16, "maker_board.gko"),
        (10, "maker_board.gts"),
        (11, "maker_board.gbs"),
        (14, "maker_board.gto"),
        (15, "maker_board.gbo"),
        (12, "maker_board.gtp"),
        (13, "maker_board.gbp"),
    ] {
        assert_eq!(name_of(&altium, layer), name);
    }
    assert_eq!(drill_names(&altium)[0], "maker_board.txt");
    let eagle = build(&board, &maker_profile("jlcpcb-eagle-names").unwrap()).unwrap();
    assert_eq!(name_of(&eagle, COPPER_TOP), "Copper_top.gbr");
    assert_eq!(name_of(&eagle, COPPER_BOTTOM), "Copper_bottom.gbr");
    assert_eq!(name_of(&eagle, 16), "Profile.gbr");
    assert_eq!(name_of(&eagle, 10), "Soldermask_top.gbr");
    assert_eq!(drill_names(&eagle)[0], "Drill_1_16.xln");
}

#[test]
fn the_inner_layers_are_numbered_the_way_each_job_does() {
    let board = four_layers("Maker board");
    for (profile, first, second) in [
        ("jlcpcb-4l-cam", "maker_board.G1", "maker_board.G2"),
        (
            "oshpark-4l-cam",
            "maker_board.internalplane1.ger",
            "maker_board.internalplane2.ger",
        ),
        // the place in the copper stack: the first inner layer is the second layer
        ("apc-4l-cam", "maker_board.L2", "maker_board.L3"),
        ("seeed-4l-cam", "maker_board.GL2", "maker_board.GL3"),
    ] {
        let set = build(&board, &maker_profile(profile).unwrap()).unwrap();
        assert_eq!(name_of(&set, COPPER_IN1), first, "{profile}");
        assert_eq!(name_of(&set, COPPER_IN2), second, "{profile}");
    }
}

#[test]
fn a_two_layer_job_leaves_the_inner_layers_with_the_numbered_names() {
    let set = build(
        &four_layers("Maker board"),
        &maker_profile("jlcpcb-2l-cam").unwrap(),
    )
    .unwrap();
    assert_eq!(name_of(&set, COPPER_TOP), "maker_board.GTL");
    assert_eq!(name_of(&set, COPPER_BOTTOM), "maker_board.GBL");
    // the profile has no name for an inner layer: the usual one, with the extension of the profile
    assert_eq!(name_of(&set, COPPER_IN1), "10_inner_1.gbr");
    assert_eq!(name_of(&set, COPPER_IN2), "11_inner_2.gbr");
    assert_eq!(
        drill_names(&set),
        ["maker_board.XLN", "maker_board_npth.XLN"]
    );
}

#[test]
fn a_job_without_a_name_for_a_role_keeps_the_numbered_name() {
    // the job of PCBWay has no file for the legend of the bottom side or for the paste
    let set = build(
        &four_layers("Maker board"),
        &maker_profile("pcbway-cam").unwrap(),
    )
    .unwrap();
    assert_eq!(name_of(&set, 14), "maker_board.sst");
    assert!(name_of(&set, 15).ends_with("_legend_bottom.gbr"));
    assert!(name_of(&set, 12).ends_with("_paste_top.gbr"));
    // the job starts with the copper of the top and has the outline after the bottom copper
    let names = layer_names(&set);
    assert_eq!(&names[..2], ["maker_board.top", "maker_board.bot"]);
    assert_eq!(names[2], "maker_board.out");
}

#[test]
fn a_job_that_gives_no_format_keeps_the_default_one() {
    let set = build(
        &four_layers("Maker board"),
        &maker_profile("isocam-cam").unwrap(),
    )
    .unwrap();
    assert_eq!(
        layer_names(&set)[..3],
        ["maker_board.TOP", "maker_board.BOTTOM", "maker_board.DIM"]
    );
    let top = set
        .files
        .iter()
        .find(|f| f.folder_name() == "maker_board.TOP")
        .unwrap();
    assert!(top.text().contains("%FSLAX46Y46*%"), "{}", top.text());
    assert_eq!(
        drill_names(&set),
        ["maker_board.DRD", "maker_board_npth.DRD"]
    );
}

#[test]
fn every_profile_gives_distinct_plain_names_and_follows_its_order() {
    let board = four_layers("Maker board");
    for name in maker_names() {
        let profile = maker_profile(&name).unwrap();
        let set = build(&board, &profile).unwrap();
        let mut seen: Vec<String> = set.files.iter().map(|f| f.path.clone()).collect();
        let total = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), total, "{name}: two files with one name");
        for file in &set.files {
            let folder = file.folder_name();
            assert!(
                folder
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')),
                "{name}: {folder}"
            );
        }
        // every role that the profile names is a file called what the template says
        let names = &profile.gerber.names;
        for (role, layer) in [
            ("copper_top", COPPER_TOP),
            ("copper_bottom", COPPER_BOTTOM),
            ("mask_top", 10),
            ("mask_bottom", 11),
            ("paste_top", 12),
            ("paste_bottom", 13),
            ("silk_top", 14),
            ("silk_bottom", 15),
            ("outline", 16),
        ] {
            let template = names.template(role);
            let got = name_of(&set, layer);
            if template.is_empty() {
                assert!(got.ends_with(".gbr"), "{name} {role}: {got}");
            } else {
                assert_eq!(
                    got,
                    template.replace("{board}", "maker_board"),
                    "{name} {role}"
                );
            }
        }
        // the files of the roles in `order` come first, in that order
        let listed = layer_names(&set);
        let mut last = 0usize;
        for role in &names.order {
            let layers: Vec<u16> = match role.as_str() {
                "copper_top" => vec![COPPER_TOP],
                "copper_inner" => vec![COPPER_IN1, COPPER_IN2],
                "copper_bottom" => vec![COPPER_BOTTOM],
                "mask_top" => vec![10],
                "mask_bottom" => vec![11],
                "paste_top" => vec![12],
                "paste_bottom" => vec![13],
                "silk_top" => vec![14],
                "silk_bottom" => vec![15],
                _ => vec![16],
            };
            for layer in layers {
                let at = listed
                    .iter()
                    .position(|n| *n == name_of(&set, layer))
                    .unwrap();
                assert!(at >= last, "{name}: {role} is out of order");
                last = at;
            }
        }
        // what the profile does not list comes after what it lists
        let documentation = listed.iter().position(|n| *n == name_of(&set, 17)).unwrap();
        assert!(documentation >= last, "{name}");
    }
}

/// A small generator with a fixed seed (xorshift).
struct Seeded(u64);

impl Seeded {
    fn step(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn any_board_name_gives_valid_distinct_files_with_every_profile() {
    let mut rng = Seeded(0x5eed_1234_abcd_0017);
    const ALPHABET: &[u8] = b"abcXYZ019 _-./";
    let profiles: Vec<Profile> = maker_names()
        .iter()
        .map(|n| maker_profile(n).unwrap())
        .collect();
    for _ in 0..40 {
        let length = 1 + (rng.step() % 12) as usize;
        let name: String = (0..length)
            .map(|_| ALPHABET[(rng.step() % ALPHABET.len() as u64) as usize] as char)
            .collect();
        let board = four_layers(&name);
        for profile in &profiles {
            // a name that turns into a Windows device name is refused, anything else builds
            match build(&board, profile) {
                Ok(set) => {
                    let mut paths: Vec<&str> = set.files.iter().map(|f| f.path.as_str()).collect();
                    paths.sort_unstable();
                    paths.dedup();
                    assert_eq!(paths.len(), set.files.len(), "{name:?} {}", profile.name);
                }
                Err(e) => {
                    assert_eq!(
                        e.code(),
                        codes::PROFILE_INVALID,
                        "{name:?} {}",
                        profile.name
                    );
                }
            }
        }
    }
}

#[test]
fn the_same_board_and_profile_give_the_same_bytes() {
    let board = four_layers("Maker board");
    for name in maker_names() {
        let profile = maker_profile(&name).unwrap();
        let a = build(&board, &profile).unwrap().manifest().unwrap();
        let b = build(&board, &profile).unwrap().manifest().unwrap();
        assert_eq!(a, b, "{name}");
    }
}

#[test]
fn two_layers_with_one_name_are_refused_and_so_is_a_device_name() {
    // every inner layer would be called maker_board.G
    let text = r#"{"format": "phoenix-export/1", "profiles": [
        {"name": "clash", "gerber": {"names": {"copper_inner": "{board}.G"}}}]}"#;
    let set = ProfileSet::from_json(text).unwrap();
    let error = build(&four_layers("Maker board"), set.get("clash").unwrap()).unwrap_err();
    assert_eq!(error.code(), codes::PROFILE_INVALID);
    assert!(
        error.to_string().contains("two files are called"),
        "{error}"
    );
    // the board "con" would be called con.GTL, a device name of Windows
    let error = build(
        &four_layers("con"),
        &maker_profile("jlcpcb-4l-cam").unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code(), codes::PROFILE_INVALID);
}

#[test]
fn a_profile_file_names_a_template_for_each_role() {
    let text = r#"{"format": "phoenix-export/1", "profiles": [
        {"name": "mine", "gerber": {"names": {
            "copper_top": "{board}-front.gbr", "copper_inner": "{board}-in{inner}.gbr",
            "copper_bottom": "{board}-back.gbr", "outline": "{board}-edge.gbr",
            "order": ["outline", "copper_bottom"]}},
         "drill": {"plated_file": "{board}.drl", "non_plated_file": "{board}-mount.drl"}}]}"#;
    let set = ProfileSet::from_json(text).unwrap();
    let built = build(&four_layers("My board"), set.get("mine").unwrap()).unwrap();
    // the listed roles come first; the rest follow in the usual order
    assert_eq!(
        layer_names(&built)[..3],
        ["my_board-edge.gbr", "my_board-back.gbr", "my_board-in1.gbr"]
    );
    assert_eq!(name_of(&built, COPPER_TOP), "my_board-front.gbr");
    assert_eq!(name_of(&built, COPPER_IN2), "my_board-in2.gbr");
    assert_eq!(drill_names(&built), ["my_board.drl", "my_board-mount.drl"]);
}

#[test]
fn a_template_that_is_not_usable_is_refused_with_its_key() {
    for (names, key) in [
        (
            r#"{"copper_top": "{inner}.gbr"}"#,
            "gerber.names.copper_top",
        ),
        (r#"{"outline": "{board}/x.gbr"}"#, "gerber.names.outline"),
        (r#"{"mask_top": "{nothing}.gbr"}"#, "gerber.names.mask_top"),
        (r#"{"silk_top": ".hidden"}"#, "gerber.names.silk_top"),
        (r#"{"order": ["nothing"]}"#, "gerber.names.order"),
        (r#"{"order": ["outline", "outline"]}"#, "gerber.names.order"),
    ] {
        let text = format!(
            r#"{{"format": "phoenix-export/1", "profiles": [{{"name": "a", "gerber": {{"names": {names}}}}}]}}"#
        );
        let error = ProfileSet::from_json(&text).unwrap_err();
        assert_eq!(error.code(), codes::PROFILE_INVALID, "{names}");
        assert!(error.to_string().contains(key), "{names}: {error}");
    }
    // an unknown key of the names is refused by the reader
    let text = r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "gerber": {"names": {"copper": "x"}}}]}"#;
    assert!(ProfileSet::from_json(text).is_err());
}

#[test]
fn the_relief_profile_asks_for_four_spokes_inside_the_published_ranges() {
    let profile = maker_profile("jlcpcb-thermal-relief").unwrap();
    let g = &profile.gerber;
    assert_eq!(g.zone_thermal_spokes, 4);
    assert_eq!(g.zone_thermal_spoke_width_nm, 300_000);
    assert_eq!(g.zone_thermal_gap_nm, 400_000);
    assert!(!g.zone_thermal_non_round);
    // the names of the files stay the numbered ones
    let set = build(&four_layers("Maker board"), &profile).unwrap();
    assert!(layer_names(&set).iter().all(|n| n.ends_with(".gbr")));
}

#[test]
fn the_stencil_settings_of_the_public_pages_are_read_by_the_reader_of_the_program() {
    use eh3d_phoenix_export::stencil::{Reduction, StencilSettings};
    let s = StencilSettings::parse(include_str!("../../../config/stencil_smt.cfg")).unwrap();
    assert_eq!(s.thickness_nm, 120_000);
    assert_eq!(s.min_aspect_micro, 1_500_000);
    assert_eq!(s.min_area_micro, 660_000);
    assert_eq!(s.shrink, Reduction::Percent(6_000_000));
    // the rest keeps the defaults
    assert_eq!(s.window_max_nm, StencilSettings::default().window_max_nm);
}
