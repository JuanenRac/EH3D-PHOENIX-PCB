// SPDX-License-Identifier: GPL-3.0-or-later
//! The Touchstone reader and writer and the ladder matrices, checked with values worked out by
//! hand: the conversion from Z and Y parameters, the three formats of a pair, interpolation,
//! the group delay of a pure delay, the noise lines, the limits and the lines that are refused.

use std::f64::consts::PI;

use eh3d_phoenix_rf::filter::Position;
use eh3d_phoenix_rf::matching::{Element, ElementKind};
use eh3d_phoenix_rf::matchnet::{ladder_abcd, ladder_input_impedance, Part};
use eh3d_phoenix_rf::touchstone::{Format, FreqUnit, Parameter, Touchstone};
use eh3d_phoenix_rf::{Complex, RfError, RfSettings};

fn parse(text: &str, ports: usize) -> Result<Touchstone, RfError> {
    Touchstone::parse(text, ports, &RfSettings::default())
}

fn near(z: Complex, re: f64, im: f64) -> bool {
    (z.re - re).abs() < 1e-9 && (z.im - im).abs() < 1e-9
}

#[test]
fn a_unit_is_worth_its_power_of_ten_in_hertz() {
    assert_eq!(FreqUnit::Hz.hertz(), 1.0);
    assert_eq!(FreqUnit::KHz.hertz(), 1e3);
    assert_eq!(FreqUnit::MHz.hertz(), 1e6);
    assert_eq!(FreqUnit::GHz.hertz(), 1e9);
}

#[test]
fn the_frequencies_are_scaled_by_the_unit_of_the_option_line() {
    let t = parse("# KHz S RI R 50\n2.5 0.5 0\n7 0.25 0\n", 1).unwrap();
    assert_eq!(t.unit, FreqUnit::KHz);
    assert_eq!(t.points[0].freq_hz, 2500.0);
    assert_eq!(t.points[1].freq_hz, 7000.0);
    assert_eq!(t.frequency_range_hz(), Some((2500.0, 7000.0)));
}

#[test]
fn the_three_formats_of_a_pair_give_the_same_reflection() {
    // 0.5 at 90 degrees is 0 + 0.5j in the three formats; -6.0206 dB is a magnitude of 0.5
    let ri = parse("# GHz S RI R 50\n1 0 0.5\n", 1).unwrap();
    let ma = parse("# GHz S MA R 50\n1 0.5 90\n", 1).unwrap();
    let db = parse("# GHz S DB R 50\n1 -6.0205999133 90\n", 1).unwrap();
    for t in [&ri, &ma, &db] {
        let s = t.s_matrices().unwrap();
        assert_eq!(s.len(), 1);
        assert!(near(s[0].get(0, 0), 0.0, 0.5), "{:?}", s[0].get(0, 0));
    }
    assert_eq!(db.format, Format::Db);
}

#[test]
fn z_and_y_parameters_become_s_parameters_with_the_reference_of_the_file() {
    // S = (Z - 1) / (Z + 1) for a normalised impedance of 3: (3 - 1) / (3 + 1) = 0.5
    let z = parse("# GHz Z RI R 50\n1 3 0\n", 1).unwrap();
    assert_eq!(z.parameter, Parameter::Z);
    assert!(near(z.s_matrix(0).unwrap().get(0, 0), 0.5, 0.0));
    // S = (1 - Y) / (1 + Y) for a normalised admittance of 3: (1 - 3) / (1 + 3) = -0.5
    let y = parse("# GHz Y RI R 50\n1 3 0\n", 1).unwrap();
    assert!(near(y.s_matrix(0).unwrap().get(0, 0), -0.5, 0.0));
    // a matched load: Z = 1 gives S = 0
    let matched = parse("# GHz Z RI\n1 1 0\n", 1).unwrap();
    assert!(near(matched.s_matrix(0).unwrap().get(0, 0), 0.0, 0.0));
}

