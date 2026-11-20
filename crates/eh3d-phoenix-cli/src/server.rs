// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `server`: a board, or a file of measurements, is sent to EH3D-SERVER as a job of
//! its plugin `pcb` and the report of the station is read back.
//!
//! Exit codes: 0 done (a dry run, or a report whose verdict is a pass), 1 the verdict of the
//! server is not a pass, 2 the arguments, the input or the link are unusable. Nothing is sent
//! unless `allow_network` is true in the settings and `--send` is given; nothing is written.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_server::codes::{INPUT_UNREADABLE, OPTION_INVALID};
use eh3d_phoenix_server::request::clean_name;
use eh3d_phoenix_server::{
    check_reachable, connect, dry_run, prepare_board, prepare_measurements, probe, probe_json,
    probe_text, read_report, report_json, report_text, send, Kind, ServerError, ServerReport,
    ServerSettings, SleepPause,
};

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/server.cfg";

pub(crate) const USAGE: &str = "phoenix server [--config FILE] [--kind rules|drill|bom|inspect] [--send] [--json] [--lang CODE] <board | measurements>
phoenix server [--config FILE] --health [--json] [--lang CODE]
phoenix server [--config FILE] --job ID [--json] [--lang CODE]

  server        a report of the board made by the station of EH3D-SERVER: the rules of the maker
                against the tracks, the gaps and the holes (--kind rules, the default), the
                drilling (drill), the parts list (bom) or, with a file of lines
                `name, nominal, tolerance, value[, unit]`, an inspection (inspect). Without
                --send the command only shows the request it would send; with --send, and
                allow_network = true in the settings, the request is sent as a job of the plugin
                pcb and the report of the station is read back, with the codes
                PHOENIX-SERVER-<NNN>; the exit code is 1 when the verdict is not a pass. A
                prototype that has been tried against a server in memory and a small server on
                this computer only. --health asks the server whether it is there and well, and
                --job reads the report of a job that is already on the server (one made with
                start_job = false and started by a person, for example); both need
                allow_network = true in the settings and no file
  --config      the link to the server and every limit (default: config/server.cfg when it exists)
  --kind        the report asked of the station
  --send        send the request (the settings must allow it)
  --health      ask the server whether it is there and well (no token is sent)
  --job ID      read the state and the report of the job with this id
  --json        print the report as JSON, the same every time
  --lang CODE   the words in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    send: bool,
    health: bool,
    job: Option<String>,
    lang: Lang,
    kind: Kind,
    config: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, ServerError> {
    let mut options = Options {
        json: false,
        send: false,
        health: false,
        job: None,
        lang: Lang::En,
        kind: Kind::Rules,
        config: None,
        files: Vec::new(),
    };
    let bad = |what: &str| ServerError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, ServerError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--send" => options.send = true,
            "--health" => options.health = true,
            "--job" => options.job = Some(value("--job needs the id of a job")?),
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--kind" => {
                let word = value("--kind needs rules, drill, bom or inspect")?;
                options.kind = Kind::from_word(&word)
                    .ok_or_else(|| bad("--kind is rules, drill, bom or inspect"))?;
            }
            "--config" => options.config = Some(value("--config needs a file")?),
            other if other.starts_with("--") => {
                return Err(ServerError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<ServerSettings, ServerError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(ServerSettings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| ServerError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            ServerSettings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &ServerError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Prints a report and gives the exit code that goes with it.
fn print_report(
    report: &ServerReport,
    options: &Options,
    settings: &ServerSettings,
    out: &mut dyn Write,
) -> i32 {
    let text = if options.json {
        report_json(report, settings.max_listed)
    } else {
        report_text(report, settings.max_listed, options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    i32::from(report.has_errors())
}

/// `--health` and `--job`: what the server says of itself or of a job, with no file.
fn ask_server(options: &Options, out: &mut dyn Write) -> Result<i32, ServerError> {
    if (options.health && options.job.is_some()) || options.send || !options.files.is_empty() {
        return Err(ServerError::new(
            OPTION_INVALID,
            "--health and --job take no file, no --send and are not given together",
        ));
    }
    let settings = load_settings(options.config.as_deref())?;
    let mut client = connect(&settings);
    if let Some(id) = &options.job {
        let report = read_report(&mut client, &settings, id, options.lang)?;
        return Ok(print_report(&report, options, &settings, out));
    }
    check_reachable(&settings)?;
    let found = probe(&mut client, options.lang)?;
    let text = if options.json {
        probe_json(&found)
    } else {
        probe_text(&found, options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(found.status != "ok"))
}

fn server(options: &Options, out: &mut dyn Write) -> Result<i32, ServerError> {
    if options.health || options.job.is_some() {
        return ask_server(options, out);
    }
    let [path] = options.files.as_slice() else {
        return Err(ServerError::new(
            OPTION_INVALID,
            "give exactly one board or measurements file",
        ));
    };
    let settings = load_settings(options.config.as_deref())?;
    let prepared = if options.kind == Kind::Inspect {
        let text = crate::bounded::read_config_text(path)
            .map_err(|e| ServerError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
        prepare_measurements(&text, &settings)?
    } else {
        let board = read_board_file(path).map_err(|e| ServerError::new(INPUT_UNREADABLE, e))?;
        prepare_board(options.kind, &board, &settings)?
    };
    let report = if options.send {
        let name = clean_name(
            Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("board"),
        );
        let mut client = connect(&settings);
        send(
            &mut client,
            &mut SleepPause,
            &settings,
            &name,
            prepared,
            options.lang,
        )?
    } else {
        dry_run(prepared)
    };
    Ok(print_report(&report, options, &settings, out))
}

pub(crate) fn server_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    match server(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
