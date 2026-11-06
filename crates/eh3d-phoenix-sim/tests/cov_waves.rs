// SPDX-License-Identifier: GPL-3.0-or-later
//! The waveforms of the sources, the unwrapping of a phase and the names of ground, checked
//! with values worked out by hand from their definitions (`SIN`, `PULSE` and `PWL` of the
//! netlist language).

use eh3d_phoenix_rf::Complex;
use eh3d_phoenix_sim::measure::unwrap_degrees;
use eh3d_phoenix_sim::netlist::Wave;
use eh3d_phoenix_sim::SimSettings;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn pulse() -> Wave {
    // 0 to 5 V, delay 1 ms, rise 1 ms, fall 2 ms, width 3 ms, period 10 ms
    Wave::Pulse {
        v1: 0.0,
        v2: 5.0,
        td: 1e-3,
        tr: 1e-3,
        tf: 2e-3,
        pw: 3e-3,
        per: 10e-3,
    }
}

#[test]
fn a_pulse_rises_stays_falls_and_repeats() {
    let w = pulse();
    // before the delay it is the initial value
    assert!(close(w.at(0.5e-3), 0.0));
    // halfway up the rise: 5 * 0.5 ms / 1 ms
    assert!(close(w.at(1.5e-3), 2.5));
    // on the plateau, from 2 ms to 5 ms
    assert!(close(w.at(3e-3), 5.0));
    // a quarter of the way down the fall (0.5 ms of 2 ms): 5 - 5 * 0.25
    assert!(close(w.at(5.5e-3), 3.75));
    // after the fall, until the next period starts at 11 ms
    assert!(close(w.at(8e-3), 0.0));
    // the second period: 1.5 ms after its start it is on the plateau again
    assert!(close(w.at(12.5e-3), 5.0));
}

#[test]
fn a_pulse_without_a_period_happens_once() {
    let once = Wave::Pulse {
        v1: 1.0,
        v2: 3.0,
        td: 0.0,
        tr: 1.0,
        tf: 1.0,
        pw: 2.0,
        per: 0.0,
    };
    // the middle of the rise, the plateau, the middle of the fall and the end
    assert!(close(once.at(0.5), 2.0));
    assert!(close(once.at(2.0), 3.0));
    assert!(close(once.at(3.5), 2.0));
    assert!(close(once.at(10.0), 1.0));
    assert!(close(once.at(100.0), 1.0));
}

#[test]
fn a_sine_follows_its_formula_and_waits_for_its_delay() {
    let sine = Wave::Sin {
        vo: 1.0,
        va: 2.0,
        freq: 1000.0,
        td: 0.0,
        theta: 0.0,
        phase_deg: 0.0,
    };
    assert!(close(sine.at(0.0), 1.0));
    // a quarter of a period of 1 ms: the crest
    assert!(close(sine.at(0.25e-3), 3.0));
    // three quarters: the trough
    assert!(close(sine.at(0.75e-3), -1.0));
    let late = Wave::Sin {
        vo: 1.0,
        va: 2.0,
        freq: 1000.0,
        td: 1e-3,
        theta: 0.0,
        phase_deg: 0.0,
    };
    // before the delay it is the offset, and it starts as if time began at the delay
    assert!(close(late.at(0.5e-3), 1.0));
    assert!(close(late.at(1.25e-3), 3.0));
    let shifted = Wave::Sin {
        vo: 0.0,
        va: 1.0,
        freq: 1000.0,
        td: 0.0,
        theta: 0.0,
        phase_deg: 90.0,
    };
    // a phase of 90 degrees starts at the crest
    assert!(close(shifted.at(0.0), 1.0));
}

#[test]
fn a_damped_sine_loses_its_amplitude_by_the_damping_factor() {
    let damped = Wave::Sin {
        vo: 0.0,
        va: 2.0,
        freq: 1000.0,
        td: 0.0,
        theta: 1000.0,
        phase_deg: 0.0,
    };
    // at a quarter of a period the sine is 1 and the envelope is 2 * exp(-1000 * 0.25 ms)
    let expected = 2.0 * (-0.25f64).exp();
    assert!(close(damped.at(0.25e-3), expected));
}

#[test]
fn a_piecewise_linear_wave_interpolates_and_keeps_its_ends() {
    let w = Wave::Pwl(vec![(0.0, 0.0), (1.0, 10.0), (3.0, 0.0)]);
    assert!(close(w.at(-1.0), 0.0));
    assert!(close(w.at(0.5), 5.0));
    assert!(close(w.at(1.0), 10.0));
    assert!(close(w.at(2.0), 5.0));
    assert!(close(w.at(4.0), 0.0));
    // a jump: two points at the same time; the value at that time is the one before it
    let step = Wave::Pwl(vec![(0.0, 0.0), (1.0, 0.0), (1.0, 5.0), (2.0, 5.0)]);
    assert!(close(step.at(1.0), 0.0));
    assert!(close(step.at(1.5), 5.0));
    // no points: zero
    assert!(close(Wave::Pwl(Vec::new()).at(1.0), 0.0));
    assert!(close(Wave::None.at(1.0), 0.0));
}

