// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix kicad-import`: reads a KiCad project (`.kicad_pro` with the schematic and
//! the board next to it, a folder with one project, or a single `.kicad_sch` or `.kicad_pcb`
//! file) into the design model. It is a dry run unless `--out` names a new `.json` file, and it
//! never writes over an input or over a file that exists. Nothing is written back to KiCad.
//!
//! This module also holds the readers that let the commands which take a board or a schematic
//! accept a `.kicad_pcb` or `.kicad_sch` file.
//!
//! Exit codes: 0 something was read (the report says what was changed or left out), 1 nothing
//! could be read, 2 the arguments, the files or the output are unusable.

use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_board::Board;
use eh3d_phoenix_kicad::{
    is_safe_relative_name, read_design, read_pcb, read_schematic_with_sheets, sequential_ids,
    title, DesignFiles, DesignImport, KicadError, KicadSettings, Language,
};
use eh3d_phoenix_model::ObjectId;
use eh3d_phoenix_schematic::Schematic;
use serde_json::{json, Value};

use crate::design::{has_extension, same_file};
use crate::kicad::{load_settings, section, to_list};
use crate::route::write_new;

/// The usage text of the command.
pub const KICAD_IMPORT_USAGE: &str = "\
phoenix kicad-import [--settings FILE] [--lang xx] [--json] [--out FILE.json] <project>

  kicad-import  read a KiCad 6 to 9 project into the design model: a project file (.kicad_pro, with
                the .kicad_sch and the .kicad_pcb of the same name next to it), a folder that holds
                one project file, or one .kicad_sch (with the sheets below its folder) or one
                .kicad_pcb. The report lists what was kept, changed and left out of each file, the
                findings with their codes and the fidelity level; a schematic and a board that do
                not list the same parts are reported. A prototype never tried on a real project
  --json        print the report and the models as one JSON document
  --out         write the models: for a single file the JSON of its model (a board or a schematic
                that the other commands read), for a project FILE.board.json and
                FILE.schematic.json beside the name given. Without it nothing is written; a file
                that exists, or an input file, is never written over
  --settings    the limits of the readers (config/kicad.cfg); --lang is one of en, es, de, fr, it,
                ja, zh, pt. Exit code: 0 read; 1 nothing could be read; 2 unusable input
";

/// The files a command line names.
struct Inputs {
    project: Option<PathBuf>,
    schematic: Option<PathBuf>,
    board: Option<PathBuf>,
    absent: Vec<String>,
    from_project: bool,
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn from_project(pro: &Path) -> Inputs {
    let mut inputs = Inputs {
        project: Some(pro.to_path_buf()),
        schematic: None,
        board: None,
        absent: Vec::new(),
        from_project: true,
    };
    for (extension, is_schematic) in [("kicad_sch", true), ("kicad_pcb", false)] {
        let sibling = pro.with_extension(extension);
        if sibling.is_file() {
            if is_schematic {
                inputs.schematic = Some(sibling);
            } else {
                inputs.board = Some(sibling);
            }
        } else {
            inputs.absent.push(name_of(&sibling));
        }
    }
    inputs
}

fn resolve(file: &str) -> Result<Inputs, String> {
    let path = Path::new(file);
    if path.is_dir() {
        let entries = std::fs::read_dir(path).map_err(|e| format!("cannot read {file}: {e}"))?;
        let mut projects: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && has_extension(&p.to_string_lossy(), "kicad_pro"))
            .collect();
        projects.sort();
        return match projects.as_slice() {
            [one] => Ok(from_project(one)),
            [] => Err(format!("{file} holds no .kicad_pro file")),
            many => Err(format!(
                "{file} holds {} project files; name one of them",
                many.len()
            )),
        };
    }
    if has_extension(file, "kicad_pro") {
        return Ok(from_project(path));
    }
    let single = |schematic: bool| Inputs {
        project: None,
        schematic: schematic.then(|| path.to_path_buf()),
        board: (!schematic).then(|| path.to_path_buf()),
        absent: Vec::new(),
        from_project: false,
    };
    if has_extension(file, "kicad_sch") {
        Ok(single(true))
    } else if has_extension(file, "kicad_pcb") {
        Ok(single(false))
    } else {
        Err(format!(
            "{file}: only a .kicad_pro, .kicad_sch or .kicad_pcb file, or a folder, can be imported"
        ))
    }
}

fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let shown = path.display();
    let size = std::fs::metadata(path)
        .map_err(|e| format!("cannot read {shown}: {e}"))?
        .len();
    if size > max {
        return Err(format!("{shown} is {size} bytes, the limit is {max}"));
    }
    std::fs::read(path).map_err(|e| format!("cannot read {shown}: {e}"))
}

/// The sheets of a schematic are read from below the folder of the file, and from nowhere else.
fn sheet_loader(folder: PathBuf, max: u64) -> impl FnMut(&str) -> Option<Vec<u8>> {
    move |name: &str| {
        if !is_safe_relative_name(name) {
            return None;
        }
        let full = folder.join(name.replace('\\', "/"));
        if std::fs::metadata(&full).ok()?.len() > max {
            return None;
        }
        std::fs::read(&full).ok()
    }
}

