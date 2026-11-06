// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `autoroute`: routing by an external autorouter through files.
//!
//! * `export-dsn` writes the board as a Specctra DSN file;
//! * `import-ses` reads a Specctra session as a plan that is checked against the rules, and
//!   writes the board with the routes to a new file;
//! * `run` starts the autorouter named in `config/autorouter.cfg` on the DSN file and imports the
//!   session it writes. It is a dry run unless `--execute` is given, and it starts nothing at all
//!   while `enabled = false` in the settings of the autorouter.
//!
//! Exit codes: 0 done, 2 the arguments, the input, the session or the rules refuse it. The input
//! board is never overwritten: every output is written only to a file that does not exist yet.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_route::codes::{INPUT_UNREADABLE, OPTION_INVALID};
use eh3d_phoenix_route::{
    export_dsn, import_ses, launch_plan, run_name, AutorouterConfig, Dsn, RouteError,
    RouteSettings, RuleContext, SesPlan,
};
use eh3d_phoenix_rules::Lang;

use crate::check::read_text;
use crate::route::{ids, load_board, load_context, load_settings, write_new};

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/autorouter.cfg";

pub(crate) const USAGE: &str = "\
phoenix autoroute export-dsn [--settings FILE] [--rules FILE] [--json] [--lang CODE]
                             [--out FILE] <board>
phoenix autoroute import-ses [--settings FILE] [--rules FILE] [--json] [--lang CODE]
                             [--require-complete] [--out FILE] <board> <session>
phoenix autoroute run [--config FILE] [--settings FILE] [--rules FILE] [--json] [--lang CODE]
                      [--require-complete] [--execute] [--out FILE] [--ses-out FILE] <board>
phoenix autoroute native [--nets N1,N2] [--layers L1,L2] [--config FILE] [--out FILE] <board>

  export-dsn    write the board as a Specctra DSN file for an external autorouter: layers, outline,
                the width and clearance of the rules, pads, nets, net classes and the copper that
                exists (as fixed obstacles). Without --out only a summary is printed
  import-ses    read a Specctra session and check its routes against the board and the rules:
                every net, layer and via must exist and the rule check may not find a new error;
                all or nothing. Without --out only the report is printed; with --out the board
                with the routes is written to a new file (an existing file is never replaced)
  run           start the autorouter of config/autorouter.cfg on the DSN file and import the
                session it writes. A dry run without --execute (it prints the command that would
                be started); with --execute it starts the program only if the settings say
                enabled = true. --ses-out keeps a copy of the session
  native        route with the autorouter that needs no outside program (a dry run unless --out
                is given; run `phoenix autoroute native` alone for its options)
  --require-complete  refuse a session that leaves connections unrouted
  --rules       the JSON rule file (format phoenix-rules/1 or 2)
  --settings    the pitch, penalties and limits of the router (default: config/route.cfg)
  --config      the settings of the autorouter (default: config/autorouter.cfg when it exists)
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

