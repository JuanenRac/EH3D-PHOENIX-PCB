// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `annotate`: the plan of the annotation back into the schematic, from the changes
//! made on the board, in two steps. The first step is a dry run that prints the plan, each change
//! with its state (applicable, conflict, ignored) and the confirmation code of the plan. The second
//! step, `--apply --confirm CODE --out FILE`, applies the applicable changes to a copy and writes
//! it to a NEW file: neither an input file nor a file that already exists is ever written.
//!
//! Exit codes: 0 nothing to apply and nothing that clashes, or the copy was written; 1 a dry run
//! found changes to apply or conflicts; 2 the arguments or the input are unusable.

use std::io::Write;

use eh3d_phoenix_board::History;
use eh3d_phoenix_eagle_board::{write_board, write_schematic};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sync::{plan_annotation, AnnotateSettings, SchematicHistory};

use crate::check::{id_source, read_text};
use crate::design::{footprint_names, has_extension, load_board, load_schematic, same_file};

pub(crate) const USAGE: &str = "\
phoenix annotate [--config FILE] [--json] [--lang CODE] [--renumber]
                 [--apply --confirm CODE --out FILE [--board-out FILE]] <schematic> <board>

  annotate      the annotation back into the schematic, as a plan that is read before anything is
                written: reference designators, values and footprints of the board, the renumbering
                of the references by position (--renumber, or renumber = true in the settings),
                equivalent pins and gates that the board connects the other way round, and the
                parts and components that only one document has. Each change has a state
                (applicable, conflict or ignored) and the plan has a confirmation code. Without
                --apply it is a dry run. --apply needs --confirm CODE (the code of the plan that was
                read) and --out FILE, and writes the schematic with the applicable changes to a new
                file (.json of the design model, or .sch for EAGLE); --board-out FILE also writes
                the board with the renumbered components. An input file, and a file that already
                exists, is never written. The settings come from FILE (config/annotate.cfg).
                Exit code: 0 nothing to apply or the copy was written, 1 a dry run found changes
                or conflicts, 2 unusable input
";

struct Options {
    json: bool,
    renumber: bool,
    apply: bool,
    lang: Lang,
    config: Option<String>,
    confirm: Option<String>,
    out: Option<String>,
    board_out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        renumber: false,
        apply: false,
        lang: Lang::En,
        config: None,
        confirm: None,
        out: None,
        board_out: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--renumber" => options.renumber = true,
            "--apply" => options.apply = true,
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            "--config" => {
                let path = iter.next().ok_or("--config needs a file")?;
                options.config = Some((*path).to_owned());
            }
            "--confirm" => {
                let code = iter.next().ok_or("--confirm needs the code of the plan")?;
                options.confirm = Some((*code).to_owned());
            }
            "--out" => {
                let path = iter.next().ok_or("--out needs a file")?;
                options.out = Some((*path).to_owned());
            }
            "--board-out" => {
                let path = iter.next().ok_or("--board-out needs a file")?;
                options.board_out = Some((*path).to_owned());
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

/// Writes a file that must not exist yet.
fn write_new(path: &str, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("cannot create {path} (it must not exist yet): {e}"))?;
    file.write_all(bytes)
        .map_err(|e| format!("cannot write {path}: {e}"))
}

/// The command `annotate`.
pub(crate) fn annotate(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let [schematic_path, board_path] = options.files.as_slice() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    if options.apply && (options.out.is_none() || options.confirm.is_none()) {
        return fail(
            err,
            "--apply needs --out FILE and --confirm CODE: run without --apply first and read the plan; the code is printed with it",
        );
    }
    if !options.apply && (options.out.is_some() || options.board_out.is_some()) {
        return fail(err, "--out and --board-out need --apply");
    }
    if options.board_out.is_some() && options.out.is_none() {
        return fail(err, "--board-out needs --out");
    }
    for path in [&options.out, &options.board_out].into_iter().flatten() {
        if same_file(path, schematic_path) || same_file(path, board_path) {
            return fail(
                err,
                "an output is one of the input files; give another file",
            );
        }
        if std::path::Path::new(path).exists() {
            return fail(
                err,
                &format!("{path} exists already: the command only writes new files"),
            );
        }
    }
    if let (Some(a), Some(b)) = (&options.out, &options.board_out) {
        if same_file(a, b) {
            return fail(err, "--out and --board-out are the same file");
        }
    }
    let mut settings = match &options.config {
        Some(path) => match read_text(path)
            .and_then(|text| AnnotateSettings::parse(&text).map_err(|e| format!("{path}: {e}")))
        {
            Ok(settings) => settings,
            Err(message) => return fail(err, &message),
        },
        None => AnnotateSettings::default(),
    };
    if options.renumber {
        settings.renumber = true;
    }
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

    let plan = plan_annotation(&schematic, &board, &settings);
    let text = if options.json {
        plan.to_json()
    } else {
        plan.to_text(options.lang)
    };
    let _ = out.write_all(text.as_bytes());

    if !options.apply {
        return i32::from(!plan.is_empty());
    }
    let (Some(confirm), Some(out_path)) = (&options.confirm, &options.out) else {
        return fail(err, "--apply needs --out FILE and --confirm CODE");
    };
    let mut changed = schematic.clone();
    if let Err(error) = plan.apply_to_schematic(&mut changed, &mut SchematicHistory::new(), confirm)
    {
        return fail(err, &format!("{}: {error}", error.code()));
    }
    let mut renamed_board = None;
    if let Some(board_out) = &options.board_out {
        let mut copy = board.clone();
        if let Err(error) = plan.apply_to_board(&mut copy, &mut History::new(), confirm) {
            return fail(err, &format!("{}: {error}", error.code()));
        }
        renamed_board = Some((board_out, copy));
    } else if !plan.board_operations().is_empty() {
        let _ = writeln!(
            err,
            "note: the plan renames components of the board; give --board-out FILE to write them"
        );
    }
    let schematic_bytes = if has_extension(out_path, "sch") {
        write_schematic(&board, &changed).text.into_bytes()
    } else {
        match changed.to_bytes() {
            Ok(bytes) => bytes,
            Err(e) => return fail(err, &format!("cannot write the schematic: {e:?}")),
        }
    };
    let board_bytes = match &renamed_board {
        Some((path, copy)) => Some(if has_extension(path, "brd") {
            write_board(copy).text.into_bytes()
        } else {
            match copy.to_bytes() {
                Ok(bytes) => bytes,
                Err(e) => return fail(err, &format!("cannot write the board: {e:?}")),
            }
        }),
        None => None,
    };
    if let Err(message) = write_new(out_path, &schematic_bytes) {
        return fail(err, &message);
    }
    let _ = writeln!(err, "note: wrote {out_path}");
    if let (Some((path, _)), Some(bytes)) = (&renamed_board, &board_bytes) {
        if let Err(message) = write_new(path, bytes) {
            return fail(err, &message);
        }
        let _ = writeln!(err, "note: wrote {path}");
    }
    0
}
