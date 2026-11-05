// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `export`: read a board, make the manufacturing files with a profile, check them
//! against the board and write them to a folder with a manifest of SHA-256 sums.
//!
//! Exit codes: 0 done, 1 the check found a difference between the files and the board (nothing is
//! written), 2 the arguments, the board, the profile or the output folder are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle_board::read_board;
use eh3d_phoenix_export::messages::{localize, written};
use eh3d_phoenix_export::{
    build, export_package_reproducible, verify, ExportError, Lang, Profile, ProfileSet,
};

use crate::check::{id_source, read_text};

/// The profile file that is read when `--profiles` is not given and the file exists.
const DEFAULT_PROFILES: &str = "config/export_profiles.json";

/// The largest board or profile file that is read, in bytes.
const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix export [--profile NAME] [--profiles FILE] [--lang CODE] [--eh3dproj FILE] --out <folder> <board>

  export        make the manufacturing files of a board: Gerber X2 for every layer, the drill files
                (plated and not plated), the placement file, the bill of materials and manifest.json
                with the SHA-256 of each file. <board> is an EAGLE board (.brd) or the JSON board of
                a project (.json). The files are read back and compared with the board before
                anything is written (exit 1 when they differ)
  --profile     the profile to use (default: generic); --profiles FILE is the JSON file that holds
                the profiles (default: config/export_profiles.json when it exists)
  --eh3dproj    also write the files as a reproducible .eh3dproj package
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    out: Option<String>,
    profile: String,
    profiles: Option<String>,
    package: Option<String>,
    lang: Lang,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        out: None,
        profile: "generic".to_owned(),
        profiles: None,
        package: None,
        lang: Lang::En,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--out" => options.out = Some((*iter.next().ok_or("--out needs a folder")?).to_owned()),
            "--profile" => {
                options.profile = (*iter.next().ok_or("--profile needs a name")?).to_owned()
            }
            "--profiles" => {
                options.profiles = Some((*iter.next().ok_or("--profiles needs a file")?).to_owned())
            }
            "--eh3dproj" => {
                options.package = Some((*iter.next().ok_or("--eh3dproj needs a file")?).to_owned())
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

pub(crate) fn read_board_file(path: &str) -> Result<Board, String> {
    if path.to_ascii_lowercase().ends_with(".json") {
        let size = std::fs::metadata(path)
            .map_err(|e| format!("cannot read {path}: {e}"))?
            .len();
        if size > MAX_INPUT_BYTES {
            return Err(format!(
                "{path} is {size} bytes, the limit is {MAX_INPUT_BYTES}"
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        return Board::from_bytes(&bytes).map_err(|e| format!("{path}: {e}"));
    }
    let text = read_text(path)?;
    let mut new_id = id_source()?;
    read_board(&text, &mut new_id)
        .map(|read| read.board)
        .map_err(|diagnostic| diagnostic.message)
}

fn profile_of(options: &Options) -> Result<Profile, ExportError> {
    let explicit = options.profiles.as_deref();
    let path = explicit.or_else(|| {
        Path::new(DEFAULT_PROFILES)
            .is_file()
            .then_some(DEFAULT_PROFILES)
    });
    let Some(path) = path else {
        return if options.profile == "generic" {
            Ok(Profile::default())
        } else {
            Err(ExportError::ProfileNotFound(options.profile.clone()))
        };
    };
    let text = read_text(path).map_err(ExportError::Profile)?;
    ProfileSet::from_json(&text)?.get(&options.profile).cloned()
}

fn error_line(error: &ExportError, lang: Lang) -> String {
    let code = error.code();
    format!(
        "error {code}: {}",
        localize(code, &error.diagnostic().message, lang)
    )
}

pub(crate) fn export(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let ([board_path], Some(out_dir)) = (options.files.as_slice(), options.out.as_deref()) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let lang = options.lang;
    let profile = match profile_of(&options) {
        Ok(profile) => profile,
        Err(error) => {
            let _ = writeln!(err, "{}", error_line(&error, lang));
            return 2;
        }
    };
    let board = match read_board_file(board_path) {
        Ok(board) => board,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return 2;
        }
    };
    let set = match build(&board, &profile) {
        Ok(set) => set,
        Err(error) => {
            let _ = writeln!(err, "{}", error_line(&error, lang));
            return 2;
        }
    };
    let differences = verify(&board, &profile, &set);
    if !differences.is_empty() {
        for d in &differences {
            let code = d.code.as_deref().unwrap_or("");
            let _ = writeln!(err, "error {code}: {}", localize(code, &d.message, lang));
        }
        return 1;
    }
    if let Err(error) = set.write_folder(Path::new(out_dir)) {
        let _ = writeln!(err, "{}", error_line(&error, lang));
        return 2;
    }
    if let Some(path) = &options.package {
        let result = export_package_reproducible(&board, &profile).and_then(|exported| {
            exported
                .package
                .write_to_path(Path::new(path))
                .map_err(|e| ExportError::Output(e.to_string()))
        });
        if let Err(error) = result {
            let _ = writeln!(err, "{}", error_line(&error, lang));
            return 2;
        }
    }
    let _ = writeln!(out, "{}: {out_dir}", written(set.files.len() + 1, lang));
    for file in &set.files {
        let _ = writeln!(out, "  {}", file.folder_name());
    }
    let _ = writeln!(out, "  {}", eh3d_phoenix_export::fab::MANIFEST_FILE);
    for d in &set.diagnostics {
        let code = d.code.as_deref().unwrap_or("");
        let _ = writeln!(err, "note {code}: {}", localize(code, &d.message, lang));
    }
    0
}
