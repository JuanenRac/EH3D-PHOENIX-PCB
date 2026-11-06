// SPDX-License-Identifier: GPL-3.0-or-later
//! The flow of a small EAGLE project through the modules, one after another, with the numbers
//! worked out by hand: read, rule check of the schematic, netlist, simulation, rules of the board,
//! annotation back, panel, fabrication package, and the package read again and compared with the
//! project. Every stage has one assertion whose message names the stage, so a failure says where
//! the flow broke. The same project goes through a script at the end.
//!
//! The project: a source V1 of 10 V and two resistors in series, R1 of 1k and R2 of 3k, on a board
//! of 30 mm by 20 mm. The nets are IN (V1.1 and R1.1), MID (R1.2 and R2.1) and GND (R2.2 and
//! V1.2). By the divider formula the node MID has 10 V x 3 / (1 + 3) = 7.5 V. On the board V1 is at
//! x = 3 mm, R1 at x = 8 mm and R2 at x = 18 mm, all at y = 10 mm, and one track of 0.2 mm joins
//! R1.2 (x = 9) to R2.1 (x = 17). On the board R2 has the value 3k3, which the schematic does not
//! have: the annotation back has exactly that one change to plan.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_cli::run;
use eh3d_phoenix_eagle_board::{read_board, read_schematic};
use eh3d_phoenix_export::fabpack::verify_pack;
use eh3d_phoenix_export::zipfile::read_zip;
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_schematic::Schematic;

const SCH: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="91" name="Nets" color="2" fill="1" visible="yes" active="yes"/>
<layer number="94" name="Symbols" color="4" fill="1" visible="yes" active="yes"/>
</layers>
<schematic>
<libraries>
<library name="flow">
<symbols>
<symbol name="TWO">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
</symbol>
</symbols>
<devicesets>
<deviceset name="R" prefix="R">
<gates>
<gate name="G$1" symbol="TWO" x="0" y="0"/>
</gates>
<devices>
<device name="" package="R0805"/>
</devices>
</deviceset>
<deviceset name="V" prefix="V">
<gates>
<gate name="G$1" symbol="TWO" x="0" y="0"/>
</gates>
<devices>
<device name="" package="SRC"/>
</devices>
</deviceset>
</devicesets>
</library>
</libraries>
<parts>
<part name="V1" library="flow" deviceset="V" device="" value="10"/>
<part name="R1" library="flow" deviceset="R" device="" value="1k"/>
<part name="R2" library="flow" deviceset="R" device="" value="3k"/>
</parts>
<sheets>
<sheet>
<instances>
<instance part="V1" gate="G$1" x="10.16" y="30.48" rot="R90"/>
<instance part="R1" gate="G$1" x="30.48" y="30.48" rot="R90"/>
<instance part="R2" gate="G$1" x="30.48" y="15.24" rot="R90"/>
</instances>
<nets>
<net name="IN" class="0">
<segment>
<pinref part="V1" gate="G$1" pin="1"/>
<pinref part="R1" gate="G$1" pin="1"/>
</segment>
</net>
<net name="MID" class="0">
<segment>
<pinref part="R1" gate="G$1" pin="2"/>
<pinref part="R2" gate="G$1" pin="1"/>
</segment>
</net>
<net name="GND" class="0">
<segment>
<pinref part="R2" gate="G$1" pin="2"/>
<pinref part="V1" gate="G$1" pin="2"/>
</segment>
</net>
</nets>
</sheet>
</sheets>
</schematic>
</drawing>
</eagle>
"#;

