// SPDX-License-Identifier: GPL-3.0-or-later
//! The layout of a panel: where the boards go, the lines of the outline that is routed, the tabs
//! with their rows of mouse bite holes or the V-score lines, the fiducials and the tooling holes.
//! All arithmetic is in integer nanometres (products in `i128`), so the same board and settings
//! always give the same panel.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, LayerKind, PadRef};

use crate::codes::{
    Note, PanelError, BITES_DONT_FIT, GAP_IGNORED, MARKS_OVERLAP, MARKS_SKIPPED, MARK_DOESNT_FIT,
    OUTLINE_NOT_RECTANGLE, OUTLINE_UNUSABLE, PANEL_TOO_LARGE, TABS_DONT_FIT,
};
use crate::settings::{Joint, PanelSettings};

/// A point in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pt {
    /// Horizontal coordinate.
    pub x: i64,
    /// Vertical coordinate.
    pub y: i64,
}

/// A straight line of the outline or of a V-score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    /// Start.
    pub from: Pt,
    /// End.
    pub to: Pt,
}

/// What a hole of the panel is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleKind {
    /// A hole of a board that is plated (a via or a pad on copper).
    Plated,
    /// A hole of a board that is not plated.
    NonPlated,
    /// A hole of a row that breaks a tab.
    MouseBite,
    /// A tooling hole of the frame.
    Tooling,
}

/// A hole of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hole {
    /// What it is.
    pub kind: HoleKind,
    /// Its centre.
    pub at: Pt,
    /// Its diameter.
    pub diameter: i64,
}

/// A fiducial of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    /// Its centre.
    pub at: Pt,
    /// The diameter of the copper.
    pub copper_diameter: i64,
    /// The diameter of the solder mask opening.
    pub mask_diameter: i64,
}

/// What the panel needs to know of a board: the size of its outline and its holes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardShape {
    /// The lower left corner of the box around the outline.
    pub origin: Pt,
    /// The width of the box.
    pub width: i64,
    /// The height of the box.
    pub height: i64,
    /// The holes in the coordinates of the board, with whether each is plated.
    pub holes: Vec<(bool, Pt, i64)>,
}

impl BoardShape {
    /// Reads the outline and the holes of a board. The outline needs at least three points and a
    /// box with a width and a height; one that is not a rectangle is used by its box, with a note.
    pub fn from_board(board: &Board) -> Result<(BoardShape, Vec<Note>), PanelError> {
        let mut notes = Vec::new();
        let outline = &board.outline;
        if outline.len() < 3 {
            return Err(PanelError::new(
                OUTLINE_UNUSABLE,
                "the outline has fewer than three points",
            ));
        }
        let xs = outline.iter().map(|p| p.x_nm.get());
        let ys = outline.iter().map(|p| p.y_nm.get());
        let (min_x, max_x) = (xs.clone().min().unwrap_or(0), xs.max().unwrap_or(0));
        let (min_y, max_y) = (ys.clone().min().unwrap_or(0), ys.max().unwrap_or(0));
        let (width, height) = (max_x - min_x, max_y - min_y);
        if width <= 0 || height <= 0 {
            return Err(PanelError::new(
                OUTLINE_UNUSABLE,
                "the box around the outline has no width or no height",
            ));
        }
        let mut corners: Vec<(i64, i64)> = outline
            .iter()
            .map(|p| (p.x_nm.get(), p.y_nm.get()))
            .collect();
        if corners.len() > 1 && corners.first() == corners.last() {
            corners.pop();
        }
        let on_corners = corners.len() == 4
            && corners
                .iter()
                .all(|(x, y)| (*x == min_x || *x == max_x) && (*y == min_y || *y == max_y));
        if !on_corners {
            notes.push(Note {
                code: OUTLINE_NOT_RECTANGLE,
                detail: "the box around the outline is used".to_owned(),
            });
        }
        let mut holes = Vec::new();
        for via in board.vias.values() {
            holes.push((
                true,
                Pt {
                    x: via.at.x_nm.get(),
                    y: via.at.y_nm.get(),
                },
                via.drill.get(),
            ));
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
                let plated = pad.layers.iter().any(|l| {
                    board
                        .layers
                        .get(l)
                        .is_some_and(|layer| layer.kind == LayerKind::Copper)
                });
                holes.push((
                    plated,
                    Pt {
                        x: at.x_nm.get(),
                        y: at.y_nm.get(),
                    },
                    drill.get(),
                ));
            }
        }
        Ok((
            BoardShape {
                origin: Pt { x: min_x, y: min_y },
                width,
                height,
                holes,
            },
            notes,
        ))
    }
}

