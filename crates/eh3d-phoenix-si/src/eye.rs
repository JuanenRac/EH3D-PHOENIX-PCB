// SPDX-License-Identifier: GPL-3.0-or-later
//! A simplified eye diagram: a pseudo-random bit sequence with its own generator and a fixed seed
//! goes through the rise time of the driver and a low-pass filter of one or two poles that stands
//! for the loss of the line; the delay of the line only shifts the picture and is left out. The
//! eye is folded over one bit and measured: the height, the width and the jitter that the
//! inter-symbol interference makes at the middle level.
//!
//! The reflections and the crosstalk are not in it, and the filter is a model of the loss and not
//! of the dispersion of the line: an estimate that has not been compared with a simulation or a
//! measurement.

use serde::Serialize;

/// The bits of one period of a maximal-length sequence, from the seed with every bit set:
/// `x^7 + x^6 + 1` (127 bits) or `x^9 + x^5 + 1` (511 bits); `None` for another order.
pub fn prbs_bits(order: u32) -> Option<Vec<bool>> {
    let (length, mask, tap_a, tap_b) = match order {
        7 => (127usize, 0x7F_u32, 6u32, 5u32),
        9 => (511usize, 0x1FF_u32, 8u32, 4u32),
        _ => return None,
    };
    let mut state = mask;
    let mut bits = Vec::with_capacity(length);
    for _ in 0..length {
        let feedback = ((state >> tap_a) ^ (state >> tap_b)) & 1;
        state = ((state << 1) | feedback) & mask;
        bits.push(feedback == 1);
    }
    Some(bits)
}

/// The parameters of one eye.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EyeInput {
    /// The order of the sequence, 7 or 9.
    pub prbs: u32,
    /// The duration of one bit, in picoseconds.
    pub ui_ps: f64,
    /// The rise time of the driver, in picoseconds (the whole ramp).
    pub tr_ps: f64,
    /// The cutoff of one stage of the filter, in GHz; `None` for a line without loss.
    pub cutoff_ghz: Option<f64>,
    /// The number of equal stages of the filter, 1 or 2.
    pub stages: u32,
    /// The samples of one bit.
    pub samples_per_ui: usize,
}

/// What the eye is like.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EyeOutcome {
    /// The bits of the sequence.
    pub bits: usize,
    /// The duration of one bit, in picoseconds.
    pub ui_ps: f64,
    /// The widest opening of the eye in height, as a fraction of the swing of the driver.
    pub height: f64,
    /// The width of the eye at the middle level, as a fraction of the bit.
    pub width: f64,
    /// The jitter that the interference makes, from peak to peak, in picoseconds.
    pub jitter_pp_ps: f64,
    /// The lag in bits between a bit and the part of the eye that shows it.
    pub lag_bits: usize,
}

/// The value at time `t` of the bit sequence with ramps of duration `tr` at the changes between
/// bits (the level of a bit is 0 or 1; the bit before the first one is the last one).
fn drive(bits: &[bool], ui: f64, tr: f64, t: f64) -> f64 {
    let n = bits.len();
    let level = |k: usize| if bits[k % n] { 1.0 } else { 0.0 };
    let current = (t / ui).floor();
    let k = current.max(0.0) as usize;
    let start = if tr > 0.0 {
        ((t - tr) / ui).floor().max(0.0) as usize
    } else {
        k
    };
    // the level before the first ramp that is still going on, plus the steps of the ramps after it
    let mut value = level(start + n - 1);
    for j in start..=k {
        let before = level(j + n - 1);
        let after = level(j);
        let ramp = ((t - j as f64 * ui) / tr.max(1.0e-9)).clamp(0.0, 1.0);
        value += (after - before) * ramp;
    }
    value
}

/// Builds and measures the eye of a sequence of two periods, the second one folded.
pub fn eye(input: &EyeInput) -> Option<EyeOutcome> {
    let bits = prbs_bits(input.prbs)?;
    let n = bits.len();
    let s = input.samples_per_ui;
    let dt = input.ui_ps / s as f64;
    let total = 2 * n * s;
    let mut y: Vec<f64> = (0..total)
        .map(|i| drive(&bits, input.ui_ps, input.tr_ps, i as f64 * dt))
        .collect();
    if let Some(fc) = input.cutoff_ghz {
        let tau_ps = 1.0e3 / (2.0 * std::f64::consts::PI * fc);
        let alpha = 1.0 - (-dt / tau_ps).exp();
        for _ in 0..input.stages.max(1) {
            let mut state = y[0];
            for v in y.iter_mut() {
                state += alpha * (*v - state);
                *v = state;
            }
        }
    }
    // the second period, in which the filter has settled
    let second = &y[n * s..];
    let mut best: Option<(f64, usize)> = None;
    for lag in 0..3 {
        let h = best_height(second, &bits, s, lag);
        if best.is_none_or(|(b, _)| h > b) {
            best = Some((h, lag));
        }
    }
    let (height, lag) = best?;
    let jitter = jitter_pp(second, dt, input.ui_ps);
    Some(EyeOutcome {
        bits: n,
        ui_ps: input.ui_ps,
        height: height.max(0.0),
        width: (1.0 - jitter / input.ui_ps).max(0.0),
        jitter_pp_ps: jitter,
        lag_bits: lag,
    })
}

