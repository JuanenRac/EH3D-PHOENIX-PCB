//! The set of manufacturing files of a board, and where it goes.
//!
//! [`build`] makes every file in memory from the board and a [`Profile`]: one Gerber X2 file per
//! layer, the plated and not plated drill files, the placement file and the bill of materials. The
//! result is [`FabSet`]; [`FabSet::manifest`] lists every file with its SHA-256 and size (the
//! integrity record of EH3D-COMMON, written as canonical JSON) and [`FabSet::write_folder`] writes
//! the files and the manifest to a folder. Nothing reads the clock or makes a random number: the
//! same board and profile give the same bytes.

use std::path::Path;

use eh3d_container::Integrity;
use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Id, LayerKind, Side, StackFunction};
use eh3d_writer_excellon::CoordinateStyle;

use crate::assembly::{bom_csv, placement_csv};
use crate::codes;
use crate::excellon::{write_drill_files, DrillOptions};
use crate::gerber::{format_with_decimals, write_layer, LayerError, LayerFileSpec};
use crate::package::ExportError;
use crate::profile::{DrillStyle, Profile};

/// The name of the manifest file of a folder.
pub const MANIFEST_FILE: &str = "manifest.json";

/// What a file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRole {
    /// The Gerber file of a layer.
    Layer {
        /// The layer.
        layer: Id,
        /// Its kind.
        kind: LayerKind,
        /// Its name.
        name: String,
    },
    /// An Excellon file.
    Drill {
        /// True for the plated holes.
        plated: bool,
    },
    /// An Excellon file of the blind or the buried vias that join one pair of layers.
    DrillSpan {
        /// The upper layer, counted from 1 at the top of the copper stack.
        from: usize,
        /// The lower layer.
        to: usize,
        /// True for buried vias, false for blind ones.
        buried: bool,
    },
    /// The placement file.
    Placement,
    /// The bill of materials.
    Bom,
}

/// The name of the drill file of the blind or buried vias between layers `from` and `to`: the name
/// of the plated file with `_blind_<from>-<to>` or `_buried_<from>-<to>` before its extension
/// (`drill_pth.drl` gives `drill_pth_blind_1-2.drl`). The name only follows the profile and the
/// layers, so the same board always gives the same names.
pub fn span_file_name(plated_file: &str, from: usize, to: usize, buried: bool) -> String {
    let kind = if buried { "buried" } else { "blind" };
    match plated_file.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => {
            format!("{stem}_{kind}_{from}-{to}.{extension}")
        }
        _ => format!("{plated_file}_{kind}_{from}-{to}"),
    }
}

/// One file of the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabFile {
    /// The path inside a package (`layers/...`); in a folder the file is `path` without `layers/`.
    pub path: String,
    /// The bytes.
    pub bytes: Vec<u8>,
    /// What the file is.
    pub role: FileRole,
}

impl FabFile {
    /// The name of the file in a folder.
    pub fn folder_name(&self) -> &str {
        self.path.strip_prefix("layers/").unwrap_or(&self.path)
    }

    /// The text of the file (all of them are UTF-8).
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes).unwrap_or("")
    }
}

/// Every file of a board and what was found while making them.
#[derive(Debug, Clone)]
pub struct FabSet {
    /// The name of the board.
    pub name: String,
    /// The files, in a fixed order: layers, drills, placement, bill of materials.
    pub files: Vec<FabFile>,
    /// What the files cannot say, in order.
    pub diagnostics: Vec<Diagnostic>,
}

pub(crate) fn rank(kind: LayerKind) -> u8 {
    match kind {
        LayerKind::Copper => 0,
        LayerKind::SolderMask => 1,
        LayerKind::Paste => 2,
        LayerKind::Silkscreen => 3,
        LayerKind::Outline => 4,
        LayerKind::Documentation => 5,
    }
}

pub(crate) fn slug(name: &str) -> String {
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

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Top => "Top",
        Side::Bottom => "Bot",
    }
}

