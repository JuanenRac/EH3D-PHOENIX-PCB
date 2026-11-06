// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `enclosure`: the board and its components against the walls, the lid, the floor and
//! the screw posts of an enclosure, and the board as a STEP solid with its holes. The enclosure is
//! a file of `key = value` lines (`box = LxWxH` or `contour = FILE.dxf` with `height_mm`, the posts
//! and the clearances); the components are boxes from a table of models, as in `assembly3d`.
//! Nothing is written unless `--step` names a new file; an existing file is never replaced.
//!
//! Exit codes: 0 done and nothing is through a wall, the lid, the floor or a post, 1 something is
//! (an error finding), 2 the arguments or the input are unusable.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_assembly3d::enclosure_codes::{
    CONTOUR_UNUSABLE, INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED, SOLID_SIMPLIFIED,
};
use eh3d_phoenix_assembly3d::{
    board_holes, board_step, build_scene, check_enclosure, enclosure_json, enclosure_text,
    usable_holes, AssemblyError, AssemblySettings, Cavity, EnclosureSpec, Finding, Inner,
    ModelTable, Severity,
};
use eh3d_phoenix_dxf::{extract_outline, read_dxf};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::StackupJob;

use crate::assembly3d::{load_settings, read_limited, read_model};
use crate::dxf_import::load_dxf_settings;
use crate::export::read_board_file;

pub(crate) const USAGE: &str = "\
phoenix enclosure <board> --enclosure FILE [--models FILE] [--stackup FILE] [--config FILE]
                  [--step FILE] [--json] [--lang CODE]

  enclosure     the board and its components against an enclosure: how far they are from the
                walls, the lid and the floor, whether a component runs into a screw post, whether a
                post has a hole of the board under it, with the codes PHOENIX-ENCL-NNN and the least
                clearances always reported. The enclosure is a file of `key = value` lines: `box =
                LxWxH` (the inside, in millimetres) or `contour = FILE.dxf` with `height_mm` (an
                inner contour of a DXF drawing, relative to the file), `wall_mm`, `standoff_mm`
                (the underside of the board above the floor), `min_clearance_mm`, `x_mm`, `y_mm`
                and one `post = x, y, diameter[, hole[, top]]` line for each screw post (see
                config/enclosure.cfg). The components are the boxes of the table of models, as in
                `phoenix assembly3d`; without --models only the board is checked. With --step the
                board is written as a STEP solid (the prism of its outline with its holes through
                it) to a new file. A prototype: the boxes are not the real parts and no CAD
                program has opened the STEP file. Exit code 1 when something is through a wall,
                the lid, the floor or a post
  --enclosure   the file of the enclosure
  --models      the table of models (a CSV with the columns footprint, model, dx_mm, dy_mm, dz_mm
                and rot_z_deg)
  --stackup     a stack-up job file for the thickness of the board
  --config      the limits and the height limit of the assembly (default: config/assembly3d.cfg
                when it exists)
  --step        write the board as a STEP solid to this new file
  --json        print the report as JSON. --lang CODE writes the titles in en, es, de, fr, it,
                ja, zh or pt
";

