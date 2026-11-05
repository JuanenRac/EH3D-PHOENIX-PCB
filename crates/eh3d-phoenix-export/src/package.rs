//! The manufacturing package: an `.eh3dproj` 1.0.0 project with one Gerber X2 file per layer, the
//! Excellon drill files, the placement file and the bill of materials.
//!
//! The export is deterministic: the caller gives the project identity and the time stamp (or asks
//! for [`export_package_reproducible`], which derives both from the content), nothing reads the
//! clock or makes a random number, and every ordering is fixed, so the same board and options give
//! the same package bytes.

use chrono::{DateTime, TimeZone, Utc};
use eh3d_common::project::{LayerKind as PackageLayerKind, LayerRef};
use eh3d_common::{Package, ProjectMetadata};
use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, LayerKind};
use eh3d_phoenix_model::Nm;
use uuid::Uuid;

use crate::codes;
use crate::fab::{build, FabSet, FileRole};
use crate::profile::Profile;

/// What the caller chooses.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// The identity of the exported project.
    pub project_id: Uuid,
    /// The time stamp written as created and modified.
    pub created: DateTime<Utc>,
    /// The width of the line that draws the board outline (used by [`export_package`]; a profile
    /// carries its own).
    pub outline_width: Nm,
}

/// Why the export failed. Each case has a stable code.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// A layer file could not be written.
    #[error("layer file {layer}: {message}")]
    Layer {
        /// The name of the layer.
        layer: String,
        /// What the writer said.
        message: String,
    },
    /// The drill file could not be written.
    #[error("drill file: {0}")]
    Drill(String),
    /// The package could not be assembled.
    #[error("package: {0}")]
    Package(String),
    /// A profile is not valid.
    #[error("profile: {0}")]
    Profile(String),
    /// The profile asked for is not in the profile file.
    #[error("profile '{0}' is not in the profile file")]
    ProfileNotFound(String),
    /// The output could not be written to its folder.
    #[error("output: {0}")]
    Output(String),
}

impl ExportError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            ExportError::Layer { .. } | ExportError::Drill(_) => codes::WRITE_FAILED,
            ExportError::Package(_) => codes::PACKAGE_FAILED,
            ExportError::Profile(_) => codes::PROFILE_INVALID,
            ExportError::ProfileNotFound(_) => codes::PROFILE_NOT_FOUND,
            ExportError::Output(_) => codes::OUTPUT_FAILED,
        }
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        codes::diagnostic(self.code(), Severity::Error, &self.to_string())
    }
}

/// The package and what was found while making it.
#[derive(Debug, Clone)]
pub struct Exported {
    /// The package, ready to write with `write_to_path` or `to_bytes`.
    pub package: Package,
    /// What the files cannot say, in order.
    pub diagnostics: Vec<Diagnostic>,
}

fn package_kind(kind: LayerKind) -> PackageLayerKind {
    match kind {
        LayerKind::Copper => PackageLayerKind::Copper,
        LayerKind::SolderMask => PackageLayerKind::SolderMask,
        LayerKind::Paste => PackageLayerKind::Paste,
        LayerKind::Silkscreen => PackageLayerKind::Silkscreen,
        LayerKind::Outline => PackageLayerKind::Outline,
        LayerKind::Documentation => PackageLayerKind::Other,
    }
}

/// Puts the files of a set in a package. The layers and the drill files are named in the metadata;
/// the placement file and the bill of materials are entries of the package without a layer.
pub fn package_of(set: &FabSet, options: &ExportOptions) -> Result<Package, ExportError> {
    let mut metadata = ProjectMetadata::new(set.name.clone());
    metadata.id = options.project_id;
    metadata.created = options.created;
    metadata.modified = options.created;
    for file in &set.files {
        match &file.role {
            FileRole::Layer { kind, name, .. } => metadata.layers.push(LayerRef {
                path: file.path.clone(),
                kind: package_kind(*kind),
                name: name.clone(),
            }),
            FileRole::Drill { plated } => metadata.layers.push(LayerRef {
                path: file.path.clone(),
                kind: PackageLayerKind::Drill,
                name: if *plated {
                    "Drill (plated)".to_owned()
                } else {
                    "Drill (not plated)".to_owned()
                },
            }),
            FileRole::Placement | FileRole::Bom => {}
        }
    }
    let mut package = Package::new(metadata);
    for file in &set.files {
        package
            .add_entry(&file.path, file.bytes.clone())
            .map_err(|e| ExportError::Package(e.to_string()))?;
    }
    package
        .metadata
        .validate()
        .map_err(|e| ExportError::Package(e.to_string()))?;
    Ok(package)
}

/// Exports the board as a package with a profile.
pub fn export_package_with(
    board: &Board,
    options: &ExportOptions,
    profile: &Profile,
) -> Result<Exported, ExportError> {
    let set = build(board, profile)?;
    let package = package_of(&set, options)?;
    Ok(Exported {
        package,
        diagnostics: set.diagnostics,
    })
}

/// Exports the board as a package with the default profile and the outline width of the options.
pub fn export_package(board: &Board, options: &ExportOptions) -> Result<Exported, ExportError> {
    let mut profile = Profile::default();
    profile.gerber.outline_width_nm = options.outline_width.get();
    export_package_with(board, options, &profile)
}

/// Exports the board as a package whose identity and time stamp come from its content: the
/// identity is the first 128 bits of the SHA-256 of the manifest and the time stamp is the start of
/// the Unix epoch. The same board and profile give the same bytes anywhere, any time.
pub fn export_package_reproducible(
    board: &Board,
    profile: &Profile,
) -> Result<Exported, ExportError> {
    let set = build(board, profile)?;
    let digest = eh3d_container::sha256_hex(&set.manifest()?);
    let id =
        u128::from_str_radix(&digest[..32], 16).map_err(|e| ExportError::Package(e.to_string()))?;
    let created = Utc
        .timestamp_opt(0, 0)
        .single()
        .ok_or_else(|| ExportError::Package("the epoch is not a date".to_owned()))?;
    let options = ExportOptions {
        project_id: Uuid::from_u128(id),
        created,
        outline_width: Nm::new(profile.gerber.outline_width_nm)
            .map_err(|e| ExportError::Profile(e.to_string()))?,
    };
    let package = package_of(&set, &options)?;
    Ok(Exported {
        package,
        diagnostics: set.diagnostics,
    })
}
