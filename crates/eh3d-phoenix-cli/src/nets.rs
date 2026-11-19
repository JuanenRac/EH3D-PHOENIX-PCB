// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `nets`: the nets of a board as a text netlist (`NET name PART.PAD ...`), the
//! reading and checking of such a file, the comparison of a file with a board, a lint of the
//! names and the pins, and the pins as a CSV file.
//!
//! Exit codes: 0 done and nothing differs, 1 a comparison found differences or the lint found
//! something, 2 the arguments, the input or the output are unusable. Nothing is written unless
//! `--out` or `--csv` names a new file: the text goes to the output stream, and an existing file
//! is never replaced.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_sync::nets_board::board_table;
use eh3d_phoenix_sync::nets_codes::{
    self, INPUT_UNREADABLE, LIMIT_EXCEEDED, OPTION_INVALID, OUTPUT_FAILED,
};
use eh3d_phoenix_sync::nets_text::{
    compare, differences_csv, filter_nets, lint, parse, pins_csv, pins_of_part, render,
    summary as table_summary, Comparison, LintResult, NetTable, NetsError, NetsSettings, Note,
};
use serde_json::json;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/nets.cfg";

pub(crate) const USAGE: &str =
    "phoenix nets [--config FILE] [--out FILE] [--csv FILE] [--compare NETLIST] [--import NETLIST]
             [--lint] [--net PATTERNS] [--part REF] [--top N] [--json] [--lang CODE] [<board>]

  nets          the nets of a board as a text netlist, one line for each net: `NET GND U1.1 C1.2`
                (the word NET, the name, the pins as PART.PAD; a line that starts with a blank
                goes on with the pins above, `#` starts a comment). Without options the netlist
                of the board is printed; a net name with a blank, a part with a dot and the like
                are changed or left out and said in comment lines with the codes PHOENIX-NETS-<NNN>.
                A dry run unless --out or --csv names a new file (an existing file is never
                replaced)
  --import      read a netlist file instead of a board, check it against the limits of the
                settings and print it again in its canonical form (no board is given)
  --compare     compare a netlist file with the board: nets only on one side, a pin on another net
                or on none, a net under another name; exit code 1 when anything differs. With
                --import the second side is another netlist file and no board is given. With
                --out the JSON report is written
  --lint        look for nets that are probably a mistake: a net with one pin or none, and names
                that differ only in case or in separators (GND and Gnd, VCC_3V3 and VCC-3V3); exit
                code 1 when there is a finding. With --out the JSON report is written
  --net         keep only the nets whose name matches one of the patterns, separated by commas
                (any case, `*` stands for any run of characters: `GND*,*3V3`); the netlist, the
                JSON and the CSV file have those nets only. Not given with --compare or --lint
  --part REF    the pins of one part (the reference as written, `U1`) with the net each is on and
                how many pins that net has; exit code 1 when the part has no pin
  --top N       the size of the netlist (nets, parts, pins, nets with none, one and two pins) and
                the N nets with the most pins (from 1 to 1000), instead of the netlist
  --config      the limits, the nets a comparison and the lint ignore, the lint settings and the
                guard of the CSV cells (default: config/nets.cfg when it exists)
  --out FILE    write the netlist (or, with --compare or --lint, the JSON report) to a new file
  --csv FILE    write the pins as CSV (net,part,pad), or with --compare the differences
                (code,net,pin,other), to a new file; a cell that a spreadsheet would read as a
                formula (+5V) gets a quote in front of it unless the settings turn that off
  --json        print the result as JSON, the same every time
  --lang CODE   the titles in en, es, de, fr, it, ja, zh or pt. Exit code: 0 done, 1 a
                comparison differs or the lint finds something, 2 unusable input
";

struct Options {
    json: bool,
    lint: bool,
    lang: Lang,
    config: Option<String>,
    out: Option<String>,
    csv: Option<String>,
    compare: Option<String>,
    import: Option<String>,
    net: Option<String>,
    part: Option<String>,
    top: Option<usize>,
    files: Vec<String>,
}

fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The value that follows an option.
fn next_value(iter: &mut std::slice::Iter<'_, &str>, what: &str) -> Result<String, NetsError> {
    iter.next()
        .map(|v| (*v).to_owned())
        .ok_or_else(|| NetsError::new(OPTION_INVALID, what))
}

/// The value of an option that may be given once.
fn take_once(
    iter: &mut std::slice::Iter<'_, &str>,
    slot: &mut Option<String>,
    what: &str,
) -> Result<(), NetsError> {
    if slot.is_some() {
        return Err(NetsError::new(OPTION_INVALID, what));
    }
    *slot = Some(next_value(iter, what)?);
    Ok(())
}

