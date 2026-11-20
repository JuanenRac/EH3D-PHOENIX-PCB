// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `check` and `connectivity`: read EAGLE files (a KiCad board too, for `check`), run the
//! rule engine or the connectivity analysis, and print the report as text, JSON or, for `check`, as
//! SARIF, with a table of CSV and a page of HTML written on request. `check` also lists, as
//! information, a polygon over `max_polygon_points`.
//!
//! Exit codes: 0 nothing is proven wrong, 1 at least one error is proven, 2 the arguments or the
//! input are unusable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use eh3d_phoenix_eagle_board::{read_board_with_limits, read_schematic_with_limits};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, Nm, ObjectId, StepClock};
use eh3d_phoenix_rules::{
    board_notes, builtin_names, builtin_profile, check_advanced, check_board, check_classes,
    check_connectivity, compare_profiles, dfm_findings, report_csv, report_html, to_sarif,
    AdvancedConfig, DfmProfile, Lang, NetClasses, ProfileRun, Report, RuleFile, RuleValues,
};

/// Said when a KiCad board is checked without a file of limits.
const KICAD_NEEDS_RULES: &str =
    "a KiCad board gives no design rules to this command; give the limits with --rules FILE (a JSON rule file)";

/// The largest input file that is read, in bytes.
const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix check --ipc2581 FILE [--json] [--lang CODE] [--html FILE] [--stackup-out FILE] [--overwrite] [--config FILE]
phoenix check [--rules FILE | --rules-dru FILE | --dfm FILE | --dfm-profile NAME | --dfm-compare LIST] [--advanced] [--drc-config FILE] [--limits FILE] [--json | --sarif] [--csv FILE] [--html FILE] [--time] [--lang CODE] <board.brd>

  check         with --ipc2581 FILE: read an IPC-2581 file, or a ZIP archive that holds one, and report its
                revision, unit, what the reader does not know and the counts (the reading mode of
                `phoenix ipc2581`, same exit codes). Otherwise run the design rules on an EAGLE board. The limits come from FILE (a JSON rule
                file, format phoenix-rules/1 or phoenix-rules/2, which adds net classes with
                limits of their own for tracks and vias) or, without --rules, from the
                <designrules> of the board. The report keeps confirmed and inconclusive findings
                in two lists. --advanced adds the further checks (annular ring, distance to the board
                edge, silkscreen over holes, acid traps, spacing by voltage, copper islands, stubs,
                duplicate vias, pads on no net and parts outside the outline, codes PHOENIX-DRC-nnn),
                with the levels and limits of config/drc_advanced.cfg or of the file --drc-config
                names (which implies --advanced); the spacing table is not verified against the
                standard. --sarif prints a layout like SARIF, phoenix-drc/1

phoenix connectivity [--rules FILE] [--limits FILE] [--json] [--csv FILE] [--html FILE] [--time] [--lang CODE] <schematic.sch> <board.brd>

  connectivity  the copper of the board (shorts, unrouted nets) and the board against the schematic
                (missing nets, unconnected pins, joined nets, names). The tolerance for copper
                that touches comes from the rule file

  --json        print the report as JSON (--sarif: the layout phoenix-drc/1). --lang CODE writes the text in en, es, de, fr, it, ja,
                zh or pt. Exit code 0: nothing proven wrong; 1: an error is proven; 2: unusable input
  --dfm FILE    check the board against the capability profile of a board maker (a text file, see
                docs/DFM.md and config/dfm/example.cfg): its limits replace the rules of the board
                and it also judges the copper layers, the size of the board, the largest drill and
                the number of holes. The example profile is not verified against any maker
  --dfm-profile NAME
                the same with a profile that comes with the program: jlcpcb, pcbway or example
                (the files config/dfm_jlcpcb.cfg, config/dfm_pcbway.cfg and config/dfm/example.cfg,
                whose figures are public guiding numbers of the makers and stay unverified); it
                cannot be given with --dfm, --rules or --rules-dru
  --dfm-compare LIST
                check the board against several capability profiles and compare them: LIST is
                profiles separated by commas, each the name of a profile that comes with the
                program or a profile file (a name with a dot or a slash is a file). It prints, for
                each profile, the errors, warnings and information and whether the board passes,
                then how often each code is found with each profile, the codes only some profiles
                find and the profiles with the fewest errors (--json: phoenix-dfm-compare/1). The
                figures of a profile are what its file says (verified = false for those that come
                with the program), so the comparison is no recommendation of a maker. Exit code 0
                when at least one profile accepts the board, 1 when none does, 2 for unusable
                input. It cannot be given with --rules, --rules-dru, --dfm, --dfm-profile,
                --advanced, --sarif, --csv or --html
  --limits FILE the limits of the reader of the EAGLE files (config/eagle_board.cfg), of the
                checks of a board (config/board.cfg) and the cut of the arcs of the outline
                (config/outline_arcs.cfg), as for every command; without it the defaults
                are used and the report is the same. check tells, as information that never
                changes the exit code, each polygon of the board with more points than
                max_polygon_points (code PHOENIX-BOARD-017): its edges are not tested for crossing
  --csv FILE    also write the findings as a table of CSV, one row for each, with the stable keys of
                the category and the severity and the title in the language of --lang
  --html FILE   also write the report as a page of HTML, grouped by category, with the headings in
                the language of --lang. Both write a new file: a file that exists is never replaced
  --time        also write the time the command took, in seconds, as one line, time: S.SSS s, to
                the error stream (the report on the output is the same with and without it)
