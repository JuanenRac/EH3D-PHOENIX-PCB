// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `fabpack`: the fabrication package of a board, a deterministic `.zip` file (or a new
//! folder) with the Gerber and Excellon files of an export profile, the pick and place files, the
//! bills of materials of the assembly, the stencil and a manifest `phoenix-fab-package/1`.
//!
//! It is a dry run unless `--out` names a destination: the package is made and read back in memory
//! either way, and the list of its files is shown. The destination must not exist; nothing that
//! exists is ever replaced. Exit codes: 0 done, 2 the arguments or the input are unusable.
//!
//! This module also holds the small helpers that `pnp` and `stencil` share: the reading of a
//! bounded text file, the profile, the settings files and the set of parts that are not fitted.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_bom::{BomInputs, CostSettings, PartsFile, PriceBook};
use eh3d_phoenix_export::fabcodes::{
    FabError, Finding, Severity, FABPACK_FAILED, FABPACK_INPUT_UNREADABLE, FABPACK_NOTES,
    FABPACK_OPTION_INVALID, FABPACK_PROFILE,
};
use eh3d_phoenix_export::fabpack::{
    build_pack, is_zip_destination, sha256_hex, standard_notes, write_new, PackInput,
};
use eh3d_phoenix_export::pnp::{plan, PnpSettings};
use eh3d_phoenix_export::stencil::{build_stencil, StencilSettings};
use eh3d_phoenix_export::{build, FileRole, Lang, Profile, ProfileSet};

use crate::export::read_board_file;
use crate::pnp::pnp_files;

/// The profile file that is read when `--profiles` is not given and the file exists.
const DEFAULT_PROFILES: &str = "config/export_profiles.json";
/// The settings files that are read when none is named and the file exists.
pub(crate) const DEFAULT_PNP: &str = "config/pnp_rotations.cfg";
pub(crate) const DEFAULT_STENCIL: &str = "config/stencil.cfg";
const DEFAULT_COSTS: &str = "config/costs.cfg";

/// The largest settings, parts or prices file that is read, in bytes.
const MAX_SMALL_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) const USAGE: &str = "\
phoenix fabpack [--profile NAME] [--profiles FILE] [--pnp-config FILE] [--stencil-config FILE]
                [--costs FILE] [--parts FILE] [--prices FILE] [--maker NAME] [--panel-boards N]
                [--dnp R1,R2,...] [--lang CODE] [--out FOLDER|FILE.zip] <board>

  fabpack       the fabrication package of a board: the Gerber and Excellon files of the export
                profile, the pick and place files of each face (CSV and JSON), the bills of
                materials of the assembly (top, bottom and whole board, with the quantities of a
                board and of a panel) and the solder paste stencil with its report, in one
                deterministic zip file (fixed order, every date 1980-01-01, no attributes) or in a
                new folder, with a manifest phoenix-fab-package/1 that lists every file with its
                SHA-256, the version of the program, the profile, the unit and the notes about
                what the files cannot say. The package is read back and checked against its
                manifest before it is written. A dry run unless --out names a destination, which
                must not exist (a name ending in .zip is a zip file, anything else a new folder).
                A prototype: never sent to a manufacturer
  --profile     the export profile (default: generic); --profiles FILE holds the profiles
                (default: config/export_profiles.json when it exists)
  --pnp-config  the settings of the pick and place files (default: config/pnp_rotations.cfg when
                it exists); --maker picks the section of a machine or maker
  --stencil-config the settings of the stencil (default: config/stencil.cfg when it exists)
  --costs       the settings of the bill (default: config/costs.cfg when it exists)
  --parts       a CSV of part data (maker, part number, parts not fitted); --prices a price file
  --panel-boards N the boards of a panel, for the quantities of the panel (default: 1)
  --dnp         designators of parts that are not fitted, besides those of the parts file
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

/// Reads a text file that is no larger than `limit` bytes.
pub(crate) fn read_small(path: &str, limit: u64) -> Result<String, FabError> {
    let unreadable =
        |e: std::io::Error| FabError::new(FABPACK_INPUT_UNREADABLE, format!("{path}: {e}"));
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    if size > limit {
        return Err(FabError::new(
            FABPACK_INPUT_UNREADABLE,
            format!("{path} is {size} bytes, the limit is {limit}"),
        ));
    }
    std::fs::read_to_string(path).map_err(unreadable)
}

