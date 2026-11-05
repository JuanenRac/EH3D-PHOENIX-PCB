// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `zones`: `zones fill` fills the copper zones of a board and `zones check` measures
//! a fill against the board and the rules.
//!
//! Exit codes: 0 done, 1 a rule is proven broken (`check`) or a zone could not be filled
//! (`fill`), 2 the arguments or the input are unusable. The input is never overwritten: the file
//! of filled zones is written only to `--out`, and only if that file does not exist yet.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_rules::{Lang, NetClasses, RuleFile, RuleValues};
use eh3d_phoenix_zones::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_zones::{
    check_fill, fill_board, fill_file_json, fill_json, fill_text, has_errors, CheckReport,
    FillFile, ZoneError, ZoneFile, ZoneRules, ZoneSettings,
};

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/zones.cfg";

pub(crate) const USAGE: &str = "\
phoenix zones fill [--config FILE] [--rules FILE] [--zones FILE] [--out FILE] [--json]
                   [--lang CODE] <board>
phoenix zones check [--config FILE] [--rules FILE] [--zones FILE] [--fill FILE] [--json]
                    [--lang CODE] <board>

  zones fill    fill the copper zones of a board: the copper of other nets, the edge of the board,
                the zones of a higher priority and the pads that are not joined are kept away from
                by the clearance of the rules, the pads of the net get thermal reliefs, and
                islands that are too small, too narrow or joined to nothing are dropped. Without
                --out nothing is written; with --out the filled polygons are written to a new file
                (an existing file is never replaced)
  zones check   measure a fill against the board and the rules: clearances, the outline of the
                zone and the board, the size and the width of the islands. With --fill the fill
                of that file is measured, without it the fill is made and measured
  --config      the radii, widths and limits (default: config/zones.cfg when it exists)
  --rules       the JSON rule file (format phoenix-rules/1 or 2); an EAGLE board gives its own
                design rules when the option is left out
  --zones       a JSON file (format phoenix-zones/1) with the priority and the limits of zones
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    rules: Option<String>,
    zones: Option<String>,
    fill: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, ZoneError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        rules: None,
        zones: None,
        fill: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| ZoneError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, ZoneError> {
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
            "--rules" => options.rules = Some(value("--rules needs a file")?),
            "--zones" => options.zones = Some(value("--zones needs a file")?),
            "--fill" => options.fill = Some(value("--fill needs a file")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            other if other.starts_with("--") => {
                return Err(ZoneError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn unreadable(what: String) -> ZoneError {
    ZoneError::new(INPUT_UNREADABLE, what)
}

fn load_settings(path: Option<&str>) -> Result<ZoneSettings, ZoneError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(ZoneSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            ZoneSettings::parse(&text)
        }
    }
}

fn load_rules(
    path: Option<&str>,
    board: &Board,
    board_path: &str,
    err: &mut dyn Write,
) -> Result<ZoneRules, ZoneError> {
    let (values, classes) = if let Some(path) = path {
        let text = read_text(path).map_err(unreadable)?;
        let file = RuleFile::from_json(&text).map_err(|e| unreadable(e.to_string()))?;
        (file.values, file.classes)
    } else if board_path.to_ascii_lowercase().ends_with(".brd") {
        match read_text(board_path)
            .ok()
            .and_then(|t| RuleValues::from_eagle_board(&t).ok())
        {
            Some((values, _)) => (values, NetClasses::default()),
            None => (RuleValues::default(), NetClasses::default()),
        }
    } else {
        let _ = writeln!(
            err,
            "note: no rules were given (--rules FILE): only the clearance of each zone is kept"
        );
        (RuleValues::default(), NetClasses::default())
    };
    Ok(ZoneRules::from_rules(board, &values, &classes))
}

struct Loaded {
    board: Board,
    rules: ZoneRules,
    settings: ZoneSettings,
    zones: Option<ZoneFile>,
}

fn load(options: &Options, err: &mut dyn Write) -> Result<Loaded, ZoneError> {
    let [board_path] = options.files.as_slice() else {
        return Err(ZoneError::new(
            OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let board = read_board_file(board_path).map_err(unreadable)?;
    let settings = load_settings(options.config.as_deref())?;
    let rules = load_rules(options.rules.as_deref(), &board, board_path, err)?;
    let zones = match &options.zones {
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            Some(ZoneFile::from_json(&text)?)
        }
        None => None,
    };
    Ok(Loaded {
        board,
        rules,
        settings,
        zones,
    })
}

fn fail(err: &mut dyn Write, error: &ZoneError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), ZoneError> {
    let failed = |e: std::io::Error| ZoneError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn fill(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32, ZoneError> {
    if options.fill.is_some() {
        return Err(ZoneError::new(
            OPTION_INVALID,
            "--fill belongs to zones check",
        ));
    }
    let loaded = load(options, err)?;
    let report = fill_board(
        &loaded.board,
        &loaded.rules,
        &loaded.settings,
        loaded.zones.as_ref(),
    )?;
    if let Some(path) = &options.out {
        write_new(path, &fill_file_json(&report))?;
    }
    let text = if options.json {
        fill_json(&report)
    } else {
        let mut text = fill_text(&report, &loaded.board, options.lang);
        if let Some(path) = &options.out {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(has_errors(&report.findings)))
}

fn check(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32, ZoneError> {
    if options.out.is_some() {
        return Err(ZoneError::new(
            OPTION_INVALID,
            "--out belongs to zones fill",
        ));
    }
    let loaded = load(options, err)?;
    let file: FillFile = match &options.fill {
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            serde_json::from_str(&text).map_err(|e| unreadable(format!("{path}: {e}")))?
        }
        None => {
            fill_board(
                &loaded.board,
                &loaded.rules,
                &loaded.settings,
                loaded.zones.as_ref(),
            )?
            .fill
        }
    };
    let findings = check_fill(&loaded.board, &loaded.rules, &loaded.settings, &file)?;
    let report = CheckReport::new(findings);
    let text = if options.json {
        report.to_json()
    } else {
        report.to_text(options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(report.errors > 0))
}

pub(crate) fn zones_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some((&sub, tail)) = args.split_first() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    if sub != "fill" && sub != "check" {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(tail) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &error, Lang::En);
        }
    };
    let result = if sub == "fill" {
        fill(&options, out, err)
    } else {
        check(&options, out, err)
    };
    match result {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
