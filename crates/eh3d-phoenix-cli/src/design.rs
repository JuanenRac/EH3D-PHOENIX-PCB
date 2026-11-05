// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `erc` and `sync`: the electrical rule check of a schematic, and the transfer of a
//! schematic to a board (or the annotation back) by stable identity.
//!
//! A schematic is an EAGLE `.sch` file or the JSON of the design model (`design/schematic.json`);
//! a board is an EAGLE `.brd` file or the JSON of the model (`design/board.json`).
//!
//! Exit codes of `erc`: 0 no error is proven, 1 at least one error is proven, 2 the arguments or
//! the input are unusable. Exit codes of `sync`: 0 the documents agree or the package was applied,
//! 1 a dry run found changes to make, 2 the arguments or the input are unusable.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eh3d_phoenix_board::{Board, History};
use eh3d_phoenix_eagle_board::{read_board, read_schematic, write_board, write_schematic};
use eh3d_phoenix_erc::{check, ErcConfig};
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::Schematic;
use eh3d_phoenix_sync::{plan_backward, plan_forward, SchematicHistory, SyncSettings};

use crate::check::{id_source, read_text};

pub(crate) const USAGE: &str = "\
phoenix erc [--config FILE] [--json] [--lang CODE] <schematic> [<board>]

  erc           the electrical rule check of a schematic (an EAGLE .sch file or the JSON of the
                design model): pins on no net, nets with one pin, pin types that the matrix does not
                allow together, inputs with no driver, supplies with no source, net names that look
                alike, references used twice, empty values, parts without a footprint and loose
                wires. How serious each rule is and the matrix of pin types come from FILE
                (config/erc.cfg). The board, when given, is only used to know the footprints of a
                .sch file. The report keeps confirmed and inconclusive findings apart

phoenix sync [--config FILE] [--json | --package] [--lang CODE] [--backward]
             [--apply --out FILE] <schematic> <board>

  sync          compare a schematic and a board by the identity of their parts and show what the
                transfer would change on the board (new parts, parts that are gone, references,
                values, footprints, nets). It is a dry run: nothing is written. --backward shows
                the annotation back into the schematic instead. --apply writes the result to
                --out (a .json file of the design model, or .brd / .sch for EAGLE); an input file
                is never overwritten. --package prints the package of changes as JSON. The
                settings come from FILE (config/sync.cfg)

  --json        print the report as JSON. --lang CODE writes the text in en, es, de, fr, it, ja,
                zh or pt. Exit code of erc: 0 no error proven, 1 an error is proven, 2 unusable
                input. Exit code of sync: 0 in agreement or applied, 1 changes to make (dry run),
                2 unusable input
";

struct Options {
    json: bool,
    package: bool,
    backward: bool,
    apply: bool,
    lang: Lang,
    config: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        package: false,
        backward: false,
        apply: false,
        lang: Lang::En,
        config: None,
        out: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--package" => options.package = true,
            "--backward" => options.backward = true,
            "--apply" => options.apply = true,
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            "--config" => {
                let path = iter.next().ok_or("--config needs a file")?;
                options.config = Some((*path).to_owned());
            }
            "--out" => {
                let path = iter.next().ok_or("--out needs a file")?;
                options.out = Some((*path).to_owned());
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn usage(err: &mut dyn Write) -> i32 {
    let _ = err.write_all(USAGE.as_bytes());
    2
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

fn has_extension(path: &str, wanted: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(wanted))
}

fn load_board(path: &str, new_id: &mut dyn FnMut() -> ObjectId) -> Result<Board, String> {
    let text = read_text(path)?;
    if has_extension(path, "json") {
        Board::from_bytes(text.as_bytes()).map_err(|e| format!("cannot read {path}: {e}"))
    } else {
        read_board(&text, new_id)
            .map(|read| read.board)
            .map_err(|d| format!("cannot read {path}: {}", d.message))
    }
}

fn load_schematic(
    path: &str,
    footprints: &BTreeMap<String, ObjectId>,
    new_id: &mut dyn FnMut() -> ObjectId,
) -> Result<Schematic, String> {
    let text = read_text(path)?;
    if has_extension(path, "json") {
        Schematic::from_bytes(text.as_bytes()).map_err(|e| format!("cannot read {path}: {e}"))
    } else {
        read_schematic(&text, footprints, new_id)
            .map(|read| read.schematic)
            .map_err(|d| format!("cannot read {path}: {}", d.message))
    }
}

fn footprint_names(board: Option<&Board>) -> BTreeMap<String, ObjectId> {
    board
        .map(|b| {
            b.footprints
                .iter()
                .map(|(id, f)| (f.name.clone(), *id))
                .collect()
        })
        .unwrap_or_default()
}

/// Every id that the two documents already use, so that new objects never take one of them.
fn used_ids(schematic: &Schematic, board: &Board) -> BTreeSet<ObjectId> {
    let mut used: BTreeSet<ObjectId> = BTreeSet::new();
    used.extend(board.layers.keys());
    used.extend(board.nets.keys());
    used.extend(board.footprints.keys());
    for footprint in board.footprints.values() {
        used.extend(footprint.pads.keys());
    }
    used.extend(board.components.keys());
    used.extend(board.tracks.keys());
    used.extend(board.vias.keys());
    used.extend(board.zones.keys());
    used.extend(board.silkscreen.keys());
    used.extend(schematic.symbols.keys());
    for symbol in schematic.symbols.values() {
        used.extend(symbol.pins.keys());
    }
    used.extend(schematic.parts.keys());
    used.extend(schematic.nets.keys());
    used.extend(schematic.sheets.keys());
    used.extend(schematic.wires.keys());
    used
}

fn write_atomic(path: &str, bytes: &[u8], retries: u32, wait_ms: u64) -> Result<(), String> {
    let target = Path::new(path);
    let mut name = target.as_os_str().to_owned();
    name.push(".tmp");
    let temp = PathBuf::from(name);
    std::fs::write(&temp, bytes).map_err(|e| format!("cannot write {path}: {e}"))?;
    let mut attempt = 0;
    loop {
        match std::fs::rename(&temp, target) {
            Ok(()) => return Ok(()),
            Err(_) if attempt < retries => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(wait_ms));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&temp);
                return Err(format!("cannot write {path}: {e}"));
            }
        }
    }
}

