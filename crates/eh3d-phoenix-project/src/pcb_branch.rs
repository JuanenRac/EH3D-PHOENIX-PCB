// SPDX-License-Identifier: GPL-3.0-or-later
//! The branch `pcb/` of the `.eh3d` project of the suite (`eh3d-project`): what this program keeps
//! in the one file that the programs of the suite share.
//!
//! The branch holds `board.json` and `schematic.json` (the canonical JSON of the model) and, for a
//! design that came from a file of another program, that file under `origin/` with the report of
//! what was read. Writing changes only the branch `pcb/`: the branches of the other programs, and
//! the files of this branch that this version does not know, are kept byte for byte.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use eh3d_container::{sha256_hex, ContainerError};
use eh3d_project::meta::{ConversionReport, Converter, Counted, SourceFile, REPORT_FILE};
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Writer};

/// The name of the branch of this program.
pub const PCB_BRANCH: &str = "pcb";
/// The version of the format of the branch that this program writes.
pub const PCB_FORMAT: &str = "1.0";
/// The newest major number of the format that this program understands.
const PCB_MAJOR: u64 = 1;
/// The board inside the branch.
pub const BOARD_FILE: &str = "board.json";
/// The schematic inside the branch.
pub const SCHEMATIC_FILE: &str = "schematic.json";
/// The file extension of a project.
pub const EXTENSION: &str = "eh3d";
/// The largest project that is copied to a new name, in bytes.
const MAX_COPY_BYTES: u64 = 1 << 30;

/// The file of another program that a design came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcbOrigin {
    /// The name of the file as it was.
    pub name: String,
    /// The format, for example `eagle-brd`.
    pub format: String,
    /// The file as it was read.
    pub bytes: Vec<u8>,
    /// How many parts of it were not read or were read in a simpler form.
    pub not_read: u64,
}

/// What the branch holds, or what a save puts into it. A part that is `None` when saving is
/// left as the project has it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PcbParts {
    /// The board, as the canonical JSON of the model.
    pub board: Option<Vec<u8>>,
    /// The schematic, as the canonical JSON of the model.
    pub schematic: Option<Vec<u8>>,
    /// The file the design came from.
    pub origin: Option<PcbOrigin>,
}

/// What a new project is called, when the file that is saved is not there yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewPcbProject {
    /// The identifier; one is made from the content when it is empty.
    pub id: String,
    /// The name.
    pub name: String,
    /// When it was made, as the caller writes it.
    pub created: Option<String>,
}

/// Why the branch could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum PcbError {
    /// The container refused the file or the write; its code is `EH3D-PKG-NNN`.
    #[error(transparent)]
    Container(#[from] ContainerError),
    /// The project has no branch `pcb/`.
    #[error("the project has no branch pcb/: no program of boards has written it yet")]
    NoBranch,
    /// The branch was written by a newer version of the format; it is left as it is.
    #[error("the branch pcb/ has the format {0}, which is newer than this program writes")]
    Newer(String),
}

impl From<io::Error> for PcbError {
    fn from(e: io::Error) -> Self {
        PcbError::Container(ContainerError::from(e))
    }
}

/// The format of the branch of an open project, refused when it is newer than this program.
fn check_format(project: &Project) -> Result<bool, PcbError> {
    let Some(branch) = project.branch(PCB_BRANCH) else {
        return Ok(false);
    };
    if let Some(record) = &branch.record {
        let major = record
            .format
            .split('.')
            .next()
            .and_then(|m| m.parse::<u64>().ok());
        if major.is_none_or(|m| m > PCB_MAJOR) {
            return Err(PcbError::Newer(record.format.clone()));
        }
    }
    Ok(true)
}

/// Every file of the branch except its record.
fn files(project: &mut Project) -> Result<BTreeMap<String, Vec<u8>>, PcbError> {
    let names: Vec<String> = project.branch(PCB_BRANCH).map_or_else(Vec::new, |branch| {
        branch
            .files
            .keys()
            .filter(|n| n.as_str() != "branch.json")
            .cloned()
            .collect()
    });
    let mut out = BTreeMap::new();
    for name in names {
        let bytes = project.read_entry(PCB_BRANCH, &name)?;
        out.insert(name, bytes);
    }
    Ok(out)
}

/// Reads the branch `pcb/` of a project. The whole file is checked against its integrity record.
pub fn read_pcb(path: &Path) -> Result<PcbParts, PcbError> {
    let mut project = Project::open(path, &OpenOptions::default())?;
    if !check_format(&project)? {
        return Err(PcbError::NoBranch);
    }
    let mut files = files(&mut project)?;
    let origin = files
        .get(REPORT_FILE)
        .and_then(|bytes| serde_json::from_slice::<ConversionReport>(bytes).ok())
        .and_then(|report| {
            let bytes = files.get(report.original.as_deref()?)?.clone();
            Some(PcbOrigin {
                name: report.source.name,
                format: report.source.format,
                bytes,
                not_read: report.not_read.iter().map(|c| c.count).sum(),
            })
        });
    Ok(PcbParts {
        board: files.remove(BOARD_FILE),
        schematic: files.remove(SCHEMATIC_FILE),
        origin,
    })
}

