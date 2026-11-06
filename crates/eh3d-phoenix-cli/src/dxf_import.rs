// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `dxf-import`: the outline of a board from the contours of a DXF drawing. It reads
//! the drawing, joins its lines and arcs into closed contours, and applies the operation
//! `outline_from_dxf` of the board editor to the board: all or nothing, with the design rules
//! counted before and after. It is a dry run unless `--out` names a new file; the board and the
//! drawing that are read are never written.
//!
//! Exit codes: 0 done and the rules prove no error on the result, 1 done and the rules prove an
//! error on the result, 2 the arguments, the drawing, the board or the output are unusable or the
//! operation was refused.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditOp};
use eh3d_phoenix_dxf::codes::{DxfError, INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_dxf::{extract_outline, read_dxf, DxfSettings, Finding};
use eh3d_phoenix_rules::{Lang, Severity};
use serde_json::{json, Value};

use crate::design::{has_extension, same_file};
use crate::export::read_board_file;
use crate::route::{load_context, write_new};

/// The settings file that is read when `--config` is not given and the file exists.
pub(crate) const DEFAULT_CONFIG: &str = "config/dxf.cfg";

pub(crate) const USAGE: &str = "\
phoenix dxf-import <drawing.dxf> --board BOARD [--layer LAYER] [--join-tolerance-mm N]
                   [--config FILE] [--rules FILE] [--force] [--out FILE.json] [--json] [--lang CODE]

  dxf-import    the outline of a board from the contours of a DXF drawing (release 12 or later, in
                ASCII): lines, arcs, circles, polylines with bulges, splines (cut into straight
                pieces, with a finding), blocks and units are read, the pieces are joined end to
                end into closed contours, the largest outer contour becomes the outline of the
                board and its cutouts are listed and not kept (the board holds its outline
                only). What is not understood is listed, never dropped without a word. The
                operation is all or nothing and checked by the design rules before and after:
                one that adds a rule error is refused unless --force. A dry run: nothing is
                written unless --out names a new .json file (the board is never written over). A
                prototype that has been tried on drawings made by the tests only
  --board       the board whose outline is replaced (the JSON of the design model, an EAGLE .brd
                file or a KiCad .kicad_pcb file)
  --layer       the layer to take the contours from, without regard to case (default: all)
  --join-tolerance-mm  how far apart two ends may be and still be joined (default: the setting
                join_tolerance_nm of config/dxf.cfg)
  --config      the settings of the reader: layers, tolerances and limits (default:
                config/dxf.cfg when it exists)
  --rules       the JSON rule file for the design rules (an EAGLE board gives its own)
  --force       keep the outline even if it adds a rule error
  --json        print the report as JSON. --lang CODE writes the titles in en, es, de, fr, it,
                ja, zh or pt
";

struct Options {
    file: Option<String>,
    board: Option<String>,
    layer: Option<String>,
    join_tolerance_nm: Option<i64>,
    config: Option<String>,
    rules: Option<String>,
    out: Option<String>,
    force: bool,
    json: bool,
    lang: Lang,
}

fn parse_options(args: &[&str]) -> Result<Options, DxfError> {
    let bad = |what: &str| DxfError::new(OPTION_INVALID, what.to_owned());
    let mut o = Options {
        file: None,
        board: None,
        layer: None,
        join_tolerance_nm: None,
        config: None,
        rules: None,
        out: None,
        force: false,
        json: false,
        lang: Lang::En,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, DxfError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => o.json = true,
            "--force" => o.force = true,
            "--board" => o.board = Some(value("--board needs a file")?),
            "--layer" => o.layer = Some(value("--layer needs a name")?),
            "--config" => o.config = Some(value("--config needs a file")?),
            "--rules" => o.rules = Some(value("--rules needs a file")?),
            "--out" => o.out = Some(value("--out needs a file")?),
            "--join-tolerance-mm" => {
                let text = value("--join-tolerance-mm needs a number")?;
                let mm: f64 = text
                    .parse()
                    .ok()
                    .filter(|v: &f64| (0.0..=1000.0).contains(v))
                    .ok_or_else(|| bad("--join-tolerance-mm is a number from 0 to 1000"))?;
                o.join_tolerance_nm = Some((mm * 1_000_000.0).round() as i64);
            }
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                o.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => {
                if o.file.is_some() {
                    return Err(bad("give only one drawing"));
                }
                o.file = Some(other.to_owned());
            }
        }
    }
    Ok(o)
}

/// The text of a settings file and the settings it makes: `config` or the default file when it
/// exists, else the defaults.
pub(crate) fn load_dxf_settings(
    config: Option<&str>,
) -> Result<(DxfSettings, Option<String>), DxfError> {
    let path = config.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok((DxfSettings::default(), None)),
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| DxfError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            Ok((DxfSettings::parse(&text)?, Some(text)))
        }
    }
}

