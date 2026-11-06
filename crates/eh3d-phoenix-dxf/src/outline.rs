// SPDX-License-Identifier: GPL-3.0-or-later
//! The outline of a board from the loose entities of a drawing. The lines and arcs are joined end
//! to end (the ends may be a tolerance apart) into chains; a chain that returns to its start is
//! a closed contour, as is a closed polyline or a circle. The contours are put in order by
//! containment: one inside no other is an outer contour, one inside one is a cutout, one inside
//! two is an outer contour again, and so on. The largest outer contour is the outline of the
//! board; its cutouts are reported (the board keeps them as the cutouts of its outline). The report says what
//! was found: chains that are open, duplicates dropped, contours that cross, several outer
//! contours. Nothing is done silently.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::codes::{
    Finding, Report, Severity, ARCS_APPROXIMATED, CHAIN_OPEN, CONTOUR_CROSSES, CONTOUR_DEGENERATE,
    CUTOUT_NOT_KEPT, DUPLICATE_DROPPED, NO_CONTOUR, SEVERAL_OUTER,
};
use crate::geom::{
    bounds, circle_points, distance, locate, properly_cross, tessellate, twice_area, Loc, Seg, P,
};
use crate::model::{Drawing, Shape};
use crate::settings::DxfSettings;

/// What was counted on the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutlineStats {
    /// The pieces and circles taken from the drawing.
    pub pieces: usize,
    /// The chains made of loose pieces.
    pub chains: usize,
    /// The contours found (chains that closed, closed polylines and circles).
    pub contours: usize,
    /// The chains that did not close.
    pub open_chains: usize,
    /// The pieces and contours dropped because they repeat another or have no length.
    pub duplicates: usize,
    /// The arcs and circles replaced by straight pieces.
    pub arcs_approximated: usize,
}

/// The outline found in a drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outline {
    /// The outer contour, counter-clockwise; `None` when the drawing has no closed contour.
    pub exterior: Option<Vec<P>>,
    /// The cutouts inside the outer contour, clockwise, the largest first.
    pub holes: Vec<Vec<P>>,
    /// What the run found.
    pub report: Report,
    /// The counts.
    pub stats: OutlineStats,
}

struct Chain {
    pieces: Vec<Seg>,
    closed: bool,
}

struct Contour {
    points: Vec<P>,
    area2: i128,
    low: P,
    high: P,
}

fn mm(nm: i64) -> String {
    format!("{:.3}", nm as f64 / 1_000_000.0)
}

fn at(p: P) -> String {
    format!("({}, {}) mm", mm(p.0), mm(p.1))
}

fn near(a: P, b: P, tolerance: i64) -> bool {
    let (dx, dy) = (i128::from(a.0 - b.0), i128::from(a.1 - b.1));
    dx * dx + dy * dy <= i128::from(tolerance) * i128::from(tolerance)
}

type Grid = BTreeMap<(i64, i64), Vec<(usize, bool)>>;

fn cell_of(p: P, cell: i64) -> (i64, i64) {
    (p.0.div_euclid(cell), p.1.div_euclid(cell))
}

/// The unused piece that has an end within the tolerance of `p`: the nearest, then the lowest
/// number. The flag is true when the end that matches is the end of the piece.
fn find(
    grid: &Grid,
    segs: &[Seg],
    used: &[bool],
    p: P,
    tolerance: i64,
    cell: i64,
) -> Option<(usize, bool)> {
    let (kx, ky) = cell_of(p, cell);
    let mut best: Option<(i128, usize, bool)> = None;
    for dx in -1..=1 {
        for dy in -1..=1 {
            let Some(list) = grid.get(&(kx + dx, ky + dy)) else {
                continue;
            };
            for &(i, is_end) in list {
                if used[i] {
                    continue;
                }
                let q = if is_end { segs[i].b } else { segs[i].a };
                if !near(p, q, tolerance) {
                    continue;
                }
                let (ex, ey) = (i128::from(p.0 - q.0), i128::from(p.1 - q.1));
                let candidate = (ex * ex + ey * ey, i, is_end);
                if best.is_none_or(|b| candidate < b) {
                    best = Some(candidate);
                }
            }
        }
    }
    best.map(|(_, i, is_end)| (i, is_end))
}