/// The widest opening in height over the phases of the bit when the samples of bit `k` are taken as
/// the bit `k - lag`.
fn best_height(wave: &[f64], bits: &[bool], s: usize, lag: usize) -> f64 {
    let n = bits.len();
    let mut widest = f64::MIN;
    for phase in 0..s {
        let mut lowest_one = f64::MAX;
        let mut highest_zero = f64::MIN;
        for k in 0..n {
            let v = wave[k * s + phase];
            if bits[(k + n - lag) % n] {
                lowest_one = lowest_one.min(v);
            } else {
                highest_zero = highest_zero.max(v);
            }
        }
        widest = widest.max(lowest_one - highest_zero);
    }
    widest
}

/// The spread of the times at which the wave crosses the middle level, folded over one bit, in
/// picoseconds.
fn jitter_pp(wave: &[f64], dt: f64, ui: f64) -> f64 {
    let mut first: Option<f64> = None;
    let mut low = 0.0_f64;
    let mut high = 0.0_f64;
    for (i, pair) in wave.windows(2).enumerate() {
        let (a, b) = (pair[0] - 0.5, pair[1] - 0.5);
        if a * b < 0.0 {
            let t = (i as f64 + a / (a - b)) * dt;
            let phase = t.rem_euclid(ui);
            let reference = *first.get_or_insert(phase);
            let mut d = phase - reference;
            if d >= ui / 2.0 {
                d -= ui;
            } else if d < -ui / 2.0 {
                d += ui;
            }
            low = low.min(d);
            high = high.max(d);
        }
    }
    high - low
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(cutoff_ghz: Option<f64>) -> EyeInput {
        EyeInput {
            prbs: 7,
            ui_ps: 1000.0,
            tr_ps: 100.0,
            cutoff_ghz,
            stages: 1,
            samples_per_ui: 32,
        }
    }

    #[test]
    fn the_sequences_have_their_period_and_one_more_one_than_zeros() {
        let a = prbs_bits(7).unwrap();
        assert_eq!(a.len(), 127);
        assert_eq!(a.iter().filter(|b| **b).count(), 64);
        let b = prbs_bits(9).unwrap();
        assert_eq!(b.len(), 511);
        assert_eq!(b.iter().filter(|x| **x).count(), 256);
        assert!(prbs_bits(8).is_none());
        // the same every time
        assert_eq!(prbs_bits(7).unwrap(), a);
    }

    #[test]
    fn the_sequence_does_not_repeat_inside_its_period() {
        // a maximal sequence of order 7 shows every pattern of 7 bits but one exactly once
        let bits = prbs_bits(7).unwrap();
        let n = bits.len();
        let mut seen = std::collections::BTreeSet::new();
        for start in 0..n {
            let mut word = 0u32;
            for k in 0..7 {
                word = (word << 1) | u32::from(bits[(start + k) % n]);
            }
            assert!(seen.insert(word), "pattern {word} repeats");
        }
        assert_eq!(seen.len(), 127);
    }

    #[test]
    fn a_line_without_loss_has_an_open_eye() {
        let out = eye(&input(None)).unwrap();
        assert!(out.height > 0.99, "{}", out.height);
        assert!(out.width > 0.99, "{}", out.width);
        assert!(out.jitter_pp_ps < 1.0, "{}", out.jitter_pp_ps);
    }

    #[test]
    fn a_slower_line_closes_the_eye_and_adds_jitter() {
        let mild = eye(&input(Some(2.0))).unwrap();
        let heavy = eye(&input(Some(0.3))).unwrap();
        assert!(mild.height > 0.9, "{}", mild.height);
        assert!(heavy.height < mild.height);
        assert!(heavy.jitter_pp_ps > mild.jitter_pp_ps);
        assert!(heavy.width < mild.width);
    }

    #[test]
    fn two_stages_close_the_eye_more_than_one_of_the_same_cutoff() {
        let one = eye(&input(Some(0.5))).unwrap();
        let two = eye(&EyeInput {
            stages: 2,
            ..input(Some(0.5))
        })
        .unwrap();
        assert!(two.height < one.height);
    }

    #[test]
    fn the_eye_is_the_same_every_time() {
        assert_eq!(eye(&input(Some(0.7))), eye(&input(Some(0.7))));
        let mut bad = input(None);
        bad.prbs = 5;
        assert!(eye(&bad).is_none());
    }
}