#[derive(Default)]
struct Options {
    json: bool,
    lang: Option<Lang>,
    settings: Option<String>,
    rules: Option<String>,
    config: Option<String>,
    out: Option<String>,
    ses_out: Option<String>,
    execute: bool,
    require_complete: bool,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, RouteError> {
    let mut o = Options::default();
    let bad = |what: &str| RouteError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| -> Result<String, RouteError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(&format!("{name} needs a value")))
        };
        match *arg {
            "--json" => o.json = true,
            "--execute" => o.execute = true,
            "--require-complete" => o.require_complete = true,
            "--lang" => {
                let tag = value("--lang")?;
                o.lang = Some(Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--settings" => o.settings = Some(value("--settings")?),
            "--rules" => o.rules = Some(value("--rules")?),
            "--config" => o.config = Some(value("--config")?),
            "--out" => o.out = Some(value("--out")?),
            "--ses-out" => o.ses_out = Some(value("--ses-out")?),
            other if other.starts_with("--") => {
                return Err(RouteError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn fail(err: &mut dyn Write, error: &RouteError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error {}", error.message(lang));
    2
}

fn json_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn dsn_line(dsn: &Dsn) -> String {
    format!(
        "{} layers, {} components, {} pads, {} nets, {} padstacks, {} tracks, {} vias, {} bytes",
        dsn.layers,
        dsn.components,
        dsn.pads,
        dsn.nets,
        dsn.padstacks,
        dsn.tracks,
        dsn.vias,
        dsn.text.len()
    )
}

fn dsn_json(dsn: &Dsn) -> String {
    format!(
        "{{\"layers\":{},\"components\":{},\"pads\":{},\"nets\":{},\"padstacks\":{},\"tracks\":{},\"vias\":{},\"bytes\":{}}}",
        dsn.layers,
        dsn.components,
        dsn.pads,
        dsn.nets,
        dsn.padstacks,
        dsn.tracks,
        dsn.vias,
        dsn.text.len()
    )
}

/// What every sub-command needs: the settings, the board and the rules.
struct Inputs {
    settings: RouteSettings,
    board: Board,
    ctx: RuleContext,
}

fn load_inputs(
    options: &Options,
    board_path: &str,
    err: &mut dyn Write,
) -> Result<Inputs, RouteError> {
    let settings = load_settings(options.settings.as_deref())?;
    let board = load_board(board_path)?;
    let ctx = load_context(options.rules.as_deref(), board_path, err)?;
    Ok(Inputs {
        settings,
        board,
        ctx,
    })
}

pub(crate) fn autoroute_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let Some((&sub, tail)) = args.split_first() else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    if sub == "native" {
        return crate::autoroute_native::native_command(tail, out, err);
    }
    if !matches!(sub, "export-dsn" | "import-ses" | "run") {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(tail) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang.unwrap_or(Lang::En);
    let result = match sub {
        "export-dsn" => export_command(&options, out, err),
        "import-ses" => import_command(&options, out, err),
        _ => run_command(&options, out, err),
    };
    match result {
        Ok(()) => 0,
        Err(Failure::Usage) => {
            let _ = err.write_all(USAGE.as_bytes());
            2
        }
        Err(Failure::Error(error)) => fail(err, &error, lang),
    }
}

enum Failure {
    Usage,
    Error(RouteError),
}

impl From<RouteError> for Failure {
    fn from(error: RouteError) -> Failure {
        Failure::Error(error)
    }
}

fn export_command(
    options: &Options,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    let [board_path] = options.files.as_slice() else {
        return Err(Failure::Usage);
    };
    let inputs = load_inputs(options, board_path, err)?;
    let dsn = export_dsn(&inputs.board, &inputs.ctx, &inputs.settings)?;
    for note in &dsn.notes {
        let _ = writeln!(err, "note: {note}");
    }
    if let Some(path) = &options.out {
        write_new(path, dsn.text.as_bytes())?;
    } else {
        let _ = writeln!(err, "note: dry run, nothing is written (give --out FILE)");
    }
    if options.json {
        let _ = writeln!(
            out,
            "{{\"written\":{},\"dsn\":{}}}",
            options.out.is_some(),
            dsn_json(&dsn)
        );
    } else {
        let _ = writeln!(out, "dsn: {}", dsn_line(&dsn));
    }
    Ok(())
}

/// Reads the session, makes the plan and, with `--out`, writes the board with the routes.
fn plan_session(options: &Options, inputs: &Inputs, session: &str) -> Result<SesPlan, RouteError> {
    let mut new_id = ids()?;
    let plan = import_ses(
        &inputs.board,
        &inputs.ctx,
        &inputs.settings,
        session,
        &mut new_id,
    )?;
    if options.require_complete {
        plan.require_complete()?;
    }
    if let Some(path) = &options.out {
        let mut routed = inputs.board.clone();
        plan.apply(&mut routed)?;
        let bytes = routed.to_bytes().map_err(|e| {
            RouteError::new(eh3d_phoenix_route::codes::OUTPUT_FAILED, e.to_string())
        })?;
        write_new(path, &bytes)?;
    }
    Ok(plan)
}

fn print_plan(options: &Options, plan: &SesPlan, out: &mut dyn Write) {
    if options.json {
        let _ = writeln!(out, "{}", plan.to_json());
    } else {
        let _ = out.write_all(plan.to_text().as_bytes());
    }
}

fn import_command(
    options: &Options,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    let [board_path, session_path] = options.files.as_slice() else {
        return Err(Failure::Usage);
    };
    let inputs = load_inputs(options, board_path, err)?;
    let session = read_text(session_path).map_err(|m| RouteError::new(INPUT_UNREADABLE, m))?;
    let plan = plan_session(options, &inputs, &session)?;
    if options.out.is_none() {
        let _ = writeln!(err, "note: dry run, nothing is written (give --out FILE)");
    }
    print_plan(options, &plan, out);
    Ok(())
}

fn load_config(path: Option<&str>) -> Result<AutorouterConfig, RouteError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(AutorouterConfig::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| RouteError::new(INPUT_UNREADABLE, m))?;
            AutorouterConfig::parse(&text)
        }
    }
}

fn run_command(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    let [board_path] = options.files.as_slice() else {
        return Err(Failure::Usage);
    };
    let config = load_config(options.config.as_deref())?;
    let inputs = load_inputs(options, board_path, err)?;
    let dsn = export_dsn(&inputs.board, &inputs.ctx, &inputs.settings)?;
    if !options.execute {
        let command_text = match launch_plan(&config, &run_name(&dsn.text)) {
            Ok(launch) => launch.describe(),
            Err(e) => format!("not configured ({})", e.detail),
        };
        if options.json {
            let _ = writeln!(
                out,
                "{{\"mode\":\"dry_run\",\"enabled\":{},\"dsn\":{},\"command\":\"{}\"}}",
                config.enabled,
                dsn_json(&dsn),
                json_text(&command_text)
            );
        } else {
            let _ = writeln!(out, "autoroute dry run, nothing is started");
            let _ = writeln!(out, "  dsn: {}", dsn_line(&dsn));
            let switch = if config.enabled {
                "on"
            } else {
                "off (enabled = false)"
            };
            let _ = writeln!(out, "  external autorouter: {switch}");
            let _ = writeln!(out, "  command: {command_text}");
        }
        return Ok(());
    }
    let session = eh3d_phoenix_route::autorouter::run(&config, &dsn.text)?;
    if let Some(path) = &options.ses_out {
        write_new(path, session.as_bytes())?;
    }
    let plan = plan_session(options, &inputs, &session)?;
    if options.out.is_none() {
        let _ = writeln!(err, "note: the board is not written (give --out FILE)");
    }
    print_plan(options, &plan, out);
    Ok(())
}
