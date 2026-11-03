//! The manufacturing package: an `.eh3dproj` 1.0.0 project with one Gerber X2 file per layer and
//! one Excellon drill file.
//!
//! The export is deterministic: the caller gives the project identity and the time stamp, nothing
//! reads the clock or makes a random number, and every ordering is fixed, so the same board and
//! options give the same package bytes.

use chrono::{DateTime, Utc};
use eh3d_common::project::{LayerKind as PackageLayerKind, LayerRef};
use eh3d_common::{Package, ProjectMetadata};
use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Id, LayerKind, Side, StackFunction};
use eh3d_phoenix_model::Nm;
use uuid::Uuid;

use crate::codes;
use crate::excellon::write_drills;
use crate::gerber::{write_layer, LayerFileSpec};

/// What the caller chooses.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// The identity of the exported project.
    pub project_id: Uuid,
    /// The time stamp written as created and modified.
    pub created: DateTime<Utc>,
    /// The width of the line that draws the board outline.
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
}

impl ExportError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            ExportError::Layer { .. } | ExportError::Drill(_) => codes::WRITE_FAILED,
            ExportError::Package(_) => codes::PACKAGE_FAILED,
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

fn rank(kind: LayerKind) -> u8 {
    match kind {
        LayerKind::Copper => 0,
        LayerKind::SolderMask => 1,
        LayerKind::Paste => 2,
        LayerKind::Silkscreen => 3,
        LayerKind::Outline => 4,
        LayerKind::Documentation => 5,
    }
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

fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let trimmed = out.trim_end_matches('_');
    if trimmed.is_empty() {
        "layer".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn file_function(board: &Board, kind: LayerKind, layer: Id) -> Option<Vec<String>> {
    match kind {
        LayerKind::Copper => {
            let (number, function) = board.stack_position(layer)?;
            let position = match function {
                StackFunction::Top => "Top",
                StackFunction::Inner => "Inr",
                StackFunction::Bottom => "Bot",
            };
            Some(vec![
                "Copper".to_owned(),
                format!("L{number}"),
                position.to_owned(),
            ])
        }
        LayerKind::Outline => Some(vec!["Profile".to_owned(), "NP".to_owned()]),
        LayerKind::Silkscreen => {
            let side = match board.layers.get(&layer)?.side? {
                Side::Top => "Top",
                Side::Bottom => "Bot",
            };
            Some(vec!["Legend".to_owned(), side.to_owned()])
        }
        _ => None,
    }
}

/// Exports the board as a package.
pub fn export_package(board: &Board, options: &ExportOptions) -> Result<Exported, ExportError> {
    let mut diagnostics = Vec::new();
    let name = if board.name.trim().is_empty() {
        "board".to_owned()
    } else {
        board.name.clone()
    };
    let mut metadata = ProjectMetadata::new(name);
    metadata.id = options.project_id;
    metadata.created = options.created;
    metadata.modified = options.created;

    let mut layers: Vec<(&Id, &eh3d_phoenix_board::Layer)> = board.layers.iter().collect();
    layers.sort_by(|a, b| {
        (rank(a.1.kind), a.1.name.as_str(), a.0).cmp(&(rank(b.1.kind), b.1.name.as_str(), b.0))
    });
    let mut package_entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut copper_without_function = false;
    for (index, (id, layer)) in layers.iter().enumerate() {
        let function = file_function(board, layer.kind, **id);
        if layer.kind == LayerKind::Copper && function.is_none() {
            copper_without_function = true;
        }
        if layer.kind == LayerKind::Silkscreen && function.is_none() {
            diagnostics.push(codes::diagnostic(
                codes::SILK_SIDE_MISSING,
                Severity::Info,
                &layer.name,
            ));
        }
        let spec = LayerFileSpec {
            layer: **id,
            kind: layer.kind,
            file_function: function,
        };
        let (output, mut found) =
            write_layer(board, &spec, options.outline_width.get()).map_err(|e| {
                ExportError::Layer {
                    layer: layer.name.clone(),
                    message: e.to_string(),
                }
            })?;
        diagnostics.append(&mut found);
        let path = format!("layers/{:02}_{}.gbr", index + 1, slug(&layer.name));
        metadata.layers.push(LayerRef {
            path: path.clone(),
            kind: package_kind(layer.kind),
            name: layer.name.clone(),
        });
        package_entries.push((path, output.text.into_bytes()));
    }
    if copper_without_function {
        diagnostics.push(codes::diagnostic(
            codes::STACK_ORDER_MISSING,
            Severity::Info,
            "",
        ));
    }
    if let Some((text, mut found)) =
        write_drills(board).map_err(|e| ExportError::Drill(e.to_string()))?
    {
        diagnostics.append(&mut found);
        let path = "layers/drill.drl".to_owned();
        metadata.layers.push(LayerRef {
            path: path.clone(),
            kind: PackageLayerKind::Drill,
            name: "Drill".to_owned(),
        });
        package_entries.push((path, text.into_bytes()));
    }
    let mut package = Package::new(metadata);
    for (path, bytes) in package_entries {
        package
            .add_entry(&path, bytes)
            .map_err(|e| ExportError::Package(e.to_string()))?;
    }
    package
        .metadata
        .validate()
        .map_err(|e| ExportError::Package(e.to_string()))?;
    Ok(Exported {
        package,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_become_safe_file_names() {
        assert_eq!(slug("Top Copper"), "top_copper");
        assert_eq!(slug("  Bottom/Silk!! "), "bottom_silk");
        assert_eq!(slug("日本語"), "layer");
        assert_eq!(slug(""), "layer");
    }
}
