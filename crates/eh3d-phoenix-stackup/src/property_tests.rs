// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the closed forms and of the spread of the impedance with a fixed seed: the
//! trends that any correct model has (a wider track has a lower impedance, a taller dielectric a
//! higher one, a higher permittivity a lower one), checked on a few hundred geometries that are
//! the same on every run.

use crate::models::{
    differential_microstrip, differential_stripline, microstrip, stripline_offset,
    stripline_symmetric,
};
use crate::spread::{
    impedance_spread, max_width_tolerance, SpreadGeometry, SpreadKind, SpreadTolerances,
    MAX_TOLERANCE_FRACTION,
};

const MM: f64 = 1.0e-3;

/// A generator with a fixed seed, so every run makes the same geometries.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// A number in `[low, high)`.
    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * (self.next() % 1_000_000) as f64 / 1_000_000.0
    }
}

#[test]
fn a_wider_microstrip_has_a_lower_impedance() {
    let mut rng = Lcg(0x5a_0001);
    for _ in 0..300 {
        let h = rng.between(0.05, 1.0) * MM;
        let t = rng.between(0.0, 0.1) * h;
        let er = rng.between(2.0, 10.0);
        let w = rng.between(0.1, 3.0) * h;
        let (narrow, _) = microstrip(w, t, h, er).unwrap();
        let (wide, _) = microstrip(w * 1.1, t, h, er).unwrap();
        assert!(wide < narrow, "{w} {t} {h} {er}: {wide} {narrow}");
    }
}

#[test]
fn a_taller_microstrip_substrate_has_a_higher_impedance() {
    let mut rng = Lcg(0x5a_0002);
    for _ in 0..300 {
        let h = rng.between(0.05, 1.0) * MM;
        let w = rng.between(0.2, 3.0) * h;
        let t = rng.between(0.0, 0.05) * h;
        let er = rng.between(2.0, 10.0);
        let (low, _) = microstrip(w, t, h, er).unwrap();
        let (high, _) = microstrip(w, t, h * 1.1, er).unwrap();
        assert!(high > low, "{w} {t} {h} {er}: {high} {low}");
    }
}

#[test]
fn a_higher_permittivity_gives_a_lower_microstrip_impedance() {
    let mut rng = Lcg(0x5a_0003);
    for _ in 0..300 {
        let h = rng.between(0.05, 1.0) * MM;
        let w = rng.between(0.2, 3.0) * h;
        let er = rng.between(2.0, 9.0);
        let (a, ea) = microstrip(w, 0.0, h, er).unwrap();
        let (b, eb) = microstrip(w, 0.0, h, er + 1.0).unwrap();
        assert!(b < a, "{w} {h} {er}: {b} {a}");
        assert!(eb > ea, "{w} {h} {er}: {eb} {ea}");
    }
}

#[test]
fn a_stripline_follows_the_same_trends() {
    let mut rng = Lcg(0x5a_0004);
    for _ in 0..300 {
        let b = rng.between(0.1, 1.0) * MM;
        let t = rng.between(0.0, 0.1) * b;
        let er = rng.between(2.0, 10.0);
        let w = rng.between(0.1, 1.5) * b;
        let z = stripline_symmetric(w, t, b, er).unwrap();
        assert!(
            stripline_symmetric(w * 1.1, t, b, er).unwrap() < z,
            "{w} {b}"
        );
        assert!(
            stripline_symmetric(w, t, b * 1.1, er).unwrap() > z,
            "{w} {b}"
        );
        assert!(
            stripline_symmetric(w, t, b, er * 1.1).unwrap() < z,
            "{w} {b}"
        );
    }
}

#[test]
fn an_offset_stripline_with_equal_sides_is_the_centred_one() {
    let mut rng = Lcg(0x5a_0005);
    for _ in 0..200 {
        let h = rng.between(0.05, 0.5) * MM;
        let t = rng.between(0.0, 0.2) * h;
        let er = rng.between(2.0, 10.0);
        let w = rng.between(0.1, 2.0) * h;
        let (z, eps, _) = stripline_offset(w, t, [(h, er, 0.01), (h, er, 0.01)]).unwrap();
        let centred = stripline_symmetric(w, t, 2.0 * h + t, er).unwrap();
        assert!((z - centred).abs() <= centred * 1.0e-9, "{z} {centred}");
        assert!((eps - er).abs() <= er * 1.0e-9, "{eps} {er}");
    }
}

