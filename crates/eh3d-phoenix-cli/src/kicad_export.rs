// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix kicad-export`: writes a board, a schematic or the libraries of a design as
//! KiCad 8 files and prints what was written and what the model could not say.
//!
//! A dry run unless `--out` is given; it never writes over an input or over a file that exists.
//!
//! Exit codes: 0 done (the report may list changes), 1 the files written do not read back the
//! same, 2 the arguments, the input or the output are unusable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_board::Board;
use eh3d_phoenix_kicad::write_codes::REFUSED;
use eh3d_phoenix_kicad::{
    export_title, write_footprints, write_pcb, write_schematic, write_symbol_library, Export,
    Language, WriteSettings,
};
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_schematic::Schematic;

use crate::check::id_source;
use crate::design::{has_extension, load_schematic, same_file};
use crate::export::read_board_file;

/// The usage text of the command.
pub const KICAD_EXPORT_USAGE: &str = "\
phoenix kicad-export [--settings FILE] [--lang xx] [--json] [--board FILE] [--out PATH]
                     <board|schematic|library> <input>

  kicad-export  write a design as KiCad 8 files: a board (.kicad_pcb) from a board file, a
                schematic (.kicad_sch, one file for each sheet) from a schematic file, or a
                library from a board (one .kicad_mod for each footprint) or from a schematic
                (one .kicad_sym with its symbols). The input is a JSON file of the design model
                or an EAGLE or KiCad file. --board gives the footprints that a schematic refers
                to. A dry run: it prints the files and the report (what was written, changed
                and left out, with codes PHOENIX-KICADX-nnn) and writes nothing unless --out
                names a folder, or a file when there is a single file to write; an input or a
                file that exists is never written over. A prototype: the files were never opened
                in KiCad. The values come from FILE (config/kicad_export.cfg); --lang is one of
                en, es, de, fr, it, ja, zh, pt (placa, esquema and biblioteca are accepted for
                board, schematic and library). Exit code 0: done; 1: a file does not read back
                the same; 2: unusable input or output
";

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Board,
    Schematic,
    Library,
}

impl Target {
    fn from_word(word: &str) -> Option<Target> {
        match word {
            "board" | "pcb" | "placa" => Some(Target::Board),
            "schematic" | "esquema" => Some(Target::Schematic),
            "library" | "biblioteca" => Some(Target::Library),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Target::Board => "board",
            Target::Schematic => "schematic",
            Target::Library => "library",
        }
    }
}

struct Options {
    json: bool,
    language: Language,
    settings: Option<String>,
    out: Option<String>,
    board: Option<String>,
    words: Vec<String>,
}

