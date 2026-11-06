// SPDX-License-Identifier: GPL-3.0-or-later
//! The deep reading of EAGLE files on files written for the tests: the design rule file, the CAM
//! jobs in their three forms, the variants and attributes, the layers, the entities and the loss
//! report, on small files of `tests/corpus/eagle` and of `tests/data`.

use std::path::PathBuf;

use eh3d_phoenix_eagle::xml::{parse, write_document, Limits};
use eh3d_phoenix_eagle_board::read_board;
use eh3d_phoenix_eagle_deep::cam::CamFormat;
use eh3d_phoenix_eagle_deep::codes::{
    CAM_DEVICE_UNSUPPORTED, CAM_LAYER_NOT_EXPORTED, CAM_OPTION_IGNORED, DRU_FOLDED, DRU_NOT_USED,
    KEPT_OPAQUE, LAYER_NUMBER_INVALID, LAYER_RENAMED, LAYER_UNMAPPED, LEFT_OUT_BY_MODEL,
    NOT_EAGLE_FILE, SIMPLIFIED_BY_MODEL, VARIANT_INCONSISTENT, VARIANT_UNKNOWN,
};
use eh3d_phoenix_eagle_deep::dru::{DruFile, DruLine};
use eh3d_phoenix_eagle_deep::{
    apply_to_board, cam_to_profile, check_eagle, decode_text, dru_to_rules, read_entities,
    read_layers, read_variants, rules_to_dru, write_variants, CamJob, LayerMap, Severity,
};
use eh3d_phoenix_export::ProfileSet;
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_rules::RuleValues;

const DRU: &str = include_str!("data/two_layer.dru");
const JOB_FULL: &str = include_str!("data/job_full.cam");
const JOB_SMALL: &str = include_str!("data/job_small.cam");
const JOB_SMALL_XML: &str = include_str!("data/job_small_xml.cam");
const JOB_SMALL_JSON: &str = include_str!("data/job_small_json.cam");

const VARIANTS_BOARD: &str = include_str!("../../../tests/corpus/eagle/variants_board.brd");
const VARIANTS_SCHEMATIC: &str = include_str!("../../../tests/corpus/eagle/variants_schematic.sch");
const USER_LAYERS: &str = include_str!("../../../tests/corpus/eagle/user_layers.brd");
const POURS: &str = include_str!("../../../tests/corpus/eagle/pours_and_holes.brd");
const RULES_BOARD: &str = include_str!("../../../tests/corpus/eagle/rules_and_autorouter.brd");
const ODD: &str = include_str!("../../../tests/corpus/eagle/attributes_odd.brd");

fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn tree(text: &str) -> eh3d_phoenix_eagle::xml::Element {
    parse(text, &Limits::default()).unwrap()
}

#[test]
fn a_realistic_design_rule_file_is_read_written_and_turned_into_rules() {
    let dru = DruFile::parse(DRU).unwrap();
    assert_eq!(lf(&dru.to_text()), lf(DRU));
    assert_eq!(DruFile::parse(&dru.to_text()).unwrap(), dru);
    assert!(matches!(&dru.lines[0], DruLine::Description { lang, .. } if lang == "de"));
    assert_eq!(dru.param("layerSetup"), Some("(1*16)"));
    let done = dru_to_rules(&dru);
    let v = done.values;
    assert_eq!(v.min_track_width.get(), 254_000);
    assert_eq!(v.min_drill.get(), 300_000);
    assert_eq!(v.min_edge_clearance.get(), 1_016_000);
    assert_eq!(v.min_drill_spacing.get(), 203_200);
    assert_eq!(v.min_clearance.get(), 203_200);
    assert_eq!(v.min_annular_ring.get(), 254_000);
    let codes: Vec<&str> = done.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![DRU_FOLDED, DRU_NOT_USED]);
    let unused = &done.findings[1].detail;
    for key in [
        "layerSetup",
        "mtCopper",
        "rvPadTop",
        "psTop",
        "checkGrid",
        "maxErrors",
    ] {
        assert!(unused.contains(key), "{key}");
    }
    assert!(!unused.contains("msWidth") && !unused.contains("mdWireWire"));
}

