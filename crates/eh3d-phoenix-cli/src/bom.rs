// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `bom`: the bill of materials of a board with the cost of the parts and of the
//! boards.
//!
//! Exit codes: 0 done, 1 the bill has an error finding (a component whose footprint is not on the
//! board), 2 the arguments or the input are unusable. Nothing is written unless `--csv`,
//! `--json-file` or `--html` name a new file; an existing file is never replaced, and none is
//! written when one of them exists.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_bom::codes::{FILE_TOO_LARGE, INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED};
use eh3d_phoenix_bom::report::counts;
use eh3d_phoenix_bom::{
    bom_csv, bom_html, bom_json, bom_text, build_bom, BomError, BomInputs, CostSettings, PartsFile,
    PriceBook,
};
use eh3d_phoenix_parts::{entries_of, link_bom, load as load_parts, PartsSettings};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schematic::Schematic;
use eh3d_phoenix_stackup::StackupJob;

use crate::export::read_board_file;

/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/costs.cfg";

pub(crate) const USAGE: &str = "\
phoenix bom [--config FILE] [--prices FILE] [--parts FILE] [--parts-db PATH] [--schematic FILE]
            [--stackup FILE] [--boards N] [--variant NAME [--variant-file FILE]] [--csv FILE] [--json-file FILE] [--html FILE] [--json] [--lang CODE]
            <board>

  bom           the bill of materials of a board: the parts grouped by value, footprint, maker and
                part number with their references, the cheapest supplier for the batch from a
                local price file (a CSV or JSON file with quantity breaks; nothing is fetched from
                a network), the cost per board and per batch with the waste, and an estimate of
                the cost of the boards from the size, the layers and the tariffs of the settings
                (an example, not a quote). Parts without a price or without a footprint are
                listed. Without --csv, --json-file and --html nothing is written; each of them
                writes a new file (an existing file is never replaced and none is written when
                one exists). --json prints the JSON on the screen instead of the text
  --config      the rates, percentages and limits (default: config/costs.cfg when it exists)
  --prices      the file of prices by quantity break
  --parts       a CSV with the maker, the part number and the parts that are not fitted
  --parts-db    the local parts base (a file or a folder, see `phoenix parts`): the maker, the part
                number and the prices of each part are completed from it by value and footprint,
                or by the part number that --parts gives; what --parts and --prices say wins. The
                text ends with the parts without a match or with several, and with the
                alternatives proposed for the parts that are obsolete (nothing is changed)
  --schematic   a JSON schematic: its parts that are not on the board are added
  --stackup     a stack-up job file (the file of `phoenix stackup`) for the thickness of the board
  --boards N    the number of boards of the batch (default: the settings)
  --variant     an assembly variant of an EAGLE file: the parts it does not fit are left out and the
                values it changes replace the values of the board; the variants are read from the
                board file when it is an EAGLE .brd or .sch, otherwise from --variant-file
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

