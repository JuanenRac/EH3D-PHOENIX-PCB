// SPDX-License-Identifier: GPL-3.0-or-later
//! The layer files of a panel: the copper, the solder mask, the paste and the legend of every
//! copy and the drill files, written by the export of the suite (`eh3d-phoenix-export`) from the
//! board that [`compose`](crate::compose::compose) makes, so the panel has the Gerber X2
//! attributes and the files that the export profile asks for. They are read back with the importer
//! of Gerber and Excellon files (`eh3d-phoenix-gerber`) and compared with the board of the panel
//! (a round trip), and the holes of the board of the panel are compared with the holes of the
//! plan, so the geometry of the panel is the geometry of the copies plus the marks.
//!
//! The outline that is routed and the V-score lines come from [`crate::files`]; the placement file
//! and the bill of materials of the panel are made from the copies only.

use std::collections::BTreeMap;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_export::fab::FileRole;
use eh3d_phoenix_export::{build, Profile};
use eh3d_phoenix_gerber::{compare_files_with, ImportSettings};

use crate::codes::{PanelError, EXPORT_FAILED, ROUNDTRIP_MISMATCH};
use crate::compose::compose;
use crate::files::{panel_files, PanelFile};
use crate::layout::{BoardShape, HoleKind, PanelPlan};
use crate::settings::{Joint, PanelSettings};
use crate::vscore::check_v_score;

/// The files of the layers of a panel and what the export had to say about them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelLayers {
    /// The files, in a fixed order: the layers and the drills as the export orders them, the
    /// placement file, the bill of materials, the outline that is routed and the V-score lines.
    pub files: Vec<PanelFile>,
    /// What the files cannot say (the messages of the export), in English.
    pub messages: Vec<String>,
}

/// Holes by kind, diameter and position in micrometres.
type HolePool = BTreeMap<(bool, i64, i64, i64), Vec<(i64, i64)>>;

/// Compares the holes of the board of the panel with those of the plan: the same plated and not
/// plated holes of the same diameters at the same places (a difference of 2 nm is allowed for a
/// component that is turned by an angle that is not a right angle).
fn same_holes(full: &Board, plan: &PanelPlan) -> Result<(), PanelError> {
    let (shape, _) = BoardShape::from_board(full)?;
    let mut pool: HolePool = BTreeMap::new();
    for (plated, at, diameter) in &shape.holes {
        pool.entry((
            *plated,
            *diameter,
            at.x.div_euclid(1000),
            at.y.div_euclid(1000),
        ))
        .or_default()
        .push((at.x, at.y));
    }
    let mismatch = |what: String| PanelError::new(ROUNDTRIP_MISMATCH, what);
    for hole in &plan.holes {
        let plated = hole.kind == HoleKind::Plated;
        let (bx, by) = (hole.at.x.div_euclid(1000), hole.at.y.div_euclid(1000));
        let mut found = false;
        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                let key = (plated, hole.diameter, bx + dx, by + dy);
                if let Some(list) = pool.get_mut(&key) {
                    let near = list.iter().position(|(x, y)| {
                        (x - hole.at.x).abs() <= 2 && (y - hole.at.y).abs() <= 2
                    });
                    if let Some(position) = near {
                        list.swap_remove(position);
                        found = true;
                        break 'search;
                    }
                }
            }
        }
        if !found {
            return Err(mismatch(format!(
                "the plan has a hole of {} nm at ({}, {}) that the board of the panel does not have",
                hole.diameter, hole.at.x, hole.at.y
            )));
        }
    }
    let left: usize = pool.values().map(Vec::len).sum();
    if left > 0 {
        return Err(mismatch(format!(
            "the board of the panel has {left} hole(s) that the plan does not have"
        )));
    }
    Ok(())
}

