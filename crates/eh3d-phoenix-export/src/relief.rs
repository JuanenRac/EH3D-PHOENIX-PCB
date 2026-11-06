//! The gaps of a thermal relief around a pad that is not a circle, or with two spokes.
//!
//! The relief of a pad is the ring between its edge and `gap` beyond it, less the spokes that
//! cross it. A spoke of four runs along each axis, so the gaps are one piece in each quadrant; a
//! spoke of two runs left and right, so the gaps are the part of the ring above and the part
//! below. The outline of the pad is sampled in `ARC_STEPS` steps for each quarter turn where it is
//! round, and the ring grows from it by `gap` on every side. Every piece is a closed polygon that
//! the export writes as a region of clear polarity.

use eh3d_phoenix_model::Point;

type Xy = (f64, f64);

/// The steps for each quarter turn of a round edge.
const ARC_STEPS: usize = 8;

/// The shape of the pad that gets a relief, at a right angle already (the sizes along x and y).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliefShape {
    /// A circle of the diameter.
    Circle(i64),
    /// A rectangle of the width and the height.
    Rect(i64, i64),
    /// A rectangle with round ends, of the width and the height.
    Obround(i64, i64),
}

/// Drops a point equal to the one before it.
fn dedup(points: &mut Vec<Xy>) {
    points.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    if points.len() > 1 {
        let (first, last) = (points[0], points[points.len() - 1]);
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
            points.pop();
        }
    }
}

/// The convex outline of the pad grown by `grow` on every side, about the origin.
fn outline(shape: ReliefShape, grow: f64) -> Vec<Xy> {
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut points: Vec<Xy> = Vec::new();
    match shape {
        ReliefShape::Circle(d) => {
            let r = d as f64 / 2.0 + grow;
            let total = 4 * ARC_STEPS;
            for k in 0..total {
                let a = quarter * k as f64 / ARC_STEPS as f64;
                points.push((r * a.cos(), r * a.sin()));
            }
        }
        ReliefShape::Rect(w, h) => {
            let (hx, hy) = (w as f64 / 2.0 + grow, h as f64 / 2.0 + grow);
            points.extend([(hx, -hy), (hx, hy), (-hx, hy), (-hx, -hy)]);
        }
        ReliefShape::Obround(w, h) => {
            let tall = h > w;
            let (long, short): (f64, f64) = if tall {
                (h as f64, w as f64)
            } else {
                (w as f64, h as f64)
            };
            let r = short / 2.0 + grow;
            let c = (long - short) / 2.0;
            let steps = 2 * ARC_STEPS;
            // the cap on the right, from below to above, then the cap on the left
            for (centre, start) in [(c, -quarter), (-c, quarter)] {
                for k in 0..=steps {
                    let a = start + 2.0 * quarter * k as f64 / steps as f64;
                    points.push((centre + r * a.cos(), r * a.sin()));
                }
            }
            if tall {
                points = points.into_iter().map(|(x, y)| (y, x)).collect();
            }
        }
    }
    dedup(&mut points);
    points
}

/// How far `p` is inside the half plane `c . p >= limit` (negative when it is outside).
fn value(c: &Xy, p: &Xy, limit: f64) -> f64 {
    c.0 * p.0 + c.1 * p.1 - limit
}

/// The run of the outline that lies where every constraint `n . p >= limit` holds, from the point
/// where it enters to the point where it leaves. Empty when no vertex is outside.
fn run(poly: &[Xy], constraints: &[Xy], limit: f64) -> Vec<Xy> {
    let inside = |p: &Xy| constraints.iter().all(|c| value(c, p, limit) >= 0.0);
    let n = poly.len();
    let Some(start) = (0..n).find(|&i| !inside(&poly[i])) else {
        return Vec::new();
    };
    let mut out: Vec<Xy> = Vec::new();
    for step in 0..n {
        let p = poly[(start + step) % n];
        let q = poly[(start + step + 1) % n];
        let (ip, iq) = (inside(&p), inside(&q));
        let at = |t: f64| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
        if !ip && iq {
            let t = constraints
                .iter()
                .filter(|c| value(c, &p, limit) < 0.0)
                .map(|c| value(c, &p, limit) / (value(c, &p, limit) - value(c, &q, limit)))
                .fold(0.0_f64, f64::max);
            out.push(at(t));
            out.push(q);
        } else if ip && iq {
            out.push(q);
        } else if ip && !iq {
            let t = constraints
                .iter()
                .filter(|c| value(c, &q, limit) < 0.0)
                .map(|c| value(c, &p, limit) / (value(c, &p, limit) - value(c, &q, limit)))
                .fold(1.0_f64, f64::min);
            out.push(at(t));
        }
    }
    out
}

