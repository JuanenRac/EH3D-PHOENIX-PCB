// SPDX-License-Identifier: GPL-3.0-or-later
//! The benchmark of the viewer: three synthetic boards (small, medium and large, by the number of
//! primitives) and the time it takes to prepare their primitives, to build the spatial index, to
//! pick an item by position (with the index and by a scan of every primitive) and to list what a
//! window shows.
//!
//! What it does not measure is the drawing on a screen, which needs a window and a graphics
//! card. The numbers are those of this computer and of the preparation of the data only.

use std::time::Instant;

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Side, SilkItem, Track, Via, Zone};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use eh3d_phoenix_rules::Lang;

use crate::codes::{ViewError, BENCH_INVALID};
use crate::load::id_source;
use crate::msg::Msg;
use crate::prim::P;
use crate::scene::Scene;
use crate::settings::ViewerSettings;
use crate::viewer::Viewer;

/// The smallest board that the benchmark generates, in primitives.
const MIN_TARGET: usize = 10;
/// How many nets the generated boards have.
const NET_COUNT: usize = 64;

/// The times and counts of one board.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchRow {
    /// The name of the size.
    pub name: &'static str,
    /// The size asked for, in primitives.
    pub target: usize,
    /// The primitives made.
    pub prims: usize,
    /// The items of the document.
    pub items: usize,
    /// The cells of the spatial index.
    pub cells: usize,
    /// The milliseconds taken to turn the board into primitives.
    pub convert_ms: f64,
    /// The milliseconds taken to build the spatial index.
    pub index_ms: f64,
    /// The picks made with the index.
    pub picks: usize,
    /// How many of them found an item.
    pub hits: usize,
    /// The microseconds taken by a pick with the index, on average.
    pub pick_us: f64,
    /// The picks made by a scan of every primitive.
    pub linear_picks: usize,
    /// The microseconds taken by a pick by a scan, on average.
    pub linear_us: f64,
    /// How many of the scans gave another answer than the index (it has to be zero).
    pub disagreements: usize,
    /// The primitives that a window showing the whole board lists.
    pub all_listed: usize,
    /// The milliseconds taken to list them.
    pub all_ms: f64,
    /// The primitives that a window showing a sixteenth of the board lists.
    pub part_listed: usize,
    /// The milliseconds taken to list them.
    pub part_ms: f64,
}

/// A small generator of numbers between 0 and 1, the same on every computer.
struct Lcg(u64);

impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn nm(mm: f64) -> Nm {
    Nm::new((mm * 1_000_000.0).round() as i64).unwrap_or(Nm::ZERO)
}

fn point(x: f64, y: f64) -> Point {
    Point {
        x_nm: nm(x),
        y_nm: nm(y),
    }
}

fn check(settings: &ViewerSettings) -> Result<(), ViewError> {
    for (name, target) in [
        ("bench_small", settings.bench_small),
        ("bench_medium", settings.bench_medium),
        ("bench_large", settings.bench_large),
    ] {
        if target < MIN_TARGET {
            return Err(ViewError::new(
                BENCH_INVALID,
                format!("{name} must be at least {MIN_TARGET}"),
            ));
        }
    }
    if settings.bench_linear_picks > settings.bench_picks {
        return Err(ViewError::new(
            BENCH_INVALID,
            "bench_linear_picks must not exceed bench_picks",
        ));
    }
    Ok(())
}