#[test]
fn rules_written_over_a_base_file_keep_its_other_lines_and_read_back() {
    let base = DruFile::parse(DRU).unwrap();
    let values = RuleValues::from_json(
        r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":150000,"min_clearance_nm":200000,"min_via_diameter_nm":600000}}"#,
    )
    .unwrap()
    .1;
    let written = rules_to_dru(&values, &Default::default(), "fab", Some(&base));
    assert_eq!(written.dru.param("msWidth"), Some("0.15mm"));
    assert_eq!(written.dru.param("mdWireWire"), Some("0.2mm"));
    assert_eq!(written.dru.param("layerSetup"), Some("(1*16)"));
    assert_eq!(written.dru.param("msDrill"), Some("0.3mm"));
    assert_eq!(written.findings.len(), 1);
    assert_eq!(
        written.findings[0].subject.as_deref(),
        Some("min_via_diameter")
    );
    let back = dru_to_rules(&DruFile::parse(&written.dru.to_text()).unwrap());
    assert_eq!(back.values.min_track_width, values.min_track_width);
    assert_eq!(back.values.min_clearance, values.min_clearance);
}

#[test]
fn a_hostile_design_rule_file_is_refused_or_named_and_never_panics() {
    for text in [
        "msWidth\n",
        "msWidth = 6mil\n\0",
        "= = =\n",
        "description[ = x\n",
    ] {
        assert!(DruFile::parse(text).is_err(), "{text:?}");
    }
    let weird =
        DruFile::parse("msWidth = 99999999999999999999mil\nmdWireWire = -3mm\nx_y = 1\n").unwrap();
    let done = dru_to_rules(&weird);
    assert_eq!(done.values, RuleValues::default());
    assert_eq!(done.findings.len(), 3);
    assert!(done.findings.iter().any(|f| f.detail.contains("x_y")));
}

#[test]
fn a_full_cam_job_is_turned_into_a_profile_and_what_is_lost_is_named() {
    let job = CamJob::parse(JOB_FULL).unwrap();
    assert_eq!(job.sections.len(), 9);
    let done = cam_to_profile(&job, "board-job", &LayerMap::default());
    assert_eq!((done.gerber_sections, done.drill_sections), (7, 1));
    let set = ProfileSet::from_json(&done.profile_json).unwrap();
    let profile = set.get("board-job").unwrap();
    assert_eq!(profile.gerber.extension, "gbr");
    assert_eq!(profile.drill.fixed_integer_digits, 2);
    assert_eq!(profile.drill.fixed_decimal_digits, 4);
    let codes: Vec<&str> = done.findings.iter().map(|f| f.code).collect();
    assert_eq!(
        codes,
        vec![
            CAM_OPTION_IGNORED,
            CAM_OPTION_IGNORED,
            CAM_LAYER_NOT_EXPORTED,
            CAM_LAYER_NOT_EXPORTED,
            CAM_OPTION_IGNORED,
            CAM_DEVICE_UNSUPPORTED,
            CAM_OPTION_IGNORED,
        ]
    );
    assert_eq!(
        done.findings[0].detail,
        "options not respected: wheel, rack, scale, offset"
    );
    assert_eq!(done.findings[2].detail, "layers 25 (tNames), 27 (tValues)");
    assert_eq!(done.findings[3].detail, "layers 46 (Milling)");
    assert_eq!(
        done.findings[5].subject.as_deref(),
        Some("Plot of the documents")
    );
    assert!(done.findings[6]
        .detail
        .contains("7 different extensions (cmp, crc, dim, plc, sol, stc, sts)"));
    assert!(done.findings.iter().all(|f| f.severity != Severity::Error));
}

