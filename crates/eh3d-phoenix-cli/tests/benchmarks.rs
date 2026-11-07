// SPDX-License-Identifier: GPL-3.0-or-later
//! Benchmarks of the readers and of the rule check on boards and files made by the test itself
//! from fixed formulas (nothing large is kept in the repository): the KiCad board and schematic
//! readers, the EAGLE board reader, the DXF reader and its outline search, the Gerber and Excellon
//! readers and the copper clearance check. Each run is made on a small and on a large input (four
//! times as many objects); the test fails when a run takes longer than the limit of
//! `config/performance.cfg`, or when the large run takes much more than four times as long as the
//! small one, which is what work that grows with the square of the object count does.
//!
//! The limits are in a configuration file and are far above what a computer needs. The clearance
//! check is also checked by counting and not by timing: the candidates the spatial index hands out
//! for each object stay below a number of the file, the same on every computer, and the index and
//! the pass over every pair give the same report.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use eh3d_phoenix_board::{Board, Layer, LayerKind, Net, Segment, Track};
use eh3d_phoenix_corpus::suite::facts::id_source;
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use eh3d_phoenix_rules::{check_board_with, clearance_work, IndexMode, RuleValues, Severity};
use eh3d_phoenix_spatial::SpatialConfig;

const MM: i64 = 1_000_000;

/// The timing tests share the computer; one at a time, so that they do not slow each other down.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------------------------
// the settings

/// The text of `config/performance.cfg` as the program has it built in.
const DEFAULT_FILE: &str = r#"# Settings of the benchmarks (`crates/eh3d-phoenix-cli/tests/benchmarks.rs`).
# The test makes boards and files of a growing size from fixed formulas (nothing is kept as a file),
# gives them to the readers of KiCad, EAGLE, DXF, Gerber and Excellon files and to the rule check,
# and measures how long each takes. A run that takes longer than `max_ms`, or that takes much more
# than four times as long on four times as many objects, fails the test.
# The limits are far above what a computer needs, so that a slow one does not fail the test; make
# them tighter on a computer of your own. Another file can be named with the environment variable
# PHOENIX_BENCH_CFG. Every line is required; a key that is not here is refused.

# The number of objects (tracks, wires, holes, entries) of the small and of the large run. The
# large one is four times the small one: work that grows with the object count takes about four
# times as long, work that grows with its square about sixteen times as long.
small_size = 5000
large_size = 20000

# How many times each run is made; the shortest time counts, so that a pause of the computer does
# not decide the result.
repeats = 3

# The longest time of a run, in milliseconds.
max_ms = 20000

# The most the time of the large run may be of the time of the small run, in percent. Four times
# the work is 400 percent; 1200 leaves room for the noise of a shared computer and still fails
# a reader that takes time with the square of the number of objects.
max_growth_percent = 1200

# A small run shorter than this, in milliseconds, is too short to compare: only `max_ms` is checked.
min_ms_for_growth = 50

# The closed contours of the drawing in the DXF run. The reader refuses more than `max_contours`
# of `config/dxf.cfg` (5000), so this stays below it.
dxf_contours = 4000

# The most candidates, for each object, that the spatial index may hand to the clearance check
# on the lattice board of the test (counted, not timed, so the same on every computer).
max_candidates_per_item = 64
"#;

/// The contents of `config/performance.cfg`.
#[derive(Debug, PartialEq, Eq)]
struct Settings {
    small_size: usize,
    large_size: usize,
    repeats: usize,
    max_ms: u64,
    max_growth_percent: u64,
    min_ms_for_growth: u64,
    dxf_contours: usize,
    max_candidates_per_item: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            small_size: 5000,
            large_size: 20000,
            repeats: 3,
            max_ms: 20000,
            max_growth_percent: 1200,
            min_ms_for_growth: 50,
            dxf_contours: 4000,
            max_candidates_per_item: 64,
        }
    }
}

/// The text of the file with the default values, as `config/performance.cfg` holds it.
fn default_file() -> String {
    DEFAULT_FILE.to_owned()
}

const KEYS: [&str; 8] = [
    "small_size",
    "large_size",
    "repeats",
    "max_ms",
    "max_growth_percent",
    "min_ms_for_growth",
    "dxf_contours",
    "max_candidates_per_item",
];

