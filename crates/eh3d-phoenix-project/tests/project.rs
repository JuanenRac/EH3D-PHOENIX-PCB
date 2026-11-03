//! Reading and writing `.phoenixproj` files and the rules of opening.

use std::collections::BTreeSet;
use std::path::PathBuf;

use eh3d_container::{AtomicOptions, ContainerError, PkgCode};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, ObjectId, StepClock};
use eh3d_phoenix_project::codes;
use eh3d_phoenix_project::{
    check_links, Link, Manifest, Migration, OpenOptions, Project, ProjectError, Strength,
};
use serde_json::json;

fn id(n: u64) -> ObjectId {
    IdGenerator::new(
        StepClock::new(1_700_000_000_000 + n, 1),
        DeterministicRandom::new(n),
    )
    .generate()
    .unwrap()
}

fn folder(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_project_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample() -> Project {
    let manifest = Manifest::new(id(1), "Example")
        .with_domain("design", "0.1.0", true)
        .with_domain("history", "0.1.0", false);
    let mut p = Project::new(manifest);
    p.set_entry("design/board.json", br#"{"layers":2}"#.to_vec());
    p.set_entry("history/log.json", br#"{"ops":[]}"#.to_vec());
    p.set_entry("opaque/imported.bin", vec![0, 1, 2, 3]);
    p.set_entry("x-vendor/data.txt", b"third party".to_vec());
    p.set_entry("audit/run.json", br#"{"program":"x"}"#.to_vec());
    p
}

fn save(p: &Project, dir: &std::path::Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    p.save_to_path(&path, &AtomicOptions::default()).unwrap();
    path
}

fn refused_code(error: ProjectError) -> String {
    error
        .diagnostic()
        .and_then(|d| d.code.map(|c| c.into_owned()))
        .unwrap_or_else(|| panic!("no code: {error}"))
}

#[test]
fn a_project_saves_and_opens_with_everything_it_had() {
    let dir = folder("roundtrip");
    let project = sample();
    let path = save(&project, &dir, "a.phoenixproj");
    let opened = Project::open_path(&path, &OpenOptions::default()).unwrap();
    assert_eq!(opened.project, project);
    assert!(opened.diagnostics.is_empty());
    assert!(!opened.project.is_read_only());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn saving_twice_gives_the_same_bytes_and_a_reopened_project_saves_the_same_bytes() {
    let dir = folder("bytes");
    let first = save(&sample(), &dir, "a.phoenixproj");
    let second = save(&sample(), &dir, "b.phoenixproj");
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap()
    );
    let reopened = Project::open_path(&first, &OpenOptions::default())
        .unwrap()
        .project;
    let third = save(&reopened, &dir, "c.phoenixproj");
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&third).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_newer_layout_is_refused_with_the_container_code() {
    let dir = folder("newer");
    let mut p = sample();
    p.manifest.format_version = "0.2.0".into();
    let path = save(&p, &dir, "n.phoenixproj");
    match Project::open_path(&path, &OpenOptions::default()) {
        Err(ProjectError::Container(e)) => assert_eq!(e.code(), Some(PkgCode::NewerVersion)),
        other => panic!("{other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn step_zero_to_one(project: &mut Project) -> Result<(), String> {
    project.set_entry("history/migrated.json", b"{}".to_vec());
    Ok(())
}

fn failing_step(_: &mut Project) -> Result<(), String> {
    Err("cannot".into())
}

#[test]
fn an_older_project_needs_a_registered_migration() {
    let dir = folder("older");
    let mut p = sample();
    p.manifest.format_version = "0.0.0".into();
    let path = save(&p, &dir, "o.phoenixproj");

    let error = Project::open_path(&path, &OpenOptions::default()).unwrap_err();
    assert_eq!(refused_code(error), codes::MIGRATION_MISSING);

    let mut with = OpenOptions::default();
    with.migrations.push(Migration {
        name: "zero-to-one",
        from_minor: 0,
        to_minor: 1,
        apply: step_zero_to_one,
    });
    let opened = Project::open_path(&path, &with).unwrap();
    assert_eq!(opened.project.manifest.format_version, "0.1.0");
    assert_eq!(opened.project.manifest.migrations_applied, ["zero-to-one"]);
    assert!(opened.project.entry("history/migrated.json").is_some());

    let mut failing = OpenOptions::default();
    failing.migrations.push(Migration {
        name: "bad",
        from_minor: 0,
        to_minor: 1,
        apply: failing_step,
    });
    let error = Project::open_path(&path, &failing).unwrap_err();
    assert_eq!(refused_code(error), codes::MIGRATION_FAILED);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_critical_domain_that_is_newer_is_refused_or_opened_read_only() {
    let dir = folder("critical");
    let mut p = sample();
    p.manifest.domains[0].version = "0.2.0".into();
    let path = save(&p, &dir, "c.phoenixproj");

    let error = Project::open_path(&path, &OpenOptions::default()).unwrap_err();
    assert_eq!(refused_code(error), codes::CRITICAL_DOMAIN_UNKNOWN);

    let options = OpenOptions {
        read_only_if_unknown_critical: true,
        ..OpenOptions::default()
    };
    let opened = Project::open_path(&path, &options).unwrap();
    assert!(opened.project.is_read_only());
    assert!(opened
        .diagnostics
        .iter()
        .any(|d| d.code.as_deref() == Some(codes::OPENED_READ_ONLY)));
    let again = dir.join("copy.phoenixproj");
    assert!(matches!(
        opened
            .project
            .save_to_path(&again, &AtomicOptions::default()),
        Err(ProjectError::ReadOnly)
    ));
    assert!(!again.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_domain_that_is_not_critical_and_not_understood_is_kept_untouched() {
    let dir = folder("opaque");
    let mut p = sample();
    p.manifest.domains[1].version = "0.2.0".into();
    let path = save(&p, &dir, "h.phoenixproj");
    let opened = Project::open_path(&path, &OpenOptions::default()).unwrap();
    assert!(opened
        .diagnostics
        .iter()
        .any(|d| d.code.as_deref() == Some(codes::DOMAIN_PRESERVED)));
    assert!(!opened.project.is_read_only());
    // saving again keeps the domain exactly as it was
    let copy = save(&opened.project, &dir, "h2.phoenixproj");
    assert_eq!(std::fs::read(&path).unwrap(), std::fs::read(&copy).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_object_marked_must_understand_is_refused_unless_its_type_is_known() {
    let dir = folder("must");
    let mut p = sample();
    p.set_entry(
        "design/future.json",
        serde_json::to_vec(&json!({"items": [{"type": "fancy", "must_understand": true}]}))
            .unwrap(),
    );
    p.set_entry(
        "design/ok.json",
        serde_json::to_vec(&json!({"a": {"type": "plain", "must_understand": false}})).unwrap(),
    );
    let path = save(&p, &dir, "m.phoenixproj");
    let error = Project::open_path(&path, &OpenOptions::default()).unwrap_err();
    assert_eq!(refused_code(error), codes::MUST_UNDERSTAND);

    let knows = OpenOptions {
        understood_types: BTreeSet::from(["fancy".to_owned()]),
        ..OpenOptions::default()
    };
    assert!(Project::open_path(&path, &knows).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_top_level_field_marked_must_understand_is_refused() {
    let dir = folder("topfield");
    let mut p = sample();
    p.manifest
        .extra
        .insert("future".into(), json!({"must_understand": true}));
    let path = save(&p, &dir, "t.phoenixproj");
    let error = Project::open_path(&path, &OpenOptions::default()).unwrap_err();
    assert_eq!(refused_code(error), codes::MUST_UNDERSTAND);
    // a harmless unknown field survives a round trip
    let mut harmless = sample();
    harmless
        .manifest
        .extra
        .insert("note".into(), json!({"a": 1}));
    let path = save(&harmless, &dir, "u.phoenixproj");
    let opened = Project::open_path(&path, &OpenOptions::default()).unwrap();
    assert_eq!(opened.project.manifest.extra["note"], json!({"a": 1}));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_entry_in_an_undeclared_domain_is_refused() {
    let dir = folder("undeclared");
    let mut p = sample();
    p.set_entry("thermal/setup.json", b"{}".to_vec());
    let path = save(&p, &dir, "d.phoenixproj");
    let error = Project::open_path(&path, &OpenOptions::default()).unwrap_err();
    assert_eq!(refused_code(error), codes::DOMAIN_UNDECLARED);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bad_manifest_is_refused() {
    let dir = folder("badmanifest");
    let mut p = sample();
    p.manifest.name = String::new();
    let path = save(&p, &dir, "b.phoenixproj");
    let error = Project::open_path(&path, &OpenOptions::default()).unwrap_err();
    assert_eq!(refused_code(error), codes::MANIFEST_INVALID);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn damaged_files_get_a_code_and_never_panic() {
    let dir = folder("damaged");
    let path = save(&sample(), &dir, "g.phoenixproj");
    let good = std::fs::read(&path).unwrap();
    let broken = dir.join("broken.phoenixproj");
    // every single flipped byte: either still a valid project or an error, never a panic
    let mut refused = 0;
    for at in (0..good.len()).step_by(7) {
        let mut bytes = good.clone();
        bytes[at] ^= 0x55;
        std::fs::write(&broken, &bytes).unwrap();
        if Project::open_path(&broken, &OpenOptions::default()).is_err() {
            refused += 1;
        }
    }
    assert!(
        refused > good.len() / 28,
        "most changes are noticed ({refused})"
    );
    for cut in [0, 1, 10, good.len() / 2, good.len() - 1] {
        std::fs::write(&broken, &good[..cut]).unwrap();
        match Project::open_path(&broken, &OpenOptions::default()) {
            Err(ProjectError::Container(ContainerError::Rejected(v))) => {
                assert_eq!(v.code, PkgCode::DamagedArchive);
            }
            other => panic!("cut at {cut}: {other:?}"),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn saving_refuses_a_name_outside_the_roots() {
    let dir = folder("outside");
    let mut p = sample();
    p.set_entry("weird/a.json", b"{}".to_vec());
    let error = p
        .save_to_path(&dir.join("w.phoenixproj"), &AtomicOptions::default())
        .unwrap_err();
    match error {
        ProjectError::Container(e) => assert_eq!(e.code(), Some(PkgCode::OutsideRoots)),
        other => panic!("{other:?}"),
    }
    assert!(!dir.join("w.phoenixproj").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn strong_links_must_not_dangle_and_weak_ones_become_information() {
    let mut m = Manifest::new(id(1), "x");
    m.links = vec![
        Link {
            from: id(2),
            kind: "footprint_of".into(),
            strength: Strength::Strong,
            to: id(3),
        },
        Link {
            from: id(4),
            kind: "mentions".into(),
            strength: Strength::Weak,
            to: id(5),
        },
        Link {
            from: id(2),
            kind: "ok".into(),
            strength: Strength::Strong,
            to: id(2),
        },
    ];
    let existing: BTreeSet<ObjectId> = [id(2), id(3)].into_iter().collect();
    let found = check_links(&m, &existing);
    assert_eq!(found.len(), 1, "only the weak link is missing an end");
    assert_eq!(found[0].code.as_deref(), Some(codes::WEAK_REFERENCE_STALE));
    let only_two: BTreeSet<ObjectId> = [id(2)].into_iter().collect();
    let found = check_links(&m, &only_two);
    let codes_found: Vec<_> = found.iter().filter_map(|d| d.code.as_deref()).collect();
    assert_eq!(
        codes_found,
        [codes::STRONG_REFERENCE_DANGLES, codes::WEAK_REFERENCE_STALE]
    );
}
