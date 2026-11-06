// SPDX-License-Identifier: GPL-3.0-or-later
//! A panel of different boards: a list of boards, each with its place (or packed by rows with the
//! gap between them) and its turn and mirror, inside the same frame, with the same tabs and mouse
//! bites or V-score lines, fiducials and tooling holes as a matrix of one board. All arithmetic is
//! in integer nanometres.

use crate::codes::{
    Note, PanelError, BITES_DONT_FIT, BOARDS_INVALID, BOARDS_OVERLAP, BOARD_NOT_JOINED,
    GAP_IGNORED, VSCORE_CANNOT_SEPARATE,
};
use crate::layout::{
    bite_offsets, frame_marks, local_marks, tab_centres, to_i64, too_large, BoardShape, Hole,
    HoleKind, Line, PanelCopy, PanelPlan, Pt,
};
use crate::settings::{Joint, PanelSettings};

/// One board of a list: which one, where (or `None` to pack it) and how it is turned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    /// The number of the board in the list of shapes, from 0.
    pub board: usize,
    /// The lower left corner of its box inside the frame, or `None` to pack it by rows.
    pub at: Option<Pt>,
    /// The turn in degrees, counter-clockwise: 0, 90, 180 or 270.
    pub rotation_deg: u32,
    /// Mirror the board about its vertical axis before the turn.
    pub mirror: bool,
}

/// The items of a list of `boards` boards: the `place` lines of the settings when there are any,
/// else every board once, in order, packed by rows and turned as the settings say.
pub fn items_from_settings(boards: usize, settings: &PanelSettings) -> Vec<Item> {
    if settings.placements.is_empty() {
        (0..boards)
            .map(|board| Item {
                board,
                at: None,
                rotation_deg: settings.copy_rotation_deg,
                mirror: settings.copy_mirror,
            })
            .collect()
    } else {
        settings
            .placements
            .iter()
            .map(|p| Item {
                board: p.board,
                at: Some(Pt {
                    x: p.x_nm,
                    y: p.y_nm,
                }),
                rotation_deg: p.rotation_deg,
                mirror: p.mirror,
            })
            .collect()
    }
}

/// A box in the panel: the lower left corner, the width and the height.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

impl Rect {
    fn right(&self) -> i64 {
        self.x + self.w
    }

    fn top(&self) -> i64 {
        self.y + self.h
    }
}

/// The pieces cut at the sides of every board: the stretches of each side that a tab covers.
type Cuts = [Vec<(i64, i64)>; 4];

const LEFT: usize = 0;
const RIGHT: usize = 1;
const BOTTOM: usize = 2;
const TOP: usize = 3;

/// The tabs of one stretch of an edge, with the stretches they cover and their mouse bite holes.
struct TabRun {
    intervals: Vec<(i64, i64)>,
    tabs: Vec<Pt>,
    holes: Vec<Hole>,
}

/// The tabs along a stretch `lo..hi` of an edge. `vertical` says the stretch runs along y at the
/// x of `fixed`; otherwise it runs along x at the y of `fixed`.
fn tab_run(
    settings: &PanelSettings,
    offsets: &[i64],
    span: (i64, i64),
    fixed: i64,
    vertical: bool,
) -> Result<TabRun, PanelError> {
    let (lo, hi) = span;
    let tw = settings.tab_width_nm;
    let centres = tab_centres(hi - lo, settings.tabs_per_edge, tw)?;
    let mut run = TabRun {
        intervals: Vec::new(),
        tabs: Vec::new(),
        holes: Vec::new(),
    };
    let point = |along: i64| {
        if vertical {
            Pt { x: fixed, y: along }
        } else {
            Pt { x: along, y: fixed }
        }
    };
    for centre in centres {
        let along = lo + centre;
        run.tabs.push(point(along));
        run.intervals.push((along - tw / 2, along - tw / 2 + tw));
        for offset in offsets {
            run.holes.push(Hole {
                kind: HoleKind::MouseBite,
                at: point(along + offset),
                diameter: settings.mouse_bite_drill_nm,
            });
        }
    }
    Ok(run)
}

