//! The board model: round trips, validation, reversible operations and the undo/redo history,
//! checked on thousands of generated boards.

mod common;

use common::Gen;
use eh3d_diagnostic::Severity;
use eh3d_phoenix_board::validate::{self, REGISTRY};
use eh3d_phoenix_board::{Board, History, Operation, PadNet, Segment, Track};
use eh3d_phoenix_model::{Nm, Point, Udeg};

fn codes(board: &Board) -> Vec<String> {
    board
        .validate()
        .into_iter()
        .filter_map(|d| d.code.map(|c| c.into_owned()))
        .collect()
}

#[test]
fn a_generated_board_is_consistent_and_round_trips_byte_for_byte() {
    let mut count = 0;
    for seed in 1..=400u64 {
        let mut g = Gen::new(seed);
        let board = g.board(1 + (seed % 25) as usize);
        let diagnostics = board.validate();
        assert!(
            diagnostics.iter().all(|d| d.severity != Severity::Error),
            "seed {seed}: {diagnostics:?}"
        );
        let bytes = board.to_bytes().unwrap();
        let back = Board::from_bytes(&bytes).unwrap();
        assert_eq!(back, board, "seed {seed}");
        assert_eq!(back.to_bytes().unwrap(), bytes, "seed {seed}");
        count += 1;
    }
    assert_eq!(count, 400);
}

#[test]
fn the_same_seed_gives_the_same_bytes_in_two_runs() {
    let a = Gen::new(77).board(12).to_bytes().unwrap();
    let b = Gen::new(77).board(12).to_bytes().unwrap();
    assert_eq!(a, b);
    assert_ne!(a, Gen::new(78).board(12).to_bytes().unwrap());
}

#[test]
fn every_kind_of_problem_has_its_code() {
    let mut g = Gen::new(5);
    let base = g.board(6);
    assert!(base.is_consistent());

    let mut b = base.clone();
    b.outline.truncate(2);
    assert!(codes(&b).contains(&validate::BAD_OUTLINE.to_string()));

    let mut b = base.clone();
    let first = *b.components.keys().next().unwrap();
    b.components.get_mut(&first).unwrap().footprint = g.id();
    assert!(codes(&b).contains(&validate::DANGLING.to_string()));

    let mut b = base.clone();
    let ids: Vec<_> = b.components.keys().copied().collect();
    let refdes = b.components[&ids[0]].refdes.clone();
    b.components.get_mut(&ids[1]).unwrap().refdes = refdes;
    assert!(codes(&b).contains(&validate::DUPLICATE_REFDES.to_string()));

    let mut b = base.clone();
    let tid = *b.tracks.keys().next().unwrap();
    let t = b.tracks.get_mut(&tid).unwrap();
    t.width = Nm::ZERO;
    assert!(codes(&b).contains(&validate::BAD_SIZE.to_string()));

    let mut b = base.clone();
    let t = b.tracks.get_mut(&tid).unwrap();
    let last = t.path.last().copied().unwrap();
    t.path.push(Segment::Line {
        start: Point::from_nm(1, 1).unwrap(),
        end: last.end(),
    });
    assert!(codes(&b).contains(&validate::PATH_NOT_CONNECTED.to_string()));

    let mut b = base.clone();
    let t = b.tracks.get_mut(&tid).unwrap();
    let start = t.path[0].start();
    t.path = vec![Segment::Line {
        start,
        end: Point::from_nm(start.x_nm.get() + 3_000_000, start.y_nm.get() + 1_000_000).unwrap(),
    }];
    let found = b.validate();
    let warning = found
        .iter()
        .find(|d| d.code.as_deref() == Some(validate::OFF_GRID_ANGLE))
        .unwrap();
    assert_eq!(warning.severity, Severity::Warning);
    assert!(
        b.is_consistent(),
        "an off-grid piece is a warning, not an error"
    );

    let mut b = base.clone();
    let t = b.tracks.get_mut(&tid).unwrap();
    let start = t.path[0].start();
    t.path = vec![Segment::Arc {
        start,
        end: start,
        sweep: Udeg::new(90_000_000).unwrap(),
    }];
    assert!(codes(&b).contains(&validate::BAD_ARC.to_string()));

    let mut b = base.clone();
    let vid = b.vias.keys().next().copied();
    if let Some(vid) = vid {
        let v = b.vias.get_mut(&vid).unwrap();
        v.to_layer = v.from_layer;
        assert!(codes(&b).contains(&validate::BAD_LAYER.to_string()));
    }

    let mut b = base.clone();
    if let Some(p) = b.pad_nets.first().copied() {
        b.pad_nets.push(PadNet { net: p.net, ..p });
        assert!(codes(&b).contains(&validate::PAD_NET_CONFLICT.to_string()));
    }
}

#[test]
fn the_registry_has_every_code_the_checks_can_give() {
    assert_eq!(REGISTRY.len(), 9);
    let mut seen = std::collections::BTreeSet::new();
    for (code, _) in REGISTRY {
        assert!(seen.insert(*code));
    }
}