#[test]
fn h_parameters_are_read_but_not_converted() {
    let h = parse("# GHz H RI R 50\n1 1 0 0 0 0 0 1 0\n", 2).unwrap();
    assert_eq!(h.parameter, Parameter::H);
    assert!(matches!(h.s_matrix(0), Err(RfError::TouchstoneData { .. })));
    assert!(h.s_matrices().is_err());
}

#[test]
fn a_point_that_is_not_in_the_file_is_refused() {
    let t = parse("# GHz S RI\n1 0.5 0\n", 1).unwrap();
    assert!(t.s_matrix(0).is_ok());
    assert!(matches!(t.s_matrix(1), Err(RfError::TouchstoneData { .. })));
}

const ONE_PORT: &str = "! a comment\n# MHz S RI R 50\n100 0.5 0\n200 0 0.5\n300 -0.5 0\n";

#[test]
fn the_matrix_between_two_points_is_a_straight_line_of_the_real_and_imaginary_parts() {
    let t = parse(ONE_PORT, 1).unwrap();
    assert_eq!(t.comments, vec!["a comment".to_string()]);
    assert_eq!(t.s_matrices().unwrap().len(), 3);
    // halfway from (0.5, 0) to (0, 0.5)
    assert!(near(t.s_matrix_at(150e6).unwrap().get(0, 0), 0.25, 0.25));
    // a quarter of the way from (0, 0.5) to (-0.5, 0)
    assert!(near(t.s_matrix_at(225e6).unwrap().get(0, 0), -0.125, 0.375));
    // on the points themselves, the data of the point
    assert!(near(t.s_matrix_at(100e6).unwrap().get(0, 0), 0.5, 0.0));
    assert!(near(t.s_matrix_at(200e6).unwrap().get(0, 0), 0.0, 0.5));
    assert!(near(t.s_matrix_at(300e6).unwrap().get(0, 0), -0.5, 0.0));
}

#[test]
fn nothing_is_extrapolated_outside_the_points_of_the_file() {
    let t = parse(ONE_PORT, 1).unwrap();
    for freq in [99e6, 301e6, 0.0] {
        match t.s_matrix_at(freq) {
            Err(RfError::OutsideRange {
                freq_hz,
                low_hz,
                high_hz,
            }) => {
                assert_eq!(freq_hz, freq);
                assert_eq!(low_hz, 100e6);
                assert_eq!(high_hz, 300e6);
            }
            other => panic!("{freq}: {other:?}"),
        }
    }
}

const DELAY: &str = "# GHz S MA R 50\n\
    1 0 0 1 -36 1 -36 0 0\n\
    2 0 0 1 -72 1 -72 0 0\n";

#[test]
fn the_group_delay_of_a_pure_delay_is_the_delay() {
    // 100 ps of delay turns the phase by 36 degrees at 1 GHz and 72 degrees at 2 GHz
    let t = parse(DELAY, 2).unwrap();
    let tau = t.group_delay_s(1.5e9).unwrap();
    assert!((tau - 100e-12).abs() < 1e-15, "{tau}");
    // the ends of the file give the same value
    assert!((t.group_delay_s(1e9).unwrap() - 100e-12).abs() < 1e-15);
    assert!((t.group_delay_s(2e9).unwrap() - 100e-12).abs() < 1e-15);
    assert!(matches!(
        t.group_delay_s(3e9),
        Err(RfError::OutsideRange { .. })
    ));
}

#[test]
fn the_group_delay_needs_two_ports_and_two_points() {
    let one_port = parse(ONE_PORT, 1).unwrap();
    assert!(matches!(
        one_port.group_delay_s(150e6),
        Err(RfError::InvalidNetwork { .. })
    ));
    let one_point = parse("# GHz S MA\n1 0 0 1 0 1 0 0 0\n", 2).unwrap();
    assert!(matches!(
        one_point.group_delay_s(1e9),
        Err(RfError::InvalidNetwork { .. })
    ));
}

