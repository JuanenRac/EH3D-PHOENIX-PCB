// SPDX-License-Identifier: GPL-3.0-or-later
//! The command line of EH3D-PHOENIX-PCB. It has fifteen commands, `inventory`, `eagle-rate`,
//! `stackup`, `check`, `connectivity`, `erc`, `sync`, `export`, `rf`, `kicad-read`, `sanitize`, `sim`,
//! `route`, `ratsnest` and `view`.
//!
//! Exit codes: 0 done, 1 `stackup` could not compute one or more of the lines it was asked for,
//! `check`, `connectivity` and `erc` proved an error, or `sync` found changes to make, 2 the
//! arguments or the input are unusable.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod check;
mod design;
mod export;
mod kicad;
mod rf;
mod route;
mod sanitize;
mod sim;
mod view;

use eh3d_phoenix_corpus::{inventory, InventoryConfig};
use eh3d_phoenix_stackup::{report_json, report_text, Language, StackupJob, StackupSettings};
use std::io::Write;
use std::path::Path;

const USAGE: &str = "\
phoenix [--config FILE] inventory [--json] <folder>

  inventory   count the design files of a private corpus without showing any name or value:
              files by extension, kind and declared version, and one fingerprint of the content

phoenix eagle-rate <folder of originals> <new folder for the copies>

  eagle-rate  copy the .lbr, .brd and .sch files of a folder (the originals are only read, each copy
              is checked by SHA-256) and measure how many the EAGLE reader reads: counts, versions
              and diagnostic codes only, never a name or a value

phoenix stackup <job.json> --freq-ghz <GHz> [--settings FILE] [--lang xx] [--json]

  stackup     compute the transmission lines of a stack-up (microstrip, stripline, coplanar
              waveguide, coupled pairs): impedance, effective permittivity, losses, delay and
              guided wavelength at a frequency, or the track width for a target impedance; the
              job file holds the layers, the materials and the lines; a prototype estimate, not
              a simulation; --lang is one of en, es, de, fr, it, ja, zh, pt

phoenix check [--rules FILE] [--json] [--lang xx] <board.brd>
phoenix connectivity [--rules FILE] [--json] [--lang xx] <schematic.sch> <board.brd>

  check         run the design rules of FILE (JSON, phoenix-rules/1) or of the board on an EAGLE board
  connectivity  shorts and unrouted nets of the board, and the board against the schematic
                (exit 1 when an error is proven; the report keeps confirmed and inconclusive apart)

phoenix erc [--config FILE] [--json] [--lang xx] <schematic> [<board>]
phoenix sync [--config FILE] [--json | --package] [--lang xx] [--backward] [--apply --out FILE] <schematic> <board>

  erc           the electrical rule check of a schematic (pins on no net, pin types by a matrix in a
                settings file, inputs without a driver, supplies without a source, net names that
                look alike, references used twice, empty values, parts without a footprint, loose
                wires); the report keeps confirmed and inconclusive findings apart
  sync          what the transfer of the schematic to the board (or, with --backward, the annotation
                back into the schematic) would change, by the identity of the parts; a dry run
                unless --apply writes the result to --out, and an input file is never overwritten

phoenix export [--profile NAME] [--profiles FILE] [--lang xx] [--eh3dproj FILE] --out <folder> <board>

  export        the manufacturing files of a board (Gerber X2, plated and not plated drills,
                placement, bill of materials) and manifest.json with their SHA-256, made with a
                profile of a JSON file and checked against the board before they are written
                (exit 1 when they differ)

phoenix rf touchstone|match|filter ...

  rf            everyday RF tools: Touchstone files, impedance matching and ladder filters
                (run `phoenix rf` alone for its usage)

phoenix kicad-read [--settings FILE] [--lang xx] [--redacted] <file>

  kicad-read    read a KiCad 6 to 9 file (schematic with its sheets, board, symbol library,
                footprint or project) into the design model and summarise it: what was kept,
                changed and left out, the findings with their codes and the fidelity level

phoenix sanitize [--out BASE] [--threshold N] [--config FILE] [--lang xx] [--json] <folder>

  sanitize      find the footprints, symbols and device sets that are duplicated or nearly
                duplicated in a folder of EAGLE libraries: an inventory, a canonical geometric
                hash, a score against a threshold, groups to confirm by hand, an HTML and a JSON
                report and a plan of the master library that is never applied; the libraries
                are only read