fn parse_options(args: &[&str]) -> Result<Options, NetsError> {
    let mut options = Options {
        json: false,
        lint: false,
        lang: Lang::En,
        config: None,
        out: None,
        csv: None,
        compare: None,
        import: None,
        net: None,
        part: None,
        top: None,
        files: Vec::new(),
    };
    let bad = |what: &str| NetsError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--lint" => options.lint = true,
            "--lang" => {
                let tag = next_value(&mut iter, "--lang needs a language code")?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => take_once(&mut iter, &mut options.config, "--config needs one file")?,
            "--out" => take_once(&mut iter, &mut options.out, "--out needs one file")?,
            "--csv" => take_once(&mut iter, &mut options.csv, "--csv needs one file")?,
            "--compare" => take_once(&mut iter, &mut options.compare, "--compare needs one file")?,
            "--import" => take_once(&mut iter, &mut options.import, "--import needs one file")?,
            "--net" => take_once(
                &mut iter,
                &mut options.net,
                "--net needs one list of patterns",
            )?,
            "--part" => take_once(&mut iter, &mut options.part, "--part needs one reference")?,
            "--top" => {
                if options.top.is_some() {
                    return Err(bad("--top needs one number"));
                }
                let n = next_value(&mut iter, "--top needs a number")?;
                options.top = match n.parse::<usize>() {
                    Ok(n) if (1..=1000).contains(&n) => Some(n),
                    _ => return Err(bad("--top is a whole number from 1 to 1000")),
                };
            }
            other if other.starts_with("--") => {
                return Err(NetsError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    if options.import.is_some() {
        if !options.files.is_empty() {
            return Err(bad("--import reads a netlist and takes no board"));
        }
    } else if options.files.len() != 1 {
        return Err(bad("give exactly one board file"));
    }
    if options.lint && (options.compare.is_some() || options.csv.is_some()) {
        return Err(bad("--lint is not given with --compare or --csv"));
    }
    let writes = options.out.is_some() || options.csv.is_some();
    let reads_whole = options.lint || options.compare.is_some();
    if options.net.is_some() && reads_whole {
        return Err(bad("--net is not given with --lint or --compare"));
    }
    if options.part.is_some() && (reads_whole || writes || options.net.is_some()) {
        return Err(bad(
            "--part is not given with another way to use the command",
        ));
    }
    if options.top.is_some()
        && (reads_whole || writes || options.net.is_some() || options.part.is_some())
    {
        return Err(bad(
            "--top is not given with another way to use the command",
        ));
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<NetsSettings, NetsError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(NetsSettings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| NetsError::new(INPUT_UNREADABLE, format!("{path}: {e}")))?;
            NetsSettings::parse(&text)
        }
    }
}

/// Reads a netlist file as text; a file larger than the limit of the settings is refused after
/// at most one byte more than the limit was read.
fn read_netlist(path: &str, settings: &NetsSettings) -> Result<String, NetsError> {
    let unreadable = |what: String| NetsError::new(INPUT_UNREADABLE, format!("{path}: {what}"));
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(settings.max_input_bytes + 1).read_to_end(&mut bytes))
        .map_err(|e| unreadable(e.to_string()))?;
    if bytes.len() as u64 > settings.max_input_bytes {
        return Err(NetsError::new(
            LIMIT_EXCEEDED,
            format!(
                "{path}: the file is larger than max_input_bytes {}",
                settings.max_input_bytes
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|_| unreadable("the file is not UTF-8 text".to_owned()))
}

/// The nets that the command works on: those of the netlist file that `--import` names, or else
/// those of the board, with the notes about what could not be written as it is.
fn load_side(
    options: &Options,
    settings: &NetsSettings,
) -> Result<(NetTable, Vec<Note>), NetsError> {
    match (&options.import, options.files.as_slice()) {
        (Some(path), _) => Ok((parse(&read_netlist(path, settings)?, settings)?, Vec::new())),
        (None, [board_path]) => {
            let board =
                read_board_file(board_path).map_err(|e| NetsError::new(INPUT_UNREADABLE, e))?;
            Ok(board_table(&board))
        }
        _ => Err(NetsError::new(
            OPTION_INVALID,
            "give exactly one board file",
        )),
    }
}

/// Refuses the run before anything is written when an output names a file that is already there
/// (whatever it is) or when `--out` and `--csv` name the same file.
fn check_outputs(options: &Options) -> Result<(), NetsError> {
    let paths: Vec<&String> = options.out.iter().chain(options.csv.iter()).collect();
    if let [a, b] = paths.as_slice() {
        if a == b {
            return Err(NetsError::new(
                OUTPUT_FAILED,
                "--out and --csv name the same file",
            ));
        }
    }
    for path in paths {
        if std::fs::symlink_metadata(path).is_ok() {
            return Err(NetsError::new(
                OUTPUT_FAILED,
                format!("{path}: it exists, an existing file is never replaced"),
            ));
        }
    }
    Ok(())
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), NetsError> {
    let failed = |e: std::io::Error| NetsError::new(OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

fn fail(err: &mut dyn Write, error: &NetsError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(column(lang)));
    2
}

fn table_json(table: &NetTable) -> serde_json::Value {
    let nets: Vec<serde_json::Value> = table
        .ordered()
        .into_iter()
        .map(|(name, pins)| {
            let pins: Vec<String> = pins.iter().map(ToString::to_string).collect();
            json!({ "name": name, "pins": pins })
        })
        .collect();
    json!({ "net_count": table.nets.len(), "pin_count": table.pin_count(), "nets": nets })
}

fn notes_json(notes: &[Note]) -> serde_json::Value {
    let notes: Vec<serde_json::Value> = notes
        .iter()
        .map(|n| json!({ "code": n.code, "detail": n.detail }))
        .collect();
    serde_json::Value::Array(notes)
}

fn comparison_json(file: &NetTable, board: &NetTable, result: &Comparison) -> serde_json::Value {
    let differences: Vec<serde_json::Value> = result
        .differences
        .iter()
        .map(|d| {
            json!({
                "code": d.code,
                "net": d.net,
                "pin": d.pin.as_ref().map(ToString::to_string),
                "other": d.other,
            })
        })
        .collect();
    json!({
        "file": { "net_count": file.nets.len(), "pin_count": file.pin_count() },
        "board": { "net_count": board.nets.len(), "pin_count": board.pin_count() },
        "difference_count": result.total,
        "listed": result.differences.len(),
        "differences": differences,
    })
}

fn lint_json(table: &NetTable, result: &LintResult) -> serde_json::Value {
    let findings: Vec<serde_json::Value> = result
        .findings
        .iter()
        .map(|f| json!({ "code": f.code, "net": f.net, "other": f.other }))
        .collect();
    json!({
        "net_count": table.nets.len(),
        "pin_count": table.pin_count(),
        "finding_count": result.total,
        "listed": result.findings.len(),
        "findings": findings,
    })
}

fn pretty(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

/// The netlist of a table with the notes as comment lines after it.
fn text_with_notes(table: &NetTable, notes: &[Note]) -> String {
    let mut text = render(table);
    for note in notes {
        text.push_str(&format!("# {}: {}\n", note.code, note.detail));
    }
    text
}

fn summary(table: &NetTable) -> String {
    format!("{} nets, {} pins", table.nets.len(), table.pin_count())
}

fn export_or_import(options: &Options, out: &mut dyn Write) -> Result<i32, NetsError> {
    let settings = load_settings(options.config.as_deref())?;
    let (table, notes) = load_side(options, &settings)?;
    if options.lint {
        return lint_table(options, &settings, &table, &notes, out);
    }
    if let Some(part) = &options.part {
        return part_report(options, &table, part, out);
    }
    if let Some(top) = options.top {
        return top_report(options, &table, &notes, top, out);
    }
    let table = match &options.net {
        Some(patterns) => filter_nets(&table, patterns),
        None => table,
    };
    let text = text_with_notes(&table, &notes);
    let mut report = table_json(&table);
    report["notes"] = notes_json(&notes);
    check_outputs(options)?;
    if let Some(path) = &options.out {
        write_new(path, &text)?;
    }
    if let Some(path) = &options.csv {
        write_new(path, &pins_csv(&table, settings.csv_guard_formulas))?;
    }
    let shown = if options.json {
        pretty(&report)
    } else if options.out.is_some() || options.csv.is_some() {
        let mut shown = String::new();
        if let Some(path) = &options.out {
            shown.push_str(&format!("written: {path} ({})\n", summary(&table)));
        }
        if let Some(path) = &options.csv {
            shown.push_str(&format!("written: {path} ({} rows)\n", table.pin_count()));
        }
        shown
    } else {
        text
    };
    let _ = out.write_all(shown.as_bytes());
    Ok(0)
}

/// The pins of one part with their nets (`--part`); exit code 1 when the part has no pin.
fn part_report(
    options: &Options,
    table: &NetTable,
    part: &str,
    out: &mut dyn Write,
) -> Result<i32, NetsError> {
    let pins = pins_of_part(table, part);
    let text = if options.json {
        let list: Vec<serde_json::Value> = pins
            .iter()
            .map(|(pin, net, count)| json!({ "pin": pin.to_string(), "net": net, "net_pins": count }))
            .collect();
        pretty(&json!({ "part": part, "pin_count": pins.len(), "pins": list }))
    } else if pins.is_empty() {
        format!("part {part}: no pin\n")
    } else {
        let mut text = format!("part {part}: {} pins\n", pins.len());
        for (pin, net, count) in &pins {
            text.push_str(&format!("{pin} {net} ({count} pins)\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(pins.is_empty()))
}

/// The size of the netlist and its largest nets (`--top`).
fn top_report(
    options: &Options,
    table: &NetTable,
    notes: &[Note],
    top: usize,
    out: &mut dyn Write,
) -> Result<i32, NetsError> {
    let sum = table_summary(table, top);
    let text = if options.json {
        let largest: Vec<serde_json::Value> = sum
            .largest
            .iter()
            .map(|(name, pins)| json!({ "name": name, "pins": pins }))
            .collect();
        let mut report = json!({
            "net_count": sum.nets,
            "pin_count": sum.pins,
            "part_count": sum.parts,
            "empty_nets": sum.empty,
            "single_pin_nets": sum.single_pin,
            "two_pin_nets": sum.two_pin,
            "largest": largest,
        });
        report["notes"] = notes_json(notes);
        pretty(&report)
    } else {
        let mut text = format!(
            "nets: {}\nparts: {}\nnets with no pin: {}, one pin: {}, two pins: {}\n",
            summary(table),
            sum.parts,
            sum.empty,
            sum.single_pin,
            sum.two_pin
        );
        for note in notes {
            text.push_str(&format!("note {}: {}\n", note.code, note.detail));
        }
        text.push_str(&format!("largest nets: {}\n", sum.largest.len()));
        for (name, pins) in &sum.largest {
            text.push_str(&format!("{name} {pins}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

fn lint_table(
    options: &Options,
    settings: &NetsSettings,
    table: &NetTable,
    notes: &[Note],
    out: &mut dyn Write,
) -> Result<i32, NetsError> {
    let result = lint(table, settings);
    let mut report = lint_json(table, &result);
    report["notes"] = notes_json(notes);
    check_outputs(options)?;
    if let Some(path) = &options.out {
        write_new(path, &pretty(&report))?;
    }
    let text = if options.json {
        pretty(&report)
    } else {
        let lang = column(options.lang);
        let mut text = format!("nets: {}\n", summary(table));
        for note in notes {
            text.push_str(&format!("note {}: {}\n", note.code, note.detail));
        }
        for f in &result.findings {
            text.push_str(&format!(
                "{}: {}: {}",
                f.code,
                nets_codes::title(f.code, lang),
                f.net
            ));
            if let Some(other) = &f.other {
                text.push_str(&format!(" ~ {other}"));
            }
            text.push('\n');
        }
        if result.total > result.findings.len() {
            text.push_str(&format!(
                "{} more findings are not listed (max_differences)\n",
                result.total - result.findings.len()
            ));
        }
        text.push_str(&format!("findings: {}\n", result.total));
        if let Some(path) = &options.out {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(result.total > 0))
}

fn compare_with_board(
    options: &Options,
    file_path: &str,
    out: &mut dyn Write,
) -> Result<i32, NetsError> {
    let settings = load_settings(options.config.as_deref())?;
    let file = parse(&read_netlist(file_path, &settings)?, &settings)?;
    // the other side is the board, or a second netlist file when --import names one
    let (board_table, notes) = load_side(options, &settings)?;
    let other_label = if options.import.is_some() {
        "second netlist"
    } else {
        "board"
    };
    let result = compare(&file, &board_table, &settings);
    let mut report = comparison_json(&file, &board_table, &result);
    report["notes"] = notes_json(&notes);
    check_outputs(options)?;
    if let Some(path) = &options.out {
        write_new(path, &pretty(&report))?;
    }
    if let Some(path) = &options.csv {
        write_new(
            path,
            &differences_csv(&result.differences, settings.csv_guard_formulas),
        )?;
    }
    let text = if options.json {
        pretty(&report)
    } else {
        let lang = column(options.lang);
        let mut text = format!(
            "netlist: {}\n{other_label}: {}\n",
            summary(&file),
            summary(&board_table)
        );
        for note in &notes {
            text.push_str(&format!("note {}: {}\n", note.code, note.detail));
        }
        for d in &result.differences {
            text.push_str(&format!(
                "{}: {}: {}",
                d.code,
                nets_codes::title(d.code, lang),
                d.net
            ));
            if let Some(pin) = &d.pin {
                text.push_str(&format!(" {pin}"));
            }
            if let Some(other) = &d.other {
                text.push_str(&format!(" -> {other}"));
            }
            text.push('\n');
        }
        if result.total > result.differences.len() {
            text.push_str(&format!(
                "{} more differences are not listed (max_differences)\n",
                result.total - result.differences.len()
            ));
        }
        text.push_str(&format!("differences: {}\n", result.total));
        for path in options.out.iter().chain(options.csv.iter()) {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    Ok(i32::from(result.total > 0))
}

pub(crate) fn nets_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    let done = match options.compare.clone() {
        Some(file_path) => compare_with_board(&options, &file_path, out),
        None => export_or_import(&options, out),
    };
    match done {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}
