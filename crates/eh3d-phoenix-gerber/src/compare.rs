// SPDX-License-Identifier: GPL-3.0-or-later
//! The round trip: the files that the export of this suite writes for a board are read back by
//! this import and compared with the board they came from. Positions and diameters are compared
//! with a tolerance from the settings, so a coarser coordinate format does not count as a
//! difference. Silkscreen layers are only read; their strokes depend on the font.

use std::collections::BTreeMap;

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, LayerKind, PadRef, Segment};
use eh3d_phoenix_export::excellon::{via_span, ViaSpan};
use eh3d_phoenix_export::fab::{FabFile, FileRole};
use eh3d_phoenix_export::gerber::{pad_is_region, via_on_layer};
use eh3d_phoenix_export::profile::{ObjectAttribute, X2Level, X2Profile};
use eh3d_phoenix_export::x2::{unescape_field, NO_NET};
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

/// True when the pad has no copper to put on the layer: a hole that is not plated, on a copper
/// layer.
fn no_copper(board: &Board, layer: Id, pad: &eh3d_phoenix_board::Pad) -> bool {
    pad.is_npth()
        && board
            .layers
            .get(&layer)
            .is_some_and(|l| l.kind == LayerKind::Copper)
}

/// The positions of the pads that appear on `layer` and that the export flashes (a pad with
/// rounded corners is a region, see [`region_pads_on`]).
fn pads_on(board: &Board, layer: Id) -> Points {
    let mut points = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            if !pad.layers.contains(&layer)
                || no_copper(board, layer, pad)
                || pad_is_region(pad, component.rotation)
            {
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

/// The nets (as the label of `.N`) of the pads on `layer` that the export writes as regions.
fn region_pads_on(board: &Board, layer: Id) -> Vec<String> {
    let mut nets = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            if !pad.layers.contains(&layer)
                || no_copper(board, layer, pad)
                || !pad_is_region(pad, component.rotation)
            {
                continue;
            }
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            if pad_position(board, pad_ref).is_some() {
                nets.push(net_label(board, board.net_of(pad_ref)));
            }
        }
    }
    nets.sort_unstable();
    nets
}

/// Holes as `(diameter, x, y)`.
type Holes = Vec<(i64, i64, i64)>;

