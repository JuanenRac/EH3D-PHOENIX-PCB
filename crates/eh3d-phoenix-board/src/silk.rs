//! Silkscreen: text and strokes printed on the board.
//!
//! A silkscreen item is an addition to the board and has no effect on a board that has none: the
//! map is not written when it is empty and a file made before it existed reads as before. Two kinds
//! of item exist:
//!
//! * a stroke: a connected run of lines and arcs of one width, like a track without a net;
//! * a text: characters drawn by the built-in stroke font below, at any rotation (a multiple of 90
//!   degrees is exact, another angle is rounded once per point), so that
//!   every drawn stroke has integer nanometre end points and the text can be checked and exported
//!   exactly. Lower case letters are drawn as capitals; a character the font does not have is drawn
//!   as an empty box and reported (`PHOENIX-BOARD-016`).
//!
//! The side of a silkscreen layer is the `side` of the layer. All lengths are integer nanometres.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::model::{Board, Id, LayerKind, Segment};
use crate::validate::{BAD_ARC, BAD_LAYER, BAD_SIZE, DANGLING, PATH_NOT_CONNECTED};

/// The rotation of a silkscreen text is not a multiple of 90 degrees.
pub const TEXT_ROTATION: &str = "PHOENIX-BOARD-014";
/// A silkscreen text whose strokes do not fit in the coordinate range.
pub const TEXT_OUT_OF_RANGE: &str = "PHOENIX-BOARD-015";
/// A silkscreen text with characters that the font does not have.
pub const TEXT_NO_GLYPH: &str = "PHOENIX-BOARD-016";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (
        TEXT_ROTATION,
        "Silkscreen text rotation is not a right angle",
    ),
    (
        TEXT_OUT_OF_RANGE,
        "Silkscreen text does not fit the coordinate range",
    ),
    (
        TEXT_NO_GLYPH,
        "Silkscreen text has characters without a glyph",
    ),
];

fn is_false(b: &bool) -> bool {
    !*b
}

/// One item printed on a silkscreen layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SilkItem {
    /// Characters of the built-in stroke font.
    Text {
        /// The silkscreen layer.
        layer: Id,
        /// Where the lower left corner of the first character is (before the rotation).
        at: Point,
        /// The characters.
        text: String,
        /// Height of a capital letter.
        height: Nm,
        /// Width of the printed stroke.
        stroke: Nm,
        /// Rotation about `at`. A multiple of 90 degrees is exact; any other angle is rounded once
        /// to the nearest nanometre for each point, like the rotation of a pad.
        rotation: Udeg,
        /// Written back to front (for the bottom side), mirrored about the vertical axis of `at`.
        #[serde(default, skip_serializing_if = "is_false")]
        mirrored: bool,
    },
    /// A connected run of lines and arcs.
    Stroke {
        /// The silkscreen layer.
        layer: Id,
        /// Width of the printed line.
        width: Nm,
        /// The pieces, each starting where the one before ends.
        path: Vec<Segment>,
    },
}

/// The pieces that are really printed for an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawn {
    /// The width of the printed line.
    pub width: Nm,
    /// The lines and arcs.
    pub pieces: Vec<Segment>,
    /// How many characters had no glyph and were drawn as a box.
    pub missing_glyphs: usize,
}

/// Why the strokes of a text cannot be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawError {
    /// The rotation is not a multiple of 90 degrees.
    Rotation,
    /// A point is outside the coordinate range, or the sizes are not positive.
    Range,
}

/// Width of a character cell and the distance between two characters, in font units.
#[cfg(test)]
const CELL_W: i64 = 4;
const ADVANCE: i64 = 5;
/// Height of a capital letter in font units.
const CELL_H: i64 = 6;

