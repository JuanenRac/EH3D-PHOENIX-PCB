// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `ulp`: the reader, the compatibility report and the evaluator of the user programs
//! of the reference tool. `phoenix ulp check` reads programs and lists what is wrong with their
//! shape; `phoenix ulp report` says what they use and what the list of allowed names covers;
//! `phoenix ulp run` runs one program (pure computation and read-only walks over a board and a
//! schematic) and prints what it prints. Nothing is written to a file, and only `run` runs a
//! program.
//!
//! Exit codes of `check`: 0 every program was read without a problem, 1 some program has a
//! problem, 2 the arguments or a file are unusable. Exit codes of `report`: 0 every program is
//! covered, 1 some program is partly or not covered, 2 as above. Exit codes of `run`: 0 the program
//! ran and, when `--expect` is given, its output is the expected one; 2 the verification did not
//! pass (the output differs, or the program ended with `exit` and a number other than zero); 1 the
//! program could not be run (bad arguments, an unreadable file, problems of reading, an error of
//! the run or a limit).

use std::io::{Read, Write};
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_ulp::codes::{INPUT_UNREADABLE, OPTION_INVALID, RUN_OUTPUT_DIFFERS};
use eh3d_phoenix_ulp::{
    analyse, parse, read_text_limited, report_json, report_text, run_source, Diag, Support,
    UlpSettings, Verdict, World,
};
use serde_json::json;

use crate::check::id_source;
use crate::design::{footprint_names, load_board, load_schematic};

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/ulp.cfg";

/// The most programs one command reads.
const MAX_FILES: usize = 10_000;

pub(crate) const USAGE: &str =
    "phoenix ulp check|report [--config FILE] [--support FILE] [--json] [--lang CODE] <program>...
phoenix ulp run [--config FILE] [--board FILE] [--schematic FILE] [--expect FILE] [--lang CODE]
                <program>

  ulp check     read the programs and list what is wrong with their shape, with the line and
                the column and the codes PHOENIX-ULP-<NNN>: texts and comments that are not
                closed, numbers that are not valid, a mark that is missing, blocks nested too
                deep; exit code 1 when a program has a problem
  ulp report    what each program uses (directives, kinds of statement, types, functions,
                members and named values) and what the list of allowed names says about each,
                with a verdict: covered, partly covered or not covered; exit code 1 unless every
                program is covered. Nothing is run: the evaluator is not written yet, so the
                verdict says what the list would let through
  ulp run       run one program with the evaluator: pure computation (numbers, texts, arrays,
                functions of the program) and read-only walks over the design (board(B),
                B.elements(E), ...), within the limits of steps, memory, nesting and output of
                the settings; what the program prints goes to the standard output, the same
                bytes for the same program and design. It opens no file, starts no program and
                changes nothing. The design is given with --board (a .json of the model, an
                EAGLE .brd or a KiCad .kicad_pcb) and --schematic. --expect FILE compares the
                output with a file (line ends normalised). Exit code 0 ran and, with --expect,
                the output is the expected one; 2 the verification did not pass (other output,
                or exit with a number other than 0); 1 it could not be run. The sandbox in a
                process of its own is not written yet
  --config      the limits of the reader and of the run (default: config/ulp.cfg when it exists)
  --support     the list of allowed names (default: the list that ships with the program,
                config/ulp_support.cfg)
  --json        print the result as JSON, the same every time
  --lang CODE   the words in en, es, de, fr, it, ja, zh or pt
  A prototype; it reads files and writes none.
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Check,
    Report,
    Run,
}

