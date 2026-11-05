// SPDX-License-Identifier: GPL-3.0-or-later
//! Textbook cases and properties that cross the modules of the RF crate: the L network from 50
//! to 100 ohm, the third-order Butterworth filter at 100 MHz, the quarter-wave line with
//! `z = sqrt(z1 z2)`, the conversions between S, Z, Y and ABCD parameters, reciprocity and
//! passivity, and a filter that is written as a Touchstone file and read back.
//!
//! The component values and the S parameters written here are inputs of the cases, not data of a
//! part.

use eh3d_phoenix_rf::filter::{design, prototype, Approximation, Band, Position};
use eh3d_phoenix_rf::line::Line;
use eh3d_phoenix_rf::matching::{
    l_match, quarter_wave, quarter_wave_impedance, stub_match, ElementKind, LKind, StubKind,
    Termination,
};
use eh3d_phoenix_rf::network::{
    abcd_series, abcd_shunt, abcd_to_s, abcd_to_y, abcd_to_z, cascade, is_passive, is_reciprocal,
    s_to_abcd, s_to_y, s_to_z, y_to_abcd, y_to_s, z_to_abcd, z_to_s, Mat2,
};
use eh3d_phoenix_rf::smith::SmithPoint;
use eh3d_phoenix_rf::touchstone::{Format, FreqUnit, Parameter, Point, Touchstone};
use eh3d_phoenix_rf::{Complex, RfError, RfSettings};
use std::f64::consts::PI;

fn settings() -> RfSettings {
    RfSettings::default()
}

#[test]
fn the_l_network_from_50_to_100_ohm_at_100_mhz() {
    let found = l_match(
        Complex::real(50.0),
        Complex::real(100.0),
        100e6,
        &settings(),
    )
    .unwrap();
    assert_eq!(found.len(), 2);
    let low = found.iter().find(|m| m.kind == LKind::LowPass).unwrap();
    // Q = sqrt(100 / 50 - 1) = 1, X = Q * 50 = 50 ohm, B = Q / 100 = 10 mS
    let omega = 2.0 * PI * 100e6;
    assert_eq!(low.series.kind, ElementKind::Inductor);
    assert_eq!(low.shunt.kind, ElementKind::Capacitor);
    assert!((low.series.value - 50.0 / omega).abs() / low.series.value < 1e-9);
    assert!((low.shunt.value - 0.01 / omega).abs() / low.shunt.value < 1e-9);
    // seen from the source, the network ending on 100 ohm is 50 ohm
    let zin = low.input_impedance(Complex::real(100.0)).unwrap();
    assert!((zin - Complex::real(50.0)).abs() < 1e-9);
}

#[test]
fn the_third_order_butterworth_filter_at_100_mhz() {
    let p = prototype(
        Approximation::Butterworth,
        3,
        None,
        Position::Shunt,
        &settings(),
    )
    .unwrap();
    for (g, want) in p.g.iter().zip([1.0, 2.0, 1.0]) {
        assert!((g - want).abs() < 1e-12);
    }
    let f = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 50.0, &settings()).unwrap();
    let omega = 2.0 * PI * 100e6;
    let c = 1.0 / (50.0 * omega);
    let l = 2.0 * 50.0 / omega;
    let at = |freq: f64| f.response_at(freq).unwrap();
    // -3.01 dB at the cutoff
    assert!((at(100e6).insertion_loss_db() - 10.0 * 2f64.log10()).abs() < 1e-9);
    // the same answer from the textbook formula 1 / (1 + (f / fc)^6)
    for freq in [10e6, 50e6, 80e6, 150e6, 300e6] {
        let want = 1.0 / (1.0 + (freq / 100e6).powi(6));
        assert!((at(freq).s21.norm_sqr() - want).abs() < 1e-9, "{freq}");
    }
    // and from the elements typed by hand
    let t = {
        let jw = |x: f64| Complex::new(0.0, x);
        let w = 2.0 * PI * 70e6;
        let shunt = abcd_shunt(jw(w * c));
        let series = abcd_series(jw(w * l));
        cascade(&cascade(&shunt, &series), &shunt)
    };
    let s = abcd_to_s(&t, 50.0).unwrap();
    assert!((s.m21 - at(70e6).s21).abs() < 1e-9);
    assert!((s.m11 - at(70e6).s11).abs() < 1e-9);
}