fn chains_of(segs: &[Seg], tolerance: i64) -> Vec<Chain> {
    let cell = tolerance.max(1);
    let mut grid: Grid = BTreeMap::new();
    for (i, s) in segs.iter().enumerate() {
        grid.entry(cell_of(s.a, cell)).or_default().push((i, false));
        grid.entry(cell_of(s.b, cell)).or_default().push((i, true));
    }
    let mut used = vec![false; segs.len()];
    let mut chains = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut pieces: VecDeque<Seg> = VecDeque::new();
        pieces.push_back(segs[start]);
        let mut closed = false;
        loop {
            let end = pieces.back().map_or((0, 0), |s| s.b);
            let first = pieces.front().map_or((0, 0), |s| s.a);
            if pieces.len() >= 2 && near(end, first, tolerance) {
                closed = true;
                break;
            }
            let Some((i, is_end)) = find(&grid, segs, &used, end, tolerance, cell) else {
                break;
            };
            used[i] = true;
            pieces.push_back(if is_end { segs[i].reversed() } else { segs[i] });
        }
        while !closed {
            let first = pieces.front().map_or((0, 0), |s| s.a);
            let Some((i, is_end)) = find(&grid, segs, &used, first, tolerance, cell) else {
                break;
            };
            used[i] = true;
            pieces.push_front(if is_end { segs[i] } else { segs[i].reversed() });
            let end = pieces.back().map_or((0, 0), |s| s.b);
            let first = pieces.front().map_or((0, 0), |s| s.a);
            if pieces.len() >= 2 && near(end, first, tolerance) {
                closed = true;
            }
        }
        chains.push(Chain {
            pieces: pieces.into_iter().collect(),
            closed,
        });
    }
    chains
}

/// The corners of a run of pieces, the arcs cut into straight pieces.
fn polygon_of(pieces: &[Seg], arc_tolerance: i64, arcs: &mut usize) -> Vec<P> {
    let mut points = Vec::with_capacity(pieces.len());
    for s in pieces {
        points.push(s.a);
        if s.sweep_udeg != 0 {
            *arcs += 1;
            let mut cut = tessellate(s, arc_tolerance);
            cut.pop();
            points.extend(cut);
        }
    }
    points
}

/// A polygon without repeated corners, counter-clockwise, or `None` when it has no area.
fn cleaned(mut points: Vec<P>) -> Option<Vec<P>> {
    points.dedup();
    while points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if points.len() < 3 {
        return None;
    }
    let area = twice_area(&points);
    if area == 0 {
        return None;
    }
    if area < 0 {
        points.reverse();
    }
    Some(points)
}

/// The polygon turned so that its least corner comes first: two copies of one contour are equal.
fn canonical(points: &[P]) -> Vec<P> {
    let start = points
        .iter()
        .enumerate()
        .min_by_key(|(_, p)| **p)
        .map_or(0, |(i, _)| i);
    points[start..]
        .iter()
        .chain(points[..start].iter())
        .copied()
        .collect()
}

/// True when the contour `inner` lies inside `outer`: no corner of it is outside and one is
/// strictly inside (a contour that only touches `outer` along its boundary is not inside it).
fn inside(inner: &Contour, outer: &Contour) -> bool {
    if inner.low.0 < outer.low.0
        || inner.low.1 < outer.low.1
        || inner.high.0 > outer.high.0
        || inner.high.1 > outer.high.1
    {
        return false;
    }
    let mut any_inside = false;
    for &corner in &inner.points {
        match locate(corner, &outer.points) {
            Loc::Inside => any_inside = true,
            Loc::Outside => return false,
            Loc::On => {}
        }
    }
    any_inside
}