#[test]
fn the_noise_lines_after_the_data_of_a_two_port_are_read_in_magnitude_and_angle() {
    let text = "# GHz S RI R 50\n\
        1 0.1 0 0.8 0 0.8 0 0.2 0\n\
        2 0.1 0 0.7 0 0.7 0 0.2 0\n\
        1.5 2.0 0.5 90 0.4\n";
    let t = parse(text, 2).unwrap();
    assert_eq!(t.points.len(), 2);
    assert_eq!(t.noise.len(), 1);
    let n = t.noise[0];
    assert_eq!(n.freq_hz, 1.5e9);
    assert_eq!(n.nf_min_db, 2.0);
    assert!(near(n.gamma_opt, 0.0, 0.5));
    assert_eq!(n.rn_normalized, 0.4);
}

#[test]
fn noise_data_must_follow_the_network_data_and_increase_in_frequency() {
    let before = "# GHz S RI\n1.5 2.0 0.5 90 0.4\n";
    assert!(matches!(
        parse(before, 2),
        Err(RfError::TouchstoneData { line: 2, .. })
    ));
    let backwards = "# GHz S RI\n1 0.1 0 0.8 0 0.8 0 0.2 0\n\
        2 0.1 0 0.7 0 0.7 0 0.2 0\n\
        1.5 2.0 0.5 90 0.4\n1.2 2.0 0.5 90 0.4\n";
    assert!(matches!(
        parse(backwards, 2),
        Err(RfError::TouchstoneData { line: 5, .. })
    ));
}

#[test]
fn the_two_port_order_of_the_file_is_s11_s21_s12_s22() {
    let t = parse("# GHz S RI\n1 0.1 0 0.2 0 0.3 0 0.4 0\n", 2).unwrap();
    let s = t.s_matrix(0).unwrap();
    assert!(near(s.get(0, 0), 0.1, 0.0));
    assert!(near(s.get(1, 0), 0.2, 0.0));
    assert!(near(s.get(0, 1), 0.3, 0.0));
    assert!(near(s.get(1, 1), 0.4, 0.0));
}

#[test]
fn a_file_written_in_ri_is_read_back_with_the_same_values() {
    let t = parse(DELAY, 2).unwrap();
    let converted = Touchstone {
        format: Format::Ri,
        ..t.clone()
    };
    let text = converted.to_text().unwrap();
    assert!(text.contains("# GHz S RI R 50"));
    let back = parse(&text, 2).unwrap();
    assert_eq!(back.points.len(), 2);
    for (a, b) in t.points.iter().zip(&back.points) {
        assert_eq!(a.freq_hz, b.freq_hz);
        for (x, y) in a.data.iter().zip(&b.data) {
            assert!(near(*x, y.re, y.im));
        }
    }
}

#[test]
fn a_zero_cannot_be_written_in_decibels() {
    let t = parse("# GHz S RI R 50\n1 0 0\n", 1).unwrap();
    let in_db = Touchstone {
        format: Format::Db,
        ..t
    };
    assert!(matches!(
        in_db.to_text(),
        Err(RfError::InvalidNumber { .. })
    ));
}

