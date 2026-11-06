// SPDX-License-Identifier: GPL-3.0-or-later
//! Tolerances of resistors, capacitors and inductors, and the statistics of a Monte Carlo study.
//!
//! A tolerance is a relative band (`tol=5%` on the line of an element, or the option `tol=` of a
//! rule of `config/sim_models.cfg`). A run draws one factor for each element that has one and
//! multiplies its value by it. With a uniform distribution the factor is `1 + tol u` with `u` in
//! `[-1, 1)`; with a normal distribution the band is `mc_sigmas` standard deviations, the draws
//! are cut at the band, and the deviation of the factor is `tol / mc_sigmas`. The draws go in the
//! order of the elements of the netlist, so the same seed gives the same values.

use std::collections::BTreeMap;

use crate::codes::{SimError, MC_INVALID};
use crate::netlist::{Kind, Netlist};
use crate::random::Rng;
use crate::units::parse_number;

/// The distribution of a tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dist {
    /// Evenly over the band.
    Uniform,
    /// A bell over the band (cut at the band).
    Normal,
}

/// The tolerance of one element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// The half width of the band as a fraction of the value (0.05 is 5 %), below 1.
    pub rel: f64,
    /// The distribution, or `None` for the one of the settings.
    pub dist: Option<Dist>,
}

/// The fraction a text means: `5%` is 0.05, and `0.05` is 0.05 too. The result is between 0
/// (included) and 1 (excluded).
pub fn parse_fraction(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let value = match trimmed.strip_suffix('%') {
        Some(number) => parse_number(number)? / 100.0,
        None => parse_number(trimmed)?,
    };
    if value.is_finite() && (0.0..1.0).contains(&value) {
        Some(value)
    } else {
        None
    }
}

/// The distribution a word names.
pub fn parse_dist(word: &str) -> Option<Dist> {
    match word.trim().to_ascii_lowercase().as_str() {
        "uniform" => Some(Dist::Uniform),
        "normal" | "gauss" | "gaussian" => Some(Dist::Normal),
        _ => None,
    }
}

impl Tolerance {
    /// The factor of one draw. `normal_default` and `sigmas` come from the settings.
    pub fn factor(&self, rng: &mut Rng, normal_default: bool, sigmas: f64) -> f64 {
        let normal = match self.dist {
            Some(Dist::Normal) => true,
            Some(Dist::Uniform) => false,
            None => normal_default,
        };
        if !normal {
            return 1.0 + self.rel * rng.symmetric();
        }
        let deviation = self.rel / sigmas;
        for _ in 0..100 {
            let z = rng.normal();
            if z.abs() <= sigmas {
                return 1.0 + deviation * z;
            }
        }
        1.0
    }
}

/// The default tolerances by the first letter of the element (`r`, `c`, `l`) from a text such as
/// `R=1%,C=10%,L=20%`.
pub fn parse_default_tolerances(text: &str) -> Result<BTreeMap<char, Tolerance>, SimError> {
    let bad = |why: String| SimError::new(MC_INVALID, why);
    let mut out: BTreeMap<char, Tolerance> = BTreeMap::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((letter, value)) = part.split_once('=') else {
            return Err(bad(format!("'{part}' is not LETTER=TOLERANCE")));
        };
        let letter = letter.trim().to_ascii_lowercase();
        let mut chars = letter.chars();
        let (Some(first), None) = (chars.next(), chars.next()) else {
            return Err(bad(format!("'{letter}' is not one letter (r, c or l)")));
        };
        if !matches!(first, 'r' | 'c' | 'l') {
            return Err(bad(format!("'{letter}' is not r, c or l")));
        }
        let Some(rel) = parse_fraction(value) else {
            return Err(bad(format!(
                "'{}' is not a tolerance (such as 5% or 0.05, below 100%)",
                value.trim()
            )));
        };
        out.insert(first, Tolerance { rel, dist: None });
    }
    if out.is_empty() {
        return Err(bad("no tolerance is given".to_string()));
    }
    Ok(out)
}

/// Multiplies the value of each resistor, capacitor and inductor that has a tolerance (its own,
/// or the default of its letter) by a random factor.
pub fn apply_tolerances(
    netlist: &mut Netlist,
    defaults: &BTreeMap<char, Tolerance>,
    rng: &mut Rng,
    normal_default: bool,
    sigmas: f64,
) {
    let own = netlist.tolerances.clone();
    for element in netlist.elements.iter_mut() {
        let letter = element
            .name
            .chars()
            .next()
            .map_or(' ', |c| c.to_ascii_lowercase());
        let tolerance = own
            .get(&element.name.to_ascii_lowercase())
            .copied()
            .or_else(|| defaults.get(&letter).copied());
        let Some(tolerance) = tolerance else {
            continue;
        };
        if tolerance.rel == 0.0 {
            continue;
        }
        let factor = tolerance.factor(rng, normal_default, sigmas);
        match &mut element.kind {
            Kind::Resistor(value) => *value *= factor,
            Kind::Capacitor { value, .. } => *value *= factor,
            Kind::Inductor { value, .. } => *value *= factor,
            _ => {}
        }
    }
}