/// The gaps of the relief of a pad of `shape` centred at `centre`, as closed polygons: four for
/// `spokes` 4 (one in each quadrant), two for `spokes` 2 (above and below the spokes, which run
/// along the x axis). Nothing for another number of spokes, a gap or a spoke that is not positive,
/// or a spoke as wide as the narrower side of the pad.
pub fn relief_gaps(
    shape: ReliefShape,
    centre: Point,
    gap: i64,
    spoke: i64,
    spokes: u32,
) -> Vec<Vec<Point>> {
    let narrow = match shape {
        ReliefShape::Circle(d) => d,
        ReliefShape::Rect(w, h) | ReliefShape::Obround(w, h) => w.min(h),
    };
    if gap <= 0 || spoke <= 0 || spoke >= narrow {
        return Vec::new();
    }
    let half = spoke as f64 / 2.0;
    let regions: Vec<Vec<Xy>> = match spokes {
        4 => vec![
            vec![(1.0, 0.0), (0.0, 1.0)],
            vec![(-1.0, 0.0), (0.0, 1.0)],
            vec![(-1.0, 0.0), (0.0, -1.0)],
            vec![(1.0, 0.0), (0.0, -1.0)],
        ],
        2 => vec![vec![(0.0, 1.0)], vec![(0.0, -1.0)]],
        _ => return Vec::new(),
    };
    let inner = outline(shape, 0.0);
    let outer = outline(shape, gap as f64);
    let (cx, cy) = (centre.x_nm.get(), centre.y_nm.get());
    let mut out: Vec<Vec<Point>> = Vec::new();
    for constraints in &regions {
        let near = run(&inner, constraints, half);
        let far = run(&outer, constraints, half);
        if near.len() < 2 || far.len() < 2 {
            continue;
        }
        let ring: Option<Vec<Point>> = near
            .iter()
            .chain(far.iter().rev())
            .map(|&(x, y)| Point::from_nm(cx + x.round() as i64, cy + y.round() as i64).ok())
            .collect();
        if let Some(ring) = ring {
            out.push(ring);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// Twice the signed area of a polygon, in square nanometres.
    fn area(ring: &[Point]) -> f64 {
        let n = ring.len();
        let mut sum: f64 = 0.0;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            sum += a.x_nm.get() as f64 * b.y_nm.get() as f64
                - b.x_nm.get() as f64 * a.y_nm.get() as f64;
        }
        sum.abs() / 2.0
    }

    const AT: (i64, i64) = (10_000_000, 5_000_000);

    #[test]
    fn a_rectangle_with_four_spokes_has_an_l_shaped_gap_in_each_quadrant() {
        // a pad of 1 by 0.6 mm, a gap of 0.25 mm and spokes of 0.25 mm: a spoke covers 0.125 mm
        // on each side of its axis. In the upper right quadrant the gap is the L between the
        // corner (0.5, 0.3) and the corner (0.75, 0.55), cut at x = 0.125 and y = 0.125
        let gaps = relief_gaps(
            ReliefShape::Rect(1_000_000, 600_000),
            pt(AT.0, AT.1),
            250_000,
            250_000,
            4,
        );
        assert_eq!(gaps.len(), 4);
        for ring in &gaps {
            assert_eq!(ring.len(), 6);
            // (0.75 - 0.125) * (0.55 - 0.125) - (0.5 - 0.125) * (0.3 - 0.125) = 0.265625 - 0.065625
            assert!(
                (area(ring) - 200_000_000_000.0).abs() < 1.0,
                "{}",
                area(ring)
            );
        }
        let mut first: Vec<(i64, i64)> = gaps[0]
            .iter()
            .map(|p| (p.x_nm.get() - AT.0, p.y_nm.get() - AT.1))
            .collect();
        first.sort_unstable();
        assert_eq!(
            first,
            vec![
                (125_000, 300_000),
                (125_000, 550_000),
                (500_000, 125_000),
                (500_000, 300_000),
                (750_000, 125_000),
                (750_000, 550_000),
            ]
        );
    }

    #[test]
    fn a_rectangle_with_two_spokes_has_a_gap_above_and_a_gap_below() {
        let gaps = relief_gaps(
            ReliefShape::Rect(1_000_000, 600_000),
            pt(AT.0, AT.1),
            250_000,
            250_000,
            2,
        );
        assert_eq!(gaps.len(), 2);
        for ring in &gaps {
            assert_eq!(ring.len(), 8);
            // 1.5 * 0.425 - 1.0 * 0.175
            assert!(
                (area(ring) - 462_500_000_000.0).abs() < 1.0,
                "{}",
                area(ring)
            );
            // no corner is inside a spoke
            assert!(ring.iter().all(|p| (p.y_nm.get() - AT.1).abs() >= 125_000));
        }
        // one piece is above the pad and the other below
        let ys: Vec<bool> = gaps.iter().map(|g| g[0].y_nm.get() > AT.1).collect();
        assert_eq!(ys.iter().filter(|up| **up).count(), 1);
    }

    #[test]
    fn an_oblong_pad_gets_a_gap_ring_that_follows_its_round_ends() {
        // 1 by 0.5 mm: the ends are half circles of 0.25 mm about (+-0.25, 0); the gap ring ends
        // 0.25 mm beyond them. Two spokes of 0.1 mm leave a gap above and a gap below
        let gaps = relief_gaps(
            ReliefShape::Obround(1_000_000, 500_000),
            pt(AT.0, AT.1),
            250_000,
            100_000,
            2,
        );
        assert_eq!(gaps.len(), 2);
        for ring in &gaps {
            for p in ring {
                let x = (p.x_nm.get() - AT.0) as f64;
                let y = (p.y_nm.get() - AT.1) as f64;
                assert!(y.abs() >= 49_999.0);
                // the distance to the segment (-0.25, 0)..(0.25, 0) is 0.25 mm on the pad and 0.5 mm
                // on the far side of the gap
                let d = (x.abs() - 250_000.0).max(0.0).hypot(y);
                assert!(
                    (d - 250_000.0).abs() < 2_500.0 || (d - 500_000.0).abs() < 2_500.0,
                    "{d}"
                );
            }
        }
        // a tall pad is the same pad turned: the pieces are still above and below, and reach farther
        let tall = relief_gaps(
            ReliefShape::Obround(500_000, 1_000_000),
            pt(AT.0, AT.1),
            250_000,
            100_000,
            2,
        );
        assert_eq!(tall.len(), 2);
        let widest = tall
            .iter()
            .flatten()
            .map(|p| (p.y_nm.get() - AT.1).abs())
            .max();
        assert_eq!(widest, Some(750_000));
    }

    #[test]
    fn a_circle_with_two_spokes_has_two_arches() {
        let gaps = relief_gaps(
            ReliefShape::Circle(1_000_000),
            pt(0, 0),
            250_000,
            250_000,
            2,
        );
        assert_eq!(gaps.len(), 2);
        for ring in &gaps {
            for p in ring {
                let r = (p.x_nm.get() as f64).hypot(p.y_nm.get() as f64);
                assert!((497_000.0..=750_001.0).contains(&r), "{r}");
                assert!(p.y_nm.get().abs() >= 124_999);
            }
        }
    }

    #[test]
    fn a_relief_that_cannot_be_made_is_nothing() {
        let rect = ReliefShape::Rect(1_000_000, 600_000);
        let at = pt(0, 0);
        assert!(relief_gaps(rect, at, 0, 250_000, 4).is_empty());
        assert!(relief_gaps(rect, at, 250_000, 0, 4).is_empty());
        // a spoke as wide as the shorter side
        assert!(relief_gaps(rect, at, 250_000, 600_000, 4).is_empty());
        // only two and four spokes are made
        assert!(relief_gaps(rect, at, 250_000, 250_000, 3).is_empty());
        assert!(relief_gaps(rect, at, 250_000, 250_000, 0).is_empty());
    }
}