/// Reads the settings: `key = value` lines, comments and blank lines; every key once and no
/// other key.
fn parse_settings(text: &str) -> Result<Settings, String> {
    let mut found: BTreeMap<String, u64> = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: no '='", n + 1))?;
        let key = key.trim();
        if !KEYS.contains(&key) {
            return Err(format!("line {}: unknown key {key}", n + 1));
        }
        let value: u64 = value
            .trim()
            .parse()
            .map_err(|_| format!("line {}: {key} is not a whole number", n + 1))?;
        if found.insert(key.to_owned(), value).is_some() {
            return Err(format!("line {}: {key} twice", n + 1));
        }
    }
    let get = |key: &str| {
        found
            .get(key)
            .copied()
            .ok_or_else(|| format!("{key} is missing"))
    };
    let to_usize =
        |key: &str| usize::try_from(get(key)?).map_err(|_| format!("{key} is too large"));
    let settings = Settings {
        small_size: to_usize("small_size")?,
        large_size: to_usize("large_size")?,
        repeats: to_usize("repeats")?,
        max_ms: get("max_ms")?,
        max_growth_percent: get("max_growth_percent")?,
        min_ms_for_growth: get("min_ms_for_growth")?,
        dxf_contours: to_usize("dxf_contours")?,
        max_candidates_per_item: get("max_candidates_per_item")?,
    };
    if settings.small_size < 100
        || settings.large_size <= settings.small_size
        || settings.large_size > 100_000
        || settings.repeats == 0
        || settings.repeats > 20
        || settings.max_ms == 0
        || settings.max_growth_percent == 0
        || settings.dxf_contours < 10
        || settings.max_candidates_per_item == 0
    {
        return Err("a value is outside its range".to_owned());
    }
    Ok(settings)
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The settings of the file named by `PHOENIX_BENCH_CFG`, or else of `config/performance.cfg`.
fn settings() -> Settings {
    let path = std::env::var_os("PHOENIX_BENCH_CFG")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("config/performance.cfg"));
    let text = std::fs::read_to_string(&path).unwrap();
    parse_settings(&text).unwrap()
}

// ---------------------------------------------------------------------------------------------
// the inputs: every one is made from a count, always the same for a count

