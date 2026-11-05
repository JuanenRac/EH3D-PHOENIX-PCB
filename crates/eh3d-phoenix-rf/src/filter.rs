// SPDX-License-Identifier: GPL-3.0-or-later
//! Ladder filters from the low-pass prototypes of Butterworth, Chebyshev and Bessel.
//!
//! * The prototype is the table of element values `g1 ... gn` and the load `g(n+1)` for a
//!   cutoff of 1 rad/s and a source of 1 ohm. Butterworth and Chebyshev come from the closed
//!   forms (Matthaei, Young and Jones; Pozar, section 8.3); Bessel comes from the published
//!   table of maximally flat group delay, which is normalised to a delay of 1 s at zero
//!   frequency (so the cutoff given to [`design`] for a Bessel filter is the inverse of that
//!   delay, not the 3 dB point).
//! * [`design`] turns the prototype into a low-pass, high-pass, band-pass or band-stop ladder
//!   of inductors and capacitors for a source impedance and band edges, with the transformations
//!   of Pozar, table 8.7 (a band-pass or band-stop is made at the geometric centre of its
//!   edges).
//! * The response is computed from the ABCD matrices of the sections and is exact for the ideal
//!   elements: no loss, no parasitics, no tolerance.
//!
//! The first element of the ladder is a shunt capacitor or a series inductor, as chosen. With
//! the first element in shunt the load is a resistance if the last element is in shunt and a
//! conductance if it is in series, and the other way round with the first element in series.

use crate::complex::Complex;
use crate::error::RfError;
use crate::network::{abcd_series, abcd_shunt, Mat2};
use crate::settings::RfSettings;
use std::f64::consts::PI;

/// The family of the response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Approximation {
    /// Maximally flat magnitude.
    Butterworth,
    /// Equal ripple in the pass band.
    Chebyshev,
    /// Maximally flat group delay.
    Bessel,
}

/// Where an element sits in the ladder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// In series with the signal.
    Series,
    /// Across the signal, to ground.
    Shunt,
}

/// The highest order of the Bessel table.
pub const MAX_BESSEL_ORDER: usize = 8;

/// The Bessel (maximally flat delay) prototype values, delay-normalised, by order. The values
/// of each row sum to 2 (a delay of 1 s). Source: the standard tables of Matthaei, Young and
/// Jones and of Zverev.
const BESSEL: [&[f64]; MAX_BESSEL_ORDER] = [
    &[2.0],
    &[1.5774, 0.4226],
    &[1.2550, 0.5528, 0.1922],
    &[1.0598, 0.5116, 0.3181, 0.1104],
    &[0.9303, 0.4577, 0.3312, 0.2090, 0.0718],
    &[0.8377, 0.4116, 0.3158, 0.2364, 0.1480, 0.0505],
    &[0.7677, 0.3744, 0.2944, 0.2378, 0.1778, 0.1104, 0.0375],
    &[
        0.7125, 0.3446, 0.2735, 0.2297, 0.1867, 0.1387, 0.0855, 0.0289,
    ],
];

/// A low-pass prototype: cutoff 1 rad/s, source 1 ohm.
#[derive(Clone, Debug, PartialEq)]
pub struct Prototype {
    /// The family.
    pub approximation: Approximation,
    /// The order: the number of elements.
    pub order: usize,
    /// The ripple of a Chebyshev filter in dB.
    pub ripple_db: Option<f64>,
    /// The element values `g1 ... gn`.
    pub g: Vec<f64>,
    /// The load value `g(n+1)`.
    pub g_load: f64,
    /// The position of the first element.
    pub first: Position,
}

impl Prototype {
    /// The position of the element number `index` (from 0).
    pub fn position(&self, index: usize) -> Position {
        let same_as_first = index & 1 == 0;
        match (self.first, same_as_first) {
            (Position::Shunt, true) | (Position::Series, false) => Position::Shunt,
            _ => Position::Series,
        }
    }

    /// The load resistance in ohm for a source resistance of `z0_ohm`.
    pub fn load_ohm(&self, z0_ohm: f64) -> f64 {
        let last_in_shunt = self.position(self.order.saturating_sub(1)) == Position::Shunt;
        if last_in_shunt {
            self.g_load * z0_ohm
        } else {
            z0_ohm / self.g_load
        }
    }
}