struct Options {
    mode: Mode,
    json: bool,
    lang: Lang,
    config: Option<String>,
    support: Option<String>,
    board: Option<String>,
    schematic: Option<String>,
    expect: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, Diag> {
    let bad = |what: &str| Diag::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    let mode = match iter.next() {
        Some(&"check") => Mode::Check,
        Some(&"report") => Mode::Report,
        Some(&"run") => Mode::Run,
        _ => return Err(bad("the first word is `check`, `report` or `run`")),
    };
    let mut options = Options {
        mode,
        json: false,
        lang: Lang::En,
        config: None,
        support: None,
        board: None,
        schematic: None,
        expect: None,
        files: Vec::new(),
    };
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, Diag> {
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
            "--support" => options.support = Some(value("--support needs a file")?),
            "--board" if mode == Mode::Run => {
                options.board = Some(value("--board needs a file")?);
            }
            "--schematic" if mode == Mode::Run => {
                options.schematic = Some(value("--schematic needs a file")?);
            }
            "--expect" if mode == Mode::Run => {
                options.expect = Some(value("--expect needs a file")?);
            }
            other if other.starts_with("--") => {
                return Err(Diag::new(OPTION_INVALID, format!("unknown option {other}")))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    if options.files.is_empty() {
        return Err(bad("give at least one program"));
    }
    if mode == Mode::Run && options.files.len() != 1 {
        return Err(bad("`run` takes one program"));
    }
    if options.files.len() > MAX_FILES {
        return Err(bad("too many programs for one command"));
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<UlpSettings, Diag> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(UlpSettings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| Diag::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            UlpSettings::parse(&text)
        }
    }
}

fn load_support(path: Option<&str>) -> Result<Support, Diag> {
    match path {
        None => Ok(Support::shipped()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| Diag::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            Support::parse(&text).map_err(|mut d| {
                d.detail = format!("{path}: {}", d.detail);
                d
            })
        }
    }
}

fn fail(err: &mut dyn Write, error: &Diag, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// One program read by `check`: the lines of text and the JSON, and whether it is clean.
fn check_one(
    path: &str,
    source: &str,
    settings: &UlpSettings,
    lang: Lang,
) -> (String, serde_json::Value, bool) {
    let diag_json = |d: &Diag| {
        json!({
            "code": d.code,
            "line": d.line,
            "col": d.col,
            "detail": d.detail,
        })
    };
    match parse(source, settings) {
        Ok(parsed) => {
            let p = &parsed.program;
            let clean = parsed.problems.total() == 0;
            let mut text = format!(
                "{path}: {} statements, {} functions, {} directives, {} problems\n",
                p.statement_count(),
                p.functions.len(),
                p.directives.len(),
                parsed.problems.total()
            );
            for d in &parsed.problems.list {
                text.push_str(&format!("  {}\n", d.message(lang)));
            }
            if parsed.problems.dropped > 0 {
                text.push_str(&format!(
                    "  {} (+{})\n",
                    Diag::new(eh3d_phoenix_ulp::codes::LIST_CUT, "").message(lang),
                    parsed.problems.dropped
                ));
            }
            let value = json!({
                "program": path,
                "statements": p.statement_count(),
                "functions": p.functions.len(),
                "directives": p.directives.len(),
                "problems": parsed.problems.list.iter().map(diag_json).collect::<Vec<_>>(),
                "problems_dropped": parsed.problems.dropped,
                "stopped": serde_json::Value::Null,
            });
            (text, value, clean)
        }
        Err(stopped) => {
            let text = format!("{path}: {}\n", stopped.message(lang));
            let value = json!({
                "program": path,
                "problems": [],
                "problems_dropped": 0,
                "stopped": diag_json(&stopped),
            });
            (text, value, false)
        }
    }
}

fn run(options: &Options, out: &mut dyn Write) -> Result<i32, Diag> {
    let settings = load_settings(options.config.as_deref())?;
    let support = load_support(options.support.as_deref())?;
    let mut sources = Vec::new();
    for path in &options.files {
        let source = read_text_limited(Path::new(path), settings.max_source_bytes)?;
        sources.push((path.as_str(), source));
    }
    let mut all_good = true;
    let mut text = String::new();
    let mut values = Vec::new();
    for (path, source) in &sources {
        match options.mode {
            Mode::Check => {
                let (t, v, clean) = check_one(path, source, &settings, options.lang);
                all_good &= clean;
                text.push_str(&t);
                values.push(v);
            }
            Mode::Run => {}
            Mode::Report => {
                let report = analyse(path, source, &settings, &support);
                all_good &= report.verdict == Verdict::Covered;
                if options.json {
                    values.push(serde_json::from_str(&report_json(&report)).unwrap_or_default());
                } else {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&report_text(&report, options.lang));
                }
            }
        }
    }
    if options.json {
        let format = match options.mode {
            Mode::Check => "phoenix-ulp-check/1",
            Mode::Report | Mode::Run => "phoenix-ulp-reports/1",
        };
        let value = json!({ "format": format, "programs": values });
        text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
    }
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(!all_good))
}

/// The text of a file that holds the expected output, at most `max` bytes, with the line ends
/// normalised to `\n`.
fn read_expected(path: &str, max: usize) -> Result<String, Diag> {
    let unreadable = |e: std::io::Error| Diag::new(INPUT_UNREADABLE, format!("{path}: {e}"));
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(unreadable)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    Ok(String::from_utf8_lossy(&bytes).replace("\r\n", "\n"))
}

/// The place of the first difference between two texts, as a byte offset.
fn first_difference(a: &str, b: &str) -> usize {
    a.bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()))
}