/// A KiCad board with `n` tracks on a lattice 2 mm apart in three nets, `n / 4` vias, a rectangle
/// for the outline with `n / 50` rectangular openings, and a zone with `n / 10` corners.
fn kicad_board(n: usize) -> String {
    let rows = n / 200 + 1;
    let (width, height) = (400.0_f64 + 10.0, rows as f64 * 2.0 + 10.0);
    let mut t = String::with_capacity(n * 120);
    t.push_str("(kicad_pcb (version 20221018) (generator pcbnew)\n  (general (thickness 1.6))\n");
    t.push_str("  (paper \"A3\")\n  (layers\n    (0 \"F.Cu\" signal)\n    (31 \"B.Cu\" signal)\n");
    t.push_str("    (44 \"Edge.Cuts\" user)\n  )\n  (net 0 \"\")\n");
    for k in 1..=3 {
        let _ = writeln!(t, "  (net {k} \"N{k}\")");
    }
    for (a, b) in [
        ((0.0, 0.0), (width, 0.0)),
        ((width, 0.0), (width, height)),
        ((width, height), (0.0, height)),
        ((0.0, height), (0.0, 0.0)),
    ] {
        let _ = writeln!(
            t,
            "  (gr_line (start {} {}) (end {} {}) (layer \"Edge.Cuts\") (width 0.1))",
            a.0, a.1, b.0, b.1
        );
    }
    for k in 0..n / 50 {
        let (x, y) = (5.0 + (k % 100) as f64 * 4.0, 5.0 + (k / 100) as f64 * 4.0);
        let _ = writeln!(
            t,
            "  (gr_rect (start {x} {y}) (end {} {}) (layer \"Edge.Cuts\") (width 0.1))",
            x + 1.0,
            y + 1.0
        );
    }
    for k in 0..n {
        let (x, y) = ((k % 200) as f64 * 2.0 + 3.0, (k / 200) as f64 * 2.0 + 3.0);
        let _ = writeln!(
            t,
            "  (segment (start {x} {y}) (end {} {y}) (width 0.25) (layer \"F.Cu\") (net {}) (tstamp b{k}))",
            x + 1.0,
            1 + k % 3
        );
    }
    for k in 0..n / 4 {
        let (x, y) = ((k % 200) as f64 * 2.0 + 3.5, (k / 200) as f64 * 2.0 + 4.0);
        let _ = writeln!(
            t,
            "  (via (at {x} {y}) (size 0.8) (drill 0.4) (layers \"F.Cu\" \"B.Cu\") (net 1) (tstamp c{k}))"
        );
    }
    let corners = (n / 10).max(8);
    t.push_str(
        "  (zone (net 1) (net_name \"N1\") (layer \"B.Cu\") (tstamp d1) (hatch edge 0.5)
    (connect_pads (clearance 0.3)) (min_thickness 0.25)
    (polygon (pts",
    );
    for k in 0..corners {
        let angle = k as f64 / corners as f64 * std::f64::consts::TAU;
        let _ = write!(
            t,
            " (xy {:.3} {:.3})",
            width / 2.0 + (width / 2.0 - 1.0) * angle.cos(),
            height / 2.0 + (height / 2.0 - 1.0) * angle.sin()
        );
    }
    t.push_str(")))\n)\n");
    t
}

/// A KiCad schematic with one bus of 128 members and `n` bus entries, each ending at the label of
/// a member, 5 mm apart on a lattice.
fn kicad_schematic(n: usize) -> String {
    let mut t = String::with_capacity(n * 260);
    t.push_str("(kicad_sch (version 20211123) (generator eeschema)\n");
    t.push_str("  (uuid \"00000000-0000-0000-0000-0000000000b1\")\n  (paper \"A0\")\n");
    t.push_str("  (title_block (title \"Bus\"))\n");
    t.push_str(
        "  (bus (pts (xy 90 40) (xy 90 80)) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid \"bus1\"))\n",
    );
    t.push_str(
        "  (label \"D[0..127]\" (at 90 40 0) (effects (font (size 1.27 1.27))) (uuid \"lb0\"))\n",
    );
    for k in 0..n {
        let (x, y) = (100 + 5 * (k % 200), 50 + 5 * (k / 200));
        let _ = writeln!(
            t,
            "  (bus_entry (at {x} {y}) (size 2.54 2.54) (stroke (width 0) (type default) (color 0 0 0 0)) (uuid \"e{k}\"))"
        );
        let _ = writeln!(
            t,
            "  (label \"D{}\" (at {:.2} {:.2} 0) (effects (font (size 1.27 1.27))) (uuid \"l{k}\"))",
            k % 128,
            x as f64 + 2.54,
            y as f64 + 2.54
        );
    }
    t.push_str("  (sheet_instances (path \"/\" (page \"1\")))\n)\n");
    t
}

/// An EAGLE board with `n` wires on a lattice 2 mm apart in signals of 100 wires each, a via in
/// every fifth wire and a pour with `n / 10` corners.
fn eagle_board(n: usize) -> String {
    let mut t = String::with_capacity(n * 90);
    t.push_str(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n",
    );
    t.push_str("<eagle version=\"9.6.2\">\n<drawing>\n<layers>\n");
    t.push_str("<layer number=\"1\" name=\"Top\" color=\"4\" fill=\"1\" visible=\"yes\" active=\"yes\"/>\n");
    t.push_str("<layer number=\"16\" name=\"Bottom\" color=\"1\" fill=\"1\" visible=\"yes\" active=\"yes\"/>\n");
    t.push_str("<layer number=\"20\" name=\"Dimension\" color=\"15\" fill=\"1\" visible=\"yes\" active=\"yes\"/>\n");
    t.push_str("</layers>\n<board>\n<plain>\n");
    let (width, height) = (410.0_f64, (n / 200 + 1) as f64 * 2.0 + 10.0);
    for (a, b) in [
        ((0.0, 0.0), (width, 0.0)),
        ((width, 0.0), (width, height)),
        ((width, height), (0.0, height)),
        ((0.0, height), (0.0, 0.0)),
    ] {
        let _ = writeln!(
            t,
            "<wire x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" width=\"0\" layer=\"20\"/>",
            a.0, a.1, b.0, b.1
        );
    }
    t.push_str("</plain>\n<elements>\n</elements>\n<signals>\n");
    let mut k = 0;
    let mut signal = 0;
    while k < n {
        let _ = writeln!(t, "<signal name=\"S{signal}\" class=\"0\">");
        for _ in 0..100_usize.min(n - k) {
            let (x, y) = ((k % 200) as f64 * 2.0 + 3.0, (k / 200) as f64 * 2.0 + 3.0);
            let _ = writeln!(
                t,
                "<wire x1=\"{x}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" width=\"0.25\" layer=\"1\"/>",
                x + 1.0
            );
            if k % 5 == 0 {
                let _ = writeln!(
                    t,
                    "<via x=\"{x}\" y=\"{y}\" extent=\"1-16\" drill=\"0.4\" diameter=\"0.8\"/>"
                );
            }
            k += 1;
        }
        t.push_str("</signal>\n");
        signal += 1;
    }
    let corners = (n / 10).max(8);
    t.push_str("<signal name=\"POUR\" class=\"0\">\n<polygon width=\"0.2\" layer=\"16\">\n");
    for c in 0..corners {
        let angle = c as f64 / corners as f64 * std::f64::consts::TAU;
        let _ = writeln!(
            t,
            "<vertex x=\"{:.3}\" y=\"{:.3}\"/>",
            width / 2.0 + (width / 2.0 - 1.0) * angle.cos(),
            height / 2.0 + (height / 2.0 - 1.0) * angle.sin()
        );
    }
    t.push_str("</polygon>\n</signal>\n</signals>\n</board>\n</drawing>\n</eagle>\n");
    t
}

/// A DXF drawing with a large closed outline and `n` small squares inside it, as closed
/// polylines on the layer EDGE.
fn dxf_drawing(n: usize) -> String {
    let mut t = String::with_capacity(n * 140);
    t.push_str("  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1015\n  9\n$INSUNITS\n 70\n     4\n  0\nENDSEC\n");
    t.push_str("  0\nSECTION\n  2\nENTITIES\n");
    let polyline = |t: &mut String, corners: [(f64, f64); 4]| {
        t.push_str("  0\nLWPOLYLINE\n  8\nEDGE\n 90\n        4\n 70\n     1\n");
        for (x, y) in corners {
            let _ = write!(t, " 10\n{x:.3}\n 20\n{y:.3}\n");
        }
    };
    polyline(
        &mut t,
        [(0.0, 0.0), (1200.0, 0.0), (1200.0, 1200.0), (0.0, 1200.0)],
    );
    // n squares of 4 mm, 10 mm apart: 60 to a row
    for k in 0..n {
        let (x, y) = (10.0 + (k % 60) as f64 * 12.0, 10.0 + (k / 60) as f64 * 12.0);
        polyline(
            &mut t,
            [(x, y), (x + 4.0, y), (x + 4.0, y + 4.0), (x, y + 4.0)],
        );
    }
    t.push_str("  0\nENDSEC\n  0\nEOF\n");
    t
}

/// A Gerber copper layer with `n` flashes of three apertures and `n` short tracks.
fn gerber_layer(n: usize) -> String {
    let mut t = String::with_capacity(n * 60);
    t.push_str(
        "%TF.FileFunction,Copper,L1,Top*%\n%TF.FilePolarity,Positive*%\n%FSLAX46Y46*%\n%MOMM*%\n",
    );
    t.push_str("%ADD10C,0.2*%\n%ADD11R,1.2X0.6*%\n%ADD12C,0.8*%\nD10*\n");
    for k in 0..n {
        let (x, y) = ((k % 200) as i64 * 2 + 1, (k / 200) as i64 * 2 + 1);
        let _ = write!(
            t,
            "X{}Y{}D02*\nG01*\nX{}Y{}D01*\n",
            x * MM,
            y * MM,
            (x * MM) + MM / 2,
            y * MM
        );
    }
    for k in 0..n {
        let (x, y) = ((k % 200) as i64 * 2 + 1, (k / 200) as i64 * 2 + 1);
        let _ = write!(t, "D{}*\nX{}Y{}D03*\n", 11 + k % 2, x * MM, y * MM + MM / 2);
    }
    t.push_str("M02*\n");
    t
}

/// An Excellon drill file with `n` holes of three tools.
fn excellon_drills(n: usize) -> String {
    let mut t = String::with_capacity(n * 24);
    t.push_str(
        "M48\n; #@! TF.FileFunction,Plated,1,2,PTH\nMETRIC,TZ\nT01C0.8\nT02C1.0\nT03C1.2\n%\n",
    );
    for tool in 1..=3_usize {
        let _ = writeln!(t, "T0{tool}");
        for k in (0..n).filter(|k| k % 3 == tool - 1) {
            let _ = writeln!(
                t,
                "X{:.3}Y{:.3}",
                (k % 200) as f64 * 2.0 + 1.0,
                (k / 200) as f64 * 2.0 + 1.0
            );
        }
    }
    t.push_str("M30\n");
    t
}

fn id(n: usize) -> ObjectId {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n as u64).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A board with `n` tracks of 1 mm and 0.25 mm wide on one copper layer, in two nets, `columns`
/// to a row, `dx_mm` apart along a row and `dy_mm` apart from row to row. The nets alternate
/// from row to row.
fn track_board(n: usize, columns: usize, dx_mm: f64, dy_mm: f64) -> Board {
    let mut board = Board::default();
    board.layers.insert(
        id(1),
        Layer {
            name: "Top".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.copper_stack = vec![id(1)];
    for k in 0..2 {
        board.nets.insert(
            id(10 + k),
            Net {
                name: format!("N{k}"),
            },
        );
    }
    for k in 0..n {
        let (col, row) = (k % columns, k / columns);
        let x = 5 * MM + (col as f64 * dx_mm * MM as f64) as i64;
        let y = 5 * MM + (row as f64 * dy_mm * MM as f64) as i64;
        board.tracks.insert(
            id(1000 + k),
            Track {
                net: id(10 + row % 2),
                layer: id(1),
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(x, y),
                    end: pt(x + MM, y),
                }],
            },
        );
    }
    board
}

fn rule_values() -> RuleValues {
    RuleValues {
        min_clearance: Nm::new(200_000).unwrap(),
        min_track_width: Nm::new(100_000).unwrap(),
        ..RuleValues::default()
    }
}

// ---------------------------------------------------------------------------------------------
// the cases

struct Case {
    name: &'static str,
    /// The input for a count.
    make: fn(usize) -> Vec<u8>,
    /// Reads or checks the input; what it found (objects of the board, entities, files...).
    run: fn(&[u8]) -> usize,
    /// The least number of objects `run` must have found for a count (a run that refused its
    /// input would be fast and prove nothing).
    least: fn(usize) -> usize,
    /// True when the time must grow in proportion to the count (the growth test applies).
    linear: bool,
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "KiCad board",
            make: |n| kicad_board(n).into_bytes(),
            run: |b| {
                let mut ids = id_source(1);
                let read = eh3d_phoenix_kicad::read_pcb(
                    b,
                    &eh3d_phoenix_kicad::KicadSettings::default(),
                    &mut ids,
                )
                .unwrap();
                read.board.tracks.len() + read.board.vias.len()
            },
            least: |n| n / 2,
            linear: true,
        },
        Case {
            name: "KiCad schematic with bus entries",
            make: |n| kicad_schematic(n).into_bytes(),
            run: |b| {
                let mut ids = id_source(1);
                let read = eh3d_phoenix_kicad::read_schematic(
                    b,
                    &eh3d_phoenix_kicad::KicadSettings::default(),
                    &mut ids,
                )
                .unwrap();
                read.schematic.bus_entries.len()
            },
            least: |n| n / 2,
            linear: true,
        },
        Case {
            name: "EAGLE board",
            make: |n| eagle_board(n).into_bytes(),
            run: |b| {
                let mut ids = id_source(1);
                let read = eh3d_phoenix_eagle_board::read_board(&text(b), &mut ids).unwrap();
                read.board.tracks.len() + read.board.vias.len()
            },
            least: |n| n / 2,
            linear: true,
        },
        Case {
            name: "Gerber layer",
            make: |n| gerber_layer(n).into_bytes(),
            run: |b| {
                let file = eh3d_phoenix_gerber::import_file(
                    "top.gtl",
                    b,
                    &eh3d_phoenix_gerber::ImportSettings::default(),
                );
                usize::from(matches!(
                    file.content,
                    eh3d_phoenix_gerber::Content::Gerber(_)
                ))
            },
            least: |_| 1,
            linear: true,
        },
        Case {
            name: "Excellon drill file",
            make: |n| excellon_drills(n).into_bytes(),
            run: |b| {
                let file = eh3d_phoenix_gerber::import_file(
                    "holes.drl",
                    b,
                    &eh3d_phoenix_gerber::ImportSettings::default(),
                );
                usize::from(matches!(
                    file.content,
                    eh3d_phoenix_gerber::Content::Drill(_)
                ))
            },
            least: |_| 1,
            linear: true,
        },
        Case {
            name: "rule check of a board",
            make: |n| track_board(n, 200, 2.0, 10.0).to_bytes().unwrap(),
            run: |b| {
                let board = Board::from_bytes(b).unwrap();
                let report = check_board_with(&board, &rule_values(), IndexMode::Grid);
                assert!(report.passes());
                board.tracks.len()
            },
            least: |n| n / 2,
            linear: true,
        },
    ]
}