";

struct Options {
    json: bool,
    sarif: bool,
    time: bool,
    advanced: bool,
    drc_config: Option<String>,
    dfm: Option<String>,
    dfm_profile: Option<String>,
    dfm_compare: Option<String>,
    csv: Option<String>,
    html: Option<String>,
    lang: Lang,
    rules: Option<String>,
    rules_dru: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        json: false,
        sarif: false,
        time: false,
        advanced: false,
        drc_config: None,
        dfm: None,
        dfm_profile: None,
        dfm_compare: None,
        csv: None,
        html: None,
        lang: Lang::En,
        rules: None,
        rules_dru: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--json" => options.json = true,
            "--sarif" => options.sarif = true,
            "--time" => options.time = true,
            "--advanced" => options.advanced = true,
            "--drc-config" => {
                let path = iter.next().ok_or("--drc-config needs a file")?;
                options.drc_config = Some((*path).to_owned());
                options.advanced = true;
            }
            "--dfm" => {
                let path = iter.next().ok_or("--dfm needs a profile file")?;
                options.dfm = Some((*path).to_owned());
            }
            "--dfm-profile" => {
                let name = iter
                    .next()
                    .ok_or("--dfm-profile needs the name of a profile")?;
                options.dfm_profile = Some((*name).to_owned());
            }
            "--dfm-compare" => {
                let list = iter
                    .next()
                    .ok_or("--dfm-compare needs a list of profiles separated by commas")?;
                options.dfm_compare = Some((*list).to_owned());
            }
            "--csv" => {
                let path = iter.next().ok_or("--csv needs a file to write")?;
                options.csv = Some((*path).to_owned());
            }
            "--html" => {
                let path = iter.next().ok_or("--html needs a file to write")?;
                options.html = Some((*path).to_owned());
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            "--rules" => {
                let path = iter.next().ok_or("--rules needs a file")?;
                options.rules = Some((*path).to_owned());
            }
            "--rules-dru" => {
                let path = iter.next().ok_or("--rules-dru needs a file")?;
                options.rules_dru = Some((*path).to_owned());
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

pub(crate) fn read_text(path: &str) -> Result<String, String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("cannot read {path}: {e}"))?
        .len();
    if size > MAX_INPUT_BYTES {
        return Err(format!(
            "{path} is {size} bytes, the limit is {MAX_INPUT_BYTES}"
        ));
    }
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

/// Ids for the objects read from a file: the same file always gives the same ids.
pub(crate) fn id_source() -> Result<impl FnMut() -> ObjectId, String> {
    let mut generator = IdGenerator::new(
        StepClock::new(1_700_000_000_000, 1),
        DeterministicRandom::new(1),
    );
    let mut last = generator.generate().map_err(|e| e.to_string())?;
    Ok(move || {
        if let Ok(next) = generator.generate() {
            last = next;
        }
        last
    })
}

/// Writes each output as a new file; nothing is written when one of them exists or two are named
/// alike, and an existing file is never replaced.
fn write_new_files(files: &[(&str, String)]) -> Result<(), String> {
    for (i, (path, _)) in files.iter().enumerate() {
        if std::path::Path::new(path).exists() {
            return Err(format!("{path} exists and is never replaced"));
        }
        if files[..i].iter().any(|(other, _)| other == path) {
            return Err(format!("{path} is named for two outputs"));
        }
    }
    for (path, text) in files {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("{path}: {e}"))?;
        file.write_all(text.as_bytes())
            .map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(())
}

fn finish(
    report: &Report,
    options: &Options,
    started: Instant,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let mut outputs: Vec<(&str, String)> = Vec::new();
    if let Some(path) = &options.csv {
        outputs.push((path.as_str(), report_csv(report, options.lang)));
    }
    if let Some(path) = &options.html {
        outputs.push((path.as_str(), report_html(report, options.lang)));
    }
    if let Err(message) = write_new_files(&outputs) {
        let _ = writeln!(err, "error: {message}");
        return 2;
    }
    let text = if options.sarif {
        to_sarif(report, options.lang)
    } else if options.json {
        report.to_json()
    } else {
        report.to_text(options.lang)
    };
    let _ = out.write_all(text.as_bytes());
    if options.time {
        let _ = writeln!(err, "time: {:.3} s", started.elapsed().as_secs_f64());
    }
    if report.passes() {
        0
    } else {
        1
    }
}

fn usage(err: &mut dyn Write) -> i32 {
    let _ = err.write_all(USAGE.as_bytes());
    2
}

fn fail(err: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(err, "error: {message}");
    2
}

/// `phoenix check --ipc2581 FILE`: the file is an IPC-2581 file (or a ZIP archive that holds one)
/// and not a board; the command is the reading mode of `phoenix ipc2581`, which reports the
/// revision, the unit, what the reader does not know and the counts of the file.
fn check_ipc2581(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut file: Option<&str> = None;
    let mut forwarded: Vec<&str> = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--ipc2581" => match iter.next() {
                Some(path) => file = Some(*path),
                None => {
                    let _ = writeln!(err, "error: --ipc2581 needs a file");
                    return usage(err);
                }
            },
            "--json" | "--overwrite" => forwarded.push(*arg),
            "--lang" | "--html" | "--stackup-out" | "--config" => {
                forwarded.push(*arg);
                match iter.next() {
                    Some(value) => forwarded.push(*value),
                    None => {
                        let _ = writeln!(err, "error: {arg} needs a value");
                        return usage(err);
                    }
                }
            }
            other => {
                let _ = writeln!(err, "error: --ipc2581 cannot be given with {other}");
                return usage(err);
            }
        }
    }
    let Some(file) = file else {
        return usage(err);
    };
    let mut full: Vec<&str> = vec!["--read", file];
    full.extend(forwarded);
    crate::ipc2581::ipc2581_command(&full, out, err)
}

