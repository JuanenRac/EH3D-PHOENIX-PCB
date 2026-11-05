// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `route` and `ratsnest`: the connections still to be routed on a board, and one
//! track between two elements of a net, made by the router and checked by the rules.
//!
//! Exit codes: 0 done (for `route`, the track reaches the second element), 1 the track stopped in
//! front of an obstacle, 2 the arguments, the input or the route are unusable. The input board is
//! never overwritten: the board with the new track is written only to `--out`, and only if that
//! file does not exist yet.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, StepClock};
use eh3d_phoenix_route::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_route::{
    item_label, ratsnest, route, Element, Mode, RouteError, RouteRequest, RouteSettings,
    RuleContext,
};
use eh3d_phoenix_rules::{Lang, RuleFile, RuleValues};

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--settings` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/route.cfg";

pub(crate) const USAGE: &str = "\
phoenix ratsnest [--settings FILE] [--json] [--lang CODE] <board>
phoenix route [--settings FILE] [--rules FILE] [--layer NAME] [--mode around|stop] [--json]
              [--lang CODE] --from ELEMENT --to ELEMENT [--out FILE] <board>

  ratsnest      the connections still to be routed, net by net: a minimum spanning tree over the
                pieces of copper that are not joined yet, with the length of each line
  route         one track (and the vias it needs) between two elements of the same net. An element
                is pad:R1.2 (reference designator and pad number), pad:<component id>/<pad id>,
                via:<id> or track:<id>[:start]. The width and the vias come from the net class and
                the rules; the track keeps the clearances of the rules, with the mode around it goes
                around obstacles and changes layer when needed, with the mode stop it stops in front
                of the first obstacle (exit 1). Without --out nothing is written; with --out the
                board with the track is written to a new file (an existing file is never replaced)
  --rules       the JSON rule file (format phoenix-rules/1 or 2); an EAGLE board gives its own
                design rules when the option is left out
  --settings    the pitch, penalties and limits (default: config/route.cfg when it exists)
  --layer       the layer to start on, by name or id
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    settings: Option<String>,
    rules: Option<String>,
    layer: Option<String>,
    mode: Mode,
    from: Option<String>,
    to: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, RouteError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        settings: None,
        rules: None,
        layer: None,
        mode: Mode::Around,
        from: None,
        to: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| RouteError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = iter
                    .next()
                    .ok_or_else(|| bad("--lang needs a language code"))?;
                options.lang = Lang::from_tag(tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--settings" => {
                options.settings =
                    Some((*iter.next().ok_or_else(|| bad("--settings needs a file"))?).to_owned())
            }
            "--rules" => {
                options.rules =
                    Some((*iter.next().ok_or_else(|| bad("--rules needs a file"))?).to_owned())
            }
            "--layer" => {
                options.layer =
                    Some((*iter.next().ok_or_else(|| bad("--layer needs a name"))?).to_owned())
            }
            "--mode" => {
                options.mode = match iter.next().copied() {
                    Some("around") => Mode::Around,
                    Some("stop") => Mode::Stop,
                    _ => return Err(bad("--mode is around or stop")),
                }
            }
            "--from" => {
                options.from =
                    Some((*iter.next().ok_or_else(|| bad("--from needs an element"))?).to_owned())
            }
            "--to" => {
                options.to =
                    Some((*iter.next().ok_or_else(|| bad("--to needs an element"))?).to_owned())
            }
            "--out" => {
                options.out =
                    Some((*iter.next().ok_or_else(|| bad("--out needs a file"))?).to_owned())
            }
            other if other.starts_with("--") => {
                return Err(RouteError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<RouteSettings, RouteError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_SETTINGS)
            .is_file()
            .then_some(DEFAULT_SETTINGS)
    });
    match path {
        None => Ok(RouteSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| RouteError::new(INPUT_UNREADABLE, m))?;
            RouteSettings::parse(&text)
        }
    }
}

fn load_context(
    options: &Options,
    board_path: &str,
    err: &mut dyn Write,
) -> Result<RuleContext, RouteError> {
    if let Some(path) = &options.rules {
        let text = read_text(path).map_err(|m| RouteError::new(INPUT_UNREADABLE, m))?;
        let file = RuleFile::from_json(&text)
            .map_err(|e| RouteError::new(INPUT_UNREADABLE, e.to_string()))?;
        return Ok(RuleContext {
            rules: file.values,
            classes: file.classes,
        });
    }
    if board_path.to_ascii_lowercase().ends_with(".brd") {
        if let Ok(text) = read_text(board_path) {
            if let Ok((rules, _)) = RuleValues::from_eagle_board(&text) {
                return Ok(RuleContext {
                    rules,
                    classes: Default::default(),
                });
            }
        }
    }
    let _ = writeln!(
        err,
        "note: no rules were given (--rules FILE): only the width, the edge of the board and the other nets' copper are kept apart"
    );
    Ok(RuleContext::default())
}

fn load_board(path: &str) -> Result<Board, RouteError> {
    read_board_file(path).map_err(|m| RouteError::new(INPUT_UNREADABLE, m))
}

fn fail(err: &mut dyn Write, error: &RouteError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error {}", error.message(lang));
    2
}

pub(crate) fn ratsnest_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let [board_path] = options.files.as_slice() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let result = load_settings(options.settings.as_deref()).and_then(|settings| {
        let board = load_board(board_path)?;
        let lines = ratsnest(&board, &settings)?;
        Ok((board, lines))
    });
    match result {
        Ok((board, lines)) => {
            let text = if options.json {
                eh3d_phoenix_route::ratsnest::to_json(&board, &lines)
            } else {
                eh3d_phoenix_route::ratsnest::to_text(&board, &lines)
            };
            let _ = out.write_all(text.as_bytes());
            0
        }
        Err(error) => fail(err, &error, options.lang),
    }
}

/// The layer named on the command line, by name (any case) or by id.
fn find_layer(board: &Board, text: &str) -> Result<Id, RouteError> {
    for (id, layer) in &board.layers {
        if layer.name.eq_ignore_ascii_case(text) {
            return Ok(*id);
        }
    }
    match text.parse::<Id>() {
        Ok(id) if board.layers.contains_key(&id) => Ok(id),
        _ => Err(RouteError::new(
            OPTION_INVALID,
            format!("there is no layer {text:?}"),
        )),
    }
}

fn write_new(path: &str, bytes: &[u8]) -> Result<(), RouteError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| RouteError::new(OUTPUT_FAILED, format!("{path}: {e}")))?;
    file.write_all(bytes)
        .map_err(|e| RouteError::new(OUTPUT_FAILED, format!("{path}: {e}")))
}

fn ids() -> Result<impl FnMut() -> Id, RouteError> {
    let mut generator = IdGenerator::new(
        StepClock::new(1_700_000_000_000, 1),
        DeterministicRandom::new(0x72_6f_75_74_65),
    );
    let mut last = generator
        .generate()
        .map_err(|e| RouteError::new(INPUT_UNREADABLE, e.to_string()))?;
    Ok(move || {
        if let Ok(next) = generator.generate() {
            last = next;
        }
        last
    })
}

pub(crate) fn route_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang;
    let ([board_path], Some(from), Some(to)) = (
        options.files.as_slice(),
        options.from.as_deref(),
        options.to.as_deref(),
    ) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let mut run = || -> Result<(Board, eh3d_phoenix_route::RoutePlan), RouteError> {
        let settings = load_settings(options.settings.as_deref())?;
        let board = load_board(board_path)?;
        let ctx = load_context(&options, board_path, &mut *err)?;
        let layer = match &options.layer {
            Some(text) => Some(find_layer(&board, text)?),
            None => None,
        };
        let request = RouteRequest {
            from: Element::parse(from)?,
            to: Element::parse(to)?,
            layer,
            mode: options.mode,
        };
        let mut new_id = ids()?;
        let plan = route(&board, &ctx, &settings, &request, &mut new_id)?;
        Ok((board, plan))
    };
    let (mut board, plan) = match run() {
        Ok(done) => done,
        Err(error) => return fail(err, &error, lang),
    };
    if options.out.is_none() {
        let _ = writeln!(err, "note: dry run, nothing is written (give --out FILE)");
    }
    if let Some(path) = &options.out {
        let written = board
            .apply(&plan.operation)
            .map_err(|e| RouteError::new(OUTPUT_FAILED, e.to_string()))
            .and_then(|_| {
                board
                    .to_bytes()
                    .map_err(|e| RouteError::new(OUTPUT_FAILED, e.to_string()))
            })
            .and_then(|bytes| write_new(path, &bytes));
        if let Err(error) = written {
            return fail(err, &error, lang);
        }
    }
    let state = if plan.complete { "complete" } else { "stopped" };
    if options.json {
        let notes: Vec<String> = plan.notes.iter().map(|n| format!("\"{n}\"")).collect();
        let _ = writeln!(
            out,
            "{{\"state\":\"{state}\",\"tracks\":{},\"vias\":{},\"length_nm\":{},\"corners\":{},\"notes\":[{}]}}",
            plan.tracks.len(),
            plan.vias.len(),
            plan.length_nm,
            plan.corners,
            notes.join(",")
        );
    } else {
        let _ = writeln!(
            out,
            "route {state}: {} tracks, {} vias, {} nm, {} corners",
            plan.tracks.len(),
            plan.vias.len(),
            plan.length_nm,
            plan.corners
        );
        for note in &plan.notes {
            let _ = writeln!(
                out,
                "  {note}: {}",
                eh3d_phoenix_route::codes::title(note, lang)
            );
        }
        let _ = writeln!(out, "  from {}", item_label_of(&board, from));
        let _ = writeln!(out, "  to {}", item_label_of(&board, to));
    }
    if plan.complete {
        0
    } else {
        1
    }
}

/// The element as the person typed it, with the label of the pad when it is one.
fn item_label_of(board: &Board, text: &str) -> String {
    match Element::parse(text) {
        Ok(Element::Pad(pad)) => {
            item_label(board, eh3d_phoenix_board::connectivity::Item::Pad(pad))
        }
        _ => text.to_owned(),
    }
}
