// SPDX-License-Identifier: GPL-3.0-or-later
//! Hostile input: the files of the corpus and of the examples are changed by a generator of
//! pseudo-random numbers with a fixed seed (cut, repeat a piece, change bytes, put extreme
//! numbers, nest deeply) and every changed file is given to the readers that take such a file. A
//! reader may refuse a file. It may never panic and it may never take longer than the limit.
//!
//! The seed, the number of changed files for each reader and the limits are in
//! `config/fuzz.cfg`; the same seed always makes the same files, on every computer.

use std::collections::BTreeMap;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eh3d_phoenix_board::Board;
use eh3d_phoenix_corpus::suite::facts::id_source;
use eh3d_phoenix_corpus::suite::rng::XorShift;
use eh3d_phoenix_route::live::RuleContext;
use eh3d_phoenix_route::settings::RouteSettings;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The contents of `config/fuzz.cfg`.
#[derive(Debug, PartialEq, Eq)]
struct Settings {
    seed: u64,
    iterations: usize,
    max_input_bytes: usize,
    max_seconds_per_input: u64,
}

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
        if ![
            "seed",
            "iterations",
            "max_input_bytes",
            "max_seconds_per_input",
        ]
        .contains(&key)
        {
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
        seed: get("seed")?,
        iterations: to_usize("iterations")?,
        max_input_bytes: to_usize("max_input_bytes")?,
        max_seconds_per_input: get("max_seconds_per_input")?,
    };
    if settings.iterations == 0
        || settings.max_input_bytes == 0
        || settings.max_seconds_per_input == 0
    {
        return Err(
            "iterations, max_input_bytes and max_seconds_per_input must be above 0".to_owned(),
        );
    }
    Ok(settings)
}

fn settings() -> Settings {
    let text = fs::read_to_string(root().join("config/fuzz.cfg")).unwrap();
    parse_settings(&text).unwrap()
}

// ---------------------------------------------------------------------------------------------
// the generator of changed files

const EXTREMES: [&str; 14] = [
    "9223372036854775807",
    "-9223372036854775808",
    "4294967296",
    "-1",
    "0",
    "1073741824",
    "-1073741825",
    "99999999999999999999999999",
    "1e999",
    "-1e999",
    "NaN",
    "0.00000000000000001",
    "65536",
    "340282366920938463463374607431768211455",
];

const OPENERS: [&str; 6] = ["(", "<a>", "[", "{", "<x a=\"", "(("];

fn insert_at(data: &mut Vec<u8>, at: usize, piece: &[u8]) {
    let at = at.min(data.len());
    let tail = data.split_off(at);
    data.extend_from_slice(piece);
    data.extend_from_slice(&tail);
}

/// One to three changes of `seed`, chosen and placed by `rng`; the result is cut to `cap` bytes.
fn mutate(rng: &mut XorShift, seed: &[u8], cap: usize) -> Vec<u8> {
    let mut data = seed.to_vec();
    for _ in 0..1 + rng.index(3) {
        let len = data.len();
        match rng.index(7) {
            0 => {
                // cut the file somewhere
                let at = rng.index(len + 1);
                data.truncate(at);
            }
            1 => {
                // repeat a piece of the file several times at another place
                if len > 0 {
                    let from = rng.index(len);
                    let count = 1 + rng.index((len - from).min(4096));
                    let piece = data[from..from + count].to_vec();
                    let at = rng.index(len + 1);
                    for _ in 0..1 + rng.index(8) {
                        insert_at(&mut data, at, &piece);
                    }
                }
            }
            2 => {
                // change a few bytes
                for _ in 0..1 + rng.index(8) {
                    if !data.is_empty() {
                        let at = rng.index(data.len());
                        data[at] = u8::try_from(rng.index(256)).unwrap_or(0);
                    }
                }
            }
            3 => {
                // put an extreme number
                let piece = format!(" {} ", EXTREMES[rng.index(EXTREMES.len())]);
                let at = rng.index(len + 1);
                insert_at(&mut data, at, piece.as_bytes());
            }
            4 => {
                // nest deeply
                let piece = OPENERS[rng.index(OPENERS.len())].repeat(100 + rng.index(4000));
                let at = rng.index(len + 1);
                insert_at(&mut data, at, piece.as_bytes());
            }
            5 => {
                // leave out a piece
                if len > 0 {
                    let from = rng.index(len);
                    let count = 1 + rng.index((len - from).min(512));
                    data.drain(from..from + count);
                }
            }
            _ => {
                // an extreme number in the place of a number of the file
                let start = rng.index(len.max(1)).min(len);
                if let Some(rel) = data[start..].iter().position(|b| b.is_ascii_digit()) {
                    let from = start + rel;
                    let mut to = from;
                    while to < data.len() && (data[to].is_ascii_digit() || data[to] == b'.') {
                        to += 1;
                    }
                    let tail = data.split_off(to);
                    data.truncate(from);
                    data.extend_from_slice(EXTREMES[rng.index(EXTREMES.len())].as_bytes());
                    data.extend_from_slice(&tail);
                }
            }
        }
    }
    data.truncate(cap);
    data
}