fn same_file(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The command `erc`.
pub(crate) fn erc(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return usage(err);
        }
    };
    let (schematic_path, board_path) = match options.files.as_slice() {
        [s] => (s, None),
        [s, b] => (s, Some(b)),
        _ => return usage(err),
    };
    let config = match &options.config {
        Some(path) => match read_text(path)
            .and_then(|text| ErcConfig::parse(&text).map_err(|e| format!("{path}: {e}")))
        {
            Ok(config) => config,
            Err(message) => return fail(err, &message),
        },
        None => ErcConfig::default(),
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let board = match board_path {
        Some(path) => match load_board(path, &mut new_id) {
            Ok(board) => Some(board),
            Err(message) => return fail(err, &message),
        },
        None => None,
    };
    let schematic = match load_schematic(
        schematic_path,
        &footprint_names(board.as_ref()),
        &mut new_id,
    ) {
        Ok(schematic) => schematic,
        Err(message) => return fail(err, &message),
    };
    let report = check(&schematic, &config);
    let text = if options.json {
        report.to_json()
    } else {
        report.to_text(options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    if report.passes() {
        0
    } else {
        1
    }
}

/// The command `sync`.
pub(crate) fn sync(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return usage(err);
        }
    };
    let [schematic_path, board_path] = options.files.as_slice() else {
        return usage(err);
    };
    if options.apply && options.out.is_none() {
        return fail(
            err,
            "--apply needs --out FILE: the input files are never overwritten",
        );
    }
    if let Some(path) = &options.out {
        if same_file(path, schematic_path) || same_file(path, board_path) {
            return fail(err, "--out is one of the input files; give another file");
        }
    }
    let settings = match &options.config {
        Some(path) => match read_text(path)
            .and_then(|text| SyncSettings::parse(&text).map_err(|e| format!("{path}: {e}")))
        {
            Ok(settings) => settings,
            Err(message) => return fail(err, &message),
        },
        None => SyncSettings::default(),
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let board = match load_board(board_path, &mut new_id) {
        Ok(board) => board,
        Err(message) => return fail(err, &message),
    };
    let schematic =
        match load_schematic(schematic_path, &footprint_names(Some(&board)), &mut new_id) {
            Ok(schematic) => schematic,
            Err(message) => return fail(err, &message),
        };

    let (pending, text, written) = if options.backward {
        let package = plan_backward(&schematic, &board, &settings);
        let text = if options.package {
            package.to_json()
        } else if options.json {
            package.report().to_json()
        } else {
            package.report().to_text(options.lang)
        };
        let mut written: Option<Vec<u8>> = None;
        if options.apply {
            let mut changed = schematic.clone();
            if let Err(error) = package.apply(&mut changed, &mut SchematicHistory::new()) {
                return fail(err, &error.to_string());
            }
            let wrote_eagle = options
                .out
                .as_deref()
                .is_some_and(|p| has_extension(p, "sch"));
            written = Some(if wrote_eagle {
                write_schematic(&board, &changed).text.into_bytes()
            } else {
                match changed.to_bytes() {
                    Ok(bytes) => bytes,
                    Err(e) => return fail(err, &format!("cannot write the schematic: {e:?}")),
                }
            });
        }
        (!package.is_empty(), text, written)
    } else {
        let mut used = used_ids(&schematic, &board);
        let mut fresh = || loop {
            let candidate = new_id();
            if used.insert(candidate) {
                return candidate;
            }
        };
        let package = plan_forward(&schematic, &board, &settings, &mut fresh);
        let text = if options.package {
            package.to_json()
        } else if options.json {
            package.report().to_json()
        } else {
            package.report().to_text(options.lang)
        };
        let mut written: Option<Vec<u8>> = None;
        if options.apply {
            let mut changed = board.clone();
            if let Err(error) = package.apply(&mut changed, &mut History::new()) {
                return fail(err, &error.to_string());
            }
            let wrote_eagle = options
                .out
                .as_deref()
                .is_some_and(|p| has_extension(p, "brd"));
            written = Some(if wrote_eagle {
                write_board(&changed).text.into_bytes()
            } else {
                match changed.to_bytes() {
                    Ok(bytes) => bytes,
                    Err(e) => return fail(err, &format!("cannot write the board: {e:?}")),
                }
            });
        }
        (!package.is_empty(), text, written)
    };

    let _ = out.write_all(text.as_bytes());
    if let (Some(bytes), Some(path)) = (written, &options.out) {
        if let Err(message) = write_atomic(
            path,
            &bytes,
            settings.write_retries,
            settings.write_retry_ms,
        ) {
            return fail(err, &message);
        }
        let _ = writeln!(err, "note: wrote {path}");
        return 0;
    }
    if pending {
        1
    } else {
        0
    }
}