pub(crate) fn check(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.contains(&"--ipc2581") {
        return check_ipc2581(args, out, err);
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return usage(err);
        }
    };
    let started = Instant::now();
    let [board_path] = options.files.as_slice() else {
        return usage(err);
    };
    // a KiCad board is read by the KiCad reader; it carries no design rules the tool reads, so the
    // limits must come from --rules
    let kicad = board_path.to_ascii_lowercase().ends_with(".kicad_pcb");
    let text = if kicad {
        String::new()
    } else {
        match read_text(board_path) {
            Ok(text) => text,
            Err(message) => return fail(err, &message),
        }
    };
    if let Some(list) = &options.dfm_compare {
        return compare_dfm(list, &options, board_path, kicad, &text, out, err);
    }
    if options.dfm.is_some() && options.dfm_profile.is_some() {
        return fail(err, "give --dfm or --dfm-profile, only one of them");
    }
    let dfm = match (&options.dfm, &options.dfm_profile) {
        (None, Some(name)) => {
            if options.rules.is_some() || options.rules_dru.is_some() {
                return fail(
                    err,
                    "give --dfm-profile or --rules or --rules-dru, only one of them",
                );
            }
            let Some(text) = builtin_profile(name) else {
                return fail(
                    err,
                    &format!(
                        "no profile is called {name} (the profiles that come with the program: {})",
                        builtin_names()
                    ),
                );
            };
            match DfmProfile::parse(text) {
                Ok(profile) => Some(profile),
                Err(e) => return fail(err, &format!("{name}: {e}")),
            }
        }
        (Some(path), _) => {
            if options.rules.is_some() || options.rules_dru.is_some() {
                return fail(
                    err,
                    "give --dfm or --rules or --rules-dru, only one of them",
                );
            }
            match read_text(path)
                .and_then(|t| DfmProfile::parse(&t).map_err(|e| format!("{path}: {e}")))
            {
                Ok(profile) => Some(profile),
                Err(message) => return fail(err, &message),
            }
        }
        (None, None) => None,
    };
    let (rules, classes) = if let Some(profile) = &dfm {
        (profile.rules, NetClasses::default())
    } else {
        match (&options.rules, &options.rules_dru) {
            (Some(_), Some(_)) => return fail(err, "give --rules or --rules-dru, not both"),
            (Some(path), None) => {
                match read_text(path)
                    .and_then(|t| RuleFile::from_json(&t).map_err(|e| e.to_string()))
                {
                    Ok(file) => (file.values, file.classes),
                    Err(message) => return fail(err, &message),
                }
            }
            (None, Some(path)) => match crate::eagle_deep::dru_values(path, options.lang) {
                Ok((values, notes)) => {
                    for note in notes {
                        let _ = writeln!(err, "note: {note}");
                    }
                    (values, NetClasses::default())
                }
                Err(message) => return fail(err, &message),
            },
            (None, None) if kicad => return fail(err, KICAD_NEEDS_RULES),
            (None, None) => match RuleValues::from_eagle_board(&text) {
                Ok((values, notes)) => {
                    for note in notes {
                        let _ = writeln!(err, "note: {}", note.message);
                    }
                    (values, NetClasses::default())
                }
                Err(error) => {
                    return fail(
                        err,
                        &format!("{error}; give the limits with --rules FILE (a JSON rule file)"),
                    )
                }
            },
        }
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let board = if kicad {
        match crate::kicad_import::read_kicad_board(board_path, &mut new_id) {
            Ok(board) => board,
            Err(message) => return fail(err, &message),
        }
    } else {
        let read = match read_board_with_limits(&text, &mut new_id, &crate::limits::active().eagle)
        {
            Ok(read) => read,
            Err(diagnostic) => return fail(err, &diagnostic.message),
        };
        if !read.diagnostics.is_empty() {
            let _ = writeln!(
                err,
                "note: {} parts of the board file were not read or were read in a simpler form",
                read.diagnostics.len()
            );
        }
        read.board
    };
    let mut report = check_board(&board, &rules);
    if !classes.is_empty() {
        report.merge(check_classes(&board, &classes));
    }
    if let Some(profile) = &dfm {
        report.merge(dfm_findings(&board, profile));
    }
    if options.advanced {
        let config = match &options.drc_config {
            Some(path) => match read_text(path)
                .and_then(|t| AdvancedConfig::parse(&t).map_err(|e| e.to_string()))
            {
                Ok(config) => config,
                Err(message) => return fail(err, &message),
            },
            None => AdvancedConfig::default(),
        };
        report.merge(check_advanced(&board, &config));
    }
    // a polygon over max_polygon_points is told as information; nothing else changes
    report.merge(board_notes(&board, &crate::limits::active().board));
    finish(&report, &options, started, out, err)
}

