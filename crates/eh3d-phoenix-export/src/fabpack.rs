//! The fabrication package: the files a manufacturer asks for (the Gerber and Excellon files of a
//! profile, the pick and place files, the bills of materials and the stencil) in one deterministic
//! `.zip` file, or in a new folder, with a manifest `phoenix-fab-package/1`.
//!
//! The manifest lists every file with its size and SHA-256, the version of this program, the
//! profile, the unit of the coordinates and the notes about what the files cannot say. The files
//! are in the order of their paths, the manifest is the last entry, and the package is read back
//! and checked against the manifest before it is returned ([`build_pack`]), so a package that is
//! returned is a package whose manifest agrees with its files.
//!
//! Nothing here reads the clock or makes a random number: the same files give the same bytes.

use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub use eh3d_container::sha256_hex;

use crate::fabcodes::{FabError, FABPACK_FAILED, FABPACK_MISMATCH, FABPACK_OUTPUT_FAILED};
use crate::zipfile::{check_name, read_zip, write_zip};

/// The format tag of the manifest.
pub const FORMAT: &str = "phoenix-fab-package/1";

/// The name of the manifest inside the package.
pub const MANIFEST_NAME: &str = "manifest.json";

/// What goes into a package.
#[derive(Debug, Clone)]
pub struct PackInput {
    /// The name of the board.
    pub board: String,
    /// The name of the export profile.
    pub profile: String,
    /// The unit of the coordinates of the pick and place files.
    pub units: String,
    /// The files: their paths inside the package and their bytes.
    pub files: Vec<(String, Vec<u8>)>,
    /// What the files cannot say, one note for each line.
    pub notes: Vec<String>,
}

/// A package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// The bytes of the zip file.
    pub zip: Vec<u8>,
    /// The text of the manifest.
    pub manifest: String,
    /// The files in the order of the package, the manifest last.
    pub files: Vec<(String, Vec<u8>)>,
}

/// What reading a package back found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackReport {
    /// The name of the board in the manifest.
    pub board: String,
    /// The profile in the manifest.
    pub profile: String,
    /// The number of files listed by the manifest.
    pub files: usize,
}

/// The notes that every package carries: what the files cannot say.
pub fn standard_notes() -> Vec<String> {
    vec![
        "The files were made by a prototype and have never been sent to a manufacturer or loaded in a machine.".to_owned(),
        "A copper zone is written as its outline, not poured.".to_owned(),
        "The coordinates of the pick and place files are in the unit and from the origin named in them; the rotation of the bottom face follows the profile.".to_owned(),
        "The stencil openings are reduced and split by the settings, which are not the rules of any stencil maker.".to_owned(),
    ]
}

fn sorted_files(input: &PackInput) -> Result<Vec<(String, Vec<u8>)>, FabError> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (path, _) in &input.files {
        check_name(path)?;
        if path == MANIFEST_NAME {
            return Err(FabError::new(
                FABPACK_FAILED,
                format!("{MANIFEST_NAME} is written by the package"),
            ));
        }
        if !seen.insert(path.to_lowercase()) {
            return Err(FabError::new(
                FABPACK_FAILED,
                format!("the file '{path}' is given twice"),
            ));
        }
    }
    let mut files = input.files.clone();
    files.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(files)
}