fn file_function(board: &Board, kind: LayerKind, layer: Id) -> Option<Vec<String>> {
    let side = board.layers.get(&layer).and_then(|l| l.side);
    let on_side = |what: &str| side.map(|s| vec![what.to_owned(), side_name(s).to_owned()]);
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
        LayerKind::Silkscreen => on_side("Legend"),
        LayerKind::SolderMask => on_side("Soldermask"),
        LayerKind::Paste => on_side("Paste"),
        LayerKind::Documentation => None,
    }
}

/// The polarity attribute of a kind of layer: copper, legend and paste are drawn where the material
/// is; a solder mask file draws the openings, which are where the mask is not.
fn polarity(kind: LayerKind) -> Option<&'static str> {
    match kind {
        LayerKind::Copper | LayerKind::Silkscreen | LayerKind::Paste => Some("Positive"),
        LayerKind::SolderMask => Some("Negative"),
        LayerKind::Outline | LayerKind::Documentation => None,
    }
}

/// The number of copper layers of the board.
pub(crate) fn copper_layer_count(board: &Board) -> usize {
    if board.copper_stack.is_empty() {
        board
            .layers
            .values()
            .filter(|l| l.kind == LayerKind::Copper)
            .count()
    } else {
        board.copper_stack.len()
    }
}

/// Makes every file of the set.
pub fn build(board: &Board, profile: &Profile) -> Result<FabSet, ExportError> {
    profile.validate()?;
    let mut diagnostics = Vec::new();
    let name = if board.name.trim().is_empty() {
        "board".to_owned()
    } else {
        board.name.clone()
    };
    let format = format_with_decimals(profile.gerber.decimal_digits);
    let mut layers: Vec<(&Id, &eh3d_phoenix_board::Layer)> = board.layers.iter().collect();
    layers.sort_by(|a, b| {
        (rank(a.1.kind), a.1.name.as_str(), a.0).cmp(&(rank(b.1.kind), b.1.name.as_str(), b.0))
    });
    let mut files: Vec<FabFile> = Vec::new();
    let mut copper_without_function = false;
    for (index, (id, layer)) in layers.iter().enumerate() {
        let function = file_function(board, layer.kind, **id);
        if function.is_none() {
            match layer.kind {
                LayerKind::Copper => copper_without_function = true,
                LayerKind::Silkscreen => diagnostics.push(codes::diagnostic(
                    codes::SILK_SIDE_MISSING,
                    Severity::Info,
                    &layer.name,
                )),
                LayerKind::SolderMask | LayerKind::Paste => diagnostics.push(codes::diagnostic(
                    codes::SIDE_MISSING,
                    Severity::Info,
                    &layer.name,
                )),
                LayerKind::Outline | LayerKind::Documentation => {}
            }
        }
        let spec = LayerFileSpec {
            layer: **id,
            kind: layer.kind,
            file_function: function,
            polarity: polarity(layer.kind),
            format,
            mask_expansion: profile.gerber.mask_expansion_nm,
            paste_shrink: profile.gerber.paste_shrink_nm,
            x2: profile.gerber.x2.clone(),
        };
        let (output, mut found) = write_layer(board, &spec, profile.gerber.outline_width_nm)
            .map_err(|e| match e {
                LayerError::Field(f) => ExportError::Attribute(format!("{}: {f}", layer.name)),
                other => ExportError::Layer {
                    layer: layer.name.clone(),
                    message: other.to_string(),
                },
            })?;
        diagnostics.append(&mut found);
        files.push(FabFile {
            path: format!(
                "layers/{:02}_{}.{}",
                index + 1,
                slug(&layer.name),
                profile.gerber.extension
            ),
            bytes: output.text.into_bytes(),
            role: FileRole::Layer {
                layer: **id,
                kind: layer.kind,
                name: layer.name.clone(),
            },
        });
    }
    if copper_without_function {
        diagnostics.push(codes::diagnostic(
            codes::STACK_ORDER_MISSING,
            Severity::Info,
            "",
        ));
    }
    let d = &profile.drill;
    let drill_options = DrillOptions {
        style: match d.style {
            DrillStyle::Decimal => CoordinateStyle::Decimal {
                decimals: d.decimals,
            },
            DrillStyle::Fixed => CoordinateStyle::Fixed {
                integer_digits: d.fixed_integer_digits,
                decimal_digits: d.fixed_decimal_digits,
            },
        },
        oblong_as_slot: d.oblong_as_slot,
        copper_layers: copper_layer_count(board),
        x2: profile.gerber.x2.clone(),
        board_name: name.clone(),
    };
    let mut drills = write_drill_files(board, &drill_options).map_err(|e| match e {
        LayerError::Field(f) => ExportError::Attribute(format!("drill file: {f}")),
        other => ExportError::Drill(other.to_string()),
    })?;
    diagnostics.append(&mut drills.diagnostics);
    for (text, file_name, plated) in [
        (drills.plated, &d.plated_file, true),
        (drills.non_plated, &d.non_plated_file, false),
    ] {
        if let Some(text) = text {
            files.push(FabFile {
                path: format!("layers/{file_name}"),
                bytes: text.into_bytes(),
                role: FileRole::Drill { plated },
            });
        }
    }
    for span in drills.spans {
        files.push(FabFile {
            path: format!(
                "layers/{}",
                span_file_name(&d.plated_file, span.from, span.to, span.buried)
            ),
            bytes: span.text.into_bytes(),
            role: FileRole::DrillSpan {
                from: span.from,
                to: span.to,
                buried: span.buried,
            },
        });
    }
    let (placement, mut found) = placement_csv(board, &profile.placement);
    diagnostics.append(&mut found);
    files.push(FabFile {
        path: format!("layers/{}", profile.placement.file),
        bytes: placement.into_bytes(),
        role: FileRole::Placement,
    });
    let (bom, _) = bom_csv(board, &profile.bom);
    files.push(FabFile {
        path: format!("layers/{}", profile.bom.file),
        bytes: bom.into_bytes(),
        role: FileRole::Bom,
    });
    Ok(FabSet {
        name,
        files,
        diagnostics,
    })
}