/// The crossings of the edges of one polygon with each other, or of two polygons.
fn crossings(a: &[P], b: Option<&[P]>) -> Vec<P> {
    let mut found = Vec::new();
    match b {
        None => {
            let n = a.len();
            for i in 0..n {
                for j in (i + 2)..n {
                    if i == 0 && j == n - 1 {
                        continue;
                    }
                    if properly_cross(a[i], a[(i + 1) % n], a[j], a[(j + 1) % n]) {
                        found.push(a[i]);
                    }
                }
            }
        }
        Some(b) => {
            for (i, &p) in a.iter().enumerate() {
                let q = a[(i + 1) % a.len()];
                for (j, &r) in b.iter().enumerate() {
                    if properly_cross(p, q, r, b[(j + 1) % b.len()]) {
                        found.push(p);
                    }
                }
            }
        }
    }
    found
}

/// Finds the outline in a drawing. Only the entities on `layer` count (without regard to case),
/// all of them when `layer` is `None`. Ends of pieces that are no further apart than
/// `join_tolerance_nm` are joined.
pub fn extract_outline(
    drawing: &Drawing,
    layer: Option<&str>,
    join_tolerance_nm: i64,
    settings: &DxfSettings,
) -> Outline {
    let mut report = Report::new(settings.max_findings);
    let mut stats = OutlineStats::default();
    let tolerance = join_tolerance_nm.max(0);

    let mut loops: Vec<Vec<Seg>> = Vec::new();
    let mut loose: Vec<Seg> = Vec::new();
    let mut circles: Vec<(P, i64)> = Vec::new();
    for entity in &drawing.entities {
        if !layer.is_none_or(|l| entity.layer.eq_ignore_ascii_case(l)) {
            continue;
        }
        match &entity.shape {
            Shape::Path { segs, closed: true } => {
                stats.pieces += segs.len();
                loops.push(segs.clone());
            }
            Shape::Path {
                segs,
                closed: false,
            } => {
                stats.pieces += segs.len();
                loose.extend(segs.iter().copied());
            }
            Shape::Circle { centre, radius } if *radius > 0 => {
                stats.pieces += 1;
                circles.push((*centre, *radius));
            }
            _ => {}
        }
    }

    // pieces that repeat another (in either direction) or have no length
    let mut seen: BTreeSet<(P, P, i64)> = BTreeSet::new();
    let mut unique: Vec<Seg> = Vec::new();
    for s in loose {
        if s.a == s.b && s.sweep_udeg == 0 {
            stats.duplicates += 1;
            continue;
        }
        let key = if s.a <= s.b {
            (s.a, s.b, s.sweep_udeg)
        } else {
            (s.b, s.a, -s.sweep_udeg)
        };
        if seen.insert(key) {
            unique.push(s);
        } else {
            stats.duplicates += 1;
        }
    }

    let mut polygons: Vec<Vec<P>> = Vec::new();
    let mut degenerate = 0usize;
    let arc_tolerance = settings.arc_tolerance_nm;
    for pieces in &loops {
        polygons.push(polygon_of(
            pieces,
            arc_tolerance,
            &mut stats.arcs_approximated,
        ));
    }
    for &(centre, radius) in &circles {
        stats.arcs_approximated += 1;
        polygons.push(circle_points(centre, radius, arc_tolerance));
    }
    let chains = chains_of(&unique, tolerance);
    stats.chains = chains.len();
    for (n, chain) in chains.iter().enumerate() {
        if chain.closed {
            polygons.push(polygon_of(
                &chain.pieces,
                arc_tolerance,
                &mut stats.arcs_approximated,
            ));
        } else {
            stats.open_chains += 1;
            let (Some(first), Some(last)) = (chain.pieces.first(), chain.pieces.last()) else {
                continue;
            };
            let subject = format!("chain {}", n + 1);
            report.push(Finding::new(
                CHAIN_OPEN,
                Severity::Warning,
                Some(subject.as_str()),
                format!(
                    "{} pieces from {} to {}, the ends are {} mm apart; it is not part of the outline",
                    chain.pieces.len(),
                    at(first.a),
                    at(last.b),
                    mm(distance(first.a, last.b).round() as i64)
                ),
            ));
        }
    }

    let mut contours: Vec<Contour> = Vec::new();
    let mut known: BTreeSet<Vec<P>> = BTreeSet::new();
    for polygon in polygons {
        let Some(points) = cleaned(polygon) else {
            degenerate += 1;
            continue;
        };
        if !known.insert(canonical(&points)) {
            stats.duplicates += 1;
            continue;
        }
        let (low, high) = bounds(&points);
        contours.push(Contour {
            area2: twice_area(&points),
            points,
            low,
            high,
        });
    }
    stats.contours = contours.len();
    if stats.duplicates > 0 {
        report.push(Finding::new(
            DUPLICATE_DROPPED,
            Severity::Info,
            None,
            format!(
                "{} pieces or contours that repeat another or have no length were dropped",
                stats.duplicates
            ),
        ));
    }
    if degenerate > 0 {
        report.push(Finding::new(
            CONTOUR_DEGENERATE,
            Severity::Warning,
            None,
            format!("{degenerate} closed chains without area were left out"),
        ));
    }
    contours.sort_by_key(|c| (std::cmp::Reverse(c.area2), c.points[0]));

    if contours.is_empty() {
        let subject = layer.map(str::to_owned);
        report.push(Finding::new(
            NO_CONTOUR,
            Severity::Error,
            subject.as_deref(),
            match layer {
                Some(l) => format!("no closed contour on the layer {l}"),
                None => "no closed contour in the drawing".to_owned(),
            },
        ));
        return Outline {
            exterior: None,
            holes: Vec::new(),
            report,
            stats,
        };
    }

    // the depth of each contour: how many larger contours hold it
    let depth: Vec<usize> = (0..contours.len())
        .map(|k| {
            (0..k)
                .filter(|&j| {
                    contours[j].area2 > contours[k].area2 && inside(&contours[k], &contours[j])
                })
                .count()
        })
        .collect();
    let main = (0..contours.len())
        .find(|&k| depth[k].is_multiple_of(2))
        .unwrap_or(0);
    let other_outer: Vec<usize> = (0..contours.len())
        .filter(|&k| k != main && depth[k].is_multiple_of(2))
        .collect();
    let holes: Vec<usize> = (0..contours.len())
        .filter(|&k| !depth[k].is_multiple_of(2) && inside(&contours[k], &contours[main]))
        .collect();
    let ignored_holes = (0..contours.len())
        .filter(|&k| !depth[k].is_multiple_of(2))
        .count()
        - holes.len();
    if !other_outer.is_empty() {
        report.push(Finding::new(
            SEVERAL_OUTER,
            Severity::Warning,
            None,
            format!(
                "{} outer contours besides the largest, which is {} by {} mm, were left out with {} cutouts of theirs",
                other_outer.len(),
                mm(contours[main].high.0 - contours[main].low.0),
                mm(contours[main].high.1 - contours[main].low.1),
                ignored_holes
            ),
        ));
    }
    if !holes.is_empty() {
        let corners: Vec<String> = holes.iter().take(5).map(|&k| at(contours[k].low)).collect();
        report.push(Finding::new(
            CUTOUT_NOT_KEPT,
            Severity::Info,
            None,
            format!(
                "{} cutouts inside the outline are kept as cutouts of the board outline (lower left corners: {}{})",
                holes.len(),
                corners.join(", "),
                if holes.len() > corners.len() { ", ..." } else { "" }
            ),
        ));
    }

    // crossings: inside the outline and among the cutouts
    let kept: Vec<usize> = std::iter::once(main).chain(holes.iter().copied()).collect();
    for (rank, &k) in kept.iter().enumerate() {
        let name = if rank == 0 {
            "outline".to_owned()
        } else {
            format!("cutout {rank}")
        };
        if contours[k].points.len() > settings.max_check_edges {
            report.push(Finding::new(
                CONTOUR_CROSSES,
                Severity::Info,
                Some(name.as_str()),
                format!(
                    "the check for crossings was skipped: {} edges are more than the limit {}",
                    contours[k].points.len(),
                    settings.max_check_edges
                ),
            ));
            continue;
        }
        let own = crossings(&contours[k].points, None);
        if let Some(&first) = own.first() {
            report.push(Finding::new(
                CONTOUR_CROSSES,
                Severity::Error,
                Some(name.as_str()),
                format!(
                    "the contour crosses itself {} times, first at {}",
                    own.len(),
                    at(first)
                ),
            ));
        }
        for &j in &kept[rank + 1..] {
            if contours[j].points.len() > settings.max_check_edges {
                continue;
            }
            let both = crossings(&contours[k].points, Some(&contours[j].points));
            if let Some(&first) = both.first() {
                report.push(Finding::new(
                    CONTOUR_CROSSES,
                    Severity::Error,
                    Some(name.as_str()),
                    format!(
                        "the contour crosses another {} times, first at {}",
                        both.len(),
                        at(first)
                    ),
                ));
            }
        }
    }
    if stats.arcs_approximated > 0 {
        report.push(Finding::new(
            ARCS_APPROXIMATED,
            Severity::Info,
            None,
            format!(
                "{} arcs and circles were replaced by straight pieces within {} nm",
                stats.arcs_approximated, arc_tolerance
            ),
        ));
    }

    let exterior = Some(contours[main].points.clone());
    let holes: Vec<Vec<P>> = holes
        .iter()
        .map(|&k| contours[k].points.iter().rev().copied().collect())
        .collect();
    Outline {
        exterior,
        holes,
        report,
        stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    fn p(x: i64, y: i64) -> P {
        (x * MM, y * MM)
    }

    fn lines(d: &mut Drawing, layer: &str, points: &[P]) {
        for w in points.windows(2) {
            d.line(layer, w[0], w[1]);
        }
    }

    fn run(d: &Drawing, layer: Option<&str>, tolerance: i64) -> Outline {
        extract_outline(d, layer, tolerance, &DxfSettings::default())
    }

    #[test]
    fn four_loose_lines_in_any_order_and_direction_make_one_rectangle() {
        let mut d = Drawing::new();
        d.line("EDGE", p(10, 0), p(10, 6));
        d.line("EDGE", p(0, 0), p(10, 0));
        d.line("EDGE", p(0, 6), p(10, 6));
        d.line("EDGE", p(0, 6), p(0, 0));
        let o = run(&d, Some("edge"), 10_000);
        let exterior = o.exterior.unwrap();
        assert_eq!(exterior.len(), 4);
        assert!(twice_area(&exterior) > 0);
        assert_eq!(twice_area(&exterior), 120 * i128::from(MM) * i128::from(MM));
        assert!(o.holes.is_empty());
        assert!(o.report.findings.is_empty());
        assert_eq!(
            (o.stats.chains, o.stats.contours, o.stats.open_chains),
            (1, 1, 0)
        );
    }

    #[test]
    fn ends_a_little_apart_are_joined_within_the_tolerance_and_not_beyond_it() {
        let mut d = Drawing::new();
        // the last edge ends 5 micrometres from the start of the first
        lines(
            &mut d,
            "A",
            &[p(0, 0), p(10, 0), p(10, 6), p(0, 6), (0, 5_000)],
        );
        let joined = run(&d, None, 10_000);
        assert!(joined.exterior.is_some());
        let apart = run(&d, None, 1_000);
        assert!(apart.exterior.is_none());
        assert_eq!(apart.stats.open_chains, 1);
        let codes: Vec<&str> = apart.report.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![CHAIN_OPEN, NO_CONTOUR]);
        assert!(apart.report.findings[0].detail.contains("4 pieces"));
        assert!(apart.report.has_error());
    }

    #[test]
    fn a_rectangle_with_a_round_corner_and_a_hole_is_found_with_its_cutout() {
        let mut d = Drawing::new();
        // 20 by 10 with the lower right corner rounded by a quarter arc of radius 4
        d.line("E", p(0, 0), p(16, 0));
        d.path(
            "E",
            vec![Seg {
                a: p(16, 0),
                b: p(20, 4),
                sweep_udeg: 90_000_000,
            }],
            false,
        );
        d.line("E", p(20, 4), p(20, 10));
        d.line("E", p(20, 10), p(0, 10));
        d.line("E", p(0, 10), p(0, 0));
        // a round hole and a closed polyline cutout
        d.circle("E", p(5, 5), 2 * MM);
        d.polygon("E", &[p(10, 2), p(14, 2), p(14, 4), p(10, 4)]);
        let o = run(&d, Some("E"), 10_000);
        let exterior = o.exterior.unwrap();
        // the rounding is cut into straight pieces: more corners than the 5 pieces
        assert!(exterior.len() > 8, "{}", exterior.len());
        assert_eq!(o.holes.len(), 2);
        assert!(o.holes.iter().all(|h| twice_area(h) < 0));
        // the largest cutout first: the circle (about 12.57 mm2) before the 2 by 4 rectangle (8 mm2)
        assert!(twice_area(&o.holes[0]).abs() > twice_area(&o.holes[1]).abs());
        let codes: Vec<&str> = o.report.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![CUTOUT_NOT_KEPT, ARCS_APPROXIMATED]);
        assert!(o.report.findings[0].detail.starts_with("2 cutouts"));
        assert_eq!(o.stats.arcs_approximated, 2);
        // the area of the outline: 200 minus the corner (4*4 - pi*16/4 = 3.4336)
        let area = twice_area(&exterior) as f64 / 2.0 / (MM as f64 * MM as f64);
        assert!(
            (area - (200.0 - (16.0 - std::f64::consts::PI * 4.0))).abs() < 0.01,
            "{area}"
        );
    }

    #[test]
    fn one_inside_two_is_an_outer_contour_again_and_the_largest_outer_one_is_the_board() {
        let mut d = Drawing::new();
        d.polygon("A", &[p(0, 0), p(40, 0), p(40, 40), p(0, 40)]);
        d.polygon("A", &[p(5, 5), p(35, 5), p(35, 35), p(5, 35)]);
        d.polygon("A", &[p(10, 10), p(30, 10), p(30, 30), p(10, 30)]);
        // another board beside it
        d.polygon("A", &[p(50, 0), p(60, 0), p(60, 10), p(50, 10)]);
        let o = run(&d, None, 10_000);
        let exterior = o.exterior.unwrap();
        assert_eq!(
            twice_area(&exterior),
            3200 * i128::from(MM) * i128::from(MM)
        );
        // the cutout of the board is the 30 by 30 one; the 20 by 20 inside it is an island
        assert_eq!(o.holes.len(), 1);
        assert_eq!(
            twice_area(&o.holes[0]),
            -1800 * i128::from(MM) * i128::from(MM)
        );
        let codes: Vec<&str> = o.report.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![SEVERAL_OUTER, CUTOUT_NOT_KEPT]);
    }

    #[test]
    fn a_duplicate_piece_and_a_duplicate_contour_are_dropped_with_a_finding() {
        let mut d = Drawing::new();
        d.polygon("A", &[p(0, 0), p(10, 0), p(10, 10), p(0, 10)]);
        // the same square again, walked the other way and starting elsewhere
        d.polygon("A", &[p(10, 10), p(10, 0), p(0, 0), p(0, 10)]);
        d.line("A", p(0, 0), p(0, 0));
        let o = run(&d, None, 0);
        assert_eq!(o.stats.contours, 1);
        assert_eq!(o.stats.duplicates, 2);
        assert!(o.exterior.is_some());
        assert_eq!(o.report.findings[0].code, DUPLICATE_DROPPED);
        assert_eq!(o.report.findings[0].severity, Severity::Info);
    }

    #[test]
    fn two_pieces_that_repeat_one_another_do_not_make_a_chain_twice() {
        let mut d = Drawing::new();
        d.line("A", p(0, 0), p(10, 0));
        d.line("A", p(10, 0), p(0, 0));
        d.line("A", p(10, 0), p(10, 10));
        d.line("A", p(10, 10), p(0, 10));
        d.line("A", p(0, 10), p(0, 0));
        let o = run(&d, None, 0);
        assert_eq!(o.stats.duplicates, 1);
        assert_eq!(o.stats.contours, 1);
        assert_eq!(o.exterior.unwrap().len(), 4);
    }

    #[test]
    fn a_bow_tie_crosses_itself_and_is_an_error() {
        let mut d = Drawing::new();
        d.polygon("A", &[p(0, 0), p(10, 10), p(10, 0), p(0, 5)]);
        let o = run(&d, None, 0);
        assert!(o.exterior.is_some());
        let crossing = o
            .report
            .findings
            .iter()
            .find(|f| f.code == CONTOUR_CROSSES)
            .unwrap();
        assert_eq!(crossing.severity, Severity::Error);
        assert!(
            crossing.detail.contains("crosses itself 1 times"),
            "{}",
            crossing.detail
        );
        assert!(o.report.has_error());
    }

    #[test]
    fn a_cutout_that_crosses_the_outline_is_an_error_too() {
        // a rectangle with a second rectangle that is half out of it
        let mut d = Drawing::new();
        d.polygon("A", &[p(0, 0), p(20, 0), p(20, 20), p(0, 20)]);
        d.polygon("A", &[p(10, 5), p(30, 5), p(30, 8), p(10, 8)]);
        let o = run(&d, None, 0);
        // the second one is not inside the first: it is another outer contour and is left out
        assert!(o.holes.is_empty());
        assert_eq!(o.report.findings[0].code, SEVERAL_OUTER);
        // two cutouts that cross each other
        let mut d = Drawing::new();
        d.polygon("A", &[p(0, 0), p(40, 0), p(40, 40), p(0, 40)]);
        d.polygon("A", &[p(5, 5), p(20, 5), p(20, 20), p(5, 20)]);
        d.polygon("A", &[p(10, 10), p(30, 10), p(30, 30), p(10, 30)]);
        let o = run(&d, None, 0);
        assert_eq!(o.holes.len(), 2);
        assert!(o
            .report
            .findings
            .iter()
            .any(|f| f.code == CONTOUR_CROSSES && f.detail.contains("another 2 times")));
    }

    #[test]
    fn a_closed_chain_without_area_is_left_out_and_a_drawing_without_contours_is_an_error() {
        let mut d = Drawing::new();
        d.polygon("A", &[p(0, 0), p(10, 0), p(5, 0)]);
        let o = run(&d, None, 0);
        assert!(o.exterior.is_none());
        let codes: Vec<&str> = o.report.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![CONTOUR_DEGENERATE, NO_CONTOUR]);
        let o = run(&Drawing::new(), Some("OUTLINE"), 0);
        assert_eq!(o.report.findings[0].code, NO_CONTOUR);
        assert!(o.report.findings[0].detail.contains("layer OUTLINE"));
        // points and texts are not contours
        let mut d = Drawing::new();
        d.point("A", p(1, 1));
        d.text("A", p(1, 1), MM, "x");
        assert!(run(&d, None, 0).exterior.is_none());
    }

    #[test]
    fn only_the_layer_asked_for_counts() {
        let mut d = Drawing::new();
        d.polygon("EDGE", &[p(0, 0), p(10, 0), p(10, 10), p(0, 10)]);
        d.polygon("OTHER", &[p(0, 0), p(50, 0), p(50, 50), p(0, 50)]);
        let o = run(&d, Some("EDGE"), 0);
        assert_eq!(
            twice_area(&o.exterior.unwrap()),
            200 * i128::from(MM) * i128::from(MM)
        );
        let all = run(&d, None, 0);
        assert_eq!(
            twice_area(&all.exterior.unwrap()),
            5000 * i128::from(MM) * i128::from(MM)
        );
    }

    #[test]
    fn a_wide_tolerance_joins_ends_that_a_narrow_one_leaves_apart() {
        let mut d = Drawing::new();
        lines(&mut d, "A", &[p(0, 0), p(10, 0)]);
        lines(&mut d, "A", &[(10 * MM + 300_000, 0), p(10, 10)]);
        lines(&mut d, "A", &[p(10, 10), p(0, 10), p(0, 0)]);
        assert!(run(&d, None, 100_000).exterior.is_none());
        assert!(run(&d, None, 300_000).exterior.is_some());
    }
}