/// The shortest time of `repeats` runs, and what the last one found.
fn best_of(repeats: usize, case: &Case, input: &[u8]) -> (Duration, usize) {
    let mut best = Duration::MAX;
    let mut found = 0;
    for _ in 0..repeats {
        let started = Instant::now();
        found = (case.run)(input);
        best = best.min(started.elapsed());
    }
    (best, found)
}

// ---------------------------------------------------------------------------------------------
// the tests

#[test]
fn the_settings_file_of_the_repository_is_the_built_in_default_and_is_read_as_it_says() {
    let shipped = std::fs::read_to_string(root().join("config/performance.cfg")).unwrap();
    assert_eq!(shipped.replace("\r\n", "\n"), default_file());
    assert_eq!(
        parse_settings(&default_file()).unwrap(),
        Settings::default()
    );
    // a missing key, an unknown key, a key twice, a value that is no number and a value out of
    // its range are refused with the line
    let one_less = default_file().replace("repeats = 3\n", "");
    assert_eq!(
        parse_settings(&one_less),
        Err("repeats is missing".to_owned())
    );
    let unknown = format!("{}speed = 1\n", default_file());
    assert!(parse_settings(&unknown)
        .unwrap_err()
        .contains("unknown key speed"));
    let twice = format!("{}repeats = 4\n", default_file());
    assert!(parse_settings(&twice)
        .unwrap_err()
        .contains("repeats twice"));
    let word = default_file().replace("repeats = 3", "repeats = many");
    assert!(parse_settings(&word)
        .unwrap_err()
        .contains("repeats is not a whole number"));
    let range = default_file().replace("repeats = 3", "repeats = 0");
    assert_eq!(
        parse_settings(&range),
        Err("a value is outside its range".to_owned())
    );
    let order = default_file().replace("large_size = 20000", "large_size = 100");
    assert!(parse_settings(&order).is_err());
    assert!(parse_settings("repeats\n").unwrap_err().contains("line 1"));
}

