//! The consistency checks of a board and its diagnostic codes `PHOENIX-BOARD-<NNN>`.
//!
//! A reader never panics and never guesses: whatever is wrong becomes a diagnostic with a stable
//! code. All problems are collected, in a fixed order, so the same board always gives the same list.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{segments_intersect, Nm, Point};

use crate::model::{Board, Id, LayerKind, Segment};
use crate::outline_arc::ArcSettings;

/// A reference to a part that does not exist.
pub const DANGLING: &str = "PHOENIX-BOARD-001";
/// A track whose pieces do not join.
pub const PATH_NOT_CONNECTED: &str = "PHOENIX-BOARD-002";
/// A straight piece of track that is not at a multiple of 45 degrees.
pub const OFF_GRID_ANGLE: &str = "PHOENIX-BOARD-003";
/// A size that is zero or negative where it must be positive.
pub const BAD_SIZE: &str = "PHOENIX-BOARD-004";
/// A pad given to more than one net, or the list of pad nets out of order.
pub const PAD_NET_CONFLICT: &str = "PHOENIX-BOARD-005";
/// An outline that is not a closed polygon.
pub const BAD_OUTLINE: &str = "PHOENIX-BOARD-006";
/// A layer of the wrong kind or a via that starts and ends on one layer.
pub const BAD_LAYER: &str = "PHOENIX-BOARD-007";
/// Two components with the same reference designator.
pub const DUPLICATE_REFDES: &str = "PHOENIX-BOARD-008";
/// An arc that has no shape.
pub const BAD_ARC: &str = "PHOENIX-BOARD-009";
/// A polygon with more points than the limit: its edges were not tested for crossing.
pub const POLYGON_NOT_CHECKED: &str = "PHOENIX-BOARD-017";

/// The limits of the validation, read from `config/board.cfg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardLimits {
    /// The most points of a polygon (outline, cutout or zone) whose edges are tested for
    /// crossing each other; the test takes time with the square of the number of points.
    pub max_polygon_points: usize,
}

/// The most that `max_polygon_points` may be set to.
const MAX_POLYGON_POINTS_CEILING: usize = 1_000_000;

impl Default for BoardLimits {
    fn default() -> Self {
        BoardLimits {
            max_polygon_points: 10_000,
        }
    }
}

impl BoardLimits {
    /// Reads the limits from a settings text of `key = value` lines. Blank lines and lines that
    /// start with `#` are ignored, a missing key keeps its default and an unknown key, a value
    /// that is not a whole number from 3 to 1000000 or a line that is not `key = value` is an
    /// error that names the line.
    pub fn parse(text: &str) -> Result<BoardLimits, String> {
        let mut limits = BoardLimits::default();
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
                "max_polygon_points" => {
                    let n: usize = value
                        .parse()
                        .map_err(|_| format!("line {line}: {key} is not a whole number"))?;
                    if !(3..=MAX_POLYGON_POINTS_CEILING).contains(&n) {
                        return Err(format!(
                            "line {line}: {key} must be from 3 to {MAX_POLYGON_POINTS_CEILING}"
                        ));
                    }
                    limits.max_polygon_points = n;
                }
                _ => return Err(format!("line {line}: unknown key {key}")),
            }
        }
        Ok(limits)
    }
}

/// Every code of this crate with its title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (DANGLING, "Reference to a part that does not exist"),
    (PATH_NOT_CONNECTED, "Track pieces do not join"),
    (
        OFF_GRID_ANGLE,
        "Straight track piece off the 45-degree grid",
    ),
    (BAD_SIZE, "Size is zero or negative"),
    (
        PAD_NET_CONFLICT,
        "Pad assigned twice or pad nets out of order",
    ),
    (BAD_OUTLINE, "Outline is not a closed polygon"),
    (BAD_LAYER, "Layer of the wrong kind"),
    (DUPLICATE_REFDES, "Duplicate reference designator"),
    (BAD_ARC, "Arc has no shape"),
    (
        POLYGON_NOT_CHECKED,
        "Polygon has more points than the limit, its edges were not tested for crossing",
    ),
];