// ---------------------------------------------------------------------------------------------
// the seeds: the files of the repository, in a fixed order

type Seed = (String, Vec<u8>);

fn collect(folder: &Path, base: &Path, out: &mut Vec<Seed>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, base, out);
            continue;
        }
        let name = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        if name.contains("expect") || name.ends_with(".expected") {
            continue;
        }
        if let Ok(bytes) = fs::read(&path) {
            if bytes.len() <= 256 * 1024 {
                out.push((name, bytes));
            }
        }
    }
}

fn all_seeds() -> Vec<Seed> {
    let base = root();
    let mut out = Vec::new();
    for folder in ["tests/corpus", "tests/dxf", "examples", "config"] {
        collect(&base.join(folder), &base, &mut out);
    }
    out.sort();
    out
}

struct Context {
    board: Board,
}

struct Reader {
    name: &'static str,
    /// The seeds are the files whose name ends in one of these and holds `within`.
    suffixes: &'static [&'static str],
    within: &'static str,
    /// Seeds that are made here because the repository has no file of the kind.
    extra: Vec<Seed>,
    run: fn(&[u8], &Context),
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn reader(
    name: &'static str,
    suffixes: &'static [&'static str],
    run: fn(&[u8], &Context),
) -> Reader {
    Reader {
        name,
        suffixes,
        within: "",
        extra: Vec::new(),
        run,
    }
}