struct Options {
    enclosure: Option<String>,
    models: Option<String>,
    stackup: Option<String>,
    config: Option<String>,
    step: Option<String>,
    json: bool,
    lang: Lang,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, AssemblyError> {
    let bad = |what: &str| AssemblyError::new(OPTION_INVALID, what.to_owned());
    let mut o = Options {
        enclosure: None,
        models: None,
        stackup: None,
        config: None,
        step: None,
        json: false,
        lang: Lang::En,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, AssemblyError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => o.json = true,
            "--enclosure" => o.enclosure = Some(value("--enclosure needs a file")?),
            "--models" => o.models = Some(value("--models needs a file")?),
            "--stackup" => o.stackup = Some(value("--stackup needs a file")?),
            "--config" => o.config = Some(value("--config needs a file")?),
            "--step" => o.step = Some(value("--step needs a file")?),
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                o.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn unreadable(what: String) -> AssemblyError {
    AssemblyError::new(INPUT_UNREADABLE, what)
}

fn read_text(path: &Path, limit: u64) -> Result<String, AssemblyError> {
    let bytes = read_limited(path, limit, INPUT_UNREADABLE)?;
    String::from_utf8(bytes).map_err(|_| unreadable(format!("{} is not text", path.display())))
}

/// The polygon of the inner contour that the file of the enclosure names.
fn read_contour(
    folder: &Path,
    name: &str,
    layer: Option<&str>,
    limit: u64,
) -> Result<Vec<(i64, i64)>, AssemblyError> {
    let unusable = |what: String| AssemblyError::new(CONTOUR_UNUSABLE, what);
    let (dxf_settings, _) = load_dxf_settings(None).map_err(|e| unusable(e.message(Lang::En)))?;
    let text = read_text(&folder.join(name), limit)?;
    let read = read_dxf(&text, &dxf_settings).map_err(|e| unusable(e.message(Lang::En)))?;
    let found = extract_outline(
        &read.drawing,
        layer,
        dxf_settings.join_tolerance_nm,
        &dxf_settings,
    );
    let errors: Vec<String> = read
        .report
        .findings
        .iter()
        .chain(found.report.findings.iter())
        .filter(|f| f.severity == eh3d_phoenix_dxf::Severity::Error)
        .map(|f| f.line(Lang::En))
        .collect();
    if let Some(first) = errors.first() {
        return Err(unusable(first.clone()));
    }
    found
        .exterior
        .ok_or_else(|| unusable(format!("{name} has no closed contour")))
}

fn write_new(path: &str, text: &str) -> Result<(), AssemblyError> {
    let failed = |e: std::io::Error| AssemblyError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn run(options: &Options, out: &mut dyn Write) -> Result<i32, AssemblyError> {
    let [board_path] = options.files.as_slice() else {
        return Err(AssemblyError::new(
            OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let Some(enclosure_path) = &options.enclosure else {
        return Err(AssemblyError::new(OPTION_INVALID, "--enclosure is needed"));
    };
    if let Some(step) = &options.step {
        if Path::new(step).exists() {
            return Err(AssemblyError::new(
                OUTPUT_FAILED,
                format!("{step} exists and is never replaced"),
            ));
        }
    }
    let settings: AssemblySettings = load_settings(options.config.as_deref())?;
    let board = read_board_file(board_path).map_err(unreadable)?;
    let spec = EnclosureSpec::parse(&read_text(
        Path::new(enclosure_path),
        settings.max_file_bytes,
    )?)?;
    let folder = Path::new(enclosure_path)
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);

    let thickness = match &options.stackup {
        Some(path) => {
            let text = read_text(Path::new(path), settings.max_file_bytes)?;
            let job = StackupJob::parse(&text).map_err(|e| unreadable(format!("{path}: {e}")))?;
            job.stackup.total_thickness_nm().ok()
        }
        None => None,
    };
    let mut extra: Vec<Finding> = Vec::new();
    let table = match &options.models {
        Some(path) => ModelTable::parse(
            &read_text(Path::new(path), settings.max_file_bytes)?,
            settings.max_components,
        )?,
        None => ModelTable::default(),
    };
    let models_folder = options
        .models
        .as_deref()
        .and_then(|p| Path::new(p).parent())
        .map_or_else(PathBuf::new, Path::to_path_buf);
    let mut resolve = |name: &str| read_model(&models_folder, name, &settings);
    let mut scene = build_scene(&board, thickness, &table, &settings, &mut resolve)?;
    if options.models.is_none() {
        scene
            .findings
            .retain(|f| f.code != eh3d_phoenix_assembly3d::codes::MODEL_MISSING);
        extra.push(Finding::new(
            eh3d_phoenix_assembly3d::codes::MODEL_MISSING,
            Severity::Info,
            Some("--models"),
            "no table of models was given: only the board is checked, not its components",
        ));
    }
    extra.extend(scene.findings.iter().cloned());

    let contour = match &spec.inner {
        Inner::Contour { path, layer } => Some(read_contour(
            &folder,
            path,
            layer.as_deref(),
            settings.max_file_bytes,
        )?),
        Inner::Box { .. } => None,
    };
    let cavity = Cavity::new(&spec, &scene.outline, contour.as_deref())?;
    let holes = board_holes(&board);
    let checks = check_enclosure(&scene, &holes, &cavity, &spec, &settings);

    let mut written: Option<&str> = None;
    if let Some(step) = &options.step {
        let (usable, skipped) = usable_holes(&scene.outline, &holes);
        extra.extend(skipped);
        extra.push(Finding::new(
            SOLID_SIMPLIFIED,
            Severity::Info,
            None,
            format!(
                "the outline as a prism {} nm thick with {} holes through it; copper, pads, components and the real shape of the edges are not in the file",
                scene.thickness_nm,
                usable.len()
            ),
        ));
        write_new(
            step,
            &board_step(&scene.outline, scene.thickness_nm, &usable),
        )?;
        written = Some(step.as_str());
    }

    let text = if options.json {
        enclosure_json(&scene, &cavity, &spec, &checks, &extra)
    } else {
        let mut text = enclosure_text(&scene, &cavity, &spec, &checks, &extra, options.lang);
        if let Some(path) = written {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    let has_errors = checks
        .findings
        .iter()
        .chain(extra.iter())
        .any(|f| f.severity == Severity::Error);
    Ok(i32::from(has_errors))
}

/// The command `enclosure`.
pub(crate) fn enclosure_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Lang::En));
            return 2;
        }
    };
    match run(&options, out) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "--enclosure",
            "case.cfg",
            "--models",
            "m.csv",
            "--step",
            "b.step",
            "--lang",
            "pt",
            "--json",
            "board.json",
        ])
        .unwrap();
        assert_eq!(o.enclosure.as_deref(), Some("case.cfg"));
        assert_eq!(o.models.as_deref(), Some("m.csv"));
        assert_eq!(o.step.as_deref(), Some("b.step"));
        assert!(o.json);
        assert_eq!(o.lang, Lang::Pt);
        assert_eq!(o.files, vec!["board.json".to_owned()]);
        for bad in [
            vec!["--enclosure"],
            vec!["--lang", "xx"],
            vec!["--nope"],
            vec!["--step"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
    }

    #[test]
    fn a_contour_is_read_from_a_drawing_next_to_the_file_and_a_bad_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut d = eh3d_phoenix_dxf::Drawing::new();
        d.polygon(
            "INNER",
            &[
                (0, 0),
                (30_000_000, 0),
                (30_000_000, 20_000_000),
                (0, 20_000_000),
            ],
        );
        std::fs::write(
            dir.path().join("inner.dxf"),
            eh3d_phoenix_dxf::write_dxf(&d),
        )
        .unwrap();
        let polygon = read_contour(dir.path(), "inner.dxf", Some("inner"), 1 << 20).unwrap();
        assert_eq!(polygon.len(), 4);
        // another layer: nothing to take
        let e = read_contour(dir.path(), "inner.dxf", Some("OTHER"), 1 << 20).unwrap_err();
        assert_eq!(e.code, CONTOUR_UNUSABLE);
        assert!(e.detail.contains("PHOENIX-DXF-014"), "{}", e.detail);
        // a file that is not there
        let e = read_contour(dir.path(), "none.dxf", None, 1 << 20).unwrap_err();
        assert_eq!(e.code, INPUT_UNREADABLE);
        // a file that is not a drawing
        std::fs::write(dir.path().join("bad.dxf"), "nothing").unwrap();
        let e = read_contour(dir.path(), "bad.dxf", None, 1 << 20).unwrap_err();
        assert_eq!(e.code, CONTOUR_UNUSABLE);
    }

    #[test]
    fn an_output_that_exists_is_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.step");
        write_new(&path.to_string_lossy(), "x").unwrap();
        let e = write_new(&path.to_string_lossy(), "y").unwrap_err();
        assert_eq!(e.code, OUTPUT_FAILED);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x");
    }
}
