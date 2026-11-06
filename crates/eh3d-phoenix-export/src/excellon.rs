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
use eh3d_writer_excellon::{CoordinateStyle, ExcellonOptions};

use crate::codes;
use crate::gerber::{LayerError, GENERATION_SOFTWARE};
use crate::profile::{X2Level, X2Profile};
use crate::x2::{self, DrillFunction, Fields, FileInput};

/// What the caller chooses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillOptions {
    /// How coordinates are written.
    pub style: CoordinateStyle,
    /// Write a drilled oblong pad as a slot instead of a round hole.
    pub oblong_as_slot: bool,
    /// The number of copper layers, for the `.FileFunction` comment (at least 2 is written).
    pub copper_layers: usize,
    /// The Gerber X2 attributes to write.
    pub x2: X2Profile,
    /// The name of the board, for `.ProjectId`.
    pub board_name: String,
}

impl Default for DrillOptions {
    fn default() -> Self {
        DrillOptions {
            style: CoordinateStyle::Decimal { decimals: 6 },
            oblong_as_slot: false,
            copper_layers: 2,
            x2: X2Profile::default(),
            board_name: String::new(),
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
    /// What the hole is for; `None` when the level of X2 does not name it.
    function: Option<DrillFunction>,
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

fn write_one(
    holes: &[Hole],
    plated: bool,
    options: &DrillOptions,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<String, LayerError> {
    let mut sorted = holes.to_vec();
    sorted.sort_unstable();
    let mut kinds: Vec<(i64, Option<DrillFunction>)> =
        sorted.iter().map(|h| (h.diameter, h.function)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let numbers: BTreeMap<(i64, Option<DrillFunction>), u32> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, 1 + i as u32))
        .collect();
    let file = ExcellonFile {
        units: Some(Units::Millimetres),
        zeros: None,
        tools: numbers
            .iter()
            .map(|((d, _), n)| {
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
                tool: numbers[&(h.diameter, h.function)],
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
                    tool: numbers[&(h.diameter, h.function)],
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
    let layers = options.copper_layers.max(2).to_string();
    let mut function = vec![
        if plated { "Plated" } else { "NonPlated" }.to_owned(),
        "1".to_owned(),
        layers,
        if plated { "PTH" } else { "NPTH" }.to_owned(),
    ];
    let mut fields = Fields::new(options.x2.strict);
    let comments = match options.x2.level {
        X2Level::Legacy => vec![
            format!(
                " #@! TF.GenerationSoftware,{}",
                GENERATION_SOFTWARE.join(",")
            ),
            format!(" #@! TF.FileFunction,{}", function.join(",")),
        ],
        X2Level::X1 => Vec::new(),
        _ => {
            function.push(
                if file.slots.is_empty() {
                    "Drill"
                } else {
                    "Mixed"
                }
                .to_owned(),
            );
            let input = FileInput {
                x2: &options.x2,
                board_name: &options.board_name,
                function: Some(function),
                polarity: None,
            };
            x2::drill_comments(&x2::file_attributes(&input, &mut fields)?)
        }
    };
    let write_options = ExcellonOptions {
        style: options.style,
        units: None,
        zeros: None,
        comments,
    };
    let mut text = eh3d_writer_excellon::write(&file, &write_options)?;
    if options.x2.level.has_aperture_attributes() {
        let functions: BTreeMap<u32, DrillFunction> = numbers
            .iter()
            .filter_map(|((_, f), n)| f.map(|f| (*n, f)))
            .collect();
        text = x2::insert_tool_attributes(&text, &functions);
    }
    if fields.escaped() > 0 {
        diagnostics.push(codes::diagnostic(
            codes::ATTRIBUTE_ESCAPED,
            Severity::Info,
            &format!("{} text(s) in one drill file", fields.escaped()),
        ));
    }
    Ok(text)
}

/// Writes the drill files. A file is `None` when the board has no hole of its kind.
pub fn write_drill_files(board: &Board, options: &DrillOptions) -> Result<DrillFiles, LayerError> {
    let named = options.x2.level.has_aperture_attributes();
    let mut diagnostics = Vec::new();
    let mut plated: Vec<Hole> = Vec::new();
    let mut non_plated: Vec<Hole> = Vec::new();
    for v in board.vias.values() {
        plated.push(Hole {
            diameter: v.drill.get(),
            function: named.then_some(DrillFunction::Via),
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
                    function: None,
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
                        function: None,
                        y,
                        x,
                        to: None,
                    }
                }
            };
            let hole = Hole {
                function: named.then_some(if on_copper {
                    DrillFunction::Component
                } else {
                    DrillFunction::Mechanical
                }),
                ..hole
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
    let plated_text = if plated.is_empty() {
        None
    } else {
        Some(write_one(&plated, true, options, &mut diagnostics)?)
    };
    let non_plated_text = if non_plated.is_empty() {
        None
    } else {
        Some(write_one(&non_plated, false, options, &mut diagnostics)?)
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
                function: None,
                y: 0,
                x: 0,
                to: None,
            },
            Hole {
                diameter: 1,
                function: None,
                y: 5,
                x: 1,
                to: None,
            },
            Hole {
                diameter: 1,
                function: None,
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