#[test]
fn the_start_value_is_what_the_wave_has_before_it_moves() {
    assert!(close(Wave::None.start_value(), 0.0));
    assert!(close(pulse().start_value(), 0.0));
    let sine = Wave::Sin {
        vo: 2.5,
        va: 1.0,
        freq: 50.0,
        td: 0.0,
        theta: 0.0,
        phase_deg: 0.0,
    };
    assert!(close(sine.start_value(), 2.5));
    assert!(close(
        Wave::Pwl(vec![(0.0, 7.0), (1.0, 9.0)]).start_value(),
        7.0
    ));
    assert!(close(Wave::Pwl(Vec::new()).start_value(), 0.0));
}

#[test]
fn the_breakpoints_of_a_pulse_are_its_four_corners_in_every_period() {
    let w = pulse();
    // periods starting at 1, 11 and 21 ms inside 30 ms: three periods of four corners
    let mut found = w.breakpoints(30e-3, 100).unwrap();
    found.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(found.len(), 12);
    let expected = [
        1e-3, 2e-3, 5e-3, 7e-3, 11e-3, 12e-3, 15e-3, 17e-3, 21e-3, 22e-3, 25e-3, 27e-3,
    ];
    for (got, want) in found.iter().zip(expected) {
        assert!((got - want).abs() < 1e-12, "{got} against {want}");
    }
    // a cut that leaves out the last corners of the period
    let mut early = w.breakpoints(4e-3, 100).unwrap();
    early.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(early.len(), 2);
    // more corners than the limit allows: nothing is returned
    assert!(w.breakpoints(30e-3, 11).is_none());
    assert!(w.breakpoints(30e-3, 12).is_some());
}

#[test]
fn the_breakpoints_of_the_other_waves() {
    assert_eq!(Wave::None.breakpoints(1.0, 10), Some(Vec::new()));
    let delayed = Wave::Sin {
        vo: 0.0,
        va: 1.0,
        freq: 1.0,
        td: 2e-3,
        theta: 0.0,
        phase_deg: 0.0,
    };
    assert_eq!(delayed.breakpoints(1e-3, 10), Some(Vec::new()));
    assert_eq!(delayed.breakpoints(5e-3, 10), Some(vec![2e-3]));
    // a delay of zero is not a corner inside (0, stop]
    let straight = Wave::Sin {
        vo: 0.0,
        va: 1.0,
        freq: 1.0,
        td: 0.0,
        theta: 0.0,
        phase_deg: 0.0,
    };
    assert_eq!(straight.breakpoints(1.0, 10), Some(Vec::new()));
    let w = Wave::Pwl(vec![(0.0, 0.0), (1.0, 2.0), (2.5, 2.0), (9.0, 0.0)]);
    assert_eq!(w.breakpoints(3.0, 10), Some(vec![1.0, 2.5]));
    assert!(w.breakpoints(3.0, 1).is_none());
}

fn at_degrees(values: &[f64]) -> Vec<Complex> {
    values
        .iter()
        .map(|d| Complex::from_polar(1.0, d.to_radians()))
        .collect()
}

#[test]
fn a_rising_phase_goes_past_180_degrees_without_a_jump() {
    // 170, -170 (that is 190), -130 (230) and -90 (270) degrees as the principal values give them
    let unwrapped = unwrap_degrees(&at_degrees(&[170.0, -170.0, -130.0, -90.0]));
    let expected = [170.0, 190.0, 230.0, 270.0];
    assert_eq!(unwrapped.len(), 4);
    for (got, want) in unwrapped.iter().zip(expected) {
        assert!((got - want).abs() < 1e-9, "{got} against {want}");
    }
}

#[test]
fn a_falling_phase_goes_past_minus_180_degrees_without_a_jump() {
    let unwrapped = unwrap_degrees(&at_degrees(&[-170.0, 170.0, 130.0]));
    let expected = [-170.0, -190.0, -230.0];
    for (got, want) in unwrapped.iter().zip(expected) {
        assert!((got - want).abs() < 1e-9, "{got} against {want}");
    }
}

#[test]
fn the_phase_of_nothing_and_of_one_point() {
    assert!(unwrap_degrees(&[]).is_empty());
    let one = unwrap_degrees(&at_degrees(&[-135.0]));
    assert_eq!(one.len(), 1);
    assert!((one[0] + 135.0).abs() < 1e-9);
}

#[test]
fn a_slow_phase_is_left_as_it_is() {
    let values = [0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 179.0];
    let unwrapped = unwrap_degrees(&at_degrees(&values));
    for (got, want) in unwrapped.iter().zip(values) {
        assert!((got - want).abs() < 1e-9, "{got} against {want}");
    }
}

#[test]
fn the_names_of_ground_of_a_schematic_ignore_case_and_edge_spaces() {
    let settings = SimSettings::default();
    for name in ["0", "GND", " gnd ", "AGND", "Dgnd", "pgnd"] {
        assert!(settings.is_schematic_ground(name), "{name}");
    }
    for name in ["", "vcc", "gnd2", "g nd", "earth"] {
        assert!(!settings.is_schematic_ground(name), "{name}");
    }
}