/// The statistics of a list of values.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// How many values.
    pub count: usize,
    /// The mean.
    pub mean: f64,
    /// The sample standard deviation (divided by `n - 1`; zero for one value).
    pub std: f64,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// The percentiles asked for, as `(percent, value)`, by linear interpolation between the
    /// sorted values.
    pub percentiles: Vec<(f64, f64)>,
}

/// The statistics of `values` (all finite) with the percentiles asked for; `None` for no value.
pub fn summarize(values: &[f64], percentiles: &[f64]) -> Option<Summary> {
    if values.is_empty() {
        return None;
    }
    let count = values.len();
    let mean = values.iter().sum::<f64>() / count as f64;
    let std = if count > 1 {
        (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (count - 1) as f64).sqrt()
    } else {
        0.0
    };
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank_value = |percent: f64| -> f64 {
        let rank = percent / 100.0 * (count - 1) as f64;
        let low = rank.floor() as usize;
        let high = (low + 1).min(count - 1);
        let fraction = rank - low as f64;
        sorted[low] + (sorted[high] - sorted[low]) * fraction
    };
    Some(Summary {
        count,
        mean,
        std,
        min: sorted[0],
        max: sorted[count - 1],
        percentiles: percentiles.iter().map(|p| (*p, rank_value(*p))).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use crate::settings::SimSettings;

    #[test]
    fn fractions_are_written_with_a_percent_sign_or_as_a_fraction() {
        assert_eq!(parse_fraction("5%"), Some(0.05));
        assert_eq!(parse_fraction("0.05"), Some(0.05));
        assert_eq!(parse_fraction(" 0.5% "), Some(0.005));
        assert_eq!(parse_fraction("0"), Some(0.0));
        for bad in ["", "%", "100%", "1", "-1%", "abc", "5 %%"] {
            assert_eq!(parse_fraction(bad), None, "{bad}");
        }
    }

    #[test]
    fn default_tolerances_are_read_by_letter() {
        let d = parse_default_tolerances("R=1%, c=10%").unwrap();
        assert_eq!(d[&'r'].rel, 0.01);
        assert_eq!(d[&'c'].rel, 0.1);
        assert!(!d.contains_key(&'l'));
        for bad in ["", "R", "RR=1%", "V=1%", "R=200%", "R=x"] {
            assert_eq!(
                parse_default_tolerances(bad).unwrap_err().code,
                MC_INVALID,
                "{bad}"
            );
        }
    }

    #[test]
    fn a_uniform_factor_stays_in_the_band_and_a_normal_one_is_cut_at_it() {
        let mut rng = Rng::new(5);
        let uniform = Tolerance {
            rel: 0.1,
            dist: Some(Dist::Uniform),
        };
        let normal = Tolerance {
            rel: 0.1,
            dist: Some(Dist::Normal),
        };
        for _ in 0..2000 {
            let u = uniform.factor(&mut rng, true, 3.0);
            let n = normal.factor(&mut rng, false, 3.0);
            assert!((0.9..1.1).contains(&u), "{u}");
            assert!((0.899..=1.101).contains(&n), "{n}");
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_values_in_the_netlist() {
        let s = SimSettings::default();
        let nl = parse("t\nV1 a 0 1\nR1 a b 1k tol=5%\nR2 b 0 1k\nC1 b 0 1u\n", &s).unwrap();
        let defaults = parse_default_tolerances("c=10%").unwrap();
        let run = |seed: u64| {
            let mut copy = nl.clone();
            let mut rng = Rng::stream(seed, 3);
            apply_tolerances(&mut copy, &defaults, &mut rng, false, 3.0);
            copy
        };
        assert_eq!(run(11), run(11));
        assert_ne!(run(11), run(12));
        let copy = run(11);
        // R2 has no tolerance of its own and no default for r
        assert_eq!(copy.elements[2].kind, nl.elements[2].kind);
        assert_ne!(copy.elements[1].kind, nl.elements[1].kind);
        assert_ne!(copy.elements[3].kind, nl.elements[3].kind);
    }

    #[test]
    fn the_summary_has_the_mean_the_deviation_the_extremes_and_the_percentiles() {
        let v = [4.0, 1.0, 3.0, 2.0, 5.0];
        let s = summarize(&v, &[50.0, 25.0, 100.0 - 1e-9]).unwrap();
        assert_eq!(s.count, 5);
        assert!((s.mean - 3.0).abs() < 1e-12);
        // the sample variance of 1..5 is 2.5
        assert!((s.std - 2.5f64.sqrt()).abs() < 1e-12);
        assert_eq!((s.min, s.max), (1.0, 5.0));
        assert!((s.percentiles[0].1 - 3.0).abs() < 1e-12);
        assert!((s.percentiles[1].1 - 2.0).abs() < 1e-12);
        assert!((s.percentiles[2].1 - 5.0).abs() < 1e-6);
        assert!(summarize(&[], &[50.0]).is_none());
        assert_eq!(summarize(&[7.0], &[50.0]).unwrap().std, 0.0);
    }
}
