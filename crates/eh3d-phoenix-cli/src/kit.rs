// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `kit`: the starter kit that comes with the releases. `phoenix kit list` shows the
//! pieces of its catalogue (`manifest.json`), `phoenix kit info` one piece, and `phoenix kit copy`
//! copies pieces to a folder of the person. Nothing is ever written over, and the kit itself is
//! only read.
//!
//! Exit codes: 0 done, 2 the arguments, the catalogue, a file or the destination are unusable.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use eh3d_phoenix_rules::Lang;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::kit_codes::{self, title};

/// The format of the catalogue.
const FORMAT: &str = "phoenix-kit/1";
/// The name of the catalogue inside the folder of the kit.
const CATALOGUE: &str = "manifest.json";
/// The settings file that is read when `--config` is not given and the file exists.
const DEFAULT_CONFIG: &str = "config/kit.cfg";
/// The languages of the titles of the catalogue, in the order of `Lang::ALL`.
const TAGS: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];
/// The kinds of piece.
const KINDS: [&str; 7] = [
    "library",
    "board",
    "schematic",
    "program",
    "rules",
    "dfm",
    "export-profile",
];

pub(crate) const USAGE: &str =
    "phoenix kit list [--kit FOLDER] [--type TYPE] [--config FILE] [--json] [--lang xx]
phoenix kit info [--kit FOLDER] [--config FILE] [--json] [--lang xx] <identifier>
phoenix kit copy [--kit FOLDER] [--config FILE] [--all] --out FOLDER [<identifier>...]

  kit           the starter kit: libraries, sample boards and schematics, user programs, rule and
                output profiles made for this program (GPL-3.0-or-later, not verified with any
                maker). list shows the pieces of its catalogue (manifest.json), all of them or
                those of one TYPE (library, board, schematic, program, rules, dfm or
                export-profile); info shows the licence, origin, formats and files of one piece;
                copy copies the pieces named (or --all) under --out with the folders of the kit,
                after checking the SHA-256 of every file, and copies nothing when one destination
                is already there. The kit is the folder of --kit, else `kit_dir` of the settings,
                else `kit` where the program runs, else `kit` next to the program. --config reads
                the limits (default: config/kit.cfg when it exists). Codes PHOENIX-KIT-<NNN>.
                Exit code 0 done, 2 not done
";

/// Labels of `info`: licence, origin, verified, format, exports, runs on, files, yes, no, in the
/// eight languages.
const LABELS: [[&str; 8]; 9] = [
    [
        "Licence",
        "Licencia",
        "Lizenz",
        "Licence",
        "Licenza",
        "ライセンス",
        "许可证",
        "Licença",
    ],
    [
        "Origin",
        "Procedencia",
        "Herkunft",
        "Provenance",
        "Provenienza",
        "出所",
        "来源",
        "Proveniência",
    ],
    [
        "Verified",
        "Verificado",
        "Geprüft",
        "Vérifié",
        "Verificato",
        "検証済み",
        "已验证",
        "Verificado",
    ],
    [
        "Format",
        "Formato",
        "Format der Datei",
        "Format du fichier",
        "Formato",
        "形式",
        "格式",
        "Formato",
    ],
    [
        "Can be exported to",
        "Se puede exportar a",
        "Exportierbar nach",
        "Exportable vers",
        "Esportabile in",
        "エクスポート先",
        "可导出为",
        "Pode ser exportado para",
    ],
    [
        "Runs on",
        "Se ejecuta sobre",
        "Läuft auf",
        "S'exécute sur",
        "Si esegue su",
        "実行対象",
        "运行对象",
        "Executa sobre",
    ],
    [
        "Files",
        "Ficheros",
        "Dateien",
        "Fichiers",
        "File",
        "ファイル",
        "文件",
        "Ficheiros",
    ],
    ["yes", "sí", "ja", "oui", "sì", "はい", "是", "sim"],
    ["no", "no", "nein", "non", "no", "いいえ", "否", "não"],
];

/// What a copy says at the end, with the number of files.
const COPIED: [&str; 8] = [
    "files copied",
    "ficheros copiados",
    "Dateien kopiert",
    "fichiers copiés",
    "file copiati",
    "個のファイルをコピーしました",
    "个文件已复制",
    "ficheiros copiados",
];

/// The limits and options of the command.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Settings {
    max_manifest_bytes: u64,
    max_file_bytes: u64,
    max_items: usize,
    verify_sha256: bool,
    kit_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_manifest_bytes: 1_048_576,
            max_file_bytes: 4_194_304,
            max_items: 1000,
            verify_sha256: true,
            kit_dir: String::new(),
        }
    }
}