/// A board of copper tracks (lines and arcs), vias, zones and silkscreen lines on two copper
/// layers that makes `target` primitives once it is drawn, the same for the same seed.
pub fn synthetic_board(target: usize, settings: &ViewerSettings) -> Result<Board, ViewError> {
    let mut new_id = id_source()?;
    let mut rng = Lcg(settings.bench_seed);
    let side = ((target as f64).sqrt() * settings.bench_pitch_mm).max(10.0);
    let mut board = Board {
        name: "synthetic".to_string(),
        ..Board::default()
    };
    board.outline = vec![
        point(0.0, 0.0),
        point(side, 0.0),
        point(side, side),
        point(0.0, side),
    ];
    let (top, bottom, silk) = (new_id(), new_id(), new_id());
    for (id, name, kind, layer_side) in [
        (top, "Top", LayerKind::Copper, Some(Side::Top)),
        (bottom, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
        (silk, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
    ] {
        board.layers.insert(
            id,
            Layer {
                name: name.to_string(),
                kind,
                side: layer_side,
            },
        );
    }
    board.copper_stack = vec![top, bottom];
    let nets: Vec<_> = (0..NET_COUNT)
        .map(|i| {
            let id = new_id();
            board.nets.insert(
                id,
                Net {
                    name: format!("N{i}"),
                },
            );
            id
        })
        .collect();
    // the outline is the first primitive
    let mut made = 1usize;
    while made < target {
        let remaining = target - made;
        let roll = rng.unit();
        let (x, y) = (rng.unit() * side, rng.unit() * side);
        let net = nets[(rng.unit() * NET_COUNT as f64) as usize % NET_COUNT];
        let layer = if rng.unit() < 0.5 { top } else { bottom };
        if (0.90..0.95).contains(&roll) {
            let r = 0.5 + rng.unit();
            board.zones.insert(
                new_id(),
                Zone {
                    net,
                    layer,
                    outline: vec![
                        point(x - r, y - r),
                        point(x + r, y - r),
                        point(x + r, y + r),
                        point(x, y + 1.5 * r),
                        point(x - r, y + r),
                    ],
                    clearance: nm(0.2),
                    cutouts: Vec::new(),
                },
            );
            made += 1;
        } else if roll >= 0.95 {
            board.silkscreen.insert(
                new_id(),
                SilkItem::Stroke {
                    layer: silk,
                    width: nm(0.15),
                    path: vec![Segment::Line {
                        start: point(x, y),
                        end: point(x + 2.0 * rng.unit(), y + 2.0 * rng.unit()),
                    }],
                },
            );
            made += 1;
        } else if roll >= 0.82 && remaining >= 3 {
            board.vias.insert(
                new_id(),
                Via {
                    net,
                    at: point(x, y),
                    drill: nm(0.3),
                    diameter: nm(0.6),
                    from_layer: top,
                    to_layer: bottom,
                },
            );
            made += 3;
        } else {
            let arc = roll >= 0.70;
            let end = point(x + 1.0 + 2.0 * rng.unit(), y + 1.0 + rng.unit());
            let segment = if arc {
                Segment::Arc {
                    start: point(x, y),
                    end,
                    sweep: Udeg::new(60_000_000).unwrap_or(Udeg::ZERO),
                }
            } else {
                Segment::Line {
                    start: point(x, y),
                    end,
                }
            };
            board.tracks.insert(
                new_id(),
                Track {
                    net,
                    layer,
                    width: nm(0.15 + 0.15 * rng.unit()),
                    path: vec![segment],
                },
            );
            made += 1;
        }
    }
    Ok(board)
}

fn millis(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

/// Measures one board.
pub fn run_one(
    name: &'static str,
    target: usize,
    settings: &ViewerSettings,
) -> Result<BenchRow, ViewError> {
    let board = synthetic_board(target, settings)?;

    let started = Instant::now();
    let scene = Scene::from_board(&board, settings)?;
    let convert_ms = millis(started);
    let (prims, items, bounds) = (scene.prims.len(), scene.items.len(), scene.bounds);

    let started = Instant::now();
    let viewer = Viewer::new(scene, settings.clone());
    let index_ms = millis(started);

    // the points to pick at: across the whole board
    let mut rng = Lcg(settings.bench_seed.wrapping_add(7919));
    let points: Vec<P> = (0..settings.bench_picks)
        .map(|_| {
            P::new(
                bounds.min.x + rng.unit() * bounds.width(),
                bounds.min.y + rng.unit() * bounds.height(),
            )
        })
        .collect();
    let tolerance = settings.bench_pick_tolerance_mm;

    let started = Instant::now();
    let mut hits = 0usize;
    for p in &points {
        if std::hint::black_box(viewer.pick(*p, tolerance)).is_some() {
            hits += 1;
        }
    }
    let pick_us = millis(started) * 1000.0 / points.len().max(1) as f64;

    let scanned = &points[..settings.bench_linear_picks.min(points.len())];
    let started = Instant::now();
    let mut disagreements = 0usize;
    for p in scanned {
        if std::hint::black_box(viewer.pick_linear(*p, tolerance)) != viewer.pick(*p, tolerance) {
            disagreements += 1;
        }
    }
    // the scans and the picks that check them are timed together; the picks are the small part
    let linear_us = millis(started) * 1000.0 / scanned.len().max(1) as f64;

    let started = Instant::now();
    let all = viewer.visible(bounds);
    let all_ms = millis(started);

    let (centre, w, h) = (bounds.centre(), bounds.width(), bounds.height());
    let part = crate::prim::Bounds {
        min: P::new(centre.x - w / 8.0, centre.y - h / 8.0),
        max: P::new(centre.x + w / 8.0, centre.y + h / 8.0),
    };
    let started = Instant::now();
    let some = viewer.visible(part);
    let part_ms = millis(started);

    Ok(BenchRow {
        name,
        target,
        prims,
        items,
        cells: viewer.grid().cells(),
        convert_ms,
        index_ms,
        picks: points.len(),
        hits,
        pick_us,
        linear_picks: scanned.len(),
        linear_us,
        disagreements,
        all_listed: all.len(),
        all_ms,
        part_listed: some.len(),
        part_ms,
    })
}

/// Measures the three boards of the settings.
pub fn run(settings: &ViewerSettings) -> Result<Vec<BenchRow>, ViewError> {
    check(settings)?;
    Ok(vec![
        run_one("small", settings.bench_small, settings)?,
        run_one("medium", settings.bench_medium, settings)?,
        run_one("large", settings.bench_large, settings)?,
    ])
}

/// The report as text: one line per board, and what was and was not measured.
pub fn report(rows: &[BenchRow], lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(Msg::BenchTitle.tr(lang));
    out.push('\n');
    out.push_str(&format!(
        "{:<7} {:>8} {:>8} {:>7} {:>11} {:>9} {:>9} {:>11} {:>8} {:>10} {:>10}\n",
        "board",
        "prims",
        "items",
        "cells",
        "convert_ms",
        "index_ms",
        "pick_us",
        "scan_us",
        "differ",
        "window_ms",
        "part_ms"
    ));
    for r in rows {
        out.push_str(&format!(
            "{:<7} {:>8} {:>8} {:>7} {:>11.2} {:>9.2} {:>9.2} {:>11.1} {:>8} {:>10.3} {:>10.3}\n",
            r.name,
            r.prims,
            r.items,
            r.cells,
            r.convert_ms,
            r.index_ms,
            r.pick_us,
            r.linear_us,
            r.disagreements,
            r.all_ms,
            r.part_ms
        ));
    }
    for r in rows {
        out.push_str(&format!(
            "{}: {} / {} picks hit, {} scans, {} listed for the whole board, {} for a sixteenth\n",
            r.name, r.hits, r.picks, r.linear_picks, r.all_listed, r.part_listed
        ));
    }
    out.push_str(Msg::BenchNote.tr(lang));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> ViewerSettings {
        ViewerSettings::parse(
            "bench_small = 100\nbench_medium = 600\nbench_large = 2500\nbench_picks = 120\nbench_linear_picks = 40\n",
        )
        .unwrap()
    }

    #[test]
    fn the_boards_have_the_size_asked_for() {
        let settings = small();
        for target in [100, 600, 2500] {
            let board = synthetic_board(target, &settings).unwrap();
            let scene = Scene::from_board(&board, &settings).unwrap();
            assert_eq!(scene.prims.len(), target, "target {target}");
            assert_eq!(scene.skipped, 0);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_board_and_another_seed_another_one() {
        let settings = small();
        let a = synthetic_board(300, &settings).unwrap();
        let b = synthetic_board(300, &settings).unwrap();
        assert_eq!(a.to_bytes().unwrap(), b.to_bytes().unwrap());
        let other = ViewerSettings::parse("bench_seed = 99").unwrap();
        let c = synthetic_board(300, &other).unwrap();
        assert_ne!(a.to_bytes().unwrap(), c.to_bytes().unwrap());
    }

    #[test]
    fn the_board_has_every_kind_of_item() {
        let board = synthetic_board(2000, &small()).unwrap();
        assert!(!board.tracks.is_empty());
        assert!(!board.vias.is_empty());
        assert!(!board.zones.is_empty());
        assert!(!board.silkscreen.is_empty());
        assert!(board
            .tracks
            .values()
            .any(|t| matches!(t.path[0], Segment::Arc { .. })));
    }

    #[test]
    fn the_run_gives_three_rows_and_the_index_always_agrees_with_the_scan() {
        let rows = run(&small()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows.iter().map(|r| r.name).collect::<Vec<_>>(),
            vec!["small", "medium", "large"]
        );
        for r in &rows {
            assert_eq!(r.prims, r.target);
            assert_eq!(r.disagreements, 0, "{}", r.name);
            assert_eq!(r.picks, 120);
            assert_eq!(r.linear_picks, 40);
            assert!(r.hits <= r.picks);
            assert_eq!(r.all_listed, r.prims);
            assert!(r.part_listed < r.all_listed);
            assert!(r.convert_ms >= 0.0 && r.index_ms >= 0.0 && r.pick_us >= 0.0);
            assert!(r.cells >= 1);
        }
        assert!(rows[0].prims < rows[1].prims && rows[1].prims < rows[2].prims);
    }

    #[test]
    fn options_that_make_no_sense_are_refused_with_their_code() {
        let tiny = ViewerSettings::parse("bench_small = 5").unwrap();
        assert_eq!(run(&tiny).unwrap_err().code, BENCH_INVALID);
        let more = ViewerSettings::parse("bench_picks = 10\nbench_linear_picks = 20").unwrap();
        assert_eq!(run(&more).unwrap_err().code, BENCH_INVALID);
    }

    #[test]
    fn the_report_names_the_boards_and_says_what_was_not_measured_in_each_language() {
        let rows = run(&small()).unwrap();
        for lang in Lang::ALL {
            let text = report(&rows, lang);
            assert!(text.starts_with(Msg::BenchTitle.tr(lang)));
            assert!(text.contains(Msg::BenchNote.tr(lang)));
            for name in ["small", "medium", "large"] {
                assert!(text.contains(name));
            }
        }
        let text = report(&rows, Lang::En);
        assert!(text.contains("Drawing on a screen was not measured"));
        assert!(text.contains("convert_ms") && text.contains("pick_us"));
    }
}
