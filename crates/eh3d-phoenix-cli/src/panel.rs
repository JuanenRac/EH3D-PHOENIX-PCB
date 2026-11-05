// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `panel`: a matrix of boards with a gap, a frame, tabs with mouse bite holes or
//! V-score lines, fiducials and tooling holes, taken from `config/panel.cfg`.
//!
//! It is a dry run unless `--out` names a folder; the files are then written there, each to a file
//! that does not exist yet, and nothing is written when any of them exists. The board that is read
//! is never written.
//!
//! Exit codes: 0 done, 2 the arguments, the settings, the board, the layout or the output are
//! unusable.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_panel::codes::{OPTION_INVALID, OUTPUT_NOT_ALLOWED, SETTINGS_INVALID};
use eh3d_phoenix_panel::{
    panel_files, plan, summary_json, summary_text, BoardShape, PanelError, PanelFile, PanelSettings,
};
use eh3d_phoenix_rules::Lang;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/panel.cfg";

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix panel [--config FILE] [--columns N] [--rows N] [--json] [--lang CODE] [--out FOLDER] <board>

  panel         lay a board out in a matrix of columns and rows with a routed gap and a frame, tabs
                with a row of mouse bite holes or V-score lines, fiducials and tooling holes, all
                from config/panel.cfg; writes the outline, the V-score lines, the fiducials and the
                drills of the panel as Gerber and Excellon files. A dry run unless --out names a
                folder (a file that exists is never replaced); a prototype, not checked by a
                manufacturer
  --config      the settings (default: config/panel.cfg when it exists)
  --columns N   the boards across, instead of the settings
  --rows N      the boards up, instead of the settings
  --out FOLDER  write the files to this folder (it is made when it is not there)
  --json        print the summary as JSON
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

#[derive(Default)]
struct Options {
    json: bool,
    lang: Option<Lang>,
    config: Option<String>,
    columns: Option<u32>,
    rows: Option<u32>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, PanelError> {
    let mut o = Options::default();
    let bad = |what: &str| PanelError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| -> Result<String, PanelError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(&format!("{name} needs a value")))
        };
        match *arg {
            "--json" => o.json = true,
            "--lang" => {
                let tag = value("--lang")?;
                o.lang = Some(Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--config" => o.config = Some(value("--config")?),
            "--out" => o.out = Some(value("--out")?),
            "--columns" => {
                o.columns = Some(
                    value("--columns")?
                        .parse()
                        .map_err(|_| bad("--columns needs a number"))?,
                )
            }
            "--rows" => {
                o.rows = Some(
                    value("--rows")?
                        .parse()
                        .map_err(|_| bad("--rows needs a number"))?,
                )
            }
            other if other.starts_with("--") => {
                return Err(PanelError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn load_settings(options: &Options) -> Result<PanelSettings, PanelError> {
    let chosen = options.config.as_deref().or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    let mut settings = match chosen {
        Some(path) => {
            let bad = |e: std::io::Error| PanelError::new(SETTINGS_INVALID, format!("{path}: {e}"));
            let size = std::fs::metadata(path).map_err(bad)?.len();
            if size > MAX_SETTINGS_BYTES {
                return Err(PanelError::new(
                    SETTINGS_INVALID,
                    format!("{path} is {size} bytes, the limit is {MAX_SETTINGS_BYTES}"),
                ));
            }
            let text = std::fs::read_to_string(path).map_err(bad)?;
            PanelSettings::parse(&text)?
        }
        None => PanelSettings::default(),
    };
    if let Some(columns) = options.columns {
        settings.columns = columns;
    }
    if let Some(rows) = options.rows {
        settings.rows = rows;
    }
    settings.validate()?;
    Ok(settings)
}

/// Writes the files to a folder, each to a file that does not exist yet. Nothing is written when
/// any of them exists.
fn write_files(folder: &str, files: &[PanelFile]) -> Result<(), PanelError> {
    let folder = Path::new(folder);
    for file in files {
        if folder.join(&file.name).exists() {
            return Err(PanelError::new(
                OUTPUT_NOT_ALLOWED,
                format!("{} exists", folder.join(&file.name).display()),
            ));
        }
    }
    std::fs::create_dir_all(folder)
        .map_err(|e| PanelError::new(OUTPUT_NOT_ALLOWED, format!("{}: {e}", folder.display())))?;
    for file in files {
        let path = folder.join(&file.name);
        let mut handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| PanelError::new(OUTPUT_NOT_ALLOWED, format!("{}: {e}", path.display())))?;
        handle
            .write_all(file.text.as_bytes())
            .map_err(|e| PanelError::new(OUTPUT_NOT_ALLOWED, format!("{}: {e}", path.display())))?;
    }
    Ok(())
}

pub(crate) fn panel_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang.unwrap_or(Lang::En);
    let [board_path] = options.files.as_slice() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let settings = match load_settings(&options) {
        Ok(settings) => settings,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let board = match read_board_file(board_path) {
        Ok(board) => board,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return 2;
        }
    };
    let result = BoardShape::from_board(&board).and_then(|(shape, notes)| {
        let mut panel = plan(&shape, &settings)?;
        let mut all = notes;
        all.append(&mut panel.notes);
        panel.notes = all;
        let files = panel_files(&panel, &settings)?;
        Ok((panel, files))
    });
    let (panel, files) = match result {
        Ok(pair) => pair,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    if let Some(folder) = &options.out {
        if let Err(error) = write_files(folder, &files) {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    }
    let text = if options.json {
        serde_json::to_string_pretty(&summary_json(&panel, &settings, &files)).unwrap_or_default()
            + "\n"
    } else {
        let mut text = summary_text(&panel, &settings, &files, lang);
        text.push_str(match &options.out {
            Some(_) => "written\n",
            None => "dry run: nothing written (give --out FOLDER to write the files)\n",
        });
        text
    };
    let _ = out.write_all(text.as_bytes());
    0
}