/// A failure: a code and what is wrong.
#[derive(Debug)]
struct Fault {
    code: &'static str,
    detail: String,
}

fn fault(code: &'static str, detail: impl Into<String>) -> Fault {
    Fault {
        code,
        detail: detail.into(),
    }
}

/// One file of a piece.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KitFile {
    path: String,
    sha256: String,
}

/// One piece of the kit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Piece {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    format: String,
    license: String,
    provenance: String,
    verified: bool,
    exports: Vec<String>,
    title: BTreeMap<String, String>,
    files: Vec<KitFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runs_on: Option<String>,
}

/// The catalogue.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogue {
    format: String,
    license: String,
    provenance: String,
    items: Vec<Piece>,
}

fn parse_settings(text: &str) -> Result<Settings, Fault> {
    let mut settings = Settings::default();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = || {
            fault(
                kit_codes::OPTION_INVALID,
                format!("line {}: {line}", number + 1),
            )
        };
        let (key, value) = line.split_once('=').ok_or_else(bad)?;
        let (key, value) = (key.trim(), value.trim());
        let count = |least: u64, most: u64| -> Result<u64, Fault> {
            value
                .parse::<u64>()
                .ok()
                .filter(|n| (least..=most).contains(n))
                .ok_or_else(bad)
        };
        match key {
            "max_manifest_bytes" => settings.max_manifest_bytes = count(1024, 67_108_864)?,
            "max_file_bytes" => settings.max_file_bytes = count(1024, 268_435_456)?,
            "max_items" => {
                settings.max_items = usize::try_from(count(1, 100_000)?).map_err(|_| bad())?;
            }
            "verify_sha256" => settings.verify_sha256 = value.parse::<bool>().map_err(|_| bad())?,
            "kit_dir" => settings.kit_dir = value.to_owned(),
            _ => return Err(bad()),
        }
    }
    Ok(settings)
}

/// Whether a path of the catalogue is a plain relative path with `/` that stays inside the kit.
fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && eh3d_phoenix_model::output_path_problem(path).is_none()
}

/// At most `max` bytes of a plain file: not a link, not a folder, not larger.
fn read_plain(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("not a plain file".to_owned());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err(format!("larger than {max} bytes"));
    }
    Ok(bytes)
}

fn check_catalogue(catalogue: &Catalogue, settings: &Settings) -> Result<(), Fault> {
    let invalid = |what: String| fault(kit_codes::CATALOGUE_INVALID, what);
    if catalogue.format != FORMAT {
        return Err(invalid(format!("format {:?}", catalogue.format)));
    }
    if catalogue.license.is_empty() || catalogue.provenance.is_empty() {
        return Err(invalid("no licence or origin".to_owned()));
    }
    if catalogue.items.len() > settings.max_items {
        return Err(invalid(format!("more than {} pieces", settings.max_items)));
    }
    let mut ids = BTreeSet::new();
    for piece in &catalogue.items {
        let plain_id = !piece.id.is_empty()
            && piece
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !plain_id || !ids.insert(piece.id.as_str()) {
            return Err(invalid(format!("identifier {:?}", piece.id)));
        }
        if !KINDS.contains(&piece.kind.as_str()) {
            return Err(invalid(format!("{}: type {:?}", piece.id, piece.kind)));
        }
        if TAGS.iter().any(|t| !piece.title.contains_key(*t)) {
            return Err(invalid(format!("{}: a title is missing", piece.id)));
        }
        if piece.files.is_empty() {
            return Err(invalid(format!("{}: no file", piece.id)));
        }
        for file in &piece.files {
            if !safe_relative(&file.path) {
                return Err(invalid(format!("{}: path {:?}", piece.id, file.path)));
            }
            if file.sha256.len() != 64 || !file.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(invalid(format!("{}: SHA-256 of {}", piece.id, file.path)));
            }
        }
    }
    Ok(())
}

/// The folder of the kit: the one given, the one of the settings, `kit` here, or `kit` next to the
/// program.
fn kit_folder(given: Option<&str>, settings: &Settings) -> PathBuf {
    if let Some(folder) = given {
        return PathBuf::from(folder);
    }
    if !settings.kit_dir.is_empty() {
        return PathBuf::from(&settings.kit_dir);
    }
    let here = PathBuf::from("kit");
    if here.join(CATALOGUE).is_file() {
        return here;
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|folder| folder.join("kit")))
        .filter(|folder| folder.join(CATALOGUE).is_file())
        .unwrap_or(here)
}

