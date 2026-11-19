// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `ipc2581`: a board as an IPC-2581 file (write only).
//!
//! Exit codes: 0 done, 2 the arguments, the input or the output are unusable. Nothing is written
//! unless `--out` or `--d356` names a file. A file that exists is never replaced unless `--overwrite` is
//! given; then the new file is made next to it with a name that was free, and moved over the old
//! one, so a failure leaves the old file as it was. The input board, a link and a folder are
//! never replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_ipc2581::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_ipc2581::{d356, export, Ipc2581, Ipc2581Error, Ipc2581Settings, D356};
use eh3d_phoenix_rules::Lang;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/ipc2581.cfg";

pub(crate) const USAGE: &str = "\
phoenix ipc2581 [--config FILE] [--out FILE] [--d356 FILE] [--overwrite] [--json] [--lang CODE]
                <board>

  ipc2581       a board as an IPC-2581 file (revision C, a subset): the layers, the outline and
                its cutouts, the pads with their padstacks, the vias, the tracks, the outlines of
                the zones, the nets and a basic bill of materials, as an XML document that is the
                same every time; what the file leaves out is listed. A dry run unless --out names
                the file; a file that exists is replaced only with --overwrite. Write only: the
                file is not read back, and a prototype that no viewer has opened
  --config      what goes into the file and the limit (default: config/ipc2581.cfg when it exists)
  --out FILE    write the file there
  --d356 FILE   also write the netlist of the board as an IPC-D-356A test file (one line of 80
                columns for each pad of a part: net, part, pin, hole, side, place and size, in
                micrometres), a prototype whose columns have not been compared with the standard
                or with a tester; what does not fit is listed
  --overwrite   replace the files that --out and --d356 name when they exist
  --json        print the summary as JSON
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    overwrite: bool,
    lang: Lang,
    config: Option<String>,
    out: Option<String>,
    d356: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, Ipc2581Error> {
    let mut options = Options {
        json: false,
        overwrite: false,
        lang: Lang::En,
        config: None,
        out: None,
        d356: None,
        files: Vec::new(),
    };
    let bad = |what: &str| Ipc2581Error::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, Ipc2581Error> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--overwrite" => options.overwrite = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => options.config = Some(value("--config needs a file")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            "--d356" => options.d356 = Some(value("--d356 needs a file")?),
            other if other.starts_with("--") => {
                return Err(Ipc2581Error::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    if options.overwrite && options.out.is_none() && options.d356.is_none() {
        return Err(bad("--overwrite needs --out or --d356"));
    }
    if options.out.is_some() && options.out == options.d356 {
        return Err(bad("--out and --d356 name the same file"));
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<Ipc2581Settings, Ipc2581Error> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(Ipc2581Settings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| Ipc2581Error::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            Ipc2581Settings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &Ipc2581Error, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Creates a file that must not exist and writes the text into it.
fn create_new(path: &str, text: &str) -> Result<(), Ipc2581Error> {
    let failed = |e: std::io::Error| Ipc2581Error::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn same_file(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Writes the file. A new name is created exclusively. A name that exists is refused without
/// `overwrite`; with it, a plain file that is not the input is replaced by renaming a side file
/// that was created exclusively next to it.
fn write_file(path: &str, text: &str, overwrite: bool, input: &str) -> Result<(), Ipc2581Error> {
    let failed = |what: String| Ipc2581Error::new(OUTPUT_FAILED, what);
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => create_new(path, text),
        Err(e) => Err(failed(format!("{path}: {e}"))),
        Ok(meta) => {
            if !overwrite {
                return Err(failed(format!(
                    "{path} exists, give --overwrite to replace it"
                )));
            }
            if !meta.file_type().is_file() {
                return Err(failed(format!("{path} is not a plain file")));
            }
            if same_file(path, input) {
                return Err(failed(format!("{path} is the input board")));
            }
            let side = format!("{path}.new");
            create_new(&side, text)?;
            if let Err(e) = std::fs::rename(&side, path) {
                let _ = std::fs::remove_file(&side);
                return Err(failed(format!("{path}: {e}")));
            }
            Ok(())
        }
    }
}

fn summary_text(file: &Ipc2581, lang: Lang) -> String {
    let s = &file.summary;
    let mut out = format!(
        "IPC-2581 of {}: {} layers ({} copper), {} parts in {} packages, {} padstacks, {} pads, {} vias, {} tracks, {} zones, {} nets, {} bill lines, {} bytes\n",
        s.name,
        s.layers,
        s.copper_layers,
        s.components,
        s.packages,
        s.padstacks,
        s.pads,
        s.vias,
        s.tracks,
        s.zones,
        s.nets,
        s.bom_lines,
        s.bytes
    );
    for finding in &file.findings {
        out.push_str(&finding.line(lang));
        out.push('\n');
    }
    out
}

/// What is said of the IPC-D-356A file: its records and what does not fit.
fn d356_text(file: &D356, name: &str) -> String {
    let mut out = format!("IPC-D-356A of {name}: {} records\n", file.records);
    for note in &file.notes {
        out.push_str(&format!("note: {note}\n"));
    }
    out
}

fn ipc2581(options: &Options, out: &mut dyn Write) -> Result<i32, Ipc2581Error> {
    let [board_path] = options.files.as_slice() else {
        return Err(Ipc2581Error::new(
            OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let board = read_board_file(board_path).map_err(|e| Ipc2581Error::new(INPUT_UNREADABLE, e))?;
    let settings = load_settings(options.config.as_deref())?;
    let file = export(&board, &settings)?;
    let netlist = match &options.d356 {
        Some(_) => Some(d356(&board, &settings)?),
        None => None,
    };
    if let Some(path) = &options.out {
        write_file(path, &file.xml, options.overwrite, board_path)?;
    }
    if let (Some(path), Some(netlist)) = (&options.d356, &netlist) {
        write_file(path, &netlist.text, options.overwrite, board_path)?;
    }
    let mut text = if options.json {
        // without --d356 the summary is written as it always was, in the order of its fields
        let mut json = if let Some(netlist) = &netlist {
            let mut value = serde_json::to_value(&file.summary).unwrap_or_default();
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "d356".to_owned(),
                    serde_json::json!({ "records": netlist.records, "notes": netlist.notes }),
                );
            }
            serde_json::to_string_pretty(&value).unwrap_or_default()
        } else {
            serde_json::to_string_pretty(&file.summary).unwrap_or_default()
        };
        json.push('\n');
        json
    } else {
        let mut text = summary_text(&file, options.lang);
        if let Some(netlist) = &netlist {
            text.push_str(&d356_text(netlist, &file.summary.name));
        }
        text
    };
    if !options.json {
        for path in options.out.iter().chain(options.d356.iter()) {
            text.push_str(&format!("written: {path}\n"));
        }
    }
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

pub(crate) fn ipc2581_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    match ipc2581(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
