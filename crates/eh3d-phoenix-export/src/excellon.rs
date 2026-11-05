//! The holes of the board as Excellon drill files, plated and not plated apart.
//!
//! Vias and drilled pads give holes. A hole is plated when it is a via or when its pad appears on a
//! copper layer; a drilled pad that is on no copper layer (a mounting hole) is not plated. The two
//! kinds go to two files, because they are drilled and priced apart. With the option `oblong_as_slot`
//! a drilled oblong pad is a slot (`G85`) of the same tool: the tool is the drill, the slot is as
//! long as the pad is long in proportion to its drill. Tools are numbered from 1 in the order of
//! their diameters; the holes are ordered by tool, then by Y, then by X, so the same board always
//! gives the same bytes. The units are millimetres and the coordinate style is the one of the
//! options.
//!
//! Rounding: a length in nanometres becomes millimetres in floating point and the writer rounds it
//! half to even to the decimals of the style; with six decimals nothing is lost. The length of a slot
//! is the pad length times the drill over the pad width, rounded down to a nanometre.

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_parser_excellon::{ExcellonFile, Hit, Point as EPoint, Slot, Tool, Units};
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, LayerKind, PadRef, PadShape};
use eh3d_writer_excellon::{CoordinateStyle, ExcellonOptions, WriteError};

use crate::codes;
use crate::gerber::GENERATION_SOFTWARE;

/// What the caller chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrillOptions {
    /// How coordinates are written.
    pub style: CoordinateStyle,
    /// Write a drilled oblong pad as a slot instead of a round hole.
    pub oblong_as_slot: bool,
    /// The number of copper layers, for the `.FileFunction` comment (at least 2 is written).
    pub copper_layers: usize,
}

impl Default for DrillOptions {
    fn default() -> Self {
        DrillOptions {
            style: CoordinateStyle::Decimal { decimals: 6 },
            oblong_as_slot: false,
            copper_layers: 2,
        }
    }
}

/// The drill files of a board.
#[derive(Debug, Clone, Default)]
pub struct DrillFiles {
    /// The plated holes, when there are any.
    pub plated: Option<String>,
    /// The holes that are not plated, when there are any.
    pub non_plated: Option<String>,
    /// What the files cannot say.
    pub diagnostics: Vec<Diagnostic>,
}

/// A hole or a slot in nanometres: the sort order is tool, then Y, then X, then the other end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Hole {
    diameter: i64,
    y: i64,
    x: i64,
    /// The other end of a slot, as `(x, y)`.
    to: Option<(i64, i64)>,
}

