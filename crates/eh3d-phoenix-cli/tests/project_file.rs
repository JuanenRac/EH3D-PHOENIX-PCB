// SPDX-License-Identifier: GPL-3.0-or-later
//! A board saved into an `.eh3d` project by the command line and read back by it.

use std::fs;

use eh3d_phoenix_cli::run;
use eh3d_phoenix_project::pcb_branch::read_pcb;

fn phoenix(args: &[&str]) -> (i32, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (code, String::from_utf8_lossy(&err).into_owned())
}

const BOARD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<eagle version="9.6.2"><drawing><layers><layer number="1" name="Top"/><layer number="16" name="Bottom"/><layer number="20" name="Dimension"/></layers><board><plain>
<wire x1="0" y1="0" x2="20" y2="0" width="0" layer="20"/><wire x1="20" y1="0" x2="20" y2="10" width="0" layer="20"/>
<wire x1="20" y1="10" x2="0" y2="10" width="0" layer="20"/><wire x1="0" y1="10" x2="0" y2="0" width="0" layer="20"/>
</plain><elements/><signals/></board></drawing></eagle>
"#;

#[test]
fn a_board_goes_into_a_project_and_comes_out_of_it() {
    let dir = tempfile::tempdir().unwrap();
    let at = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    let (brd, ops, first, second) = (at("in.brd"), at("ops.json"), at("a.eh3d"), at("b.eh3d"));
    fs::write(&brd, BOARD).unwrap();
    fs::write(&ops, "[]").unwrap();
    let (code, err) = phoenix(&["boardedit", "--apply", &ops, "--out", &first, &brd]);
    assert_eq!(code, 0, "{err}");
    let parts = read_pcb(dir.path().join("a.eh3d").as_path()).unwrap();
    assert!(parts.board.is_some());
    // the outline of the board of 20 by 10 mm goes along for the programs of enclosures
    let outline: serde_json::Value = serde_json::from_slice(&parts.outline2d.unwrap()).unwrap();
    assert_eq!(outline["artifact"], "outline2d");
    assert_eq!(outline["version"], 1);
    assert_eq!(outline["units"], "nm");
    assert_eq!(outline["outline"].as_array().unwrap().len(), 4);
    assert_eq!(outline["thickness"], 1_600_000);
    assert_eq!(parts.origin.unwrap().bytes, BOARD.as_bytes());
    // the project is a board input of every command that takes one
    let (code, err) = phoenix(&["boardedit", "--apply", &ops, "--out", &second, &first]);
    assert_eq!(code, 0, "{err}");
    let again = read_pcb(dir.path().join("b.eh3d").as_path()).unwrap();
    assert_eq!(again.board, parts.board);
    // an existing file is never replaced
    let (code, _) = phoenix(&["boardedit", "--apply", &ops, "--out", &first, &brd]);
    assert_eq!(code, 2);
}