#[test]
fn the_quarter_wave_line_has_the_geometric_mean_impedance() {
    let z = quarter_wave_impedance(50.0, 200.0, &settings()).unwrap();
    assert!((z - 100.0).abs() < 1e-12);
    let found = quarter_wave(Complex::real(200.0), 50.0, &settings()).unwrap();
    assert!((found[0].section_ohm - (50.0f64 * 200.0).sqrt()).abs() < 1e-12);
    // a quarter wave of 100 ohm line turns 200 ohm into 50 ohm
    let line = Line::new(100.0, 1.0).unwrap();
    let length = line.length_m(90.0, 1e9).unwrap();
    let zin = eh3d_phoenix_rf::network::input_impedance(
        &line.abcd(length, 1e9).unwrap(),
        Complex::real(200.0),
    )
    .unwrap();
    assert!((zin - Complex::real(50.0)).abs() < 1e-9);
}

#[test]
fn a_matched_line_has_the_book_stub_and_quarter_wave_solutions() {
    // 100 ohm load on 50 ohm: shunt stub at tan(theta) = sqrt(2)
    let stubs = stub_match(
        Complex::real(100.0),
        50.0,
        StubKind::Shunt,
        Termination::Open,
        &settings(),
    )
    .unwrap();
    assert!((stubs[0].distance_deg - 2f64.sqrt().atan().to_degrees()).abs() < 1e-9);
    for m in &stubs {
        let zin = m.input_impedance(Complex::real(100.0), 50.0).unwrap();
        assert!((zin - Complex::real(50.0)).abs() < 1e-6);
    }
}

/// A small deterministic generator for the property tests.
struct Lcg(u64);

