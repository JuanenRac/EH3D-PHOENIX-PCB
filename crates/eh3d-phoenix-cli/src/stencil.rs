// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `stencil`: the solder paste stencil of a board, the Gerber files of the openings of
//! each face and the report of every opening.
//!
//! It is a dry run unless `--out` names a folder, which must not exist. Exit codes: 0 done, 2 the
//! arguments or the input are unusable.

use std::io::Write;

use eh3d_phoenix_export::fabcodes::{
    FabError, Severity, STENCIL_OPTION_INVALID, STENCIL_OUTPUT_FAILED,
};
use eh3d_phoenix_export::stencil::build_stencil;
use eh3d_phoenix_export::Lang;

use crate::fabpack::{board_of, load_profile, load_stencil_settings, write_folder_new};

pub(crate) const USAGE: &str = "\
phoenix stencil [--config FILE] [--profile NAME] [--profiles FILE] [--lang CODE] [--out FOLDER]
                <board>

  stencil       the solder paste stencil of a board: for each face the Gerber file of the openings
                made from the paste layer, reduced by footprint family (a per cent or a length),
                with large pads split into windows with bridges, corners rounded, the aspect and
                area ratios of IPC-7525 checked against the thickness of the plate, and a CSV
                report with a line for every opening. The Gerber X2 attributes follow the export
                profile. The settings are in config/stencil.cfg. A dry run unless --out names a
                folder that does not exist. A prototype: no plate was ever made from it
  --config      the settings (default: config/stencil.cfg when it exists)
  --profile     the export profile for the Gerber X2 attributes (default: generic); --profiles
                FILE holds the profiles (default: config/export_profiles.json when it exists)
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    lang: Lang,
    config: Option<String>,
    profile: String,
    profiles: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, FabError> {
    let mut o = Options {
        lang: Lang::En,
        config: None,
        profile: "generic".to_owned(),
        profiles: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| FabError::new(STENCIL_OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, FabError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                o.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => o.config = Some(value("--config needs a file")?),
            "--profile" => o.profile = value("--profile needs a name")?,
            "--profiles" => o.profiles = Some(value("--profiles needs a file")?),
            "--out" => o.out = Some(value("--out needs a folder")?),
            other if other.starts_with("--") => {
                return Err(FabError::new(
                    STENCIL_OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn stencil(o: &Options, out: &mut dyn Write) -> Result<i32, FabError> {
    let [board_path] = o.files.as_slice() else {
        return Err(FabError::new(
            STENCIL_OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let settings = load_stencil_settings(o.config.as_deref())?;
    let profile = load_profile(&o.profile, o.profiles.as_deref())?;
    let board = board_of(board_path)?;
    let set = build_stencil(&board, &settings, &profile.gerber.x2)?;

    let removed = set.openings.iter().filter(|x| x.panes.is_empty()).count();
    let windows: usize = set.openings.iter().filter(|x| x.panes.len() > 1).count();
    let low = set
        .openings
        .iter()
        .filter(|x| x.aspect_low || x.area_low)
        .count();
    let mut text = format!(
        "board: {}\nopenings: {}, split into windows: {}, below a ratio: {}, taken away: {}\n",
        board.name,
        set.openings.len(),
        windows,
        low,
        removed
    );
    for f in &set.findings {
        if f.severity >= Severity::Warning || o.out.is_none() {
            text.push_str(&f.line(o.lang));
            text.push('\n');
        }
    }
    let files: Vec<(String, Vec<u8>)> = set
        .files
        .iter()
        .map(|f| (f.name.clone(), f.bytes.clone()))
        .collect();
    if let Some(dir) = &o.out {
        write_folder_new(dir, &files, STENCIL_OUTPUT_FAILED)?;
        for (name, _) in &files {
            text.push_str(&format!("written: {dir}/{name}\n"));
        }
    } else {
        for (name, bytes) in &files {
            text.push_str(&format!("  {name}: {} bytes\n", bytes.len()));
        }
        text.push_str("dry run: nothing was written, give --out to write the files\n");
    }
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

pub(crate) fn stencil_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Lang::En));
            return 2;
        }
    };
    match stencil(&options, out) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "--config",
            "s.cfg",
            "--profile",
            "x2-full",
            "--out",
            "d",
            "--lang",
            "de",
            "b.json",
        ])
        .unwrap();
        assert_eq!(o.config.as_deref(), Some("s.cfg"));
        assert_eq!(o.profile, "x2-full");
        assert_eq!(o.out.as_deref(), Some("d"));
        assert_eq!(o.lang, Lang::De);
        for bad in [
            vec!["--config"],
            vec!["--profile"],
            vec!["--lang", "xx"],
            vec!["--nope"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, STENCIL_OPTION_INVALID, "{bad:?}");
        }
    }
}