/// The panel: its size, where each board is and everything that is drawn or drilled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelPlan {
    /// The width of the panel.
    pub width: i64,
    /// The height of the panel.
    pub height: i64,
    /// The boards across.
    pub columns: u32,
    /// The boards up.
    pub rows: u32,
    /// The lower left corner of the box of each board, row by row from the bottom.
    pub origins: Vec<Pt>,
    /// The lines of the outline that is routed: the edge of the panel and the edges of the boards
    /// that have a neighbour, broken where a tab is.
    pub profile: Vec<Line>,
    /// The V-score lines, from edge to edge of the panel.
    pub score: Vec<Line>,
    /// The centres of the tabs.
    pub tabs: Vec<Pt>,
    /// Every hole: those of the boards, the mouse bites and the tooling holes.
    pub holes: Vec<Hole>,
    /// The fiducials.
    pub fiducials: Vec<Mark>,
    /// What was changed or left out.
    pub notes: Vec<Note>,
}

fn too_large(what: &str) -> PanelError {
    PanelError::new(PANEL_TOO_LARGE, what.to_owned())
}

fn to_i64(value: i128) -> Result<i64, PanelError> {
    i64::try_from(value).map_err(|_| too_large("a size does not fit"))
}

/// The centres of `count` tabs of `width` along an edge of `length`, from the start of the edge.
fn tab_centres(length: i64, count: u32, width: i64) -> Result<Vec<i64>, PanelError> {
    let n = i128::from(count);
    if i128::from(width) * n > i128::from(length) {
        return Err(PanelError::new(
            TABS_DONT_FIT,
            format!("{count} tabs of {width} nm do not fit on an edge of {length} nm"),
        ));
    }
    let mut out = Vec::new();
    for k in 0..i128::from(count) {
        out.push(to_i64(i128::from(length) * (2 * k + 1) / (2 * n))?);
    }
    Ok(out)
}

/// The pieces of an edge that stay between its tabs, as distances from its start.
fn edge_pieces(length: i64, centres: &[i64], width: i64) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut position = 0;
    for centre in centres {
        let start = centre - width / 2;
        if start > position {
            out.push((position, start));
        }
        position = start + width;
    }
    if length > position {
        out.push((position, length));
    }
    out
}

/// The row of mouse bite holes of one tab: `count` holes `pitch` apart, centred on `centre`.
fn bite_offsets(count: u32, pitch: i64) -> Vec<i64> {
    let n = i128::from(count);
    (0..i128::from(count))
        .map(|k| ((2 * k - (n - 1)) * i128::from(pitch) / 2) as i64)
        .collect()
}

struct Corner {
    x: i64,
    y: i64,
    right: bool,
    top: bool,
}

fn corners(width: i64, height: i64, margin: i64) -> [Corner; 4] {
    [
        Corner {
            x: margin,
            y: margin,
            right: false,
            top: false,
        },
        Corner {
            x: width - margin,
            y: margin,
            right: true,
            top: false,
        },
        Corner {
            x: width - margin,
            y: height - margin,
            right: true,
            top: true,
        },
        Corner {
            x: margin,
            y: height - margin,
            right: false,
            top: true,
        },
    ]
}

/// The corners that carry `count` marks: the first three in the order lower left, lower right,
/// upper right, upper left, or the diagonal for two.
fn corner_indices(count: u32) -> &'static [usize] {
    match count {
        2 => &[0, 2],
        3 => &[0, 1, 2],
        4 => &[0, 1, 2, 3],
        _ => &[],
    }
}

