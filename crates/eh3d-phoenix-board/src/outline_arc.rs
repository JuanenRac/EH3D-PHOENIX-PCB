//! Arcs in the outline of the board.
//!
//! The outline of a board (`Board::outline`) is a closed polygon of corners. An arc of the outline
//! replaces one straight edge of that polygon: it is kept apart in `Board::outline_arcs` as the
//! index of the corner where the edge starts and a signed sweep, and its end points are the two
//! corners of the edge. A board without arcs is written as before. [`Board::outline_polygon`] cuts
//! every arc into chords that are at most `tolerance_nm` from the arc (`config/outline_arcs.cfg`),
//! which is the polygon that the checks of distance, the pour of the zones and the router see.
//!
//! The arithmetic of the cut is in floating point and every point is rounded to the nanometre, so
//! the same board gives the same polygon on the machines that round the same way; the points of
//! the corners themselves are never moved.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_model::{Arc, Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::model::Board;

/// The tolerance of the cut, in nanometres, when no file says another.
const DEFAULT_TOLERANCE_NM: i64 = 1_000;
/// The most chords one arc is cut into, when no file says another.
const DEFAULT_MAX_PIECES: u32 = 1_024;
/// The largest tolerance that the file may ask for, in nanometres.
const TOLERANCE_CEILING_NM: i64 = 1_000_000;
/// The most chords that the file may allow for one arc.
const PIECES_CEILING: u32 = 65_536;

/// An arc of the outline: the edge that starts at corner `edge` and ends at the next corner is a
/// circular arc of the signed sweep, and not a straight piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineArc {
    /// The index in `Board::outline` of the corner where the edge starts.
    pub edge: u32,
    /// The sweep of the arc from the first corner to the next; positive is counter-clockwise.
    pub sweep: Udeg,
}

/// How an arc of the outline is cut into chords, read from `config/outline_arcs.cfg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcSettings {
    /// The most a chord may be from its arc, in nanometres.
    pub tolerance_nm: i64,
    /// The most chords one arc is cut into, whatever the tolerance.
    pub max_pieces: u32,
}

impl Default for ArcSettings {
    fn default() -> Self {
        ArcSettings {
            tolerance_nm: DEFAULT_TOLERANCE_NM,
            max_pieces: DEFAULT_MAX_PIECES,
        }
    }
}

thread_local! {
    /// The settings in force for the thread that runs a command or draws the window.
    static CURRENT: Cell<ArcSettings> = Cell::new(ArcSettings::default());
}

/// Puts the settings that were in force back when it is dropped; see [`ArcSettings::set_current`].
#[must_use = "the settings are in force only while the guard lives"]
pub struct ArcSettingsGuard(ArcSettings);

impl Drop for ArcSettingsGuard {
    fn drop(&mut self) {
        CURRENT.with(|cell| cell.set(self.0));
    }
}

impl ArcSettings {
    /// The settings in force for this thread: the ones that `--limits` (or the window) put in
    /// force with [`ArcSettings::set_current`], and the defaults when nothing did. The fill of
    /// zones, the router, the scene of the window and the writers cut the arcs of the outline with
    /// these.
    pub fn current() -> ArcSettings {
        CURRENT.with(Cell::get)
    }

    /// Puts these settings in force for this thread until the guard is dropped, then puts back
    /// the ones that were. Another thread keeps its own.
    pub fn set_current(self) -> ArcSettingsGuard {
        ArcSettingsGuard(CURRENT.with(|cell| cell.replace(self)))
    }

