// SPDX-License-Identifier: GPL-3.0-or-later
//! The flow of a small KiCad project through the modules, one after another, with the numbers
//! worked out by hand: read, rule check of the schematic, rules of the board, written back as KiCad
//! files and read again, panel, fabrication package, and the package read again and compared with
//! the project. Every stage has one assertion whose message names the stage.
//!
//! The project: a source V1 and two resistors R1 and R2 in series. The nets are /IN (V1.1 and
//! R1.1), /MID (R1.2 and R2.1) and /GND (R2.2 and V1.2). In the schematic (y down, millimetres) R1
//! is at (100, 50), R2 at (100, 57.62) and V1 at (120, 54); the pins of the symbol are 3.81 mm
//! above and below its origin, so R1.2 and R2.1 meet at (100, 53.81). On the board, 30 mm by 20 mm,
//! every footprint has a round through-hole pad 1 at its origin and pad 2 five millimetres to the
//! right: R1 is at (5, 5), R2 at (15, 5) and V1 at (5, 15), and one track of 0.25 mm joins R1.2
//! (10, 5) to R2.1 (15, 5). So the placement has R2 ten millimetres to the right of R1 and V1 ten
//! millimetres below it.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use eh3d_phoenix_cli::run;
use eh3d_phoenix_export::fabpack::verify_pack;
use eh3d_phoenix_export::zipfile::read_zip;

const PRO: &str = r#"{
  "meta": { "filename": "flow.kicad_pro", "version": 1 }
}
"#;

const SCH: &str = r#"(kicad_sch (version 20230121) (generator eeschema)
  (uuid "00000000-0000-0000-0000-0000000000a1")
  (paper "A4")
  (lib_symbols
    (symbol "Device:R" (pin_numbers hide) (pin_names (offset 0)) (in_bom yes) (on_board yes)
      (property "Reference" "R" (at 2.032 0 90) (effects (font (size 1.27 1.27))))
      (property "Value" "R" (at 0 0 90) (effects (font (size 1.27 1.27))))
      (symbol "R_0_1" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
      (symbol "R_1_1"
        (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
        (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27))))))))
  (wire (pts (xy 100 46.19) (xy 120 46.19)) (stroke (width 0) (type default)) (uuid "00000000-0000-0000-0000-0000000000b1"))
  (wire (pts (xy 120 46.19) (xy 120 50.19)) (stroke (width 0) (type default)) (uuid "00000000-0000-0000-0000-0000000000b2"))
  (wire (pts (xy 100 53.81) (xy 105 53.81)) (stroke (width 0) (type default)) (uuid "00000000-0000-0000-0000-0000000000b3"))
  (wire (pts (xy 100 61.43) (xy 120 61.43)) (stroke (width 0) (type default)) (uuid "00000000-0000-0000-0000-0000000000b4"))
  (wire (pts (xy 120 61.43) (xy 120 57.81)) (stroke (width 0) (type default)) (uuid "00000000-0000-0000-0000-0000000000b5"))
  (label "IN" (at 110 46.19 0) (effects (font (size 1.27 1.27))) (uuid "00000000-0000-0000-0000-0000000000c1"))
  (label "MID" (at 102.5 53.81 0) (effects (font (size 1.27 1.27))) (uuid "00000000-0000-0000-0000-0000000000c2"))
  (label "GND" (at 110 61.43 0) (effects (font (size 1.27 1.27))) (uuid "00000000-0000-0000-0000-0000000000c3"))
  (symbol (lib_id "Device:R") (at 100 50 0) (unit 1) (in_bom yes) (on_board yes) (uuid "00000000-0000-0000-0000-0000000000d1")
    (property "Reference" "R1" (at 102 50 90) (effects (font (size 1.27 1.27))))
    (property "Value" "1k" (at 98 50 90) (effects (font (size 1.27 1.27))))
    (property "Footprint" "Lib:Part" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))
    (pin "1" (uuid "00000000-0000-0000-0000-0000000000e1"))
    (pin "2" (uuid "00000000-0000-0000-0000-0000000000e2")))
  (symbol (lib_id "Device:R") (at 100 57.62 0) (unit 1) (in_bom yes) (on_board yes) (uuid "00000000-0000-0000-0000-0000000000d2")
    (property "Reference" "R2" (at 102 57.62 90) (effects (font (size 1.27 1.27))))
    (property "Value" "3k" (at 98 57.62 90) (effects (font (size 1.27 1.27))))
    (property "Footprint" "Lib:Part" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))
    (pin "1" (uuid "00000000-0000-0000-0000-0000000000e3"))
    (pin "2" (uuid "00000000-0000-0000-0000-0000000000e4")))
  (symbol (lib_id "Device:R") (at 120 54 0) (unit 1) (in_bom yes) (on_board yes) (uuid "00000000-0000-0000-0000-0000000000d3")
    (property "Reference" "V1" (at 122 54 90) (effects (font (size 1.27 1.27))))
    (property "Value" "10" (at 118 54 90) (effects (font (size 1.27 1.27))))
    (property "Footprint" "Lib:Part" (at 0 0 0) (effects (font (size 1.27 1.27)) hide))
    (pin "1" (uuid "00000000-0000-0000-0000-0000000000e5"))
    (pin "2" (uuid "00000000-0000-0000-0000-0000000000e6")))
)
"#;