fn finding_value(f: &Finding) -> Value {
    json!({
        "code": f.code,
        "severity": f.severity.as_str(),
        "subject": f.subject,
        "detail": f.detail,
    })
}

fn corners_value(points: &[eh3d_phoenix_model::Point]) -> Value {
    let xs = points.iter().map(|p| p.x_nm.get());
    let ys = points.iter().map(|p| p.y_nm.get());
    json!({
        "corners": points.len(),
        "width_nm": xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0),
        "height_nm": ys.clone().max().unwrap_or(0) - ys.min().unwrap_or(0),
    })
}

fn fail(err: &mut dyn Write, error: &DxfError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Checks the output before anything is done, so a refusal costs nothing.
fn check_output(path: &str, inputs: &[&str]) -> Result<(), DxfError> {
    let refuse = |why: String| DxfError::new(OUTPUT_FAILED, why);
    if inputs.iter().any(|input| same_file(path, input)) {
        return Err(refuse(format!("{path} is an input file")));
    }
    if Path::new(path).exists() {
        return Err(refuse(format!("{path} exists; give a new file")));
    }
    if !has_extension(path, "json") {
        return Err(refuse(format!("{path}: only .json is written here")));
    }
    Ok(())
}

/// The command `dxf-import`.
pub(crate) fn dxf_import_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(e) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &e, Lang::En);
        }
    };
    let (Some(file), Some(board_path)) = (options.file.as_deref(), options.board.as_deref()) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let lang = options.lang;
    let unreadable = |what: String| DxfError::new(INPUT_UNREADABLE, what);

    let mut inputs = vec![file, board_path];
    inputs.extend(options.config.as_deref());
    inputs.extend(options.rules.as_deref());
    if let Some(path) = options.out.as_deref() {
        if let Err(e) = check_output(path, &inputs) {
            return fail(err, &e, lang);
        }
    }
    let (settings, settings_text) = match load_dxf_settings(options.config.as_deref()) {
        Ok(loaded) => loaded,
        Err(e) => return fail(err, &e, lang),
    };
    let size = match std::fs::metadata(file) {
        Ok(meta) => meta.len(),
        Err(e) => return fail(err, &unreadable(format!("{file}: {e}")), lang),
    };
    if size > settings.max_file_bytes {
        return fail(
            err,
            &DxfError::new(
                eh3d_phoenix_dxf::codes::LIMIT_EXCEEDED,
                format!(
                    "{file} is {size} bytes, the limit is {}",
                    settings.max_file_bytes
                ),
            ),
            lang,
        );
    }
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) => return fail(err, &unreadable(format!("{file}: {e}")), lang),
    };
    let board = match read_board_file(board_path) {
        Ok(board) => board,
        Err(e) => return fail(err, &unreadable(e), lang),
    };
    let ctx = match load_context(options.rules.as_deref(), board_path, &mut *err) {
        Ok(ctx) => ctx,
        Err(e) => return fail(err, &unreadable(e.to_string()), lang),
    };

    // what the drawing holds, for the report
    let join = options
        .join_tolerance_nm
        .unwrap_or(settings.join_tolerance_nm);
    let read = match read_dxf(&text, &settings) {
        Ok(read) => read,
        Err(e) => return fail(err, &e, lang),
    };
    let found = extract_outline(&read.drawing, options.layer.as_deref(), join, &settings);
    let findings: Vec<&Finding> = read
        .report
        .findings
        .iter()
        .chain(found.report.findings.iter())
        .collect();
    let omitted = read.report.omitted + found.report.omitted;

    let before = corners_value(&board.outline);
    let edit_settings = BoardEditSettings {
        live_drc: true,
        reject_new_errors: true,
        ..BoardEditSettings::default()
    };
    let mut edit = match BoardEdit::new(board, ctx, edit_settings) {
        Ok(edit) => edit,
        Err(e) => return fail(err, &unreadable(e.to_string()), lang),
    };
    let op = EditOp::OutlineFromDxf {
        dxf: text,
        layer: options.layer.clone(),
        join_tolerance_nm: Some(join),
        settings: settings_text,
    };
    let op = if options.force {
        EditOp::Forced { with: Box::new(op) }
    } else {
        op
    };
    let outcome = match edit.apply(&op) {
        Ok(outcome) => outcome,
        Err(e) => {
            for f in &findings {
                let _ = writeln!(out, "{}", f.line(lang));
            }
            let _ = writeln!(err, "error: {}: {}", e.code, e.detail);
            return 2;
        }
    };
    let rule_errors = edit
        .findings()
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();

    let mut written: Option<String> = None;
    if let Some(path) = options.out.as_deref() {
        let bytes = match edit.board().to_bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                let _ = writeln!(err, "error: cannot write the board: {e:?}");
                return 2;
            }
        };
        if let Err(e) = write_new(path, &bytes) {
            let _ = writeln!(err, "error: {e}");
            return 2;
        }
        written = Some(path.to_owned());
    }
    let after = corners_value(&edit.board().outline);

    if options.json {
        let value = json!({
            "schema": "phoenix-dxf-import/1",
            "drawing": file,
            "layer": options.layer,
            "unit_mm": read.unit_mm,
            "entities": read.counts,
            "pieces": found.stats.pieces,
            "chains": found.stats.chains,
            "contours": found.stats.contours,
            "open_chains": found.stats.open_chains,
            "duplicates": found.stats.duplicates,
            "arcs_approximated": found.stats.arcs_approximated,
            "cutouts": found.holes.len(),
            "findings": findings.iter().map(|f| finding_value(f)).collect::<Vec<_>>(),
            "findings_omitted": omitted,
            "outline_before": before,
            "outline_after": after,
            "changed": outcome.changed,
            "drc": outcome.drc.to_value(),
            "rule_errors": rule_errors,
            "written": written,
        });
        let _ = writeln!(out, "{value}");
    } else {
        let _ = writeln!(out, "drawing: {file}");
        let _ = writeln!(
            out,
            "layer: {}",
            options.layer.as_deref().unwrap_or("(all layers)")
        );
        let kinds: Vec<String> = read
            .counts
            .iter()
            .map(|(kind, count)| format!("{kind} {count}"))
            .collect();
        let _ = writeln!(out, "entities read: {}", kinds.join(", "));
        let _ = writeln!(
            out,
            "pieces: {}, chains: {}, closed contours: {}, open chains: {}, duplicates dropped: {}",
            found.stats.pieces,
            found.stats.chains,
            found.stats.contours,
            found.stats.open_chains,
            found.stats.duplicates
        );
        for f in &findings {
            let _ = writeln!(out, "{}", f.line(lang));
        }
        if omitted > 0 {
            let _ = writeln!(out, "{omitted} more findings were counted and not listed");
        }
        for note in &outcome.notes {
            if !findings.iter().any(|f| f.line(Lang::En) == *note) {
                let _ = writeln!(out, "{note}");
            }
        }
        let _ = out.write_all(outcome.drc.to_text(lang).as_bytes());
        let _ = writeln!(out, "rule errors on the result: {rule_errors}");
        match &written {
            Some(path) => {
                let _ = writeln!(out, "written: {path}");
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written (--out FILE writes the board)"
                );
            }
        }
    }
    i32::from(rule_errors > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "a.dxf",
            "--board",
            "b.json",
            "--layer",
            "EDGE",
            "--join-tolerance-mm",
            "0.05",
            "--force",
            "--lang",
            "pt",
            "--json",
        ])
        .unwrap();
        assert_eq!(o.file.as_deref(), Some("a.dxf"));
        assert_eq!(o.board.as_deref(), Some("b.json"));
        assert_eq!(o.layer.as_deref(), Some("EDGE"));
        assert_eq!(o.join_tolerance_nm, Some(50_000));
        assert!(o.force && o.json);
        assert_eq!(o.lang, Lang::Pt);
        for bad in [
            vec!["--board"],
            vec!["--lang", "xx"],
            vec!["--nope"],
            vec!["--join-tolerance-mm", "-1"],
            vec!["--join-tolerance-mm", "abc"],
            vec!["--join-tolerance-mm", "5000"],
            vec!["a.dxf", "b.dxf"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
    }

    #[test]
    fn the_output_may_not_be_an_input_or_exist_or_have_another_extension() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("a.json");
        std::fs::write(&existing, "x").unwrap();
        let existing = existing.to_string_lossy().into_owned();
        let new = dir.path().join("b.json").to_string_lossy().into_owned();
        let other = dir.path().join("b.brd").to_string_lossy().into_owned();
        assert!(check_output(&new, &[&existing]).is_ok());
        assert_eq!(
            check_output(&existing, &[&existing]).unwrap_err().code,
            OUTPUT_FAILED
        );
        assert_eq!(
            check_output(&existing, &[]).unwrap_err().code,
            OUTPUT_FAILED
        );
        assert_eq!(check_output(&other, &[]).unwrap_err().code, OUTPUT_FAILED);
    }

    #[test]
    fn a_missing_config_is_the_defaults_and_a_bad_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("dxf.cfg");
        std::fs::write(&bad, "nope = 1\n").unwrap();
        let e = load_dxf_settings(Some(&bad.to_string_lossy())).unwrap_err();
        assert_eq!(e.code, eh3d_phoenix_dxf::codes::SETTINGS_INVALID);
        let missing = dir.path().join("none.cfg");
        let e = load_dxf_settings(Some(&missing.to_string_lossy())).unwrap_err();
        assert_eq!(e.code, INPUT_UNREADABLE);
        let good = dir.path().join("good.cfg");
        std::fs::write(&good, "layer_outline = EDGE\n").unwrap();
        let (settings, text) = load_dxf_settings(Some(&good.to_string_lossy())).unwrap();
        assert_eq!(settings.layer_outline, "EDGE");
        assert_eq!(text.as_deref(), Some("layer_outline = EDGE\n"));
    }
}