enum Shape {
    Round,
    Slot((i64, i64), (i64, i64)),
    OffAxis,
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

fn slot_shape(shape: PadShape, drill: i64, total_udeg: i64, x: i64, y: i64) -> Shape {
    let PadShape::Oblong { width, height } = shape else {
        return Shape::Round;
    };
    let (w, h) = (width.get(), height.get());
    let (long, short) = if w >= h { (w, h) } else { (h, w) };
    if short <= 0 || drill <= 0 {
        return Shape::Round;
    }
    let outer =
        i64::try_from(i128::from(long) * i128::from(drill) / i128::from(short)).unwrap_or(i64::MAX);
    if outer <= drill {
        return Shape::Round;
    }
    let quarter = 90_000_000;
    if total_udeg % quarter != 0 {
        return Shape::OffAxis;
    }
    let half = (outer - drill) / 2;
    let along_x = (w >= h) == ((total_udeg / quarter) % 2 == 0);
    if along_x {
        Shape::Slot((x - half, y), (x + half, y))
    } else {
        Shape::Slot((x, y - half), (x, y + half))
    }
}

fn write_one(holes: &[Hole], function: &str, options: &DrillOptions) -> Result<String, WriteError> {
    let mut sorted = holes.to_vec();
    sorted.sort_unstable();
    let mut diameters: Vec<i64> = sorted.iter().map(|h| h.diameter).collect();
    diameters.sort_unstable();
    diameters.dedup();
    let numbers: BTreeMap<i64, u32> = diameters
        .iter()
        .enumerate()
        .map(|(i, d)| (*d, 1 + i as u32))
        .collect();
    let file = ExcellonFile {
        units: Some(Units::Millimetres),
        zeros: None,
        tools: numbers
            .iter()
            .map(|(d, n)| {
                (
                    *n,
                    Tool {
                        number: *n,
                        diameter: mm(*d),
                        feed: None,
                        speed: None,
                    },
                )
            })
            .collect(),
        drills: sorted
            .iter()
            .filter(|h| h.to.is_none())
            .map(|h| Hit {
                line: 0,
                tool: numbers[&h.diameter],
                at: EPoint {
                    x: mm(h.x),
                    y: mm(h.y),
                },
            })
            .collect(),
        slots: sorted
            .iter()
            .filter_map(|h| {
                h.to.map(|(tx, ty)| Slot {
                    line: 0,
                    tool: numbers[&h.diameter],
                    from: EPoint {
                        x: mm(h.x),
                        y: mm(h.y),
                    },
                    to: EPoint {
                        x: mm(tx),
                        y: mm(ty),
                    },
                })
            })
            .collect(),
        routes: Vec::new(),
        ended: true,
        diagnostics: Vec::new(),
    };
    let write_options = ExcellonOptions {
        style: options.style,
        units: None,
        zeros: None,
        comments: vec![
            format!(
                " #@! TF.GenerationSoftware,{}",
                GENERATION_SOFTWARE.join(",")
            ),
            format!(" #@! TF.FileFunction,{function}"),
        ],
    };
    eh3d_writer_excellon::write(&file, &write_options)
}

/// Writes the drill files. A file is `None` when the board has no hole of its kind.
pub fn write_drill_files(board: &Board, options: &DrillOptions) -> Result<DrillFiles, WriteError> {
    let mut diagnostics = Vec::new();
    let mut plated: Vec<Hole> = Vec::new();
    let mut non_plated: Vec<Hole> = Vec::new();
    for v in board.vias.values() {
        plated.push(Hole {
            diameter: v.drill.get(),
            y: v.at.y_nm.get(),
            x: v.at.x_nm.get(),
            to: None,
        });
    }
    let (mut non_round, mut off_axis) = (0usize, 0usize);
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pid, pad) in &footprint.pads {
            let Some(drill) = pad.drill else {
                continue;
            };
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *cid,
                    pad: *pid,
                },
            ) else {
                continue;
            };
            let on_copper = pad.layers.iter().any(|l| {
                board
                    .layers
                    .get(l)
                    .is_some_and(|layer| layer.kind == LayerKind::Copper)
            });
            let total = (pad.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
            let (x, y) = (at.x_nm.get(), at.y_nm.get());
            let shape = if options.oblong_as_slot {
                slot_shape(pad.shape, drill.get(), total, x, y)
            } else {
                Shape::Round
            };
            let hole = match shape {
                Shape::Slot(from, to) => Hole {
                    diameter: drill.get(),
                    y: from.1,
                    x: from.0,
                    to: Some(to),
                },
                Shape::Round | Shape::OffAxis => {
                    if matches!(shape, Shape::OffAxis) {
                        off_axis += 1;
                    }
                    if !matches!(pad.shape, PadShape::Circle { .. }) {
                        non_round += 1;
                    }
                    Hole {
                        diameter: drill.get(),
                        y,
                        x,
                        to: None,
                    }
                }
            };
            if on_copper {
                plated.push(hole);
            } else {
                non_plated.push(hole);
            }
        }
    }
    if non_round > 0 {
        diagnostics.push(codes::diagnostic(
            codes::SLOT_AS_HOLE,
            Severity::Info,
            &format!("{non_round} pad(s)"),
        ));
    }
    if off_axis > 0 {
        diagnostics.push(codes::diagnostic(
            codes::SLOT_ROTATION,
            Severity::Warning,
            &format!("{off_axis} pad(s)"),
        ));
    }
    let layers = options.copper_layers.max(2);
    let plated_text = if plated.is_empty() {
        None
    } else {
        Some(write_one(
            &plated,
            &format!("Plated,1,{layers},PTH"),
            options,
        )?)
    };
    let non_plated_text = if non_plated.is_empty() {
        None
    } else {
        Some(write_one(
            &non_plated,
            &format!("NonPlated,1,{layers},NPTH"),
            options,
        )?)
    };
    Ok(DrillFiles {
        plated: plated_text,
        non_plated: non_plated_text,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::Nm;

    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    #[test]
    fn an_oblong_pad_becomes_a_slot_along_its_long_side() {
        let shape = PadShape::Oblong {
            width: nm(3_000_000),
            height: nm(1_500_000),
        };
        // drill 1 mm in a 1.5 mm wide pad: the slot is 3 * 1 / 1.5 = 2 mm long, centres 1 mm apart
        match slot_shape(shape, 1_000_000, 0, 10_000_000, 5_000_000) {
            Shape::Slot(a, b) => {
                assert_eq!(a, (9_500_000, 5_000_000));
                assert_eq!(b, (10_500_000, 5_000_000));
            }
            _ => panic!("expected a slot"),
        }
        // turned by 90 degrees the slot is vertical
        match slot_shape(shape, 1_000_000, 90_000_000, 0, 0) {
            Shape::Slot(a, b) => assert_eq!((a.0, b.0), (0, 0)),
            _ => panic!("expected a slot"),
        }
        assert!(matches!(
            slot_shape(shape, 1_000_000, 45_000_000, 0, 0),
            Shape::OffAxis
        ));
        // a round pad and a pad as wide as long give round holes
        let round = PadShape::Circle {
            diameter: nm(1_600_000),
        };
        assert!(matches!(slot_shape(round, 800_000, 0, 0, 0), Shape::Round));
    }

    #[test]
    fn holes_sort_by_tool_then_y_then_x() {
        let mut holes = [
            Hole {
                diameter: 2,
                y: 0,
                x: 0,
                to: None,
            },
            Hole {
                diameter: 1,
                y: 5,
                x: 1,
                to: None,
            },
            Hole {
                diameter: 1,
                y: 5,
                x: 0,
                to: None,
            },
        ];
        holes.sort_unstable();
        assert_eq!(
            holes.iter().map(|h| (h.diameter, h.x)).collect::<Vec<_>>(),
            vec![(1, 0), (1, 1), (2, 0)]
        );
    }
}