impl Lcg {
    fn uniform(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// A passive, reciprocal two-port made of random series and shunt elements and lines.
fn random_network(rng: &mut Lcg) -> Mat2 {
    // a line first, so that the network has both a Z and a Y matrix
    let mut total = eh3d_phoenix_rf::network::abcd_lossless_line(
        30.0 + 70.0 * rng.uniform(),
        0.3 + 2.5 * rng.uniform(),
    )
    .unwrap();
    for _ in 0..3 {
        let element = match (rng.uniform() * 3.0) as u32 {
            0 => abcd_series(Complex::new(
                5.0 + 90.0 * rng.uniform(),
                60.0 * rng.uniform() - 30.0,
            )),
            1 => abcd_shunt(Complex::new(
                0.001 + 0.02 * rng.uniform(),
                0.04 * rng.uniform() - 0.02,
            )),
            _ => eh3d_phoenix_rf::network::abcd_lossless_line(
                30.0 + 70.0 * rng.uniform(),
                PI * rng.uniform(),
            )
            .unwrap(),
        };
        total = cascade(&total, &element);
    }
    total
}

#[test]
fn every_conversion_round_trips_on_random_passive_networks() {
    let mut rng = Lcg(4096);
    for _ in 0..300 {
        let abcd = random_network(&mut rng);
        let z0 = 25.0 + 100.0 * rng.uniform();
        let s = abcd_to_s(&abcd, z0).unwrap();
        assert!(is_passive(&s, 1e-9), "{s:?}");
        assert!(is_reciprocal(&s, 1e-9), "{s:?}");
        assert!((abcd.det() - Complex::ONE).abs() < 1e-9);
        let back = s_to_abcd(&s, z0).unwrap();
        let scale = 1.0 + abcd.m12.abs() + abcd.m21.abs();
        assert!(back.max_difference(&abcd) < 1e-8 * scale);
        let z = s_to_z(&s, z0).unwrap();
        let y = s_to_y(&s, z0).unwrap();
        let tol = |m: &Mat2| 1e-6 * (1.0 + m.m11.abs() + m.m22.abs());
        assert!(z_to_s(&z, z0).unwrap().max_difference(&s) < 1e-6);
        assert!(y_to_s(&y, z0).unwrap().max_difference(&s) < 1e-6);
        assert!(z_to_abcd(&z).unwrap().max_difference(&abcd) < 1e-6 * scale);
        assert!(y_to_abcd(&y).unwrap().max_difference(&abcd) < 1e-6 * scale);
        assert!(abcd_to_z(&abcd).unwrap().max_difference(&z) < tol(&z));
        assert!(abcd_to_y(&abcd).unwrap().max_difference(&y) < tol(&y));
    }
}

#[test]
fn the_cascade_is_associative_and_has_an_identity() {
    let mut rng = Lcg(5);
    for _ in 0..50 {
        let (a, b, c) = (
            random_network(&mut rng),
            random_network(&mut rng),
            random_network(&mut rng),
        );
        let left = cascade(&cascade(&a, &b), &c);
        let right = cascade(&a, &cascade(&b, &c));
        assert!(left.max_difference(&right) < 1e-9 * (1.0 + left.m12.abs() + left.m21.abs()));
        assert!(cascade(&a, &Mat2::identity()).max_difference(&a) < 1e-15);
    }
}

#[test]
fn a_filter_written_as_a_touchstone_file_reads_back_the_same() {
    let p = prototype(
        Approximation::Chebyshev,
        4,
        Some(0.5),
        Position::Shunt,
        &settings(),
    )
    .unwrap();
    let f = design(&p, Band::LowPass { cutoff_hz: 200e6 }, 50.0, &settings()).unwrap();
    let sweep = f.sweep(10e6, 600e6, 25, false, &settings()).unwrap();
    // the load of an even-order filter is not 50 ohm: write the S parameters against 50 ohm
    // from the ABCD matrix instead
    let mut file = Touchstone {
        ports: 2,
        parameter: Parameter::S,
        format: Format::Ma,
        unit: FreqUnit::MHz,
        z_ref_ohm: 50.0,
        points: Vec::new(),
        noise: Vec::new(),
        comments: vec!["ideal filter".to_string()],
    };
    let mut expected = Vec::new();
    for point in &sweep {
        let s = abcd_to_s(&f.abcd_at(point.freq_hz).unwrap(), 50.0).unwrap();
        expected.push(s);
        file.points.push(Point {
            freq_hz: point.freq_hz,
            data: vec![s.m11, s.m12, s.m21, s.m22],
        });
    }
    let text = file.to_text().unwrap();
    let back = Touchstone::parse(&text, 2, &settings()).unwrap();
    assert_eq!(back.points.len(), 25);
    for (index, s) in expected.iter().enumerate() {
        let read = Mat2::from_cmatrix(&back.s_matrix(index).unwrap()).unwrap();
        assert!(read.max_difference(s) < 1e-9, "{index}");
        assert!(is_passive(&read, 1e-6) && is_reciprocal(&read, 1e-9));
    }
    // between two points the file is interpolated linearly
    let mid = 0.5 * (back.points[3].freq_hz + back.points[4].freq_hz);
    let blended = back.s_matrix_at(mid).unwrap();
    let average = (expected[3].m21 + expected[4].m21).scale(0.5);
    assert!((blended.get(1, 0) - average).abs() < 1e-9);
}

#[test]
fn a_matched_load_sits_at_the_centre_of_the_smith_chart() {
    let found = l_match(
        Complex::new(50.0, 0.0),
        Complex::new(20.0, -35.0),
        433e6,
        &settings(),
    )
    .unwrap();
    for m in found {
        let zin = m.input_impedance(Complex::new(20.0, -35.0)).unwrap();
        // the conjugate of a real source is the source: the point is the centre of the chart
        let point = SmithPoint::from_impedance(zin, 50.0, None).unwrap();
        assert!(point.gamma.abs() < 1e-9);
        assert!((point.z.unwrap() - Complex::ONE).abs() < 1e-9);
    }
}

#[test]
fn errors_name_the_line_and_carry_a_code() {
    let error = Touchstone::parse("# GHz S RI\n1 0.5 0\n2 0.4\n", 1, &settings()).unwrap_err();
    assert_eq!(error.code(), "PHOENIX-RF-003");
    assert!(matches!(error, RfError::TouchstoneData { line: 3, .. }));
    let error = Touchstone::parse("# GHz S RI\n1 0.5 x\n", 1, &settings()).unwrap_err();
    assert_eq!(error.code(), "PHOENIX-RF-002");
    assert!(error.to_string().contains("line 2"));
}

#[test]
fn nothing_panics_on_hostile_input() {
    let hostile = [
        "",
        "#",
        "# # #",
        "!",
        "1",
        "1 2",
        "# GHz S RI R 0\n1 0 0\n",
        "# GHz S RI\n1e999 0 0\n",
        "# GHz S RI\n1 1e999 0\n",
        "# GHz S DB\n1 1e999 0\n",
        "[Version] 2.0\n",
        "# GHz S RI\n\u{0} \u{1}\n",
        "# 1e308 Hz\n",
    ];
    for text in hostile {
        for ports in [1, 2, 3, 16] {
            let _ = Touchstone::parse(text, ports, &settings());
        }
    }
    let mut rng = Lcg(77);
    for _ in 0..200 {
        let mut text = String::from("# GHz S RI\n");
        for _ in 0..(rng.uniform() * 12.0) as usize {
            let token = match (rng.uniform() * 6.0) as u32 {
                0 => "1e308".to_string(),
                1 => "-0".to_string(),
                2 => "x".to_string(),
                3 => "\n".to_string(),
                4 => format!("{}", rng.uniform()),
                _ => "!".to_string(),
            };
            text.push_str(&token);
            text.push(' ');
        }
        for ports in [1, 2, 3] {
            if let Ok(file) = Touchstone::parse(&text, ports, &settings()) {
                let _ = file.to_text();
                let _ = file.s_matrix_at(1e9);
            }
        }
    }
}