const BRD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="16" name="Bottom" color="1" fill="1" visible="yes" active="yes"/>
<layer number="20" name="Dimension" color="15" fill="1" visible="yes" active="yes"/>
<layer number="21" name="tPlace" color="7" fill="1" visible="yes" active="yes"/>
<layer number="29" name="tStop" color="7" fill="3" visible="yes" active="yes"/>
<layer number="30" name="bStop" color="7" fill="6" visible="yes" active="yes"/>
<layer number="31" name="tCream" color="7" fill="4" visible="yes" active="yes"/>
<layer number="32" name="bCream" color="7" fill="5" visible="yes" active="yes"/>
</layers>
<board>
<plain>
<wire x1="0" y1="0" x2="30" y2="0" width="0" layer="20"/>
<wire x1="30" y1="0" x2="30" y2="20" width="0" layer="20"/>
<wire x1="30" y1="20" x2="0" y2="20" width="0" layer="20"/>
<wire x1="0" y1="20" x2="0" y2="0" width="0" layer="20"/>
</plain>
<libraries>
<library name="flow">
<packages>
<package name="R0805">
<smd name="1" x="-1" y="0" dx="1" dy="1.3" layer="1" roundness="0"/>
<smd name="2" x="1" y="0" dx="1" dy="1.3" layer="1" roundness="0"/>
</package>
<package name="SRC">
<smd name="1" x="-1" y="0" dx="1" dy="1.3" layer="1" roundness="0"/>
<smd name="2" x="1" y="0" dx="1" dy="1.3" layer="1" roundness="0"/>
</package>
</packages>
</library>
</libraries>
<attributes>
</attributes>
<elements>
<element name="V1" library="flow" package="SRC" value="10" x="3" y="10" rot="R0"/>
<element name="R1" library="flow" package="R0805" value="1k" x="8" y="10" rot="R0"/>
<element name="R2" library="flow" package="R0805" value="3k3" x="18" y="10" rot="R0"/>
</elements>
<signals>
<signal name="IN" class="0">
<contactref element="V1" pad="1"/>
<contactref element="R1" pad="1"/>
</signal>
<signal name="MID" class="0">
<contactref element="R1" pad="2"/>
<contactref element="R2" pad="1"/>
<wire x1="9" y1="10" x2="17" y2="10" width="0.2" layer="1"/>
</signal>
<signal name="GND" class="0">
<contactref element="R2" pad="2"/>
<contactref element="V1" pad="2"/>
</signal>
</signals>
</board>
</drawing>
</eagle>
"#;