phoenix sim <netlist or schematic.json> --op | --ac KIND N F1 F2 | --tran STEP STOP [options]
phoenix sim --check-filter FAMILY ORDER BAND F1 [F2] [options]

  sim           simulate a circuit with the solver of this program (modified nodal analysis, a
                prototype never compared with SPICE): the DC operating point, an AC sweep with
                group delay or a transient analysis of a SPICE-style netlist or of a schematic,
                as CSV or JSON; run `phoenix sim` alone for its options

phoenix ratsnest [--settings FILE] [--json] [--lang xx] <board>
phoenix route [--settings FILE] [--rules FILE] [--layer NAME] [--mode around|stop] [--json]
              [--lang xx] --from ELEMENT --to ELEMENT [--out FILE] <board>

  ratsnest      the connections still to be routed on a board, net by net
  route         one track (and the vias it needs) between two elements of a net, at 0, 45 and 90
                degrees, keeping the clearances of the rules; writes the board with the track to
                a new file given with --out (an existing file is never replaced); a prototype,
                run `phoenix route` alone for its options

phoenix view [--settings FILE] [--lang xx] [<board or schematic>]
phoenix view --summary <file>  |  phoenix view --bench

  view          a read-only viewer window of a board or a schematic (a build with the gui
                feature; a prototype), a text summary of a file, or a timing of the preparation
                and the selection on three synthetic boards
";