fn load_catalogue(folder: &Path, settings: &Settings) -> Result<Catalogue, Fault> {
    let path = folder.join(CATALOGUE);
    let unreadable = |why: String| {
        fault(
            kit_codes::CATALOGUE_UNREADABLE,
            format!("{}: {why}", path.display()),
        )
    };
    let bytes = read_plain(&path, settings.max_manifest_bytes).map_err(unreadable)?;
    let catalogue: Catalogue = serde_json::from_slice(&bytes)
        .map_err(|e| fault(kit_codes::CATALOGUE_INVALID, e.to_string()))?;
    check_catalogue(&catalogue, settings)?;
    Ok(catalogue)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    List,
    Info,
    Copy,
}

struct Options {
    mode: Mode,
    kit: Option<String>,
    out: Option<String>,
    kind: Option<String>,
    config: Option<String>,
    all: bool,
    json: bool,
    lang: Lang,
    ids: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, Fault> {
    let bad = |what: &str| fault(kit_codes::OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    let mode = match iter.next() {
        Some(&"list") => Mode::List,
        Some(&"info") => Mode::Info,
        Some(&"copy") => Mode::Copy,
        _ => return Err(bad("the first word is `list`, `info` or `copy`")),
    };
    let mut options = Options {
        mode,
        kit: None,
        out: None,
        kind: None,
        config: None,
        all: false,
        json: false,
        lang: Lang::En,
        ids: Vec::new(),
    };
    while let Some(arg) = iter.next() {
        let mut value = || -> Result<String, Fault> {
            iter.next().map(|v| (*v).to_owned()).ok_or_else(|| bad(arg))
        };
        match *arg {
            "--kit" => options.kit = Some(value()?),
            "--config" => options.config = Some(value()?),
            "--out" if mode == Mode::Copy => options.out = Some(value()?),
            "--all" if mode == Mode::Copy => options.all = true,
            "--type" if mode == Mode::List => {
                let kind = value()?;
                if !KINDS.contains(&kind.as_str()) {
                    return Err(bad(&format!("--type {kind}")));
                }
                options.kind = Some(kind);
            }
            "--json" if mode != Mode::Copy => options.json = true,
            "--lang" => {
                let tag = value()?;
                options.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            other if other.starts_with("--") => return Err(bad(other)),
            other => options.ids.push(other.to_owned()),
        }
    }
    let right = match mode {
        Mode::List => options.ids.is_empty(),
        Mode::Info => options.ids.len() == 1,
        Mode::Copy => options.out.is_some() && options.all == options.ids.is_empty(),
    };
    if !right {
        return Err(bad(match mode {
            Mode::List => "`list` takes no identifier",
            Mode::Info => "`info` takes one identifier",
            Mode::Copy => "`copy` takes --out FOLDER and identifiers or --all",
        }));
    }
    Ok(options)
}

fn load_settings(path: Option<&str>) -> Result<Settings, Fault> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_CONFIG)
            .is_file()
            .then_some(DEFAULT_CONFIG)
    });
    match path {
        None => Ok(Settings::default()),
        Some(path) => {
            let text = crate::bounded::read_config_text(path)
                .map_err(|e| fault(kit_codes::OPTION_INVALID, format!("{path}: {e}")))?;
            parse_settings(&text)
        }
    }
}

fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

fn title_of(piece: &Piece, lang: Lang) -> &str {
    piece
        .title
        .get(TAGS[column(lang)])
        .map_or("", String::as_str)
}

fn find<'a>(catalogue: &'a Catalogue, id: &str) -> Result<&'a Piece, Fault> {
    catalogue
        .items
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| fault(kit_codes::UNKNOWN_PIECE, id.to_owned()))
}

fn list(catalogue: &Catalogue, options: &Options) -> String {
    let pieces: Vec<&Piece> = catalogue
        .items
        .iter()
        .filter(|p| options.kind.as_deref().is_none_or(|k| p.kind == k))
        .collect();
    if options.json {
        let value = serde_json::json!({ "format": "phoenix-kit-list/1", "items": pieces });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        return text;
    }
    let width = pieces.iter().map(|p| p.id.len()).max().unwrap_or(0);
    let mut text = String::new();
    for p in &pieces {
        text.push_str(&format!(
            "{:<width$}  {:<14}  {}\n",
            p.id,
            p.kind,
            title_of(p, options.lang)
        ));
    }
    text.push_str(&format!(
        "{} / {}  {}  {}\n",
        pieces.len(),
        catalogue.items.len(),
        catalogue.license,
        catalogue.provenance
    ));
    text
}

