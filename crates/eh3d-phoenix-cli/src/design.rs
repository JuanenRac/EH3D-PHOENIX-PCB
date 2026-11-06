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
use eh3d_phoenix_erc::{check_hier, ErcConfig};
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::{HierConfig, Schematic};
use eh3d_phoenix_sync::{compare, plan_backward, plan_forward, SchematicHistory, SyncSettings};

use crate::check::{id_source, read_text};

pub(crate) const USAGE: &str = "\
phoenix erc [--config FILE] [--json] [--lang CODE] [--advanced] [--severity RULE=LEVEL]
            [--matrix KIND.KIND=LEVEL] [--power-flag NET] <schematic> [<board>]

  erc           the electrical rule check of a schematic (an EAGLE .sch file or the JSON of the
                design model): pins on no net, nets with one pin, pin types that the matrix does not
                allow together, inputs with no driver, supplies with no source, net names that look
                alike, references used twice, empty values, parts without a footprint and loose
                wires, nets that nothing drives, power inputs of chips without a decoupling
                capacitor near them, net names that may be typing mistakes and labels that touch
                nothing. How serious each rule is and the matrix of pin types come from FILE
                (config/erc.cfg). --advanced turns on the rules that are off by default (nets
                without a driver, decoupling), --severity RULE=LEVEL (off, info, warning or error)
                and --matrix KIND.KIND=LEVEL (ok, info, warning or error) change one line of the
                settings, and --power-flag NET declares a net as supplied from outside; all can be
                repeated and win over the file. The board, when given, is only used to know the
                footprints of a .sch file. The report keeps confirmed and inconclusive findings
                apart

phoenix sync [--config FILE] [--json | --package] [--lang CODE] [--backward]
             [--apply --out FILE] <schematic> <board>
phoenix sync --diff [--json] [--lang CODE] <schematic> <board>

  sync          compare a schematic and a board by the identity of their parts and show what the
                transfer would change on the board (new parts, parts that are gone, references,
                values, footprints, nets). It is a dry run: nothing is written. --backward shows
                the annotation back into the schematic instead. --apply writes the result to
                --out (a .json file of the design model, or .brd / .sch for EAGLE); an input file
                is never overwritten. --package prints the package of changes as JSON. The
                settings come from FILE (config/sync.cfg). --diff compares the two documents in
                both directions at once and lists the differences (parts, values, footprints,
                references, nets, pins on no net, nets with other pins) and the conflicts, in a
                fixed order; exit 1 when they differ

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
    diff: bool,
    lang: Lang,
    config: Option<String>,
    out: Option<String>,
    /// Lines of the settings of the electrical rule check given on the command line.
    erc_lines: Vec<String>,
    /// Nets declared as supplied from outside, for the electrical rule check.
    power_flags: Vec<String>,
    files: Vec<String>,
}

/// The lines of the settings that `--advanced` adds: the rules that are off by default.
const ADVANCED: &str =
    "severity.net_without_driver = warning\nseverity.missing_decoupling = warning";

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        package: false,
        backward: false,
        apply: false,
        diff: false,
        lang: Lang::En,
        config: None,
        out: None,
        erc_lines: Vec::new(),
        power_flags: Vec::new(),
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--package" => options.package = true,
            "--backward" => options.backward = true,
            "--apply" => options.apply = true,
            "--diff" => options.diff = true,
            "--advanced" => options.erc_lines.push(ADVANCED.to_owned()),
            "--severity" => {
                let pair = iter.next().ok_or("--severity needs RULE=LEVEL")?;
                let (rule, level) = pair.split_once('=').ok_or("--severity needs RULE=LEVEL")?;
                options
                    .erc_lines
                    .push(format!("severity.{} = {}", rule.trim(), level.trim()));
            }
            "--matrix" => {
                let pair = iter.next().ok_or("--matrix needs KIND.KIND=LEVEL")?;
                let (kinds, level) = pair
                    .split_once('=')
                    .ok_or("--matrix needs KIND.KIND=LEVEL")?;
                options
                    .erc_lines
                    .push(format!("matrix.{} = {}", kinds.trim(), level.trim()));
            }
            "--power-flag" => {
                let net = iter.next().ok_or("--power-flag needs a net name")?;
                options.power_flags.push((*net).to_owned());
            }
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

pub(crate) fn has_extension(path: &str, wanted: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(wanted))
}

