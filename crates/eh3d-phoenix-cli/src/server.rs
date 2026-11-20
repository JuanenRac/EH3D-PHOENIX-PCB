// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `server`: a board, or a file of measurements, is sent to EH3D-SERVER as a job of
//! its plugin `pcb` and the report of the station is read back.
//!
//! Exit codes: 0 done (a dry run, or a report whose verdict is a pass), 1 the verdict of the
//! server is not a pass, 2 the arguments, the input or the link are unusable. Nothing is sent
//! unless `allow_network` is true in the settings and `--send` is given; nothing is written.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_server::codes::{INPUT_UNREADABLE, LIMIT_EXCEEDED, OPTION_INVALID, OUTPUT_EXISTS};
use eh3d_phoenix_server::request::clean_name;
use eh3d_phoenix_server::{
    check_reachable, connect, dry_run, fetched_json, fetched_text, load_project, prepare_board,
    prepare_measurements, probe, probe_json, probe_text, read_report, report_json, report_text,
    save_project, saved_json, saved_text, send, Kind, ServerError, ServerReport, ServerSettings,
    SleepPause,
};

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/server.cfg";

pub(crate) const USAGE: &str = "phoenix server [--config FILE] [--kind rules|drill|bom|inspect] [--send] [--json] [--lang CODE] <board | measurements>
phoenix server [--config FILE] --health [--json] [--lang CODE]
phoenix server [--config FILE] --job ID [--json] [--lang CODE]
phoenix server [--config FILE] --save-project PROJECT [--name NAME] [--tag TAG] [--json] [--lang CODE] <file>
phoenix server [--config FILE] --load-project PROJECT --name NAME --out FILE [--version N] [--json] [--lang CODE]

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
                allow_network = true in the settings and no file; --save-project keeps a file
                of a board in the project store of the server (application phoenix-pcb) and
                --load-project reads it back with the tag of its version, which a later save
                names with --tag: a save over a file that exists without the tag is refused
                (PHOENIX-SERVER-015) and so is one whose tag is no longer the current version
                (PHOENIX-SERVER-016); both need allow_network = true and a token
  --config      the link to the server and every limit (default: config/server.cfg when it exists)
  --kind        the report asked of the station
  --send        send the request (the settings must allow it)
  --health      ask the server whether it is there and well (no token is sent)
  --job ID      read the state and the report of the job with this id
  --save-project PROJECT
                save the file to the project store, as NAME (default: the name of the file)
  --load-project PROJECT
                read the file NAME of the project from the store and write it to --out, a new
                file (one that exists is never replaced); --version N reads an older version
  --name NAME   the name of the file in the store (letters, digits, _, - and . in pieces
                joined by /)
  --tag TAG     the version a save is made from, as --load-project or the last save printed it;
                none for a file that does not exist yet
  --json        print the report as JSON, the same every time
  --lang CODE   the words in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    send: bool,
    health: bool,
    job: Option<String>,
    save_project: Option<String>,
    load_project: Option<String>,
    name: Option<String>,
    tag: Option<String>,
    out: Option<String>,
    version: Option<u64>,
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
        save_project: None,
        load_project: None,
        name: None,
        tag: None,
        out: None,
        version: None,
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
            "--save-project" => {
                options.save_project = Some(value("--save-project needs a project")?)
            }
            "--load-project" => {
                options.load_project = Some(value("--load-project needs a project")?)
            }
            "--name" => options.name = Some(value("--name needs a file name")?),
            "--tag" => options.tag = Some(value("--tag needs the tag of a version")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            "--version" => {
                options.version = Some(
                    value("--version needs a number")?
                        .parse()
                        .map_err(|_| bad("--version is a whole number from 1"))?,
                )
            }
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

impl Options {
    /// Whether any option of the project store was given.
    fn store_used(&self) -> bool {
        self.save_project.is_some()
            || self.load_project.is_some()
            || self.name.is_some()
            || self.tag.is_some()
            || self.out.is_some()
            || self.version.is_some()
    }
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, bytes: &[u8]) -> Result<(), ServerError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                ServerError::new(OUTPUT_EXISTS, path.to_owned())
            } else {
                ServerError::new(OPTION_INVALID, format!("{path}: {e}"))
            }
        })?;
    file.write_all(bytes)
        .map_err(|e| ServerError::new(OPTION_INVALID, format!("{path}: {e}")))
}