fn info(piece: &Piece, options: &Options) -> String {
    if options.json {
        let mut text = serde_json::to_string_pretty(piece).unwrap_or_default();
        text.push('\n');
        return text;
    }
    let c = column(options.lang);
    let label = |i: usize| LABELS[i][c];
    let mut text = format!(
        "{}  {}  {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n",
        piece.id,
        piece.kind,
        title_of(piece, options.lang),
        label(0),
        piece.license,
        label(1),
        piece.provenance,
        label(2),
        label(if piece.verified { 7 } else { 8 }),
        label(3),
        piece.format,
    );
    if !piece.exports.is_empty() {
        text.push_str(&format!("{}: {}\n", label(4), piece.exports.join(", ")));
    }
    if let Some(input) = &piece.runs_on {
        text.push_str(&format!("{}: {input}\n", label(5)));
    }
    text.push_str(&format!("{}:\n", label(6)));
    for file in &piece.files {
        text.push_str(&format!("  {}  {}\n", file.sha256, file.path));
    }
    text
}

/// Reads and checks every file of the pieces first, and writes only when no destination is there.
fn copy(
    catalogue: &Catalogue,
    folder: &Path,
    options: &Options,
    settings: &Settings,
) -> Result<String, Fault> {
    let pieces: Vec<&Piece> = if options.all {
        catalogue.items.iter().collect()
    } else {
        options
            .ids
            .iter()
            .map(|id| find(catalogue, id))
            .collect::<Result<_, _>>()?
    };
    let out = PathBuf::from(options.out.as_deref().unwrap_or_default());
    let mut wanted: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
    for piece in &pieces {
        for file in &piece.files {
            let source = folder.join(&file.path);
            let bytes = read_plain(&source, settings.max_file_bytes).map_err(|why| {
                fault(
                    kit_codes::FILE_UNUSABLE,
                    format!("{}: {why}", source.display()),
                )
            })?;
            if settings.verify_sha256 {
                let digest: String = Sha256::digest(&bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                if !digest.eq_ignore_ascii_case(&file.sha256) {
                    return Err(fault(kit_codes::FILE_CHANGED, file.path.clone()));
                }
            }
            wanted.insert(&file.path, bytes);
        }
    }
    for path in wanted.keys() {
        let target = out.join(path);
        if fs::symlink_metadata(&target).is_ok() {
            return Err(fault(
                kit_codes::DESTINATION_EXISTS,
                target.display().to_string(),
            ));
        }
    }
    let mut text = String::new();
    for (path, bytes) in &wanted {
        let target = out.join(path);
        let failed =
            |e: std::io::Error| fault(kit_codes::COPY_FAILED, format!("{}: {e}", target.display()));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(failed)?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .and_then(|mut file| file.write_all(bytes))
            .map_err(failed)?;
        text.push_str(&format!("{path}\n"));
    }
    text.push_str(&format!(
        "{} {}\n",
        wanted.len(),
        COPIED[column(options.lang)]
    ));
    Ok(text)
}

fn run(options: &Options) -> Result<String, Fault> {
    let settings = load_settings(options.config.as_deref())?;
    let folder = kit_folder(options.kit.as_deref(), &settings);
    let catalogue = load_catalogue(&folder, &settings)?;
    match options.mode {
        Mode::List => Ok(list(&catalogue, options)),
        Mode::Info => Ok(info(find(&catalogue, &options.ids[0])?, options)),
        Mode::Copy => copy(&catalogue, &folder, options, &settings),
    }
}

pub(crate) fn kit_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(f) => {
            let _ = err.write_all(USAGE.as_bytes());
            if !args.is_empty() {
                let _ = writeln!(
                    err,
                    "error: {}: {}: {}",
                    f.code,
                    title(f.code, Lang::En),
                    f.detail
                );
            }
            return 2;
        }
    };
    match run(&options) {
        Ok(text) => {
            let _ = out.write_all(text.as_bytes());
            0
        }
        Err(f) => {
            let _ = writeln!(
                err,
                "error: {}: {}: {}",
                f.code,
                title(f.code, options.lang),
                f.detail
            );
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_that_ship_are_the_defaults() {
        let shipped = parse_settings(include_str!("../../../config/kit.cfg")).unwrap();
        assert_eq!(shipped, Settings::default());
        for bad in [
            "max_items = 0",
            "verify_sha256 = maybe",
            "other = 1",
            "alone",
        ] {
            assert!(parse_settings(bad).is_err(), "{bad}");
        }
        assert!(
            !parse_settings("verify_sha256 = false")
                .unwrap()
                .verify_sha256
        );
    }

    #[test]
    fn only_plain_relative_paths_are_accepted() {
        for good in ["libraries/passives.lbr", "manifest.json"] {
            assert!(safe_relative(good), "{good}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../b",
            "a//b",
            "./a",
            "a\\b",
            "C:/x",
            "boards/NUL",
        ] {
            assert!(!safe_relative(bad), "{bad:?}");
        }
    }
}
