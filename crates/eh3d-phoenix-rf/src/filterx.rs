// SPDX-License-Identifier: GPL-3.0-or-later
//! Filters with components of the standard series: every inductor and capacitor of a designed
//! ladder is replaced by the nearest value of E12, E24 or E96, and the effect is measured on the
//! response: how far each band edge moves and how much the insertion loss changes in the pass
//! band. The response is the ideal one from ABCD matrices (no loss, no parasitics, no tolerance
//! of the components): this measures the effect of the rounding of the values and nothing else.
//!
//! A band edge here is where the insertion loss crosses the loss that the ideal filter has at its
//! design edge (3.01 dB for a Butterworth filter, the ripple for a Chebyshev one). The edge is
//! found by a scan and then by bisection.

use crate::error::RfError;
use crate::eseries::Series;
use crate::filter::{Band, Branch, Filter};
use crate::settings::RfSettings;

/// One component with the value of the design and the value of the series.
#[derive(Clone, Debug, PartialEq)]
pub struct SnappedValue {
    /// The name: `L` or `C` and the number of the section from the source (counted from 1).
    pub name: String,
    /// The value of the design in henry or farad.
    pub ideal: f64,
    /// The value of the series in henry or farad.
    pub snapped: f64,
    /// The error of the series value in percent: `(snapped - ideal) / ideal * 100`.
    pub error_percent: f64,
}

/// A filter with its components taken to a series.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapped {
    /// The series used.
    pub series: Series,
    /// The filter with the values of the series.
    pub filter: Filter,
    /// The components, in the order of the sections.
    pub values: Vec<SnappedValue>,
}

fn snap_one(
    series: Series,
    name: String,
    ideal: f64,
    values: &mut Vec<SnappedValue>,
) -> Result<f64, RfError> {
    let snapped = series.nearest(ideal)?;
    values.push(SnappedValue {
        name,
        ideal,
        snapped,
        error_percent: (snapped - ideal) / ideal * 100.0,
    });
    Ok(snapped)
}

/// The filter with every component replaced by the nearest value of the series (nearest in
/// ratio).
pub fn snap(filter: &Filter, series: Series) -> Result<Snapped, RfError> {
    let mut values = Vec::new();
    let mut sections = Vec::with_capacity(filter.sections.len());
    for (index, section) in filter.sections.iter().enumerate() {
        let number = index + 1;
        let l_name = format!("L{number}");
        let c_name = format!("C{number}");
        let branch = match section.branch {
            Branch::Inductor(l) => Branch::Inductor(snap_one(series, l_name, l, &mut values)?),
            Branch::Capacitor(c) => Branch::Capacitor(snap_one(series, c_name, c, &mut values)?),
            Branch::SeriesLc {
                inductance_h,
                capacitance_f,
            } => Branch::SeriesLc {
                inductance_h: snap_one(series, l_name, inductance_h, &mut values)?,
                capacitance_f: snap_one(series, c_name, capacitance_f, &mut values)?,
            },
            Branch::ParallelLc {
                inductance_h,
                capacitance_f,
            } => Branch::ParallelLc {
                inductance_h: snap_one(series, l_name, inductance_h, &mut values)?,
                capacitance_f: snap_one(series, c_name, capacitance_f, &mut values)?,
            },
        };
        sections.push(crate::filter::Section {
            position: section.position,
            branch,
        });
    }
    Ok(Snapped {
        series,
        filter: Filter {
            sections,
            ..filter.clone()
        },
        values,
    })
}

/// The edges of a band as designed, in Hz: one for a low-pass or a high-pass filter, two for a
/// band filter.
pub fn design_edges(band: Band) -> Vec<f64> {
    match band {
        Band::LowPass { cutoff_hz } | Band::HighPass { cutoff_hz } => vec![cutoff_hz],
        Band::BandPass { low_hz, high_hz } | Band::BandStop { low_hz, high_hz } => {
            vec![low_hz, high_hz]
        }
    }
}

/// The insertion loss in dB at a frequency; a point where the ABCD matrices are singular counts
/// as infinite loss.
fn loss_db(filter: &Filter, freq_hz: f64) -> f64 {
    filter
        .response_at(freq_hz)
        .map(|p| p.insertion_loss_db())
        .unwrap_or(f64::INFINITY)
}

/// The insertion loss in dB that the filter has at its first design edge: the reference that
/// defines a band edge.
pub fn reference_loss_db(filter: &Filter) -> Result<f64, RfError> {
    let edge = design_edges(filter.band)[0];
    let loss = filter.response_at(edge)?.insertion_loss_db();
    if loss.is_finite() {
        Ok(loss)
    } else {
        Err(RfError::Singular {
            what: "the insertion loss at the design edge".to_string(),
        })
    }
}