fn readers() -> Vec<Reader> {
    let mut list = vec![
        reader("eagle xml", &[".brd", ".sch", ".lbr"], |b, _| {
            let _ = eh3d_phoenix_eagle::read_bytes(b, &eh3d_phoenix_eagle::Limits::default());
        }),
        reader("eagle board", &[".brd"], |b, _| {
            let mut ids = id_source(1);
            let _ = eh3d_phoenix_eagle_board::read_board(&text(b), &mut ids);
        }),
        reader("eagle schematic", &[".sch"], |b, _| {
            let mut ids = id_source(1);
            let _ = eh3d_phoenix_eagle_board::read_schematic(&text(b), &BTreeMap::new(), &mut ids);
        }),
        reader("eagle deep check", &[".brd", ".sch", ".lbr"], |b, _| {
            let _ = eh3d_phoenix_eagle_deep::check_eagle(
                &text(b),
                &eh3d_phoenix_eagle_deep::LayerMap::default(),
            );
        }),
        reader("library editor", &[".lbr"], |b, _| {
            let _ = eh3d_phoenix_libedit::read_library(b, &eh3d_phoenix_eagle::Limits::default());
        }),
        reader("kicad board", &[".kicad_pcb"], |b, _| {
            let mut ids = id_source(1);
            let _ = eh3d_phoenix_kicad::read_pcb(
                b,
                &eh3d_phoenix_kicad::KicadSettings::default(),
                &mut ids,
            );
        }),
        reader("kicad schematic", &[".kicad_sch"], |b, _| {
            let mut ids = id_source(1);
            let _ = eh3d_phoenix_kicad::read_schematic(
                b,
                &eh3d_phoenix_kicad::KicadSettings::default(),
                &mut ids,
            );
        }),
        reader("kicad footprint", &[".kicad_mod"], |b, _| {
            let mut ids = id_source(1);
            let _ = eh3d_phoenix_kicad::read_footprint(
                b,
                &eh3d_phoenix_kicad::KicadSettings::default(),
                &mut ids,
            );
        }),
        reader("kicad symbols", &[".kicad_sym"], |b, _| {
            let mut ids = id_source(1);
            let _ = eh3d_phoenix_kicad::read_symbol_library(
                b,
                &eh3d_phoenix_kicad::KicadSettings::default(),
                &mut ids,
            );
        }),
        reader("kicad project", &[".kicad_pro"], |b, _| {
            let _ =
                eh3d_phoenix_kicad::read_project(b, &eh3d_phoenix_kicad::KicadSettings::default());
        }),
        reader("gerber and excellon", &[".gtl", ".gbr", ".drl"], |b, _| {
            let _ = eh3d_phoenix_gerber::import_file(
                "x.gtl",
                b,
                &eh3d_phoenix_gerber::ImportSettings::default(),
            );
        }),
        reader("dxf", &[".dxf"], |b, _| {
            let _ = eh3d_phoenix_dxf::read_dxf(&text(b), &eh3d_phoenix_dxf::DxfSettings::default());
        }),
        reader("board json", &[".board.json"], |b, _| {
            if let Ok(board) = Board::from_bytes(b) {
                let _ = board.validate();
            }
        }),
        reader("autorouter session", &[".ses"], |b, ctx| {
            let mut ids = id_source(1_000_000);
            let _ = eh3d_phoenix_route::import_ses(
                &ctx.board,
                &RuleContext::default(),
                &RouteSettings::default(),
                &text(b),
                &mut ids,
            );
        }),
        reader("spice netlist", &[".cir"], |b, _| {
            let settings = eh3d_phoenix_sim::SimSettings::default();
            if let Ok(netlist) = eh3d_phoenix_sim::parse(&text(b), &settings) {
                if let Ok(circuit) = eh3d_phoenix_sim::Circuit::compile(&netlist, &settings) {
                    let _ = eh3d_phoenix_sim::operating_point(&circuit, &settings);
                }
            }
        }),
        reader("script", &[".phs"], |b, _| {
            let settings = eh3d_phoenix_script::ScriptSettings::default();
            let source = text(b);
            let _ = eh3d_phoenix_script::parse_script(&source, &settings);
            let _ = eh3d_phoenix_script::check_text(&source, &settings);
            let _ = eh3d_phoenix_script::analyse_text("x.phs", &source, &settings);
        }),
        reader("settings files", &[".cfg"], |b, _| {
            let source = text(b);
            let _ = eh3d_phoenix_kicad::KicadSettings::parse(&source);
            let _ = eh3d_phoenix_gerber::ImportSettings::parse(&source);
            let _ = eh3d_phoenix_dxf::DxfSettings::parse(&source);
            let _ = eh3d_phoenix_sim::SimSettings::parse(&source);
            let _ = eh3d_phoenix_script::ScriptSettings::parse(&source);
            let _ = eh3d_phoenix_parts::PartsSettings::parse(&source);
            let _ = eh3d_phoenix_ipclib::IpcSettings::parse(&source);
            let _ = eh3d_phoenix_eagle_deep::LayerMap::parse(&source);
            let _ = eh3d_phoenix_eagle_deep::DruFile::parse(&source);
        }),
    ];
    let mut families = reader("footprint family", &[".json"], |b, _| {
        let _ = eh3d_phoenix_ipclib::Family::parse(&text(b));
    });
    families.within = "ipc_families";
    list.push(families);

    let mut parts_csv = reader("parts csv", &[], |b, _| {
        let _ = eh3d_phoenix_parts::parse_csv(
            &text(b),
            "x.csv",
            &eh3d_phoenix_parts::PartsSettings::default(),
        );
    });
    parts_csv.extra = vec![(
        "parts.csv".to_owned(),
        b"mpn,manufacturer,value\nRC0603FR-0710KL,Yageo,10k\n,,\n".to_vec(),
    )];
    list.push(parts_csv);

    let mut parts_json = reader("parts json", &[], |b, _| {
        let _ = eh3d_phoenix_parts::parse_json(
            &text(b),
            "x.json",
            &eh3d_phoenix_parts::PartsSettings::default(),
        );
    });
    parts_json.extra = vec![(
        "parts.json".to_owned(),
        format!(
            "{{\"format\":\"{}\",\"parts\":[{{\"mpn\":\"A\",\"manufacturer\":\"B\"}}]}}",
            eh3d_phoenix_parts::FORMAT
        )
        .into_bytes(),
    )];
    list.push(parts_json);

    let mut touchstone = reader("touchstone", &[], |b, _| {
        let _ = eh3d_phoenix_rf::Touchstone::parse(
            &text(b),
            2,
            &eh3d_phoenix_rf::RfSettings::default(),
        );
    });
    touchstone.extra = vec![(
        "two_port.s2p".to_owned(),
        b"! a thru\n# GHZ S MA R 50\n1.0 0.01 -30 1.0 90 0.01 60 0.02 -20\n2.0 0.02 -40 0.9 80 0.02 50 0.03 -10\n"
            .to_vec(),
    )];
    list.push(touchstone);

    let mut dru = reader("design rules of eagle", &[], |b, _| {
        let _ = eh3d_phoenix_eagle_deep::DruFile::parse(&text(b));
    });
    dru.extra = vec![(
        "rules.dru".to_owned(),
        b"description[en] = test\nlayerSetup = (1*16)\nmdWireWire = 6mil\nrvPadTop = 0.25\n"
            .to_vec(),
    )];
    list.push(dru);

    let mut zip = reader("fabrication package", &[], |b, _| {
        let _ = eh3d_phoenix_export::zipfile::read_zip(b);
        let _ = eh3d_phoenix_export::fabpack::verify_pack(b);
    });
    let entries = vec![
        ("a.gbr".to_owned(), b"G04 a*\nM02*\n".to_vec()),
        ("manifest.json".to_owned(), b"{}".to_vec()),
    ];
    zip.extra = vec![(
        "pack.zip".to_owned(),
        eh3d_phoenix_export::zipfile::write_zip(&entries).unwrap(),
    )];
    list.push(zip);
    list
}