/// The text of a settings file: the one named, else the default when it exists, else `None`.
fn settings_text(named: Option<&str>, default: &str) -> Result<Option<String>, FabError> {
    let path = named.or_else(|| Path::new(default).is_file().then_some(default));
    path.map(|p| read_small(p, MAX_SMALL_BYTES)).transpose()
}

pub(crate) fn load_pnp_settings(
    named: Option<&str>,
    maker: Option<&str>,
) -> Result<PnpSettings, FabError> {
    let mut settings = match settings_text(named, DEFAULT_PNP)? {
        Some(text) => PnpSettings::parse(&text)?,
        None => PnpSettings::default(),
    };
    if let Some(maker) = maker {
        settings.select_maker(maker)?;
    }
    Ok(settings)
}

pub(crate) fn load_stencil_settings(named: Option<&str>) -> Result<StencilSettings, FabError> {
    match settings_text(named, DEFAULT_STENCIL)? {
        Some(text) => StencilSettings::parse(&text),
        None => Ok(StencilSettings::default()),
    }
}

/// The export profile asked for: from the profile file, or the built-in generic one.
pub(crate) fn load_profile(name: &str, file: Option<&str>) -> Result<Profile, FabError> {
    let path = file.or_else(|| {
        Path::new(DEFAULT_PROFILES)
            .is_file()
            .then_some(DEFAULT_PROFILES)
    });
    let Some(path) = path else {
        return if name == "generic" {
            Ok(Profile::default())
        } else {
            Err(FabError::new(
                FABPACK_PROFILE,
                format!("'{name}' is not a built-in profile and there is no profile file"),
            ))
        };
    };
    let text = read_small(path, MAX_SMALL_BYTES)?;
    ProfileSet::from_json(&text)
        .and_then(|set| set.get(name).cloned())
        .map_err(|e| FabError::new(FABPACK_PROFILE, e.to_string()))
}

/// The designators of the parts that are not fitted: those the file of part data says so of, and
/// those named by `extra`.
pub(crate) fn not_fitted_set(
    board: &Board,
    parts: &PartsFile,
    extra: &[String],
) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = extra.iter().cloned().collect();
    for c in board.components.values() {
        let footprint = board
            .footprints
            .get(&c.footprint)
            .map_or("", |f| f.name.as_str());
        if parts
            .find(&c.refdes, &c.value, footprint)
            .is_some_and(|d| !d.fitted)
        {
            set.insert(c.refdes.clone());
        }
    }
    set
}

pub(crate) fn load_parts(named: Option<&str>) -> Result<PartsFile, FabError> {
    match named {
        Some(path) => PartsFile::parse(&read_small(path, MAX_SMALL_BYTES)?, 1_000_000)
            .map_err(|e| FabError::new(FABPACK_INPUT_UNREADABLE, e.to_string())),
        None => Ok(PartsFile::default()),
    }
}