/// A circle of the frame that must lie inside a rail: its centre and radius.
struct Circle {
    at: Pt,
    radius: i64,
}

fn inside_rail(c: &Circle, plan_w: i64, plan_h: i64, rail_x: i64, rail_y: i64) -> bool {
    let (x, y, r) = (c.at.x, c.at.y, c.radius);
    let in_x = rail_x > 0
        && ((x - r >= 0 && x + r <= rail_x) || (x - r >= plan_w - rail_x && x + r <= plan_w));
    let in_y = rail_y > 0
        && ((y - r >= 0 && y + r <= rail_y) || (y - r >= plan_h - rail_y && y + r <= plan_h));
    in_x || in_y
}

/// Lays the boards out. `shape` is the board; `settings` the matrix, the frame, the joint and the
/// marks.
pub fn plan(shape: &BoardShape, settings: &PanelSettings) -> Result<PanelPlan, PanelError> {
    settings.validate()?;
    let mut notes: Vec<Note> = Vec::new();
    let v_score = settings.joint == Joint::VScore;
    let gap = if v_score { 0 } else { settings.gap_nm };
    if v_score && settings.gap_nm != 0 {
        notes.push(Note {
            code: GAP_IGNORED,
            detail: format!("gap_nm = {} is not used", settings.gap_nm),
        });
    }
    let (columns, rows) = (i128::from(settings.columns), i128::from(settings.rows));
    if columns * rows > i128::from(settings.max_boards) {
        return Err(too_large(&format!(
            "{} boards, the limit is {}",
            columns * rows,
            settings.max_boards
        )));
    }
    let (bw, bh) = (shape.width, shape.height);
    let left_right = settings.frame.has_left_right();
    let top_bottom = settings.frame.has_top_bottom();
    let rail_x = if left_right {
        settings.frame_width_nm
    } else {
        0
    };
    let rail_y = if top_bottom {
        settings.frame_width_nm
    } else {
        0
    };
    let start_x = if left_right { rail_x + gap } else { 0 };
    let start_y = if top_bottom { rail_y + gap } else { 0 };
    let width =
        2 * i128::from(start_x) + columns * i128::from(bw) + (columns - 1) * i128::from(gap);
    let height = 2 * i128::from(start_y) + rows * i128::from(bh) + (rows - 1) * i128::from(gap);
    let limit = i128::from(settings.max_panel_nm);
    if width > limit || height > limit {
        return Err(too_large(&format!(
            "{width} nm by {height} nm, the limit is {limit} nm"
        )));
    }
    let (width, height) = (to_i64(width)?, to_i64(height)?);
    let pitch_x = to_i64(i128::from(bw) + i128::from(gap))?;
    let pitch_y = to_i64(i128::from(bh) + i128::from(gap))?;
    let (cols, rws) = (settings.columns, settings.rows);

    let mut origins = Vec::new();
    for r in 0..rws {
        for c in 0..cols {
            origins.push(Pt {
                x: start_x + i64::from(c) * pitch_x,
                y: start_y + i64::from(r) * pitch_y,
            });
        }
    }

    let mut profile: Vec<Line> = Vec::new();
    let mut score: Vec<Line> = Vec::new();
    let mut tabs: Vec<Pt> = Vec::new();
    let mut holes: Vec<Hole> = Vec::new();
    for origin in &origins {
        for (plated, at, diameter) in &shape.holes {
            holes.push(Hole {
                kind: if *plated {
                    HoleKind::Plated
                } else {
                    HoleKind::NonPlated
                },
                at: Pt {
                    x: origin.x + (at.x - shape.origin.x),
                    y: origin.y + (at.y - shape.origin.y),
                },
                diameter: *diameter,
            });
        }
    }
    let outer = [
        (Pt { x: 0, y: 0 }, Pt { x: width, y: 0 }),
        (
            Pt { x: width, y: 0 },
            Pt {
                x: width,
                y: height,
            },
        ),
        (
            Pt {
                x: width,
                y: height,
            },
            Pt { x: 0, y: height },
        ),
        (Pt { x: 0, y: height }, Pt { x: 0, y: 0 }),
    ];
    for (from, to) in outer {
        profile.push(Line { from, to });
    }

    if v_score {
        for j in 0..=cols {
            if (j > 0 && j < cols) || left_right {
                let x = start_x + i64::from(j) * pitch_x;
                score.push(Line {
                    from: Pt { x, y: 0 },
                    to: Pt { x, y: height },
                });
            }
        }
        for k in 0..=rws {
            if (k > 0 && k < rws) || top_bottom {
                let y = start_y + i64::from(k) * pitch_y;
                score.push(Line {
                    from: Pt { x: 0, y },
                    to: Pt { x: width, y },
                });
            }
        }
    } else {
        let n = settings.tabs_per_edge;
        let tw = settings.tab_width_nm;
        let span = i128::from(settings.mouse_bite_holes - 1)
            * i128::from(settings.mouse_bite_pitch_nm)
            + i128::from(settings.mouse_bite_drill_nm);
        if span > i128::from(tw) {
            return Err(PanelError::new(
                BITES_DONT_FIT,
                format!(
                    "{} holes take {span} nm and the tab is {tw} nm wide",
                    settings.mouse_bite_holes
                ),
            ));
        }
        let along_y = if cols > 1 || left_right {
            tab_centres(bh, n, tw)?
        } else {
            Vec::new()
        };
        let along_x = if rws > 1 || top_bottom {
            tab_centres(bw, n, tw)?
        } else {
            Vec::new()
        };
        let offsets = bite_offsets(settings.mouse_bite_holes, settings.mouse_bite_pitch_nm);
        let bite = |at: Pt| Hole {
            kind: HoleKind::MouseBite,
            at,
            diameter: settings.mouse_bite_drill_nm,
        };
        // the gaps between columns (and between a column and a rail): tabs run along y
        for j in 0..=cols {
            if !((j > 0 && j < cols) || left_right) {
                continue;
            }
            let gap_x = start_x + i64::from(j) * pitch_x - gap;
            for r in 0..rws {
                let edge_start = start_y + i64::from(r) * pitch_y;
                for centre in &along_y {
                    let y = edge_start + centre;
                    tabs.push(Pt {
                        x: gap_x + gap / 2,
                        y,
                    });
                    for offset in &offsets {
                        holes.push(bite(Pt {
                            x: gap_x + gap / 2,
                            y: y + offset,
                        }));
                    }
                }
            }
        }
        // the gaps between rows (and between a row and a rail): tabs run along x
        for k in 0..=rws {
            if !((k > 0 && k < rws) || top_bottom) {
                continue;
            }
            let gap_y = start_y + i64::from(k) * pitch_y - gap;
            for c in 0..cols {
                let edge_start = start_x + i64::from(c) * pitch_x;
                for centre in &along_x {
                    let x = edge_start + centre;
                    tabs.push(Pt {
                        x,
                        y: gap_y + gap / 2,
                    });
                    for offset in &offsets {
                        holes.push(bite(Pt {
                            x: x + offset,
                            y: gap_y + gap / 2,
                        }));
                    }
                }
            }
        }
        // the edges of the boards that have a neighbour, broken where the tabs are
        for (index, origin) in origins.iter().enumerate() {
            let c = (index as u32) % cols;
            let r = (index as u32) / cols;
            let vertical = edge_pieces(bh, &along_y, tw);
            let horizontal = edge_pieces(bw, &along_x, tw);
            if c > 0 || left_right {
                for (a, b) in &vertical {
                    profile.push(Line {
                        from: Pt {
                            x: origin.x,
                            y: origin.y + a,
                        },
                        to: Pt {
                            x: origin.x,
                            y: origin.y + b,
                        },
                    });
                }
            }
            if c + 1 < cols || left_right {
                for (a, b) in &vertical {
                    profile.push(Line {
                        from: Pt {
                            x: origin.x + bw,
                            y: origin.y + a,
                        },
                        to: Pt {
                            x: origin.x + bw,
                            y: origin.y + b,
                        },
                    });
                }
            }
            if r > 0 || top_bottom {
                for (a, b) in &horizontal {
                    profile.push(Line {
                        from: Pt {
                            x: origin.x + a,
                            y: origin.y,
                        },
                        to: Pt {
                            x: origin.x + b,
                            y: origin.y,
                        },
                    });
                }
            }
            if r + 1 < rws || top_bottom {
                for (a, b) in &horizontal {
                    profile.push(Line {
                        from: Pt {
                            x: origin.x + a,
                            y: origin.y + bh,
                        },
                        to: Pt {
                            x: origin.x + b,
                            y: origin.y + bh,
                        },
                    });
                }
            }
        }
    }

    // the marks of the frame
    let wants_marks = settings.tooling_hole_count > 0 || settings.fiducial_count > 0;
    let mut fiducials: Vec<Mark> = Vec::new();
    if wants_marks && settings.frame == crate::settings::Frame::None {
        notes.push(Note {
            code: MARKS_SKIPPED,
            detail: "set frame to put fiducials and tooling holes on the panel".to_owned(),
        });
    } else if wants_marks {
        let margin = settings.tooling_hole_margin_nm;
        let places = corners(width, height, margin);
        let mut circles: Vec<Circle> = Vec::new();
        for index in corner_indices(settings.tooling_hole_count) {
            let corner = &places[*index];
            let at = Pt {
                x: corner.x,
                y: corner.y,
            };
            holes.push(Hole {
                kind: HoleKind::Tooling,
                at,
                diameter: settings.tooling_hole_diameter_nm,
            });
            circles.push(Circle {
                at,
                radius: (settings.tooling_hole_diameter_nm + 1) / 2,
            });
        }
        for index in corner_indices(settings.fiducial_count) {
            let corner = &places[*index];
            let step = settings.fiducial_step_nm;
            let (dx, dy) = if top_bottom {
                (if corner.right { -step } else { step }, 0)
            } else {
                (0, if corner.top { -step } else { step })
            };
            let at = Pt {
                x: corner.x + dx,
                y: corner.y + dy,
            };
            fiducials.push(Mark {
                at,
                copper_diameter: settings.fiducial_diameter_nm,
                mask_diameter: settings.fiducial_mask_diameter_nm,
            });
            circles.push(Circle {
                at,
                radius: (settings.fiducial_mask_diameter_nm + 1) / 2,
            });
        }
        for circle in &circles {
            if !inside_rail(circle, width, height, rail_x, rail_y) {
                return Err(PanelError::new(
                    MARK_DOESNT_FIT,
                    format!(
                        "a mark at ({}, {}) with a radius of {} nm is not inside a rail of {} nm",
                        circle.at.x, circle.at.y, circle.radius, settings.frame_width_nm
                    ),
                ));
            }
        }
        for (i, a) in circles.iter().enumerate() {
            for b in circles.iter().skip(i + 1) {
                let dx = i128::from(a.at.x - b.at.x);
                let dy = i128::from(a.at.y - b.at.y);
                let reach = i128::from(a.radius) + i128::from(b.radius);
                if dx * dx + dy * dy < reach * reach {
                    return Err(PanelError::new(
                        MARKS_OVERLAP,
                        format!(
                            "marks at ({}, {}) and ({}, {})",
                            a.at.x, a.at.y, b.at.x, b.at.y
                        ),
                    ));
                }
            }
        }
    }

    Ok(PanelPlan {
        width,
        height,
        columns: cols,
        rows: rws,
        origins,
        profile,
        score,
        tabs,
        holes,
        fiducials,
        notes,
    })
}