#[test]
fn the_inputs_are_the_same_every_time_and_hold_what_their_count_says() {
    for n in [300, 1000] {
        assert_eq!(kicad_board(n), kicad_board(n));
        assert_eq!(eagle_board(n), eagle_board(n));
        assert_eq!(dxf_drawing(n), dxf_drawing(n));
        assert_eq!(kicad_board(n).matches("(segment ").count(), n);
        assert_eq!(kicad_board(n).matches("(via ").count(), n / 4);
        assert_eq!(kicad_schematic(n).matches("(bus_entry ").count(), n);
        assert_eq!(eagle_board(n).matches("layer=\"1\"/>").count(), n);
        assert_eq!(
            eagle_board(n).matches("<signal ").count(),
            n.div_ceil(100) + 1
        );
        assert_eq!(dxf_drawing(n).matches("LWPOLYLINE").count(), n + 1);
        assert_eq!(gerber_layer(n).matches("D03*").count(), n);
        assert_eq!(
            excellon_drills(n)
                .lines()
                .filter(|l| l.starts_with('X'))
                .count(),
            n
        );
    }
}

#[test]
fn the_readers_take_time_in_proportion_to_the_size_of_their_input_and_never_more_than_the_limit() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let settings = settings();
    let limit = Duration::from_millis(settings.max_ms);
    let mut report = String::new();
    let mut problems: Vec<String> = Vec::new();
    for case in cases() {
        let small_input = (case.make)(settings.small_size);
        let large_input = (case.make)(settings.large_size);
        let (small, found_small) = best_of(settings.repeats, &case, &small_input);
        let (large, found_large) = best_of(settings.repeats, &case, &large_input);
        let _ = writeln!(
            report,
            "{}: {} objects {} ms, {} objects {} ms",
            case.name,
            settings.small_size,
            small.as_millis(),
            settings.large_size,
            large.as_millis()
        );
        if found_small < (case.least)(settings.small_size)
            || found_large < (case.least)(settings.large_size)
        {
            problems.push(format!(
                "{}: found {found_small} and {found_large} objects, the input holds more",
                case.name
            ));
        }
        if large > limit {
            problems.push(format!(
                "{}: the large run took {} ms, the limit is {} ms",
                case.name,
                large.as_millis(),
                settings.max_ms
            ));
        }
        if case.linear && small.as_millis() as u64 >= settings.min_ms_for_growth {
            let percent = large.as_nanos() * 100 / small.as_nanos().max(1);
            if percent > u128::from(settings.max_growth_percent) {
                problems.push(format!(
                    "{}: {} times the objects took {percent} percent of the time, the limit is {} percent",
                    case.name,
                    settings.large_size / settings.small_size,
                    settings.max_growth_percent
                ));
            }
        }
    }
    println!("{report}");
    assert!(problems.is_empty(), "{}\n{report}", problems.join("\n"));
}