/// The tags of the languages of [`TITLES`], in the order of its columns.
pub const TITLE_LANGUAGES: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

/// The titles of the codes that are printed for a person, in English, Spanish, German, French, Italian, Japanese,
/// Chinese and European Portuguese. The messages of the diagnostics keep the English title.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        POLYGON_NOT_CHECKED,
        [
            "Polygon has more points than the limit, its edges were not tested for crossing",
            "El polígono tiene más puntos que el límite, no se comprobó si sus aristas se cruzan",
            "Das Polygon hat mehr Punkte als das Limit, seine Kanten wurden nicht auf Überschneidung geprüft",
            "Le polygone a plus de points que la limite, ses arêtes n'ont pas été testées pour les croisements",
            "Il poligono ha più punti del limite, i suoi lati non sono stati controllati per gli incroci",
            "ポリゴンの点数が上限を超えているため、辺の交差は検査されませんでした",
            "多边形的点数超过上限，未检查其边是否相交",
            "O polígono tem mais pontos do que o limite, as suas arestas não foram verificadas quanto a cruzamentos",
        ],
    ),
];

/// The title of a code in a language (one of [`TITLE_LANGUAGES`]), or none when the code or the language is not
/// in the table.
pub fn title(code: &str, language: &str) -> Option<&'static str> {
    let column = TITLE_LANGUAGES.iter().position(|l| *l == language)?;
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, row)| row[column])
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