/// The text of the manifest of the files of `input`.
pub fn manifest_of(input: &PackInput) -> Result<String, FabError> {
    let files = sorted_files(input)?;
    let listed: Vec<serde_json::Value> = files
        .iter()
        .map(|(path, bytes)| {
            serde_json::json!({
                "path": path,
                "bytes": bytes.len(),
                "sha256": sha256_hex(bytes),
            })
        })
        .collect();
    let value = serde_json::json!({
        "format": FORMAT,
        "software": format!("EH3D PHOENIX-PCB {}", env!("CARGO_PKG_VERSION")),
        "board": input.board,
        "profile": input.profile,
        "units": input.units,
        "files": listed,
        "notes": input.notes,
    });
    let mut text = serde_json::to_string_pretty(&value)
        .map_err(|e| FabError::new(FABPACK_FAILED, e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Reads a package back and checks it against its manifest: the format, every listed file with its
/// size and SHA-256, and that the package holds nothing else.
pub fn verify_pack(zip: &[u8]) -> Result<PackReport, FabError> {
    let mismatch = |what: String| FabError::new(FABPACK_MISMATCH, what);
    let entries = read_zip(zip)?;
    let manifest = entries
        .iter()
        .find(|(name, _)| name == MANIFEST_NAME)
        .ok_or_else(|| mismatch(format!("the package has no {MANIFEST_NAME}")))?;
    let value: serde_json::Value = serde_json::from_slice(&manifest.1)
        .map_err(|e| mismatch(format!("{MANIFEST_NAME}: {e}")))?;
    if value["format"] != FORMAT {
        return Err(mismatch(format!(
            "{MANIFEST_NAME} is not of the format {FORMAT}"
        )));
    }
    let listed = value["files"]
        .as_array()
        .ok_or_else(|| mismatch(format!("{MANIFEST_NAME} has no list of files")))?;
    let mut named: BTreeSet<&str> = BTreeSet::new();
    for item in listed {
        let path = item["path"]
            .as_str()
            .ok_or_else(|| mismatch("a listed file has no path".to_owned()))?;
        let entry = entries
            .iter()
            .find(|(name, _)| name == path)
            .ok_or_else(|| mismatch(format!("{path} is listed and is not in the package")))?;
        if item["bytes"].as_u64() != Some(entry.1.len() as u64) {
            return Err(mismatch(format!(
                "the size of {path} differs from the manifest"
            )));
        }
        if item["sha256"].as_str() != Some(sha256_hex(&entry.1).as_str()) {
            return Err(mismatch(format!(
                "the SHA-256 of {path} differs from the manifest"
            )));
        }
        if !named.insert(path) {
            return Err(mismatch(format!("{path} is listed twice")));
        }
    }
    for (name, _) in &entries {
        if name != MANIFEST_NAME && !named.contains(name.as_str()) {
            return Err(mismatch(format!(
                "{name} is in the package and not in the manifest"
            )));
        }
    }
    Ok(PackReport {
        board: value["board"].as_str().unwrap_or("").to_owned(),
        profile: value["profile"].as_str().unwrap_or("").to_owned(),
        files: listed.len(),
    })
}

/// Makes the package and reads it back.
pub fn build_pack(input: &PackInput) -> Result<Pack, FabError> {
    let mut files = sorted_files(input)?;
    let manifest = manifest_of(input)?;
    files.push((MANIFEST_NAME.to_owned(), manifest.clone().into_bytes()));
    let zip = write_zip(&files)?;
    let report = verify_pack(&zip)?;
    if report.files + 1 != files.len() {
        return Err(FabError::new(
            FABPACK_MISMATCH,
            "the manifest does not list every file",
        ));
    }
    Ok(Pack {
        zip,
        manifest,
        files,
    })
}

fn output_failed(path: &Path, e: &std::io::Error) -> FabError {
    FabError::new(FABPACK_OUTPUT_FAILED, format!("{}: {e}", path.display()))
}

/// True when the destination names a zip file (`.zip` in any case); anything else is a folder.
pub fn is_zip_destination(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

/// Writes the package to a destination that does not exist: a new `.zip` file, or a new folder with
/// the files and the manifest. A destination that exists is never touched.
pub fn write_new(dest: &Path, pack: &Pack) -> Result<(), FabError> {
    if dest.exists() {
        return Err(FabError::new(
            FABPACK_OUTPUT_FAILED,
            format!("{} exists and is never replaced", dest.display()),
        ));
    }
    if is_zip_destination(dest) {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)
            .map_err(|e| output_failed(dest, &e))?;
        return file
            .write_all(&pack.zip)
            .map_err(|e| output_failed(dest, &e));
    }
    std::fs::create_dir_all(dest).map_err(|e| output_failed(dest, &e))?;
    for (name, bytes) in &pack.files {
        let target = dest.join(name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| output_failed(parent, &e))?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| output_failed(&target, &e))?;
        file.write_all(bytes)
            .map_err(|e| output_failed(&target, &e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PackInput {
        PackInput {
            board: "demo".to_owned(),
            profile: "generic".to_owned(),
            units: "mm".to_owned(),
            files: vec![
                ("pnp/pnp_top.csv".to_owned(), b"Ref\nR1\n".to_vec()),
                ("gerber/top.gbr".to_owned(), b"M02*\n".to_vec()),
                ("bom.csv".to_owned(), b"a,b\n".to_vec()),
            ],
            notes: vec!["a note".to_owned()],
        }
    }

    #[test]
    fn the_package_lists_its_files_in_order_with_their_sums() {
        let pack = build_pack(&input()).unwrap();
        let names: Vec<&str> = pack.files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "bom.csv",
                "gerber/top.gbr",
                "pnp/pnp_top.csv",
                "manifest.json"
            ]
        );
        let value: serde_json::Value = serde_json::from_str(&pack.manifest).unwrap();
        assert_eq!(value["format"], FORMAT);
        assert_eq!(value["board"], "demo");
        assert_eq!(value["profile"], "generic");
        assert_eq!(value["units"], "mm");
        assert_eq!(value["files"][0]["path"], "bom.csv");
        assert_eq!(value["files"][0]["bytes"], 4);
        assert_eq!(value["files"][0]["sha256"], sha256_hex(b"a,b\n"));
        assert_eq!(value["notes"][0], "a note");
        assert!(value["software"]
            .as_str()
            .unwrap()
            .starts_with("EH3D PHOENIX-PCB "));
        assert!(pack.manifest.ends_with("}\n"));
    }

    #[test]
    fn the_same_files_give_the_same_bytes_in_any_order() {
        let one = build_pack(&input()).unwrap();
        let mut reversed = input();
        reversed.files.reverse();
        let two = build_pack(&reversed).unwrap();
        assert_eq!(one.zip, two.zip);
        assert_eq!(one.manifest, two.manifest);
    }

    #[test]
    fn a_package_is_read_back_and_a_change_is_found() {
        let pack = build_pack(&input()).unwrap();
        let report = verify_pack(&pack.zip).unwrap();
        assert_eq!(
            report,
            PackReport {
                board: "demo".to_owned(),
                profile: "generic".to_owned(),
                files: 3
            }
        );
        // a file that changed after the manifest was made
        let mut changed = pack.files.clone();
        changed[0].1 = b"a,c\n".to_vec();
        let zip = write_zip(&changed).unwrap();
        let e = verify_pack(&zip).unwrap_err();
        assert_eq!(e.code, FABPACK_MISMATCH);
        assert!(e.detail.contains("bom.csv"), "{}", e.detail);
        // a file that is not in the manifest
        let mut extra = pack.files.clone();
        extra.insert(0, ("extra.txt".to_owned(), b"x".to_vec()));
        let e = verify_pack(&write_zip(&extra).unwrap()).unwrap_err();
        assert!(e.detail.contains("extra.txt"), "{}", e.detail);
        // a file the manifest lists and the package lacks
        let mut missing = pack.files.clone();
        missing.remove(0);
        let e = verify_pack(&write_zip(&missing).unwrap()).unwrap_err();
        assert!(e.detail.contains("bom.csv"), "{}", e.detail);
        // no manifest at all
        let none = write_zip(&[("a.txt".to_owned(), b"x".to_vec())]).unwrap();
        assert!(verify_pack(&none).is_err());
    }

    #[test]
    fn names_that_cannot_be_in_a_package_are_refused() {
        for bad in ["../x", "manifest.json", "a\\b", "/abs"] {
            let mut i = input();
            i.files.push((bad.to_owned(), Vec::new()));
            assert_eq!(build_pack(&i).unwrap_err().code, FABPACK_FAILED, "{bad}");
        }
        let mut twice = input();
        twice.files.push(("BOM.csv".to_owned(), Vec::new()));
        assert!(build_pack(&twice).is_err());
    }

    #[test]
    fn a_package_is_written_to_a_new_zip_or_a_new_folder_only() {
        let dir = tempfile::tempdir().unwrap();
        let pack = build_pack(&input()).unwrap();
        let zip_path = dir.path().join("out.ZIP");
        write_new(&zip_path, &pack).unwrap();
        assert_eq!(std::fs::read(&zip_path).unwrap(), pack.zip);
        assert_eq!(
            write_new(&zip_path, &pack).unwrap_err().code,
            FABPACK_OUTPUT_FAILED
        );
        let folder = dir.path().join("out");
        write_new(&folder, &pack).unwrap();
        assert_eq!(
            std::fs::read(folder.join("gerber").join("top.gbr")).unwrap(),
            b"M02*\n"
        );
        assert_eq!(
            std::fs::read_to_string(folder.join(MANIFEST_NAME)).unwrap(),
            pack.manifest
        );
        assert_eq!(
            write_new(&folder, &pack).unwrap_err().code,
            FABPACK_OUTPUT_FAILED
        );
        assert!(is_zip_destination(Path::new("a/b.zip")));
        assert!(!is_zip_destination(Path::new("a/b")));
    }

    #[test]
    fn the_standard_notes_are_not_empty() {
        assert!(standard_notes().iter().all(|n| !n.is_empty()));
    }
}
