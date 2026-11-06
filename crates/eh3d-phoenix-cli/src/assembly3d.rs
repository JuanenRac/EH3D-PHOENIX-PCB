// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `assembly3d`: the board and its components as a 3D scene made of the models of a
//! table, with the checks of collisions and height, and a simplified OBJ and STEP export.
//!
//! Exit codes: 0 done, 1 a check found an error (two components collide or one is too tall), 2
//! the arguments or the input are unusable. Nothing is written unless `--obj`, `--step` or
//! `--json-file` name a new file; an existing file is never replaced, and none is written when
//! one exists.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_assembly3d::codes::{
    FILE_TOO_LARGE, INPUT_UNREADABLE, LIMIT_EXCEEDED, MODEL_UNREADABLE, OPTION_INVALID,
    OUTPUT_FAILED, STEP_EMPTY, STEP_INVALID, STEP_UNIT,
};
use eh3d_phoenix_assembly3d::{
    build_scene, check_scene, export_note, read_step, scene_json, scene_obj, scene_step,
    scene_text, AssemblyError, AssemblySettings, ModelTable, Severity, StepError, StepSolid,
};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::StackupJob;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/assembly3d.cfg";

pub(crate) const USAGE: &str = "\
phoenix assembly3d [--config FILE] --models FILE [--stackup FILE] [--obj FILE] [--step FILE]
                   [--json-file FILE] [--json] [--lang CODE] <board>

  assembly3d    the board with the thickness of its stack-up and each component as a box, from the
                model that a table gives to each footprint (a STEP file next to the table, or a
                box written in the table); a STEP file that is not a plain box is replaced by the
                box that encloses it with a finding, a footprint without a model is left out with
                a finding and no model is ever made up. It checks the boxes for collisions, the
                height of the components and the components that leave the outline. A prototype
                that has been tried on files made by the tests only. Without --obj, --step and
                --json-file nothing is written; each of them writes a new file (an existing file
                is never replaced and none is written when one exists). --json prints the JSON
                on the screen instead of the text
  --config      the thickness, tolerances and limits (default: config/assembly3d.cfg when it
                exists)
  --models      the table of models: a CSV with the columns footprint, model, dx_mm, dy_mm, dz_mm
                and rot_z_deg
  --stackup     a stack-up job file (the file of `phoenix stackup`) for the thickness of the board
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    models: Option<String>,
    stackup: Option<String>,
    obj: Option<String>,
    step: Option<String>,
    json_file: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, AssemblyError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        models: None,
        stackup: None,
        obj: None,
        step: None,
        json_file: None,
        files: Vec::new(),
    };
    let bad = |what: &str| AssemblyError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, AssemblyError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => options.config = Some(value("--config needs a file")?),
            "--models" => options.models = Some(value("--models needs a file")?),
            "--stackup" => options.stackup = Some(value("--stackup needs a file")?),
            "--obj" => options.obj = Some(value("--obj needs a file")?),
            "--step" => options.step = Some(value("--step needs a file")?),
            "--json-file" => options.json_file = Some(value("--json-file needs a file")?),
            other if other.starts_with("--") => {
                return Err(AssemblyError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

/// Reads a file that is no larger than `limit` bytes.
pub(crate) fn read_limited(
    path: &Path,
    limit: u64,
    code: &'static str,
) -> Result<Vec<u8>, AssemblyError> {
    let unreadable =
        |e: std::io::Error| AssemblyError::new(code, format!("{}: {e}", path.display()));
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    if size > limit {
        return Err(AssemblyError::new(
            FILE_TOO_LARGE,
            format!("{} is {size} bytes, the limit is {limit}", path.display()),
        ));
    }
    std::fs::read(path).map_err(unreadable)
}

pub(crate) fn read_text_limited(path: &str, limit: u64) -> Result<String, AssemblyError> {
    let bytes = read_limited(Path::new(path), limit, INPUT_UNREADABLE)?;
    String::from_utf8(bytes)
        .map_err(|_| AssemblyError::new(INPUT_UNREADABLE, format!("{path} is not text")))
}

pub(crate) fn load_settings(path: Option<&str>) -> Result<AssemblySettings, AssemblyError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(AssemblySettings::default()),
        Some(path) => AssemblySettings::parse(&read_text_limited(path, 1024 * 1024)?),
    }
}

/// Reads the STEP file that the table names, next to the table.
pub(crate) fn read_model(
    folder: &Path,
    name: &str,
    settings: &AssemblySettings,
) -> Result<StepSolid, AssemblyError> {
    let path: PathBuf = folder.join(name);
    let bytes = read_limited(&path, settings.max_file_bytes, MODEL_UNREADABLE)?;
    let text = String::from_utf8_lossy(&bytes);
    read_step(&text, settings.max_step_entities).map_err(|e| match e {
        StepError::NotStep(d) | StepError::Syntax(d) => AssemblyError::new(STEP_INVALID, d),
        StepError::Unit(d) => AssemblyError::new(STEP_UNIT, d),
        StepError::NoGeometry => AssemblyError::new(STEP_EMPTY, ""),
        StepError::TooMany(n) => {
            AssemblyError::new(LIMIT_EXCEEDED, format!("more than {n} entities"))
        }
    })
}

