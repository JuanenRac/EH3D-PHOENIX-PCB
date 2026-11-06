// SPDX-License-Identifier: GPL-3.0-or-later
//! The check of the V-score lines of a panel: no line may pass closer than a distance to a pad, a
//! track, a via or a zone of the boards. A V-score line runs from edge to edge of the panel, so
//! only the distance across the line counts. Every object is held in a box that is a little
//! larger than the object (a pad turned by an angle that is not a right angle is held in its
//! circle, an arc in the box of its ends widened by its chord), so a line that passes close to a
//! curved piece may be refused although it is clear.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, LayerKind, PadRef, PadShape, Segment};

use crate::codes::{PanelError, VSCORE_NEAR_COPPER};
use crate::files::mm;
use crate::layout::PanelPlan;

/// A box around a piece of copper, in nanometres, and what it is.
struct CopperBox {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    what: &'static str,
}

fn around(x: i64, y: i64, half_x: i64, half_y: i64, what: &'static str) -> CopperBox {
    CopperBox {
        x0: x - half_x,
        y0: y - half_y,
        x1: x + half_x,
        y1: y + half_y,
        what,
    }
}

fn isqrt_up(value: i128) -> i64 {
    let mut root = (value as f64).sqrt() as i128;
    while root * root < value {
        root += 1;
    }
    i64::try_from(root).unwrap_or(i64::MAX)
}

/// The boxes around the copper of a board.
fn copper_boxes(board: &Board) -> Vec<CopperBox> {
    let mut boxes = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            let on_copper = pad.layers.iter().any(|l| {
                board
                    .layers
                    .get(l)
                    .is_some_and(|layer| layer.kind == LayerKind::Copper)
            });
            if !on_copper {
                continue;
            }
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *component_id,
                    pad: *pad_id,
                },
            ) else {
                continue;
            };
            let (w, h) = match pad.shape {
                PadShape::Circle { diameter } => (diameter.get(), diameter.get()),
                PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
                    (width.get(), height.get())
                }
            };
            let total = (pad.rotation.get() + component.rotation.get()).rem_euclid(360_000_000);
            let (half_x, half_y) = if total % 90_000_000 == 0 {
                if (total / 90_000_000) % 2 == 1 {
                    ((h + 1) / 2, (w + 1) / 2)
                } else {
                    ((w + 1) / 2, (h + 1) / 2)
                }
            } else {
                let r = isqrt_up(i128::from(w) * i128::from(w) + i128::from(h) * i128::from(h));
                ((r + 1) / 2, (r + 1) / 2)
            };
            boxes.push(around(
                at.x_nm.get(),
                at.y_nm.get(),
                half_x,
                half_y,
                "a pad",
            ));
        }
    }
    for via in board.vias.values() {
        let half = (via.diameter.get() + 1) / 2;
        boxes.push(around(
            via.at.x_nm.get(),
            via.at.y_nm.get(),
            half,
            half,
            "a via",
        ));
    }
    for track in board.tracks.values() {
        let half = (track.width.get() + 1) / 2;
        for piece in &track.path {
            let (a, b) = (piece.start(), piece.end());
            let widen = match piece {
                Segment::Line { .. } => 0,
                Segment::Arc { .. } => {
                    let dx = i128::from(a.x_nm.get() - b.x_nm.get());
                    let dy = i128::from(a.y_nm.get() - b.y_nm.get());
                    isqrt_up(dx * dx + dy * dy)
                }
            };
            boxes.push(CopperBox {
                x0: a.x_nm.get().min(b.x_nm.get()) - half - widen,
                y0: a.y_nm.get().min(b.y_nm.get()) - half - widen,
                x1: a.x_nm.get().max(b.x_nm.get()) + half + widen,
                y1: a.y_nm.get().max(b.y_nm.get()) + half + widen,
                what: "a track",
            });
        }
    }
    for zone in board.zones.values() {
        let xs = zone.outline.iter().map(|p| p.x_nm.get());
        let ys = zone.outline.iter().map(|p| p.y_nm.get());
        if let (Some(x0), Some(x1), Some(y0), Some(y1)) =
            (xs.clone().min(), xs.max(), ys.clone().min(), ys.max())
        {
            boxes.push(CopperBox {
                x0,
                y0,
                x1,
                y1,
                what: "a zone",
            });
        }
    }
    boxes
}