/// The prototype of a family and an order. A Chebyshev filter needs its ripple in dB, the
/// other two must not be given one.
pub fn prototype(
    approximation: Approximation,
    order: usize,
    ripple_db: Option<f64>,
    first: Position,
    settings: &RfSettings,
) -> Result<Prototype, RfError> {
    let bad = |reason: String| RfError::InvalidFilter { reason };
    if order == 0 {
        return Err(bad("the order must be at least 1".to_string()));
    }
    if order > settings.max_filter_order {
        return Err(RfError::OutsideLimit {
            field: "filter order".to_string(),
            value: order as f64,
            limit: settings.max_filter_order as f64,
        });
    }
    let (g, g_load, ripple) = match approximation {
        Approximation::Butterworth => {
            if ripple_db.is_some() {
                return Err(bad("a Butterworth filter has no ripple".to_string()));
            }
            (butterworth(order), 1.0, None)
        }
        Approximation::Bessel => {
            if ripple_db.is_some() {
                return Err(bad("a Bessel filter has no ripple".to_string()));
            }
            let row = BESSEL.get(order - 1).ok_or_else(|| {
                bad(format!(
                    "the Bessel table stops at order {MAX_BESSEL_ORDER}"
                ))
            })?;
            (row.to_vec(), 1.0, None)
        }
        Approximation::Chebyshev => {
            let ripple =
                ripple_db.ok_or_else(|| bad("a Chebyshev filter needs a ripple".to_string()))?;
            if !(settings.min_ripple_db..=settings.max_ripple_db).contains(&ripple) {
                return Err(RfError::OutsideLimit {
                    field: "ripple in dB".to_string(),
                    value: ripple,
                    limit: if ripple < settings.min_ripple_db {
                        settings.min_ripple_db
                    } else {
                        settings.max_ripple_db
                    },
                });
            }
            let (g, g_load) = chebyshev(order, ripple);
            (g, g_load, Some(ripple))
        }
    };
    Ok(Prototype {
        approximation,
        order,
        ripple_db: ripple,
        g,
        g_load,
        first,
    })
}

/// `g_k = 2 sin((2k - 1) pi / 2n)`.
fn butterworth(order: usize) -> Vec<f64> {
    let n = order as f64;
    (1..=order)
        .map(|k| 2.0 * ((2.0 * k as f64 - 1.0) * PI / (2.0 * n)).sin())
        .collect()
}

/// The Chebyshev values for a ripple in dB, and the load value (1 for an odd order,
/// `coth^2(beta / 4)` for an even one).
fn chebyshev(order: usize, ripple_db: f64) -> (Vec<f64>, f64) {
    let n = order as f64;
    // beta = ln coth(Lr / 17.37), with 17.37 = 40 / ln 10
    let beta = (1.0 / (ripple_db * std::f64::consts::LN_10 / 40.0).tanh()).ln();
    let gamma = (beta / (2.0 * n)).sinh();
    let a = |k: usize| ((2.0 * k as f64 - 1.0) * PI / (2.0 * n)).sin();
    let b = |k: usize| gamma * gamma + (k as f64 * PI / n).sin().powi(2);
    let mut g: Vec<f64> = Vec::with_capacity(order);
    g.push(2.0 * a(1) / gamma);
    for k in 2..=order {
        let previous = g.last().copied().unwrap_or(1.0);
        g.push(4.0 * a(k - 1) * a(k) / (b(k - 1) * previous));
    }
    let load = if order & 1 == 1 {
        1.0
    } else {
        let coth = 1.0 / (beta / 4.0).tanh();
        coth * coth
    };
    (g, load)
}

/// The least order of a Butterworth filter whose attenuation is at most `passband_db` at the
/// cutoff and at least `stopband_db` at `ratio` times the cutoff (`ratio` above 1).
pub fn butterworth_order(passband_db: f64, stopband_db: f64, ratio: f64) -> Result<usize, RfError> {
    check_spec(passband_db, stopband_db, ratio)?;
    let numerator = (10f64.powf(stopband_db / 10.0) - 1.0) / (10f64.powf(passband_db / 10.0) - 1.0);
    order_from(numerator.log10() / (2.0 * ratio.log10()))
}

/// The least order of a Chebyshev filter with the same meaning of the arguments.
pub fn chebyshev_order(passband_db: f64, stopband_db: f64, ratio: f64) -> Result<usize, RfError> {
    check_spec(passband_db, stopband_db, ratio)?;
    let numerator = (10f64.powf(stopband_db / 10.0) - 1.0) / (10f64.powf(passband_db / 10.0) - 1.0);
    order_from(numerator.sqrt().acosh() / ratio.acosh())
}