#[test]
fn the_three_forms_of_a_job_give_the_same_sections_and_the_same_profile() {
    let ini = CamJob::parse(JOB_SMALL).unwrap();
    let xml = CamJob::parse(JOB_SMALL_XML).unwrap();
    let json = CamJob::parse(JOB_SMALL_JSON).unwrap();
    assert_eq!(
        (ini.format, xml.format, json.format),
        (CamFormat::Ini, CamFormat::Xml, CamFormat::Json)
    );
    assert_eq!(ini.sections, xml.sections);
    assert_eq!(ini.sections, json.sections);
    let map = LayerMap::default();
    let a = cam_to_profile(&ini, "p", &map);
    assert_eq!(a, cam_to_profile(&xml, "p", &map));
    assert_eq!(a, cam_to_profile(&json, "p", &map));
    let profile = ProfileSet::from_json(&a.profile_json).unwrap();
    let p = profile.get("p").unwrap();
    assert_eq!(p.gerber.extension, "gtl");
    assert_eq!(p.gerber.decimal_digits, 5);
}

#[test]
fn the_variants_of_a_board_decide_what_the_bill_and_the_placement_use() {
    let root = tree(VARIANTS_BOARD);
    let set = read_variants(&root);
    assert_eq!(set.names(), vec!["FULL", "LITE"]);
    assert_eq!(set.current(), Some("FULL"));
    assert_eq!(set.global_attributes.len(), 1);
    assert_eq!(set.parts.len(), 3);
    assert_eq!(set.parts[0].attribute("MPN"), Some("RC0805FR-0710KL"));
    assert!(set.check().is_empty());
    let lite = set.view("LITE").unwrap();
    assert_eq!(lite.not_fitted(), vec!["R1"]);
    assert_eq!(lite.parts["R2"].value, "10k");
    assert_eq!(lite.parts["R3"].value, "100R");
    assert_eq!(set.view("NOPE").unwrap_err().code, VARIANT_UNKNOWN);

    let mut n = 0u32;
    let mut new_id = || {
        n += 1;
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    };
    let mut board = read_board(VARIANTS_BOARD, &mut new_id).unwrap().board;
    assert_eq!(board.components.len(), 3);
    let applied = apply_to_board(&mut board, &lite);
    assert_eq!(applied.removed, vec!["R1"]);
    assert_eq!(
        applied.revalued,
        vec![("R2".to_owned(), "4k7".to_owned(), "10k".to_owned())]
    );
    assert_eq!(board.components.len(), 2);
    assert!(board
        .pad_nets
        .iter()
        .all(|p| board.components.contains_key(&p.component)));
    // the base design is the file as it stands
    let base = set.view("").unwrap();
    assert!(base.not_fitted().is_empty());
}

#[test]
fn the_variants_of_a_schematic_and_its_module_are_read() {
    let root = tree(VARIANTS_SCHEMATIC);
    let set = read_variants(&root);
    assert_eq!(set.names(), vec!["FULL", "LITE"]);
    assert_eq!(set.parts.len(), 2);
    assert_eq!(set.view("LITE").unwrap().not_fitted(), vec!["R1"]);
    let entities = read_entities(&root);
    assert_eq!(entities.modules.len(), 1);
    assert_eq!(entities.modules[0].ports, 1);
    assert_eq!(entities.module_instances.len(), 1);
    assert_eq!(entities.module_instances[0].variant.as_deref(), Some("A"));
    assert_eq!(entities.classes.len(), 1);
}

#[test]
fn rewriting_what_was_read_changes_nothing_and_a_new_variant_survives_a_rewrite() {
    let mut root = tree(VARIANTS_BOARD);
    let before = write_document(&root);
    let set = read_variants(&root);
    assert!(write_variants(&mut root, &set).is_empty());
    assert_eq!(write_document(&root), before);

    let mut changed = set.clone();
    changed.parts[2]
        .variants
        .push(eh3d_phoenix_eagle_deep::variants::Record {
            attrs: vec![
                ("name".to_owned(), "LITE".to_owned()),
                ("populate".to_owned(), "no".to_owned()),
            ],
        });
    assert!(write_variants(&mut root, &changed).is_empty());
    let again = tree(&write_document(&root));
    assert_eq!(read_variants(&again), changed);
    assert_eq!(
        read_variants(&again).view("LITE").unwrap().not_fitted(),
        vec!["R1", "R3"]
    );
    // everything else in the tree is as it was
    let board = |t: &str| {
        let r = tree(t);
        let b = r.child("drawing").unwrap().child("board").unwrap();
        (
            b.child("plain").unwrap().to_xml(),
            b.child("libraries").unwrap().to_xml(),
            b.child("signals").unwrap().to_xml(),
        )
    };
    assert_eq!(board(VARIANTS_BOARD), board(&write_document(&root)));
}