const RELAXED: &str = r#"{"format": "phoenix-rules/1", "rules": {"min_track_width_nm": 200000}}"#;
const STRICT: &str = r#"{"format": "phoenix-rules/1", "rules": {"min_track_width_nm": 300000}}"#;
const PANEL: &str = "columns = 2\nrows = 1\njoint = v_score\ngap_nm = 0\nframe = none\n";

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn put(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn at(dir: &Path, name: &str) -> String {
    dir.join(name).to_str().unwrap().to_owned()
}

/// The fields of the rows of a CSV file that start with a designator, by designator.
fn rows_by_designator(csv: &str) -> BTreeMap<String, Vec<String>> {
    csv.lines()
        .map(|line| line.split(',').map(str::to_owned).collect::<Vec<String>>())
        .filter(|fields| fields.len() > 4)
        .map(|fields| (fields[0].clone(), fields))
        .collect()
}

/// The number in a field of a row.
fn number(fields: &[String], index: usize) -> f64 {
    fields[index].trim().parse::<f64>().unwrap()
}

#[test]
fn an_eagle_project_goes_from_the_files_to_the_package_and_back_to_the_same_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let sch = put(dir.path(), "flow.sch", SCH);
    let brd = put(dir.path(), "flow.brd", BRD);
    let relaxed = put(dir.path(), "relaxed.json", RELAXED);
    let strict = put(dir.path(), "strict.json", STRICT);
    let panel_cfg = put(dir.path(), "panel.cfg", PANEL);

    // stage 1, read: writing the tree again changes nothing, for the board and for the schematic
    let (code_b, out_b, err_b) = call(&["eagle-check", &brd]);
    let (code_s, out_s, err_s) = call(&["eagle-check", &sch]);
    assert!(
        code_b == 0
            && code_s == 0
            && out_b.contains("rewrite of the tree: identical")
            && out_s.contains("rewrite of the tree: identical"),
        "stage 1 (read): {code_b} {code_s}\n{out_b}{err_b}\n{out_s}{err_s}"
    );

    // stage 2, electrical rule check: every pin is on a net of two pins, so nothing is proven
    let (code, out, err) = call(&["erc", "--json", &sch]);
    let value: serde_json::Value = serde_json::from_str(out.trim()).unwrap_or_default();
    assert!(
        code == 0 && value["passes"] == true,
        "stage 2 (erc): {code}\n{out}{err}"
    );

    // stage 3, netlist: the three elements with the nets in the notation of the simulator
    let (code, netlist, err) = call(&["netlist", &sch]);
    assert!(
        code == 0
            && netlist.contains("R1 in mid 1e3\n")
            && netlist.contains("R2 mid 0 3e3\n")
            && netlist.contains("V1 in 0 1e1\n"),
        "stage 3 (netlist): {code}\n{netlist}{err}"
    );

    // stage 4, simulation: the divider formula gives 7.5 V at the node MID
    let (code, op, err) = call(&["sim", "--schematic", &sch, "--op", "--probe", "v(mid)"]);
    assert!(
        code == 0 && op.contains("v(mid),7.5e0"),
        "stage 4 (simulation): {code}\n{op}{err}"
    );

    // stage 5, rules of the board: the track of 0.2 mm passes a limit of 0.2 mm and breaks 0.3 mm
    let (code_ok, out_ok, _) = call(&["check", "--rules", &relaxed, &brd]);
    let (code_bad, out_bad, _) = call(&["check", "--rules", &strict, &brd]);
    assert!(
        code_ok == 0
            && out_ok.contains("Confirmed: 0")
            && code_bad == 1
            && out_bad.contains("Confirmed: 1")
            && out_bad.contains("PHOENIX-CHECK-001"),
        "stage 5 (rules): {code_ok} {code_bad}\n{out_ok}\n{out_bad}"
    );

    // stage 6, annotation back: the parts of the files carry no link, so the test makes the one that
    // the transfer to the board makes (the component holds the id of its part) and writes both
    // documents as the JSON of the model; the plan then has the single change R2: 3k -> 3k3
    let mut counter = 0u32;
    let mut new_id = move || {
        counter += 1;
        ObjectId::from_parts(
            1_700_000_000_000,
            (counter & 0xfff) as u16,
            u64::from(counter),
        )
        .unwrap()
    };
    let mut board = read_board(BRD, &mut new_id).unwrap().board;
    let footprints: BTreeMap<String, ObjectId> = board
        .footprints
        .iter()
        .map(|(id, footprint)| (footprint.name.clone(), *id))
        .collect();
    let schematic: Schematic = read_schematic(SCH, &footprints, &mut new_id)
        .unwrap()
        .schematic;
    let part_ids: BTreeMap<String, ObjectId> = schematic
        .parts
        .iter()
        .map(|(id, part)| (part.refdes.clone(), *id))
        .collect();
    for component in board.components.values_mut() {
        component.symbol = part_ids.get(&component.refdes).copied();
    }
    let sch_json = at(dir.path(), "flow_sch.json");
    let brd_json = at(dir.path(), "flow_brd.json");
    fs::write(&sch_json, schematic.to_bytes().unwrap()).unwrap();
    fs::write(&brd_json, board.to_bytes().unwrap()).unwrap();
    let (plan_code, plan, _) = call(&["annotate", "--json", &sch_json, &brd_json]);
    let plan_value: serde_json::Value = serde_json::from_str(plan.trim()).unwrap_or_default();
    let digest = plan_value["digest"].as_str().unwrap_or("").to_owned();
    let annotated = at(dir.path(), "flow_sch_annotated.json");
    let (apply_code, _, apply_err) = call(&[
        "annotate",
        "--apply",
        "--confirm",
        &digest,
        "--out",
        &annotated,
        &sch_json,
        &brd_json,
    ]);
    let after = fs::read(&annotated)
        .ok()
        .and_then(|bytes| Schematic::from_bytes(&bytes).ok());
    let value_of = |s: &Schematic, refdes: &str| {
        s.parts
            .values()
            .find(|p| p.refdes == refdes)
            .map(|p| p.value.clone())
    };
    let (again_code, again, _) = call(&["annotate", &annotated, &brd_json]);
    assert!(
        plan_code == 1
            && plan_value["applicable"] == 1
            && apply_code == 0
            && after
                .as_ref()
                .is_some_and(|s| value_of(s, "R2").as_deref() == Some("3k3")
                    && value_of(s, "R1").as_deref() == Some("1k")
                    && value_of(s, "V1").as_deref() == Some("10"))
            && again_code == 0
            && !again.contains("[applicable]"),
        "stage 6 (annotation back): {plan_code} {apply_code} {apply_err}\n{plan}\n{again}"
    );

    // stage 7, panel: two boards side by side make 60 mm by 20 mm, and the files are written once
    let panel_dir = at(dir.path(), "panel");
    let (code, out, err) = call(&["panel", "--config", &panel_cfg, "--out", &panel_dir, &brd]);
    let (again_code, _, again_err) =
        call(&["panel", "--config", &panel_cfg, "--out", &panel_dir, &brd]);
    assert!(
        code == 0
            && out.starts_with("panel 60.0 mm by 20.0 mm, 2 x 1 boards")
            && again_code == 2
            && again_err.contains("PHOENIX-PANEL-008"),
        "stage 7 (panel): {code} {again_code}\n{out}{err}{again_err}"
    );

    // stage 8, fabrication: the package is a zip file, the same bytes every time
    let one = at(dir.path(), "one.zip");
    let two = at(dir.path(), "two.zip");
    let (code_one, out_one, err_one) = call(&["fabpack", "--out", &one, &brd]);
    let (code_two, _, _) = call(&["fabpack", "--out", &two, &brd]);
    let bytes = fs::read(&one).unwrap_or_default();
    assert!(
        code_one == 0
            && code_two == 0
            && !bytes.is_empty()
            && bytes == fs::read(&two).unwrap_or_default(),
        "stage 8 (fabrication): {code_one} {code_two}\n{out_one}{err_one}"
    );

    // stage 9, the package read again: it verifies against its own manifest and holds the layers
    let report = verify_pack(&bytes);
    let entries = read_zip(&bytes).unwrap_or_default();
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        report.as_ref().is_ok_and(|r| r.profile == "generic")
            && names.contains(&"manifest.json")
            && names.contains(&"bom/bom_board.csv")
            && names.contains(&"pnp/pnp_top.csv")
            && names.iter().any(|n| n.starts_with("gerber/")),
        "stage 9 (read the package): {:?}\n{names:?}",
        report.err()
    );

    // stage 10, comparison with the original: the parts, the values and the distances of the
    // placement file are the ones of the board, worked out by hand (R1 - V1 = 5 mm, R2 - R1 =
    // 10 mm, all on one row)
    let text_of = |name: &str| {
        entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    };
    let placed = rows_by_designator(&text_of("pnp/pnp_top.csv"));
    let bom = text_of("bom/bom_board.csv");
    let positions_ok = ["V1", "R1", "R2"].iter().all(|r| placed.contains_key(*r)) && {
        let x = |r: &str| number(&placed[r], 3);
        let y = |r: &str| number(&placed[r], 4);
        (x("R1") - x("V1") - 5.0).abs() < 1e-3
            && (x("R2") - x("R1") - 10.0).abs() < 1e-3
            && (y("R1") - y("V1")).abs() < 1e-3
            && (y("R2") - y("R1")).abs() < 1e-3
    };
    let values_ok = placed.get("R1").is_some_and(|f| f[1] == "1k")
        && placed.get("R2").is_some_and(|f| f[1] == "3k3")
        && placed.get("V1").is_some_and(|f| f[1] == "10");
    assert!(
        positions_ok && values_ok && bom.contains("R1") && bom.contains("R2") && bom.contains("V1"),
        "stage 10 (compare with the original): {placed:?}\n{bom}"
    );
}

