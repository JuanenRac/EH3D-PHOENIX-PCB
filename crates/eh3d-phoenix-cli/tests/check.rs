// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `check` and `connectivity` on EAGLE files written by the tool itself.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Track};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_eagle_board::{write_board, write_schematic};
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use eh3d_phoenix_schematic::{Part, Pin, Schematic, Symbol};

fn id(n: u32) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board with one track 0.10 mm wide.
fn board() -> Board {
    let mut b = Board::default();
    for (n, name) in [(1, "Top"), (2, "Bottom")] {
        b.layers.insert(
            id(n),
            Layer {
                name: name.into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
    }
    b.copper_stack = vec![id(1), id(2)];
    b.outline = vec![
        pt(0, 0),
        pt(20_000_000, 0),
        pt(20_000_000, 20_000_000),
        pt(0, 20_000_000),
    ];
    b.nets.insert(id(10), Net { name: "A".into() });
    b.tracks.insert(
        id(20),
        Track {
            net: id(10),
            layer: id(1),
            width: Nm::new(100_000).unwrap(),
            path: vec![Segment::Line {
                start: pt(2_000_000, 2_000_000),
                end: pt(8_000_000, 2_000_000),
            }],
        },
    );
    b
}

fn schematic() -> Schematic {
    let mut s = Schematic::default();
    let mut pins = BTreeMap::new();
    pins.insert(
        id(2),
        Pin {
            number: "1".into(),
            at: None,
        },
    );
    s.symbols.insert(
        id(1),
        Symbol {
            name: "R".into(),
            pins,
        },
    );
    s.parts.insert(
        id(3),
        Part {
            refdes: "R1".into(),
            value: "1k".into(),
            symbol: id(1),
            footprint: None,
            placement: None,
        },
    );
    s
}

const RULES: &str = r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":150000}}"#;

struct Files {
    dir: tempfile::TempDir,
}

impl Files {
    fn new() -> Files {
        Files {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    fn write(&self, name: &str, text: &str) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        path.to_str().unwrap().to_owned()
    }
}

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

#[test]
fn check_finds_the_narrow_track_and_exits_1() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("Confirmed: 1"), "{out}");
    assert!(out.contains("PHOENIX-CHECK-001"));
    assert!(out.contains("measured 100000 nm") && out.contains("limit 150000 nm"));
    assert!(out.contains("Inconclusive: 0"));
    // the same file twice gives the same text
    assert_eq!(
        call(&["check", "--rules", rules.as_str(), brd.as_str()]).1,
        out
    );
}

#[test]
fn check_prints_json_and_translated_text() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, _) = call(&["check", "--json", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!(code, 1);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["passes"], false);
    assert_eq!(json["confirmed"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["confirmed"][0]["measured_nm"], 100_000);
    let (_, es, _) = call(&[
        "check",
        "--lang",
        "es",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert!(es.contains("Pista más estrecha que el mínimo"), "{es}");
    let (_, pt, _) = call(&[
        "check",
        "--lang",
        "pt-PT",
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert!(pt.contains("Pista mais estreita do que o mínimo"), "{pt}");
}

#[test]
fn check_exits_0_when_the_limit_is_met() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write(
        "r.json",
        r#"{"format":"phoenix-rules/1","rules":{"min_track_width_nm":100000}}"#,
    );
    let (code, out, _) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert_eq!((code, out.contains("Confirmed: 0")), (0, true), "{out}");
}

#[test]
fn time_writes_the_seconds_to_the_error_stream_and_leaves_the_report_alone() {
    let f = Files::new();
    let b = board();
    let brd = f.write("b.brd", &write_board(&b).text);
    let sch = f.write("s.sch", &write_schematic(&b, &schematic()).text);
    let rules = f.write("r.json", RULES);
    let (code, out, err) = call(&["check", "--rules", rules.as_str(), brd.as_str()]);
    assert!(!err.contains("time: "), "{err}");
    let (timed_code, timed_out, timed_err) =
        call(&["check", "--time", "--rules", rules.as_str(), brd.as_str()]);
    // the exit code and the report are the ones of the run without it
    assert_eq!((timed_code, timed_out.as_str()), (code, out.as_str()));
    // one line: the word, the seconds with three decimals, and the unit
    let line = timed_err.trim_end();
    let seconds = line
        .strip_prefix("time: ")
        .and_then(|rest| rest.strip_suffix(" s"));
    let parsed = seconds.and_then(|text| text.parse::<f64>().ok());
    assert!(
        parsed.is_some_and(|value| value >= 0.0) && !line.contains('\n'),
        "{timed_err}"
    );
    assert_eq!(
        seconds
            .and_then(|text| text.split_once('.'))
            .map(|(_, f)| f.len()),
        Some(3)
    );
    // the same for the command that compares the schematic with the board
    let (code, _, err) = call(&["connectivity", "--time", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.starts_with("time: ") && err.trim_end().ends_with(" s"),
        "{err}"
    );
}

#[test]
fn check_takes_the_limits_from_the_design_rules_of_the_board() {
    let f = Files::new();
    let text = write_board(&board()).text;
    let plain = f.write("plain.brd", &text);
    let (code, _, err) = call(&["check", plain.as_str()]);
    assert_eq!(code, 2);
    assert!(
        err.contains("design rules") && err.contains("--rules"),
        "{err}"
    );
    let with_rules = text.replacen(
        "<board>",
        "<board><designrules name=\"x\"><param name=\"msWidth\" value=\"0.15mm\"/></designrules>",
        1,
    );
    assert_ne!(with_rules, text);
    let brd = f.write("rules.brd", &with_rules);
    let (code, out, _) = call(&["check", brd.as_str()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("PHOENIX-CHECK-001"));
}

#[test]
fn bad_arguments_and_bad_files_exit_2() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let missing = f.dir.path().join("absent.brd");
    for list in [
        vec!["check"],
        vec!["check", "--lang", "xx", brd.as_str()],
        vec!["check", "--bogus", brd.as_str()],
        vec!["check", "--rules"],
        vec!["connectivity", brd.as_str()],
    ] {
        let (code, out, err) = call(&list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(!err.is_empty());
    }
    let (code, _, err) = call(&[
        "check",
        "--rules",
        rules.as_str(),
        missing.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("cannot read"));
    let broken = f.write("broken.brd", "not xml");
    assert_eq!(
        call(&["check", "--rules", rules.as_str(), broken.as_str()]).0,
        2
    );
    let bad_rules = f.write("bad.json", "{}");
    assert_eq!(
        call(&["check", "--rules", bad_rules.as_str(), brd.as_str()]).0,
        2
    );
}

#[test]
fn connectivity_reads_both_files_and_prints_one_report() {
    let f = Files::new();
    let b = board();
    let brd = f.write("b.brd", &write_board(&b).text);
    let sch = f.write("s.sch", &write_schematic(&b, &schematic()).text);
    let (code, out, err) = call(&["connectivity", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out} {err}");
    assert!(
        out.contains("Confirmed: 0") && out.contains("Inconclusive: 0"),
        "{out}"
    );
    let (code, json, _) = call(&["connectivity", "--json", sch.as_str(), brd.as_str()]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-connectivity/1");
    let rules = f.write(
        "r.json",
        r#"{"format":"phoenix-rules/1","rules":{"connect_tolerance_nm":1000}}"#,
    );
    assert_eq!(
        call(&[
            "connectivity",
            "--rules",
            rules.as_str(),
            sch.as_str(),
            brd.as_str()
        ])
        .0,
        0
    );
}

#[test]
fn check_writes_the_findings_as_csv_and_html_and_never_replaces_a_file() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let rules = f.write("r.json", RULES);
    let csv = f.dir.path().join("report.csv");
    let html = f.dir.path().join("report.html");
    let (csv_name, html_name) = (csv.to_str().unwrap(), html.to_str().unwrap());
    let (code, out, _) = call(&[
        "check",
        "--rules",
        rules.as_str(),
        "--csv",
        csv_name,
        "--html",
        html_name,
        "--lang",
        "de",
        brd.as_str(),
    ]);
    assert_eq!(code, 1, "{out}");
    // one proven finding: the track of 100000 nm against a limit of 150000 nm
    let table = std::fs::read_to_string(&csv).unwrap();
    let lines: Vec<&str> = table.lines().collect();
    assert_eq!(lines.len(), 2, "{table}");
    assert!(lines[0].starts_with("list,category,severity,code,title,"));
    assert!(
        lines[1].starts_with("confirmed,size,error,PHOENIX-CHECK-001,"),
        "{table}"
    );
    assert!(lines[1].contains(",100000,150000,"), "{table}");
    let page = std::fs::read_to_string(&html).unwrap();
    assert!(page.contains("<html lang=\"de\">"));
    assert!(page.contains("<h2>Breiten und Größen</h2>"));
    assert!(page.contains("<td>100000 nm</td><td>150000 nm</td>"));
    // a second run over the same names replaces nothing and writes nothing else
    let (code, _, err) = call(&[
        "check",
        "--rules",
        rules.as_str(),
        "--csv",
        csv_name,
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("never replaced"), "{err}");
    assert_eq!(std::fs::read_to_string(&csv).unwrap(), table);
    // the same name for both outputs is refused before anything is written
    let both = f.dir.path().join("both.out");
    let both = both.to_str().unwrap();
    let (code, _, err) = call(&[
        "check",
        "--rules",
        rules.as_str(),
        "--csv",
        both,
        "--html",
        both,
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("two outputs"), "{err}");
    assert!(!std::path::Path::new(both).exists());
}

#[test]
fn check_against_a_capability_profile_judges_the_limits_and_the_layers() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    // the track of 0.10 mm is below 0.15 mm and the two copper layers are above one
    let profile = f.write(
        "maker.cfg",
        "verified = true\nmin_track_width_nm = 150000\nmax_copper_layers = 1\n",
    );
    let (code, out, err) = call(&["check", "--dfm", profile.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("PHOENIX-CHECK-001"), "{out}");
    assert!(out.contains("PHOENIX-DFM-001"), "{out}");
    assert!(
        out.contains("2 copper layers, the profile (unnamed) offers 1"),
        "{out}"
    );
    assert!(!out.contains("PHOENIX-DFM-006"), "{out}");
    // a profile that is not verified says so, and a board that fits it passes
    let open = f.write("open.cfg", "name = example\nmin_track_width_nm = 50000\n");
    let (code, out, _) = call(&["check", "--dfm", open.as_str(), brd.as_str()]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("PHOENIX-DFM-006"), "{out}");
    // the rules of the board are replaced by the profile, not added to another file of rules
    let rules = f.write("r.json", RULES);
    let (code, _, err) = call(&[
        "check",
        "--dfm",
        open.as_str(),
        "--rules",
        rules.as_str(),
        brd.as_str(),
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("only one of them"), "{err}");
    // a profile with a bad line is refused with its line
    let bad = f.write("bad.cfg", "\nmax_holes = many\n");
    let (code, _, err) = call(&["check", "--dfm", bad.as_str(), brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("line 2"), "{err}");
}

#[test]
fn check_with_the_profile_of_a_maker_that_comes_with_the_program() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    // the track of 0.10 mm is the narrowest both makers build, so only the note remains
    for name in ["jlcpcb", "pcbway", "JLCPCB"] {
        let (code, out, err) = call(&["check", "--dfm-profile", name, brd.as_str()]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert!(out.contains("PHOENIX-DFM-006"), "{name}: {out}");
        for other in ["001", "002", "003", "004", "005"] {
            assert!(
                !out.contains(&format!("PHOENIX-DFM-{other}")),
                "{name}: {out}"
            );
        }
    }
    // a name that is not one of them lists the ones that are
    let (code, _, err) = call(&["check", "--dfm-profile", "nobody", brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("no profile is called nobody"), "{err}");
    assert!(err.contains("example, jlcpcb, pcbway"), "{err}");
    // it cannot be mixed with the other ways to give the limits
    let open = f.write("open.cfg", "name = example\n");
    let rules = f.write("r.json", RULES);
    for other in [["--dfm", open.as_str()], ["--rules", rules.as_str()]] {
        let (code, _, err) = call(&[
            "check",
            "--dfm-profile",
            "jlcpcb",
            other[0],
            other[1],
            brd.as_str(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("only one of them"), "{err}");
    }
}

#[test]
fn dfm_compare_puts_the_profiles_side_by_side_and_exits_with_whether_one_accepts_the_board() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    // the track of 0.10 mm and the two copper layers break the first profile, not the second
    let strict = f.write(
        "strict.cfg",
        "verified = true\nmin_track_width_nm = 150000\nmax_copper_layers = 1\n",
    );
    let open = f.write("open.cfg", "name = example\nmin_track_width_nm = 50000\n");
    let list = format!("{strict},{open}");
    let (code, out, err) = call(&["check", "--dfm-compare", list.as_str(), brd.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.starts_with("Profiles compared: 2\n"), "{out}");
    assert!(
        out.contains("- strict.cfg (verified: yes): passes: no"),
        "{out}"
    );
    assert!(
        out.contains("- open.cfg (verified: no): passes: yes"),
        "{out}"
    );
    assert!(
        out.contains(
            "PHOENIX-CHECK-001 Track narrower than the minimum: strict.cfg 1 | open.cfg 0"
        ),
        "{out}"
    );
    assert!(out.contains("Only on some of the profiles:\n"), "{out}");
    assert!(out.ends_with("Fewest errors: open.cfg\n"), "{out}");
    // the same bytes every time, in another language and as JSON
    assert_eq!(
        call(&["check", "--dfm-compare", list.as_str(), brd.as_str()]).1,
        out
    );
    let (_, pt, _) = call(&[
        "check",
        "--dfm-compare",
        list.as_str(),
        "--lang",
        "pt",
        brd.as_str(),
    ]);
    assert!(pt.starts_with("Perfis comparados: 2\n"), "{pt}");
    let (code, json, _) = call(&[
        "check",
        "--dfm-compare",
        list.as_str(),
        "--json",
        brd.as_str(),
    ]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["schema"], "phoenix-dfm-compare/1");
    assert_eq!(value["any_passes"], true);
    assert_eq!(value["fewest_errors"][0], "open.cfg");
    assert_eq!(value["profiles"][0]["name"], "strict.cfg");
    assert_eq!(value["profiles"][0]["passes"], false);
    // when no profile accepts the board the exit code is 1
    let strict_too = f.write(
        "strict2.cfg",
        "min_track_width_nm = 150000\nmax_copper_layers = 1\n",
    );
    let both = format!("{strict},{strict_too}");
    let (code, out, _) = call(&["check", "--dfm-compare", both.as_str(), brd.as_str()]);
    assert_eq!(code, 1, "{out}");
}

#[test]
fn dfm_compare_takes_the_profiles_that_come_with_the_program() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    // both accept the track of 0.10 mm and both say that they are not verified
    let (code, out, err) = call(&["check", "--dfm-compare", "jlcpcb, pcbway", brd.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains("- jlcpcb (verified: no): passes: yes"),
        "{out}"
    );
    assert!(
        out.contains("- pcbway (verified: no): passes: yes"),
        "{out}"
    );
    assert!(out.contains("jlcpcb 1 | pcbway 1"), "{out}");
    // what is not a profile, a list that is too short and a profile twice are refused
    let (code, _, err) = call(&["check", "--dfm-compare", "jlcpcb,nobody", brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("no profile is called nobody"), "{err}");
    let (code, _, err) = call(&["check", "--dfm-compare", "jlcpcb", brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("from 2 to 8 profiles"), "{err}");
    let (code, _, err) = call(&["check", "--dfm-compare", "jlcpcb,,pcbway", brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("from 2 to 8 profiles"), "{err}");
    let (code, _, err) = call(&["check", "--dfm-compare", "jlcpcb,JLCPCB.cfg", brd.as_str()]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = call(&["check", "--dfm-compare", "pcbway,pcbway", brd.as_str()]);
    assert_eq!(code, 2);
    assert!(err.contains("is given twice"), "{err}");
    // it does not mix with the other ways to give the limits, and it is not for connectivity
    let rules = f.write("r.json", RULES);
    for other in [
        ["--rules", rules.as_str()],
        ["--dfm-profile", "jlcpcb"],
        ["--csv", "x.csv"],
    ] {
        let (code, _, err) = call(&[
            "check",
            "--dfm-compare",
            "jlcpcb,pcbway",
            other[0],
            other[1],
            brd.as_str(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("cannot be given with"), "{err}");
    }
    let (code, _, err) = call(&["check", "--dfm-compare"]);
    assert_eq!(code, 2, "{err}");
}

const DRU_LIST_A: &str = "jlcpcb,pcbway-dru,oshpark-2l-dru,oshpark-4l-dru,eurocircuits-2l-dru,pcbpool-dru,allpcb-2l-dru,generic-8mil-dru";
const DRU_LIST_B: &str = "jlcpcb,eurocircuits-1l-dru,eurocircuits-4l-dru,eurocircuits-6l-dru,allpcb-1l-dru,allpcb-4l-dru,allpcb-6l-dru,generic-10mil-dru";

#[test]
fn a_profile_made_from_design_rules_judges_the_track_of_the_board() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    // the track of 0.10 mm is narrower than the 6 mil of the design rules of PCBWay
    let (code, out, err) = call(&["check", "--dfm-profile", "pcbway-dru", brd.as_str()]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("PHOENIX-CHECK-001"), "{out}");
    assert!(out.contains("PHOENIX-DFM-006"), "{out}");
    // NCAB gives 0.05 mm, so the track passes
    let (code, out, err) = call(&["check", "--dfm-profile", "ncab", brd.as_str()]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("PHOENIX-CHECK-001"), "{out}");
}

#[test]
fn the_design_rule_profiles_of_the_makers_are_compared_side_by_side() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let (code, out, err) = call(&["check", "--dfm-compare", DRU_LIST_A, brd.as_str()]);
    // jlcpcb takes the track of 0.10 mm, so one profile accepts the board
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.starts_with("Profiles compared: 8\n"), "{out}");
    assert!(
        out.contains("- jlcpcb (verified: no): passes: yes"),
        "{out}"
    );
    for name in [
        "pcbway-dru",
        "oshpark-2l-dru",
        "oshpark-4l-dru",
        "eurocircuits-2l-dru",
        "pcbpool-dru",
        "allpcb-2l-dru",
        "generic-8mil-dru",
    ] {
        assert!(
            out.contains(&format!("- {name} (verified: no): passes: no")),
            "{name}: {out}"
        );
    }
    assert!(
        out.contains(
            "Track narrower than the minimum: jlcpcb 0 | pcbway-dru 1 | oshpark-2l-dru 1 | oshpark-4l-dru 1 | eurocircuits-2l-dru 1 | pcbpool-dru 1 | allpcb-2l-dru 1 | generic-8mil-dru 1"
        ),
        "{out}"
    );
    // the profiles of one layer find the board of two layers too big for them
    let (code, out, err) = call(&["check", "--dfm-compare", DRU_LIST_B, brd.as_str()]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(
        out.contains(
            "jlcpcb 0 | eurocircuits-1l-dru 1 | eurocircuits-4l-dru 0 | eurocircuits-6l-dru 0 | allpcb-1l-dru 1 | allpcb-4l-dru 0 | allpcb-6l-dru 0 | generic-10mil-dru 0"
        ),
        "{out}"
    );
    assert!(out.contains("PHOENIX-DFM-001"), "{out}");
}

#[test]
fn the_comparison_of_the_makers_is_written_in_the_eight_languages() {
    let f = Files::new();
    let brd = f.write("b.brd", &write_board(&board()).text);
    let mut reports = std::collections::BTreeSet::new();
    for lang in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
        let (code, out, err) = call(&[
            "check",
            "--dfm-compare",
            DRU_LIST_A,
            "--lang",
            lang,
            brd.as_str(),
        ]);
        assert_eq!((code, err.as_str()), (0, ""), "{lang}: {out}");
        // the names of the profiles are never translated
        for name in ["pcbway-dru", "eurocircuits-2l-dru", "generic-8mil-dru"] {
            assert!(out.contains(name), "{lang} {name}: {out}");
        }
        reports.insert(out);
    }
    assert_eq!(reports.len(), 8, "every language has its own words");
    let (_, pt, _) = call(&[
        "check",
        "--dfm-compare",
        DRU_LIST_A,
        "--lang",
        "pt",
        brd.as_str(),
    ]);
    assert!(pt.starts_with("Perfis comparados: 8\n"), "{pt}");
}
