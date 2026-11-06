// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `gerber-import` and `panel` on files and boards written by the test itself.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side, Via,
};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

const MM: i64 = 1_000_000;

fn command(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board of 30 by 20 mm with two copper layers in a stack, one component with a drilled pad and
/// a pad on the top side only, and a via.
fn board() -> Board {
    let (top, bottom) = (id(1), id(2));
    let net = id(3);
    let mut b = Board {
        name: "gerber".into(),
        outline: vec![
            pt(0, 0),
            pt(30 * MM, 0),
            pt(30 * MM, 20 * MM),
            pt(0, 20 * MM),
        ],
        copper_stack: vec![top, bottom],
        ..Board::default()
    };
    for (l, name) in [(top, "Top"), (bottom, "Bottom")] {
        b.layers.insert(
            l,
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    b.nets.insert(net, Net { name: "N".into() });
    let (round, rect) = (id(10), id(11));
    let mut pads = BTreeMap::new();
    pads.insert(
        round,
        Pad {
            number: "1".into(),
            shape: PadShape::Circle {
                diameter: Nm::new(1_600_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: Some(Nm::new(800_000).unwrap()),
            layers: vec![top, bottom],
        },
    );
    pads.insert(
        rect,
        Pad {
            number: "2".into(),
            shape: PadShape::Rect {
                width: Nm::new(1_000_000).unwrap(),
                height: Nm::new(2_000_000).unwrap(),
            },
            at: pt(3 * MM, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![top],
        },
    );
    let fp = id(20);
    b.footprints.insert(
        fp,
        Footprint {
            name: "F".into(),
            pads,
        },
    );
    let cid = id(30);
    b.components.insert(
        cid,
        Component {
            refdes: "R1".into(),
            value: "1k".into(),
            symbol: None,
            footprint: fp,
            at: pt(10 * MM, 10 * MM),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.pad_nets.push(PadNet {
        component: cid,
        pad: round,
        net,
    });
    b.vias.insert(
        id(40),
        Via {
            net,
            at: pt(20 * MM, 10 * MM),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: top,
            to_layer: bottom,
        },
    );
    b
}

const COPPER: &str = "%TF.FileFunction,Copper,L1,Top*%\n%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.2*%\nD10*\nX1500000Y2500000D02*\nX4500000Y2500000D01*\nX0Y0D03*\nM02*\n";
const DRILL: &str = "M48\nMETRIC,TZ\nT1C0.8\n%\nT1\nX1.0Y2.0\nX3.0Y2.0\nM30\n";

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn files_and_folders_are_read_and_summarised() {
    let dir = tempfile::tempdir().unwrap();
    let copper = write(dir.path(), "board.gtl", COPPER);
    write(dir.path(), "board.drl", DRILL);
    let (code, text, err) = command(&["gerber-import", copper.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(
        text.contains("board.gtl: copper top (from the attributes)"),
        "{text}"
    );
    assert!(
        text.contains("1 flashes, 1 lines, 0 arcs, 0 regions"),
        "{text}"
    );
    let folder = dir.path().to_str().unwrap();
    let (code, text, _) = command(&["gerber-import", folder]);
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("board.drl: drill (from the file name)"),
        "{text}"
    );
    assert!(text.contains("2 holes, 0 slots"), "{text}");
    assert!(text.ends_with("2 files, 2 read, 0 failed\n"), "{text}");
    let (code, json, _) = command(&["gerber-import", "--json", folder]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["files"].as_array().unwrap().len(), 2);
    assert_eq!(value["failed"], 0);
}

#[test]
fn a_file_that_cannot_be_read_gives_exit_1_and_its_code_in_the_language_asked() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "good.gtl", COPPER);
    write(dir.path(), "bad.gbr", "%FSLAX46Y46*%\nX0Y0D03*\n");
    let folder = dir.path().to_str().unwrap();
    let (code, text, _) = command(&["gerber-import", folder]);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("error PHOENIX-GBR-005"), "{text}");
    assert!(text.ends_with("2 files, 1 read, 1 failed\n"), "{text}");
    let (_, german, _) = command(&["gerber-import", "--lang", "de", folder]);
    assert!(
        german.contains("Koordinatenformat oder Einheiten fehlen"),
        "{german}"
    );
    let (_, portuguese, _) = command(&["gerber-import", "--lang", "pt", folder]);
    assert!(
        portuguese.contains("O formato das coordenadas"),
        "{portuguese}"
    );
}

#[test]
fn arguments_settings_and_limits_that_cannot_be_used_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let copper = write(dir.path(), "a.gtl", COPPER);
    let other = write(dir.path(), "b.gtl", COPPER);
    let config = write(dir.path(), "limits.cfg", "max_files = 1\n");
    let broken = write(dir.path(), "broken.cfg", "nonsense\n");
    let (copper, other, config, broken) = (
        copper.as_str(),
        other.as_str(),
        config.as_str(),
        broken.as_str(),
    );
    for list in [
        &["gerber-import"][..],
        &["gerber-import", "--nonsense", copper],
        &["gerber-import", "--lang", "xx", copper],
        &["gerber-import", "--lang"],
        &["gerber-import", "--config", broken, copper],
        &["gerber-import", "--config", config, copper, other],
        &["gerber-import", "--profile", "fast", copper],
        &["gerber-import", "--roundtrip", copper, other],
        &["gerber-import", "--roundtrip", "absent.json"],
    ] {
        let (code, out, err) = command(list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty(), "{list:?}");
    }
    let (_, _, err) = command(&["gerber-import", "--config", config, copper, other]);
    assert!(err.contains("PHOENIX-GBR-003"), "{err}");
    let (_, _, err) = command(&["gerber-import", "--config", broken, copper]);
    assert!(err.contains("PHOENIX-GBR-001"), "{err}");
}

#[test]
fn the_summary_is_written_to_a_new_file_and_never_over_an_input_or_a_file_that_exists() {
    let dir = tempfile::tempdir().unwrap();
    let copper = write(dir.path(), "a.gtl", COPPER);
    let copper = copper.as_str();
    let out_path = dir.path().join("summary.json");
    let out_text = out_path.to_str().unwrap();
    let (code, _, err) = command(&["gerber-import", "--out", out_text, copper]);
    assert_eq!((code, err.as_str()), (0, ""));
    let written = fs::read_to_string(&out_path).unwrap();
    assert!(written.contains("\"flashes\": 1"), "{written}");
    let (code, _, err) = command(&["gerber-import", "--out", out_text, copper]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-GBR-018"), "{err}");
    assert_eq!(fs::read_to_string(&out_path).unwrap(), written);
    let (code, _, _) = command(&["gerber-import", "--out", copper, copper]);
    assert_eq!(code, 2);
    assert_eq!(fs::read_to_string(copper).unwrap(), COPPER);
}

#[test]
fn a_board_makes_the_round_trip_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = board().to_bytes().unwrap();
    let path = dir.path().join("board.json");
    fs::write(&path, &bytes).unwrap();
    let board_path = path.to_str().unwrap();
    let (code, text, err) = command(&["gerber-import", "--roundtrip", board_path]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(
        text.contains("round trip: 3 files read back, 0 differences"),
        "{text}"
    );
    assert!(text.contains("copper top (from the attributes)"), "{text}");
    let (code, json, _) = command(&["gerber-import", "--json", "--roundtrip", board_path]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["roundtrip"]["files"], 3);
    assert_eq!(
        value["roundtrip"]["differences"].as_array().unwrap().len(),
        0
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn panel_is_a_dry_run_unless_a_folder_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = board().to_bytes().unwrap();
    let path = dir.path().join("board.json");
    fs::write(&path, &bytes).unwrap();
    let board_path = path.to_str().unwrap();
    let (code, text, err) = command(&["panel", board_path]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(
        text.starts_with("panel 82.0 mm by 62.0 mm, 2 x 2 boards\n"),
        "{text}"
    );
    assert!(text.contains("joint: mouse bites, "), "{text}");
    assert!(text.contains("dry run: nothing written"), "{text}");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    let (code, text, _) = command(&["panel", "--columns", "3", "--rows", "1", board_path]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("3 x 1 boards"), "{text}");
    let (code, json, _) = command(&["panel", "--json", board_path]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["columns"], 2);
    assert_eq!(value["boards"].as_array().unwrap().len(), 4);
    assert_eq!(value["holes"]["tooling"], 4);
    let bare = dir.path().join("none.cfg");
    fs::write(&bare, "frame = none\n").unwrap();
    let (_, german, _) = command(&[
        "panel",
        "--lang",
        "de",
        "--config",
        bare.to_str().unwrap(),
        board_path,
    ]);
    assert!(
        german.contains("PHOENIX-PANEL-011: Marken übersprungen"),
        "{german}"
    );
}

#[test]
fn panel_files_are_written_to_a_new_folder_and_read_back_by_the_import() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = board().to_bytes().unwrap();
    let path = dir.path().join("board.json");
    fs::write(&path, &bytes).unwrap();
    let board_path = path.to_str().unwrap();
    let out = dir.path().join("panel");
    let out_text = out.to_str().unwrap();
    let (code, text, err) = command(&["panel", "--out", out_text, board_path]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(text.ends_with("written\n"), "{text}");
    let mut names: Vec<String> = fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "panel_drill_npth.drl",
            "panel_drill_pth.drl",
            "panel_fiducial_copper_top.gbr",
            "panel_fiducial_mask_top.gbr",
            "panel_profile.gbr"
        ]
    );
    // the import reads every file of the panel
    let (code, text, err) = command(&["gerber-import", out_text]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(
        text.contains("panel_profile.gbr: outline (from the attributes)"),
        "{text}"
    );
    assert!(
        text.contains("panel_drill_pth.drl: drill plated (from the attributes)"),
        "{text}"
    );
    assert!(
        text.contains("panel_drill_npth.drl: drill not plated (from the attributes)"),
        "{text}"
    );
    assert!(text.ends_with("5 files, 5 read, 0 failed\n"), "{text}");
    // four boards with one plated pad hole and one via each
    assert!(text.contains("8 holes, 0 slots"), "{text}");
    // a second run does not replace anything
    let before = fs::read(out.join("panel_profile.gbr")).unwrap();
    let (code, _, err) = command(&["panel", "--out", out_text, board_path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-PANEL-008"), "{err}");
    assert_eq!(fs::read(out.join("panel_profile.gbr")).unwrap(), before);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn a_v_score_panel_has_its_own_file_and_unusable_input_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = board().to_bytes().unwrap();
    let path = dir.path().join("board.json");
    fs::write(&path, &bytes).unwrap();
    let board_path = path.to_str().unwrap();
    let cfg = dir.path().join("v.cfg");
    fs::write(&cfg, "joint = v_score\ncolumns = 3\nrows = 1\n").unwrap();
    let cfg_text = cfg.to_str().unwrap();
    let out = dir.path().join("out");
    let out_text = out.to_str().unwrap();
    let (code, text, err) =
        command(&["panel", "--config", cfg_text, "--out", out_text, board_path]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(text.contains("joint: V-score, "), "{text}");
    assert!(out.join("panel_vscore.gbr").is_file());
    let broken = dir.path().join("broken.cfg");
    fs::write(&broken, "tab_width_nm = wide\n").unwrap();
    let thin = Board {
        outline: vec![pt(0, 0), pt(MM, 0)],
        ..Board::default()
    };
    let thin_path = dir.path().join("thin.json");
    fs::write(&thin_path, thin.to_bytes().unwrap()).unwrap();
    for list in [
        &["panel"][..],
        &["panel", "--nonsense", board_path],
        &["panel", "--lang", "xx", board_path],
        &["panel", "--columns", "many", board_path],
        &["panel", "--columns", "0", board_path],
        &["panel", "--config", broken.to_str().unwrap(), board_path],
        &["panel", "absent.json"],
        &["panel", thin_path.to_str().unwrap()],
        &["panel", board_path, board_path],
    ] {
        let (code, stdout, err) = command(list);
        assert_eq!((code, stdout.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty(), "{list:?}");
    }
}

const PROFILES: &str = include_str!("../../../config/export_profiles.json");

#[test]
fn the_round_trip_reads_back_every_level_of_x2() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board.json");
    fs::write(&path, board().to_bytes().unwrap()).unwrap();
    let board_path = path.to_str().unwrap();
    let profiles = write(dir.path(), "profiles.json", PROFILES);
    for name in ["generic", "x1", "x2-full"] {
        let (code, text, err) = command(&[
            "gerber-import",
            "--profiles",
            profiles.as_str(),
            "--profile",
            name,
            "--roundtrip",
            board_path,
        ]);
        assert_eq!((code, err.as_str()), (0, ""), "{name}: {text}");
        assert!(
            text.contains(" files read back, 0 differences"),
            "{name}: {text}"
        );
    }
}

#[test]
fn gerber_check_reports_the_codes_and_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "board.gtl", COPPER);
    let (code, text, err) = command(&["gerber-check", good.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{text}");
    assert!(text.contains("board.gtl: 3 finding(s)"), "{text}");
    assert!(
        text.contains("warning PHOENIX-GX2-003: Recommended file attribute missing: .Part"),
        "{text}"
    );
    assert!(
        text.ends_with(
            "1 files, 0 with errors
"
        ),
        "{text}"
    );
    let cut = write(
        dir.path(),
        "cut.gbr",
        "%TF.FileFunction,Copper,L1,Top*%
%FSLAX46Y46*%
%MOMM*%
",
    );
    let (code, text, _) = command(&["gerber-check", cut.as_str()]);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("error PHOENIX-GX2-001"), "{text}");
    let (_, portuguese, _) = command(&["gerber-check", "--lang", "pt", cut.as_str()]);
    assert!(
        portuguese.contains("O comando de fim falta ou não é o último"),
        "{portuguese}"
    );
    let (code, json, _) = command(&["gerber-check", "--json", dir.path().to_str().unwrap()]);
    assert_eq!(code, 1);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["files"].as_array().unwrap().len(), 2);
    assert_eq!(value["failed"], 1);
    assert_eq!(value["files"][0]["findings"][0]["code"], "PHOENIX-GX2-003");
}

#[test]
fn gerber_check_refuses_arguments_it_cannot_use() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "board.gtl", COPPER);
    for list in [
        &["gerber-check"][..],
        &["gerber-check", "--nonsense", good.as_str()],
        &["gerber-check", "--lang", "xx", good.as_str()],
        &["gerber-check", "--lang"],
        &["gerber-check", "--config"],
    ] {
        let (code, out, err) = command(list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty(), "{list:?}");
    }
    let (code, text, _) = command(&["gerber-check", "absent.gbr"]);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("PHOENIX-GBR-015"), "{text}");
}