fn fail(err: &mut dyn Write, error: &AssemblyError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Checks that none of the outputs exists or is named twice, then writes each as a new file.
fn write_new_files(files: &[(&str, String)]) -> Result<(), AssemblyError> {
    for (i, (path, _)) in files.iter().enumerate() {
        if Path::new(path).exists() {
            return Err(AssemblyError::new(
                OUTPUT_FAILED,
                format!("{path} exists and is never replaced"),
            ));
        }
        if files[..i].iter().any(|(other, _)| other == path) {
            return Err(AssemblyError::new(
                OUTPUT_FAILED,
                format!("{path} is named for two outputs"),
            ));
        }
    }
    for (path, text) in files {
        let failed = |e: std::io::Error| AssemblyError::new(OUTPUT_FAILED, format!("{path}: {e}"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(failed)?;
        file.write_all(text.as_bytes()).map_err(failed)?;
    }
    Ok(())
}

fn assembly3d(options: &Options, out: &mut dyn Write) -> Result<i32, AssemblyError> {
    let [board_path] = options.files.as_slice() else {
        return Err(AssemblyError::new(
            OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let Some(models_path) = &options.models else {
        return Err(AssemblyError::new(OPTION_INVALID, "--models is needed"));
    };
    let settings = load_settings(options.config.as_deref())?;
    let unreadable = |what: String| AssemblyError::new(INPUT_UNREADABLE, what);
    let board = read_board_file(board_path).map_err(unreadable)?;
    let table = ModelTable::parse(
        &read_text_limited(models_path, settings.max_file_bytes)?,
        settings.max_components,
    )?;
    let thickness = match &options.stackup {
        Some(path) => {
            let text = read_text_limited(path, settings.max_file_bytes)?;
            let job = StackupJob::parse(&text).map_err(|e| unreadable(format!("{path}: {e}")))?;
            job.stackup.total_thickness_nm().ok()
        }
        None => None,
    };
    let folder = Path::new(models_path)
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    let mut resolve = |name: &str| read_model(&folder, name, &settings);
    let scene = build_scene(&board, thickness, &table, &settings, &mut resolve)?;
    let checks = check_scene(&scene, &settings);

    let wants_export = options.obj.is_some() || options.step.is_some();
    let extra = if wants_export {
        vec![export_note(&scene)]
    } else {
        Vec::new()
    };
    let mut files: Vec<(&str, String)> = Vec::new();
    if let Some(path) = &options.obj {
        files.push((path.as_str(), scene_obj(&scene)));
    }
    if let Some(path) = &options.step {
        files.push((path.as_str(), scene_step(&scene)));
    }
    if let Some(path) = &options.json_file {
        files.push((path.as_str(), scene_json(&scene, &checks, &extra)));
    }
    write_new_files(&files)?;

    let text = if options.json {
        scene_json(&scene, &checks, &extra)
    } else {
        let mut text = scene_text(&scene, &checks, &extra, options.lang);
        for (path, _) in &files {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    let has_errors = scene
        .findings
        .iter()
        .chain(checks.findings.iter())
        .any(|f| f.severity == Severity::Error);
    Ok(i32::from(has_errors))
}

pub(crate) fn assembly3d_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    match assembly3d(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "--models", "m.csv", "--obj", "a.obj", "--lang", "de", "b.json",
        ])
        .unwrap();
        assert_eq!(o.models.as_deref(), Some("m.csv"));
        assert_eq!(o.obj.as_deref(), Some("a.obj"));
        assert_eq!(o.lang, Lang::De);
        assert_eq!(o.files, vec!["b.json".to_owned()]);
        for bad in [
            vec!["--models"],
            vec!["--lang", "xx"],
            vec!["--nope"],
            vec!["--step"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
    }

    #[test]
    fn a_model_that_is_missing_is_a_model_error_and_one_that_is_big_is_too_large() {
        let dir = tempfile::tempdir().unwrap();
        let settings = AssemblySettings::default();
        let e = read_model(dir.path(), "none.step", &settings).unwrap_err();
        assert_eq!(e.code, MODEL_UNREADABLE);
        std::fs::write(dir.path().join("big.step"), "x".repeat(100)).unwrap();
        let small = AssemblySettings {
            max_file_bytes: 10,
            ..settings
        };
        let e = read_model(dir.path(), "big.step", &small).unwrap_err();
        assert_eq!(e.code, FILE_TOO_LARGE);
    }
}
