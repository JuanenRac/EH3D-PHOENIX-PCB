// SPDX-License-Identifier: GPL-3.0-or-later
//! The command line of EH3D-PHOENIX-PCB. It has two commands, `inventory` and `eagle-rate`.
//!
//! Exit codes: 0 done, 2 the arguments or the input are unusable.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use eh3d_phoenix_corpus::{inventory, InventoryConfig};
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
}
