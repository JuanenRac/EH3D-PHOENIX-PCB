// SPDX-License-Identifier: GPL-3.0-or-later
//! The round trip: the files that the export of this suite writes for a board are read back by
//! this import and compared with the board they came from. Positions and diameters are compared
//! with a tolerance from the settings, so a coarser coordinate format does not count as a
//! difference. Silkscreen layers are only read; their strokes depend on the font.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, Segment};
use eh3d_phoenix_export::fab::{FabFile, FileRole};
use eh3d_phoenix_export::{build, Profile};

use crate::codes::{Finding, GbrError, Severity, ROUND_TRIP_DIFFERENCE, ROUND_TRIP_FAILED};
use crate::identify::{from_attributes, Role};
use crate::import::{import_file, Content, ImportedFile, ImportedSet};
use crate::settings::ImportSettings;

/// One thing that the files and the board do not agree on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// The file.
    pub file: String,
    /// What differs, in English.
    pub what: String,
}

/// The result of a round trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTrip {
    /// The files of the export that were read back.
    pub files: usize,
    /// What the files and the board do not agree on; empty when they agree.
    pub differences: Vec<Difference>,
    /// What the import found in the files.
    pub findings: Vec<Finding>,
    /// The files as they were read back.
    pub imported: ImportedSet,
}

impl RoundTrip {
    /// The differences as findings with the code `PHOENIX-GBR-016`.
    pub fn difference_findings(&self) -> Vec<Finding> {
        self.differences
            .iter()
            .map(|d| {
                Finding::new(
                    ROUND_TRIP_DIFFERENCE,
                    Severity::Error,
                    0,
                    format!("{}: {}", d.file, d.what),
                )
            })
            .collect()
    }
}

type Points = Vec<(i64, i64)>;

fn sorted(mut points: Points) -> Points {
    points.sort_unstable();
    points
}

fn near(a: i64, b: i64, tolerance: i64) -> bool {
    (a - b).abs() <= tolerance
}

fn same_points(expected: &[(i64, i64)], found: &[(i64, i64)], tolerance: i64) -> bool {
    expected.len() == found.len()
        && expected
            .iter()
            .zip(found.iter())
            .all(|(a, b)| near(a.0, b.0, tolerance) && near(a.1, b.1, tolerance))
}

/// The positions of the pads that appear on `layer`.
fn pads_on(board: &Board, layer: Id) -> Points {
    let mut points = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            if !pad.layers.contains(&layer) {
                continue;
            }
            let at = pad_position(
                board,
                PadRef {
                    component: *component_id,
                    pad: *pad_id,
                },
            );
            if let Some(at) = at {
                points.push((at.x_nm.get(), at.y_nm.get()));
            }
        }
    }
    sorted(points)
}

/// Holes as `(diameter, x, y)`.
type Holes = Vec<(i64, i64, i64)>;

/// The holes of the board as `(diameter, x, y)`, plated and not plated apart.
fn holes_of(board: &Board) -> (Holes, Holes) {
    let mut plated = Vec::new();
    let mut not_plated = Vec::new();
    for via in board.vias.values() {
        plated.push((via.drill.get(), via.at.x_nm.get(), via.at.y_nm.get()));
    }
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            let Some(drill) = pad.drill else { continue };
            let at = pad_position(
                board,
                PadRef {
                    component: *component_id,
                    pad: *pad_id,
                },
            );
            let Some(at) = at else { continue };
            let on_copper = pad.layers.iter().any(|l| {
                board
                    .layers
                    .get(l)
                    .is_some_and(|layer| layer.kind == LayerKind::Copper)
            });
            let hole = (drill.get(), at.x_nm.get(), at.y_nm.get());
            if on_copper {
                plated.push(hole);
            } else {
                not_plated.push(hole);
            }
        }
    }
    plated.sort_unstable();
    not_plated.sort_unstable();
    (plated, not_plated)
}

/// True when the export can write the arc: its ends differ and its sweep is not a whole turn.
fn arc_is_written(start: (i64, i64), end: (i64, i64), sweep_udeg: i64) -> bool {
    let (dx, dy) = ((end.0 - start.0) as f64, (end.1 - start.1) as f64);
    let distance = dx.hypot(dy);
    let half = (sweep_udeg.abs() as f64) / 1_000_000.0 * std::f64::consts::PI / 360.0;
    distance > 0.0 && distance.is_finite() && half.sin().abs() >= 1e-9
}

struct Checker<'a> {
    board: &'a Board,
    tolerance: i64,
    differences: Vec<Difference>,
}