/// The holes of the board as `(diameter, x, y)`, plated and not plated apart. The plated holes are
/// the through vias and the plated pads; the blind and buried vias have a file of their own
/// ([`span_holes`]).
fn holes_of(board: &Board) -> (Holes, Holes) {
    let mut plated = Vec::new();
    let mut not_plated = Vec::new();
    for via in board.vias.values() {
        if via_span(board, via) == ViaSpan::Through {
            plated.push((via.drill.get(), via.at.x_nm.get(), via.at.y_nm.get()));
        }
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
            if on_copper && pad.plated {
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

/// The holes of the blind (`buried` false) or buried vias between layers `from` and `to`.
fn span_holes(board: &Board, from: usize, to: usize, buried: bool) -> Holes {
    let wanted = if buried {
        ViaSpan::Buried(from, to)
    } else {
        ViaSpan::Blind(from, to)
    };
    let mut holes: Holes = board
        .vias
        .values()
        .filter(|v| via_span(board, v) == wanted)
        .map(|v| (v.drill.get(), v.at.x_nm.get(), v.at.y_nm.get()))
        .collect();
    holes.sort_unstable();
    holes
}

/// True when the export can write the arc: its ends differ and its sweep is not a whole turn.
fn arc_is_written(start: (i64, i64), end: (i64, i64), sweep_udeg: i64) -> bool {
    let (dx, dy) = ((end.0 - start.0) as f64, (end.1 - start.1) as f64);
    let distance = dx.hypot(dy);
    let half = (sweep_udeg.abs() as f64) / 1_000_000.0 * std::f64::consts::PI / 360.0;
    distance > 0.0 && distance.is_finite() && half.sin().abs() >= 1e-9
}

fn dark_regions(gerber: &crate::model::GerberLayer) -> usize {
    gerber
        .items
        .iter()
        .filter(|i| matches!(i, crate::model::Item::Region { dark: true, .. }))
        .count()
}

fn dark_lines(gerber: &crate::model::GerberLayer) -> usize {
    gerber
        .items
        .iter()
        .filter(|i| matches!(i, crate::model::Item::Line { dark: true, .. }))
        .count()
}

fn dark_arcs(gerber: &crate::model::GerberLayer) -> usize {
    gerber
        .items
        .iter()
        .filter(|i| matches!(i, crate::model::Item::Arc { dark: true, .. }))
        .count()
}

/// The label of a net for the attribute `.N`: its name, or `N/C` when there is no net.
fn net_label(board: &Board, id: Option<Id>) -> String {
    id.and_then(|i| board.nets.get(&i))
        .map(|n| n.name.as_str())
        .filter(|n| !n.is_empty())
        .unwrap_or(NO_NET)
        .to_owned()
}

/// The pads of the board on a layer: for each reference designator and pin number, where the pads
/// with that name are and on which net.
type PadTable = BTreeMap<(String, String), Vec<((i64, i64), String)>>;

fn pad_table(board: &Board, layer: Id) -> PadTable {
    let mut table = PadTable::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            if !pad.layers.contains(&layer)
                || component.refdes.is_empty()
                || pad.number.is_empty()
                || no_copper(board, layer, pad)
                || pad_is_region(pad, component.rotation)
            {
                continue;
            }
            let pad_ref = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            let Some(at) = pad_position(board, pad_ref) else {
                continue;
            };
            table
                .entry((component.refdes.clone(), pad.number.clone()))
                .or_default()
                .push((
                    (at.x_nm.get(), at.y_nm.get()),
                    net_label(board, board.net_of(pad_ref)),
                ));
        }
    }
    table
}

/// The nets of the tracks of a layer that the export writes, one per line or arc, sorted.
fn track_nets(board: &Board, layer: Id) -> Vec<String> {
    let mut nets = Vec::new();
    for track in board.tracks.values().filter(|t| t.layer == layer) {
        let label = net_label(board, Some(track.net));
        for segment in &track.path {
            let written = match segment {
                Segment::Line { .. } => true,
                Segment::Arc { start, end, sweep } => arc_is_written(
                    (start.x_nm.get(), start.y_nm.get()),
                    (end.x_nm.get(), end.y_nm.get()),
                    sweep.get(),
                ),
            };
            if written {
                nets.push(label.clone());
            }
        }
    }
    nets.sort_unstable();
    nets
}

struct Checker<'a> {
    board: &'a Board,
    tolerance: i64,
    x2: &'a X2Profile,
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
                    if via_on_layer(self.board, via, layer) {
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
                let pad_regions = region_pads_on(self.board, layer).len();
                self.count(name, "regions", zones + pad_regions, dark_regions(gerber));
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
                self.count(name, "lines", lines, dark_lines(gerber));
                self.count(name, "arcs", arcs, dark_arcs(gerber));
            }
            LayerKind::SolderMask | LayerKind::Paste => {
                let expected = pads_on(self.board, layer);
                self.flashes(name, expected, gerber);
                let pad_regions = region_pads_on(self.board, layer).len();
                self.count(name, "regions", pad_regions, dark_regions(gerber));
            }
            LayerKind::Outline => {
                let board = self.board;
                let outline = &board.outline;
                if outline.len() < 3 {
                    return;
                }
                let cutout_lines: usize = board
                    .outline_cutouts
                    .iter()
                    .filter(|c| c.len() >= 3)
                    .map(Vec::len)
                    .sum();
                self.count(name, "lines", outline.len() + cutout_lines, gerber.lines());
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

    /// The attributes of a file against the level of X2 the export was asked for.
    fn x2_file(&mut self, name: &str, attributes: &BTreeMap<String, Vec<String>>) {
        let level = self.x2.level;
        if level == X2Level::X1 {
            if !attributes.is_empty() {
                self.differ(
                    name,
                    "the file has attributes although the level is X1".to_owned(),
                );
            }
            return;
        }
        if !level.has_file_attributes() {
            return;
        }
        for required in [
            ".GenerationSoftware",
            ".ProjectId",
            ".SameCoordinates",
            ".Part",
        ] {
            if !attributes.contains_key(required) {
                self.differ(name, format!("the attribute {required} is missing"));
            }
        }
        if let Some(values) = attributes.get(".ProjectId") {
            let board_name = if self.board.name.trim().is_empty() {
                "board"
            } else {
                self.board.name.as_str()
            };
            let name_ok = values
                .first()
                .is_some_and(|v| unescape_field(v) == board_name);
            let id_ok = values.get(1).is_some_and(|v| {
                v.len() == 36
                    && v.matches('-').count() == 4
                    && self
                        .x2
                        .project_id
                        .as_deref()
                        .is_none_or(|wanted| v.eq_ignore_ascii_case(wanted))
            });
            if values.len() != 3 || !name_ok || !id_ok {
                self.differ(
                    name,
                    "the attribute .ProjectId does not agree with the board".to_owned(),
                );
            }
        }
        if attributes.contains_key(".CreationDate") != self.x2.creation_date.is_some() {
            self.differ(
                name,
                "the attribute .CreationDate does not agree with the profile".to_owned(),
            );
        }
    }

    /// The attributes of a layer file: the file attributes, the function of each aperture and the
    /// attributes of the objects.
    fn x2_layer(&mut self, name: &str, layer: Id, kind: LayerKind, imported: &ImportedFile) {
        let Some(gerber) = imported.gerber() else {
            return;
        };
        self.x2_file(name, &gerber.attributes);
        let level = self.x2.level;
        if level.has_aperture_attributes() && kind != LayerKind::Silkscreen {
            for number in gerber.apertures.keys() {
                let named = gerber
                    .aperture_attributes
                    .get(number)
                    .is_some_and(|a| a.contains_key(".AperFunction"));
                if !named {
                    self.differ(name, format!("the aperture D{number} has no function"));
                }
            }
        }
        if level.has_object_attributes()
            && matches!(
                kind,
                LayerKind::Copper | LayerKind::SolderMask | LayerKind::Paste
            )
        {
            self.x2_objects(name, layer, kind, gerber);
        }
    }

    /// The object attributes of a copper, solder mask or paste layer against the board: the pins
    /// of the pads (reference designator, pin number, place and net), the components and the nets
    /// of vias, tracks and zones.
    fn x2_objects(
        &mut self,
        name: &str,
        layer: Id,
        kind: LayerKind,
        gerber: &crate::model::GerberLayer,
    ) {
        let copper = kind == LayerKind::Copper;
        let (nets_wanted, pins_wanted, components_wanted) = (
            copper && self.x2.writes(ObjectAttribute::Net),
            self.x2.writes(ObjectAttribute::Pin),
            self.x2.writes(ObjectAttribute::Component),
        );
        let table = pad_table(self.board, layer);
        let pad_total: usize = table.values().map(Vec::len).sum();
        let (mut with_pin, mut with_component, mut without_net) = (0usize, 0usize, 0usize);
        let (mut loose, mut lines, mut regions): (Vec<String>, Vec<String>, Vec<String>) =
            (Vec::new(), Vec::new(), Vec::new());
        for (index, item) in gerber.items.iter().enumerate() {
            let dark = match item {
                crate::model::Item::Flash { dark, .. }
                | crate::model::Item::Line { dark, .. }
                | crate::model::Item::Arc { dark, .. }
                | crate::model::Item::Region { dark, .. } => *dark,
            };
            if !dark {
                // what takes copper away (a cutout of a zone, a keep-away) has no attributes
                continue;
            }
            let attributes = gerber.object_attributes(index);
            let value = |key: &str, at: usize| {
                attributes
                    .and_then(|a| a.get(key))
                    .and_then(|v| v.get(at))
                    .map(|v| unescape_field(v))
            };
            let net = value(".N", 0);
            if nets_wanted && net.is_none() {
                without_net += 1;
            }
            let label = net.clone().unwrap_or_default();
            match item {
                crate::model::Item::Flash { at, .. } => {
                    let pin = (value(".P", 0), value(".P", 1));
                    if let (Some(refdes), Some(number)) = pin {
                        with_pin += 1;
                        let key = (refdes, number);
                        match table.get(&key) {
                            None => self.differ(
                                name,
                                format!("the pin {},{} is not a pad of the board", key.0, key.1),
                            ),
                            Some(list) => {
                                let found = list.iter().any(|(place, pad_net)| {
                                    near(place.0, at.x, self.tolerance)
                                        && near(place.1, at.y, self.tolerance)
                                        && net.as_ref().is_none_or(|n| n == pad_net)
                                });
                                if !found {
                                    self.differ(
                                        name,
                                        format!(
                                            "the pin {},{} is not where the board has it or is on another net",
                                            key.0, key.1
                                        ),
                                    );
                                }
                            }
                        }
                        if components_wanted {
                            if value(".C", 0).as_deref() == Some(key.0.as_str()) {
                                with_component += 1;
                            } else {
                                self.differ(
                                    name,
                                    format!(
                                        "the component of the pin {},{} is wrong",
                                        key.0, key.1
                                    ),
                                );
                            }
                        }
                    } else {
                        loose.push(label);
                    }
                }
                crate::model::Item::Line { .. } | crate::model::Item::Arc { .. } => {
                    lines.push(label)
                }
                crate::model::Item::Region { .. } => regions.push(label),
            }
        }
        if pins_wanted {
            self.count(name, "pads with a pin attribute", pad_total, with_pin);
        }
        if components_wanted && pins_wanted {
            self.count(
                name,
                "pads with a component attribute",
                pad_total,
                with_component,
            );
        }
        if nets_wanted {
            if without_net > 0 {
                self.differ(name, format!("{without_net} object(s) without a net"));
            }
            let mut vias: Vec<String> = self
                .board
                .vias
                .values()
                .filter(|v| via_on_layer(self.board, v, layer))
                .map(|v| net_label(self.board, Some(v.net)))
                .collect();
            vias.sort_unstable();
            let mut zones: Vec<String> = self
                .board
                .zones
                .values()
                .filter(|z| z.layer == layer && z.outline.len() >= 3)
                .map(|z| net_label(self.board, Some(z.net)))
                .collect();
            zones.extend(region_pads_on(self.board, layer));
            zones.sort_unstable();
            let tracks = track_nets(self.board, layer);
            for (what, mut expected, mut found, applies) in [
                ("vias", vias, loose, pins_wanted),
                ("lines", tracks, lines, true),
                ("regions", zones, regions, true),
            ] {
                expected.sort_unstable();
                found.sort_unstable();
                if applies && expected != found {
                    self.differ(
                        name,
                        format!("the nets of the {what} do not agree with the board"),
                    );
                }
            }
        }
    }

    fn x2_drill(&mut self, name: &str, imported: &ImportedFile) {
        let Some(file) = imported.drill() else {
            return;
        };
        self.x2_file(name, &file.attributes);
        if self.x2.level.has_aperture_attributes() {
            for tool in file.tools.keys() {
                let named = file
                    .tool_attributes
                    .get(tool)
                    .is_some_and(|a| a.contains_key(".AperFunction"));
                if !named {
                    self.differ(name, format!("the tool T{tool} has no function"));
                }
            }
        }
    }

    fn flashes(&mut self, name: &str, expected: Points, gerber: &crate::model::GerberLayer) {
        let found = sorted(
            gerber
                .items
                .iter()
                .filter_map(|i| match i {
                    crate::model::Item::Flash { at, dark: true, .. } => Some((at.x, at.y)),
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

    /// The drill file of the blind (or buried) vias between two layers against the vias of that
    /// span, and the layers it says in its `.FileFunction` against the ones of its role.
    fn drill_span(&mut self, name: &str, span: (usize, usize, bool), imported: &ImportedFile) {
        let Some(file) = imported.drill() else {
            return;
        };
        let (from, to, buried) = span;
        let expected = span_holes(self.board, from, to, buried);
        let total = file.hits.len() + file.slots.len();
        self.count(name, "holes", expected.len(), total);
        match imported.drill_span() {
            Some(found)
                if found.buried == buried
                    && usize::try_from(found.from) == Ok(from)
                    && usize::try_from(found.to) == Ok(to) => {}
            _ => self.differ(
                name,
                "the attribute .FileFunction does not name the layers of the vias".to_owned(),
            ),
        }
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
    Ok(compare_files_with(
        board,
        &set.files,
        settings,
        &profile.gerber.x2,
    ))
}

/// Reads files that were made from `board` (the files of an export, perhaps changed since) and
/// compares them with it. The files are expected to carry the attributes of the default profile,
/// which are the three file attributes of a layer file.
pub fn compare_files(board: &Board, set_files: &[FabFile], settings: &ImportSettings) -> RoundTrip {
    compare_files_with(board, set_files, settings, &X2Profile::default())
}

/// Like [`compare_files`], and also compares the Gerber X2 attributes of the files with what the
/// profile `x2` asks for: the file attributes, the function of each aperture and of each drill
/// tool, and the net, pin and component of the objects, read back and checked against the board.
pub fn compare_files_with(
    board: &Board,
    set_files: &[FabFile],
    settings: &ImportSettings,
    x2: &X2Profile,
) -> RoundTrip {
    let mut checker = Checker {
        board,
        tolerance: settings.compare_tolerance_nm,
        x2,
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
                    checker.x2_layer(&name, *layer, *kind, &imported);
                }
                files.push(imported);
            }
            FileRole::Drill { plated } => {
                let imported = import_file(&name, bytes, settings);
                if let Content::Failed(error) = &imported.content {
                    checker.differ(&name, format!("the file cannot be read back: {error}"));
                } else {
                    checker.drill(&name, *plated, &imported);
                    checker.x2_drill(&name, &imported);
                }
                files.push(imported);
            }
            FileRole::DrillSpan { from, to, buried } => {
                let imported = import_file(&name, bytes, settings);
                if let Content::Failed(error) = &imported.content {
                    checker.differ(&name, format!("the file cannot be read back: {error}"));
                } else {
                    checker.drill_span(&name, (*from, *to, *buried), &imported);
                    checker.x2_drill(&name, &imported);
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