#[test]
fn odd_attributes_and_variants_are_kept_and_the_oddities_named() {
    let root = tree(ODD);
    let set = read_variants(&root);
    assert_eq!(set.parts.len(), 3);
    assert_eq!(set.parts[0].value(), "<1k>");
    assert_eq!(
        set.parts[0].attribute("NOTE"),
        Some("line one\nline \"two\" & more")
    );
    let findings = set.check();
    assert_eq!(findings.len(), 4);
    assert!(findings.iter().all(|f| f.code == VARIANT_INCONSISTENT));
    let first = set.view("A & B").unwrap();
    assert_eq!(first.not_fitted(), vec!["R1"]);
    assert_eq!(set.view("Ünï").unwrap().parts["R1"].value, "日本語");
    // a rewrite keeps all of it
    let mut again = root.clone();
    assert!(write_variants(&mut again, &set).is_empty());
    assert_eq!(tree(&write_document(&again)), root);
}

#[test]
fn the_layers_of_a_file_are_judged_and_a_map_gives_the_user_layers_a_role() {
    let root = tree(USER_LAYERS);
    let none = read_layers(&root, &LayerMap::default());
    let numbers: Vec<u16> = none.layers.iter().map(|l| l.number).collect();
    assert_eq!(numbers, vec![1, 16, 20, 25, 100, 101, 102]);
    let count = |code: &str| none.findings.iter().filter(|f| f.code == code).count();
    assert_eq!(count(LAYER_RENAMED), 1);
    assert_eq!(count(LAYER_UNMAPPED), 3);
    assert_eq!(count(LAYER_NUMBER_INVALID), 2);
    assert!(!none.layers[6].visible);

    let map = LayerMap::parse("layer.100 = milling\nname.Notes = document\n").unwrap();
    let mapped = read_layers(&root, &map);
    let unmapped: Vec<&str> = mapped
        .findings
        .iter()
        .filter(|f| f.code == LAYER_UNMAPPED)
        .filter_map(|f| f.subject.as_deref())
        .collect();
    assert_eq!(unmapped, vec!["Assembly"]);
}

#[test]
fn the_entities_the_model_has_no_place_for_are_listed() {
    let e = read_entities(&tree(POURS));
    assert_eq!(e.dimensions.len(), 1);
    assert_eq!(e.holes.len(), 2);
    assert_eq!(e.vias.len(), 2);
    assert_eq!(e.polygons.len(), 2);
    assert_eq!(e.polygons[0].pour, "hatch");
    assert_eq!(e.polygons[0].thermals, "no");
    assert_eq!(e.polygons[0].orphans, "yes");
    assert_eq!(e.polygons[0].rank, "2");
    assert_eq!(e.polygons[0].isolate.as_deref(), Some("0.3"));
    assert_eq!(e.polygons[1].pour, "cutout");
    assert_eq!(e.polygons[1].vertices, 3);
    assert_eq!(e.vias[1].extent, "1-2");
    assert_eq!(e.vias[1].shape, "square");
    assert_eq!(e.elements.len(), 2);
    assert!(e.elements[0].smashed && e.elements[1].locked);
    assert_eq!(e.signals[0].class, "1");

    let e = read_entities(&tree(RULES_BOARD));
    assert_eq!(e.classes.len(), 3);
    assert_eq!(e.classes[1].clearances, 2);
    let rules = e.design_rules.unwrap();
    assert_eq!((rules.name.as_str(), rules.params), ("fab-one", 8));
    assert_eq!(e.autorouter.len(), 2);
    assert_eq!(e.autorouter[0].params, 2);
    assert_eq!(e.autorouter[1].active, "no");
}