/// The designators of a `--dnp` list.
pub(crate) fn split_designators(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(crate) fn board_of(path: &str) -> Result<Board, FabError> {
    read_board_file(path).map_err(|e| FabError::new(FABPACK_INPUT_UNREADABLE, e))
}

/// Writes the files to a folder that does not exist.
pub(crate) fn write_folder_new(
    dir: &str,
    files: &[(String, Vec<u8>)],
    code: &'static str,
) -> Result<(), FabError> {
    let dest = Path::new(dir);
    if dest.exists() {
        return Err(FabError::new(
            code,
            format!("{dir} exists and is never replaced"),
        ));
    }
    let failed = |what: &str, e: std::io::Error| FabError::new(code, format!("{what}: {e}"));
    std::fs::create_dir_all(dest).map_err(|e| failed(dir, e))?;
    for (name, bytes) in files {
        let target = dest.join(name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| failed(name, e))?;
        file.write_all(bytes).map_err(|e| failed(name, e))?;
    }
    Ok(())
}

struct Options {
    lang: Lang,
    profile: String,
    profiles: Option<String>,
    pnp_config: Option<String>,
    stencil_config: Option<String>,
    costs: Option<String>,
    parts: Option<String>,
    prices: Option<String>,
    maker: Option<String>,
    panel_boards: u64,
    dnp: Vec<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, FabError> {
    let mut o = Options {
        lang: Lang::En,
        profile: "generic".to_owned(),
        profiles: None,
        pnp_config: None,
        stencil_config: None,
        costs: None,
        parts: None,
        prices: None,
        maker: None,
        panel_boards: 1,
        dnp: Vec::new(),
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| FabError::new(FABPACK_OPTION_INVALID, what.to_owned());
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
            "--profile" => o.profile = value("--profile needs a name")?,
            "--profiles" => o.profiles = Some(value("--profiles needs a file")?),
            "--pnp-config" => o.pnp_config = Some(value("--pnp-config needs a file")?),
            "--stencil-config" => o.stencil_config = Some(value("--stencil-config needs a file")?),
            "--costs" => o.costs = Some(value("--costs needs a file")?),
            "--parts" => o.parts = Some(value("--parts needs a file")?),
            "--prices" => o.prices = Some(value("--prices needs a file")?),
            "--maker" => o.maker = Some(value("--maker needs a name")?),
            "--panel-boards" => {
                o.panel_boards = value("--panel-boards needs a number")?
                    .parse::<u64>()
                    .ok()
                    .filter(|n| (1..=10_000).contains(n))
                    .ok_or_else(|| bad("--panel-boards is a whole number from 1 to 10000"))?;
            }
            "--dnp" => o
                .dnp
                .extend(split_designators(&value("--dnp needs a list")?)),
            "--out" => o.out = Some(value("--out needs a destination")?),
            other if other.starts_with("--") => {
                return Err(FabError::new(
                    FABPACK_OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn fabpack(o: &Options, out: &mut dyn Write) -> Result<i32, FabError> {
    let [board_path] = o.files.as_slice() else {
        return Err(FabError::new(
            FABPACK_OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let board = board_of(board_path)?;
    let profile = load_profile(&o.profile, o.profiles.as_deref())?;
    let pnp_settings = load_pnp_settings(o.pnp_config.as_deref(), o.maker.as_deref())?;
    let stencil_settings = load_stencil_settings(o.stencil_config.as_deref())?;
    let costs = match settings_text(o.costs.as_deref(), DEFAULT_COSTS)? {
        Some(text) => CostSettings::parse(&text)
            .map_err(|e| FabError::new(FABPACK_INPUT_UNREADABLE, e.to_string()))?,
        None => CostSettings::default(),
    };
    costs
        .validate()
        .map_err(|e| FabError::new(FABPACK_INPUT_UNREADABLE, e.to_string()))?;
    let parts = load_parts(o.parts.as_deref())?;
    let prices = match &o.prices {
        Some(path) => PriceBook::parse(&read_small(path, MAX_SMALL_BYTES)?, costs.max_items)
            .map_err(|e| FabError::new(FABPACK_INPUT_UNREADABLE, e.to_string()))?,
        None => PriceBook::default(),
    };

    let mut findings: Vec<Finding> = Vec::new();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    let set = build(&board, &profile).map_err(|e| FabError::new(FABPACK_FAILED, e.to_string()))?;
    for file in &set.files {
        let folder = match file.role {
            FileRole::Layer { .. } => "gerber",
            FileRole::Drill { .. } => "drill",
            FileRole::Placement | FileRole::Bom => continue,
        };
        files.push((
            format!("{folder}/{}", file.folder_name()),
            file.bytes.clone(),
        ));
    }
    let export_notes = set.diagnostics.len();

    let not_fitted = not_fitted_set(&board, &parts, &o.dnp);
    let placement = plan(&board, &pnp_settings, &not_fitted);
    files.extend(pnp_files(&placement, &pnp_settings, "pnp/"));
    findings.extend(placement.findings.iter().cloned());

    let stencil = build_stencil(&board, &stencil_settings, &profile.gerber.x2)?;
    for f in &stencil.files {
        files.push((format!("stencil/{}", f.name), f.bytes.clone()));
    }
    findings.extend(stencil.findings.iter().cloned());

    let inputs = BomInputs {
        board: &board,
        schematic: None,
        parts: &parts,
        prices: &prices,
        settings: &costs,
        stackup: None,
    };
    let assembly = eh3d_phoenix_bom::assembly::build_assembly(&inputs, o.panel_boards)
        .map_err(|e| FabError::new(FABPACK_INPUT_UNREADABLE, e.to_string()))?;
    for face in &assembly.faces {
        if face.name != "board" && face.lines.is_empty() && face.not_fitted.is_empty() {
            continue;
        }
        files.push((
            format!("bom/bom_{}.csv", face.name),
            eh3d_phoenix_bom::assembly::assembly_csv(face, assembly.boards_per_panel).into_bytes(),
        ));
    }
    files.push((
        "bom/bom_assembly.json".to_owned(),
        eh3d_phoenix_bom::assembly::assembly_json(&assembly).into_bytes(),
    ));

    let mut notes = standard_notes();
    for f in &findings {
        if f.severity >= Severity::Warning {
            notes.push(f.line(Lang::En));
        }
    }
    let note_count = notes.len();
    let pack = build_pack(&PackInput {
        board: if board.name.trim().is_empty() {
            "board".to_owned()
        } else {
            board.name.clone()
        },
        profile: profile.name.clone(),
        units: pnp_settings.unit.name().to_owned(),
        files,
        notes,
    })?;

    let mut text = format!(
        "board: {}\nprofile: {}\npackage: {} file(s) and the manifest, {} bytes as a zip file\n",
        board.name,
        profile.name,
        pack.files.len() - 1,
        pack.zip.len()
    );
    for (name, bytes) in &pack.files {
        text.push_str(&format!(
            "  {name}  {} bytes  {}\n",
            bytes.len(),
            &sha256_hex(bytes)[..12]
        ));
    }
    if export_notes > 0 {
        text.push_str(&format!(
            "the export of the layers left {export_notes} note(s) (run `phoenix export` for them)\n"
        ));
    }
    let noted = Finding::new(
        FABPACK_NOTES,
        Severity::Info,
        "",
        format!("{note_count} note(s) in the manifest"),
    );
    text.push_str(&noted.line(o.lang));
    text.push('\n');
    for f in &findings {
        text.push_str(&f.line(o.lang));
        text.push('\n');
    }
    if let Some(dest) = &o.out {
        let dest_path = Path::new(dest);
        write_new(dest_path, &pack)?;
        let kind = if is_zip_destination(dest_path) {
            "zip file"
        } else {
            "folder"
        };
        text.push_str(&format!("written: {dest} ({kind})\n"));
    } else {
        text.push_str("dry run: nothing was written, give --out to write the package\n");
    }
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

pub(crate) fn fabpack_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    match fabpack(&options, out) {
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
    use eh3d_phoenix_export::fabcodes::FABPACK_OUTPUT_FAILED;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "--profile",
            "x2-full",
            "--panel-boards",
            "6",
            "--dnp",
            "R1, C2,",
            "--dnp",
            "U3",
            "--out",
            "a.zip",
            "--lang",
            "pt",
            "b.json",
        ])
        .unwrap();
        assert_eq!(o.profile, "x2-full");
        assert_eq!(o.panel_boards, 6);
        assert_eq!(o.dnp, vec!["R1", "C2", "U3"]);
        assert_eq!(o.out.as_deref(), Some("a.zip"));
        assert_eq!(o.lang, Lang::Pt);
        assert_eq!(o.files, vec!["b.json".to_owned()]);
        for bad in [
            vec!["--panel-boards", "0"],
            vec!["--panel-boards", "x"],
            vec!["--profile"],
            vec!["--lang", "xx"],
            vec!["--nope"],
            vec!["--out"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, FABPACK_OPTION_INVALID, "{bad:?}");
        }
    }

    #[test]
    fn designators_are_split_on_commas() {
        assert_eq!(split_designators("R1, R2,,C3 "), vec!["R1", "R2", "C3"]);
        assert!(split_designators(" , ").is_empty());
    }

    #[test]
    fn a_new_folder_is_written_and_an_existing_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out");
        let target = target.to_str().unwrap();
        let files = vec![("a.txt".to_owned(), b"x".to_vec())];
        write_folder_new(target, &files, FABPACK_OUTPUT_FAILED).unwrap();
        assert_eq!(
            std::fs::read(Path::new(target).join("a.txt")).unwrap(),
            b"x"
        );
        let e = write_folder_new(target, &files, FABPACK_OUTPUT_FAILED).unwrap_err();
        assert_eq!(e.code, FABPACK_OUTPUT_FAILED);
    }

    #[test]
    fn an_unknown_profile_without_a_profile_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("none.json");
        let e = load_profile("other", missing.to_str()).unwrap_err();
        assert_eq!(e.code, FABPACK_INPUT_UNREADABLE);
        assert_eq!(load_profile("generic", None).unwrap().name, "generic");
    }
}