/// The stretches of `lo..hi` that the sorted `intervals` leave free.
fn cut_pieces(lo: i64, hi: i64, intervals: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    let mut position = lo;
    for (start, end) in intervals {
        let start = (*start).max(lo);
        let end = (*end).min(hi);
        if start > position {
            out.push((position, start));
        }
        if end > position {
            position = end;
        }
    }
    if hi > position {
        out.push((position, hi));
    }
    out
}

/// True when the stretches `a0..a1` and `b0..b1` share a length.
fn overlap(a0: i64, a1: i64, b0: i64, b1: i64) -> bool {
    a0.max(b0) < a1.min(b1)
}

/// What the rails of the frame and the gap make of the area around the boards.
struct Frame {
    rail_x: i64,
    rail_y: i64,
    gap: i64,
    width: i64,
    height: i64,
    left_right: bool,
    top_bottom: bool,
}

/// The V-score lines: the lines at the edges of the boards that cross no board and separate two
/// boards that touch (or a board and a rail), from edge to edge of the panel.
fn v_score_lines(rects: &[Rect], f: &Frame) -> Result<Vec<Line>, PanelError> {
    let mut xs: Vec<i64> = rects.iter().flat_map(|r| [r.x, r.right()]).collect();
    let mut ys: Vec<i64> = rects.iter().flat_map(|r| [r.y, r.top()]).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let vertical: Vec<i64> = xs
        .into_iter()
        .filter(|c| *c > 0 && *c < f.width)
        .filter(|c| !rects.iter().any(|r| r.x < *c && *c < r.right()))
        .filter(|c| {
            let rail = f.left_right && (*c == f.rail_x || *c == f.width - f.rail_x);
            let touches = rects.iter().any(|r| r.x == *c || r.right() == *c);
            let meets = rects.iter().any(|a| {
                a.right() == *c
                    && rects
                        .iter()
                        .any(|b| b.x == *c && overlap(a.y, a.top(), b.y, b.top()))
            });
            meets || (rail && touches)
        })
        .collect();
    let horizontal: Vec<i64> = ys
        .into_iter()
        .filter(|c| *c > 0 && *c < f.height)
        .filter(|c| !rects.iter().any(|r| r.y < *c && *c < r.top()))
        .filter(|c| {
            let rail = f.top_bottom && (*c == f.rail_y || *c == f.height - f.rail_y);
            let touches = rects.iter().any(|r| r.y == *c || r.top() == *c);
            let meets = rects.iter().any(|a| {
                a.top() == *c
                    && rects
                        .iter()
                        .any(|b| b.y == *c && overlap(a.x, a.right(), b.x, b.right()))
            });
            meets || (rail && touches)
        })
        .collect();
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate().skip(i + 1) {
            let by_x = vertical
                .iter()
                .any(|c| (a.right()..=b.x).contains(c) || (b.right()..=a.x).contains(c));
            let by_y = horizontal
                .iter()
                .any(|c| (a.top()..=b.y).contains(c) || (b.top()..=a.y).contains(c));
            if !(by_x || by_y) {
                return Err(PanelError::new(
                    VSCORE_CANNOT_SEPARATE,
                    format!("no line runs between boards {} and {}", i + 1, j + 1),
                ));
            }
        }
    }
    let mut lines = Vec::new();
    for x in vertical {
        lines.push(Line {
            from: Pt { x, y: 0 },
            to: Pt { x, y: f.height },
        });
    }
    for y in horizontal {
        lines.push(Line {
            from: Pt { x: 0, y },
            to: Pt { x: f.width, y },
        });
    }
    Ok(lines)
}

/// The tabs between the boards and between the boards and the rails, with their holes, and the
/// stretches of each side that they cover.
struct Joints {
    tabs: Vec<Pt>,
    holes: Vec<Hole>,
    cuts: Vec<Cuts>,
    joined: Vec<bool>,
}