#[test]
fn lines_that_cannot_be_read_name_their_line() {
    // version 2 keywords
    assert!(matches!(
        parse("[Version] 2.0\n# GHz S RI\n1 0 0\n", 1),
        Err(RfError::TouchstoneSyntax { line: 1, .. })
    ));
    // an option that does not exist
    assert!(matches!(
        parse("# GHz S XX\n1 0 0\n", 1),
        Err(RfError::TouchstoneSyntax { line: 1, .. })
    ));
    // a value that is not a number
    assert!(matches!(
        parse("# GHz S RI\n1 abc 0\n", 1),
        Err(RfError::TouchstoneSyntax { line: 2, .. })
    ));
    // more values than a record of one port holds
    assert!(matches!(
        parse("# GHz S RI\n1 0.5 0 0.1\n", 1),
        Err(RfError::TouchstoneSyntax { line: 2, .. })
    ));
    // a record that stops half way
    assert!(matches!(
        parse("# GHz S RI\n1 0.5\n", 1),
        Err(RfError::TouchstoneData { line: 2, .. })
    ));
    // frequencies that do not increase
    assert!(matches!(
        parse("# GHz S RI\n1 0.5 0\n1 0.4 0\n", 1),
        Err(RfError::TouchstoneData { line: 3, .. })
    ));
    // a negative magnitude
    assert!(matches!(
        parse("# GHz S MA\n1 -0.5 0\n", 1),
        Err(RfError::TouchstoneData { line: 2, .. })
    ));
    // nothing but options and comments
    assert!(matches!(
        parse("! nothing\n# GHz S RI\n", 1),
        Err(RfError::TouchstoneData { line: 0, .. })
    ));
}

#[test]
fn a_reference_resistance_must_be_a_number_above_zero() {
    assert!(matches!(
        parse("# GHz S RI R\n1 0 0\n", 1),
        Err(RfError::TouchstoneSyntax { line: 1, .. })
    ));
    assert!(matches!(
        parse("# GHz S RI R abc\n1 0 0\n", 1),
        Err(RfError::TouchstoneSyntax { line: 1, .. })
    ));
    assert!(matches!(
        parse("# GHz S RI R 0\n1 0 0\n", 1),
        Err(RfError::TouchstoneData { line: 1, .. })
    ));
    assert_eq!(
        parse("# GHz S RI R 75\n1 0 0\n", 1).unwrap().z_ref_ohm,
        75.0
    );
}