/// Reads a file to save, at most `limit` bytes (a larger file is refused).
fn read_upload(path: &str, limit: usize) -> Result<Vec<u8>, ServerError> {
    let unreadable = |e: std::io::Error| ServerError::new(INPUT_UNREADABLE, format!("{path}: {e}"));
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(unreadable)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() > limit {
        return Err(ServerError::new(
            LIMIT_EXCEEDED,
            format!("{path} is larger than project_max_mib"),
        ));
    }
    Ok(bytes)
}

/// `--save-project` and `--load-project`: a file of a board to the project store of the server
/// and back, from the version it was made from.
fn use_store(options: &Options, out: &mut dyn Write) -> Result<i32, ServerError> {
    let bad = |what: &str| ServerError::new(OPTION_INVALID, what.to_owned());
    if options.health || options.job.is_some() || options.send {
        return Err(bad(
            "--save-project and --load-project are not given with --health, --job or --send",
        ));
    }
    let settings = load_settings(options.config.as_deref())?;
    let mut client = connect(&settings);
    let text = match (&options.save_project, &options.load_project) {
        (Some(project), None) => {
            let [path] = options.files.as_slice() else {
                return Err(bad("--save-project takes exactly one file"));
            };
            if options.out.is_some() || options.version.is_some() {
                return Err(bad("--out and --version belong to --load-project"));
            }
            let name = options.name.clone().unwrap_or_else(|| {
                Path::new(path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_owned()
            });
            let bytes = read_upload(path, settings.project_max_mib as usize * 1024 * 1024)?;
            let saved = save_project(
                &mut client,
                &settings,
                project,
                &name,
                &bytes,
                options.tag.as_deref(),
                options.lang,
            )?;
            if options.json {
                saved_json(&saved)
            } else {
                saved_text(&saved, options.lang)
            }
        }
        (None, Some(project)) => {
            if !options.files.is_empty() || options.tag.is_some() {
                return Err(bad("--load-project takes no file and no --tag"));
            }
            let name = options
                .name
                .as_deref()
                .ok_or_else(|| bad("--load-project needs --name"))?;
            let path = options
                .out
                .as_deref()
                .ok_or_else(|| bad("--load-project needs --out"))?;
            if Path::new(path).exists() {
                return Err(ServerError::new(OUTPUT_EXISTS, path.to_owned()));
            }
            let fetched = load_project(
                &mut client,
                &settings,
                project,
                name,
                options.version,
                options.lang,
            )?;
            write_new(path, &fetched.bytes)?;
            if options.json {
                fetched_json(&fetched)
            } else {
                format!("{}written: {path}\n", fetched_text(&fetched, options.lang))
            }
        }
        _ => {
            return Err(bad(
                "give --save-project or --load-project, not both and not neither",
            ))
        }
    };
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

/// `--health` and `--job`: what the server says of itself or of a job, with no file.
fn ask_server(options: &Options, out: &mut dyn Write) -> Result<i32, ServerError> {
    if (options.health && options.job.is_some())
        || options.send
        || !options.files.is_empty()
        || options.store_used()
    {
        return Err(ServerError::new(
            OPTION_INVALID,
            "--health and --job take no file, no --send, no option of the store and are not given together",
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
    if options.save_project.is_some() || options.load_project.is_some() {
        return use_store(options, out);
    }
    if options.health || options.job.is_some() {
        return ask_server(options, out);
    }
    if options.store_used() {
        return Err(ServerError::new(
            OPTION_INVALID,
            "--name, --tag, --out and --version belong to --save-project and --load-project",
        ));
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