/// The band edges of a filter in Hz, where the insertion loss crosses `reference_db`: one for a
/// low-pass or a high-pass filter, the lower and the upper for a band filter. The scan goes from
/// half the lowest design edge to twice the highest, in `scan_points` steps of a geometric
/// progression; an edge outside that range is an error.
pub fn band_edges(
    filter: &Filter,
    reference_db: f64,
    settings: &RfSettings,
) -> Result<Vec<f64>, RfError> {
    let designed = design_edges(filter.band);
    let low = designed.iter().copied().fold(f64::INFINITY, f64::min) / 2.0;
    let high = designed.iter().copied().fold(0.0, f64::max) * 2.0;
    let n = settings.scan_points.max(20);
    let limit = reference_db + 1e-9;
    let passes = |f: f64| loss_db(filter, f) <= limit;
    let grid: Vec<f64> = (0..n)
        .map(|k| low * (high / low).powf(k as f64 / (n - 1) as f64))
        .collect();
    let states: Vec<bool> = grid.iter().map(|f| passes(*f)).collect();
    // the places where the state changes: the index of the point before and whether it passes
    let changes: Vec<(usize, bool)> = (0..n - 1)
        .filter(|k| states[*k] != states[*k + 1])
        .map(|k| (k, states[k]))
        .collect();
    let first_to_pass = changes.iter().find(|(_, p)| !*p).map(|(k, _)| *k);
    let last_to_pass = changes.iter().rev().find(|(_, p)| !*p).map(|(k, _)| *k);
    let first_to_fail = changes.iter().find(|(_, p)| *p).map(|(k, _)| *k);
    let last_to_fail = changes.iter().rev().find(|(_, p)| *p).map(|(k, _)| *k);
    let wanted: Vec<Option<usize>> = match filter.band {
        Band::LowPass { .. } => vec![last_to_fail],
        Band::HighPass { .. } => vec![first_to_pass],
        Band::BandPass { .. } => vec![first_to_pass, last_to_fail],
        Band::BandStop { .. } => vec![first_to_fail, last_to_pass],
    };
    let mut edges = Vec::with_capacity(wanted.len());
    for found in wanted {
        let k = found.ok_or_else(|| RfError::InvalidFilter {
            reason: "a band edge is outside the range of the scan".to_string(),
        })?;
        let (mut a, mut b) = (grid[k], grid[k + 1]);
        let a_passes = states[k];
        for _ in 0..60 {
            let middle = (a * b).sqrt();
            if passes(middle) == a_passes {
                a = middle;
            } else {
                b = middle;
            }
        }
        edges.push((a * b).sqrt());
    }
    Ok(edges)
}

/// What the standard values did to the response.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapEffect {
    /// The insertion loss in dB that defines a band edge.
    pub reference_loss_db: f64,
    /// The band edges of the ideal filter in Hz, as found by the scan.
    pub ideal_edges_hz: Vec<f64>,
    /// The band edges of the filter with standard values in Hz.
    pub snapped_edges_hz: Vec<f64>,
    /// The shift of each edge in percent of the ideal edge.
    pub edge_errors_percent: Vec<f64>,
    /// The largest change of the insertion loss in the pass band, in dB.
    pub max_passband_change_db: f64,
}

/// The pass bands of a filter as ranges of frequency in Hz.
fn pass_ranges(band: Band) -> Vec<(f64, f64)> {
    match band {
        Band::LowPass { cutoff_hz } => vec![(cutoff_hz / 10.0, cutoff_hz)],
        Band::HighPass { cutoff_hz } => vec![(cutoff_hz, cutoff_hz * 10.0)],
        Band::BandPass { low_hz, high_hz } => vec![(low_hz, high_hz)],
        Band::BandStop { low_hz, high_hz } => {
            vec![(low_hz / 10.0, low_hz), (high_hz, high_hz * 10.0)]
        }
    }
}