/// Writes the layer files of the panel of `plan`, made of the `boards` that the plan names.
/// `profile` is the export profile: the format of the files, the names of the drill files, the
/// expansion of the solder mask and the Gerber X2 attributes.
///
/// A V-score line closer than `v_score_clearance_nm` to the copper of a board is refused
/// (`PHOENIX-PANEL-015`), the boards need the same layers (`PHOENIX-PANEL-018`) and files that
/// do not read back as the board of the panel are refused (`PHOENIX-PANEL-019`).
pub fn panel_layers(
    boards: &[Board],
    plan: &PanelPlan,
    settings: &PanelSettings,
    profile: &Profile,
) -> Result<PanelLayers, PanelError> {
    let expansion = profile.gerber.mask_expansion_nm;
    let copies = compose(boards, plan, settings, expansion, false)?;
    if settings.joint == Joint::VScore {
        check_v_score(&copies, plan, settings.v_score_clearance_nm)?;
    }
    let full = compose(boards, plan, settings, expansion, true)?;
    same_holes(&full, plan)?;
    let export_failed =
        |e: eh3d_phoenix_export::ExportError| PanelError::new(EXPORT_FAILED, e.to_string());
    let set = build(&full, profile).map_err(export_failed)?;
    let trip = compare_files_with(
        &full,
        &set.files,
        &ImportSettings::default(),
        &profile.gerber.x2,
    );
    if !trip.differences.is_empty() {
        let shown: Vec<String> = trip
            .differences
            .iter()
            .take(5)
            .map(|d| format!("{}: {}", d.file, d.what))
            .collect();
        return Err(PanelError::new(ROUNDTRIP_MISMATCH, shown.join("; ")));
    }
    let assembly =
        build(&copies, profile).map_err(|e| PanelError::new(EXPORT_FAILED, e.to_string()))?;

    let mut files: Vec<PanelFile> = Vec::new();
    for file in &set.files {
        if matches!(
            file.role,
            FileRole::Layer { .. } | FileRole::Drill { .. } | FileRole::DrillSpan { .. }
        ) {
            files.push(PanelFile {
                name: format!("panel_{}", file.folder_name()),
                text: file.text().to_owned(),
            });
        }
    }
    for file in &assembly.files {
        if matches!(file.role, FileRole::Placement | FileRole::Bom) {
            files.push(PanelFile {
                name: format!("panel_{}", file.folder_name()),
                text: file.text().to_owned(),
            });
        }
    }
    for file in panel_files(plan, settings)? {
        if file.name == "panel_profile.gbr" || file.name == "panel_vscore.gbr" {
            files.push(file);
        }
    }
    let messages = set
        .diagnostics
        .iter()
        .map(|d| format!("{}: {}", d.code.as_deref().unwrap_or(""), d.message))
        .collect();
    Ok(PanelLayers { files, messages })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::plan;
    use eh3d_phoenix_board::{
        Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
        Track, Via,
    };
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

    const MM: i64 = 1_000_000;

    fn id(n: u64) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x * MM, y * MM).unwrap()
    }

    /// A board of 30 by 20 mm: two copper layers, a mask and a legend on the top, one resistor, a
    /// header with a drilled pad, a track and a via.
    fn sample() -> Board {
        let (top, bottom, mask_top, silk_top, outline_layer) = (id(1), id(2), id(3), id(4), id(5));
        let mut b = Board {
            name: "sample".to_owned(),
            outline: vec![pt(0, 0), pt(30, 0), pt(30, 20), pt(0, 20)],
            ..Board::default()
        };
        for (layer, name, kind, side) in [
            (top, "Top", LayerKind::Copper, Some(Side::Top)),
            (bottom, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
            (mask_top, "Mask top", LayerKind::SolderMask, Some(Side::Top)),
            (
                silk_top,
                "Legend top",
                LayerKind::Silkscreen,
                Some(Side::Top),
            ),
            (outline_layer, "Outline", LayerKind::Outline, None),
        ] {
            b.layers.insert(
                layer,
                Layer {
                    name: name.to_owned(),
                    kind,
                    side,
                },
            );
        }
        b.copper_stack = vec![top, bottom];
        let net = id(10);
        b.nets.insert(
            net,
            Net {
                name: "SIG".to_owned(),
            },
        );
        let pad = |number: &str, x: i64, drill: Option<i64>, layers: Vec<Id>| Pad {
            number: number.to_owned(),
            shape: PadShape::Rect {
                width: Nm::new(1_200_000).unwrap(),
                height: Nm::new(1_000_000).unwrap(),
            },
            at: Point::from_nm(x, 0).unwrap(),
            rotation: Udeg::ZERO,
            drill: drill.map(|d| Nm::new(d).unwrap()),
            layers,
            roundness: 0,
            plated: true,
        };
        b.footprints.insert(
            id(20),
            Footprint {
                name: "R0603".to_owned(),
                pads: BTreeMap::from([
                    (id(30), pad("1", -800_000, None, vec![top, mask_top])),
                    (id(31), pad("2", 800_000, None, vec![top, mask_top])),
                ]),
            },
        );
        b.footprints.insert(
            id(21),
            Footprint {
                name: "HEADER".to_owned(),
                pads: BTreeMap::from([(
                    id(32),
                    pad("1", 0, Some(900_000), vec![top, bottom, mask_top]),
                )]),
            },
        );
        for (cid, refdes, footprint, at) in [
            (id(40), "R1", id(20), pt(10, 5)),
            (id(41), "J1", id(21), pt(25, 15)),
        ] {
            b.components.insert(
                cid,
                Component {
                    refdes: refdes.to_owned(),
                    value: String::new(),
                    symbol: None,
                    footprint,
                    at,
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            );
        }
        b.pad_nets = vec![
            PadNet {
                component: id(40),
                pad: id(30),
                net,
            },
            PadNet {
                component: id(41),
                pad: id(32),
                net,
            },
        ];
        b.pad_nets.sort_by_key(PadNet::pad_ref);
        b.tracks.insert(
            id(50),
            Track {
                net,
                layer: top,
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(10, 5),
                    end: pt(25, 15),
                }],
            },
        );
        b.vias.insert(
            id(51),
            Via {
                net,
                at: pt(5, 12),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: top,
                to_layer: bottom,
            },
        );
        b
    }

    fn layers_of(text: &str, boards: &[Board]) -> PanelLayers {
        let settings = PanelSettings::parse(text).unwrap();
        let (shape, _) = BoardShape::from_board(&boards[0]).unwrap();
        let panel = plan(&shape, &settings).unwrap();
        panel_layers(boards, &panel, &settings, &Profile::default()).unwrap()
    }

    fn file<'a>(layers: &'a PanelLayers, name: &str) -> &'a str {
        &layers
            .files
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("no file {name}"))
            .text
    }

    #[test]
    fn the_panel_gets_the_copper_mask_legend_and_drills_of_every_copy() {
        let layers = layers_of("columns = 3\nrows = 2\n", &[sample()]);
        let names: Vec<&str> = layers.files.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"panel_02_top.gbr"), "{names:?}");
        assert!(names.contains(&"panel_profile.gbr"), "{names:?}");
        assert!(names.contains(&"panel_placement.csv"), "{names:?}");
        // 6 copies of two pads and a drilled pad on the top copper, 6 vias, and 3 fiducials
        let top = file(&layers, "panel_02_top.gbr");
        let flashes = top.matches("D03*").count();
        assert_eq!(flashes, 6 * 3 + 6 + 3, "{top}");
        // the placement file lists the resistors and the headers of the six copies
        let placement = file(&layers, "panel_placement.csv");
        assert_eq!(placement.matches("R0603").count(), 6, "{placement}");
        assert_eq!(placement.matches("HEADER").count(), 6, "{placement}");
        // the drill files hold the vias and the header pads, and the mouse bites and tooling holes
        let plated = file(&layers, "panel_drill_pth.drl");
        assert!(plated.contains("M30"), "{plated}");
        assert!(file(&layers, "panel_drill_npth.drl").contains("M30"));
    }

    #[test]
    fn a_mirrored_matrix_reads_back_as_the_board_of_the_panel() {
        let layers = layers_of(
            "columns = 2\nrows = 2\ncopy_mirror = true\ncopy_rotation_deg = 90\nlocal_fiducial_count = 2\n",
            &[sample()],
        );
        assert!(layers.files.iter().any(|f| f.name == "panel_01_bottom.gbr"));
    }

    #[test]
    fn the_same_plan_always_gives_the_same_files() {
        let one = layers_of("columns = 2\nrows = 1\ncopy_suffix = true\n", &[sample()]);
        let two = layers_of("columns = 2\nrows = 1\ncopy_suffix = true\n", &[sample()]);
        assert_eq!(one, two);
        assert!(file(&one, "panel_placement.csv").contains("R1_2"));
    }

    #[test]
    fn a_v_score_panel_refuses_a_line_through_the_copper() {
        let settings = PanelSettings::parse("joint = v_score\ngap_nm = 0\nframe = none\n").unwrap();
        let mut board = sample();
        // a via on the edge of the board, where the V-score line runs
        board.vias.insert(
            id(60),
            Via {
                net: id(10),
                at: pt(30, 10),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: id(1),
                to_layer: id(2),
            },
        );
        let (shape, _) = BoardShape::from_board(&board).unwrap();
        let panel = plan(&shape, &settings).unwrap();
        let e = panel_layers(&[board], &panel, &settings, &Profile::default()).unwrap_err();
        assert_eq!(e.code, crate::codes::VSCORE_NEAR_COPPER);
    }

    #[test]
    fn the_holes_of_the_plan_and_of_the_board_of_the_panel_are_compared() {
        let board = sample();
        let settings = PanelSettings::parse("columns = 2\nrows = 1\n").unwrap();
        let (shape, _) = BoardShape::from_board(&board).unwrap();
        let mut panel = plan(&shape, &settings).unwrap();
        let full = compose(&[board], &panel, &settings, 0, true).unwrap();
        assert!(same_holes(&full, &panel).is_ok());
        panel.holes.pop();
        let e = same_holes(&full, &panel).unwrap_err();
        assert_eq!(e.code, ROUNDTRIP_MISMATCH);
    }
}