/// The profile that one entry of `--dfm-compare` names: a profile that comes with the program, or
/// a profile file when the entry has a dot or a slash. The label is the name of the profile as it
/// was written, or the name of the file without its folder.
fn read_profile(entry: &str) -> Result<(String, DfmProfile), String> {
    if entry.contains(['.', '/', '\\']) {
        let text = read_text(entry)?;
        let profile = DfmProfile::parse(&text).map_err(|e| format!("{entry}: {e}"))?;
        let label = Path::new(entry).file_name().map_or_else(
            || entry.to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        return Ok((label, profile));
    }
    let Some(text) = builtin_profile(entry) else {
        return Err(format!(
            "no profile is called {entry} (the profiles that come with the program: {})",
            builtin_names()
        ));
    };
    let profile = DfmProfile::parse(text).map_err(|e| format!("{entry}: {e}"))?;
    Ok((entry.to_owned(), profile))
}

/// `--dfm-compare`: the same board checked with the limits of each profile and the results put
/// side by side. The profile gives the limits of the usual rule check and its own lines, as with
/// `--dfm`; nothing else is judged.
fn compare_dfm(
    list: &str,
    options: &Options,
    board_path: &str,
    kicad: bool,
    text: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if options.dfm.is_some()
        || options.dfm_profile.is_some()
        || options.rules.is_some()
        || options.rules_dru.is_some()
        || options.advanced
        || options.sarif
        || options.csv.is_some()
        || options.html.is_some()
    {
        return fail(
            err,
            "--dfm-compare cannot be given with --rules, --rules-dru, --dfm, --dfm-profile, --advanced, --sarif, --csv or --html",
        );
    }
    let entries: Vec<&str> = list.split(',').map(str::trim).collect();
    if entries.iter().any(|e| e.is_empty()) || entries.len() < 2 || entries.len() > 8 {
        return fail(
            err,
            "--dfm-compare needs from 2 to 8 profiles separated by commas",
        );
    }
    let mut profiles: Vec<(String, DfmProfile)> = Vec::new();
    for entry in &entries {
        match read_profile(entry) {
            Ok((label, profile)) => {
                if profiles.iter().any(|(known, _)| *known == label) {
                    return fail(err, &format!("the profile {label} is given twice"));
                }
                profiles.push((label, profile));
            }
            Err(message) => return fail(err, &message),
        }
    }
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let board = if kicad {
        match crate::kicad_import::read_kicad_board(board_path, &mut new_id) {
            Ok(board) => board,
            Err(message) => return fail(err, &message),
        }
    } else {
        match read_board_with_limits(text, &mut new_id, &crate::limits::active().eagle) {
            Ok(read) => read.board,
            Err(diagnostic) => return fail(err, &diagnostic.message),
        }
    };
    let runs: Vec<ProfileRun> = profiles
        .iter()
        .map(|(label, profile)| {
            let mut report = check_board(&board, &profile.rules);
            report.merge(dfm_findings(&board, profile));
            ProfileRun {
                label: label.clone(),
                verified: profile.verified,
                report,
            }
        })
        .collect();
    let comparison = compare_profiles(&runs);
    let shown = if options.json {
        comparison.to_json()
    } else {
        comparison.to_text(options.lang)
    };
    let _ = out.write_all(shown.as_bytes());
    i32::from(!comparison.any_passes())
}

pub(crate) fn connectivity(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return usage(err);
        }
    };
    let started = Instant::now();
    if options.dfm.is_some() || options.dfm_profile.is_some() || options.dfm_compare.is_some() {
        return fail(
            err,
            "--dfm, --dfm-profile and --dfm-compare belong to the command check, not to connectivity",
        );
    }
    if options.advanced {
        return fail(
            err,
            "--advanced and --drc-config belong to the command check, not to connectivity",
        );
    }
    if options.rules_dru.is_some() {
        return fail(
            err,
            "--rules-dru belongs to the command check, not to connectivity",
        );
    }
    let [schematic_path, board_path] = options.files.as_slice() else {
        return usage(err);
    };
    let tolerance = match &options.rules {
        Some(path) => match read_text(path)
            .and_then(|t| RuleValues::from_json(&t).map_err(|e| e.to_string()))
        {
            Ok((_, values)) => values.connect_tolerance,
            Err(message) => return fail(err, &message),
        },
        None => Nm::ZERO,
    };
    let (schematic_text, board_text) = match (read_text(schematic_path), read_text(board_path)) {
        (Ok(s), Ok(b)) => (s, b),
        (Err(message), _) | (_, Err(message)) => return fail(err, &message),
    };
    let mut new_id = match id_source() {
        Ok(source) => source,
        Err(message) => return fail(err, &message),
    };
    let board =
        match read_board_with_limits(&board_text, &mut new_id, &crate::limits::active().eagle) {
            Ok(read) => read.board,
            Err(diagnostic) => return fail(err, &diagnostic.message),
        };
    let footprints: BTreeMap<String, ObjectId> = board
        .footprints
        .iter()
        .map(|(id, footprint)| (footprint.name.clone(), *id))
        .collect();
    let schematic = match read_schematic_with_limits(
        &schematic_text,
        &footprints,
        &mut new_id,
        &crate::limits::active().eagle,
    ) {
        Ok(read) => read.schematic,
        Err(diagnostic) => return fail(err, &diagnostic.message),
    };
    let report = check_connectivity(&schematic, &board, tolerance);
    finish(&report, &options, started, out, err)
}