fn mouse_bite_joints(
    rects: &[Rect],
    f: &Frame,
    settings: &PanelSettings,
) -> Result<Joints, PanelError> {
    let span = i128::from(settings.mouse_bite_holes - 1) * i128::from(settings.mouse_bite_pitch_nm)
        + i128::from(settings.mouse_bite_drill_nm);
    if span > i128::from(settings.tab_width_nm) {
        return Err(PanelError::new(
            BITES_DONT_FIT,
            format!(
                "{} holes take {span} nm and the tab is {} nm wide",
                settings.mouse_bite_holes, settings.tab_width_nm
            ),
        ));
    }
    let offsets = bite_offsets(settings.mouse_bite_holes, settings.mouse_bite_pitch_nm);
    let mut joints = Joints {
        tabs: Vec::new(),
        holes: Vec::new(),
        cuts: vec![Cuts::default(); rects.len()],
        joined: vec![false; rects.len()],
    };
    let half = f.gap / 2;
    for (ai, a) in rects.iter().enumerate() {
        // to the right: a board at the gap, or the rail
        let mut runs: Vec<(usize, Option<usize>, TabRun)> = Vec::new();
        for (bi, b) in rects.iter().enumerate() {
            if bi != ai && b.x == a.right() + f.gap && overlap(a.y, a.top(), b.y, b.top()) {
                let span = (a.y.max(b.y), a.top().min(b.top()));
                let run = tab_run(settings, &offsets, span, a.right() + half, true)?;
                runs.push((RIGHT, Some(bi), run));
            }
            if bi != ai && b.y == a.top() + f.gap && overlap(a.x, a.right(), b.x, b.right()) {
                let span = (a.x.max(b.x), a.right().min(b.right()));
                let run = tab_run(settings, &offsets, span, a.top() + half, false)?;
                runs.push((TOP, Some(bi), run));
            }
        }
        if f.left_right && a.right() + f.gap == f.width - f.rail_x {
            let run = tab_run(settings, &offsets, (a.y, a.top()), a.right() + half, true)?;
            runs.push((RIGHT, None, run));
        }
        if f.left_right && a.x - f.gap == f.rail_x {
            let run = tab_run(settings, &offsets, (a.y, a.top()), a.x - f.gap + half, true)?;
            runs.push((LEFT, None, run));
        }
        if f.top_bottom && a.top() + f.gap == f.height - f.rail_y {
            let run = tab_run(settings, &offsets, (a.x, a.right()), a.top() + half, false)?;
            runs.push((TOP, None, run));
        }
        if f.top_bottom && a.y - f.gap == f.rail_y {
            let run = tab_run(
                settings,
                &offsets,
                (a.x, a.right()),
                a.y - f.gap + half,
                false,
            )?;
            runs.push((BOTTOM, None, run));
        }
        for (side, other, run) in runs {
            joints.tabs.extend(run.tabs);
            joints.holes.extend(run.holes);
            joints.joined[ai] = true;
            joints.cuts[ai][side].extend(run.intervals.iter().copied());
            if let Some(bi) = other {
                // the board on the other side of the gap is cut at the same stretches
                let opposite = if side == RIGHT { LEFT } else { BOTTOM };
                joints.cuts[bi][opposite].extend(run.intervals.iter().copied());
                joints.joined[bi] = true;
            }
        }
    }
    Ok(joints)
}