#[test]
fn the_limits_of_the_settings_stop_a_file_before_it_is_read_in_full() {
    let three = "# GHz S RI\n1 0 0\n2 0 0\n3 0 0\n";
    let few = RfSettings {
        max_points: 2,
        ..RfSettings::default()
    };
    match Touchstone::parse(three, 1, &few) {
        Err(RfError::OutsideLimit { value, limit, .. }) => {
            assert_eq!(value, 3.0);
            assert_eq!(limit, 2.0);
        }
        other => panic!("{other:?}"),
    }
    let small = RfSettings {
        max_file_bytes: 10,
        ..RfSettings::default()
    };
    assert!(matches!(
        Touchstone::parse(three, 1, &small),
        Err(RfError::OutsideLimit { .. })
    ));
    assert!(matches!(parse(three, 0), Err(RfError::OutsideLimit { .. })));
    assert!(matches!(
        parse(three, 17),
        Err(RfError::OutsideLimit { .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// the ladders

/// The angular frequency of this frequency is one radian per second.
const ONE_RADIAN: f64 = 1.0 / (2.0 * PI);

fn part(position: Position, kind: ElementKind, value: f64) -> Part {
    Part {
        position,
        element: Element { kind, value },
    }
}

#[test]
fn a_series_inductor_then_a_shunt_capacitor_multiply_in_that_order() {
    // at 1 rad/s, L = 2 H and C = 0.5 F: [1 2j; 0 1] [1 0; 0.5j 1] = [1 - 1 , 2j; 0.5j , 1]
    let ladder = [
        part(Position::Series, ElementKind::Inductor, 2.0),
        part(Position::Shunt, ElementKind::Capacitor, 0.5),
    ];
    let m = ladder_abcd(&ladder, ONE_RADIAN).unwrap();
    assert!(near(m.m11, 0.0, 0.0));
    assert!(near(m.m12, 0.0, 2.0));
    assert!(near(m.m21, 0.0, 0.5));
    assert!(near(m.m22, 1.0, 0.0));
    // a lossless reciprocal network has a determinant of one
    assert!(near(m.det(), 1.0, 0.0));
}

#[test]
fn the_other_order_gives_another_matrix() {
    // [1 0; 0.5j 1] [1 2j; 0 1] = [1, 2j; 0.5j, 1 + 0.5j * 2j] = [1, 2j; 0.5j, 0]
    let ladder = [
        part(Position::Shunt, ElementKind::Capacitor, 0.5),
        part(Position::Series, ElementKind::Inductor, 2.0),
    ];
    let m = ladder_abcd(&ladder, ONE_RADIAN).unwrap();
    assert!(near(m.m11, 1.0, 0.0));
    assert!(near(m.m12, 0.0, 2.0));
    assert!(near(m.m21, 0.0, 0.5));
    assert!(near(m.m22, 0.0, 0.0));
}

#[test]
fn a_series_capacitor_and_a_shunt_inductor_have_the_opposite_reactance() {
    // the series capacitor of 0.5 F is 1 / (j 0.5) = -2j ohm; the shunt inductor of 4 H is -0.25j S
    let series = ladder_abcd(
        &[part(Position::Series, ElementKind::Capacitor, 0.5)],
        ONE_RADIAN,
    )
    .unwrap();
    assert!(near(series.m12, 0.0, -2.0));
    let shunt = ladder_abcd(
        &[part(Position::Shunt, ElementKind::Inductor, 4.0)],
        ONE_RADIAN,
    )
    .unwrap();
    assert!(near(shunt.m21, 0.0, -0.25));
}

#[test]
fn an_empty_ladder_is_a_plain_connection_and_the_input_is_the_load() {
    let m = ladder_abcd(&[], 1e6).unwrap();
    assert!(near(m.m11, 1.0, 0.0) && near(m.m22, 1.0, 0.0));
    assert!(near(m.m12, 0.0, 0.0) && near(m.m21, 0.0, 0.0));
    let z = ladder_input_impedance(&[], Complex::real(75.0), 1e6).unwrap();
    assert!(near(z, 75.0, 0.0));
}

#[test]
fn the_input_impedance_of_a_ladder_ending_on_a_load() {
    // (A Z + B) / (C Z + D) with Z = 1: j2 / (1 + 0.5j) = (j2 + 1) / 1.25 = 0.8 + 1.6j
    let ladder = [
        part(Position::Series, ElementKind::Inductor, 2.0),
        part(Position::Shunt, ElementKind::Capacitor, 0.5),
    ];
    let z = ladder_input_impedance(&ladder, Complex::real(1.0), ONE_RADIAN).unwrap();
    assert!(near(z, 0.8, 1.6), "{z:?}");
}

#[test]
fn the_reactance_of_each_kind_of_element_has_its_sign() {
    let at = |position, kind, value| {
        part(position, kind, value)
            .reactance_or_susceptance(ONE_RADIAN)
            .unwrap()
    };
    assert!((at(Position::Series, ElementKind::Inductor, 2.0) - 2.0).abs() < 1e-9);
    assert!((at(Position::Series, ElementKind::Capacitor, 0.5) + 2.0).abs() < 1e-9);
    assert!((at(Position::Shunt, ElementKind::Capacitor, 0.5) - 0.5).abs() < 1e-9);
    assert!((at(Position::Shunt, ElementKind::Inductor, 4.0) + 0.25).abs() < 1e-9);
}

#[test]
fn a_zero_element_or_a_zero_frequency_is_refused() {
    let zero_capacitor = part(Position::Series, ElementKind::Capacitor, 0.0);
    assert!(matches!(
        zero_capacitor.reactance_or_susceptance(1e6),
        Err(RfError::Singular { .. })
    ));
    let inductor = part(Position::Series, ElementKind::Inductor, 1e-6);
    assert!(inductor.abcd(0.0).is_err());
    assert!(inductor.abcd(-1.0).is_err());
    assert!(inductor.abcd(f64::NAN).is_err());
    assert!(ladder_abcd(&[inductor], 0.0).is_err());
}