    /// Reads the settings from a text of `key = value` lines. Blank lines and lines that start
    /// with `#` are ignored and a missing key keeps its default; an unknown key, a value that is
    /// not a whole number in its range or a line that is not `key = value` is an error that names
    /// the line.
    pub fn parse(text: &str) -> Result<ArcSettings, String> {
        let mut settings = ArcSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {line}: not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "tolerance_nm" => {
                    let n: i64 = value
                        .parse()
                        .map_err(|_| format!("line {line}: {key} is not a whole number"))?;
                    if !(1..=TOLERANCE_CEILING_NM).contains(&n) {
                        return Err(format!(
                            "line {line}: {key} must be from 1 to {TOLERANCE_CEILING_NM}"
                        ));
                    }
                    settings.tolerance_nm = n;
                }
                "max_pieces" => {
                    let n: u32 = value
                        .parse()
                        .map_err(|_| format!("line {line}: {key} is not a whole number"))?;
                    if !(1..=PIECES_CEILING).contains(&n) {
                        return Err(format!(
                            "line {line}: {key} must be from 1 to {PIECES_CEILING}"
                        ));
                    }
                    settings.max_pieces = n;
                }
                _ => return Err(format!("line {line}: unknown key {key}")),
            }
        }
        Ok(settings)
    }
}

/// Why an arc of the outline cannot be used, or `None` when it can. `corners` is the number of
/// corners of the outline.
pub fn arc_problem(
    corners: usize,
    arc: &OutlineArc,
    start: Option<Point>,
    end: Option<Point>,
) -> Option<String> {
    if arc.edge as usize >= corners {
        return Some(format!(
            "its edge {} is not an edge of an outline of {corners} corners",
            arc.edge
        ));
    }
    let (Some(start), Some(end)) = (start, end) else {
        return Some("its end points are not corners of the outline".to_owned());
    };
    if arc.sweep == Udeg::ZERO
        || arc.sweep.get().unsigned_abs() >= Udeg::FULL_TURN.get().unsigned_abs()
    {
        return Some("its sweep must be above zero and below a full turn".to_owned());
    }
    match Arc::new(start, end, arc.sweep) {
        Ok(_) => None,
        Err(e) => Some(e.to_string()),
    }
}

/// The points of an arc between its end points, not counting the end points, with chords at most
/// `settings.tolerance_nm` from the arc. Nothing when the arc has no shape.
pub fn arc_interior(start: Point, end: Point, sweep: Udeg, settings: &ArcSettings) -> Vec<Point> {
    let (sx, sy) = (start.x_nm.get() as f64, start.y_nm.get() as f64);
    let (ex, ey) = (end.x_nm.get() as f64, end.y_nm.get() as f64);
    let (cx, cy) = (ex - sx, ey - sy);
    let d = (cx * cx + cy * cy).sqrt();
    let theta = sweep.get() as f64 / 1_000_000.0 * std::f64::consts::PI / 180.0;
    let half = theta.abs() / 2.0;
    if d == 0.0 || half.sin().abs() < 1e-9 || !d.is_finite() {
        return Vec::new();
    }
    let radius = d / (2.0 * half.sin());
    let h = radius * half.cos();
    let sign = if sweep.get() >= 0 { 1.0 } else { -1.0 };
    let centre = (
        sx + cx / 2.0 + sign * (-cy / d) * h,
        sy + cy / 2.0 + sign * (cx / d) * h,
    );
    // the angle one chord may span so that the arc is at most the tolerance from it
    let ratio = (1.0 - settings.tolerance_nm as f64 / radius).clamp(-1.0, 1.0);
    let widest = 2.0 * ratio.acos();
    let wanted = if widest > 1e-9 {
        (theta.abs() / widest).ceil()
    } else {
        f64::from(settings.max_pieces)
    };
    let pieces = wanted.clamp(1.0, f64::from(settings.max_pieces)) as u32;
    let first = (sy - centre.1).atan2(sx - centre.0);
    let mut out: Vec<Point> = Vec::new();
    for k in 1..pieces {
        let a = first + theta * f64::from(k) / f64::from(pieces);
        let x = (centre.0 + radius * a.cos()).round();
        let y = (centre.1 + radius * a.sin()).round();
        if let Ok(p) = Point::from_nm(x as i64, y as i64) {
            if p != start && p != end && out.last() != Some(&p) {
                out.push(p);
            }
        }
    }
    out
}