fn check_spec(passband_db: f64, stopband_db: f64, ratio: f64) -> Result<(), RfError> {
    let bad = |reason: &str| RfError::InvalidFilter {
        reason: reason.to_string(),
    };
    if !passband_db.is_finite() || !stopband_db.is_finite() || !ratio.is_finite() {
        return Err(bad("the specification must be finite"));
    }
    if passband_db <= 0.0 || stopband_db <= passband_db {
        return Err(bad(
            "the stop band attenuation must exceed the pass band one, above zero",
        ));
    }
    if ratio <= 1.0 {
        return Err(bad("the stop band edge must be above the pass band edge"));
    }
    Ok(())
}

fn order_from(exact: f64) -> Result<usize, RfError> {
    if !exact.is_finite() || exact > 1.0e6 {
        return Err(RfError::InvalidFilter {
            reason: "the order is not a usable number".to_string(),
        });
    }
    Ok(exact.ceil().max(1.0) as usize)
}

/// The band of the filter with its edges in Hz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Band {
    /// Passes below the cutoff.
    LowPass {
        /// The cutoff frequency in Hz.
        cutoff_hz: f64,
    },
    /// Passes above the cutoff.
    HighPass {
        /// The cutoff frequency in Hz.
        cutoff_hz: f64,
    },
    /// Passes between two frequencies.
    BandPass {
        /// The lower edge in Hz.
        low_hz: f64,
        /// The upper edge in Hz.
        high_hz: f64,
    },
    /// Stops between two frequencies.
    BandStop {
        /// The lower edge in Hz.
        low_hz: f64,
        /// The upper edge in Hz.
        high_hz: f64,
    },
}

/// The circuit of one branch of the ladder. Values are in henry and farad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Branch {
    /// An inductor.
    Inductor(f64),
    /// A capacitor.
    Capacitor(f64),
    /// An inductor and a capacitor in series.
    SeriesLc {
        /// The inductance in henry.
        inductance_h: f64,
        /// The capacitance in farad.
        capacitance_f: f64,
    },
    /// An inductor and a capacitor in parallel.
    ParallelLc {
        /// The inductance in henry.
        inductance_h: f64,
        /// The capacitance in farad.
        capacitance_f: f64,
    },
}

impl Branch {
    /// The impedance of the branch at an angular frequency in rad/s.
    pub fn impedance(&self, omega: f64) -> Result<Complex, RfError> {
        match *self {
            Branch::Inductor(l) => Ok(Complex::new(0.0, omega * l)),
            Branch::Capacitor(c) => Complex::ONE.checked_div(Complex::new(0.0, omega * c)),
            Branch::SeriesLc {
                inductance_h,
                capacitance_f,
            } => {
                let capacitor =
                    Complex::ONE.checked_div(Complex::new(0.0, omega * capacitance_f))?;
                Ok(Complex::new(0.0, omega * inductance_h) + capacitor)
            }
            Branch::ParallelLc { .. } => Complex::ONE.checked_div(self.admittance(omega)?),
        }
    }

    /// The admittance of the branch at an angular frequency in rad/s.
    pub fn admittance(&self, omega: f64) -> Result<Complex, RfError> {
        match *self {
            Branch::Capacitor(c) => Ok(Complex::new(0.0, omega * c)),
            Branch::Inductor(l) => Complex::ONE.checked_div(Complex::new(0.0, omega * l)),
            Branch::ParallelLc {
                inductance_h,
                capacitance_f,
            } => {
                let inductor = Complex::ONE.checked_div(Complex::new(0.0, omega * inductance_h))?;
                Ok(Complex::new(0.0, omega * capacitance_f) + inductor)
            }
            Branch::SeriesLc { .. } => Complex::ONE.checked_div(self.impedance(omega)?),
        }
    }
}

/// One section of the ladder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    /// Whether the branch is in series or in shunt.
    pub position: Position,
    /// The branch.
    pub branch: Branch,
}

/// A designed ladder filter.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    /// The prototype it comes from.
    pub prototype: Prototype,
    /// The band and its edges.
    pub band: Band,
    /// The source impedance in ohm.
    pub z0_ohm: f64,
    /// The load resistance in ohm (it differs from the source for an even-order Chebyshev
    /// filter).
    pub load_ohm: f64,
    /// The sections from the source to the load.
    pub sections: Vec<Section>,
}

/// One point of the response.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterPoint {
    /// The frequency in Hz.
    pub freq_hz: f64,
    /// The transmission from the source to the load, relative to the maximum power available.
    pub s21: Complex,
    /// The reflection at the input, against the source impedance.
    pub s11: Complex,
}

impl FilterPoint {
    /// The insertion loss in dB.
    pub fn insertion_loss_db(&self) -> f64 {
        -self.s21.db20()
    }