/// A `.kicad_pcb` file as a board of the model, with the default limits of the readers.
pub(crate) fn read_kicad_board(
    path: &str,
    new_id: &mut dyn FnMut() -> ObjectId,
) -> Result<Board, String> {
    let settings = KicadSettings::default();
    let bytes = read_limited(Path::new(path), settings.max_file_bytes)?;
    read_pcb(&bytes, &settings, new_id)
        .map(|read| read.board)
        .map_err(|d| format!("cannot read {path}: {}", d.message))
}

/// A `.kicad_sch` file, with the sheets below its folder, as a schematic of the model.
pub(crate) fn read_kicad_schematic(
    path: &str,
    new_id: &mut dyn FnMut() -> ObjectId,
) -> Result<Schematic, String> {
    let settings = KicadSettings::default();
    let bytes = read_limited(Path::new(path), settings.max_file_bytes)?;
    let folder = Path::new(path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut loader = sheet_loader(folder, settings.max_file_bytes);
    read_schematic_with_sheets(&bytes, &settings, new_id, &mut loader)
        .map(|read| read.schematic)
        .map_err(|d| format!("cannot read {path}: {}", d.message))
}

struct Options {
    settings: Option<String>,
    lang: Language,
    json: bool,
    out: Option<String>,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        settings: None,
        lang: Language::En,
        json: false,
        out: None,
        file: None,
    };
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--json" => options.json = true,
            "--settings" => {
                options.settings = Some(iter.next().ok_or("--settings needs a file")?.to_owned());
            }
            "--out" => {
                options.out = Some(iter.next().ok_or("--out needs a file")?.to_owned());
            }
            "--lang" => {
                options.lang = iter
                    .next()
                    .and_then(Language::from_tag)
                    .ok_or("--lang needs en, es, de, fr, it, ja, zh or pt")?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => {
                if options.file.is_some() {
                    return Err("only one project can be imported at a time".to_owned());
                }
                options.file = Some(other.to_owned());
            }
        }
    }
    Ok(options)
}

/// What an output file holds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Model {
    Board,
    Schematic,
}