/// Lays a list of boards out. `shapes` are the boards, `items` say which board goes where and how
/// it is turned.
pub fn plan_items(
    shapes: &[BoardShape],
    items: &[Item],
    settings: &PanelSettings,
) -> Result<PanelPlan, PanelError> {
    settings.validate()?;
    let invalid = |what: String| PanelError::new(BOARDS_INVALID, what);
    if items.is_empty() {
        return Err(invalid("the list of boards is empty".to_owned()));
    }
    if items.len() > settings.max_boards as usize {
        return Err(too_large(&format!(
            "{} boards, the limit is {}",
            items.len(),
            settings.max_boards
        )));
    }
    let placed = items.iter().filter(|i| i.at.is_some()).count();
    if placed != 0 && placed != items.len() {
        return Err(invalid(
            "some boards have a place and some do not".to_owned(),
        ));
    }
    let mut oriented: Vec<BoardShape> = Vec::new();
    for item in items {
        let shape = shapes.get(item.board).ok_or_else(|| {
            invalid(format!(
                "board {} is not in the list of {} boards",
                item.board,
                shapes.len()
            ))
        })?;
        if !matches!(item.rotation_deg, 0 | 90 | 180 | 270) {
            return Err(invalid(format!(
                "a turn of {} degrees is not 0, 90, 180 or 270",
                item.rotation_deg
            )));
        }
        oriented.push(shape.oriented(item.rotation_deg, item.mirror));
    }
    let mut notes: Vec<Note> = Vec::new();
    let v_score = settings.joint == Joint::VScore;
    let gap = if v_score { 0 } else { settings.gap_nm };
    if v_score && settings.gap_nm != 0 {
        notes.push(Note {
            code: GAP_IGNORED,
            detail: format!("gap_nm = {} is not used", settings.gap_nm),
        });
    }
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

    // the places inside the frame: given, or packed by rows
    let mut inner: Vec<(i64, i64)> = Vec::new();
    if placed == 0 {
        let (mut x, mut y, mut row) = (0i64, 0i64, 0i64);
        for o in &oriented {
            if x > 0 && x + o.width > settings.pack_row_width_nm {
                y += row + gap;
                x = 0;
                row = 0;
            }
            inner.push((x, y));
            x += o.width + gap;
            row = row.max(o.height);
        }
    } else {
        for item in items {
            let at = item.at.unwrap_or(Pt { x: 0, y: 0 });
            inner.push((at.x, at.y));
        }
    }
    let rects: Vec<Rect> = inner
        .iter()
        .zip(&oriented)
        .map(|((x, y), o)| Rect {
            x: start_x + x,
            y: start_y + y,
            w: o.width,
            h: o.height,
        })
        .collect();
    for (i, a) in rects.iter().enumerate() {
        for (j, b) in rects.iter().enumerate().skip(i + 1) {
            let apart_x = a.right() + gap <= b.x || b.right() + gap <= a.x;
            let apart_y = a.top() + gap <= b.y || b.top() + gap <= a.y;
            if !(apart_x || apart_y) {
                return Err(PanelError::new(
                    BOARDS_OVERLAP,
                    format!("boards {} and {}", i + 1, j + 1),
                ));
            }
        }
    }
    let content_w = rects.iter().map(Rect::right).max().unwrap_or(0) - start_x;
    let content_h = rects.iter().map(Rect::top).max().unwrap_or(0) - start_y;
    let width = 2 * i128::from(start_x) + i128::from(content_w);
    let height = 2 * i128::from(start_y) + i128::from(content_h);
    let limit = i128::from(settings.max_panel_nm);
    if width > limit || height > limit {
        return Err(too_large(&format!(
            "{width} nm by {height} nm, the limit is {limit} nm"
        )));
    }
    let (width, height) = (to_i64(width)?, to_i64(height)?);
    let frame = Frame {
        rail_x,
        rail_y,
        gap,
        width,
        height,
        left_right,
        top_bottom,
    };

    let mut holes: Vec<Hole> = Vec::new();
    for (rect, o) in rects.iter().zip(&oriented) {
        for (plated, at, diameter) in &o.holes {
            holes.push(Hole {
                kind: if *plated {
                    HoleKind::Plated
                } else {
                    HoleKind::NonPlated
                },
                at: Pt {
                    x: rect.x + at.x,
                    y: rect.y + at.y,
                },
                diameter: *diameter,
            });
        }
    }
    let corners = [
        Pt { x: 0, y: 0 },
        Pt { x: width, y: 0 },
        Pt {
            x: width,
            y: height,
        },
        Pt { x: 0, y: height },
    ];
    let mut profile: Vec<Line> = Vec::new();
    for (k, from) in corners.iter().enumerate() {
        profile.push(Line {
            from: *from,
            to: corners[(k + 1) % 4],
        });
    }
    let mut score: Vec<Line> = Vec::new();
    let mut tabs: Vec<Pt> = Vec::new();
    if v_score {
        score = v_score_lines(&rects, &frame)?;
    } else {
        let joints = mouse_bite_joints(&rects, &frame, settings)?;
        for (number, joined) in joints.joined.iter().enumerate() {
            if !joined {
                notes.push(Note {
                    code: BOARD_NOT_JOINED,
                    detail: format!("board {}", number + 1),
                });
            }
        }
        tabs = joints.tabs;
        holes.extend(joints.holes);
        for (a, cuts) in rects.iter().zip(&joints.cuts) {
            let sides = [
                (a.x != 0, true, a.x, a.y, a.top()),
                (a.right() != width, true, a.right(), a.y, a.top()),
                (a.y != 0, false, a.y, a.x, a.right()),
                (a.top() != height, false, a.top(), a.x, a.right()),
            ];
            for (side, (routed, vertical, fixed, lo, hi)) in sides.iter().enumerate() {
                if !routed {
                    continue;
                }
                let mut intervals = cuts[side].clone();
                intervals.sort_unstable();
                for (from, to) in cut_pieces(*lo, *hi, &intervals) {
                    profile.push(if *vertical {
                        Line {
                            from: Pt { x: *fixed, y: from },
                            to: Pt { x: *fixed, y: to },
                        }
                    } else {
                        Line {
                            from: Pt { x: from, y: *fixed },
                            to: Pt { x: to, y: *fixed },
                        }
                    });
                }
            }
        }
    }

    let copies: Vec<PanelCopy> = items
        .iter()
        .zip(&rects)
        .map(|(item, rect)| PanelCopy {
            board: item.board,
            origin: Pt {
                x: rect.x,
                y: rect.y,
            },
            width: rect.w,
            height: rect.h,
            rotation_deg: item.rotation_deg,
            mirror: item.mirror,
        })
        .collect();
    let fiducials = frame_marks(
        settings,
        width,
        height,
        (rail_x, rail_y),
        &mut holes,
        &mut notes,
    )?;
    let local_fiducials = local_marks(&copies, settings)?;
    Ok(PanelPlan {
        width,
        height,
        columns: u32::try_from(items.len()).unwrap_or(u32::MAX),
        rows: 1,
        origins: copies.iter().map(|c| c.origin).collect(),
        profile,
        score,
        tabs,
        holes,
        fiducials,
        local_fiducials,
        mixed: true,
        copies,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Placement;

    const MM: i64 = 1_000_000;

    fn board(w: i64, h: i64) -> BoardShape {
        BoardShape {
            origin: Pt { x: 0, y: 0 },
            width: w * MM,
            height: h * MM,
            holes: vec![(
                true,
                Pt {
                    x: 2 * MM,
                    y: 3 * MM,
                },
                800_000,
            )],
        }
    }

    fn settings(text: &str) -> PanelSettings {
        PanelSettings::parse(text).unwrap()
    }

    #[test]
    fn boards_are_packed_by_rows_with_the_gap_between_them() {
        let s = settings("frame = none\ngap_nm = 2000000\npack_row_width_nm = 70000000\n");
        let shapes = [board(30, 20), board(20, 10), board(30, 20)];
        let items = items_from_settings(3, &s);
        let plan = plan_items(&shapes, &items, &s).unwrap();
        // the first two share a row (30 + 2 + 20), the third does not fit in 70 mm and starts a row
        assert_eq!(plan.copies[0].origin, Pt { x: 0, y: 0 });
        assert_eq!(plan.copies[1].origin, Pt { x: 32 * MM, y: 0 });
        assert_eq!(plan.copies[2].origin, Pt { x: 0, y: 22 * MM });
        assert_eq!((plan.width, plan.height), (52 * MM, 42 * MM));
        assert_eq!(plan.holes_of(HoleKind::Plated), 3);
        // 600 + 200 + 600 square millimetres in 52 x 42
        assert_eq!(plan.utilisation_permille(), 1_400_000 * 1000 / 2_184_000);
    }

    #[test]
    fn a_list_with_places_keeps_them_and_the_turns_swap_the_box() {
        let mut s = settings("frame = none\n");
        s.placements = vec![
            Placement {
                board: 0,
                x_nm: 0,
                y_nm: 0,
                rotation_deg: 90,
                mirror: false,
            },
            Placement {
                board: 1,
                x_nm: 40 * MM,
                y_nm: 0,
                rotation_deg: 0,
                mirror: true,
            },
        ];
        let shapes = [board(30, 20), board(20, 10)];
        let items = items_from_settings(2, &s);
        let plan = plan_items(&shapes, &items, &s).unwrap();
        assert_eq!(
            (plan.copies[0].width, plan.copies[0].height),
            (20 * MM, 30 * MM)
        );
        assert_eq!(plan.copies[1].origin, Pt { x: 40 * MM, y: 0 });
        assert!(plan.copies[1].mirror);
        // the hole at (2, 3) of the first board turned 90 degrees: (h - y, x) = (17, 2)
        assert!(plan.holes.iter().any(|h| h.at
            == Pt {
                x: 17 * MM,
                y: 2 * MM
            }));
        // mirrored in a box 20 wide: (18, 3), moved 40 to the right
        assert!(plan.holes.iter().any(|h| h.at
            == Pt {
                x: 58 * MM,
                y: 3 * MM
            }));
    }

    #[test]
    fn boards_that_overlap_or_are_too_close_are_refused() {
        let mut s = settings("frame = none\n");
        s.placements = vec![
            Placement {
                board: 0,
                x_nm: 0,
                y_nm: 0,
                rotation_deg: 0,
                mirror: false,
            },
            Placement {
                board: 0,
                x_nm: 31 * MM,
                y_nm: 0,
                rotation_deg: 0,
                mirror: false,
            },
        ];
        let shapes = [board(30, 20)];
        let items = items_from_settings(1, &s);
        let e = plan_items(&shapes, &items, &s).unwrap_err();
        assert_eq!(e.code, BOARDS_OVERLAP);
        s.placements[1].x_nm = 32 * MM;
        assert!(plan_items(&shapes, &items_from_settings(1, &s), &s).is_ok());
    }

    #[test]
    fn a_bad_list_is_refused() {
        let s = settings("frame = none\n");
        let shapes = [board(30, 20)];
        assert_eq!(
            plan_items(&shapes, &[], &s).unwrap_err().code,
            BOARDS_INVALID
        );
        let item = |board, rotation_deg| Item {
            board,
            at: None,
            rotation_deg,
            mirror: false,
        };
        assert_eq!(
            plan_items(&shapes, &[item(3, 0)], &s).unwrap_err().code,
            BOARDS_INVALID
        );
        assert_eq!(
            plan_items(&shapes, &[item(0, 45)], &s).unwrap_err().code,
            BOARDS_INVALID
        );
        let mixed = [
            Item {
                at: Some(Pt { x: 0, y: 0 }),
                ..item(0, 0)
            },
            item(0, 0),
        ];
        assert_eq!(
            plan_items(&shapes, &mixed, &s).unwrap_err().code,
            BOARDS_INVALID
        );
    }

    #[test]
    fn mouse_bites_join_boards_of_different_size_and_the_frame() {
        let s = settings("gap_nm = 2000000\ntabs_per_edge = 1\n");
        let shapes = [board(30, 20), board(20, 20)];
        let items = items_from_settings(2, &s);
        let plan = plan_items(&shapes, &items, &s).unwrap();
        // one tab between the boards, one to the left rail, one to the right rail, and one to each
        // of the rails above and below for the two boards
        assert_eq!(plan.tabs.len(), 1 + 1 + 1 + 2 + 2);
        assert_eq!(plan.holes_of(HoleKind::MouseBite), 7 * 3);
        assert_eq!(plan.holes_of(HoleKind::Tooling), 4);
        assert!(plan.notes.iter().all(|n| n.code != BOARD_NOT_JOINED));
        // the tab between the boards is in the middle of the gap, in the middle of the shared edge
        assert!(plan.tabs.contains(&Pt {
            x: (8 + 2 + 30 + 1) * MM,
            y: (8 + 2 + 10) * MM
        }));
    }

    #[test]
    fn v_score_lines_run_between_boards_that_touch_in_a_grid() {
        let s = settings("joint = v_score\ngap_nm = 0\nframe = all\ntabs_per_edge = 1\n");
        let shapes = [board(30, 20)];
        let items = items_from_settings(1, &s);
        let two = [items[0], items[0]];
        let plan = plan_items(&[shapes[0].clone()], &two, &s).unwrap();
        // two boards side by side: lines at the rails (2) and between the boards (1) across, and at
        // the rails above and below (2)
        assert_eq!(plan.score.len(), 5);
        assert_eq!(plan.width, 76 * MM);
        assert_eq!(plan.height, 36 * MM);
    }

    #[test]
    fn v_score_cannot_separate_boards_that_are_not_in_a_grid() {
        let mut s = settings("joint = v_score\ngap_nm = 0\nframe = none\n");
        let place = |board, x, y| Placement {
            board,
            x_nm: x * MM,
            y_nm: y * MM,
            rotation_deg: 0,
            mirror: false,
        };
        // a wide board on the left and two squares one above the other on the right
        s.placements = vec![place(0, 0, 0), place(1, 30, 0), place(1, 30, 20)];
        let items = items_from_settings(2, &s);
        let shapes = [board(30, 20), board(20, 20)];
        // the line at y = 20 mm crosses no board and the line at x = 30 mm crosses none
        let plan = plan_items(&shapes, &items, &s).unwrap();
        assert_eq!(plan.score.len(), 2);
        // a taller board on the left makes the line at y = 20 mm cross it: the two squares on the
        // right can then be separated by no line
        let shapes = [board(30, 40), board(20, 20)];
        let e = plan_items(&shapes, &items, &s).unwrap_err();
        assert_eq!(e.code, VSCORE_CANNOT_SEPARATE);
    }

    #[test]
    fn local_fiducials_go_on_each_board() {
        let s =
            settings("frame = none\nlocal_fiducial_count = 3\nlocal_fiducial_inset_nm = 3000000\n");
        let shapes = [board(30, 20), board(20, 10)];
        let items = items_from_settings(2, &s);
        let plan = plan_items(&shapes, &items, &s).unwrap();
        assert_eq!(plan.local_fiducials.len(), 6);
        assert_eq!(
            plan.local_fiducials[0].at,
            Pt {
                x: 3 * MM,
                y: 3 * MM
            }
        );
        let s =
            settings("frame = none\nlocal_fiducial_count = 3\nlocal_fiducial_inset_nm = 6000000\n");
        let e = plan_items(&shapes, &items_from_settings(2, &s), &s).unwrap_err();
        assert_eq!(e.code, crate::codes::LOCAL_MARK_DOESNT_FIT);
    }

    #[test]
    fn the_stretches_between_tabs_are_cut_in_pieces() {
        assert_eq!(
            cut_pieces(0, 100, &[(10, 20), (40, 50)]),
            [(0, 10), (20, 40), (50, 100)]
        );
        assert!(cut_pieces(0, 100, &[(0, 100)]).is_empty());
        assert_eq!(cut_pieces(0, 100, &[]), [(0, 100)]);
    }
}