fn seeds_of<'a>(reader: &'a Reader, all: &'a [Seed]) -> Vec<&'a Seed> {
    let mut out: Vec<&Seed> = all
        .iter()
        .filter(|(name, _)| {
            reader.suffixes.iter().any(|s| name.ends_with(s)) && name.contains(reader.within)
        })
        .collect();
    out.extend(reader.extra.iter());
    out
}

fn context() -> Context {
    let bytes = fs::read(root().join("tests/corpus/route/mini_pair.board.json")).unwrap();
    Context {
        board: Board::from_bytes(&bytes).unwrap(),
    }
}

fn message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "a panic without a message".to_owned()
    }
}

/// Runs `reader` on `input`; the problem found, if any.
fn try_once(reader: &Reader, input: &[u8], ctx: &Context, limit: Duration) -> Option<String> {
    let started = Instant::now();
    let result = catch_unwind(AssertUnwindSafe(|| (reader.run)(input, ctx)));
    let took = started.elapsed();
    match result {
        Err(payload) => Some(format!("panic: {}", message(payload.as_ref()))),
        Ok(()) if took > limit => Some(format!("took {} s", took.as_secs())),
        Ok(()) => None,
    }
}

#[test]
fn no_reader_panics_or_takes_too_long_on_a_changed_file_of_the_corpus() {
    let settings = settings();
    let all = all_seeds();
    assert!(all.len() > 100, "{} seed files", all.len());
    let ctx = context();
    let limit = Duration::from_secs(settings.max_seconds_per_input);
    let mut problems: Vec<String> = Vec::new();
    let mut tried = 0usize;
    for reader in readers() {
        let seeds = seeds_of(&reader, &all);
        assert!(
            !seeds.is_empty(),
            "no seed file for the reader {}",
            reader.name
        );
        // the files as they are
        for (name, bytes) in &seeds {
            tried += 1;
            if let Some(problem) = try_once(&reader, bytes, &ctx, limit) {
                problems.push(format!("{} on {name} as it is: {problem}", reader.name));
            }
        }
        // the changed files: the generator starts again for every reader, so adding a reader
        // does not change the files that the others get
        let mut rng = XorShift::new(settings.seed);
        for i in 0..settings.iterations {
            let (name, bytes) = seeds[i % seeds.len()];
            let changed = mutate(&mut rng, bytes, settings.max_input_bytes);
            tried += 1;
            if let Some(problem) = try_once(&reader, &changed, &ctx, limit) {
                problems.push(format!(
                    "{} on {name}, change {i} ({} bytes): {problem}",
                    reader.name,
                    changed.len()
                ));
            }
        }
    }
    assert!(tried > 2000, "{tried} files tried");
    assert!(
        problems.is_empty(),
        "{} problems, the first ones: {:#?}",
        problems.len(),
        &problems[..problems.len().min(10)]
    );
}