impl Checker<'_> {
    fn differ(&mut self, file: &str, what: String) {
        self.differences.push(Difference {
            file: file.to_owned(),
            what,
        });
    }

    fn count(&mut self, file: &str, what: &str, model: usize, found: usize) {
        if model != found {
            self.differ(
                file,
                format!("{what}: the board has {model}, the file has {found}"),
            );
        }
    }

    fn layer(&mut self, name: &str, layer: Id, kind: LayerKind, imported: &ImportedFile) {
        let Some(gerber) = imported.gerber() else {
            return;
        };
        match kind {
            LayerKind::Copper => {
                let mut expected = pads_on(self.board, layer);
                for via in self.board.vias.values() {
                    if via.from_layer == layer || via.to_layer == layer {
                        expected.push((via.at.x_nm.get(), via.at.y_nm.get()));
                    }
                }
                self.flashes(name, expected, gerber);
                let zones = self
                    .board
                    .zones
                    .values()
                    .filter(|z| z.layer == layer && z.outline.len() >= 3)
                    .count();
                self.count(name, "regions", zones, gerber.regions());
                let (mut lines, mut arcs) = (0usize, 0usize);
                for track in self.board.tracks.values().filter(|t| t.layer == layer) {
                    for segment in &track.path {
                        match segment {
                            Segment::Line { .. } => lines += 1,
                            Segment::Arc { start, end, sweep } => {
                                let s = (start.x_nm.get(), start.y_nm.get());
                                let e = (end.x_nm.get(), end.y_nm.get());
                                if arc_is_written(s, e, sweep.get()) {
                                    arcs += 1;
                                }
                            }
                        }
                    }
                }
                self.count(name, "lines", lines, gerber.lines());
                self.count(name, "arcs", arcs, gerber.arcs());
            }
            LayerKind::SolderMask | LayerKind::Paste => {
                let expected = pads_on(self.board, layer);
                self.flashes(name, expected, gerber);
            }
            LayerKind::Outline => {
                let board = self.board;
                let outline = &board.outline;
                if outline.len() < 3 {
                    return;
                }
                self.count(name, "lines", outline.len(), gerber.lines());
                let xs = outline.iter().map(|p| p.x_nm.get());
                let ys = outline.iter().map(|p| p.y_nm.get());
                let model = (
                    xs.clone().min().unwrap_or(0),
                    ys.clone().min().unwrap_or(0),
                    xs.max().unwrap_or(0),
                    ys.max().unwrap_or(0),
                );
                let found = gerber
                    .centre_bounds()
                    .map(|(lo, hi)| (lo.x, lo.y, hi.x, hi.y));
                let same = found.is_some_and(|f| {
                    near(f.0, model.0, self.tolerance)
                        && near(f.1, model.1, self.tolerance)
                        && near(f.2, model.2, self.tolerance)
                        && near(f.3, model.3, self.tolerance)
                });
                if !same {
                    self.differ(name, "the bounding box of the outline".to_owned());
                }
            }
            LayerKind::Silkscreen | LayerKind::Documentation => {}
        }
    }

    fn flashes(&mut self, name: &str, expected: Points, gerber: &crate::model::GerberLayer) {
        let found = sorted(
            gerber
                .items
                .iter()
                .filter_map(|i| match i {
                    crate::model::Item::Flash { at, .. } => Some((at.x, at.y)),
                    _ => None,
                })
                .collect(),
        );
        let expected = sorted(expected);
        if expected.len() != found.len() {
            self.count(name, "flashes", expected.len(), found.len());
        } else if !same_points(&expected, &found, self.tolerance) {
            self.differ(name, "the positions of the flashes".to_owned());
        }
    }

    fn drill(&mut self, name: &str, plated: bool, imported: &ImportedFile) {
        let Some(file) = imported.drill() else {
            return;
        };
        let (plated_holes, not_plated_holes) = holes_of(self.board);
        let expected = if plated {
            plated_holes
        } else {
            not_plated_holes
        };
        let total = file.hits.len() + file.slots.len();
        self.count(name, "holes", expected.len(), total);
        if file.units != crate::model::Units::Millimetres {
            self.differ(name, "the units are not millimetres".to_owned());
        }
        if !file.slots.is_empty() || expected.len() != total {
            return;
        }
        let mut found: Vec<(i64, i64, i64)> = file
            .hits
            .iter()
            .map(|h| (file.diameter(h.tool), h.at.x, h.at.y))
            .collect();
        found.sort_unstable();
        let same = found.iter().zip(expected.iter()).all(|(f, m)| {
            near(f.0, m.0, self.tolerance)
                && near(f.1, m.1, self.tolerance)
                && near(f.2, m.2, self.tolerance)
        });
        if !same {
            self.differ(name, "the diameters or positions of the holes".to_owned());
        }
    }
}

