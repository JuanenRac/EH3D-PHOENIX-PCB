// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `autoroute native`: the autorouter of `eh3d-phoenix-autoroute`, which needs no
//! outside program. It reads the board, the rules and the settings, routes the open connections
//! of the nets asked for (all of them by default) and prints the report. Without `--out` it is a
//! dry run; with `--out` the board with the routes is written to a new file, and an existing
//! file is never replaced.
//!
//! Exit codes: 0 done, 2 the arguments, the input, the settings or the rules refuse it, or
//! `--require-complete` was given and a connection is left unrouted.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_autoroute::codes::{INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_autoroute::{autoroute, AutorouteConfig, AutorouteError, Request};
use eh3d_phoenix_route::RouteError;
use eh3d_phoenix_rules::Lang;

use crate::check::read_text;
use crate::route::{load_board, load_context, load_settings, write_new};

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/autoroute_native.cfg";

pub(crate) const USAGE: &str = "\
phoenix autoroute native [--nets N1,N2] [--layers L1,L2] [--config FILE] [--settings FILE]
                         [--rules FILE] [--json] [--lang CODE] [--require-complete]
                         [--out FILE] <board>

  native        route the open connections with the autorouter that needs no outside program: a
                grid from the rules, a search per layer with vias, rip-up and reroute within
                limits that count nodes. The plan only adds tracks and vias, is checked by the
                rules before and after and never adds a rule error. Without --out only the
                report is printed; with --out the board with the routes is written to a new
                file (an existing file is never replaced). A prototype
  --nets        route only these nets, by name (default: every net with an open connection)
  --layers      let the tracks use only these copper layers, by name (default: all of them)
  --config      the settings of the search (default: config/autoroute_native.cfg when it exists)
  --settings    the width of the tracks and the size of the vias (default: config/route.cfg)
  --rules       the JSON rule file (format phoenix-rules/1 or 2)
  --require-complete  refuse a plan that leaves connections unrouted (exit code 2)
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

#[derive(Debug, Default)]
struct Options {
    json: bool,
    lang: Option<Lang>,
    nets: Vec<String>,
    layers: Vec<String>,
    settings: Option<String>,
    rules: Option<String>,
    config: Option<String>,
    out: Option<String>,
    require_complete: bool,
    files: Vec<String>,
}

enum Failure {
    Usage,
    Option(AutorouteError),
    Route(RouteError),
    Auto(AutorouteError),
}

impl From<RouteError> for Failure {
    fn from(error: RouteError) -> Failure {
        Failure::Route(error)
    }
}

impl From<AutorouteError> for Failure {
    fn from(error: AutorouteError) -> Failure {
        Failure::Auto(error)
    }
}

fn names(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

fn parse_options(args: &[&str]) -> Result<Options, AutorouteError> {
    let mut o = Options::default();
    let bad = |what: &str| AutorouteError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| -> Result<String, AutorouteError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(&format!("{name} needs a value")))
        };
        match *arg {
            "--json" => o.json = true,
            "--require-complete" => o.require_complete = true,
            "--lang" => {
                let tag = value("--lang")?;
                o.lang = Some(Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--nets" => o.nets.extend(names(&value("--nets")?)),
            "--layers" => o.layers.extend(names(&value("--layers")?)),
            "--settings" => o.settings = Some(value("--settings")?),
            "--rules" => o.rules = Some(value("--rules")?),
            "--config" => o.config = Some(value("--config")?),
            "--out" => o.out = Some(value("--out")?),
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn load_config(path: Option<&str>) -> Result<AutorouteConfig, AutorouteError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(AutorouteConfig::default()),
        Some(path) => {
            let text = read_text(path).map_err(|m| AutorouteError::new(INPUT_UNREADABLE, m))?;
            AutorouteConfig::parse(&text)
        }
    }
}

fn execute(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    let [board_path] = options.files.as_slice() else {
        return Err(Failure::Usage);
    };
    let lang = options.lang.unwrap_or(Lang::En);
    let config = load_config(options.config.as_deref())?;
    let settings = load_settings(options.settings.as_deref())?;
    let board = load_board(board_path)?;
    let ctx = load_context(options.rules.as_deref(), board_path, err)?;
    let request = Request {
        nets: options.nets.clone(),
        layers: options.layers.clone(),
    };
    let plan = autoroute(&board, &ctx, &settings, &config, &request)?;
    if options.require_complete {
        plan.require_complete()?;
    }
    if let Some(path) = &options.out {
        let bytes = plan
            .board_after
            .to_bytes()
            .map_err(|e| AutorouteError::new(OUTPUT_FAILED, e.to_string()))?;
        write_new(path, &bytes)?;
    } else {
        let _ = writeln!(err, "note: dry run, nothing is written (give --out FILE)");
    }
    if options.json {
        let _ = writeln!(out, "{}", plan.to_json());
    } else {
        let _ = out.write_all(plan.to_text(lang.tag()).as_bytes());
    }
    Ok(())
}

/// Runs `phoenix autoroute native`; `args` are the words after `native`.
pub(crate) fn native_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => return finish(Failure::Option(error), Lang::En, err),
    };
    let lang = options.lang.unwrap_or(Lang::En);
    match execute(&options, out, err) {
        Ok(()) => 0,
        Err(failure) => finish(failure, lang, err),
    }
}

fn finish(failure: Failure, lang: Lang, err: &mut dyn Write) -> i32 {
    match failure {
        Failure::Usage => {
            let _ = err.write_all(USAGE.as_bytes());
        }
        Failure::Option(error) => {
            let _ = writeln!(err, "error {}", error.message(lang.tag()));
            let _ = err.write_all(USAGE.as_bytes());
        }
        Failure::Route(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
        }
        Failure::Auto(error) => {
            let _ = writeln!(err, "error {}", error.message(lang.tag()));
        }
    }
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_options_are_read_and_the_lists_are_split_on_commas() {
        let o = parse_options(&[
            "--nets",
            "A, B,,C",
            "--layers",
            "Top",
            "--json",
            "--require-complete",
            "--lang",
            "pt",
            "--out",
            "x.json",
            "board.json",
        ])
        .unwrap();
        assert_eq!(o.nets, words(&["A", "B", "C"]));
        assert_eq!(o.layers, words(&["Top"]));
        assert!(o.json && o.require_complete);
        assert_eq!(o.lang, Some(Lang::Pt));
        assert_eq!(o.out.as_deref(), Some("x.json"));
        assert_eq!(o.files, words(&["board.json"]));
    }

    #[test]
    fn a_wrong_option_is_refused_with_its_code() {
        for args in [
            &["--nope"][..],
            &["--nets"][..],
            &["--lang", "xx"][..],
            &["--out"][..],
        ] {
            let e = parse_options(args).expect_err("refused");
            assert_eq!(e.code, OPTION_INVALID, "{args:?}");
        }
    }

    #[test]
    fn without_a_board_the_usage_is_printed_and_the_code_is_two() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(native_command(&[], &mut out, &mut err), 2);
        assert!(String::from_utf8(err).unwrap().contains("autoroute native"));
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(native_command(&["--nope"], &mut out, &mut err), 2);
        assert!(String::from_utf8(err)
            .unwrap()
            .contains("PHOENIX-AROUTE-002"));
    }

    #[test]
    fn a_board_that_cannot_be_read_is_an_error_with_a_code() {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = native_command(&["definitely-not-a-file.json"], &mut out, &mut err);
        assert_eq!(code, 2);
        let text = String::from_utf8(err).unwrap();
        assert!(text.starts_with("error PHOENIX-ROUTE-"), "{text}");
    }
}