fn usage(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    let _ = err.write_all(KICAD_EXPORT_USAGE.as_bytes());
    2
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        language: Language::En,
        settings: None,
        out: None,
        board: None,
        words: Vec::new(),
    };
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--json" => options.json = true,
            "--settings" => {
                options.settings = Some(iter.next().ok_or("--settings needs a file")?.to_owned())
            }
            "--out" => {
                options.out = Some(
                    iter.next()
                        .ok_or("--out needs a folder or a file")?
                        .to_owned(),
                )
            }
            "--board" => {
                options.board = Some(iter.next().ok_or("--board needs a file")?.to_owned())
            }
            "--lang" => {
                options.language = iter
                    .next()
                    .and_then(Language::from_tag)
                    .ok_or("--lang needs en, es, de, fr, it, ja, zh or pt")?
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.words.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<WriteSettings, String> {
    let Some(path) = path else {
        return Ok(WriteSettings::default());
    };
    let size = std::fs::metadata(path)
        .map_err(|e| format!("cannot read {path}: {e}"))?
        .len();
    if size > MAX_SETTINGS_BYTES {
        return Err(format!("{path} is larger than {MAX_SETTINGS_BYTES} bytes"));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    WriteSettings::parse(&text).map_err(|e| format!("{path}: {e}"))
}

fn read_schematic_file(
    path: &str,
    board: Option<&Board>,
) -> Result<(Schematic, BTreeMap<ObjectId, String>), String> {
    let mut by_name: BTreeMap<String, ObjectId> = BTreeMap::new();
    let mut by_id: BTreeMap<ObjectId, String> = BTreeMap::new();
    if let Some(board) = board {
        for (id, footprint) in &board.footprints {
            by_name.insert(footprint.name.clone(), *id);
            by_id.insert(*id, footprint.name.clone());
        }
    }
    let mut new_id = id_source()?;
    let schematic = load_schematic(path, &by_name, &mut new_id)?;
    Ok((schematic, by_id))
}

fn make_export(
    target: Target,
    input: &str,
    board_path: Option<&str>,
    settings: &WriteSettings,
) -> Result<Export, String> {
    match target {
        Target::Board => Ok(write_pcb(&read_board_file(input)?, settings)),
        Target::Schematic => {
            let board = match board_path {
                Some(path) => Some(read_board_file(path)?),
                None => None,
            };
            let (schematic, names) = read_schematic_file(input, board.as_ref())?;
            Ok(write_schematic(&schematic, &names, settings))
        }
        Target::Library => {
            let lower = input.to_ascii_lowercase();
            let is_schematic = lower.ends_with(".kicad_sch") || lower.ends_with(".sch");
            let is_board = lower.ends_with(".kicad_pcb") || lower.ends_with(".brd");
            if is_schematic {
                let (schematic, _) = read_schematic_file(input, None)?;
                return Ok(write_symbol_library(&schematic, settings));
            }
            if is_board {
                return Ok(write_footprints(&read_board_file(input)?, settings));
            }
            // a JSON file of the design model: a board, or else a schematic
            match read_board_file(input) {
                Ok(board) => Ok(write_footprints(&board, settings)),
                Err(board_error) => match read_schematic_file(input, None) {
                    Ok((schematic, _)) => Ok(write_symbol_library(&schematic, settings)),
                    Err(_) => Err(board_error),
                },
            }
        }
    }
}

/// Where each file goes. A single file goes to `--out` itself when that names a file of the same
/// kind; otherwise `--out` is a folder. Nothing that exists is chosen.
fn plan_targets(out_path: &str, export: &Export, input: &str) -> Result<Vec<PathBuf>, String> {
    if same_file(out_path, input) {
        return Err(format!("{out_path} is the input"));
    }
    let single_extension = export
        .files
        .first()
        .and_then(|f| Path::new(&f.name).extension())
        .and_then(|e| e.to_str())
        .map(str::to_owned);
    let names_a_file = single_extension
        .as_deref()
        .is_some_and(|ext| has_extension(out_path, ext));
    let targets: Vec<PathBuf> = if names_a_file {
        if export.files.len() != 1 {
            return Err(format!(
                "{} files have to be written, name a folder and not a file",
                export.files.len()
            ));
        }
        vec![PathBuf::from(out_path)]
    } else {
        export
            .files
            .iter()
            .map(|f| Path::new(out_path).join(&f.name))
            .collect()
    };
    for target in &targets {
        if target.exists() {
            return Err(format!("{} already exists", target.display()));
        }
        if same_file(&target.to_string_lossy(), input) {
            return Err(format!("{} is the input", target.display()));
        }
    }
    Ok(targets)
}

fn write_new(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    file.write_all(text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn refuse(err: &mut dyn Write, language: Language, reason: &str) -> i32 {
    let title = export_title(REFUSED, language).unwrap_or("");
    let _ = writeln!(err, "error {REFUSED}: {title}: {reason}");
    2
}

/// The command.
pub(crate) fn kicad_export(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => return usage(err, &message),
    };
    let (target_word, input) = match options.words.as_slice() {
        [target, input] => (target.as_str(), input.as_str()),
        _ => {
            return usage(
                err,
                "a target (board, schematic or library) and one input are needed",
            )
        }
    };
    let Some(target) = Target::from_word(target_word) else {
        return usage(err, &format!("unknown target {target_word}"));
    };
    let settings = match load_settings(options.settings.as_deref()) {
        Ok(settings) => settings,
        Err(message) => return usage(err, &message),
    };
    let export = match make_export(target, input, options.board.as_deref(), &settings) {
        Ok(export) => export,
        Err(message) => return usage(err, &message),
    };

    let mut written: Vec<String> = Vec::new();
    if let Some(out_path) = options.out.as_deref() {
        let targets = match plan_targets(out_path, &export, input) {
            Ok(targets) => targets,
            Err(reason) => return refuse(err, options.language, &reason),
        };
        for (path, file) in targets.iter().zip(export.files.iter()) {
            if let Err(message) = write_new(path, &file.text) {
                return refuse(err, options.language, &message);
            }
            written.push(path.display().to_string());
        }
    }

    if options.json {
        let files: Vec<serde_json::Value> = export
            .files
            .iter()
            .map(|f| serde_json::json!({ "name": f.name, "bytes": f.text.len() }))
            .collect();
        let document = serde_json::json!({
            "format": "KiCad 8 (prototype, never opened in KiCad)",
            "target": target.name(),
            "dry_run": options.out.is_none(),
            "files": files,
            "written": written,
            "report": export.report.to_json(options.language),
        });
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&document).unwrap_or_default()
        );
    } else {
        let mut text = String::new();
        text.push_str("format: KiCad 8 (prototype, never opened in KiCad)\n");
        text.push_str(&format!("target: {}\n", target.name()));
        text.push_str("files:\n");
        for file in &export.files {
            text.push_str(&format!("  {}: {} bytes\n", file.name, file.text.len()));
        }
        text.push_str(&export.report.text(options.language));
        if options.out.is_some() {
            for path in &written {
                text.push_str(&format!("written: {path}\n"));
            }
        } else {
            text.push_str("dry run: nothing was written\n");
        }
        let _ = out.write_all(text.as_bytes());
    }
    if export.report.has_problem() {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{
        Component, Footprint, Layer, LayerKind, Net, Pad, PadNet, PadShape, Side,
    };
    use eh3d_phoenix_model::{Nm, Point, Udeg};
    use eh3d_phoenix_schematic::{NetPin, Part, Pin, SchematicNet, Symbol};

    fn id(n: u16) -> ObjectId {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    fn tiny_board() -> Board {
        let mut b = Board {
            name: "tiny".into(),
            ..Board::default()
        };
        b.layers.insert(
            id(1),
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.copper_stack = vec![id(1)];
        b.nets.insert(id(2), Net { name: "N".into() });
        let mut pads = BTreeMap::new();
        pads.insert(
            id(3),
            Pad {
                number: "1".into(),
                shape: PadShape::Circle {
                    diameter: Nm::new(1_000_000).unwrap(),
                },
                at: Point::from_nm(0, 0).unwrap(),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![id(1)],
            },
        );
        b.footprints.insert(
            id(4),
            Footprint {
                name: "F".into(),
                pads,
            },
        );
        b.components.insert(
            id(5),
            Component {
                refdes: "U1".into(),
                value: "v".into(),
                symbol: None,
                footprint: id(4),
                at: Point::from_nm(1_000_000, 1_000_000).unwrap(),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        b.pad_nets = vec![PadNet {
            component: id(5),
            pad: id(3),
            net: id(2),
        }];
        b
    }

    fn tiny_schematic() -> Schematic {
        let mut s = Schematic {
            name: "tiny".into(),
            ..Schematic::default()
        };
        let mut pins = BTreeMap::new();
        pins.insert(
            id(11),
            Pin {
                number: "1".into(),
                at: None,
            },
        );
        s.symbols.insert(
            id(10),
            Symbol {
                name: "S".into(),
                pins,
            },
        );
        s.parts.insert(
            id(12),
            Part {
                refdes: "U1".into(),
                value: "v".into(),
                symbol: id(10),
                footprint: Some(id(4)),
                placement: None,
            },
        );
        s.nets.insert(
            id(13),
            SchematicNet {
                name: "N".into(),
                pins: vec![NetPin {
                    part: id(12),
                    pin: id(11),
                }],
            },
        );
        s
    }

    struct Files {
        _dir: tempfile::TempDir,
        board: String,
        schematic: String,
        folder: String,
    }

    fn files() -> Files {
        let dir = tempfile::tempdir().unwrap();
        let board = dir.path().join("board.json");
        std::fs::write(&board, tiny_board().to_bytes().unwrap()).unwrap();
        let schematic = dir.path().join("schematic.json");
        std::fs::write(&schematic, tiny_schematic().to_bytes().unwrap()).unwrap();
        let folder = dir.path().join("out");
        Files {
            board: board.to_string_lossy().into_owned(),
            schematic: schematic.to_string_lossy().into_owned(),
            folder: folder.to_string_lossy().into_owned(),
            _dir: dir,
        }
    }

    fn run(args: &[&str]) -> (i32, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = kicad_export(args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn a_dry_run_lists_the_files_and_writes_nothing() {
        let f = files();
        let (code, out, _) = run(&["board", &f.board]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("tiny.kicad_pcb"));
        assert!(out.contains("dry run: nothing was written"));
        assert!(out.contains("PHOENIX-KICADX-001"));
        assert!(!Path::new(&f.folder).exists());
    }

    #[test]
    fn a_board_is_written_to_a_folder_and_never_over_a_file_that_exists() {
        let f = files();
        let (code, out, _) = run(&["board", &f.board, "--out", &f.folder]);
        assert_eq!(code, 0, "{out}");
        let written = Path::new(&f.folder).join("tiny.kicad_pcb");
        let text = std::fs::read_to_string(&written).unwrap();
        assert!(text.starts_with("(kicad_pcb"));
        let (code, _, err) = run(&["board", &f.board, "--out", &f.folder]);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-KICADX-005"), "{err}");
        assert_eq!(std::fs::read_to_string(&written).unwrap(), text);
    }

    #[test]
    fn a_single_file_can_be_named_and_the_input_is_never_the_output() {
        let f = files();
        let target = Path::new(&f.folder).join("named.kicad_pcb");
        let target = target.to_string_lossy().into_owned();
        let (code, out, _) = run(&["placa", &f.board, "--out", &target]);
        assert_eq!(code, 0, "{out}");
        assert!(Path::new(&target).is_file());
        let (code, _, err) = run(&["board", &f.board, "--out", &f.board]);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-KICADX-005"), "{err}");
        assert!(tiny_board().to_bytes().unwrap() == std::fs::read(&f.board).unwrap());
    }

    #[test]
    fn a_schematic_is_written_with_the_footprints_of_a_board() {
        let f = files();
        let (code, out, _) = run(&[
            "esquema",
            &f.schematic,
            "--board",
            &f.board,
            "--out",
            &f.folder,
        ]);
        assert_eq!(code, 0, "{out}");
        let text = std::fs::read_to_string(Path::new(&f.folder).join("tiny.kicad_sch")).unwrap();
        assert!(text.contains("phoenix:F"));
    }

    #[test]
    fn libraries_come_from_a_board_or_from_a_schematic() {
        let f = files();
        let (code, out, _) = run(&["library", &f.board, "--out", &f.folder]);
        assert_eq!(code, 0, "{out}");
        assert!(Path::new(&f.folder).join("F.kicad_mod").is_file());
        let (code, out, _) = run(&["biblioteca", &f.schematic]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("tiny.kicad_sym"));
    }

    #[test]
    fn several_files_need_a_folder() {
        let f = files();
        let mut s = tiny_schematic();
        s.sheets.insert(
            id(20),
            eh3d_phoenix_schematic::Sheet {
                name: "A".into(),
                width: Nm::new(297_000_000).unwrap(),
                height: Nm::new(210_000_000).unwrap(),
            },
        );
        s.sheets.insert(
            id(21),
            eh3d_phoenix_schematic::Sheet {
                name: "B".into(),
                width: Nm::new(297_000_000).unwrap(),
                height: Nm::new(210_000_000).unwrap(),
            },
        );
        std::fs::write(&f.schematic, s.to_bytes().unwrap()).unwrap();
        let target = Path::new(&f.folder).join("one.kicad_sch");
        let (code, _, err) = run(&[
            "schematic",
            &f.schematic,
            "--out",
            &target.to_string_lossy(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-KICADX-005"), "{err}");
        let (code, out, _) = run(&["schematic", &f.schematic, "--out", &f.folder]);
        assert_eq!(code, 0, "{out}");
        assert!(Path::new(&f.folder).join("tiny.kicad_sch").is_file());
        assert!(Path::new(&f.folder).join("tiny_B.kicad_sch").is_file());
    }

    #[test]
    fn json_and_language_options_work_and_bad_arguments_are_refused() {
        let f = files();
        let (code, out, _) = run(&["--json", "--lang", "es", "board", &f.board]);
        assert_eq!(code, 0);
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["dry_run"], true);
        assert_eq!(value["files"][0]["name"], "tiny.kicad_pcb");
        assert_eq!(
            value["report"]["notes"][0]["title"],
            export_title(
                value["report"]["notes"][0]["code"].as_str().unwrap(),
                Language::Es
            )
            .unwrap()
        );
        for args in [
            vec!["board"],
            vec!["nothing", f.board.as_str()],
            vec!["board", f.board.as_str(), "--lang", "xx"],
            vec!["board", f.board.as_str(), "--what"],
            vec!["board", "no-such-file.json"],
            vec![
                "board",
                f.board.as_str(),
                "--settings",
                "no-such-settings.cfg",
            ],
        ] {
            let (code, _, err) = run(&args);
            assert_eq!(code, 2, "{args:?}");
            assert!(err.contains("error"), "{args:?}");
        }
    }

    #[test]
    fn the_settings_file_changes_what_is_written() {
        let f = files();
        let settings = Path::new(&f.folder).with_file_name("kicad_export.cfg");
        std::fs::write(&settings, "lib_nickname = mine\n").unwrap();
        let (code, out, _) = run(&[
            "board",
            &f.board,
            "--settings",
            &settings.to_string_lossy(),
            "--out",
            &f.folder,
        ]);
        assert_eq!(code, 0, "{out}");
        let text = std::fs::read_to_string(Path::new(&f.folder).join("tiny.kicad_pcb")).unwrap();
        assert!(text.contains("mine:F"));
    }
}