fn role_agrees(role: Role, kind: LayerKind) -> bool {
    match kind {
        LayerKind::Copper => matches!(
            role,
            Role::CopperTop | Role::CopperBottom | Role::CopperInner(_)
        ),
        LayerKind::SolderMask => matches!(role, Role::MaskTop | Role::MaskBottom),
        LayerKind::Paste => matches!(role, Role::PasteTop | Role::PasteBottom),
        LayerKind::Silkscreen => matches!(role, Role::LegendTop | Role::LegendBottom),
        LayerKind::Outline => role == Role::Outline,
        LayerKind::Documentation => true,
    }
}

/// Exports `board` with `profile`, reads every layer and drill file back and compares them with
/// the board.
pub fn round_trip(
    board: &Board,
    profile: &Profile,
    settings: &ImportSettings,
) -> Result<RoundTrip, GbrError> {
    let set = build(board, profile)
        .map_err(|e| GbrError::new(ROUND_TRIP_FAILED, e.diagnostic().message))?;
    Ok(compare_files(board, &set.files, settings))
}

/// Reads files that were made from `board` (the files of an export, perhaps changed since) and
/// compares them with it.
pub fn compare_files(board: &Board, set_files: &[FabFile], settings: &ImportSettings) -> RoundTrip {
    let mut checker = Checker {
        board,
        tolerance: settings.compare_tolerance_nm,
        differences: Vec::new(),
    };
    let mut files: Vec<ImportedFile> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    for file in set_files {
        let FabFile { bytes, role, .. } = file;
        let name = file.folder_name().to_owned();
        match role {
            FileRole::Layer { layer, kind, .. } => {
                let imported = import_file(&name, bytes, settings);
                if let Content::Failed(error) = &imported.content {
                    checker.differ(&name, format!("the file cannot be read back: {error}"));
                } else {
                    let told = imported
                        .gerber()
                        .and_then(|g| from_attributes(&g.attributes))
                        .filter(|r| !role_agrees(*r, *kind));
                    if let Some(found) = told {
                        checker.differ(&name, format!("the attribute says {}", found.name()));
                    }
                    checker.layer(&name, *layer, *kind, &imported);
                }
                files.push(imported);
            }
            FileRole::Drill { plated } => {
                let imported = import_file(&name, bytes, settings);
                if let Content::Failed(error) = &imported.content {
                    checker.differ(&name, format!("the file cannot be read back: {error}"));
                } else {
                    checker.drill(&name, *plated, &imported);
                }
                files.push(imported);
            }
            FileRole::Placement | FileRole::Bom => {}
        }
    }
    let (plated, not_plated) = holes_of(board);
    for (is_plated, model) in [(true, plated.len()), (false, not_plated.len())] {
        let present = set_files
            .iter()
            .any(|f| f.role == FileRole::Drill { plated: is_plated });
        if model > 0 && !present {
            checker.differ(
                "drill",
                format!(
                    "the board has {model} {} hole(s) and there is no file for them",
                    if is_plated { "plated" } else { "not plated" }
                ),
            );
        }
    }
    for imported in &files {
        findings.extend(imported.findings().iter().cloned());
    }
    RoundTrip {
        files: files.len(),
        differences: checker.differences,
        findings,
        imported: ImportedSet { files },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_and_points_use_the_tolerance() {
        assert!(near(10, 12, 2) && !near(10, 13, 2));
        let a = [(0, 0), (10, 10)];
        let b = [(1, -1), (10, 11)];
        assert!(same_points(&a, &b, 1));
        assert!(!same_points(&a, &b, 0));
        assert!(!same_points(&a, &b[..1], 5));
    }

    #[test]
    fn an_arc_is_written_when_its_ends_differ_and_it_is_not_a_whole_turn() {
        assert!(arc_is_written((0, 0), (1_000_000, 0), 90_000_000));
        assert!(!arc_is_written((0, 0), (0, 0), 90_000_000));
        assert!(!arc_is_written((0, 0), (1_000_000, 0), 0));
        assert!(!arc_is_written((0, 0), (1_000_000, 0), 360_000_000));
    }

    #[test]
    fn roles_agree_with_kinds() {
        assert!(role_agrees(Role::CopperInner(1), LayerKind::Copper));
        assert!(role_agrees(Role::MaskBottom, LayerKind::SolderMask));
        assert!(!role_agrees(Role::MaskBottom, LayerKind::Copper));
        assert!(role_agrees(Role::Outline, LayerKind::Outline));
        assert!(role_agrees(Role::Drill, LayerKind::Documentation));
    }

    #[test]
    fn a_difference_becomes_a_finding_with_its_code() {
        let trip = RoundTrip {
            files: 1,
            differences: vec![Difference {
                file: "a.gbr".to_owned(),
                what: "flashes: the board has 2, the file has 1".to_owned(),
            }],
            findings: Vec::new(),
            imported: ImportedSet::default(),
        };
        let found = trip.difference_findings();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, ROUND_TRIP_DIFFERENCE);
        assert!(found[0].detail.starts_with("a.gbr: flashes"));
    }
}
