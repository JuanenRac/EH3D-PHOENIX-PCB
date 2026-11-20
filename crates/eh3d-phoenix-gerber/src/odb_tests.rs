// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the reader of the ODB++ matrix with a fixed seed: matrices made of random layers are
//! read back as they were written, and texts damaged at random are either read into a matrix that
//! keeps the rules of the reader or refused with one of its codes, never with a panic.

use crate::codes::{INPUT_TOO_LARGE, LIMIT_EXCEEDED, ODB_UNSUPPORTED};
use crate::odb::{parse_matrix, OdbContext, OdbLayerKind, OdbLimits};

/// A generator with a fixed seed, so every run makes the same texts.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

const KINDS: [(&str, OdbLayerKind); 11] = [
    ("SIGNAL", OdbLayerKind::Signal),
    ("POWER_GROUND", OdbLayerKind::PowerGround),
    ("MIXED", OdbLayerKind::Mixed),
    ("SOLDER_MASK", OdbLayerKind::SolderMask),
    ("SOLDER_PASTE", OdbLayerKind::SolderPaste),
    ("SILK_SCREEN", OdbLayerKind::SilkScreen),
    ("DRILL", OdbLayerKind::Drill),
    ("ROUT", OdbLayerKind::Rout),
    ("DOCUMENT", OdbLayerKind::Document),
    ("COMPONENT", OdbLayerKind::Component),
    ("DIELECTRIC", OdbLayerKind::Dielectric),
];

/// The layers that were written: the row, the name, the index of the kind, whether it is of the
/// board and whether it is positive.
type Made = Vec<(u32, String, usize, bool, bool)>;

/// A matrix of `count` layers with random kinds, contexts and polarities, and the text of it.
fn random_matrix(rng: &mut Lcg, count: usize) -> (String, Made) {
    let mut text = String::new();
    for step in 0..1 + rng.next() % 3 {
        text.push_str(&format!(
            "STEP {{\n  COL={}\n  NAME=step{step}\n}}\n",
            step + 1
        ));
    }
    let mut made = Vec::new();
    // The rows are written in a shuffled order.
    let mut rows: Vec<u32> = (1..=count as u32).collect();
    for i in (1..rows.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        rows.swap(i, j);
    }
    for (n, row) in rows.into_iter().enumerate() {
        let kind = (rng.next() % KINDS.len() as u64) as usize;
        let board = !rng.next().is_multiple_of(5);
        let positive = !rng.next().is_multiple_of(4);
        let name = format!("L{n}_{}", rng.next() % 1000);
        text.push_str(&format!(
            "LAYER {{\n  ROW={row}\n  CONTEXT={}\n  TYPE={}\n  NAME={name}\n  POLARITY={}\n  COLOR={}\n}}\n",
            if board { "BOARD" } else { "MISC" },
            KINDS[kind].0,
            if positive { "POSITIVE" } else { "NEGATIVE" },
            rng.next() % 99999
        ));
        made.push((row, name, kind, board, positive));
    }
    made.sort();
    (text, made)
}

#[test]
fn a_matrix_of_random_layers_is_read_back_as_it_was_written() {
    let mut rng = Lcg(0x5a_0b01);
    for _ in 0..60 {
        let count = 1 + (rng.next() % 30) as usize;
        let (text, made) = random_matrix(&mut rng, count);
        let m = parse_matrix(text.as_bytes(), &OdbLimits::default()).unwrap();
        assert_eq!(m.layers.len(), made.len());
        for (layer, (row, name, kind, board, positive)) in m.layers.iter().zip(&made) {
            assert_eq!(layer.row, *row);
            assert_eq!(&layer.name, name);
            assert_eq!(layer.kind, KINDS[*kind].1);
            assert_eq!(layer.context == OdbContext::Board, *board);
            assert_eq!(layer.positive, *positive);
        }
        assert!(!m.steps.is_empty());
        assert_eq!(m.roles().len(), m.layers.len());
    }
}

#[test]
fn a_text_damaged_at_random_is_read_by_the_rules_or_refused_with_a_code() {
    let mut rng = Lcg(0x5a_0b02);
    let (base, _) = random_matrix(&mut rng, 12);
    let bytes = base.as_bytes();
    let limits = OdbLimits::default();
    for round in 0..400 {
        let mut damaged = bytes.to_vec();
        match round % 4 {
            0 => {
                let cut = (rng.next() as usize) % damaged.len();
                damaged.truncate(cut);
            }
            1 => {
                for _ in 0..1 + rng.next() % 5 {
                    let at = (rng.next() as usize) % damaged.len();
                    damaged[at] = (rng.next() % 256) as u8;
                }
            }
            2 => {
                let at = (rng.next() as usize) % damaged.len();
                let end = (at + 1 + (rng.next() as usize) % 40).min(damaged.len());
                damaged.drain(at..end);
            }
            _ => {
                let at = (rng.next() as usize) % damaged.len();
                damaged.insert(at, b"{}=\n"[(rng.next() % 4) as usize]);
            }
        }
        match parse_matrix(&damaged, &limits) {
            Ok(m) => {
                assert!(!m.layers.is_empty());
                assert!(m.layers.windows(2).all(|w| w[0].row < w[1].row));
                for (i, a) in m.layers.iter().enumerate() {
                    assert!(a.row >= 1 && !a.name.is_empty());
                    for b in &m.layers[i + 1..] {
                        assert!(!a.name.eq_ignore_ascii_case(&b.name));
                    }
                }
            }
            Err(e) => assert!(
                [ODB_UNSUPPORTED, INPUT_TOO_LARGE, LIMIT_EXCEEDED].contains(&e.code),
                "{e}"
            ),
        }
    }
}

#[test]
fn the_same_text_gives_the_same_matrix_every_time() {
    let mut rng = Lcg(0x5a_0b03);
    let (text, _) = random_matrix(&mut rng, 20);
    let limits = OdbLimits::default();
    assert_eq!(
        parse_matrix(text.as_bytes(), &limits),
        parse_matrix(text.as_bytes(), &limits)
    );
}
