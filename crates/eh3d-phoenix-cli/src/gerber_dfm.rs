// SPDX-License-Identifier: GPL-3.0-or-later
//! The mode `phoenix check --gerber-dfm FOLDER|ZIP --profile NAME`: read the Gerber and Excellon
//! files of a folder or of a ZIP archive and check their geometry against a capability profile of a
//! maker (spacing, width, annular ring in the worst case, copper at the edge, mask bridge, legend
//! over pads, via in pad, floating copper).
//!
//! Exit codes of this mode: 0 the files pass, 2 the verification is not passed (at least one error
//! finding), 1 the command could not run (unusable arguments or input).

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_gerber::geodfm_text::{report_csv, report_json, report_text};
use eh3d_phoenix_gerber::{check_set, import_files, GeoSettings, ImportSettings};
use eh3d_phoenix_ipc2581::zipread::{list, looks_like_zip, unpack, ZipLimits};
use eh3d_phoenix_rules::{builtin_names, builtin_profile, DfmProfile, Lang};

pub(crate) const USAGE: &str = "\
phoenix check --gerber-dfm FOLDER|ZIP --profile NAME|FILE [--config FILE] [--json] [--csv FILE] [--lang CODE]

  --gerber-dfm PATH  a folder of Gerber and Excellon files, or a ZIP archive of them, checked by their
                     geometry: spacing, conductor width, annular ring in the worst case, copper at the
                     edge, mask bridge, legend over pads, via in a pad and floating copper
  --profile NAME     the capability profile of a maker: a name that comes with the program or a file
                     (its figures are the ones of the profile and are not verified unless it says so)
  --config FILE      the settings of the checks (default config/gerber_dfm.cfg)
  --json             print the report as JSON
  --csv FILE         also write the findings as CSV to a new file
  --lang CODE        en, es, de, fr, it, ja, zh or pt (Portuguese of Portugal)

Exit code 0: the files pass. 2: the verification is not passed. 1: the command could not run.
";

struct Options<'a> {
    path: &'a str,
    profile: &'a str,
    config: Option<&'a str>,
    json: bool,
    csv: Option<&'a str>,
    lang: Lang,
}

fn parse<'a>(args: &[&'a str]) -> Result<Options<'a>, String> {
    let mut path = None;
    let mut profile = None;
    let mut config = None;
    let mut json = false;
    let mut csv = None;
    let mut lang = Lang::En;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--gerber-dfm" => {
                path = Some(*iter.next().ok_or("--gerber-dfm needs a folder or a ZIP")?)
            }
            "--profile" => profile = Some(*iter.next().ok_or("--profile needs a name or a file")?),
            "--config" => config = Some(*iter.next().ok_or("--config needs a file")?),
            "--json" => json = true,
            "--csv" => csv = Some(*iter.next().ok_or("--csv needs a file to write")?),
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                lang = Lang::from_tag(tag).ok_or_else(|| format!("unknown language {tag}"))?;
            }
            other => return Err(format!("{other} cannot be given with --gerber-dfm")),
        }
    }
    Ok(Options {
        path: path.ok_or("--gerber-dfm needs a folder or a ZIP")?,
        profile: profile.ok_or("--gerber-dfm needs --profile")?,
        config,
        json,
        csv,
        lang,
    })
}