impl Board {
    /// The arcs of the outline that can be used, by the index of the corner where the edge starts:
    /// for each edge the first arc that has a shape (see [`arc_problem`]).
    pub fn usable_outline_arcs(&self) -> BTreeMap<usize, Udeg> {
        let n = self.outline.len();
        let mut out: BTreeMap<usize, Udeg> = BTreeMap::new();
        if n < 3 {
            return out;
        }
        for arc in &self.outline_arcs {
            let edge = arc.edge as usize;
            if edge >= n || out.contains_key(&edge) {
                continue;
            }
            let (start, end) = (self.outline[edge], self.outline[(edge + 1) % n]);
            if arc_problem(n, arc, Some(start), Some(end)).is_none() {
                out.insert(edge, arc.sweep);
            }
        }
        out
    }

    /// The outline as a polygon of straight edges: the corners of `outline` with the points of
    /// each arc of `outline_arcs` between the two corners of its edge. An arc that cannot be used
    /// (see [`arc_problem`]) is left as the straight edge. The outline itself when it has no arcs.
    pub fn outline_polygon(&self, settings: &ArcSettings) -> Vec<Point> {
        let n = self.outline.len();
        if self.outline_arcs.is_empty() || n < 3 {
            return self.outline.clone();
        }
        let arcs = self.usable_outline_arcs();
        let mut out: Vec<Point> = Vec::with_capacity(n);
        for i in 0..n {
            let (start, end) = (self.outline[i], self.outline[(i + 1) % n]);
            out.push(start);
            if let Some(sweep) = arcs.get(&i) {
                out.extend(arc_interior(start, end, *sweep, settings));
            }
        }
        out
    }