const PCB: &str = r#"(kicad_pcb (version 20221018) (generator pcbnew)
  (general (thickness 1.6))
  (paper "A4")
  (layers
    (0 "F.Cu" signal)
    (31 "B.Cu" signal)
    (35 "F.Paste" user)
    (39 "F.Mask" user)
    (44 "Edge.Cuts" user))
  (setup (pad_to_mask_clearance 0))
  (net 0 "")
  (net 1 "/IN")
  (net 2 "/MID")
  (net 3 "/GND")
  (gr_rect (start 0 0) (end 30 20) (stroke (width 0.1) (type default)) (fill none) (layer "Edge.Cuts"))
  (footprint "Lib:Part" (layer "F.Cu") (at 5 5)
    (fp_text reference "R1" (at 0 -2) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (fp_text value "1k" (at 0 2) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 1.6 1.6) (drill 0.8) (layers "*.Cu" "*.Mask") (net 1 "/IN"))
    (pad "2" thru_hole circle (at 5 0) (size 1.6 1.6) (drill 0.8) (layers "*.Cu" "*.Mask") (net 2 "/MID")))
  (footprint "Lib:Part" (layer "F.Cu") (at 15 5)
    (fp_text reference "R2" (at 0 -2) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (fp_text value "3k" (at 0 2) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 1.6 1.6) (drill 0.8) (layers "*.Cu" "*.Mask") (net 2 "/MID"))
    (pad "2" thru_hole circle (at 5 0) (size 1.6 1.6) (drill 0.8) (layers "*.Cu" "*.Mask") (net 3 "/GND")))
  (footprint "Lib:Part" (layer "F.Cu") (at 5 15)
    (fp_text reference "V1" (at 0 -2) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
    (fp_text value "10" (at 0 2) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
    (pad "1" thru_hole circle (at 0 0) (size 1.6 1.6) (drill 0.8) (layers "*.Cu" "*.Mask") (net 1 "/IN"))
    (pad "2" thru_hole circle (at 5 0) (size 1.6 1.6) (drill 0.8) (layers "*.Cu" "*.Mask") (net 3 "/GND")))
  (segment (start 10 5) (end 15 5) (width 0.25) (layer "F.Cu") (net 2))
)
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

/// The designator and the value of every component of an imported board, and the counts of what
/// the model holds, as the lines of one text that two imports can be compared by.
fn board_facts(json: &serde_json::Value) -> BTreeSet<String> {
    let model = &json["board"]["model"];
    let mut facts = BTreeSet::new();
    for component in model["components"]
        .as_object()
        .map(|o| o.values().collect::<Vec<_>>())
        .unwrap_or_default()
    {
        facts.insert(format!(
            "{} {}",
            component["refdes"].as_str().unwrap_or(""),
            component["value"].as_str().unwrap_or("")
        ));
    }
    for key in ["tracks", "vias", "nets", "footprints"] {
        facts.insert(format!(
            "{key} {}",
            model[key].as_object().map_or(0, |o| o.len())
        ));
    }
    facts.insert(format!(
        "pad_nets {}",
        model["pad_nets"].as_array().map_or(0, Vec::len)
    ));
    facts
}

fn number(fields: &[&str], index: usize) -> f64 {
    fields[index].trim().parse::<f64>().unwrap()
}

#[test]
fn a_kicad_project_goes_from_the_files_to_the_package_and_back_to_the_same_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let pro = put(dir.path(), "flow.kicad_pro", PRO);
    let sch = put(dir.path(), "flow.kicad_sch", SCH);
    let pcb = put(dir.path(), "flow.kicad_pcb", PCB);
    let relaxed = put(dir.path(), "relaxed.json", RELAXED);
    let strict = put(dir.path(), "strict.json", STRICT);
    let panel_cfg = put(dir.path(), "panel.cfg", PANEL);

    // stage 1, read: three components of one footprint, one track, no via, three nets, six pads
    // with a net, a rectangle for an outline, and three parts on three nets in the schematic
    let (code, out, err) = call(&["kicad-import", "--json", &pro]);
    let imported: serde_json::Value = serde_json::from_str(out.trim()).unwrap_or_default();
    let model = &imported["board"]["model"];
    let sch_model = &imported["schematic"]["model"];
    let count =
        |value: &serde_json::Value, key: &str| value[key].as_object().map_or(0, |o| o.len());
    assert!(
        code == 0
            && count(model, "components") == 3
            && count(model, "footprints") == 1
            && count(model, "tracks") == 1
            && count(model, "vias") == 0
            && count(model, "nets") == 3
            && model["pad_nets"].as_array().map_or(0, Vec::len) == 6
            && model["outline"].as_array().map_or(0, Vec::len) == 4
            && count(sch_model, "parts") == 3
            && count(sch_model, "nets") == 3,
        "stage 1 (read): {code}\n{out}{err}"
    );

    // stage 2, electrical rule check: every pin is on a net of two pins, so nothing is proven
    let (code, out, err) = call(&["erc", "--json", &sch]);
    let value: serde_json::Value = serde_json::from_str(out.trim()).unwrap_or_default();
    assert!(
        code == 0 && value["passes"] == true,
        "stage 2 (erc): {code}\n{out}{err}"
    );

    // stage 3, rules of the board: the track of 0.25 mm passes 0.2 mm and breaks 0.3 mm once
    let (code_ok, out_ok, _) = call(&["check", "--rules", &relaxed, &pcb]);
    let (code_bad, out_bad, _) = call(&["check", "--rules", &strict, &pcb]);
    assert!(
        code_ok == 0
            && out_ok.contains("Confirmed: 0")
            && code_bad == 1
            && out_bad.contains("Confirmed: 1")
            && out_bad.contains("PHOENIX-CHECK-001"),
        "stage 3 (rules): {code_ok} {code_bad}\n{out_ok}\n{out_bad}"
    );

    // stage 4, written back as KiCad and read again: the same designators, values and counts
    let written = at(dir.path(), "again/flow.kicad_pcb");
    let (code, out, err) = call(&["kicad-export", "--out", &written, "board", &pcb]);
    let (code_again, out_again, err_again) = call(&["kicad-import", "--json", &written]);
    let reread: serde_json::Value = serde_json::from_str(out_again.trim()).unwrap_or_default();
    assert!(
        code == 0 && code_again == 0 && board_facts(&reread) == board_facts(&imported),
        "stage 4 (write back and read again): {code} {code_again}\n{out}{err}\n{err_again}\n{:?}\n{:?}",
        board_facts(&reread),
        board_facts(&imported)
    );

    // stage 5, panel: two boards side by side make 60 mm by 20 mm, and the files are written once
    let panel_dir = at(dir.path(), "panel");
    let (code, out, err) = call(&["panel", "--config", &panel_cfg, "--out", &panel_dir, &pcb]);
    let (again_code, _, again_err) =
        call(&["panel", "--config", &panel_cfg, "--out", &panel_dir, &pcb]);
    assert!(
        code == 0
            && out.starts_with("panel 60.0 mm by 20.0 mm, 2 x 1 boards")
            && again_code == 2
            && again_err.contains("PHOENIX-PANEL-008"),
        "stage 5 (panel): {code} {again_code}\n{out}{err}{again_err}"
    );

    // stage 6, fabrication: the package is a zip file, the same bytes every time
    let one = at(dir.path(), "one.zip");
    let two = at(dir.path(), "two.zip");
    let (code_one, out_one, err_one) = call(&["fabpack", "--out", &one, &pcb]);
    let (code_two, _, _) = call(&["fabpack", "--out", &two, &pcb]);
    let bytes = fs::read(&one).unwrap_or_default();
    assert!(
        code_one == 0
            && code_two == 0
            && !bytes.is_empty()
            && bytes == fs::read(&two).unwrap_or_default(),
        "stage 6 (fabrication): {code_one} {code_two}\n{out_one}{err_one}"
    );

    // stage 7, the package read again: it verifies against its own manifest and holds the layers
    let report = verify_pack(&bytes);
    let entries = read_zip(&bytes).unwrap_or_default();
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        report.as_ref().is_ok_and(|r| r.profile == "generic")
            && names.contains(&"manifest.json")
            && names.contains(&"bom/bom_board.csv")
            && names.contains(&"pnp/pnp_top.csv")
            && names.iter().any(|n| n.starts_with("gerber/")),
        "stage 7 (read the package): {:?}\n{names:?}",
        report.err()
    );

    // stage 8, comparison with the original: the three designators with their values and, in the
    // placement file, R2 ten millimetres to the right of R1 and V1 ten millimetres from it down
    let text_of = |name: &str| {
        entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default()
    };
    let placement = text_of("pnp/pnp_top.csv");
    let rows: Vec<Vec<&str>> = placement
        .lines()
        .map(|line| line.split(',').collect::<Vec<&str>>())
        .filter(|fields| fields.len() > 4)
        .collect();
    let row = |refdes: &str| rows.iter().find(|f| f[0] == refdes);
    let geometry_ok = match (row("R1"), row("R2"), row("V1")) {
        (Some(r1), Some(r2), Some(v1)) => {
            (number(r2, 3) - number(r1, 3) - 10.0).abs() < 1e-3
                && (number(r2, 4) - number(r1, 4)).abs() < 1e-3
                && (number(v1, 3) - number(r1, 3)).abs() < 1e-3
                && ((number(v1, 4) - number(r1, 4)).abs() - 10.0).abs() < 1e-3
        }
        _ => false,
    };
    let values_ok = row("R1").is_some_and(|f| f[1] == "1k")
        && row("R2").is_some_and(|f| f[1] == "3k")
        && row("V1").is_some_and(|f| f[1] == "10");
    assert!(
        geometry_ok && values_ok,
        "stage 8 (compare with the original): {rows:?}"
    );
}