impl PanelPlan {
    /// The number of holes of a kind.
    pub fn holes_of(&self, kind: HoleKind) -> usize {
        self.holes.iter().filter(|h| h.kind == kind).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    fn shape() -> BoardShape {
        BoardShape {
            origin: Pt {
                x: 10 * MM,
                y: 5 * MM,
            },
            width: 30 * MM,
            height: 20 * MM,
            holes: vec![
                (
                    true,
                    Pt {
                        x: 12 * MM,
                        y: 7 * MM,
                    },
                    800_000,
                ),
                (
                    false,
                    Pt {
                        x: 38 * MM,
                        y: 23 * MM,
                    },
                    3 * MM,
                ),
            ],
        }
    }

    fn settings(text: &str) -> PanelSettings {
        PanelSettings::parse(text).unwrap()
    }

    #[test]
    fn the_matrix_has_the_size_of_its_boards_gaps_and_frame() {
        let plan = plan(&shape(), &PanelSettings::default()).unwrap();
        // 2 columns: frame 8 + gap 2 on each side, 2 boards of 30 and a gap of 2
        assert_eq!(plan.width, 2 * (8 + 2) * MM + 60 * MM + 2 * MM);
        assert_eq!(plan.height, 2 * (8 + 2) * MM + 40 * MM + 2 * MM);
        assert_eq!(plan.origins.len(), 4);
        assert_eq!(
            plan.origins[0],
            Pt {
                x: 10 * MM,
                y: 10 * MM
            }
        );
        assert_eq!(
            plan.origins[1],
            Pt {
                x: 42 * MM,
                y: 10 * MM
            }
        );
        assert_eq!(
            plan.origins[2],
            Pt {
                x: 10 * MM,
                y: 32 * MM
            }
        );
        assert!(plan.score.is_empty());
    }

    #[test]
    fn the_holes_of_the_board_are_copied_to_every_board() {
        let plan = plan(&shape(), &PanelSettings::default()).unwrap();
        assert_eq!(plan.holes_of(HoleKind::Plated), 4);
        assert_eq!(plan.holes_of(HoleKind::NonPlated), 4);
        let first = plan
            .holes
            .iter()
            .find(|h| h.kind == HoleKind::Plated)
            .unwrap();
        // (12, 7) on the board is 2 mm and 2 mm from its corner (10, 5)
        assert_eq!(
            first.at,
            Pt {
                x: 12 * MM,
                y: 12 * MM
            }
        );
        let plated: Vec<&Hole> = plan
            .holes
            .iter()
            .filter(|h| h.kind == HoleKind::Plated)
            .collect();
        assert_eq!(
            plated[3].at,
            Pt {
                x: 44 * MM,
                y: 34 * MM
            }
        );
    }

    #[test]
    fn tabs_sit_in_the_middle_of_each_part_of_an_edge_with_a_row_of_bites() {
        let cfg = settings(
            "columns = 2\nrows = 1\nframe = none\ntabs_per_edge = 2\nmouse_bite_holes = 3\n",
        );
        let plan = plan(&shape(), &cfg).unwrap();
        // one gap between the two boards, one row: two tabs along the 20 mm edge
        assert_eq!(plan.tabs.len(), 2);
        assert_eq!(
            plan.tabs[0],
            Pt {
                x: 31 * MM,
                y: 5 * MM
            }
        );
        assert_eq!(
            plan.tabs[1],
            Pt {
                x: 31 * MM,
                y: 15 * MM
            }
        );
        assert_eq!(plan.holes_of(HoleKind::MouseBite), 6);
        let bites: Vec<Pt> = plan
            .holes
            .iter()
            .filter(|h| h.kind == HoleKind::MouseBite)
            .map(|h| h.at)
            .take(3)
            .collect();
        assert_eq!(
            bites,
            vec![
                Pt {
                    x: 31 * MM,
                    y: 5 * MM - 800_000
                },
                Pt {
                    x: 31 * MM,
                    y: 5 * MM
                },
                Pt {
                    x: 31 * MM,
                    y: 5 * MM + 800_000
                },
            ]
        );
        // no frame: the marks are left out with a note
        assert_eq!(plan.notes.len(), 1);
        assert_eq!(plan.notes[0].code, MARKS_SKIPPED);
    }

    #[test]
    fn the_profile_is_the_panel_edge_and_the_board_edges_broken_at_the_tabs() {
        let cfg = settings("columns = 2\nrows = 1\nframe = none\ntabs_per_edge = 1\n");
        let plan = plan(&shape(), &cfg).unwrap();
        // 4 lines of the panel, then for each board one inner vertical edge cut in two pieces
        assert_eq!(plan.profile.len(), 4 + 2 * 2);
        let inner: Vec<&Line> = plan.profile.iter().skip(4).collect();
        // the first board's right edge is at x = 30 mm, broken around y = 10 mm by a tab of 3 mm
        assert_eq!(inner[0].from, Pt { x: 30 * MM, y: 0 });
        assert_eq!(
            inner[0].to,
            Pt {
                x: 30 * MM,
                y: 10 * MM - 1_500_000
            }
        );
        assert_eq!(
            inner[1].from,
            Pt {
                x: 30 * MM,
                y: 10 * MM + 1_500_000
            }
        );
        assert_eq!(
            inner[1].to,
            Pt {
                x: 30 * MM,
                y: 20 * MM
            }
        );
    }

    #[test]
    fn v_score_boards_touch_and_the_lines_cross_the_whole_panel() {
        let cfg =
            settings("columns = 3\nrows = 2\nframe = all\njoint = v_score\ngap_nm = 2000000\n");
        let plan = plan(&shape(), &cfg).unwrap();
        assert_eq!(plan.width, 2 * 8 * MM + 90 * MM);
        assert_eq!(plan.height, 2 * 8 * MM + 40 * MM);
        // with a frame all the lines between boards and rails are scored: 4 + 3
        assert_eq!(plan.score.len(), 4 + 3);
        assert_eq!(plan.score[0].from, Pt { x: 8 * MM, y: 0 });
        assert_eq!(
            plan.score[0].to,
            Pt {
                x: 8 * MM,
                y: plan.height
            }
        );
        assert_eq!(plan.profile.len(), 4);
        assert!(plan.tabs.is_empty());
        assert_eq!(plan.holes_of(HoleKind::MouseBite), 0);
        assert!(plan.notes.iter().any(|n| n.code == GAP_IGNORED));
        // without a frame only the lines between boards
        let bare = settings("columns = 3\nrows = 2\nframe = none\njoint = v_score\n");
        let plan = super::plan(&shape(), &bare).unwrap();
        assert_eq!(plan.score.len(), 2 + 1);
    }

    #[test]
    fn the_marks_are_in_the_rails_and_apart() {
        let plan = plan(&shape(), &PanelSettings::default()).unwrap();
        assert_eq!(plan.holes_of(HoleKind::Tooling), 4);
        assert_eq!(plan.fiducials.len(), 3);
        let tooling: Vec<Pt> = plan
            .holes
            .iter()
            .filter(|h| h.kind == HoleKind::Tooling)
            .map(|h| h.at)
            .collect();
        assert_eq!(
            tooling[0],
            Pt {
                x: 4 * MM,
                y: 4 * MM
            }
        );
        assert_eq!(
            tooling[2],
            Pt {
                x: plan.width - 4 * MM,
                y: plan.height - 4 * MM
            }
        );
        // fiducials are 12 mm along the rail from the tooling hole of their corner
        assert_eq!(
            plan.fiducials[0].at,
            Pt {
                x: 16 * MM,
                y: 4 * MM
            }
        );
        assert_eq!(
            plan.fiducials[1].at,
            Pt {
                x: plan.width - 16 * MM,
                y: 4 * MM
            }
        );
        // a rail on the sides only: the marks go along y
        let cfg = settings("frame = left_right\n");
        let plan = super::plan(&shape(), &cfg).unwrap();
        assert_eq!(
            plan.fiducials[0].at,
            Pt {
                x: 4 * MM,
                y: 16 * MM
            }
        );
        let none = settings("tooling_hole_count = 0\nfiducial_count = 0\n");
        let plan = super::plan(&shape(), &none).unwrap();
        assert!(plan.holes.iter().all(|h| h.kind != HoleKind::Tooling));
        assert!(plan.fiducials.is_empty());
    }

    #[test]
    fn a_layout_that_cannot_be_made_has_a_code() {
        let base = PanelSettings::default();
        let too_many = PanelSettings {
            max_boards: 3,
            ..base.clone()
        };
        assert_eq!(plan(&shape(), &too_many).unwrap_err().code, PANEL_TOO_LARGE);
        let too_big = PanelSettings {
            max_panel_nm: 50 * MM,
            ..base.clone()
        };
        assert_eq!(plan(&shape(), &too_big).unwrap_err().code, PANEL_TOO_LARGE);
        let wide_tabs = PanelSettings {
            tab_width_nm: 11 * MM,
            ..base.clone()
        };
        assert_eq!(plan(&shape(), &wide_tabs).unwrap_err().code, TABS_DONT_FIT);
        let long_row = PanelSettings {
            mouse_bite_holes: 10,
            ..base.clone()
        };
        assert_eq!(plan(&shape(), &long_row).unwrap_err().code, BITES_DONT_FIT);
        let thin_rail = PanelSettings {
            frame_width_nm: 4 * MM,
            ..base.clone()
        };
        assert_eq!(
            plan(&shape(), &thin_rail).unwrap_err().code,
            MARK_DOESNT_FIT
        );
        let crowded = PanelSettings {
            fiducial_step_nm: MM,
            ..base.clone()
        };
        assert_eq!(plan(&shape(), &crowded).unwrap_err().code, MARKS_OVERLAP);
        let invalid = PanelSettings { columns: 0, ..base };
        assert_eq!(
            plan(&shape(), &invalid).unwrap_err().code,
            crate::codes::SETTINGS_INVALID
        );
    }

    #[test]
    fn edge_pieces_and_centres() {
        assert_eq!(tab_centres(20, 2, 4).unwrap(), vec![5, 15]);
        assert!(tab_centres(20, 6, 4).is_err());
        assert_eq!(
            edge_pieces(20, &[5, 15], 4),
            vec![(0, 3), (7, 13), (17, 20)]
        );
        assert_eq!(edge_pieces(20, &[], 4), vec![(0, 20)]);
        assert_eq!(bite_offsets(3, 800), vec![-800, 0, 800]);
        assert_eq!(bite_offsets(2, 800), vec![-400, 400]);
        assert_eq!(bite_offsets(1, 800), vec![0]);
        assert_eq!(corner_indices(2), &[0, 2]);
        assert!(corner_indices(0).is_empty());
    }

    #[test]
    fn the_shape_of_a_board_is_its_box_and_a_note_when_it_is_not_a_rectangle() {
        use eh3d_phoenix_model::Point;
        let mut board = Board::default();
        let pt = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
        board.outline = vec![
            pt(1_000_000, 2_000_000),
            pt(31_000_000, 2_000_000),
            pt(31_000_000, 22_000_000),
            pt(1_000_000, 22_000_000),
        ];
        let (read, notes) = BoardShape::from_board(&board).unwrap();
        assert_eq!(
            read.origin,
            Pt {
                x: 1_000_000,
                y: 2_000_000
            }
        );
        assert_eq!((read.width, read.height), (30_000_000, 20_000_000));
        assert!(notes.is_empty());
        // closing point repeated
        board.outline.push(pt(1_000_000, 2_000_000));
        assert!(BoardShape::from_board(&board).unwrap().1.is_empty());
        // a corner cut off
        board.outline = vec![
            pt(0, 0),
            pt(30_000_000, 0),
            pt(30_000_000, 10_000_000),
            pt(20_000_000, 20_000_000),
            pt(0, 20_000_000),
        ];
        let (read, notes) = BoardShape::from_board(&board).unwrap();
        assert_eq!(read.width, 30_000_000);
        assert_eq!(notes[0].code, OUTLINE_NOT_RECTANGLE);
        board.outline = vec![pt(0, 0), pt(1, 1)];
        assert_eq!(
            BoardShape::from_board(&board).unwrap_err().code,
            OUTLINE_UNUSABLE
        );
        board.outline = vec![pt(0, 0), pt(5, 0), pt(10, 0)];
        assert_eq!(
            BoardShape::from_board(&board).unwrap_err().code,
            OUTLINE_UNUSABLE
        );
    }
}