/// The extension of the name of a file, in lower case, when it is a plain one.
fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.len() <= 16 && e.bytes().all(|b| b.is_ascii_alphanumeric()))
        .map_or_else(|| "bin".to_owned(), str::to_ascii_lowercase)
}

/// The files of the branch that keep an original, and the report of what was read of it.
fn put_origin(
    files: &mut BTreeMap<String, Vec<u8>>,
    origin: &PcbOrigin,
    app_version: &str,
) -> Result<(), PcbError> {
    files.retain(|name, _| !name.starts_with("origin/"));
    let kept = format!("origin/source.{}", extension_of(&origin.name));
    let report = ConversionReport {
        schema: 1,
        source: SourceFile {
            format: origin.format.clone(),
            name: origin.name.clone(),
            sha256: sha256_hex(&origin.bytes),
            size: origin.bytes.len() as u64,
        },
        converter: Converter {
            name: "eh3d-phoenix-pcb".to_owned(),
            version: app_version.to_owned(),
        },
        original: Some(kept.clone()),
        read: Vec::new(),
        not_read: if origin.not_read == 0 {
            Vec::new()
        } else {
            vec![Counted {
                item: "parts".to_owned(),
                count: origin.not_read,
                reason: Some("not read or read in a simpler form".to_owned()),
            }]
        },
        export_losses: Vec::new(),
        extra: BTreeMap::new(),
    };
    let bytes = eh3d_container::to_canonical_file_bytes(&report)
        .map_err(|e| PcbError::from(io::Error::other(e.to_string())))?;
    files.insert(kept, origin.bytes.clone());
    files.insert(REPORT_FILE.to_owned(), bytes);
    Ok(())
}

/// An identifier in the shape of a UUID made from the content, so that the same content gives the
/// same project.
fn id_from(parts: &PcbParts, name: &str) -> String {
    let mut seed = name.as_bytes().to_vec();
    for bytes in [&parts.board, &parts.schematic].into_iter().flatten() {
        seed.extend_from_slice(bytes);
    }
    let hex = sha256_hex(&seed);
    let part = |from: usize, to: usize| hex.get(from..to).unwrap_or("0");
    format!(
        "{}-{}-8{}-8{}-{}",
        part(0, 8),
        part(8, 12),
        part(13, 16),
        part(17, 20),
        part(20, 32)
    )
}

/// Copies a project to a name that is not taken.
fn copy_to_new(source: &Path, target: &Path) -> Result<(), PcbError> {
    let mut bytes = Vec::new();
    fs::File::open(source)?
        .take(MAX_COPY_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_COPY_BYTES {
        return Err(io::Error::other("the project is too large to copy").into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    file.write_all(&bytes)?;
    Ok(())
}

/// Writes the branch `pcb/` of the project at `target`.
///
/// When there is no file at `target`, it is first copied from `source` (a project, so that the
/// branches of the other programs come with it) or made new. The file is then replaced in one
/// piece with the new branch; every other branch keeps its bytes. A part of `parts` that is
/// `None` is left as the project has it.
pub fn write_pcb(
    target: &Path,
    source: Option<&Path>,
    parts: &PcbParts,
    new: &NewPcbProject,
    app_version: &str,
) -> Result<(), PcbError> {
    let options = OpenOptions::default();
    if !target.exists() {
        if let Some(source) = source {
            // refused before anything is written when the source is not a sound project
            check_format(&Project::open(source, &options)?)?;
            copy_to_new(source, target)?;
        } else {
            let name = if new.name.trim().is_empty() {
                "board".to_owned()
            } else {
                new.name.clone()
            };
            Project::create(
                target,
                NewProject {
                    id: if new.id.is_empty() {
                        id_from(parts, &name)
                    } else {
                        new.id.clone()
                    },
                    name,
                    units: "millimetres".to_owned(),
                    created: new.created.clone(),
                },
                &options,
                &SaveOptions::default().atomic,
            )?;
        }
    }
    let mut project = Project::open(target, &options)?;
    check_format(&project)?;
    let mut entries = files(&mut project)?;
    if let Some(board) = &parts.board {
        entries.insert(BOARD_FILE.to_owned(), board.clone());
    }
    if let Some(schematic) = &parts.schematic {
        entries.insert(SCHEMATIC_FILE.to_owned(), schematic.clone());
    }
    if let Some(origin) = &parts.origin {
        put_origin(&mut entries, origin, app_version)?;
    }
    let (depends, extra) = project
        .branch(PCB_BRANCH)
        .and_then(|b| b.record.as_ref())
        .map(|r| (r.depends.clone(), r.extra.clone()))
        .unwrap_or_default();
    let mut update = BranchUpdate::new(PCB_BRANCH, PCB_FORMAT, app_version);
    update.entries = entries;
    update.depends = depends;
    update.extra = extra;
    project.save_branch(
        &Writer::for_app(PCB_BRANCH),
        update,
        &SaveOptions::default(),
    )?;
    Ok(())
}
