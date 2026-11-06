// SPDX-License-Identifier: GPL-3.0-or-later
//! The eye diagram of each net: the loss of the line at half the bit rate (the frequency of the
//! fastest pattern) gives the cutoff of the filter of one or two poles, and `eye` does the rest.

use serde::Serialize;

use crate::codes::{Finding, Severity, EYE_HEIGHT, EYE_WIDTH};
use crate::env::Env;
use crate::eye::{eye, EyeInput};
use crate::netdata::NetData;
use crate::phys::{lowpass_cutoff_ghz, round_to};
use crate::settings::SiSettings;

/// The eye of one net.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NetEye {
    /// The net.
    pub net: String,
    /// The bit rate, in megabit per second.
    pub bit_rate_mbps: f64,
    /// The duration of one bit, in picoseconds.
    pub ui_ps: f64,
    /// The rise time of the driver, in picoseconds.
    pub rise_ps: f64,
    /// The frequency at which the loss is taken (half the bit rate), in GHz.
    pub nyquist_ghz: f64,
    /// The loss of the whole line at that frequency, in decibel.
    pub loss_db: f64,
    /// The cutoff of one stage of the filter, in GHz; none for a line without loss.
    pub cutoff_ghz: Option<f64>,
    /// The order of the filter.
    pub order: u32,
    /// The order of the sequence of bits.
    pub prbs: u32,
    /// The bits of the sequence.
    pub bits: usize,
    /// The widest opening in height, as a fraction of the swing.
    pub height: f64,
    /// The width at the middle level, as a fraction of the bit.
    pub width: f64,
    /// The jitter from the interference, peak to peak, in picoseconds.
    pub jitter_pp_ps: f64,
    /// The lag in bits between a bit and the part of the eye that shows it.
    pub lag_bits: usize,
}

/// The eye of every net of the list.
pub(crate) fn analyse(
    nets: &[NetData],
    settings: &SiSettings,
    env: &mut Env,
) -> (Vec<NetEye>, Vec<Finding>) {
    let mut out = Vec::new();
    let mut findings = Vec::new();
    let ui_ps = 1.0e6 / settings.bit_rate_mbps;
    let nyquist = settings.bit_rate_mbps / 2000.0;
    for net in nets {
        let tr = settings.rise_for(&net.name);
        let line = env.line(&net.layer, net.width_nm, nyquist);
        let loss_db = line.loss_db_per_cm * net.length_nm as f64 / 1.0e7;
        let cutoff = lowpass_cutoff_ghz(nyquist, loss_db, settings.eye_order);
        let input = EyeInput {
            prbs: settings.prbs,
            ui_ps,
            tr_ps: tr,
            cutoff_ghz: cutoff,
            stages: settings.eye_order,
            samples_per_ui: settings.samples_per_ui,
        };
        let Some(result) = eye(&input) else {
            continue;
        };
        let subject = Some(net.name.as_str());
        let judge = |value: f64, limit: f64| {
            if value >= limit {
                None
            } else if value < limit / 2.0 {
                Some(Severity::Error)
            } else {
                Some(Severity::Warning)
            }
        };
        if let Some(severity) = judge(result.height, settings.eye_height_warn) {
            findings.push(Finding::new(
                EYE_HEIGHT,
                severity,
                subject,
                format!(
                    "{:.0} % of the swing at {} Mbit/s (limit {:.0} %)",
                    result.height * 100.0,
                    settings.bit_rate_mbps,
                    settings.eye_height_warn * 100.0
                ),
            ));
        }
        if let Some(severity) = judge(result.width, settings.eye_width_warn) {
            findings.push(Finding::new(
                EYE_WIDTH,
                severity,
                subject,
                format!(
                    "{:.0} % of the bit, jitter {:.1} ps (limit {:.0} %)",
                    result.width * 100.0,
                    result.jitter_pp_ps,
                    settings.eye_width_warn * 100.0
                ),
            ));
        }
        out.push(NetEye {
            net: net.name.clone(),
            bit_rate_mbps: settings.bit_rate_mbps,
            ui_ps: round_to(ui_ps, 2),
            rise_ps: tr,
            nyquist_ghz: round_to(nyquist, 4),
            loss_db: round_to(loss_db, 4),
            cutoff_ghz: cutoff.map(|c| round_to(c, 4)),
            order: settings.eye_order,
            prbs: settings.prbs,
            bits: result.bits,
            height: round_to(result.height, 4),
            width: round_to(result.width, 4),
            jitter_pp_ps: round_to(result.jitter_pp_ps, 2),
            lag_bits: result.lag_bits,
        });
    }
    (out, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(length_nm: i64) -> NetData {
        NetData {
            name: "D0".to_owned(),
            length_nm,
            td_ps: 100.0,
            layer: "F.Cu".to_owned(),
            width_nm: 200_000,
        }
    }

    fn run(settings: &SiSettings, length_nm: i64) -> (Vec<NetEye>, Vec<Finding>) {
        let mut env = Env::new(None, settings);
        analyse(&[net(length_nm)], settings, &mut env)
    }

    #[test]
    fn a_short_slow_net_has_an_open_eye_and_no_findings() {
        let settings = SiSettings::default();
        let (eyes, findings) = run(&settings, 20_000_000);
        assert_eq!(eyes.len(), 1);
        assert!(eyes[0].height > 0.9, "{}", eyes[0].height);
        assert!(eyes[0].width > 0.9, "{}", eyes[0].width);
        assert!(findings.is_empty());
        assert_eq!(eyes[0].prbs, 7);
        assert_eq!(eyes[0].bits, 127);
        assert!((eyes[0].ui_ps - 10_000.0).abs() < 1e-9);
    }

    #[test]
    fn a_long_fast_net_loses_its_eye() {
        let settings = SiSettings {
            bit_rate_mbps: 5000.0,
            rise_time_ps: 100.0,
            ..SiSettings::default()
        };
        let (eyes, findings) = run(&settings, 400_000_000);
        assert!(eyes[0].loss_db > 3.0, "{}", eyes[0].loss_db);
        assert!(eyes[0].cutoff_ghz.is_some());
        assert!(eyes[0].height < 0.6, "{}", eyes[0].height);
        assert!(findings.iter().any(|f| f.code == EYE_HEIGHT));
    }

    #[test]
    fn the_second_order_filter_closes_the_eye_more_than_the_first() {
        let first = SiSettings {
            bit_rate_mbps: 2000.0,
            rise_time_ps: 100.0,
            ..SiSettings::default()
        };
        let second = SiSettings {
            eye_order: 2,
            ..first.clone()
        };
        let a = run(&first, 300_000_000).0.remove(0);
        let b = run(&second, 300_000_000).0.remove(0);
        assert!(a.cutoff_ghz.is_some() && b.cutoff_ghz.is_some());
        // both filters lose the same at the fastest pattern, so the second one has the higher
        // cutoff per stage; the results are different
        assert!(a.cutoff_ghz.unwrap() < b.cutoff_ghz.unwrap());
        assert_ne!(a.height, b.height);
    }
}