#[test]
fn the_dxf_outline_search_with_many_contours_finishes_inside_the_limit_and_finds_them_all() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let settings = settings();
    let dxf = eh3d_phoenix_dxf::DxfSettings::default();
    assert!(settings.dxf_contours < dxf.max_contours);
    let input = dxf_drawing(settings.dxf_contours);
    let started = Instant::now();
    let read = eh3d_phoenix_dxf::read_dxf(&input, &dxf).unwrap();
    let outline =
        eh3d_phoenix_dxf::extract_outline(&read.drawing, Some("EDGE"), dxf.join_tolerance_nm, &dxf);
    let took = started.elapsed();
    println!(
        "DXF drawing: {} contours {} ms",
        settings.dxf_contours,
        took.as_millis()
    );
    assert_eq!(outline.stats.contours, settings.dxf_contours + 1);
    assert!(outline.exterior.is_some());
    assert_eq!(outline.holes.len(), settings.dxf_contours);
    assert!(
        took <= Duration::from_millis(settings.max_ms),
        "{} ms against the limit {} ms",
        took.as_millis(),
        settings.max_ms
    );
    // one contour more than the limit of the reader is refused and not searched
    let over = dxf_drawing(dxf.max_contours);
    let read = eh3d_phoenix_dxf::read_dxf(&over, &dxf).unwrap();
    let refused =
        eh3d_phoenix_dxf::extract_outline(&read.drawing, Some("EDGE"), dxf.join_tolerance_nm, &dxf);
    assert!(refused.exterior.is_none());
    assert!(refused
        .report
        .findings
        .iter()
        .any(|f| f.code == eh3d_phoenix_dxf::codes::LIMIT_EXCEEDED));
}