struct Options {
    json: bool,
    lang: Lang,
    config: Option<String>,
    prices: Option<String>,
    parts: Option<String>,
    parts_db: Option<String>,
    schematic: Option<String>,
    stackup: Option<String>,
    boards: Option<u64>,
    variant: Option<String>,
    variant_file: Option<String>,
    csv: Option<String>,
    json_file: Option<String>,
    html: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, BomError> {
    let mut options = Options {
        json: false,
        lang: Lang::En,
        config: None,
        prices: None,
        parts: None,
        parts_db: None,
        schematic: None,
        stackup: None,
        boards: None,
        variant: None,
        variant_file: None,
        csv: None,
        json_file: None,
        html: None,
        files: Vec::new(),
    };
    let bad = |what: &str| BomError::new(OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, BomError> {
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
            "--prices" => options.prices = Some(value("--prices needs a file")?),
            "--parts" => options.parts = Some(value("--parts needs a file")?),
            "--parts-db" => options.parts_db = Some(value("--parts-db needs a path")?),
            "--schematic" => options.schematic = Some(value("--schematic needs a file")?),
            "--stackup" => options.stackup = Some(value("--stackup needs a file")?),
            "--boards" => {
                let n = value("--boards needs a number")?;
                options.boards = Some(
                    n.parse::<u64>()
                        .ok()
                        .filter(|n| (1..=1_000_000).contains(n))
                        .ok_or_else(|| bad("--boards is a whole number from 1 to 1000000"))?,
                );
            }
            "--variant" => options.variant = Some(value("--variant needs a name")?),
            "--variant-file" => options.variant_file = Some(value("--variant-file needs a file")?),
            "--csv" => options.csv = Some(value("--csv needs a file")?),
            "--json-file" => options.json_file = Some(value("--json-file needs a file")?),
            "--html" => options.html = Some(value("--html needs a file")?),
            other if other.starts_with("--") => {
                return Err(BomError::new(
                    OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

/// Reads a text file that is no larger than `limit` bytes.
fn read_limited(path: &str, limit: u64) -> Result<String, BomError> {
    let unreadable = |e: std::io::Error| BomError::new(INPUT_UNREADABLE, format!("{path}: {e}"));
    let size = std::fs::metadata(path).map_err(unreadable)?.len();
    if size > limit {
        return Err(BomError::new(
            FILE_TOO_LARGE,
            format!("{path} is {size} bytes, the limit is {limit}"),
        ));
    }
    std::fs::read_to_string(path).map_err(unreadable)
}

fn load_settings(path: Option<&str>) -> Result<CostSettings, BomError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(CostSettings::default()),
        Some(path) => {
            let text = read_limited(path, 1024 * 1024)?;
            CostSettings::parse(&text)
        }
    }
}

fn fail(err: &mut dyn Write, error: &BomError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

/// Checks that none of the outputs exists or is named twice, then writes each as a new file.
fn write_new_files(files: &[(&str, String)]) -> Result<(), BomError> {
    for (i, (path, _)) in files.iter().enumerate() {
        if Path::new(path).exists() {
            return Err(BomError::new(
                OUTPUT_FAILED,
                format!("{path} exists and is never replaced"),
            ));
        }
        if files[..i].iter().any(|(other, _)| other == path) {
            return Err(BomError::new(
                OUTPUT_FAILED,
                format!("{path} is named for two outputs"),
            ));
        }
    }
    for (path, text) in files {
        let failed = |e: std::io::Error| BomError::new(OUTPUT_FAILED, format!("{path}: {e}"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(failed)?;
        file.write_all(text.as_bytes()).map_err(failed)?;
    }
    Ok(())
}

fn bom(options: &Options, out: &mut dyn Write) -> Result<i32, BomError> {
    let [board_path] = options.files.as_slice() else {
        return Err(BomError::new(OPTION_INVALID, "give exactly one board file"));
    };
    let mut settings = load_settings(options.config.as_deref())?;
    if let Some(boards) = options.boards {
        settings.boards = boards;
    }
    settings.validate()?;
    let unreadable = |what: String| BomError::new(INPUT_UNREADABLE, what);
    let mut board = read_board_file(board_path).map_err(unreadable)?;
    let mut variant_note = String::new();
    if let Some(name) = &options.variant {
        let source = options.variant_file.as_deref().unwrap_or(board_path);
        let view = crate::eagle_deep::variant_view(source, name, options.lang)
            .map_err(|message| BomError::new(OPTION_INVALID, message))?;
        let applied = eh3d_phoenix_eagle_deep::apply_to_board(&mut board, &view);
        variant_note = format!(
            "variant: {name}, left out: {}, values changed: {}
",
            applied.removed.len(),
            applied.revalued.len()
        );
    } else if options.variant_file.is_some() {
        return Err(BomError::new(
            OPTION_INVALID,
            "--variant-file needs --variant",
        ));
    }
    let prices = match &options.prices {
        Some(path) => PriceBook::parse(
            &read_limited(path, settings.max_file_bytes)?,
            settings.max_items,
        )?,
        None => PriceBook::default(),
    };
    let parts = match &options.parts {
        Some(path) => PartsFile::parse(
            &read_limited(path, settings.max_file_bytes)?,
            settings.max_items,
        )?,
        None => PartsFile::default(),
    };
    let schematic = match &options.schematic {
        Some(path) => {
            if !path.to_ascii_lowercase().ends_with(".json") {
                return Err(BomError::new(
                    OPTION_INVALID,
                    "--schematic is a JSON schematic file",
                ));
            }
            let text = read_limited(path, settings.max_file_bytes)?;
            Some(
                Schematic::from_bytes(text.as_bytes())
                    .map_err(|e| unreadable(format!("{path}: {e}")))?,
            )
        }
        None => None,
    };
    let stackup = match &options.stackup {
        Some(path) => {
            let text = read_limited(path, settings.max_file_bytes)?;
            Some(
                StackupJob::parse(&text)
                    .map_err(|e| unreadable(format!("{path}: {e}")))?
                    .stackup,
            )
        }
        None => None,
    };
    let (parts, prices, link) = match &options.parts_db {
        Some(path) => {
            let db = load_parts(Path::new(path), &PartsSettings::default())
                .map_err(|e| unreadable(e.to_string()))?;
            let entries = entries_of(&board, schematic.as_ref());
            let linked = link_bom(&db, &entries, &parts, &prices);
            (linked.parts, linked.prices, Some(linked.report))
        }
        None => (parts, prices, None),
    };
    let bom = build_bom(&BomInputs {
        board: &board,
        schematic: schematic.as_ref(),
        parts: &parts,
        prices: &prices,
        settings: &settings,
        stackup: stackup.as_ref(),
    })?;

    let mut files: Vec<(&str, String)> = Vec::new();
    if let Some(path) = &options.csv {
        files.push((path.as_str(), bom_csv(&bom)));
    }
    if let Some(path) = &options.json_file {
        files.push((path.as_str(), bom_json(&bom)));
    }
    if let Some(path) = &options.html {
        files.push((path.as_str(), bom_html(&bom, options.lang)));
    }
    write_new_files(&files)?;

    let text = if options.json {
        bom_json(&bom)
    } else {
        let mut text = variant_note;
        text.push_str(&bom_text(&bom, options.lang));
        for (path, _) in &files {
            text.push_str(&format!("written: {path}\n"));
        }
        if let Some(link) = &link {
            text.push_str(&link.to_text(options.lang));
        }
        text
    };
    let _ = out.write_all(text.as_bytes());
    let (errors, _, _) = counts(&bom.findings);
    Ok(i32::from(errors > 0))
}

pub(crate) fn bom_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    match bom(&options, out) {
        Ok(code) => code,
        Err(error) => fail(err, &error, options.lang),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_bom::codes::SETTINGS_INVALID;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o =
            parse_options(&["--boards", "25", "--csv", "a.csv", "--lang", "pt", "b.json"]).unwrap();
        assert_eq!(o.boards, Some(25));
        assert_eq!(o.csv.as_deref(), Some("a.csv"));
        assert_eq!(o.lang, Lang::Pt);
        assert_eq!(o.files, vec!["b.json".to_owned()]);
        for bad in [
            vec!["--boards", "0"],
            vec!["--boards", "x"],
            vec!["--boards"],
            vec!["--lang", "xx"],
            vec!["--nope"],
            vec!["--csv"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
    }

    #[test]
    fn a_bad_settings_file_is_a_settings_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("costs.cfg");
        std::fs::write(&path, "nope = 1\n").unwrap();
        let e = load_settings(path.to_str()).unwrap_err();
        assert_eq!(e.code, SETTINGS_INVALID);
    }
}