    /// The return loss in dB.
    pub fn return_loss_db(&self) -> f64 {
        -self.s11.db20()
    }
}

/// Design the ladder for a prototype, a band and a source impedance.
pub fn design(
    prototype: &Prototype,
    band: Band,
    z0_ohm: f64,
    settings: &RfSettings,
) -> Result<Filter, RfError> {
    settings.check_reference_ohm("source impedance", z0_ohm)?;
    let edge = |name: &str, hz: f64| -> Result<f64, RfError> {
        settings.check_frequency_hz(name, hz)?;
        Ok(2.0 * PI * hz)
    };
    // the angular cutoff for the low-pass and the high-pass; the centre and the fractional
    // width for the other two
    let (omega, delta) = match band {
        Band::LowPass { cutoff_hz } | Band::HighPass { cutoff_hz } => {
            (edge("cutoff frequency", cutoff_hz)?, 0.0)
        }
        Band::BandPass { low_hz, high_hz } | Band::BandStop { low_hz, high_hz } => {
            let (w1, w2) = (edge("lower edge", low_hz)?, edge("upper edge", high_hz)?);
            if w1 >= w2 {
                return Err(RfError::InvalidFilter {
                    reason: "the lower edge must be below the upper edge".to_string(),
                });
            }
            let centre = (w1 * w2).sqrt();
            (centre, (w2 - w1) / centre)
        }
    };
    let mut sections = Vec::with_capacity(prototype.order);
    for (index, g) in prototype.g.iter().enumerate() {
        let position = prototype.position(index);
        let g = *g;
        let branch = match (band, position) {
            (Band::LowPass { .. }, Position::Series) => Branch::Inductor(g * z0_ohm / omega),
            (Band::LowPass { .. }, Position::Shunt) => Branch::Capacitor(g / (z0_ohm * omega)),
            (Band::HighPass { .. }, Position::Series) => {
                Branch::Capacitor(1.0 / (omega * z0_ohm * g))
            }
            (Band::HighPass { .. }, Position::Shunt) => Branch::Inductor(z0_ohm / (omega * g)),
            (Band::BandPass { .. }, Position::Series) => Branch::SeriesLc {
                inductance_h: g * z0_ohm / (delta * omega),
                capacitance_f: delta / (omega * z0_ohm * g),
            },
            (Band::BandPass { .. }, Position::Shunt) => Branch::ParallelLc {
                inductance_h: delta * z0_ohm / (omega * g),
                capacitance_f: g / (delta * omega * z0_ohm),
            },
            (Band::BandStop { .. }, Position::Series) => Branch::ParallelLc {
                inductance_h: delta * g * z0_ohm / omega,
                capacitance_f: 1.0 / (delta * omega * g * z0_ohm),
            },
            (Band::BandStop { .. }, Position::Shunt) => Branch::SeriesLc {
                inductance_h: z0_ohm / (delta * omega * g),
                capacitance_f: delta * g / (omega * z0_ohm),
            },
        };
        sections.push(Section { position, branch });
    }
    Ok(Filter {
        prototype: prototype.clone(),
        band,
        z0_ohm,
        load_ohm: prototype.load_ohm(z0_ohm),
        sections,
    })
}

impl Filter {
    /// The ABCD matrix of the whole ladder at a frequency in Hz.
    pub fn abcd_at(&self, freq_hz: f64) -> Result<Mat2, RfError> {
        let omega = 2.0 * PI * crate::error::positive("frequency", freq_hz)?;
        let mut total = Mat2::identity();
        for section in &self.sections {
            let element = match section.position {
                Position::Series => abcd_series(section.branch.impedance(omega)?),
                Position::Shunt => abcd_shunt(section.branch.admittance(omega)?),
            };
            total = total.times(&element);
        }
        Ok(total)
    }

    /// The response at a frequency in Hz, between the source impedance and the load of the
    /// filter.
    pub fn response_at(&self, freq_hz: f64) -> Result<FilterPoint, RfError> {
        let t = self.abcd_at(freq_hz)?;
        let (rs, rl) = (self.z0_ohm, self.load_ohm);
        let load = Complex::real(rl);
        let denominator = t.m11 * load + t.m12 + (t.m21 * load + t.m22).scale(rs);
        let s21 = Complex::real(2.0 * (rs * rl).sqrt()).checked_div(denominator)?;
        let zin = (t.m11 * load + t.m12).checked_div(t.m21 * load + t.m22)?;
        let s11 = (zin - Complex::real(rs)).checked_div(zin + Complex::real(rs))?;
        Ok(FilterPoint { freq_hz, s21, s11 })
    }