/// The files `--out` asks for, checked before anything is read: the extension, that none is an
/// input and that none exists.
fn output_files(
    out: &str,
    inputs: &Inputs,
    others: &[&str],
) -> Result<Vec<(Model, String)>, KicadError> {
    let refuse = |reason: String| KicadError::OutputRefused { reason };
    if !has_extension(out, "json") {
        return Err(refuse(format!(
            "{out}: only a .json file (the design model) is written"
        )));
    }
    let targets: Vec<(Model, String)> = if inputs.from_project {
        let base = Path::new(out).with_extension("");
        let base = base.to_string_lossy().into_owned();
        vec![
            (Model::Board, format!("{base}.board.json")),
            (Model::Schematic, format!("{base}.schematic.json")),
        ]
    } else if inputs.board.is_some() {
        vec![(Model::Board, out.to_owned())]
    } else {
        vec![(Model::Schematic, out.to_owned())]
    };
    let mut inputs_named: Vec<String> = others.iter().map(|s| (*s).to_owned()).collect();
    for path in [&inputs.project, &inputs.schematic, &inputs.board]
        .into_iter()
        .flatten()
    {
        inputs_named.push(path.to_string_lossy().into_owned());
    }
    for (_, path) in &targets {
        if inputs_named.iter().any(|input| same_file(path, input)) {
            return Err(refuse(format!("{path} is an input file")));
        }
        if Path::new(path).exists() {
            return Err(refuse(format!("{path} exists; give a new file")));
        }
    }
    Ok(targets)
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

fn usage(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    let _ = err.write_all(KICAD_IMPORT_USAGE.as_bytes());
    2
}

fn report_text(design: &DesignImport, file: &str, language: Language) -> String {
    let mut text = String::new();
    text.push_str("format: kicad\n");
    text.push_str(&format!("file: {file}\n"));
    let mut read: Vec<&str> = Vec::new();
    if design.project.is_some() {
        read.push("project");
    }
    if design.schematic.is_some() {
        read.push("schematic");
    }
    if design.board.is_some() {
        read.push("board");
    }
    text.push_str(&format!(
        "read: {}\n",
        if read.is_empty() {
            "nothing".to_owned()
        } else {
            read.join(", ")
        }
    ));
    for (part, _) in &design.unreadable {
        text.push_str(&format!("not readable: {part}\n"));
    }
    text.push_str(&format!(
        "usable: {}\nfidelity: {}\n",
        if design.usable() { "yes" } else { "no" },
        design.fidelity().name()
    ));
    for (part, quality) in design.reports() {
        section(&mut text, &format!("{part} kept"), &to_list(&quality.kept));
        section(
            &mut text,
            &format!("{part} changed"),
            &to_list(&quality.transformed),
        );
        section(
            &mut text,
            &format!("{part} left out"),
            &to_list(&quality.dropped),
        );
    }
    let findings = design.findings();
    text.push_str(&format!("findings: {}\n", findings.len()));
    for (part, d) in findings {
        let code = d.code.as_deref().unwrap_or("-");
        let line = if d.line > 0 {
            format!(" line {}", d.line)
        } else {
            String::new()
        };
        let severity = format!("{:?}", d.severity).to_lowercase();
        let head = title(code, language).unwrap_or("");
        text.push_str(&format!(
            "  {severity} {code} [{part}]{line}: {head}: {}\n",
            d.message
        ));
    }
    text
}

/// The bytes of the model an output file holds, or why there are none.
fn model_bytes(design: &DesignImport, model: Model) -> Result<Vec<u8>, String> {
    match model {
        Model::Board => {
            let read = design.board.as_ref().ok_or("the board was not read")?;
            read.board
                .to_bytes()
                .map_err(|e| format!("cannot write the board: {e:?}"))
        }
        Model::Schematic => {
            let read = design
                .schematic
                .as_ref()
                .ok_or("the schematic was not read")?;
            read.schematic
                .to_bytes()
                .map_err(|e| format!("cannot write the schematic: {e:?}"))
        }
    }
}

/// The command.
pub(crate) fn kicad_import(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => return usage(err, &message),
    };
    let Some(file) = options.file.as_deref() else {
        return usage(err, "a project, a schematic or a board is needed");
    };
    let language = options.lang;
    let settings = match load_settings(options.settings.as_deref(), language) {
        Ok(settings) => settings,
        Err(message) => return usage(err, &message),
    };
    let inputs = match resolve(file) {
        Ok(inputs) => inputs,
        Err(message) => return usage(err, &message),
    };
    let targets = match options.out.as_deref() {
        None => Vec::new(),
        Some(path) => {
            let others: Vec<&str> = options.settings.as_deref().into_iter().collect();
            match output_files(path, &inputs, &others) {
                Ok(targets) => targets,
                Err(e) => return fail(err, &e.message(language)),
            }
        }
    };

    let max = settings.max_file_bytes;
    let mut bytes: [Option<Vec<u8>>; 3] = [None, None, None];
    for (slot, path) in [&inputs.project, &inputs.schematic, &inputs.board]
        .into_iter()
        .enumerate()
    {
        if let Some(path) = path {
            match read_limited(path, max) {
                Ok(read) => bytes[slot] = Some(read),
                Err(message) => return usage(err, &message),
            }
        }
    }
    let files = DesignFiles {
        project: bytes[0].as_deref(),
        schematic: bytes[1].as_deref(),
        board: bytes[2].as_deref(),
        absent: inputs.absent.clone(),
    };
    let mut ids = match sequential_ids() {
        Ok(ids) => ids,
        Err(e) => return fail(err, &e.message(language)),
    };
    let folder = inputs
        .schematic
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut loader = sheet_loader(folder, max);
    let design = read_design(&files, &settings, &mut ids, &mut loader);

    let mut written: Vec<String> = Vec::new();
    let mut not_written: Vec<String> = Vec::new();
    if design.usable() {
        for (model, path) in &targets {
            match model_bytes(&design, *model) {
                Ok(content) => {
                    if let Err(e) = write_new(path, &content) {
                        return fail(err, &e.to_string());
                    }
                    written.push(path.clone());
                }
                Err(why) => not_written.push(format!("{path}: {why}")),
            }
        }
    }

    if options.json {
        let mut value = match design.to_value() {
            Ok(value) => value,
            Err(message) => return fail(err, &message),
        };
        if let Value::Object(map) = &mut value {
            map.insert("written".to_owned(), json!(written));
            map.insert("not_written".to_owned(), json!(not_written));
            map.insert("dry_run".to_owned(), json!(options.out.is_none()));
        }
        let _ = writeln!(out, "{value}");
    } else {
        let _ = out.write_all(report_text(&design, file, language).as_bytes());
        for path in &written {
            let _ = writeln!(out, "written: {path}");
        }
        for note in &not_written {
            let _ = writeln!(out, "not written: {note}");
        }
        if options.out.is_none() {
            let _ = writeln!(
                out,
                "dry run: nothing was written (--out FILE.json writes the models)"
            );
        }
    }
    if design.usable() {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_names_its_schematic_and_its_board_by_the_stem() {
        let dir = tempfile::tempdir().unwrap();
        let pro = dir.path().join("demo.kicad_pro");
        std::fs::write(&pro, "{}").unwrap();
        std::fs::write(dir.path().join("demo.kicad_pcb"), "(kicad_pcb)").unwrap();
        let inputs = from_project(&pro);
        assert!(inputs.board.is_some());
        assert!(inputs.schematic.is_none());
        assert_eq!(inputs.absent, vec!["demo.kicad_sch".to_owned()]);
    }

    #[test]
    fn a_folder_with_two_projects_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.kicad_pro"), "{}").unwrap();
        std::fs::write(dir.path().join("b.kicad_pro"), "{}").unwrap();
        let named = dir.path().to_str().unwrap();
        let error = resolve(named).err().unwrap();
        assert!(error.contains("2 project files"), "{error}");
        assert!(resolve("notes.txt").is_err());
    }
}