#[test]
fn the_generator_makes_the_same_files_for_the_same_seed_and_other_files_for_another() {
    let seed = b"(kicad_pcb (version 1) (net 0 \"\") (gr_line (start 1 2) (end 3 4)))".to_vec();
    let make = |s: u64| -> Vec<Vec<u8>> {
        let mut rng = XorShift::new(s);
        (0..20).map(|_| mutate(&mut rng, &seed, 1 << 20)).collect()
    };
    assert_eq!(make(7), make(7));
    assert_ne!(make(7), make(8));
    // the cap is respected
    let mut rng = XorShift::new(3);
    for _ in 0..50 {
        assert!(mutate(&mut rng, &seed, 64).len() <= 64);
    }
}

#[test]
fn the_settings_of_the_test_are_strict_and_the_shipped_ones_are_read() {
    let shipped = settings();
    assert!(shipped.iterations >= 100);
    assert!(shipped.max_input_bytes >= 1 << 16);
    let good = "seed = 1\niterations = 2\nmax_input_bytes = 3\nmax_seconds_per_input = 4\n";
    assert_eq!(
        parse_settings(good).unwrap(),
        Settings {
            seed: 1,
            iterations: 2,
            max_input_bytes: 3,
            max_seconds_per_input: 4,
        }
    );
    for bad in [
        "seed = 1\niterations = 2\nmax_input_bytes = 3\n",
        "seed = 1\nseed = 2\niterations = 2\nmax_input_bytes = 3\nmax_seconds_per_input = 4\n",
        "seed = 1\niterations = two\nmax_input_bytes = 3\nmax_seconds_per_input = 4\n",
        "seed = 1\niterations = 2\nmax_input_bytes = 3\nmax_seconds_per_input = 4\nextra = 1\n",
        "seed = 1\niterations = 0\nmax_input_bytes = 3\nmax_seconds_per_input = 4\n",
        "seed 1\n",
    ] {
        assert!(parse_settings(bad).is_err(), "{bad}");
    }
}

// ---------------------------------------------------------------------------------------------
// the same seeds cut at fixed places and repeated, made here and never kept

/// The pieces that were added to the corpus for the newer readers and writers: they must be seeds
/// of the readers that take them.
const NEWER_SEEDS: [&str; 14] = [
    "tests/corpus/kicad/outline_cutouts.kicad_pcb",
    "tests/corpus/kicad/outline_poly_hole.kicad_pcb",
    "tests/corpus/kicad/pad_shapes.kicad_pcb",
    "tests/corpus/kicad/zones_keepout.kicad_pcb",
    "tests/corpus/kicad/bus_entries.kicad_sch",
    "tests/corpus/kicad/sheets_hier.kicad_sch",
    "tests/corpus/eagle/restrict_and_pad_shapes.brd",
    "tests/corpus/eagle/buses.sch",
    "tests/corpus/route/zone_cutouts.board.json",
    "tests/dxf/stadium_holes.dxf",
    "tests/dxf/empty_entities.dxf",
    "tests/dxf/rectangle_repeated_line.dxf",
    "examples/guides/bus.sch",
    "examples/guides/cutout.brd",
];