    /// The response at `points` frequencies from `start_hz` to `stop_hz`, spaced linearly or in
    /// a geometric progression.
    pub fn sweep(
        &self,
        start_hz: f64,
        stop_hz: f64,
        points: usize,
        logarithmic: bool,
        settings: &RfSettings,
    ) -> Result<Vec<FilterPoint>, RfError> {
        settings.check_frequency_hz("start frequency", start_hz)?;
        settings.check_frequency_hz("stop frequency", stop_hz)?;
        if stop_hz <= start_hz {
            return Err(RfError::InvalidNumber {
                field: "stop frequency".to_string(),
                reason: "must be above the start frequency".to_string(),
            });
        }
        if points < 2 {
            return Err(RfError::InvalidNumber {
                field: "number of points".to_string(),
                reason: "must be at least 2".to_string(),
            });
        }
        if points > settings.max_sweep_points {
            return Err(RfError::OutsideLimit {
                field: "number of points".to_string(),
                value: points as f64,
                limit: settings.max_sweep_points as f64,
            });
        }
        let last = (points - 1) as f64;
        (0..points)
            .map(|i| {
                let t = i as f64 / last;
                let f = if logarithmic {
                    start_hz * (stop_hz / start_hz).powf(t)
                } else {
                    start_hz + (stop_hz - start_hz) * t
                };
                self.response_at(f)
            })
            .collect()
    }