/// What the test of a polygon found.
enum PolygonCheck {
    /// Nothing is wrong.
    Fine,
    /// Something is wrong, said in a few words.
    Problem(&'static str),
    /// The polygon has more points than the limit; the edges were not tested for crossing.
    NotCrossChecked,
}

fn polygon_problem(points: &[Point], max_points: usize) -> PolygonCheck {
    if points.len() < 3 {
        return PolygonCheck::Problem("fewer than three points");
    }
    let n = points.len();
    for i in 0..n {
        if points[i] == points[(i + 1) % n] {
            return PolygonCheck::Problem("two consecutive points coincide");
        }
    }
    if n > max_points {
        return PolygonCheck::NotCrossChecked;
    }
    for i in 0..n {
        for j in (i + 1)..n {
            // edges that share a point are neighbours
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            if segments_intersect(
                points[i],
                points[(i + 1) % n],
                points[j],
                points[(j + 1) % n],
            ) {
                return PolygonCheck::Problem("the outline crosses itself");
            }
        }
    }
    PolygonCheck::Fine
}

/// Tests one polygon: an error for a problem, a warning for a polygon over the limit.
fn polygon_findings(
    points: &[Point],
    what: &str,
    max_points: usize,
    error: &mut dyn FnMut(&'static str, String),
    warnings: &mut Vec<Diagnostic>,
) {
    match polygon_problem(points, max_points) {
        PolygonCheck::Fine => {}
        PolygonCheck::Problem(why) => error(BAD_OUTLINE, format!("{what}: {why}")),
        PolygonCheck::NotCrossChecked => warnings.push(make(
            POLYGON_NOT_CHECKED,
            Severity::Warning,
            format!(
                "{what}: {} points are more than the limit max_polygon_points = {max_points}",
                points.len()
            ),
        )),
    }
}

fn positive(value: Nm) -> bool {
    value.get() > 0
}

impl Board {
    /// Every problem of the board, errors and warnings, in a fixed order.
    pub fn validate(&self) -> Vec<Diagnostic> {
        self.validate_with(&BoardLimits::default())
    }

    /// Like [`Board::validate`] with the limits given.
    pub fn validate_with(&self, limits: &BoardLimits) -> Vec<Diagnostic> {
        let max_points = limits.max_polygon_points;
        let mut out = Vec::new();
        let mut error =
            |code: &'static str, detail: String| out.push(make(code, Severity::Error, detail));
        let mut warnings: Vec<Diagnostic> = Vec::new();

        if !self.outline.is_empty() {
            // an outline with arcs is tested as the polygon its arcs are cut into, so that an arc
            // that crosses another edge is found
            let flat = self.outline_polygon(&ArcSettings::default());
            polygon_findings(&flat, "the board", max_points, &mut error, &mut warnings);
        }
        for problem in self.outline_arc_problems() {
            error(BAD_ARC, problem);
        }
        for (zone_id, zone) in &self.zones {
            for (i, cutout) in zone.cutouts.iter().enumerate() {
                let what = format!("cutout {} of zone {zone_id}", i + 1);
                polygon_findings(cutout, &what, max_points, &mut error, &mut warnings);
            }
        }
        for (i, cutout) in self.outline_cutouts.iter().enumerate() {
            let what = format!("cutout {}", i + 1);
            polygon_findings(cutout, &what, max_points, &mut error, &mut warnings);
        }
        let layer_check =
            |id: &Id, what: &str, error: &mut dyn FnMut(&'static str, String)| match self
                .layers
                .get(id)
            {
                None => error(
                    DANGLING,
                    format!("{what} names a layer that does not exist"),
                ),
                Some(l) if l.kind != LayerKind::Copper => {
                    error(
                        BAD_LAYER,
                        format!("{what} is on a layer that is not copper"),
                    );
                }
                Some(_) => {}
            };

        for (id, fp) in &self.footprints {
            for (pad_id, pad) in &fp.pads {
                for layer in &pad.layers {
                    if !self.layers.contains_key(layer) {
                        error(
                            DANGLING,
                            format!("pad {pad_id} of footprint {id} names a missing layer"),
                        );
                    }
                }
                if let Some(d) = pad.drill {
                    if !positive(d) {
                        error(BAD_SIZE, format!("drill of pad {pad_id}"));
                    }
                }
                let sizes = match pad.shape {
                    crate::model::PadShape::Circle { diameter } => vec![diameter],
                    crate::model::PadShape::Rect { width, height }
                    | crate::model::PadShape::Oblong { width, height } => vec![width, height],
                };
                if sizes.iter().any(|s| !positive(*s)) {
                    error(BAD_SIZE, format!("shape of pad {pad_id}"));
                }
                if pad.roundness > crate::model::ROUNDNESS_MAX {
                    error(BAD_SIZE, format!("roundness of pad {pad_id}"));
                }
                if !pad.plated && pad.drill.is_none() {
                    error(
                        BAD_SIZE,
                        format!("pad {pad_id} is not plated and has no drill"),
                    );
                }
            }
        }

        let mut refdes = std::collections::BTreeMap::new();
        for (id, c) in &self.components {
            if !self.footprints.contains_key(&c.footprint) {
                error(
                    DANGLING,
                    format!("component {id} names a missing footprint"),
                );
            }
            if c.refdes.is_empty() {
                error(
                    BAD_SIZE,
                    format!("component {id} has an empty reference designator"),
                );
            } else if let Some(first) = refdes.insert(c.refdes.clone(), *id) {
                error(DUPLICATE_REFDES, format!("components {first} and {id}"));
            }
        }

        let mut previous = None;
        for pn in &self.pad_nets {
            let key = pn.pad_ref();
            if previous.is_some_and(|p| p >= key) {
                error(
                    PAD_NET_CONFLICT,
                    format!("pad {} of component {}", pn.pad, pn.component),
                );
            }
            previous = Some(key);
            match self.components.get(&pn.component) {
                None => error(
                    DANGLING,
                    format!(
                        "a pad assignment names a missing component {}",
                        pn.component
                    ),
                ),
                Some(c) => {
                    let has = self
                        .footprints
                        .get(&c.footprint)
                        .is_some_and(|f| f.pads.contains_key(&pn.pad));
                    if !has {
                        error(
                            DANGLING,
                            format!("component {} has no pad {}", pn.component, pn.pad),
                        );
                    }
                }
            }
            if !self.nets.contains_key(&pn.net) {
                error(
                    DANGLING,
                    format!("a pad assignment names a missing net {}", pn.net),
                );
            }
        }

        for (id, t) in &self.tracks {
            if !self.nets.contains_key(&t.net) {
                error(DANGLING, format!("track {id} names a missing net"));
            }
            layer_check(&t.layer, &format!("track {id}"), &mut error);
            if !positive(t.width) {
                error(BAD_SIZE, format!("width of track {id}"));
            }
            if t.path.is_empty() {
                error(PATH_NOT_CONNECTED, format!("track {id} has no pieces"));
            }
            for (i, piece) in t.path.iter().enumerate() {
                if i > 0 && t.path[i - 1].end() != piece.start() {
                    error(PATH_NOT_CONNECTED, format!("track {id} at piece {i}"));
                }
                match piece {
                    Segment::Line { start, end } => {
                        let dx = end.x_nm.get() - start.x_nm.get();
                        let dy = end.y_nm.get() - start.y_nm.get();
                        if dx == 0 && dy == 0 {
                            error(BAD_SIZE, format!("track {id} has a piece of length zero"));
                        } else if !(dx == 0 || dy == 0 || dx.abs() == dy.abs()) {
                            warnings.push(make(
                                OFF_GRID_ANGLE,
                                Severity::Warning,
                                format!("track {id} at piece {i}"),
                            ));
                        }
                    }
                    Segment::Arc { .. } => {
                        if let Some(Err(e)) = piece.to_arc() {
                            error(BAD_ARC, format!("track {id} at piece {i}: {e}"));
                        }
                    }
                }
            }
        }

        for (id, v) in &self.vias {
            if !self.nets.contains_key(&v.net) {
                error(DANGLING, format!("via {id} names a missing net"));
            }
            layer_check(&v.from_layer, &format!("via {id}"), &mut error);
            layer_check(&v.to_layer, &format!("via {id}"), &mut error);
            if v.from_layer == v.to_layer {
                error(BAD_LAYER, format!("via {id} starts and ends on one layer"));
            }
            if !positive(v.drill) || v.diameter.get() <= v.drill.get() {
                error(BAD_SIZE, format!("drill and diameter of via {id}"));
            }
        }

        for (id, z) in &self.zones {
            if !self.nets.contains_key(&z.net) {
                error(DANGLING, format!("zone {id} names a missing net"));
            }
            layer_check(&z.layer, &format!("zone {id}"), &mut error);
            polygon_findings(
                &z.outline,
                &format!("zone {id}"),
                max_points,
                &mut error,
                &mut warnings,
            );
            if z.clearance.get() < 0 {
                error(BAD_SIZE, format!("clearance of zone {id}"));
            }
        }
        out.extend(warnings);
        out.extend(self.stack_problems());
        out.extend(self.silk_problems());
        out
    }

    /// The polygons whose edges [`Board::validate_with`] did not test for crossing because they
    /// have more points than the limit: the place of each (`the board`, `cutout 2`, `cutout 1 of
    /// zone ...`) and its number of points, in the order of the checks. A polygon with a repeated
    /// point is not listed, as the check reports that problem instead.
    pub fn polygons_not_cross_checked(&self, limits: &BoardLimits) -> Vec<(String, usize)> {
        let max_points = limits.max_polygon_points;
        let mut found = Vec::new();
        let mut look = |what: String, points: &[Point]| {
            // the test of crossing edges only runs at or under the limit, so this is cheap
            if points.len() > max_points
                && matches!(
                    polygon_problem(points, max_points),
                    PolygonCheck::NotCrossChecked
                )
            {
                found.push((what, points.len()));
            }
        };
        if !self.outline.is_empty() {
            look("the board".to_owned(), &self.outline);
        }
        for (zone_id, zone) in &self.zones {
            for (i, cutout) in zone.cutouts.iter().enumerate() {
                look(format!("cutout {} of zone {zone_id}", i + 1), cutout);
            }
        }
        for (i, cutout) in self.outline_cutouts.iter().enumerate() {
            look(format!("cutout {}", i + 1), cutout);
        }
        found
    }

    /// True when [`Board::validate`] finds no error (warnings do not count).
    pub fn is_consistent(&self) -> bool {
        self.is_consistent_with(&BoardLimits::default())
    }

    /// True when [`Board::validate_with`] finds no error with these limits.
    pub fn is_consistent_with(&self, limits: &BoardLimits) -> bool {
        self.validate_with(limits)
            .iter()
            .all(|d| d.severity != Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_documented() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            assert!(seen.insert(*code));
            assert!(
                code.starts_with("PHOENIX-BOARD-") && code.len() == 17,
                "{code}"
            );
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }

    #[test]
    fn the_titles_in_eight_languages_agree_with_the_registry_and_the_document() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, texts) in TITLES {
            let english = REGISTRY
                .iter()
                .find(|(c, _)| c == code)
                .unwrap_or_else(|| panic!("{code} is not in the registry"))
                .1;
            assert_eq!(texts[0], english, "{code}");
            for (language, text) in TITLE_LANGUAGES.iter().zip(texts) {
                assert!(!text.is_empty(), "{code} {language}");
                assert_eq!(title(code, language), Some(*text));
            }
            assert_ne!(texts[5], texts[0], "{code}: Japanese");
            assert_ne!(texts[6], texts[0], "{code}: Chinese");
            assert_ne!(texts[7], texts[1], "{code}: Portuguese is not Spanish");
            let wanted = format!("| {code} | {} |", texts.join(" | "));
            assert!(doc.lines().any(|l| l == wanted), "{code}: {wanted}");
        }
        assert_eq!(
            title(POLYGON_NOT_CHECKED, "pt"),
            Some("O polígono tem mais pontos do que o limite, as suas arestas não foram verificadas quanto a cruzamentos")
        );
        assert_eq!(title("PHOENIX-BOARD-999", "en"), None);
        assert_eq!(title(POLYGON_NOT_CHECKED, "xx"), None);
    }

    #[test]
    fn no_code_of_the_board_crate_is_used_twice() {
        use crate::{connectivity, silk, stack};
        let mut seen = std::collections::BTreeSet::new();
        for (code, _) in REGISTRY
            .iter()
            .chain(connectivity::REGISTRY)
            .chain(silk::REGISTRY)
            .chain(stack::REGISTRY)
        {
            assert!(seen.insert(*code), "{code} is defined twice");
        }
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn only(board: &Board, limits: &BoardLimits, code: &str) -> usize {
        board
            .validate_with(limits)
            .iter()
            .filter(|d| d.code.as_deref() == Some(code))
            .count()
    }

    #[test]
    fn the_shipped_limits_are_the_default() {
        let shipped = include_str!("../../../config/board.cfg");
        assert_eq!(BoardLimits::parse(shipped), Ok(BoardLimits::default()));
    }

    #[test]
    fn the_settings_are_strict() {
        assert_eq!(
            BoardLimits::parse("max_polygon_points = 3")
                .unwrap()
                .max_polygon_points,
            3
        );
        assert_eq!(
            BoardLimits::parse("# only a note\n\n"),
            Ok(BoardLimits::default())
        );
        for (text, line) in [
            ("max_polygon_points = 2", 1),
            ("max_polygon_points = 1000001", 1),
            ("\nmax_polygon_points = x", 2),
            ("max_polygon_points = -4", 1),
            ("max_polygon_points", 1),
            ("\n\nwhat = 3", 3),
        ] {
            let e = BoardLimits::parse(text).unwrap_err();
            assert!(e.starts_with(&format!("line {line}:")), "{text}: {e}");
        }
    }

    #[test]
    fn a_polygon_over_the_limit_is_not_tested_for_crossings_and_gets_a_warning() {
        // a bow tie: the edges (0,0)-(10,10) and (10,0)-(0,10) cross
        let board = Board {
            outline: vec![pt(0, 0), pt(10, 10), pt(10, 0), pt(0, 10)],
            ..Board::default()
        };
        let strict = BoardLimits::default();
        assert_eq!(only(&board, &strict, BAD_OUTLINE), 1);
        assert_eq!(only(&board, &strict, POLYGON_NOT_CHECKED), 0);
        // at the limit (4 points, limit 4) it is still tested
        let at = BoardLimits {
            max_polygon_points: 4,
        };
        assert_eq!(only(&board, &at, BAD_OUTLINE), 1);
        assert_eq!(only(&board, &at, POLYGON_NOT_CHECKED), 0);
        // one point over the limit: no crossing test, a warning
        let low = BoardLimits {
            max_polygon_points: 3,
        };
        assert_eq!(only(&board, &low, BAD_OUTLINE), 0);
        let warned: Vec<_> = board
            .validate_with(&low)
            .into_iter()
            .filter(|d| d.code.as_deref() == Some(POLYGON_NOT_CHECKED))
            .collect();
        assert_eq!(warned.len(), 1);
        assert_eq!(warned[0].severity, Severity::Warning);
        assert!(warned[0].message.contains("4 points"));
        assert!(warned[0].message.contains("max_polygon_points = 3"));
    }

    #[test]
    fn the_polygons_not_cross_checked_are_listed_with_their_place_and_size() {
        let mut board = Board {
            outline: vec![pt(0, 0), pt(10, 0), pt(10, 10), pt(0, 10)],
            outline_cutouts: vec![
                vec![pt(2, 2), pt(3, 2), pt(3, 3)],
                vec![pt(5, 5), pt(6, 5), pt(6, 6), pt(5, 6), pt(5, 7)],
            ],
            ..Board::default()
        };
        let at = |max: usize| BoardLimits {
            max_polygon_points: max,
        };
        assert!(board.polygons_not_cross_checked(&at(5)).is_empty());
        assert_eq!(
            board.polygons_not_cross_checked(&at(4)),
            vec![("cutout 2".to_owned(), 5)]
        );
        assert_eq!(
            board.polygons_not_cross_checked(&at(3)),
            vec![("the board".to_owned(), 4), ("cutout 2".to_owned(), 5)]
        );
        // a repeated point is reported as an error and the polygon is not listed
        board.outline[2] = board.outline[1];
        assert_eq!(
            board.polygons_not_cross_checked(&at(3)),
            vec![("cutout 2".to_owned(), 5)]
        );
    }

    #[test]
    fn a_large_convex_polygon_over_the_default_limit_is_fast_and_warned_once() {
        // 10001 points of the curve y = x * x closed by a chord: convex, no crossings
        let limits = BoardLimits::default();
        let n = i64::try_from(limits.max_polygon_points).unwrap() + 1;
        let mut board = Board {
            outline: (0..n).map(|i| pt(i * 1000, i * i)).collect(),
            ..Board::default()
        };
        assert_eq!(board.outline.len(), 10_001);
        assert_eq!(only(&board, &limits, POLYGON_NOT_CHECKED), 1);
        assert_eq!(only(&board, &limits, BAD_OUTLINE), 0);
        // the test of repeated points still runs over the limit
        board.outline[5] = board.outline[4];
        assert_eq!(only(&board, &limits, BAD_OUTLINE), 1);
        assert_eq!(only(&board, &limits, POLYGON_NOT_CHECKED), 0);
    }
}