fn small_text(path: &str) -> Result<String, String> {
    let length = std::fs::metadata(path)
        .map_err(|e| format!("{path}: {e}"))?
        .len();
    if length > 1024 * 1024 {
        return Err(format!("{path}: a settings or profile file over 1 MiB"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}

fn load_profile(name: &str) -> Result<DfmProfile, String> {
    let text = match builtin_profile(name) {
        Some(text) => text.to_owned(),
        None if Path::new(name).is_file() => small_text(name)?,
        None => {
            return Err(format!(
                "no profile named {name}; the names are {}, or give a file",
                builtin_names()
            ))
        }
    };
    DfmProfile::parse(&text).map_err(|e| format!("profile {name}: {e}"))
}

/// The files to read, as (name, bytes): the files of a folder, or the entries of a ZIP archive
/// unpacked in memory (a name with a folder keeps only its last part).
fn inputs(
    path: &str,
    limits: &ZipLimits,
    max_file_bytes: u64,
    max_files: usize,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let p = Path::new(path);
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    if p.is_dir() {
        let mut names: Vec<std::path::PathBuf> = std::fs::read_dir(p)
            .map_err(|e| format!("{path}: {e}"))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|f| f.is_file())
            .collect();
        names.sort();
        if names.len() > max_files {
            return Err(format!("{} files, the limit is {max_files}", names.len()));
        }
        for file in names {
            let length = std::fs::metadata(&file).map_or(0, |m| m.len());
            if length > max_file_bytes {
                continue;
            }
            let name = file
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            if let Ok(bytes) = std::fs::read(&file) {
                out.push((name, bytes));
            }
        }
        return Ok(out);
    }
    let length = std::fs::metadata(p)
        .map_err(|e| format!("{path}: {e}"))?
        .len();
    if length > limits.max_total_bytes {
        return Err(format!("{path}: larger than the limit of the archive"));
    }
    let bytes = std::fs::read(p).map_err(|e| format!("{path}: {e}"))?;
    if !looks_like_zip(&bytes) {
        return Err(format!("{path} is neither a folder nor a ZIP archive"));
    }
    let entries = list(&bytes, limits).map_err(|e| format!("{path}: {e}"))?;
    for entry in entries.iter().filter(|e| !e.is_folder()) {
        if out.len() >= max_files {
            return Err(format!("more than {max_files} files in the archive"));
        }
        let data = unpack(&bytes, entry, limits).map_err(|e| format!("{path}: {e}"))?;
        let name = entry
            .name
            .rsplit('/')
            .next()
            .unwrap_or(&entry.name)
            .to_owned();
        out.push((name, data));
    }
    Ok(out)
}

pub(crate) fn check_gerber_dfm(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            return 1;
        }
    };
    let run = |options: &Options| -> Result<(String, bool), String> {
        let profile = load_profile(options.profile)?;
        let settings = match options.config {
            Some(file) => GeoSettings::parse(&small_text(file)?)?,
            None => GeoSettings::standard(),
        };
        let import = ImportSettings::default();
        let limits = ZipLimits {
            max_entries: settings.zip_max_entries,
            max_total_bytes: settings.zip_max_total_bytes,
            max_entry_bytes: import.max_file_bytes as u64,
            max_ratio: settings.zip_max_ratio,
        };
        let files = inputs(
            options.path,
            &limits,
            import.max_file_bytes as u64,
            import.max_files,
        )?;
        let set = import_files(&files, &import).map_err(|e| e.to_string())?;
        let report = check_set(&set, &profile, &settings);
        if let Some(csv) = options.csv {
            if Path::new(csv).exists() {
                return Err(format!("{csv} exists; a file is never replaced"));
            }
            std::fs::write(csv, report_csv(&report, options.lang))
                .map_err(|e| format!("{csv}: {e}"))?;
        }
        let text = if options.json {
            let mut text =
                serde_json::to_string_pretty(&report_json(&report)).map_err(|e| e.to_string())?;
            text.push('\n');
            text
        } else {
            report_text(&report, options.lang)
        };
        Ok((text, report.passed()))
    };
    match run(&options) {
        Ok((text, passed)) => {
            let _ = out.write_all(text.as_bytes());
            if passed {
                0
            } else {
                2
            }
        }
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COPPER: &str = "%FSLAX46Y46*%\n%MOMM*%\n%TF.FileFunction,Copper,L1,Top*%\n%ADD10C,0.100000*%\nD10*\nX1000000Y1000000D02*\nX9000000Y1000000D01*\nM02*\n";

    fn run(args: &[&str]) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = check_gerber_dfm(args, &mut out, &mut err);
        (
            code,
            String::from_utf8_lossy(&out).into_owned(),
            String::from_utf8_lossy(&err).into_owned(),
        )
    }

    fn folder_with_board() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("board-F_Cu.gtl"), COPPER).unwrap();
        let profile = dir.path().join("maker.cfg");
        std::fs::write(&profile, "name = test maker\nmin_track_width_nm = 150000\n").unwrap();
        let folder = dir.path().join("gerbers");
        std::fs::create_dir(&folder).unwrap();
        std::fs::copy(
            dir.path().join("board-F_Cu.gtl"),
            folder.join("board-F_Cu.gtl"),
        )
        .unwrap();
        let profile = profile.to_string_lossy().into_owned();
        (dir, profile)
    }

    #[test]
    fn a_folder_that_breaks_the_profile_exits_with_2_and_one_that_passes_with_0() {
        let (dir, profile) = folder_with_board();
        let folder = dir.path().join("gerbers").to_string_lossy().into_owned();
        let (code, out, err) = run(&["--gerber-dfm", &folder, "--profile", &profile]);
        assert_eq!(code, 2, "{out}{err}");
        assert!(out.starts_with("FAIL"), "{out}");
        assert!(out.contains("PHOENIX-GDF-002"), "{out}");
        let (code, out, _) = run(&[
            "--gerber-dfm",
            &folder,
            "--profile",
            &profile,
            "--lang",
            "pt",
        ]);
        assert_eq!(code, 2);
        assert!(out.starts_with("REPROVADO"), "{out}");
        std::fs::write(&profile, "name = test maker\nmin_track_width_nm = 90000\n").unwrap();
        let (code, out, err) = run(&["--gerber-dfm", &folder, "--profile", &profile]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.starts_with("PASS"), "{out}");
    }

    #[test]
    fn the_json_and_the_csv_are_written_and_a_csv_is_never_replaced() {
        let (dir, profile) = folder_with_board();
        let folder = dir.path().join("gerbers").to_string_lossy().into_owned();
        let csv = dir.path().join("out.csv").to_string_lossy().into_owned();
        let (code, out, err) = run(&[
            "--gerber-dfm",
            &folder,
            "--profile",
            &profile,
            "--json",
            "--csv",
            &csv,
        ]);
        assert_eq!(code, 2, "{err}");
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["passed"], false);
        assert_eq!(value["format"], "phoenix-gerber-dfm/1");
        let text = std::fs::read_to_string(&csv).unwrap();
        assert!(text.starts_with("code,severity,layer,"));
        assert!(text.contains("PHOENIX-GDF-002"));
        let (code, _, err) = run(&[
            "--gerber-dfm",
            &folder,
            "--profile",
            &profile,
            "--csv",
            &csv,
        ]);
        assert_eq!(code, 1);
        assert!(err.contains("exists"), "{err}");
    }

    #[test]
    fn a_wrong_call_exits_with_1_and_says_why() {
        let (code, _, err) = run(&["--gerber-dfm", "x"]);
        assert_eq!(code, 1);
        assert!(err.contains("--profile"), "{err}");
        let (code, _, err) = run(&["--gerber-dfm", "/no/such/folder", "--profile", "jlcpcb"]);
        assert_eq!(code, 1, "{err}");
        let (code, _, err) = run(&["--gerber-dfm", ".", "--profile", "no-such-maker"]);
        assert_eq!(code, 1);
        assert!(err.contains("no profile named"), "{err}");
        let (code, _, _) = run(&["--gerber-dfm", ".", "--profile", "jlcpcb", "--lang", "xx"]);
        assert_eq!(code, 1);
    }
}