/// The effect of the standard values: the shift of the band edges and the largest change of the
/// insertion loss in the pass band, between a filter and the same filter with other values.
pub fn snap_effect(
    ideal: &Filter,
    snapped: &Filter,
    settings: &RfSettings,
) -> Result<SnapEffect, RfError> {
    let reference = reference_loss_db(ideal)?;
    let ideal_edges = band_edges(ideal, reference, settings)?;
    let snapped_edges = band_edges(snapped, reference, settings)?;
    let errors: Vec<f64> = ideal_edges
        .iter()
        .zip(&snapped_edges)
        .map(|(a, b)| (b - a) / a * 100.0)
        .collect();
    let n = settings.scan_points.max(20);
    let mut worst = 0.0f64;
    for (start, stop) in pass_ranges(ideal.band) {
        for k in 0..n {
            let f = start + (stop - start) * k as f64 / (n - 1) as f64;
            let (a, b) = (loss_db(ideal, f), loss_db(snapped, f));
            if a.is_finite() && b.is_finite() {
                worst = worst.max((a - b).abs());
            }
        }
    }
    Ok(SnapEffect {
        reference_loss_db: reference,
        ideal_edges_hz: ideal_edges,
        snapped_edges_hz: snapped_edges,
        edge_errors_percent: errors,
        max_passband_change_db: worst,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{design, prototype, Approximation, Position};

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    fn lowpass(approximation: Approximation, order: usize, ripple: Option<f64>) -> Filter {
        let s = settings();
        let p = prototype(approximation, order, ripple, Position::Shunt, &s).unwrap();
        design(&p, Band::LowPass { cutoff_hz: 1e8 }, 50.0, &s).unwrap()
    }

    #[test]
    fn a_filter_against_itself_has_no_shift() {
        let f = lowpass(Approximation::Butterworth, 3, None);
        let effect = snap_effect(&f, &f.clone(), &settings()).unwrap();
        assert!((effect.reference_loss_db - 3.010299956639812).abs() < 1e-9);
        assert!((effect.ideal_edges_hz[0] - 1e8).abs() < 1.0, "{effect:?}");
        assert_eq!(effect.ideal_edges_hz, effect.snapped_edges_hz);
        assert_eq!(effect.edge_errors_percent, vec![0.0]);
        assert_eq!(effect.max_passband_change_db, 0.0);
    }

    #[test]
    fn the_values_of_a_third_order_butterworth_filter_go_to_the_series() {
        let f = lowpass(Approximation::Butterworth, 3, None);
        let e24 = snap(&f, Series::E24).unwrap();
        let names: Vec<&str> = e24.values.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["C1", "L2", "C3"]);
        assert!((e24.values[0].snapped - 33e-12).abs() < 1e-24);
        assert!((e24.values[1].snapped - 160e-9).abs() < 1e-20);
        assert!((e24.values[0].ideal - 3.183098861837907e-11).abs() < 1e-22);
        let want = (33.0 - 31.83098861837907) / 31.83098861837907 * 100.0;
        assert!((e24.values[0].error_percent - want).abs() < 1e-9);
        // the structure of the filter is kept
        assert_eq!(e24.filter.sections.len(), 3);
        assert_eq!(e24.filter.band, f.band);
        assert_eq!(e24.filter.z0_ohm, f.z0_ohm);
    }

    #[test]
    fn the_shift_of_the_cutoff_follows_the_series() {
        // reference values from an independent calculation: 31.83 pF and 159.15 nH
        let f = lowpass(Approximation::Butterworth, 3, None);
        let s = settings();
        for (series, want_percent) in [
            (Series::E12, 2.684926),
            (Series::E24, -1.553503),
            (Series::E96, 0.730977),
        ] {
            let snapped = snap(&f, series).unwrap();
            let effect = snap_effect(&f, &snapped.filter, &s).unwrap();
            assert!(
                (effect.edge_errors_percent[0] - want_percent).abs() < 1e-3,
                "{series:?}: {:?}",
                effect.edge_errors_percent
            );
            assert!(effect.max_passband_change_db > 0.0);
        }
    }

    #[test]
    fn a_finer_series_does_not_move_the_edge_more_than_its_step() {
        let f = lowpass(Approximation::Chebyshev, 5, Some(0.5));
        let s = settings();
        // the largest error of a value is half a step: 1.2% for E96 and 10% for E12
        for (series, bound) in [(Series::E96, 1.3), (Series::E12, 11.0)] {
            let snapped = snap(&f, series).unwrap();
            let effect = snap_effect(&f, &snapped.filter, &s).unwrap();
            assert!(
                effect.edge_errors_percent[0].abs() < bound,
                "{series:?}: {:?}",
                effect.edge_errors_percent
            );
        }
    }

    #[test]
    fn the_edges_of_band_filters_are_found() {
        let s = settings();
        let p = prototype(Approximation::Butterworth, 3, None, Position::Shunt, &s).unwrap();
        for band in [
            Band::BandPass {
                low_hz: 9e7,
                high_hz: 1.1e8,
            },
            Band::BandStop {
                low_hz: 9e7,
                high_hz: 1.1e8,
            },
        ] {
            let f = design(&p, band, 50.0, &s).unwrap();
            let reference = reference_loss_db(&f).unwrap();
            let edges = band_edges(&f, reference, &s).unwrap();
            assert_eq!(edges.len(), 2);
            assert!((edges[0] - 9e7).abs() / 9e7 < 1e-6, "{edges:?}");
            assert!((edges[1] - 1.1e8).abs() / 1.1e8 < 1e-6, "{edges:?}");
            let snapped = snap(&f, Series::E96).unwrap();
            let effect = snap_effect(&f, &snapped.filter, &s).unwrap();
            assert_eq!(effect.edge_errors_percent.len(), 2);
            assert!(effect.edge_errors_percent.iter().all(|e| e.abs() < 5.0));
        }
    }

    #[test]
    fn the_edge_of_a_high_pass_filter_is_found() {
        let s = settings();
        let p = prototype(Approximation::Butterworth, 4, None, Position::Series, &s).unwrap();
        let f = design(&p, Band::HighPass { cutoff_hz: 2e8 }, 50.0, &s).unwrap();
        let reference = reference_loss_db(&f).unwrap();
        let edges = band_edges(&f, reference, &s).unwrap();
        assert!((edges[0] - 2e8).abs() / 2e8 < 1e-9, "{edges:?}");
    }

    #[test]
    fn an_edge_outside_the_scan_is_an_error() {
        let f = lowpass(Approximation::Butterworth, 3, None);
        // a reference loss of 80 dB is never reached below twice the cutoff
        assert!(matches!(
            band_edges(&f, 80.0, &settings()),
            Err(RfError::InvalidFilter { .. })
        ));
    }
}