#[test]
fn damaged_board_text_is_an_error_and_never_a_panic() {
    let board = Gen::new(9).board(8);
    let good = board.to_bytes().unwrap();
    for cut in (0..good.len()).step_by(37) {
        let _ = Board::from_bytes(&good[..cut]);
    }
    let mut g = Gen::new(10);
    for _ in 0..300 {
        let mut bytes = good.clone();
        for _ in 0..1 + g.rng.below(4) {
            let at = g.rng.below(bytes.len() as u64) as usize;
            bytes[at] = g.rng.below(256) as u8;
        }
        if let Ok(b) = Board::from_bytes(&bytes) {
            // whatever was read can be checked without panicking
            let _ = b.validate();
        }
    }
    // a value outside the range of the design domain is refused when reading
    let text = std::str::from_utf8(&good).unwrap().replacen(
        "\"x_nm\":",
        "\"x_nm\":99999999999999,\"y_ignored\":",
        1,
    );
    assert!(Board::from_bytes(text.as_bytes()).is_err());
    // an unknown field is refused, because it could change the meaning
    let text = std::str::from_utf8(&good).unwrap().replacen(
        "{\"components\"",
        "{\"extra\":1,\"components\"",
        1,
    );
    assert!(Board::from_bytes(text.as_bytes()).is_err());
}

#[test]
fn undoing_every_change_gives_back_the_same_bytes_and_redoing_gives_the_final_board() {
    for seed in 1..=120u64 {
        let mut g = Gen::new(seed + 1000);
        let start = g.board(1 + (seed % 10) as usize);
        let start_bytes = start.to_bytes().unwrap();
        let mut board = start.clone();
        let mut history = History::new();
        let mut applied = 0;
        for _ in 0..60 {
            let op = g.operation(&board);
            let before = board.clone();
            match history.execute(&mut board, op) {
                Ok(()) => applied += 1,
                Err(_) => assert_eq!(
                    board, before,
                    "seed {seed}: a refused operation changed the board"
                ),
            }
            assert!(board.is_consistent(), "seed {seed}: {:?}", board.validate());
        }
        assert_eq!(history.done().len(), applied);
        let final_board = board.clone();

        // the history written and read back replays to the same result
        let saved = history.to_bytes().unwrap();
        let loaded = History::from_bytes(&saved).unwrap();
        assert_eq!(loaded, history);
        let mut replayed = start.clone();
        loaded.replay(&mut replayed).unwrap();
        assert_eq!(
            replayed.to_bytes().unwrap(),
            final_board.to_bytes().unwrap(),
            "seed {seed}"
        );

        while history.undo(&mut board) {}
        assert_eq!(board.to_bytes().unwrap(), start_bytes, "seed {seed}: undo");
        while history.redo(&mut board) {}
        assert_eq!(board, final_board, "seed {seed}: redo");
    }
}

#[test]
fn a_new_change_after_an_undo_clears_what_could_be_redone() {
    let mut g = Gen::new(3);
    let mut board = g.board(4);
    let mut history = History::new();
    let a = g.id();
    history
        .execute(
            &mut board,
            Operation::AddNet {
                id: a,
                name: "A".into(),
            },
        )
        .unwrap();
    assert!(history.undo(&mut board));
    assert!(!board.nets.contains_key(&a));
    let b = g.id();
    history
        .execute(
            &mut board,
            Operation::AddNet {
                id: b,
                name: "B".into(),
            },
        )
        .unwrap();
    assert!(!history.redo(&mut board), "nothing left to redo");
    assert!(!History::new().undo(&mut board.clone()));
}

#[test]
fn a_batch_is_all_or_nothing() {
    let mut g = Gen::new(4);
    let mut board = g.board(3);
    let before = board.clone();
    let x = g.id();
    let batch = Operation::Batch {
        ops: vec![
            Operation::AddNet {
                id: x,
                name: "X".into(),
            },
            Operation::RemoveNet { id: g.id() }, // does not exist: fails
        ],
    };
    assert!(board.apply(&batch).is_err());
    assert_eq!(board, before);
}

#[test]
fn removing_a_net_in_use_is_refused_and_a_track_needs_its_net_and_layer() {
    let mut g = Gen::new(6);
    let mut board = g.board(5);
    let used = board.tracks.values().next().unwrap().net;
    assert!(board.apply(&Operation::RemoveNet { id: used }).is_err());
    let track = Track {
        net: g.id(),
        layer: *board.layers.keys().next().unwrap(),
        width: Nm::new(100_000).unwrap(),
        path: vec![],
    };
    assert!(board
        .apply(&Operation::AddTrack { id: g.id(), track })
        .is_err());
}

#[test]
fn changing_the_fields_of_a_component_is_reversible_and_keeps_its_place() {
    let mut g = Gen::new(11);
    let mut board = g.board(4);
    let before = board.clone();
    let (cid, c) = board
        .components
        .iter()
        .next()
        .map(|(k, v)| (*k, v.clone()))
        .unwrap();
    let op = Operation::SetComponentFields {
        id: cid,
        refdes: "Z9".into(),
        value: "99k".into(),
        footprint: c.footprint,
    };
    let inverse = board.apply(&op).unwrap();
    assert_eq!(board.components[&cid].refdes, "Z9");
    assert_eq!(board.components[&cid].value, "99k");
    assert_eq!(board.components[&cid].at, c.at);
    board.apply(&inverse).unwrap();
    assert_eq!(board, before);
    // a footprint that the board does not have is refused and nothing changes
    let bad = Operation::SetComponentFields {
        id: cid,
        refdes: "Q1".into(),
        value: String::new(),
        footprint: g.id(),
    };
    assert!(board.apply(&bad).is_err());
    assert_eq!(board, before);
}