/// The strokes of one glyph: runs of points in a cell 4 units wide and 6 high, origin at the lower
/// left. Only the characters that board silkscreen needs are in the font.
/// The base letter of a letter with a basic accent (so an e with an acute accent is drawn as E).
fn fold(c: char) -> char {
    match c.to_lowercase().next().unwrap_or(c) {
        '\u{e1}' | '\u{e0}' | '\u{e4}' | '\u{e2}' | '\u{e3}' | '\u{e5}' => 'a',
        '\u{e9}' | '\u{e8}' | '\u{eb}' | '\u{ea}' => 'e',
        '\u{ed}' | '\u{ec}' | '\u{ef}' | '\u{ee}' => 'i',
        '\u{f3}' | '\u{f2}' | '\u{f6}' | '\u{f4}' | '\u{f5}' => 'o',
        '\u{fa}' | '\u{f9}' | '\u{fc}' | '\u{fb}' => 'u',
        '\u{f1}' => 'n',
        '\u{e7}' => 'c',
        other => other,
    }
}

fn glyph(c: char) -> Option<&'static str> {
    Some(match fold(c).to_ascii_uppercase() {
        ' ' => "",
        '0' => "0,0 4,0 4,6 0,6 0,0;0,0 4,6",
        '1' => "1,5 2,6 2,0;0,0 4,0",
        '2' => "0,6 4,6 4,3 0,3 0,0 4,0",
        '3' => "0,6 4,6 4,0 0,0;1,3 4,3",
        '4' => "0,6 0,3 4,3;4,6 4,0",
        '5' => "4,6 0,6 0,3 4,3 4,0 0,0",
        '6' => "4,6 0,6 0,0 4,0 4,3 0,3",
        '7' => "0,6 4,6 4,0",
        '8' => "0,0 4,0 4,6 0,6 0,0;0,3 4,3",
        '9' => "4,0 4,6 0,6 0,3 4,3",
        'A' => "0,0 0,4 2,6 4,4 4,0;0,3 4,3",
        'B' => "0,0 0,6 3,6 4,5 4,4 3,3 0,3;3,3 4,2 4,1 3,0 0,0",
        'C' => "4,6 0,6 0,0 4,0",
        'D' => "0,0 0,6 3,6 4,5 4,1 3,0 0,0",
        'E' => "4,6 0,6 0,0 4,0;0,3 3,3",
        'F' => "4,6 0,6 0,0;0,3 3,3",
        'G' => "4,6 0,6 0,0 4,0 4,3 2,3",
        'H' => "0,0 0,6;4,0 4,6;0,3 4,3",
        'I' => "1,6 3,6;2,6 2,0;1,0 3,0",
        'J' => "4,6 4,0 0,0 0,2",
        'K' => "0,0 0,6;4,6 0,3 4,0",
        'L' => "0,6 0,0 4,0",
        'M' => "0,0 0,6 2,3 4,6 4,0",
        'N' => "0,0 0,6 4,0 4,6",
        'O' => "0,0 4,0 4,6 0,6 0,0",
        'P' => "0,0 0,6 4,6 4,3 0,3",
        'Q' => "0,0 4,0 4,6 0,6 0,0;2,2 4,0",
        'R' => "0,0 0,6 4,6 4,3 0,3;2,3 4,0",
        'S' => "4,5 3,6 1,6 0,5 0,4 4,2 4,1 3,0 1,0 0,1",
        'T' => "0,6 4,6;2,6 2,0",
        'U' => "0,6 0,0 4,0 4,6",
        'V' => "0,6 2,0 4,6",
        'W' => "0,6 1,0 2,3 3,0 4,6",
        'X' => "0,0 4,6;0,6 4,0",
        'Y' => "0,6 2,3 4,6;2,3 2,0",
        'Z' => "0,6 4,6 0,0 4,0",
        '-' => "0,3 4,3",
        '+' => "0,3 4,3;2,1 2,5",
        '.' => "2,0 2,1",
        '/' => "0,0 4,6",
        '_' => "0,0 4,0",
        '(' => "3,6 1,4 1,2 3,0",
        ')' => "1,6 3,4 3,2 1,0",
        '=' => "0,2 4,2;0,4 4,4",
        ':' => "2,1 2,2;2,4 2,5",
        '*' => "0,1 4,5;0,5 4,1;2,0 2,6",
        '%' => "0,0 4,6;0,5 1,5;3,1 4,1",
        _ => return None,
    })
}

const BOX: &str = "0,0 4,0 4,6 0,6 0,0";