pub(crate) fn load_board(
    path: &str,
    new_id: &mut dyn FnMut() -> ObjectId,
) -> Result<Board, String> {
    if has_extension(path, "kicad_pcb") {
        return crate::kicad_import::read_kicad_board(path, new_id);
    }
    let text = read_text(path)?;
    if has_extension(path, "json") {
        Board::from_bytes(text.as_bytes()).map_err(|e| format!("cannot read {path}: {e}"))
    } else {
        read_board(&text, new_id)
            .map(|read| read.board)
            .map_err(|d| format!("cannot read {path}: {}", d.message))
    }
}

pub(crate) fn load_schematic(
    path: &str,
    footprints: &BTreeMap<String, ObjectId>,
    new_id: &mut dyn FnMut() -> ObjectId,
) -> Result<Schematic, String> {
    if has_extension(path, "kicad_sch") {
        return crate::kicad_import::read_kicad_schematic(path, new_id);
    }
    let text = read_text(path)?;
    if has_extension(path, "json") {
        Schematic::from_bytes(text.as_bytes()).map_err(|e| format!("cannot read {path}: {e}"))
    } else {
        read_schematic(&text, footprints, new_id)
            .map(|read| read.schematic)
            .map_err(|d| format!("cannot read {path}: {}", d.message))
    }
}

pub(crate) fn footprint_names(board: Option<&Board>) -> BTreeMap<String, ObjectId> {
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

pub(crate) fn write_atomic(
    path: &str,
    bytes: &[u8],
    retries: u32,
    wait_ms: u64,
) -> Result<(), String> {
    let target = Path::new(path);
    let mut name = target.as_os_str().to_owned();
    name.push(".tmp");
    let temp = PathBuf::from(name);
    // the side file is made exclusively: a link or a file left at its name is never followed or
    // written over
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| {
                let code = if e.kind() == std::io::ErrorKind::AlreadyExists {
                    crate::sec_codes::SIDE_FILE_EXISTS
                } else {
                    "side file"
                };
                format!("cannot write {path}: {code}: {}: {e}", temp.display())
            })?;
        let written = file.write_all(bytes).and_then(|()| file.sync_all());
        if let Err(e) = written {
            drop(file);
            let _ = std::fs::remove_file(&temp);
            return Err(format!("cannot write {path}: {e}"));
        }
    }
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

pub(crate) fn same_file(a: &str, b: &str) -> bool {
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
    let mut config = config;
    for line in options.erc_lines.iter().flat_map(|text| text.lines()) {
        if let Err(error) = config.merge(line) {
            return fail(err, &format!("{line}: {error}"));
        }
    }
    config
        .power_flags
        .extend(options.power_flags.iter().cloned());
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
    // a schematic with sheet symbols is checked with every instance of every sheet drawn out
    let report = match check_hier(&schematic, &HierConfig::default(), &config) {
        Ok(report) => report,
        Err(e) => return fail(err, &e.to_string()),
    };
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
    if !options.erc_lines.is_empty() || !options.power_flags.is_empty() {
        return fail(
            err,
            "--advanced, --severity, --matrix and --power-flag are options of erc",
        );
    }
    if options.diff && (options.apply || options.package || options.backward) {
        return fail(
            err,
            "--diff only compares: it does not go with --apply, --package or --backward",
        );
    }
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

    if options.diff {
        let report = compare(&schematic, &board);
        let text = if options.json {
            report.to_json()
        } else {
            report.to_text(options.lang)
        };
        let _ = out.write_all(text.as_bytes());
        return i32::from(!report.in_sync());
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_atomic_write_makes_the_file_and_leaves_no_side_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.txt");
        let path = target.to_str().unwrap();
        write_atomic(path, b"one", 0, 0).unwrap();
        write_atomic(path, b"two", 0, 0).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"two");
        assert!(!dir.path().join("out.txt.tmp").exists());
    }

    #[test]
    fn the_atomic_write_never_writes_over_a_side_file_that_is_already_there() {
        let dir = tempfile::tempdir().unwrap();
        let side = dir.path().join("out.txt.tmp");
        std::fs::write(&side, b"not mine").unwrap();
        let target = dir.path().join("out.txt");
        let message = write_atomic(target.to_str().unwrap(), b"data", 0, 0).unwrap_err();
        assert!(message.contains("PHOENIX-SEC-002"), "{message}");
        assert_eq!(std::fs::read(&side).unwrap(), b"not mine");
        assert!(!target.exists());
    }
}
