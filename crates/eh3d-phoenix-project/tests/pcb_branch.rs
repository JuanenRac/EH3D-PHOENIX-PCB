// SPDX-License-Identifier: GPL-3.0-or-later
//! The branch `pcb/` of the `.eh3d` project: written, read back, and the branches of the other
//! programs kept byte for byte. All the content here is invented.

use std::fs;
use std::path::Path;

use eh3d_phoenix_project::pcb_branch::{
    read_pcb, write_pcb, NewPcbProject, PcbError, PcbOrigin, PcbParts, PCB_BRANCH,
};
use eh3d_project::{BranchUpdate, OpenOptions, Project, SaveOptions, Writer};

const VERSION: &str = "0.0.0-test";

fn board(text: &str) -> PcbParts {
    PcbParts {
        board: Some(format!("{{\"name\":\"{text}\"}}\n").into_bytes()),
        ..PcbParts::default()
    }
}

fn named(name: &str) -> NewPcbProject {
    NewPcbProject {
        name: name.to_owned(),
        ..NewPcbProject::default()
    }
}

fn add_cad(path: &Path) {
    let project = Project::open(path, &OpenOptions::default()).unwrap();
    let update = BranchUpdate::new("cad", "1.0", "9.9.9").with_entry("part.txt", b"enclosure");
    project
        .save_branch(&Writer::for_app("cad"), update, &SaveOptions::default())
        .unwrap();
}

fn branch_hash(path: &Path, branch: &str) -> String {
    let project = Project::open(path, &OpenOptions::default()).unwrap();
    project.branch(branch).unwrap().hash.clone()
}

#[test]
fn a_new_project_holds_the_board_and_the_same_content_gives_the_same_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.eh3d"), dir.path().join("b.eh3d"));
    for path in [&a, &b] {
        write_pcb(path, None, &board("one"), &named("demo"), VERSION).unwrap();
    }
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    assert_eq!(read_pcb(&a).unwrap(), board("one"));
}

#[test]
fn saving_changes_only_the_branch_of_the_boards() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.eh3d");
    write_pcb(&path, None, &board("one"), &named("demo"), VERSION).unwrap();
    add_cad(&path);
    let cad = branch_hash(&path, "cad");
    let pcb = branch_hash(&path, PCB_BRANCH);
    let schematic = PcbParts {
        schematic: Some(b"{}\n".to_vec()),
        ..PcbParts::default()
    };
    write_pcb(&path, None, &schematic, &named("demo"), VERSION).unwrap();
    assert_eq!(branch_hash(&path, "cad"), cad);
    assert_ne!(branch_hash(&path, PCB_BRANCH), pcb);
    let read = read_pcb(&path).unwrap();
    // the part that was not given stays
    assert_eq!(read.board, board("one").board);
    assert_eq!(read.schematic, schematic.schematic);
}

#[test]
fn saving_under_a_new_name_takes_the_other_branches_along_and_leaves_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("in.eh3d");
    let target = dir.path().join("out.eh3d");
    write_pcb(&source, None, &board("one"), &named("demo"), VERSION).unwrap();
    add_cad(&source);
    let before = fs::read(&source).unwrap();
    write_pcb(&target, Some(&source), &board("two"), &named("x"), VERSION).unwrap();
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(branch_hash(&target, "cad"), branch_hash(&source, "cad"));
    assert_eq!(read_pcb(&target).unwrap().board, board("two").board);
}

#[test]
fn the_file_a_design_came_from_is_kept_with_what_was_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.eh3d");
    let origin = PcbOrigin {
        name: "my board.BRD".to_owned(),
        format: "eagle-brd".to_owned(),
        bytes: b"<eagle/>".to_vec(),
        not_read: 3,
    };
    let parts = PcbParts {
        origin: Some(origin.clone()),
        ..board("one")
    };
    write_pcb(&path, None, &parts, &named("demo"), VERSION).unwrap();
    assert_eq!(read_pcb(&path).unwrap().origin, Some(origin));
    let mut project = Project::open(&path, &OpenOptions::default()).unwrap();
    assert_eq!(
        project.read_entry(PCB_BRANCH, "origin/source.brd").unwrap(),
        b"<eagle/>"
    );
    let findings = project.diagnose(&eh3d_project::Checks::default());
    assert!(findings.iter().all(|f| !f.is_error()), "{findings:?}");
}

#[test]
fn a_project_without_the_branch_a_newer_one_and_a_damaged_one_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.eh3d");
    write_pcb(&path, None, &board("one"), &named("demo"), VERSION).unwrap();
    // only another program wrote a project
    let other = dir.path().join("cad.eh3d");
    let new = eh3d_project::NewProject {
        id: "00000000-0000-4000-8000-000000000001".to_owned(),
        name: "case".to_owned(),
        units: "millimetres".to_owned(),
        created: None,
    };
    Project::create(
        &other,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    add_cad(&other);
    assert!(matches!(read_pcb(&other), Err(PcbError::NoBranch)));
    // a newer format of the branch is neither read nor written over
    let project = Project::open(&path, &OpenOptions::default()).unwrap();
    let update = BranchUpdate::new(PCB_BRANCH, "2.0", "9.9.9").with_entry("board.json", b"{}");
    project
        .save_branch(
            &Writer::for_app(PCB_BRANCH),
            update,
            &SaveOptions::default(),
        )
        .unwrap();
    let before = fs::read(&path).unwrap();
    assert!(matches!(read_pcb(&path), Err(PcbError::Newer(_))));
    assert!(matches!(
        write_pcb(&path, None, &board("two"), &named("demo"), VERSION),
        Err(PcbError::Newer(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    // bytes that are no project, and a project cut short, are errors and never a panic
    let bad = dir.path().join("bad.eh3d");
    fs::write(&bad, b"not a project").unwrap();
    assert!(read_pcb(&bad).is_err());
    let step = (before.len() / 40).max(1);
    for cut in (0..before.len()).step_by(step) {
        fs::write(&bad, &before[..cut]).unwrap();
        assert!(read_pcb(&bad).is_err(), "cut at {cut}");
    }
}