    /// The group delay in seconds at a frequency in Hz, from the phase of the transmission a
    /// small step (a fraction of the frequency set in the settings) below and above it.
    pub fn group_delay_s(&self, freq_hz: f64, settings: &RfSettings) -> Result<f64, RfError> {
        let step = freq_hz * settings.delay_step_fraction;
        let (f1, f2) = (freq_hz - step, freq_hz + step);
        let (a, b) = (self.response_at(f1)?, self.response_at(f2)?);
        crate::network::group_delay_s(a.s21, b.s21, f1, f2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    fn proto(a: Approximation, n: usize, ripple: Option<f64>) -> Prototype {
        prototype(a, n, ripple, Position::Shunt, &settings()).unwrap()
    }

    #[test]
    fn butterworth_values_are_the_book_values() {
        let p = proto(Approximation::Butterworth, 3, None);
        assert_eq!(p.g.len(), 3);
        for (got, want) in p.g.iter().zip([1.0, 2.0, 1.0]) {
            assert!((got - want).abs() < 1e-12, "{got} {want}");
        }
        assert_eq!(p.g_load, 1.0);
        let five = proto(Approximation::Butterworth, 5, None);
        for (got, want) in five.g.iter().zip([0.6180, 1.6180, 2.0, 1.6180, 0.6180]) {
            assert!((got - want).abs() < 1e-4, "{got} {want}");
        }
    }

    #[test]
    fn chebyshev_values_are_the_book_values() {
        // Pozar, table 8.4: 0.5 dB ripple, order 3: 1.5963, 1.0967, 1.5963, 1.0
        let p = proto(Approximation::Chebyshev, 3, Some(0.5));
        for (got, want) in p.g.iter().zip([1.5963, 1.0967, 1.5963]) {
            assert!((got - want).abs() < 2e-4, "{got} {want}");
        }
        assert_eq!(p.g_load, 1.0);
        // 0.5 dB ripple, order 2: 1.4029, 0.7071 and a load of 1.9841
        let two = proto(Approximation::Chebyshev, 2, Some(0.5));
        for (got, want) in two.g.iter().zip([1.4029, 0.7071]) {
            assert!((got - want).abs() < 1e-4, "{got} {want}");
        }
        assert!((two.g_load - 1.9841).abs() < 1e-3, "{}", two.g_load);
        // 3 dB ripple, order 3: 3.3487, 0.7117, 3.3487
        let wide = proto(Approximation::Chebyshev, 3, Some(3.0));
        for (got, want) in wide.g.iter().zip([3.3487, 0.7117, 3.3487]) {
            assert!((got - want).abs() < 1e-3, "{got} {want}");
        }
    }

    #[test]
    fn bessel_rows_sum_to_a_delay_of_two_halves() {
        for order in 1..=MAX_BESSEL_ORDER {
            let p = proto(Approximation::Bessel, order, None);
            assert_eq!(p.g.len(), order);
            let sum: f64 = p.g.iter().sum();
            assert!((sum - 2.0).abs() < 2e-3, "order {order}: {sum}");
            assert!(p.g.windows(2).all(|w| w[0] > w[1]), "order {order}");
        }
        assert!(prototype(Approximation::Bessel, 9, None, Position::Shunt, &settings()).is_err());
    }

    #[test]
    fn the_prototype_alternates_positions_and_places_the_load() {
        let p = proto(Approximation::Butterworth, 3, None);
        assert_eq!(
            (p.position(0), p.position(1), p.position(2)),
            (Position::Shunt, Position::Series, Position::Shunt)
        );
        assert_eq!(p.load_ohm(50.0), 50.0);
        let series_first = prototype(
            Approximation::Butterworth,
            3,
            None,
            Position::Series,
            &settings(),
        )
        .unwrap();
        assert_eq!(series_first.position(0), Position::Series);
        // an even Chebyshev filter with its first element in shunt ends in series: the load is a
        // conductance
        let even = proto(Approximation::Chebyshev, 2, Some(0.5));
        assert!((even.load_ohm(50.0) - 50.0 / even.g_load).abs() < 1e-12);
        let even_series = prototype(
            Approximation::Chebyshev,
            2,
            Some(0.5),
            Position::Series,
            &settings(),
        )
        .unwrap();
        assert!((even_series.load_ohm(50.0) - 50.0 * even_series.g_load).abs() < 1e-12);
    }

    #[test]
    fn the_book_butterworth_low_pass_at_100_mhz() {
        let p = proto(Approximation::Butterworth, 3, None);
        let f = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 50.0, &settings()).unwrap();
        let omega = 2.0 * PI * 100e6;
        assert_eq!(f.sections.len(), 3);
        assert_eq!(f.sections[0].position, Position::Shunt);
        match (
            f.sections[0].branch,
            f.sections[1].branch,
            f.sections[2].branch,
        ) {
            (Branch::Capacitor(c1), Branch::Inductor(l2), Branch::Capacitor(c3)) => {
                assert!((c1 - 1.0 / (50.0 * omega)).abs() < 1e-18);
                assert!((l2 - 2.0 * 50.0 / omega).abs() < 1e-15);
                assert!((c3 - c1).abs() < 1e-18);
                // 31.8 pF, 159 nH, 31.8 pF
                assert!((c1 * 1e12 - 31.831).abs() < 1e-2);
                assert!((l2 * 1e9 - 159.155).abs() < 1e-2);
            }
            other => panic!("{other:?}"),
        }
        // -3.01 dB at the cutoff, flat well below, a rolloff of 18 dB per octave above
        let at_cutoff = f.response_at(100e6).unwrap();
        assert!((at_cutoff.insertion_loss_db() - 3.0102999566).abs() < 1e-6);
        assert!(f.response_at(1e6).unwrap().insertion_loss_db() < 1e-9);
        let octave = f.response_at(1e9).unwrap().insertion_loss_db()
            - f.response_at(500e6).unwrap().insertion_loss_db();
        assert!((octave - 18.0).abs() < 0.5, "{octave}");
        // matched in the pass band
        assert!(f.response_at(1e6).unwrap().return_loss_db() > 60.0);
    }

    #[test]
    fn the_chebyshev_response_has_its_ripple_at_the_cutoff() {
        for (order, ripple) in [(3, 0.5), (5, 0.1), (4, 0.5), (2, 1.0)] {
            let p = proto(Approximation::Chebyshev, order, Some(ripple));
            let f = design(&p, Band::LowPass { cutoff_hz: 1e9 }, 50.0, &settings()).unwrap();
            let loss = f.response_at(1e9).unwrap().insertion_loss_db();
            assert!(
                (loss - ripple).abs() < 2e-3,
                "order {order} ripple {ripple}: {loss}"
            );
            // never above the ripple inside the pass band
            for k in 1..=20 {
                let l = f
                    .response_at(1e9 * f64::from(k) / 20.0)
                    .unwrap()
                    .insertion_loss_db();
                assert!(l <= ripple + 2e-3, "order {order} k {k}: {l}");
            }
            // and rising fast outside
            assert!(f.response_at(2e9).unwrap().insertion_loss_db() > ripple + 10.0 || order == 2);
        }
    }

    #[test]
    fn a_lossless_filter_conserves_power() {
        let cases = [
            (Approximation::Butterworth, 4, None),
            (Approximation::Chebyshev, 4, Some(0.25)),
            (Approximation::Bessel, 5, None),
        ];
        for (approx, order, ripple) in cases {
            let p = proto(approx, order, ripple);
            for band in [
                Band::LowPass { cutoff_hz: 50e6 },
                Band::HighPass { cutoff_hz: 50e6 },
                Band::BandPass {
                    low_hz: 40e6,
                    high_hz: 60e6,
                },
                Band::BandStop {
                    low_hz: 40e6,
                    high_hz: 60e6,
                },
            ] {
                let f = design(&p, band, 50.0, &settings()).unwrap();
                for k in 1..40 {
                    let freq = 3.1e6 * f64::from(k) + 0.123456e6;
                    let point = f.response_at(freq).unwrap();
                    let total = point.s21.norm_sqr() + point.s11.norm_sqr();
                    assert!(
                        (total - 1.0).abs() < 1e-9,
                        "{approx:?} {band:?} {freq}: {total}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_high_pass_is_the_mirror_of_the_low_pass() {
        let p = proto(Approximation::Butterworth, 3, None);
        let lp = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 50.0, &settings()).unwrap();
        let hp = design(&p, Band::HighPass { cutoff_hz: 100e6 }, 50.0, &settings()).unwrap();
        // |H_hp(f)| = |H_lp(fc^2 / f)| for the same prototype and cutoff
        for f in [20e6, 70e6, 130e6, 400e6] {
            let a = hp.response_at(f).unwrap().s21.abs();
            let b = lp.response_at(1e16 / f).unwrap().s21.abs();
            assert!((a - b).abs() < 1e-9, "{f}: {a} {b}");
        }
        assert!((hp.response_at(100e6).unwrap().insertion_loss_db() - 3.0102999566).abs() < 1e-6);
    }

    #[test]
    fn the_band_pass_is_centred_on_the_geometric_mean_of_its_edges() {
        let p = proto(Approximation::Butterworth, 3, None);
        let f = design(
            &p,
            Band::BandPass {
                low_hz: 90e6,
                high_hz: 110e6,
            },
            50.0,
            &settings(),
        )
        .unwrap();
        let centre = (90e6f64 * 110e6).sqrt();
        assert!(f.response_at(centre).unwrap().insertion_loss_db() < 1e-6);
        // 3 dB at both edges
        assert!((f.response_at(90e6).unwrap().insertion_loss_db() - 3.0103).abs() < 1e-3);
        assert!((f.response_at(110e6).unwrap().insertion_loss_db() - 3.0103).abs() < 1e-3);
        assert!(f.response_at(30e6).unwrap().insertion_loss_db() > 40.0);
        assert!(matches!(f.sections[0].branch, Branch::ParallelLc { .. }));
        assert!(matches!(f.sections[1].branch, Branch::SeriesLc { .. }));
    }

    #[test]
    fn the_band_stop_rejects_at_its_centre() {
        let p = proto(Approximation::Butterworth, 3, None);
        let f = design(
            &p,
            Band::BandStop {
                low_hz: 90e6,
                high_hz: 110e6,
            },
            50.0,
            &settings(),
        )
        .unwrap();
        let centre = (90e6f64 * 110e6).sqrt();
        assert!(
            f.response_at(centre * 1.0000001)
                .unwrap()
                .insertion_loss_db()
                > 60.0
        );
        assert!(f.response_at(10e6).unwrap().insertion_loss_db() < 0.01);
        assert!((f.response_at(90e6).unwrap().insertion_loss_db() - 3.0103).abs() < 1e-3);
        assert!(matches!(f.sections[0].branch, Branch::ParallelLc { .. }));
        assert!(matches!(f.sections[1].branch, Branch::SeriesLc { .. }));
    }

    #[test]
    fn the_bessel_filter_has_a_flat_group_delay() {
        let p = proto(Approximation::Bessel, 4, None);
        // the delay of the prototype is 1 s at DC: a cutoff of 1 / delay
        let delay = 5e-9;
        let cutoff = 1.0 / (2.0 * PI * delay);
        let f = design(&p, Band::LowPass { cutoff_hz: cutoff }, 50.0, &settings()).unwrap();
        let low = f.group_delay_s(cutoff / 50.0, &settings()).unwrap();
        assert!((low - delay).abs() < 0.02 * delay, "{low}");
        let mid = f.group_delay_s(cutoff / 3.0, &settings()).unwrap();
        assert!((mid - delay).abs() < 0.05 * delay, "{mid}");
        // a Butterworth filter of the same order is far less flat
        let b = proto(Approximation::Butterworth, 4, None);
        let g = design(&b, Band::LowPass { cutoff_hz: cutoff }, 50.0, &settings()).unwrap();
        let dc = g.group_delay_s(cutoff / 50.0, &settings()).unwrap();
        let edge = g.group_delay_s(cutoff, &settings()).unwrap();
        assert!((edge - dc).abs() > 0.2 * dc);
    }

    #[test]
    fn a_different_source_impedance_scales_the_elements() {
        let p = proto(Approximation::Butterworth, 3, None);
        let a = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 50.0, &settings()).unwrap();
        let b = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 75.0, &settings()).unwrap();
        match (a.sections[1].branch, b.sections[1].branch) {
            (Branch::Inductor(la), Branch::Inductor(lb)) => assert!((lb / la - 1.5).abs() < 1e-12),
            other => panic!("{other:?}"),
        }
        assert!((b.response_at(100e6).unwrap().insertion_loss_db() - 3.0103).abs() < 1e-3);
    }

    #[test]
    fn the_sweep_has_the_requested_points() {
        let p = proto(Approximation::Butterworth, 3, None);
        let f = design(&p, Band::LowPass { cutoff_hz: 100e6 }, 50.0, &settings()).unwrap();
        let linear = f.sweep(10e6, 200e6, 20, false, &settings()).unwrap();
        assert_eq!(linear.len(), 20);
        assert!(
            (linear[0].freq_hz - 10e6).abs() < 1e-6 && (linear[19].freq_hz - 200e6).abs() < 1e-3
        );
        let log = f.sweep(1e6, 1e9, 4, true, &settings()).unwrap();
        assert!((log[1].freq_hz - 1e7).abs() < 1.0 && (log[2].freq_hz - 1e8).abs() < 10.0);
        assert!(f.sweep(1e6, 1e6, 5, false, &settings()).is_err());
        assert!(f.sweep(1e6, 1e9, 1, false, &settings()).is_err());
        assert!(f.sweep(1e6, 1e9, 100_000, false, &settings()).is_err());
        assert!(f.sweep(0.0, 1e9, 5, false, &settings()).is_err());
    }

    #[test]
    fn orders_come_from_the_specification() {
        // 3 dB at the cutoff and 20 dB at twice the cutoff
        assert_eq!(butterworth_order(3.0103, 20.0, 2.0).unwrap(), 4);
        assert_eq!(chebyshev_order(0.5, 20.0, 2.0).unwrap(), 4);
        assert_eq!(butterworth_order(0.5, 20.0, 2.0).unwrap(), 5);
        assert!(
            chebyshev_order(0.5, 20.0, 2.0).unwrap() <= butterworth_order(0.5, 20.0, 2.0).unwrap()
        );
        for (a, s, r) in [
            (3.0, 3.0, 2.0),
            (-1.0, 20.0, 2.0),
            (3.0, 20.0, 1.0),
            (3.0, f64::NAN, 2.0),
        ] {
            assert!(butterworth_order(a, s, r).is_err());
            assert!(chebyshev_order(a, s, r).is_err());
        }
    }

    #[test]
    fn bad_descriptions_are_refused() {
        let s = settings();
        assert!(prototype(Approximation::Butterworth, 0, None, Position::Shunt, &s).is_err());
        assert!(matches!(
            prototype(Approximation::Butterworth, 13, None, Position::Shunt, &s),
            Err(RfError::OutsideLimit { .. })
        ));
        assert!(prototype(
            Approximation::Butterworth,
            3,
            Some(0.5),
            Position::Shunt,
            &s
        )
        .is_err());
        assert!(prototype(Approximation::Bessel, 3, Some(0.5), Position::Shunt, &s).is_err());
        assert!(prototype(Approximation::Chebyshev, 3, None, Position::Shunt, &s).is_err());
        for ripple in [0.0, 0.0001, 7.0, f64::NAN] {
            assert!(prototype(
                Approximation::Chebyshev,
                3,
                Some(ripple),
                Position::Shunt,
                &s
            )
            .is_err());
        }
        let p = proto(Approximation::Butterworth, 3, None);
        assert!(design(&p, Band::LowPass { cutoff_hz: 0.0 }, 50.0, &s).is_err());
        assert!(design(&p, Band::LowPass { cutoff_hz: 1e13 }, 50.0, &s).is_err());
        assert!(design(&p, Band::LowPass { cutoff_hz: 1e8 }, -50.0, &s).is_err());
        assert!(design(
            &p,
            Band::BandPass {
                low_hz: 2e8,
                high_hz: 1e8
            },
            50.0,
            &s
        )
        .is_err());
        assert!(design(
            &p,
            Band::BandStop {
                low_hz: 1e8,
                high_hz: 1e8
            },
            50.0,
            &s
        )
        .is_err());
        let f = design(&p, Band::LowPass { cutoff_hz: 1e8 }, 50.0, &s).unwrap();
        assert!(f.response_at(0.0).is_err() && f.response_at(f64::NAN).is_err());
    }
}