impl FabSet {
    /// The integrity record of the files, by their names in a folder.
    pub fn integrity(&self) -> Integrity {
        Integrity::for_entries(
            self.files
                .iter()
                .map(|f| (f.folder_name(), f.bytes.as_slice())),
        )
    }

    /// The manifest: the integrity record as canonical JSON.
    pub fn manifest(&self) -> Result<Vec<u8>, ExportError> {
        eh3d_container::to_canonical_file_bytes(&self.integrity())
            .map_err(|e| ExportError::Package(e.to_string()))
    }

    /// Writes the files and the manifest to `dir`, which is created when it is missing. A file
    /// that is already there (a link included) is never replaced: the call fails before it
    /// writes anything.
    pub fn write_folder(&self, dir: &Path) -> Result<(), ExportError> {
        use std::io::Write;
        let fail = |what: &str, e: std::io::Error| ExportError::Output(format!("{what}: {e}"));
        let manifest = self.manifest()?;
        let mut entries: Vec<(&str, &[u8])> = self
            .files
            .iter()
            .map(|f| (f.folder_name(), f.bytes.as_slice()))
            .collect();
        entries.push((MANIFEST_FILE, manifest.as_slice()));
        std::fs::create_dir_all(dir).map_err(|e| fail("the output folder", e))?;
        for &(name, _) in &entries {
            if std::fs::symlink_metadata(dir.join(name)).is_ok() {
                let exists = std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "it exists and is never replaced",
                );
                return Err(fail(name, exists));
            }
        }
        for &(name, bytes) in &entries {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join(name))
                .map_err(|e| fail(name, e))?;
            file.write_all(bytes).map_err(|e| fail(name, e))?;
        }
        Ok(())
    }
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

    #[test]
    fn a_folder_name_drops_the_layers_folder() {
        let file = FabFile {
            path: "layers/bom.csv".to_owned(),
            bytes: b"x".to_vec(),
            role: FileRole::Bom,
        };
        assert_eq!(file.folder_name(), "bom.csv");
        assert_eq!(file.text(), "x");
    }
}