/// A seed cut at fixed places (a quarter, a half, three quarters, one byte less), twice in a row,
/// and with its first half repeated in the middle. Nothing here is random, so every computer
/// tries the same files.
fn variants(seed: &[u8], cap: usize) -> Vec<Vec<u8>> {
    let len = seed.len();
    let mut out: Vec<Vec<u8>> = Vec::new();
    for cut in [len / 4, len / 2, len * 3 / 4, len.saturating_sub(1)] {
        out.push(seed[..cut].to_vec());
    }
    let mut twice = seed.to_vec();
    twice.extend_from_slice(seed);
    out.push(twice);
    let half = len / 2;
    let mut repeated = seed[..half].to_vec();
    repeated.extend_from_slice(&seed[..half]);
    repeated.extend_from_slice(&seed[half..]);
    out.push(repeated);
    for v in &mut out {
        v.truncate(cap);
    }
    out
}

#[test]
fn the_newer_pieces_of_the_corpus_and_the_guides_are_seeds_of_their_readers() {
    let all = all_seeds();
    for wanted in NEWER_SEEDS {
        assert!(
            all.iter().any(|(name, _)| name == wanted),
            "{wanted} is not among the seeds"
        );
    }
    let list = readers();
    let seeds_for = |reader: &str| -> Vec<String> {
        let reader = list.iter().find(|r| r.name == reader).unwrap();
        seeds_of(reader, &all)
            .into_iter()
            .map(|(name, _)| name.clone())
            .collect()
    };
    assert!(seeds_for("dxf").contains(&"tests/dxf/stadium_holes.dxf".to_owned()));
    assert!(
        seeds_for("kicad board").contains(&"tests/corpus/kicad/zones_keepout.kicad_pcb".to_owned())
    );
    assert!(seeds_for("kicad schematic")
        .contains(&"tests/corpus/kicad/bus_entries.kicad_sch".to_owned()));
    assert!(seeds_for("eagle schematic").contains(&"tests/corpus/eagle/buses.sch".to_owned()));
    assert!(
        seeds_for("board json").contains(&"tests/corpus/route/zone_cutouts.board.json".to_owned())
    );
}

#[test]
fn no_reader_panics_or_takes_too_long_on_a_seed_cut_at_fixed_places_or_repeated() {
    let settings = settings();
    let all = all_seeds();
    let ctx = context();
    let limit = Duration::from_secs(settings.max_seconds_per_input);
    let mut problems: Vec<String> = Vec::new();
    let mut tried = 0usize;
    for reader in readers() {
        for (name, bytes) in seeds_of(&reader, &all) {
            for (n, variant) in variants(bytes, settings.max_input_bytes)
                .into_iter()
                .enumerate()
            {
                tried += 1;
                if let Some(problem) = try_once(&reader, &variant, &ctx, limit) {
                    problems.push(format!(
                        "{} on {name}, variant {n} ({} bytes): {problem}",
                        reader.name,
                        variant.len()
                    ));
                }
            }
        }
    }
    assert!(tried > 1000, "{tried} files tried");
    assert!(
        problems.is_empty(),
        "{} problems, the first ones: {:#?}",
        problems.len(),
        &problems[..problems.len().min(10)]
    );
}

#[test]
fn the_variants_are_the_same_every_time_and_stay_inside_the_cap() {
    let seed = b"(kicad_pcb (version 1) (net 0 \"\") (gr_line (start 1 2) (end 3 4)))";
    let first = variants(seed, 1 << 20);
    assert_eq!(first, variants(seed, 1 << 20));
    assert_eq!(first.len(), 6);
    // a quarter, a half, three quarters, one byte less, twice, the first half repeated
    let len = seed.len();
    assert_eq!(first[0], seed[..len / 4]);
    assert_eq!(first[1], seed[..len / 2]);
    assert_eq!(first[2], seed[..len * 3 / 4]);
    assert_eq!(first[3], seed[..len - 1]);
    assert_eq!(first[4].len(), 2 * len);
    assert_eq!(first[5].len(), len + len / 2);
    for v in variants(seed, 10) {
        assert!(v.len() <= 10);
    }
    // an empty seed gives six empty files and no panic
    assert!(variants(b"", 8).iter().all(Vec::is_empty));
}