/// Run the command line with the arguments that follow the program name. Returns the exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut args = args.to_vec();
    let mut config = InventoryConfig::default();
    if args.first().map(String::as_str) == Some("--config") {
        if args.len() < 3 {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
        let parsed = std::fs::read_to_string(&args[1])
            .map_err(|e| e.to_string())
            .and_then(|text| InventoryConfig::parse(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(parsed) => config = parsed,
            Err(message) => {
                let _ = writeln!(err, "error: cannot use the configuration: {message}");
                return 2;
            }
        }
        args.drain(..2);
    }
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    if let Some((&"stackup", tail)) = rest.split_first() {
        return stackup_command(tail, out, err);
    }
    if let Some((&"check", tail)) = rest.split_first() {
        return check::check(tail, out, err);
    }
    if let Some((&"connectivity", tail)) = rest.split_first() {
        return check::connectivity(tail, out, err);
    }
    if let Some((&"erc", tail)) = rest.split_first() {
        return design::erc(tail, out, err);
    }
    if let Some((&"sync", tail)) = rest.split_first() {
        return design::sync(tail, out, err);
    }
    if let Some((&"export", tail)) = rest.split_first() {
        return export::export(tail, out, err);
    }
    if let Some((&"rf", tail)) = rest.split_first() {
        return rf::rf(tail, out, err);
    }
    if let Some((&"kicad-read", tail)) = rest.split_first() {
        return kicad::kicad_read(tail, out, err);
    }
    if let Some((&"sanitize", tail)) = rest.split_first() {
        return sanitize::sanitize_command(tail, out, err);
    }
    if let Some((&"route", tail)) = rest.split_first() {
        return route::route_command(tail, out, err);
    }
    if let Some((&"ratsnest", tail)) = rest.split_first() {
        return route::ratsnest_command(tail, out, err);
    }
    if let Some((&"sim", tail)) = rest.split_first() {
        return sim::sim_command(tail, out, err);
    }
    if let Some((&"view", tail)) = rest.split_first() {
        return view::view(tail, out, err);
    }
    if let ["eagle-rate", source, copies] = rest.as_slice() {
        return eagle_rate(Path::new(source), Path::new(copies), out, err);
    }
    let (json, folder) = match rest.as_slice() {
        ["inventory", "--json", folder] => (true, *folder),
        ["inventory", folder] if *folder != "--json" => (false, *folder),
        _ => {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    match inventory(Path::new(folder), &config) {
        Ok(report) => {
            let text = if json {
                report.to_json() + "\n"
            } else {
                report.to_text()
            };
            let _ = out.write_all(text.as_bytes());
            0
        }
        Err(error) => {
            // the message names a path the caller gave or one below it: the caller already knows it
            let _ = writeln!(err, "error: {error}");
            2
        }
    }
}

/// The largest job or settings file that `stackup` reads, in bytes.
const MAX_STACKUP_FILE_BYTES: u64 = 4 * 1024 * 1024;

fn read_bounded(path: &str) -> Result<String, String> {
    let length = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if length > MAX_STACKUP_FILE_BYTES {
        return Err(format!(
            "the file is larger than {MAX_STACKUP_FILE_BYTES} bytes"
        ));
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

fn stackup_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut file: Option<&str> = None;
    let mut freq: Option<f64> = None;
    let mut settings_path: Option<&str> = None;
    let mut language = Language::En;
    let mut json = false;
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--json" => json = true,
            "--freq-ghz" => freq = iter.next().and_then(|v| v.parse::<f64>().ok()),
            "--settings" => settings_path = iter.next(),
            "--lang" => match iter.next().and_then(Language::from_tag) {
                Some(found) => language = found,
                None => {
                    let _ = err.write_all(USAGE.as_bytes());
                    return 2;
                }
            },
            other if !other.starts_with("--") && file.is_none() => file = Some(other),
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                return 2;
            }
        }
    }
    let (Some(file), Some(freq)) = (file, freq) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let settings = match settings_path {
        None => StackupSettings::default(),
        Some(path) => match read_bounded(path)
            .and_then(|text| StackupSettings::parse(&text).map_err(|e| e.to_string()))
        {
            Ok(settings) => settings,
            Err(message) => {
                let _ = writeln!(err, "error: cannot use the settings: {message}");
                return 2;
            }
        },
    };
    let job = match read_bounded(file) {
        Ok(text) => match StackupJob::parse(&text) {
            Ok(job) => job,
            Err(error) => {
                let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
                return 2;
            }
        },
        Err(message) => {
            let _ = writeln!(err, "error: cannot read the job file: {message}");
            return 2;
        }
    };
    let outcomes = job.run(freq, &settings);
    let text = if json {
        let mut json_text = report_json(&job.stackup.name, freq, &outcomes, language);
        json_text.push('\n');
        json_text
    } else {
        report_text(&job.stackup.name, freq, &outcomes, language)
    };
    let _ = out.write_all(text.as_bytes());
    if outcomes.iter().any(|o| o.result.is_err()) {
        1
    } else {
        0
    }
}

fn eagle_rate(source: &Path, copies: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    use eh3d_phoenix_eagle::{corpus, Limits};
    match corpus::snapshot(source, copies, &["lbr", "brd", "sch"]) {
        Ok((records, problems)) => {
            let rate = corpus::measure(&records, &Limits::default());
            let _ = out.write_all(rate.to_text().as_bytes());
            let _ = writeln!(out, "copy problems: {}", problems.len());
            0
        }
        Err(error) => {
            let _ = writeln!(err, "error: {error}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(list: &[&str]) -> (i32, String, String) {
        let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn the_view_command_is_reached_and_its_usage_errors_exit_2() {
        let (code, out, err) = call(&["view", "--summary"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.contains("--summary needs a file"), "{err}");
        let (code, out, _) = call(&["view", "--bench", "--settings", "/nonexistent/viewer.cfg"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
    }

    #[test]
    fn usage_errors_exit_2() {
        for list in [
            &[][..],
            &["inventory"],
            &["inventory", "--json"],
            &["nonsense", "x"],
            &["--config", "x"],
        ] {
            let (code, out, err) = call(list);
            assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
            assert!(err.contains("inventory"));
        }
    }

    #[test]
    fn inventory_prints_text_and_json_without_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("TopSecret.brd"),
            "<eagle version=\"9.6.2\"></eagle>",
        )
        .unwrap();
        let folder = dir.path().to_str().unwrap();
        let (code, text, _) = call(&["inventory", folder]);
        assert_eq!(code, 0);
        assert!(text.contains("files: 1") && text.contains("version 9.6.2: 1"));
        let (code, json, _) = call(&["inventory", "--json", folder]);
        assert_eq!(code, 0);
        assert!(json.contains("\"eagle-xml\""));
        assert!(!text.contains("TopSecret") && !json.contains("TopSecret"));
    }

    #[test]
    fn eagle_rate_copies_measures_and_shows_no_names() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("originals");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("TopSecret.brd"),
            "<eagle version=\"9.6.2\"><drawing><board><elements/><signals/></board></drawing></eagle>",
        )
        .unwrap();
        std::fs::write(src.join("Broken.sch"), "not xml").unwrap();
        let copies = dir.path().join("copies");
        let (code, text, _) = call(&[
            "eagle-rate",
            src.to_str().unwrap(),
            copies.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);
        assert!(text.contains("files: 2") && text.contains("version 9.6.2: 1"));
        assert!(text.contains("code PHOENIX-EAGLE-001: 1"));
        assert!(text.contains("copy problems: 0"));
        assert!(!text.contains("TopSecret") && !text.contains("Broken"));
        // a folder for the copies inside the originals is refused
        let (code, _, err) = call(&[
            "eagle-rate",
            src.to_str().unwrap(),
            src.join("in").to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("outside"));
    }

    #[test]
    fn a_missing_folder_and_a_bad_config_exit_2() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent");
        assert_eq!(call(&["inventory", missing.to_str().unwrap()]).0, 2);
        let cfg = dir.path().join("p.cfg");
        std::fs::write(&cfg, "max_files = lots\n").unwrap();
        let (code, _, err) = call(&[
            "--config",
            cfg.to_str().unwrap(),
            "inventory",
            dir.path().to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("configuration"));
    }

    #[test]
    fn the_config_file_changes_the_bounds() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..3 {
            std::fs::write(dir.path().join(format!("f{n}.txt")), "x").unwrap();
        }
        let cfg = dir.path().join("p.cfg");
        std::fs::write(&cfg, "max_files = 2\n").unwrap();
        let (code, _, err) = call(&[
            "--config",
            cfg.to_str().unwrap(),
            "inventory",
            dir.path().to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("number of files"));
    }

    /// A job with invented material values: one line with a width and one with a target.
    const STACKUP_JOB: &str = r#"{
        "stackup": {"name": "two-layer", "layers": [
            {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
            {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4-test"},
            {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
        "materials": [{"name": "fr4-test", "dk_points": [[1.0, 4.5], [10.0, 4.3]],
                       "df_points": [[1.0, 0.02], [10.0, 0.022]]}],
        "lines": [
            {"name": "fixed", "kind": "microstrip", "layer": "top", "width_nm": 3000000},
            {"name": "fifty", "kind": "microstrip", "layer": "top", "target_ohm": 50.0}]
    }"#;

    #[test]
    fn stackup_prints_the_lines_as_text_and_as_json() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("job.json");
        std::fs::write(&job, STACKUP_JOB).unwrap();
        let path = job.to_str().unwrap();
        let (code, text, err) = call(&["stackup", path, "--freq-ghz", "2"]);
        assert_eq!((code, err.is_empty()), (0, true), "{err}");
        assert!(text.contains("line fixed (microstrip)") && text.contains("impedance:"));
        assert!(text.contains("(found from the target impedance)"));
        let (code, json, _) = call(&["stackup", "--json", "--freq-ghz", "2", path]);
        assert_eq!(code, 0);
        assert!(json.starts_with('{') && json.ends_with("}\n"));
        assert!(json.contains("\"impedance_ohm\""));
        let (code, german, _) = call(&["stackup", path, "--freq-ghz", "20", "--lang", "de"]);
        assert_eq!(code, 1);
        assert!(german.contains("Frequenz außerhalb der veröffentlichten Punkte"));
    }

    #[test]
    fn stackup_reads_the_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("job.json");
        std::fs::write(&job, STACKUP_JOB).unwrap();
        let path = job.to_str().unwrap();
        let settings = dir.path().join("s.cfg");
        std::fs::write(&settings, "synth_max_iterations = 1\n").unwrap();
        let (code, text, _) = call(&[
            "stackup",
            path,
            "--freq-ghz",
            "2",
            "--settings",
            settings.to_str().unwrap(),
        ]);
        // one step is not enough to find the width of the second line
        assert_eq!(code, 1);
        assert!(text.contains("PHOENIX-STACK-008"));
        std::fs::write(&settings, "nonsense\n").unwrap();
        let (code, _, err) = call(&[
            "stackup",
            path,
            "--freq-ghz",
            "2",
            "--settings",
            settings.to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("settings"));
    }

    #[test]
    fn stackup_arguments_and_files_that_cannot_be_used_exit_2() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("job.json");
        std::fs::write(&job, STACKUP_JOB).unwrap();
        let path = job.to_str().unwrap();
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "not json").unwrap();
        let missing = dir.path().join("absent.json");
        let (bad_path, missing_path) = (bad.to_str().unwrap(), missing.to_str().unwrap());
        for list in [
            &["stackup"][..],
            &["stackup", path],
            &["stackup", path, "--freq-ghz"],
            &["stackup", path, "--freq-ghz", "fast"],
            &["stackup", path, "--freq-ghz", "2", "--lang", "xx"],
            &["stackup", path, "--freq-ghz", "2", "--nonsense"],
            &["stackup", "--freq-ghz", "2", bad_path],
            &["stackup", "--freq-ghz", "2", missing_path],
        ] {
            let (code, out, err) = call(list);
            assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
            assert!(!err.is_empty(), "{list:?}");
        }
    }
}