/// Checks the V-score lines of `plan` against the copper of `board` (the board of the panel).
/// A clearance of 0 turns the check off. The error names the first line that is too close.
pub fn check_v_score(board: &Board, plan: &PanelPlan, clearance_nm: i64) -> Result<(), PanelError> {
    if clearance_nm <= 0 || plan.score.is_empty() {
        return Ok(());
    }
    let boxes = copper_boxes(board);
    for line in &plan.score {
        let vertical = line.from.x == line.to.x;
        let at = if vertical { line.from.x } else { line.from.y };
        for b in &boxes {
            let (low, high) = if vertical { (b.x0, b.x1) } else { (b.y0, b.y1) };
            let distance = if at < low {
                low - at
            } else if at > high {
                at - high
            } else {
                0
            };
            if distance < clearance_nm {
                return Err(PanelError::new(
                    VSCORE_NEAR_COPPER,
                    format!(
                        "the line at {} = {} mm is {} nm from {}, the clearance is {} nm",
                        if vertical { "x" } else { "y" },
                        mm(at),
                        distance,
                        b.what,
                        clearance_nm
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Line, Pt};
    use eh3d_phoenix_board::{Id, Layer, Net, Via};
    use eh3d_phoenix_model::{Nm, ObjectId, Point};

    fn id(n: u64) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n).unwrap()
    }

    fn board_with_via(x: i64) -> Board {
        let mut b = Board::default();
        b.layers.insert(
            id(1),
            Layer {
                name: "Top".to_owned(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.nets.insert(
            id(2),
            Net {
                name: "N".to_owned(),
            },
        );
        b.vias.insert(
            id(3),
            Via {
                net: id(2),
                at: Point::from_nm(x, 5_000_000).unwrap(),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: id(1),
                to_layer: id(1),
            },
        );
        b
    }

    fn plan_with_line_at(x: i64) -> PanelPlan {
        let settings =
            crate::settings::PanelSettings::parse("joint = v_score\ngap_nm = 0\n").unwrap();
        let shape = crate::layout::BoardShape {
            origin: Pt { x: 0, y: 0 },
            width: 30_000_000,
            height: 20_000_000,
            holes: Vec::new(),
        };
        let mut plan = crate::layout::plan(&shape, &settings).unwrap();
        plan.score = vec![Line {
            from: Pt { x, y: 0 },
            to: Pt { x, y: 20_000_000 },
        }];
        plan
    }

    #[test]
    fn a_line_closer_than_the_clearance_to_a_via_is_refused() {
        let plan = plan_with_line_at(10_000_000);
        // the via is 0.3 mm in radius; at 10.5 mm its edge is 0.2 mm from the line
        let near = board_with_via(10_500_000);
        let e = check_v_score(&near, &plan, 300_000).unwrap_err();
        assert_eq!(e.code, VSCORE_NEAR_COPPER);
        assert!(e.detail.contains("a via"), "{}", e.detail);
        assert!(e.detail.contains("x = 10.0 mm"), "{}", e.detail);
        // a line through the via is refused too
        assert!(check_v_score(&board_with_via(10_000_000), &plan, 300_000).is_err());
        // 0.6 mm from the edge of the via is clear
        let clear = board_with_via(10_900_000);
        assert!(check_v_score(&clear, &plan, 300_000).is_ok());
        // a clearance of 0 turns the check off
        assert!(check_v_score(&near, &plan, 0).is_ok());
    }

    #[test]
    fn a_horizontal_line_is_checked_across_y() {
        let mut plan = plan_with_line_at(10_000_000);
        plan.score = vec![Line {
            from: Pt { x: 0, y: 5_200_000 },
            to: Pt {
                x: 30_000_000,
                y: 5_200_000,
            },
        }];
        let b = board_with_via(1_000_000);
        let e = check_v_score(&b, &plan, 300_000).unwrap_err();
        assert!(e.detail.contains("y = 5.2 mm"), "{}", e.detail);
    }
}
