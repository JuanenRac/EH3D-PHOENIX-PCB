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
    check_fab_rules, fab_report_json, fab_report_text, facts_from, items_from_settings,
    panel_layers, plan, plan_items, summary_json, summary_text, BoardShape, FabConfig, FabSeverity,
    Note, PanelError, PanelFile, PanelPlan, PanelSettings,
};
use eh3d_phoenix_rules::Lang;

use crate::export::{load_profile, read_board_file};

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/panel.cfg";

/// The rules of the makers that are read when `--fab-config` is not given and the file exists.
const DEFAULT_FAB_CONFIG: &str = "config/panel_fab.cfg";

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix panel [--config FILE] [--template NAME] [--fab-rules NAME] [--fab-config FILE] [--columns N] [--rows N] [--boards A,B,...] [--profile NAME] [--profiles FILE] [--json] [--lang CODE] [--out FOLDER] [<board>]

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
  --template NAME  apply the lines under `[template NAME]` of the settings file after the
                lines that come before the first template (several templates can live in one file)
  --columns N   the boards across, instead of the settings (a matrix of one board)
  --rows N      the boards up, instead of the settings (a matrix of one board)
  --boards LIST the board files of a panel of different boards, separated by commas; a board that
                appears twice is placed twice. Boards are numbered from 0 in `place` lines
  --profile     the export profile of the layer files (default: generic); --profiles FILE is the
                JSON file that holds the profiles (default: config/export_profiles.json)
  --fab-rules NAME  also judge the panel against the rules of a maker (V-cuts, routed tabs and
                break-away holes, fiducials, tooling edge and tooling holes) from the sets of
                config/panel_fab.cfg (common, jlcpcb, pcbway: public guiding figures, not verified
                with the makers); the report is in --lang. Exit code 1 when a finding is an error
                (the panel is still made). --fab-config FILE reads the sets from another file
  --out FOLDER  write the files to this folder (it is made; it must not exist)
  --json        print the summary as JSON
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Option<Lang>,
    config: Option<String>,
    template: Option<String>,
    columns: Option<u32>,
    rows: Option<u32>,
    boards: Option<Vec<String>>,
    profile: String,
    profiles: Option<String>,
    fab_rules: Option<String>,
    fab_config: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, PanelError> {
    let mut o = Options {
        json: false,
        lang: None,
        config: None,
        template: None,
        columns: None,
        rows: None,
        boards: None,
        profile: "generic".to_owned(),
        profiles: None,
        fab_rules: None,
        fab_config: None,
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
            "--template" => o.template = Some(value("--template")?),
            "--out" => o.out = Some(value("--out")?),
            "--profile" => o.profile = value("--profile")?,
            "--profiles" => o.profiles = Some(value("--profiles")?),
            "--fab-rules" => o.fab_rules = Some(value("--fab-rules")?),
            "--fab-config" => o.fab_config = Some(value("--fab-config")?),
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
    if o.fab_config.is_some() && o.fab_rules.is_none() {
        return Err(bad("--fab-config needs --fab-rules"));
    }
    Ok(o)
}

/// The rules of a maker: the set named by `--fab-rules` from the file of the sets.
fn load_fab_rules(options: &Options, name: &str) -> Result<(FabConfig, usize), PanelError> {
    let chosen = options.fab_config.as_deref().or_else(|| {
        Path::new(DEFAULT_FAB_CONFIG)
            .is_file()
            .then_some(DEFAULT_FAB_CONFIG)
    });
    let config = match chosen {
        Some(path) => {
            let bad = |e: std::io::Error| PanelError::new(SETTINGS_INVALID, format!("{path}: {e}"));
            let size = std::fs::metadata(path).map_err(bad)?.len();
            if size > MAX_SETTINGS_BYTES {
                return Err(PanelError::new(
                    SETTINGS_INVALID,
                    format!("{path} is {size} bytes, the limit is {MAX_SETTINGS_BYTES}"),
                ));
            }
            FabConfig::parse(&std::fs::read_to_string(path).map_err(bad)?)?
        }
        None => {
            return Err(PanelError::new(
                SETTINGS_INVALID,
                "--fab-rules needs config/panel_fab.cfg or --fab-config FILE",
            ))
        }
    };
    match config.rules.iter().position(|r| r.name == name) {
        Some(index) => Ok((config, index)),
        None => Err(PanelError::new(
            OPTION_INVALID,
            format!(
                "no rules called '{name}' (the sets are: {})",
                config.names().join(", ")
            ),
        )),
    }
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
            match &options.template {
                Some(name) => PanelSettings::parse_template(&text, name)?,
                None => PanelSettings::parse(&text)?,
            }
        }
        None if options.template.is_some() => {
            return Err(PanelError::new(
                SETTINGS_INVALID,
                "--template needs a settings file that has templates",
            ))
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
    let fab = match options
        .fab_rules
        .as_deref()
        .map(|n| load_fab_rules(&options, n))
    {
        Some(Ok(found)) => Some(found),
        Some(Err(error)) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
        None => None,
    };
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
    let judged = fab.as_ref().map(|(config, index)| {
        let rules = &config.rules[*index];
        let facts = facts_from(&built.plan, &settings, config.board_thickness_nm);
        let findings = check_fab_rules(&facts, rules, config.smt_assembly);
        (rules, findings)
    });
    let failed = judged
        .as_ref()
        .is_some_and(|(_, found)| found.iter().any(|f| f.severity == FabSeverity::Error));
    let text = if options.json {
        let mut value = summary_json(&built.plan, &settings, &built.files);
        if let (Some((rules, findings)), Some(map)) = (&judged, value.as_object_mut()) {
            map.insert("fab_rules".to_owned(), fab_report_json(rules, findings));
        }
        serde_json::to_string_pretty(&value).unwrap_or_default() + "\n"
    } else {
        let mut text = summary_text(&built.plan, &settings, &built.files, lang);
        for message in &built.messages {
            text.push_str(&format!("export {message}\n"));
        }
        if let Some((rules, findings)) = &judged {
            text.push_str(&fab_report_text(rules, findings, lang));
        }
        text.push_str(match &options.out {
            Some(_) => "written\n",
            None => "dry run: nothing written (give --out FOLDER to write the files)\n",
        });
        text
    };
    let _ = out.write_all(text.as_bytes());
    i32::from(failed)
}