#[test]
fn the_design_rules_inside_a_board_are_the_same_as_the_file_of_rules() {
    let root = tree(RULES_BOARD);
    let element = root
        .child("drawing")
        .unwrap()
        .child("board")
        .unwrap()
        .child("designrules")
        .unwrap();
    let dru = DruFile::from_designrules(element);
    assert_eq!(dru.name, "fab-one");
    assert_eq!(dru.param("msWidth"), Some("6mil"));
    assert_eq!(dru.lines.len(), 10);
    let back = dru.to_designrules();
    assert_eq!(DruFile::from_designrules(&back), dru);
    let done = dru_to_rules(&dru);
    assert_eq!(done.values.min_track_width.get(), 152_400);
    assert_eq!(done.values.min_clearance.get(), 254_000);
    assert_eq!(done.values.min_annular_ring.get(), 254_000);
}

#[test]
fn the_loss_report_of_a_board_names_what_the_model_would_lose() {
    let check = check_eagle(POURS, &LayerMap::default()).unwrap();
    assert!(check.tree_lossless && !check.has_errors());
    let find = |code: &str, subject: &str| {
        check
            .findings
            .iter()
            .filter(|f| f.code == code && f.subject.as_deref() == Some(subject))
            .map(|f| f.detail.clone())
            .collect::<Vec<String>>()
    };
    assert_eq!(
        find(SIMPLIFIED_BY_MODEL, "polygon"),
        vec![
            "1 element(s): pour not kept",
            "1 element(s): pour, thermals, orphans, rank, spacing not kept"
        ]
    );
    assert_eq!(
        find(SIMPLIFIED_BY_MODEL, "via"),
        vec!["1 element(s): shape not kept"]
    );
    assert_eq!(
        find(SIMPLIFIED_BY_MODEL, "element"),
        vec![
            "1 element(s): locked not kept",
            "1 element(s): smashed not kept"
        ]
    );
    assert_eq!(
        find(SIMPLIFIED_BY_MODEL, "signal"),
        vec!["1 element(s): class not kept"]
    );
    assert_eq!(find(LEFT_OUT_BY_MODEL, "dimension").len(), 1);
    assert_eq!(find(LEFT_OUT_BY_MODEL, "hole").len(), 1);
    let rules = check_eagle(RULES_BOARD, &LayerMap::default()).unwrap();
    for name in ["classes", "designrules", "autorouter"] {
        assert!(rules
            .findings
            .iter()
            .any(|f| f.code == LEFT_OUT_BY_MODEL && f.subject.as_deref() == Some(name)));
    }
    let odd = check_eagle(ODD, &LayerMap::default()).unwrap();
    assert!(odd.tree_lossless);
    assert!(odd.findings.iter().any(|f| f.code == VARIANT_INCONSISTENT));
    let sch = check_eagle(VARIANTS_SCHEMATIC, &LayerMap::default()).unwrap();
    assert_eq!(sch.kind, "schematic");
    assert!(sch.findings.iter().any(|f| f.code == KEPT_OPAQUE));
}

#[test]
fn every_file_of_the_eagle_corpus_is_rewritten_without_loss_or_refused_with_a_code() {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/eagle");
    let mut read = 0;
    let mut refused = 0;
    for entry in std::fs::read_dir(folder.as_path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !(name.ends_with(".brd") || name.ends_with(".sch") || name.ends_with(".lbr")) {
            continue;
        }
        let (text, _) = decode_text(&std::fs::read(path.as_path()).unwrap());
        match check_eagle(&text, &LayerMap::default()) {
            Ok(check) => {
                assert!(check.tree_lossless, "{name}");
                assert_eq!(check.opaque_nodes, check.opaque_identical, "{name}");
                read += 1;
            }
            Err(error) => {
                assert_eq!(error.code, NOT_EAGLE_FILE, "{name}");
                refused += 1;
            }
        }
    }
    assert!(read >= 12, "{read}");
    assert!(refused >= 3, "{refused}");
}