fn point(x: i128, y: i128) -> Result<Point, DrawError> {
    let x = i64::try_from(x).map_err(|_| DrawError::Range)?;
    let y = i64::try_from(y).map_err(|_| DrawError::Range)?;
    Point::from_nm(x, y).map_err(|_| DrawError::Range)
}

impl SilkItem {
    /// The silkscreen layer of the item.
    pub fn layer(&self) -> Id {
        match self {
            SilkItem::Text { layer, .. } | SilkItem::Stroke { layer, .. } => *layer,
        }
    }

    /// The width of the printed line.
    pub fn line_width(&self) -> Nm {
        match self {
            SilkItem::Text { stroke, .. } => *stroke,
            SilkItem::Stroke { width, .. } => *width,
        }
    }

    /// What is really printed: for a stroke its pieces, for a text the lines of its characters.
    pub fn drawn(&self) -> Result<Drawn, DrawError> {
        match self {
            SilkItem::Stroke { width, path, .. } => Ok(Drawn {
                width: *width,
                pieces: path.clone(),
                missing_glyphs: 0,
            }),
            SilkItem::Text {
                at,
                text,
                height,
                stroke,
                rotation,
                mirrored,
                ..
            } => {
                let turn = rotation.get().rem_euclid(360_000_000);
                let exact = turn % 90_000_000 == 0;
                let quarter = (turn / 90_000_000) as u8;
                let angle = (turn as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
                let (sin, cos) = angle.sin_cos();
                if height.get() <= 0 || stroke.get() <= 0 {
                    return Err(DrawError::Range);
                }
                let h = i128::from(height.get());
                let mut pieces = Vec::new();
                let mut missing = 0usize;
                for (index, c) in text.chars().enumerate() {
                    let (runs, absent) = match glyph(c) {
                        Some(g) => (g, false),
                        None => (BOX, true),
                    };
                    if absent {
                        missing += 1;
                    }
                    let origin = index as i64 * ADVANCE;
                    for run in runs.split(';').filter(|r| !r.is_empty()) {
                        let mut previous: Option<Point> = None;
                        for pair in run.split(' ') {
                            let (gx, gy) = pair.split_once(',').expect("the font is well formed");
                            let gx: i64 = gx.parse().expect("the font is well formed");
                            let gy: i64 = gy.parse().expect("the font is well formed");
                            let mut x = i128::from(origin + gx) * h / i128::from(CELL_H);
                            let y = i128::from(gy) * h / i128::from(CELL_H);
                            if *mirrored {
                                x = -x;
                            }
                            let (rx, ry) = if exact {
                                match quarter {
                                    0 => (x, y),
                                    1 => (-y, x),
                                    2 => (-x, -y),
                                    _ => (y, -x),
                                }
                            } else {
                                let (fx, fy) = (x as f64, y as f64);
                                (
                                    (fx * cos - fy * sin).round() as i128,
                                    (fx * sin + fy * cos).round() as i128,
                                )
                            };
                            let p = point(
                                i128::from(at.x_nm.get()) + rx,
                                i128::from(at.y_nm.get()) + ry,
                            )?;
                            if let Some(from) = previous {
                                if from != p {
                                    pieces.push(Segment::Line {
                                        start: from,
                                        end: p,
                                    });
                                }
                            }
                            previous = Some(p);
                        }
                    }
                }
                Ok(Drawn {
                    width: *stroke,
                    pieces,
                    missing_glyphs: missing,
                })
            }
        }
    }
}

fn make(code: &'static str, severity: Severity, detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, severity, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Geometric)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

impl Board {
    /// The problems of the silkscreen items, in a fixed order (by id). Nothing when there are none.
    pub fn silk_problems(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for (id, item) in &self.silkscreen {
            let what = format!("silkscreen item {id}");
            match self.layers.get(&item.layer()) {
                None => out.push(make(
                    DANGLING,
                    Severity::Error,
                    format!("{what} names a layer that does not exist"),
                )),
                Some(l) if l.kind != LayerKind::Silkscreen => out.push(make(
                    BAD_LAYER,
                    Severity::Error,
                    format!("{what} is on a layer that is not silkscreen"),
                )),
                Some(_) => {}
            }
            if item.line_width().get() <= 0 {
                out.push(make(
                    BAD_SIZE,
                    Severity::Error,
                    format!("line width of {what}"),
                ));
            }
            match item {
                SilkItem::Text { text, height, .. } => {
                    if text.is_empty() || height.get() <= 0 {
                        out.push(make(BAD_SIZE, Severity::Error, format!("text of {what}")));
                    }
                    match item.drawn() {
                        Err(DrawError::Rotation) => {
                            out.push(make(TEXT_ROTATION, Severity::Error, what.clone()))
                        }
                        Err(DrawError::Range) if height.get() > 0 => {
                            out.push(make(TEXT_OUT_OF_RANGE, Severity::Error, what.clone()))
                        }
                        Err(DrawError::Range) => {}
                        Ok(d) if d.missing_glyphs > 0 => out.push(make(
                            TEXT_NO_GLYPH,
                            Severity::Warning,
                            format!("{what}: {} character(s)", d.missing_glyphs),
                        )),
                        Ok(_) => {}
                    }
                }
                SilkItem::Stroke { path, .. } => {
                    if path.is_empty() {
                        out.push(make(
                            PATH_NOT_CONNECTED,
                            Severity::Error,
                            format!("{what} has no pieces"),
                        ));
                    }
                    for (i, piece) in path.iter().enumerate() {
                        if i > 0 && path[i - 1].end() != piece.start() {
                            out.push(make(
                                PATH_NOT_CONNECTED,
                                Severity::Error,
                                format!("{what} at piece {i}"),
                            ));
                        }
                        if let Some(Err(e)) = piece.to_arc() {
                            out.push(make(
                                BAD_ARC,
                                Severity::Error,
                                format!("{what} at piece {i}: {e}"),
                            ));
                        }
                        if let Segment::Line { start, end } = piece {
                            if start == end {
                                out.push(make(
                                    BAD_SIZE,
                                    Severity::Error,
                                    format!("{what} has a piece of length zero"),
                                ));
                            }
                        }
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Layer;
    use crate::ops::Operation;
    use eh3d_phoenix_model::ObjectId;

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }
    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    fn board() -> Board {
        let mut b = Board::default();
        b.layers.insert(
            id(1),
            Layer {
                name: "Top silk".into(),
                kind: LayerKind::Silkscreen,
                side: Some(crate::model::Side::Top),
            },
        );
        b.layers.insert(
            id(2),
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b
    }

    fn text(s: &str, rotation: i64, mirrored: bool) -> SilkItem {
        SilkItem::Text {
            layer: id(1),
            at: pt(1_000_000, 2_000_000),
            text: s.into(),
            height: nm(1_200_000),
            stroke: nm(150_000),
            rotation: Udeg::new(rotation).unwrap(),
            mirrored,
        }
    }

    #[test]
    fn the_font_is_well_formed_and_every_glyph_fits_its_cell() {
        for c in (b' '..=b'~').map(char::from) {
            if let Some(g) = glyph(c) {
                for run in g.split(';').filter(|r| !r.is_empty()) {
                    assert!(run.split(' ').count() >= 2, "{c}: {run}");
                    for pair in run.split(' ') {
                        let (x, y) = pair.split_once(',').unwrap();
                        let (x, y): (i64, i64) = (x.parse().unwrap(), y.parse().unwrap());
                        assert!(
                            (0..=CELL_W).contains(&x) && (0..=CELL_H).contains(&y),
                            "{c}"
                        );
                    }
                }
            }
        }
        for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-+./_()=:*% ".chars() {
            assert!(glyph(c).is_some(), "{c}");
        }
        assert!(glyph('a') == glyph('A'));
        assert!(glyph('\u{e9}') == glyph('E') && glyph('\u{d1}') == glyph('N'));
        assert!(glyph('\u{e7}') == glyph('C') && glyph('\u{fc}') == glyph('U'));
        assert!(glyph('€').is_none());
    }

    #[test]
    fn a_text_is_drawn_with_integer_points_and_lower_case_as_capitals() {
        let d = text("R1", 0, false).drawn().unwrap();
        assert_eq!(d.missing_glyphs, 0);
        assert!(!d.pieces.is_empty());
        assert_eq!(d, text("r1", 0, false).drawn().unwrap());
        // the first stroke of R starts at the origin of the text
        assert_eq!(d.pieces[0].start(), pt(1_000_000, 2_000_000));
        // the height of the capitals is the height given
        let top = d
            .pieces
            .iter()
            .map(|s| s.start().y_nm.get().max(s.end().y_nm.get()))
            .max()
            .unwrap();
        assert_eq!(top, 2_000_000 + 1_200_000);
    }

    #[test]
    fn rotation_and_mirror_move_the_strokes_exactly() {
        let flat = text("L", 0, false).drawn().unwrap();
        let up = text("L", 90_000_000, false).drawn().unwrap();
        let back = text("L", 0, true).drawn().unwrap();
        let (at_x, at_y) = (1_000_000, 2_000_000);
        for (a, b) in flat.pieces.iter().zip(&up.pieces) {
            // a quarter turn about `at`: (x, y) -> (-y, x)
            let (x, y) = (a.start().x_nm.get() - at_x, a.start().y_nm.get() - at_y);
            assert_eq!(b.start(), pt(at_x - y, at_y + x));
        }
        for (a, b) in flat.pieces.iter().zip(&back.pieces) {
            assert_eq!(b.start().x_nm.get() - at_x, -(a.start().x_nm.get() - at_x));
            assert_eq!(b.start().y_nm, a.start().y_nm);
        }
        // any other angle is allowed: the origin stays, the points turn about it
        let turned = text("L", 45_000_000, false).drawn().unwrap();
        assert_eq!(turned.pieces.len(), flat.pieces.len());
        assert_eq!(turned.pieces[0].end(), flat.pieces[0].end());
        assert_ne!(turned.pieces[0].start(), flat.pieces[0].start());
        // the stem of the L is 6 units long, 1.2 mm; turned by 45 degrees its end is 0.8485 mm away
        // on both axes
        let (sx, sy) = (turned.pieces[0].start(), turned.pieces[0].end());
        let (dx, dy) = (sy.x_nm.get() - sx.x_nm.get(), sy.y_nm.get() - sx.y_nm.get());
        assert!(
            (dx.abs() - 848_528).abs() <= 1 && (dy.abs() - 848_528).abs() <= 1,
            "{dx} {dy}"
        );
    }

    #[test]
    fn a_character_without_a_glyph_is_a_box_and_a_warning() {
        let d = text("A\u{3a9}", 0, false).drawn().unwrap();
        assert_eq!(d.missing_glyphs, 1);
        let mut b = board();
        b.silkscreen.insert(id(50), text("A\u{3a9}", 0, false));
        let codes: Vec<_> = b
            .silk_problems()
            .iter()
            .map(|d| (d.code.as_deref().unwrap().to_owned(), d.severity))
            .collect();
        assert_eq!(codes, vec![(TEXT_NO_GLYPH.to_owned(), Severity::Warning)]);
        assert!(b.is_consistent());
    }

    #[test]
    fn each_kind_of_problem_has_its_code() {
        let mut b = board();
        let codes = |b: &Board| -> Vec<String> {
            b.silk_problems()
                .iter()
                .filter_map(|d| d.code.as_deref().map(str::to_owned))
                .collect()
        };
        assert!(codes(&b).is_empty());
        // a layer that does not exist, and one that is not silkscreen
        b.silkscreen.insert(
            id(51),
            SilkItem::Stroke {
                layer: id(99),
                width: nm(100_000),
                path: vec![Segment::Line {
                    start: pt(0, 0),
                    end: pt(1, 1),
                }],
            },
        );
        b.silkscreen.insert(
            id(52),
            SilkItem::Stroke {
                layer: id(2),
                width: nm(100_000),
                path: vec![Segment::Line {
                    start: pt(0, 0),
                    end: pt(1, 1),
                }],
            },
        );
        assert_eq!(codes(&b), vec![DANGLING, BAD_LAYER]);
        // sizes, empty text, pieces that do not join and an empty stroke
        b.silkscreen.clear();
        b.silkscreen.insert(id(53), text("", 0, false));
        b.silkscreen.insert(
            id(54),
            SilkItem::Stroke {
                layer: id(1),
                width: nm(0),
                path: vec![],
            },
        );
        b.silkscreen.insert(
            id(55),
            SilkItem::Stroke {
                layer: id(1),
                width: nm(100_000),
                path: vec![
                    Segment::Line {
                        start: pt(0, 0),
                        end: pt(5, 5),
                    },
                    Segment::Line {
                        start: pt(9, 9),
                        end: pt(10, 10),
                    },
                ],
            },
        );
        assert_eq!(
            codes(&b),
            vec![BAD_SIZE, BAD_SIZE, PATH_NOT_CONNECTED, PATH_NOT_CONNECTED]
        );
        assert!(!b.is_consistent());
    }

    #[test]
    fn a_text_at_the_edge_of_the_range_is_reported_not_a_panic() {
        let mut b = board();
        b.silkscreen.insert(
            id(50),
            SilkItem::Text {
                layer: id(1),
                at: Point::from_nm(Nm::MAX.get(), 0).unwrap(),
                text: "WIDE".into(),
                height: nm(1_000_000),
                stroke: nm(100_000),
                rotation: Udeg::ZERO,
                mirrored: false,
            },
        );
        let codes: Vec<_> = b
            .silk_problems()
            .iter()
            .filter_map(|d| d.code.as_deref().map(str::to_owned))
            .collect();
        assert_eq!(codes, vec![TEXT_OUT_OF_RANGE]);
    }

    #[test]
    fn silkscreen_is_additive_in_the_file() {
        let mut b = board();
        b.layers.get_mut(&id(1)).unwrap().side = None;
        let without = String::from_utf8(b.to_bytes().unwrap()).unwrap();
        assert!(
            !without.contains("\"silkscreen\":"),
            "an empty map is not written"
        );
        assert!(
            !without.contains("\"side\""),
            "a layer without a side is as before"
        );
        assert_eq!(Board::from_bytes(without.as_bytes()).unwrap(), b);
        b.silkscreen.insert(id(50), text("R1", 0, true));
        b.silkscreen.insert(
            id(51),
            SilkItem::Stroke {
                layer: id(1),
                width: nm(150_000),
                path: vec![Segment::Line {
                    start: pt(0, 0),
                    end: pt(3_000_000, 0),
                }],
            },
        );
        let with = b.to_bytes().unwrap();
        let back = Board::from_bytes(&with).unwrap();
        assert_eq!(back, b);
        assert_eq!(back.to_bytes().unwrap(), with);
        // a text that is not mirrored does not write the flag
        let mut c = board();
        c.silkscreen.insert(id(50), text("R1", 0, false));
        assert!(!String::from_utf8(c.to_bytes().unwrap())
            .unwrap()
            .contains("mirrored"));
    }

    #[test]
    fn adding_and_removing_silkscreen_has_an_exact_inverse() {
        let mut b = board();
        let before = b.clone();
        let undo = b
            .apply(&Operation::AddSilk {
                id: id(50),
                item: text("R1", 0, false),
            })
            .unwrap();
        assert_eq!(b.silkscreen.len(), 1);
        let undo_again = b.apply(&undo).unwrap();
        assert_eq!(b, before);
        b.apply(&undo_again).unwrap();
        assert_eq!(b.silkscreen.len(), 1);
        // refused, and the board stays: the id exists, the layer is missing, the layer is copper
        let kept = b.clone();
        for bad in [
            Operation::AddSilk {
                id: id(50),
                item: text("A", 0, false),
            },
            Operation::AddSilk {
                id: id(51),
                item: SilkItem::Stroke {
                    layer: id(99),
                    width: nm(1),
                    path: vec![],
                },
            },
            Operation::AddSilk {
                id: id(52),
                item: SilkItem::Stroke {
                    layer: id(2),
                    width: nm(1),
                    path: vec![],
                },
            },
            Operation::RemoveSilk { id: id(77) },
        ] {
            assert!(b.apply(&bad).is_err());
            assert_eq!(b, kept);
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
