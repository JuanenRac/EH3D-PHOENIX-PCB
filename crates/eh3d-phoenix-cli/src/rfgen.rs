// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `rfgen`: a structure of microstrip drawn from a stack-up and a frequency and put on
//! a board, with the report of what was calculated.
//!
//! Exit codes: 0 done, 1 the structure was made (with the gate of the settings off) and the rule
//! check finds a new error, 2 the arguments, the input or the design are unusable. The input board
//! is never overwritten: the board with the structure is written only to `--out`, and only if that
//! file does not exist yet.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Id;
use eh3d_phoenix_eagle_board::write_board;
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, StepClock};
use eh3d_phoenix_rfgen::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_rfgen::layout::nm_from_m;
use eh3d_phoenix_rfgen::{
    generate, list_json, list_text, new_board, parse_stackup_file, Environment, Kind, Lang,
    NetNames, Params, Placement, Request, RfgenError, RfgenSettings,
};
use eh3d_phoenix_stackup::StackupSettings;

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--settings` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/rfgen.cfg";
/// The settings of the stack-up calculations, read the same way.
const DEFAULT_STACKUP_SETTINGS: &str = "config/stackup.cfg";

pub(crate) const USAGE: &str = "\
phoenix rfgen --list [--json]
phoenix rfgen <type> --stackup FILE --freq-ghz GHZ [--layer NAME] [--board FILE] [--out FILE]
              [--settings FILE] [--stackup-settings FILE] [--net NAME] [--ground-net NAME]
              [--x-mm X] [--y-mm Y] [--rotation DEG] [--name TEXT] [--json] [--lang CODE]
              [--<parameter> VALUE ...]

  rfgen         draw a microstrip structure from the stack-up and the frequency and put it on a board
                as copper polygons, arcs, vias and pads, with a report of the dimensions, the
                formulas used (with their sources and ranges), the warnings and the rule check of
                the result. A prototype: not checked with an electromagnetic simulator or measured
  --list        the types of structure and the parameters of each
  --stackup     a JSON file with `stackup` and `materials` (a job file of `phoenix stackup` works)
  --freq-ghz    the frequency of the design; for a filter, the cutoff or the centre frequency
  --layer       the copper layer of the structure (default: the first copper layer of the stack-up)
  --board       a board to add the structure to (EAGLE .brd or JSON); without it a new board with
                the copper layers of the stack-up is made
  --out         write the board with the structure to a new file: .json (canonical JSON) or .brd
                (EAGLE XML); an existing file is never replaced. Without --out nothing is written
  --net         the name of the signal net (default RF), --ground-net the ground net (default GND)
  --x-mm --y-mm --rotation  where the structure goes: the origin of its frame and a turn that is a
                multiple of 90 degrees
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    list: bool,
    json: bool,
    lang: Lang,
    kind: Option<String>,
    stackup: Option<String>,
    freq_ghz: Option<f64>,
    layer: Option<String>,
    board: Option<String>,
    out: Option<String>,
    settings: Option<String>,
    stackup_settings: Option<String>,
    net: Option<String>,
    ground_net: Option<String>,
    x_mm: f64,
    y_mm: f64,
    rotation: i32,
    name: String,
    params: Params,
}

fn bad(what: &str) -> RfgenError {
    RfgenError::new(OPTION_INVALID, what.to_owned())
}

fn parse_options(args: &[&str]) -> Result<Options, RfgenError> {
    let mut options = Options {
        list: false,
        json: false,
        lang: Lang::En,
        kind: None,
        stackup: None,
        freq_ghz: None,
        layer: None,
        board: None,
        out: None,
        settings: None,
        stackup_settings: None,
        net: None,
        ground_net: None,
        x_mm: 0.0,
        y_mm: 0.0,
        rotation: 0,
        name: "RF structure".to_owned(),
        params: Params::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let Some(flag) = arg.strip_prefix("--") else {
            if options.kind.is_some() {
                return Err(bad(&format!("unexpected argument {arg}")));
            }
            options.kind = Some((*arg).to_owned());
            continue;
        };
        let name = flag.replace('-', "_");
        match name.as_str() {
            "list" => options.list = true,
            "json" => options.json = true,
            _ => {
                let value = *iter
                    .next()
                    .ok_or_else(|| bad(&format!("--{flag} needs a value")))?;
                match name.as_str() {
                    "stackup" => options.stackup = Some(value.to_owned()),
                    "freq_ghz" => {
                        options.freq_ghz = Some(
                            value
                                .parse::<f64>()
                                .map_err(|_| bad("--freq-ghz needs a number"))?,
                        )
                    }
                    "layer" => options.layer = Some(value.to_owned()),
                    "board" => options.board = Some(value.to_owned()),
                    "out" => options.out = Some(value.to_owned()),
                    "settings" => options.settings = Some(value.to_owned()),
                    "stackup_settings" => options.stackup_settings = Some(value.to_owned()),
                    "net" => options.net = Some(value.to_owned()),
                    "ground_net" => options.ground_net = Some(value.to_owned()),
                    "x_mm" => {
                        options.x_mm = value
                            .parse::<f64>()
                            .map_err(|_| bad("--x-mm needs a number"))?
                    }
                    "y_mm" => {
                        options.y_mm = value
                            .parse::<f64>()
                            .map_err(|_| bad("--y-mm needs a number"))?
                    }
                    "rotation" => {
                        options.rotation = value
                            .parse::<i32>()
                            .map_err(|_| bad("--rotation needs a whole number of degrees"))?
                    }
                    "name" => options.name = value.to_owned(),
                    "lang" => {
                        options.lang =
                            Lang::from_tag(value).ok_or_else(|| bad("unknown language code"))?
                    }
                    _ => options.params.set(&name, value),
                }
            }
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<RfgenSettings, RfgenError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_SETTINGS)
            .is_file()
            .then_some(DEFAULT_SETTINGS)
    });
    match path {
        None => Ok(RfgenSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| RfgenError::new(INPUT_UNREADABLE, m))?;
            RfgenSettings::parse(&text)
        }
    }
}

fn load_stackup_settings(path: Option<&str>) -> Result<StackupSettings, RfgenError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_STACKUP_SETTINGS)
            .is_file()
            .then_some(DEFAULT_STACKUP_SETTINGS)
    });
    match path {
        None => Ok(StackupSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| RfgenError::new(INPUT_UNREADABLE, m))?;
            StackupSettings::parse(&text)
                .map_err(|e| RfgenError::new(INPUT_UNREADABLE, e.to_string()))
        }
    }
}

fn ids() -> Result<impl FnMut() -> Id, RfgenError> {
    let mut generator = IdGenerator::new(
        StepClock::new(1_700_000_000_000, 1),
        DeterministicRandom::new(0x72_66_67_65_6e),
    );
    let mut last = generator
        .generate()
        .map_err(|e| RfgenError::new(INPUT_UNREADABLE, e.to_string()))?;
    Ok(move || {
        if let Ok(next) = generator.generate() {
            last = next;
        }
        last
    })
}

fn write_new(path: &str, bytes: &[u8]) -> Result<(), RfgenError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| RfgenError::new(OUTPUT_FAILED, format!("{path}: {e}")))?;
    file.write_all(bytes)
        .map_err(|e| RfgenError::new(OUTPUT_FAILED, format!("{path}: {e}")))
}

/// The file type of an output path, checked before anything is calculated.
fn output_kind(path: &str) -> Result<bool, RfgenError> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".json") {
        Ok(true)
    } else if lower.ends_with(".brd") {
        Ok(false)
    } else {
        Err(bad("--out must end in .json or .brd"))
    }
}

pub(crate) fn rfgen_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    if options.list {
        let text = if options.json {
            format!("{}\n", list_json())
        } else {
            list_text()
        };
        let _ = out.write_all(text.as_bytes());
        return 0;
    }
    let lang = options.lang;
    let (Some(kind_name), Some(stackup_path), Some(freq_ghz)) = (
        options.kind.as_deref(),
        options.stackup.as_deref(),
        options.freq_ghz,
    ) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let run = || -> Result<(eh3d_phoenix_rfgen::Generated, Option<bool>), RfgenError> {
        let kind = Kind::from_name(kind_name)?;
        let as_json = options.out.as_deref().map(output_kind).transpose()?;
        let settings = load_settings(options.settings.as_deref())?;
        let stack_settings = load_stackup_settings(options.stackup_settings.as_deref())?;
        let text = read_text(stackup_path).map_err(|m| RfgenError::new(INPUT_UNREADABLE, m))?;
        let (stackup, materials) = parse_stackup_file(&text)?;
        let env = Environment::new(
            &stackup,
            &materials,
            options.layer.as_deref(),
            freq_ghz,
            settings,
            stack_settings,
        )?;
        let mut next = ids()?;
        let board = match options.board.as_deref() {
            Some(path) => {
                read_board_file(path).map_err(|m| RfgenError::new(INPUT_UNREADABLE, m))?
            }
            None => new_board(&env, &options.name, &mut next)?,
        };
        let defaults = NetNames::default();
        let request = Request {
            kind,
            params: options.params.clone(),
            nets: NetNames {
                signal: options.net.clone().unwrap_or(defaults.signal),
                ground: options.ground_net.clone().unwrap_or(defaults.ground),
            },
            placement: Placement::new(
                nm_from_m(options.x_mm * 1.0e-3)?,
                nm_from_m(options.y_mm * 1.0e-3)?,
                options.rotation,
            )?,
        };
        let generated = generate(&env, &board, &request, &mut next)?;
        Ok((generated, as_json))
    };
    let (generated, as_json) = match run() {
        Ok(done) => done,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            return 2;
        }
    };
    match (&options.out, as_json) {
        (Some(path), Some(json)) => {
            let bytes = if json {
                generated
                    .board
                    .to_bytes()
                    .map_err(|e| RfgenError::new(OUTPUT_FAILED, e.to_string()))
            } else {
                let written = write_board(&generated.board);
                if !written.diagnostics.is_empty() {
                    let _ = writeln!(
                        err,
                        "note: {} items were simplified or left out in the EAGLE file",
                        written.diagnostics.len()
                    );
                }
                Ok(written.text.into_bytes())
            };
            if let Err(error) = bytes.and_then(|b| write_new(path, &b)) {
                let _ = writeln!(err, "error {}", error.message(lang));
                return 2;
            }
        }
        _ => {
            let _ = writeln!(err, "note: dry run, nothing is written (give --out FILE)");
        }
    }
    let text = if options.json {
        format!("{}\n", generated.report.to_json())
    } else {
        generated.report.to_text(lang)
    };
    let _ = out.write_all(text.as_bytes());
    match &generated.report.rules {
        Some(rules) if !rules.passes => 1,
        _ => 0,
    }
}