#[test]
fn a_pair_is_between_the_single_line_and_twice_it_and_grows_with_the_spacing() {
    let mut rng = Lcg(0x5a_0006);
    for _ in 0..300 {
        let z = rng.between(30.0, 80.0);
        let h = rng.between(0.05, 1.0) * MM;
        let s = rng.between(0.1, 3.0) * h;
        for pair in [differential_microstrip, differential_stripline] {
            let near = pair(z, s, h);
            let far = pair(z, s * 1.5, h);
            assert!(near > z && near < 2.0 * z, "{z} {s} {h}: {near}");
            assert!(far > near, "{z} {s} {h}: {far} {near}");
        }
    }
}

#[test]
fn every_point_inside_the_box_of_tolerances_is_inside_the_spread() {
    let mut rng = Lcg(0x5a_0007);
    for round in 0..200 {
        let kind = if round % 2 == 0 {
            SpreadKind::Microstrip
        } else {
            SpreadKind::Stripline
        };
        let height = rng.between(0.1, 0.8) * MM;
        let g = SpreadGeometry {
            kind,
            width_m: rng.between(0.2, 1.5) * height,
            thickness_m: rng.between(0.0, 0.1) * height,
            height_m: height,
            er: rng.between(2.5, 9.0),
        };
        let t = SpreadTolerances {
            width_m: rng.between(0.0, 0.2) * g.width_m,
            height_m: rng.between(0.0, 0.1) * height,
            er: rng.between(0.0, 0.3),
        };
        let s = impedance_spread(&g, &t).unwrap();
        assert!(s.min_ohm <= s.nominal_ohm && s.nominal_ohm <= s.max_ohm);
        for _ in 0..8 {
            let w = g.width_m + rng.between(-1.0, 1.0) * t.width_m;
            let h = g.height_m + rng.between(-1.0, 1.0) * t.height_m;
            let e = g.er + rng.between(-1.0, 1.0) * t.er;
            let z = match kind {
                SpreadKind::Microstrip => microstrip(w, g.thickness_m, h, e).unwrap().0,
                SpreadKind::Stripline => stripline_symmetric(w, g.thickness_m, h, e).unwrap(),
            };
            let slack = s.nominal_ohm * 1.0e-9;
            assert!(
                z >= s.min_ohm - slack && z <= s.max_ohm + slack,
                "{g:?} {t:?}: {z} not in {} .. {}",
                s.min_ohm,
                s.max_ohm
            );
        }
    }
}

#[test]
fn the_same_geometry_gives_the_same_spread_every_time() {
    let g = SpreadGeometry {
        kind: SpreadKind::Microstrip,
        width_m: 0.25 * MM,
        thickness_m: 35.0e-6,
        height_m: 0.15 * MM,
        er: 4.4,
    };
    let t = SpreadTolerances {
        width_m: 0.02 * MM,
        height_m: 0.01 * MM,
        er: 0.2,
    };
    assert_eq!(impedance_spread(&g, &t), impedance_spread(&g, &t));
}

#[test]
fn the_largest_width_tolerance_fits_and_a_little_more_does_not() {
    let mut rng = Lcg(0x5a_0008);
    let mut limited = 0;
    for round in 0..200 {
        let kind = if round % 2 == 0 {
            SpreadKind::Microstrip
        } else {
            SpreadKind::Stripline
        };
        let height = rng.between(0.1, 0.8) * MM;
        let g = SpreadGeometry {
            kind,
            width_m: rng.between(0.2, 1.5) * height,
            thickness_m: rng.between(0.0, 0.1) * height,
            height_m: height,
            er: rng.between(2.5, 9.0),
        };
        let others = SpreadTolerances {
            width_m: 0.0,
            height_m: rng.between(0.0, 0.03) * height,
            er: rng.between(0.0, 0.1),
        };
        let nominal = impedance_spread(&g, &others).unwrap().nominal_ohm;
        let percent = rng.between(3.0, 20.0);
        let at = |w: f64| {
            impedance_spread(
                &g,
                &SpreadTolerances {
                    width_m: w,
                    ..others
                },
            )
            .unwrap()
        };
        match max_width_tolerance(&g, &others, nominal, percent).unwrap() {
            None => assert!(!at(0.0).fits(nominal, percent)),
            Some(found) => {
                assert!(at(found).fits(nominal, percent), "{g:?} {others:?}");
                let limit = g.width_m * MAX_TOLERANCE_FRACTION;
                if found < limit {
                    limited += 1;
                    assert!(!at((found * 1.001).min(limit)).fits(nominal, percent));
                }
            }
        }
    }
    assert!(limited > 20, "{limited}");
}
