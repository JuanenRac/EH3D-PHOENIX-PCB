// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `panel`: a matrix of one board, or a list of different boards, with a gap, a frame,
//! tabs with mouse bite holes or V-score lines, fiducials and tooling holes, taken from
//! `config/panel.cfg`.
//!
//! It is a dry run unless `--out` names a folder that does not exist yet; the files are then
//! written there, each to a file that does not exist, and the board files that are read are never
//! written. The copper, the solder mask, the paste, the legend and the drills of every copy are
//! made by the export of the suite with an export profile.
//!
//! Exit codes: 0 done, 2 the arguments, the settings, the boards, the layout or the output are
//! unusable.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_panel::codes::{OPTION_INVALID, OUTPUT_NOT_ALLOWED, SETTINGS_INVALID};
use eh3d_phoenix_panel::{
    items_from_settings, panel_layers, plan, plan_items, summary_json, summary_text, BoardShape,
    Note, PanelError, PanelFile, PanelPlan, PanelSettings,
};
use eh3d_phoenix_rules::Lang;

use crate::export::{load_profile, read_board_file};

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/panel.cfg";

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix panel [--config FILE] [--columns N] [--rows N] [--boards A,B,...] [--profile NAME] [--profiles FILE] [--json] [--lang CODE] [--out FOLDER] [<board>]

  panel         lay a board out in a matrix of columns and rows, or a list of different boards
                packed by rows or placed with the `place` lines of the settings, with a routed gap
                and a frame, tabs with a row of mouse bite holes or V-score lines (no closer to
                copper than the settings say), fiducials (of the frame and of the boards) and
                tooling holes, all from config/panel.cfg; writes the copper, solder mask, paste and
                legend of every copy, the drills, the outline and the V-score lines of the panel,
                made by the export of the suite and read back before anything is written. A dry run
                unless --out names a folder that does not exist yet (a file is never replaced); a
                prototype, not checked by a manufacturer
  --config      the settings (default: config/panel.cfg when it exists)
  --columns N   the boards across, instead of the settings (a matrix of one board)
  --rows N      the boards up, instead of the settings (a matrix of one board)
  --boards LIST the board files of a panel of different boards, separated by commas; a board that
                appears twice is placed twice. Boards are numbered from 0 in `place` lines
  --profile     the export profile of the layer files (default: generic); --profiles FILE is the
                JSON file that holds the profiles (default: config/export_profiles.json)
  --out FOLDER  write the files to this folder (it is made; it must not exist)
  --json        print the summary as JSON
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Option<Lang>,
    config: Option<String>,
    columns: Option<u32>,
    rows: Option<u32>,
    boards: Option<Vec<String>>,
    profile: String,
    profiles: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, PanelError> {
    let mut o = Options {
        json: false,
        lang: None,
        config: None,
        columns: None,
        rows: None,
        boards: None,
        profile: "generic".to_owned(),
        profiles: None,
        out: None,
        files: Vec::new(),
    };
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
            "--profile" => o.profile = value("--profile")?,
            "--profiles" => o.profiles = Some(value("--profiles")?),
            "--boards" => {
                let list = value("--boards")?;
                let names: Vec<String> = list.split(',').map(|n| n.trim().to_owned()).collect();
                if names.iter().any(String::is_empty) {
                    return Err(bad("--boards needs board files separated by commas"));
                }
                o.boards = Some(names);
            }
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

/// Writes the files to a folder that does not exist yet, each to a file that does not exist.
fn write_files(folder: &str, files: &[PanelFile]) -> Result<(), PanelError> {
    let folder = Path::new(folder);
    let refuse = |e: std::io::Error, path: &Path| {
        PanelError::new(OUTPUT_NOT_ALLOWED, format!("{}: {e}", path.display()))
    };
    if folder.exists() {
        return Err(PanelError::new(
            OUTPUT_NOT_ALLOWED,
            format!("{} exists", folder.display()),
        ));
    }
    if let Some(parent) = folder.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| refuse(e, parent))?;
    }
    std::fs::create_dir(folder).map_err(|e| refuse(e, folder))?;
    for file in files {
        let path = folder.join(&file.name);
        let mut handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| refuse(e, &path))?;
        handle
            .write_all(file.text.as_bytes())
            .map_err(|e| refuse(e, &path))?;
    }
    Ok(())
}

/// The plan of the panel, the layer files and what the export had to say.
struct Built {
    plan: PanelPlan,
    files: Vec<PanelFile>,
    messages: Vec<String>,
}

fn build_panel(
    boards: &[Board],
    settings: &PanelSettings,
    options: &Options,
    list: bool,
) -> Result<Built, PanelError> {
    let mut notes: Vec<Note> = Vec::new();
    let mut shapes: Vec<BoardShape> = Vec::new();
    for board in boards {
        let (shape, mut found) = BoardShape::from_board(board)?;
        shapes.push(shape);
        notes.append(&mut found);
    }
    let matrix = !list || (boards.len() == 1 && settings.placements.is_empty());
    let mut panel = if matrix {
        plan(&shapes[0], settings)?
    } else {
        let items = items_from_settings(boards.len(), settings);
        plan_items(&shapes, &items, settings)?
    };
    notes.append(&mut panel.notes);
    panel.notes = notes;
    let profile = load_profile(&options.profile, options.profiles.as_deref())
        .map_err(|e| PanelError::new(OPTION_INVALID, e.to_string()))?;
    let layers = panel_layers(boards, &panel, settings, &profile)?;
    Ok(Built {
        plan: panel,
        files: layers.files,
        messages: layers.messages,
    })
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
    let paths: Vec<String> = match (&options.boards, options.files.as_slice()) {
        (Some(list), []) => list.clone(),
        (None, [one]) => vec![one.clone()],
        _ => {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let settings = match load_settings(&options) {
        Ok(settings) => settings,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    let mut boards: Vec<Board> = Vec::new();
    for path in &paths {
        match read_board_file(path) {
            Ok(board) => boards.push(board),
            Err(message) => {
                let _ = writeln!(err, "error: {message}");
                return 2;
            }
        }
    }
    let built = match build_panel(&boards, &settings, &options, options.boards.is_some()) {
        Ok(built) => built,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    if let Some(folder) = &options.out {
        if let Err(error) = write_files(folder, &built.files) {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    }
    let text = if options.json {
        serde_json::to_string_pretty(&summary_json(&built.plan, &settings, &built.files))
            .unwrap_or_default()
            + "\n"
    } else {
        let mut text = summary_text(&built.plan, &settings, &built.files, lang);
        for message in &built.messages {
            text.push_str(&format!("export {message}\n"));
        }
        text.push_str(match &options.out {
            Some(_) => "written\n",
            None => "dry run: nothing written (give --out FOLDER to write the files)\n",
        });
        text
    };
    let _ = out.write_all(text.as_bytes());
    0
}