/// `phoenix ulp run`: exit code 0 ran (and the output is the expected one), 2 the verification did
/// not pass, 1 it could not be run.
fn run_program(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let lang = options.lang;
    let stop = |err: &mut dyn Write, d: &Diag| {
        let _ = writeln!(err, "error: {}", d.message(lang));
        1
    };
    let settings = match load_settings(options.config.as_deref()) {
        Ok(s) => s,
        Err(d) => return stop(err, &d),
    };
    let support = match load_support(options.support.as_deref()) {
        Ok(s) => s,
        Err(d) => return stop(err, &d),
    };
    let path = &options.files[0];
    let source = match read_text_limited(Path::new(path), settings.max_source_bytes) {
        Ok(text) => text,
        Err(d) => return stop(err, &d),
    };
    let unreadable = |message: String| Diag::new(INPUT_UNREADABLE, message);
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return stop(err, &unreadable(message)),
    };
    let board = match options.board.as_deref().map(|p| load_board(p, &mut new_id)) {
        None => None,
        Some(Ok(board)) => Some(board),
        Some(Err(message)) => return stop(err, &unreadable(message)),
    };
    let schematic = match options
        .schematic
        .as_deref()
        .map(|p| load_schematic(p, &footprint_names(board.as_ref()), &mut new_id))
    {
        None => None,
        Some(Ok(schematic)) => Some(schematic),
        Some(Err(message)) => return stop(err, &unreadable(message)),
    };
    let world = World {
        board: board.as_ref(),
        schematic: schematic.as_ref(),
    };
    let outcome = match run_source(&source, &settings, &support, world) {
        Ok(outcome) => outcome,
        Err(d) => return stop(err, &d),
    };
    let _ = out.write_all(outcome.output.as_bytes());
    if outcome.exit_code != 0 {
        let _ = writeln!(err, "exit code {}", outcome.exit_code);
        return 2;
    }
    if let Some(expected_path) = &options.expect {
        let expected = match read_expected(expected_path, settings.max_output_bytes) {
            Ok(text) => text,
            Err(d) => return stop(err, &d),
        };
        if expected != outcome.output {
            let d = Diag::new(
                RUN_OUTPUT_DIFFERS,
                format!(
                    "first difference at byte {}",
                    first_difference(&expected, &outcome.output)
                ),
            );
            let _ = writeln!(err, "error: {}", d.message(lang));
            return 2;
        }
    }
    0
}

pub(crate) fn ulp_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let run_mode = args.first() == Some(&"run");
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let code = fail(err, &error, Lang::En);
            // a run that cannot start is 1 and not 2: 2 is the verification that did not pass
            return if run_mode { 1 } else { code };
        }
    };
    if options.mode == Mode::Run {
        return run_program(&options, out, err);
    }
    match run(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