#[test]
fn a_script_reads_the_same_project_and_changes_a_copy_only() {
    let dir = tempfile::tempdir().unwrap();
    let brd = put(dir.path(), "flow.brd", BRD);
    let list = put(
        dir.path(),
        "list.phs",
        "phoenix-script/2 board\n\
         let n = 0\n\
         for part in sort_refdes(select(\"component\", \"R*\")) {\n\
         print(part.refdes, part.value)\n\
         n = n + 1\n\
         }\n\
         print(n, \"resistors\")\n",
    );
    let before = fs::read(&brd).unwrap();

    // stage 1, check: the script is read without running it
    let (code, out, err) = call(&["script", "check", &list]);
    assert!(code == 0, "stage 1 (script check): {code}\n{out}{err}");

    // stage 2, run: it lists the two resistors and not the source, and writes nothing
    let (code, out, err) = call(&["script", "run", &list, &brd]);
    assert!(
        code == 0 && out.contains("R1 1k") && out.contains("R2 3k3") && out.contains("2 resistors"),
        "stage 2 (script run): {code}\n{out}{err}"
    );

    // stage 3, an edit goes to a new file only; the input keeps its bytes
    let edit = put(
        dir.path(),
        "edit.phs",
        "phoenix-script/2 board\nlet t = route(\"GND\", \"Top\", 0.2mm, [1mm, 5mm, 6mm, 5mm])\n",
    );
    let target = at(dir.path(), "result.json");
    let (code, out, err) = call(&["script", "run", "--out", &target, &edit, &brd]);
    let result = fs::read(&target)
        .ok()
        .and_then(|bytes| Board::from_bytes(&bytes).ok());
    assert!(
        code == 0 && result.as_ref().is_some_and(|b| b.tracks.len() == 2),
        "stage 3 (script edit): {code}\n{out}{err}"
    );
    assert_eq!(
        fs::read(&brd).unwrap(),
        before,
        "stage 4 (the input is untouched)"
    );
}