    /// Every problem of the arcs of the outline, each said in a few words with the number of the
    /// arc (from 1): an edge that does not exist, an edge with two arcs, a sweep that is zero or a
    /// full turn or more, end points that cannot make an arc.
    pub fn outline_arc_problems(&self) -> Vec<String> {
        let n = self.outline.len();
        let mut out: Vec<String> = Vec::new();
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        for (i, arc) in self.outline_arcs.iter().enumerate() {
            let at = |k: usize| self.outline.get(k).copied();
            let (start, end) = (
                at(arc.edge as usize),
                at((arc.edge as usize + 1) % n.max(1)),
            );
            if let Some(why) = arc_problem(n, arc, start, end) {
                out.push(format!("outline arc {}: {why}", i + 1));
            } else if seen.contains(&arc.edge) {
                out.push(format!(
                    "outline arc {}: edge {} already has an arc",
                    i + 1,
                    arc.edge
                ));
            }
            seen.insert(arc.edge);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    const MM: i64 = 1_000_000;

    fn deg(d: i64) -> Udeg {
        Udeg::new(d * 1_000_000).unwrap()
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/outline_arcs.cfg");
        assert_eq!(ArcSettings::parse(text).unwrap(), ArcSettings::default());
    }

    #[test]
    fn lines_change_the_values_and_bad_lines_name_their_number() {
        let s = ArcSettings::parse("# a\ntolerance_nm = 250\nmax_pieces = 64\n").unwrap();
        assert_eq!((s.tolerance_nm, s.max_pieces), (250, 64));
        assert!(ArcSettings::parse("tolerance_nm = 0\n")
            .unwrap_err()
            .contains("line 1"));
        assert!(ArcSettings::parse("\nmax_pieces = 70000\n")
            .unwrap_err()
            .contains("line 2"));
        assert!(ArcSettings::parse("tolerance_nm = x\n").is_err());
        assert!(ArcSettings::parse("size = 3\n").is_err());
        assert!(ArcSettings::parse("tolerance_nm\n").is_err());
    }

    #[test]
    fn a_quarter_circle_of_one_millimetre_is_cut_into_eighteen_chords() {
        // radius 1 mm, tolerance 1 micrometre: a chord may span 2 * acos(1 - 0.001) = 0.08945 rad,
        // so 90 degrees (1.5708 rad) need ceil(17.56) = 18 chords of 5 degrees; 17 points between
        // the end points. 17 chords would have a sagitta of 1067 nm, 18 have 951.8 nm.
        let points = arc_interior(pt(MM, 0), pt(0, MM), deg(90), &ArcSettings::default());
        assert_eq!(points.len(), 17);
        // 5 degrees: (cos, sin) = (0.9961947, 0.0871557) mm
        assert_eq!(points[0], pt(996_195, 87_156));
        // 30 degrees and 45 degrees, from the exact values 0.8660254 and 0.5, 0.7071068
        assert_eq!(points[5], pt(866_025, 500_000));
        assert_eq!(points[8], pt(707_107, 707_107));
        assert_eq!(points[16], pt(87_156, 996_195));
        // every point is on the circle, within the rounding of a nanometre
        for p in &points {
            let r = (p.x_nm.get() as f64).hypot(p.y_nm.get() as f64);
            assert!((r - 1e6).abs() <= 0.8, "{r}");
        }
    }

    #[test]
    fn a_clockwise_arc_bulges_to_the_other_side() {
        // the same end points with a sweep of -90 degrees: the centre is at (1 mm, 1 mm) and the
        // arc passes through (1 - cos 45, 1 - sin 45) mm = (0.2928932, 0.2928932) mm
        let points = arc_interior(pt(MM, 0), pt(0, MM), deg(-90), &ArcSettings::default());
        assert_eq!(points.len(), 17);
        assert_eq!(points[8], pt(292_893, 292_893));
    }

    #[test]
    fn a_looser_tolerance_or_a_cap_gives_fewer_chords() {
        // tolerance 10 micrometres: 2 * acos(0.99) = 0.283078 rad, ceil(5.549) = 6 chords
        let loose = ArcSettings {
            tolerance_nm: 10_000,
            max_pieces: 1_024,
        };
        assert_eq!(arc_interior(pt(MM, 0), pt(0, MM), deg(90), &loose).len(), 5);
        // a cap of 4 chords wins over the tolerance
        let capped = ArcSettings {
            tolerance_nm: 1_000,
            max_pieces: 4,
        };
        assert_eq!(
            arc_interior(pt(MM, 0), pt(0, MM), deg(90), &capped).len(),
            3
        );
        // no shape, no points
        assert!(arc_interior(pt(0, 0), pt(0, 0), deg(90), &loose).is_empty());
        assert!(arc_interior(pt(0, 0), pt(MM, 0), Udeg::ZERO, &loose).is_empty());
    }

    fn rounded_corner_board() -> Board {
        // a square of 10 mm with the corner at (10, 10) cut by a quarter circle of radius 2 mm
        // that bulges outwards... here the edge from (10, 8) to (8, 10) is the arc, centre
        // (8, 8), and the polygon is (0,0), (10,0), (10,8), (8,10), (0,10)
        Board {
            outline: vec![
                pt(0, 0),
                pt(10 * MM, 0),
                pt(10 * MM, 8 * MM),
                pt(8 * MM, 10 * MM),
                pt(0, 10 * MM),
            ],
            outline_arcs: vec![OutlineArc {
                edge: 2,
                sweep: deg(90),
            }],
            ..Board::default()
        }
    }

    #[test]
    fn the_polygon_of_an_outline_with_an_arc_has_the_points_of_the_arc_in_its_place() {
        let board = rounded_corner_board();
        let flat = board.outline_polygon(&ArcSettings::default());
        // radius 2 mm: 2 * acos(1 - 0.0005) = 0.063246 rad, ceil(24.84) = 25 chords, 24 points
        assert_eq!(flat.len(), 5 + 24);
        // the corners keep their order: the arc points come right after (10, 8)
        assert_eq!(flat[2], pt(10 * MM, 8 * MM));
        assert_eq!(flat[27], pt(8 * MM, 10 * MM));
        assert_eq!(flat[28], pt(0, 10 * MM));
        // the arc points are on the circle of radius 2 mm about (8, 8), between the corners
        for p in &flat[3..27] {
            let r = ((p.x_nm.get() - 8 * MM) as f64).hypot((p.y_nm.get() - 8 * MM) as f64);
            assert!((r - 2e6).abs() <= 0.8, "{r}");
            assert!(p.x_nm.get() > 8 * MM && p.y_nm.get() > 8 * MM);
        }
        // a board without arcs is its own polygon
        let plain = Board {
            outline_arcs: Vec::new(),
            ..board.clone()
        };
        assert_eq!(
            plain.outline_polygon(&ArcSettings::default()),
            plain.outline
        );
    }

    #[test]
    fn the_settings_in_force_change_the_polygon_and_are_put_back() {
        let board = rounded_corner_board();
        assert_eq!(ArcSettings::current(), ArcSettings::default());
        let default_len = board.outline_polygon(&ArcSettings::current()).len();
        assert_eq!(default_len, 5 + 24);
        let loose = ArcSettings {
            tolerance_nm: 10_000,
            max_pieces: 1_024,
        };
        {
            let _guard = loose.set_current();
            assert_eq!(ArcSettings::current(), loose);
            let coarse = board.outline_polygon(&ArcSettings::current());
            // radius 2 mm, 10 micrometres: 2 * acos(0.995) = 0.200 rad, ceil(7.85) = 8 chords
            assert_eq!(coarse.len(), 5 + 7);
            // a guard inside gives its own, and the outer one is back after it
            {
                let cap = ArcSettings {
                    tolerance_nm: 1_000,
                    max_pieces: 2,
                };
                let _inner = cap.set_current();
                assert_eq!(board.outline_polygon(&ArcSettings::current()).len(), 5 + 1);
            }
            assert_eq!(ArcSettings::current(), loose);
        }
        assert_eq!(ArcSettings::current(), ArcSettings::default());
        assert_eq!(
            board.outline_polygon(&ArcSettings::current()).len(),
            default_len
        );
    }

    #[test]
    fn the_settings_in_force_belong_to_the_thread_that_set_them() {
        let _guard = ArcSettings {
            tolerance_nm: 5_000,
            max_pieces: 8,
        }
        .set_current();
        let other = std::thread::spawn(ArcSettings::current).join().unwrap();
        assert_eq!(other, ArcSettings::default());
        assert_eq!(ArcSettings::current().max_pieces, 8);
    }

    #[test]
    fn the_problems_of_the_arcs_are_found() {
        let mut board = rounded_corner_board();
        assert!(board.outline_arc_problems().is_empty());
        board.outline_arcs = vec![
            OutlineArc {
                edge: 9,
                sweep: deg(90),
            },
            OutlineArc {
                edge: 2,
                sweep: Udeg::ZERO,
            },
            OutlineArc {
                edge: 2,
                sweep: deg(90),
            },
            OutlineArc {
                edge: 2,
                sweep: deg(90),
            },
            OutlineArc {
                edge: 0,
                sweep: Udeg::FULL_TURN,
            },
        ];
        let problems = board.outline_arc_problems();
        assert_eq!(problems.len(), 5);
        assert!(problems[0]
            .starts_with("outline arc 1: its edge 9 is not an edge of an outline of 5 corners"));
        assert!(problems[1].starts_with("outline arc 2: its sweep must be above zero"));
        // the third and the fourth arc are fine by themselves, but the edge is taken
        assert!(
            problems[2].starts_with("outline arc 3") && problems[2].contains("already has an arc")
        );
        assert!(
            problems[3].starts_with("outline arc 4") && problems[3].contains("already has an arc")
        );
        assert!(problems[4].starts_with("outline arc 5: its sweep"));
        // an arc that cannot be used leaves the straight edge
        let mut bad = rounded_corner_board();
        bad.outline_arcs[0].sweep = Udeg::ZERO;
        assert_eq!(bad.outline_polygon(&ArcSettings::default()), bad.outline);
    }
}