#[test]
fn the_index_hands_each_track_a_few_candidates_and_gives_the_report_of_the_pass_over_every_pair() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let settings = settings();
    let rules = rule_values();
    // counted and not timed: the candidates stay in proportion to the tracks at both sizes
    for n in [settings.small_size, settings.large_size] {
        let board = track_board(n, 200, 2.0, 10.0);
        let work = clearance_work(&board, &rules, IndexMode::Grid, &SpatialConfig::default());
        assert_eq!(work.items, n);
        assert_eq!(work.refused, None);
        assert!(
            work.comparisons <= settings.max_candidates_per_item * n as u64,
            "{} candidates for {n} tracks",
            work.comparisons
        );
        // the pass over every pair would look at n * n
        assert!((work.comparisons as u128) * 100 < (n as u128) * (n as u128));
    }
    // the same report with the index and with the pass over every pair, on a board with
    // violations: rows 0.4 mm apart of different nets leave 0.15 mm between the edges
    for n in [250, 600] {
        let board = track_board(n, 10, 2.0, 0.4);
        let grid = check_board_with(&board, &rules, IndexMode::Grid);
        let naive = check_board_with(&board, &rules, IndexMode::Naive);
        assert!(!grid.passes());
        assert!(grid.count(Severity::Error) > 0);
        assert_eq!(grid.to_json(), naive.to_json());
    }
}
