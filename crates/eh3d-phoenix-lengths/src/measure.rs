// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper length of each net and its delay.
//!
//! The length of a net is the sum of the length of every piece of track of the net and of the
//! barrel of every via of the net; it is the total copper, not the path from one pad to another,
//! so a net with a branch counts the branch too. A straight piece is measured to the nearest
//! nanometre with an exact integer square root; an arc is measured with floating-point
//! trigonometry and rounded, so its length is an estimate and the report says so.
//!
//! The delay of a track is its length times the delay of its layer. The delay of a layer is, in
//! this order: the one the settings give for the layer by name, the one worked out from the
//! stack-up for the width of the track (microstrip on an outer copper layer, stripline on an inner
//! one), and the default of the settings. The delay of a via is its length times the delay of the
//! settings. Lengths are in nanometres and delays in whole femtoseconds, so the sums are exact.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use eh3d_phoenix_board::{Board, Id, Segment, Via};
use eh3d_phoenix_stackup::{
    analyze, Layer as StackLayer, LineKind, LineSpec, Stackup, StackupJob, StackupSettings,
};
use serde::Serialize;

use crate::codes::{
    Finding, LengthError, Severity, ARC_ESTIMATED, LAYER_NO_DELAY, LIMIT_EXCEEDED, STACKUP_FALLBACK,
};
use crate::geom::{seg_len, P};
use crate::settings::{to_fs, LengthSettings};

/// The length and the delay of a net on one layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LayerLength {
    /// The name of the layer.
    pub layer: String,
    /// The length of track on it, in nanometres.
    pub length_nm: i64,
    /// Its delay, in femtoseconds.
    pub delay_fs: i64,
}

/// The length of one net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetLength {
    /// The net.
    pub net: Id,
    /// Its name.
    pub name: String,
    /// The length of its tracks, in nanometres.
    pub tracks_nm: i64,
    /// The number of its vias.
    pub vias: usize,
    /// The length of its vias, in nanometres.
    pub vias_nm: i64,
    /// Tracks and vias together, in nanometres.
    pub total_nm: i64,
    /// The delay of tracks and vias together, in femtoseconds.
    pub delay_fs: i64,
    /// How many of its pieces are arcs.
    pub arcs: usize,
    /// The length and delay by layer, in the order of the layers.
    pub layers: Vec<LayerLength>,
}

/// Rounded division of `n` by `d` (above zero) to the nearest whole number, halves up.
fn round_div(n: i128, d: i128) -> i128 {
    (2 * n + d).div_euclid(2 * d)
}

fn to_i64(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// The length of an arc piece from its end points and its sweep, to the nearest nanometre.
fn arc_len(a: P, b: P, sweep_udeg: i64) -> i64 {
    let chord = seg_len(a, b);
    let theta = (sweep_udeg.unsigned_abs() as f64) / 1_000_000.0 * PI / 180.0;
    let sin_half = (theta / 2.0).sin();
    if chord == 0 || !theta.is_finite() || theta <= 1e-9 || sin_half <= 1e-6 {
        return chord;
    }
    let radius = (chord as f64) / (2.0 * sin_half);
    let length = radius * theta;
    if length.is_finite() && length < 4.0e18 {
        length.round() as i64
    } else {
        chord
    }
}

/// What the delay of a layer is worked out from.
struct Delays<'a> {
    board: &'a Board,
    settings: &'a LengthSettings,
    job: Option<&'a StackupJob>,
    stackup_settings: StackupSettings,
    cache: BTreeMap<(Id, i64), i64>,
    told: BTreeSet<(Id, &'static str)>,
    findings: Vec<Finding>,
}

impl Delays<'_> {
    fn layer_name(&self, layer: Id) -> String {
        self.board
            .layers
            .get(&layer)
            .map_or_else(|| layer.to_string(), |l| l.name.clone())
    }

    fn tell(&mut self, layer: Id, code: &'static str, severity: Severity, detail: String) {
        if self.told.insert((layer, code)) {
            let name = self.layer_name(layer);
            self.findings
                .push(Finding::new(code, severity, Some(name.as_str()), detail));
        }
    }

    /// The delay of a track of this width on this layer, in femtoseconds per millimetre.
    fn rate(&mut self, layer: Id, width_nm: i64) -> i64 {
        if let Some(v) = self.cache.get(&(layer, width_nm)) {
            return *v;
        }
        let name = self.layer_name(layer);
        let value = self.work_out(layer, &name, width_nm);
        self.cache.insert((layer, width_nm), value);
        value
    }

    fn work_out(&mut self, layer: Id, name: &str, width_nm: i64) -> i64 {
        if let Some(v) = self.settings.layer_delay_fs_per_mm(name) {
            return v;
        }
        if let (true, Some(job)) = (self.settings.use_stackup, self.job) {
            match stack_delay(job, name, width_nm, self.settings, &self.stackup_settings) {
                Ok(v) => return v,
                Err(why) => {
                    self.tell(layer, STACKUP_FALLBACK, Severity::Warning, why);
                    return to_fs(self.settings.default_delay_ps_per_mm);
                }
            }
        }
        self.tell(
            layer,
            LAYER_NO_DELAY,
            Severity::Info,
            format!("{} ps/mm is used", self.settings.default_delay_ps_per_mm),
        );
        to_fs(self.settings.default_delay_ps_per_mm)
    }
}

/// The delay of a track of this width on a layer of the stack-up, in femtoseconds per millimetre,
/// or why it could not be worked out.
fn stack_delay(
    job: &StackupJob,
    layer: &str,
    width_nm: i64,
    settings: &LengthSettings,
    stackup_settings: &StackupSettings,
) -> Result<i64, String> {
    let stackup = &job.stackup;
    let at = stackup
        .find(layer)
        .ok_or_else(|| format!("the stack-up has no layer named {layer}"))?;
    let coppers: Vec<usize> = stackup
        .layers
        .iter()
        .enumerate()
        .filter(|(_, l)| l.is_copper())
        .map(|(i, _)| i)
        .collect();
    let outer = coppers.first() == Some(&at) || coppers.last() == Some(&at);
    let spec = LineSpec {
        name: layer.to_owned(),
        kind: if outer {
            LineKind::Microstrip
        } else {
            LineKind::Stripline
        },
        layer: layer.to_owned(),
        width_nm: Some(width_nm),
        target_ohm: None,
        gap_nm: None,
    };
    let result = analyze(
        stackup,
        &job.materials,
        &spec,
        width_nm,
        settings.stackup_frequency_ghz,
        stackup_settings,
    )
    .map_err(|e| e.to_string())?;
    Ok(to_fs(result.delay_ps_per_mm))
}

/// The length of a via barrel: from the middle of the copper of its first layer to the middle of
/// the copper of its last, when the stack-up has both, else the default.
fn via_length(board: &Board, via: &Via, stackup: Option<&Stackup>, default: i64) -> i64 {
    let Some(stackup) = stackup else {
        return default;
    };
    let name = |id: Id| board.layers.get(&id).map(|l| l.name.as_str());
    let (Some(from), Some(to)) = (name(via.from_layer), name(via.to_layer)) else {
        return default;
    };
    let (Some(i), Some(j)) = (stackup.find(from), stackup.find(to)) else {
        return default;
    };
    if i == j {
        return default;
    }
    let (lo, hi) = (i.min(j), i.max(j));
    let mut sum: i64 = 0;
    for (k, layer) in stackup.layers.iter().enumerate() {
        if k < lo || k > hi {
            continue;
        }
        let Ok(t) = layer.thickness_nm() else {
            return default;
        };
        let copper_end = matches!(layer, StackLayer::Copper { .. }) && (k == lo || k == hi);
        sum = sum.saturating_add(if copper_end { t / 2 } else { t });
    }
    sum
}

/// Measures every net that has copper. The nets come in the order of their names, then their ids.
pub fn measure_nets(
    board: &Board,
    settings: &LengthSettings,
    job: Option<&StackupJob>,
) -> Result<(Vec<NetLength>, Vec<Finding>), LengthError> {
    settings.validate()?;
    if board.tracks.len() + board.vias.len() > settings.max_items {
        return Err(LengthError::new(
            LIMIT_EXCEEDED,
            "the board has more tracks and vias than max_items",
        ));
    }
    let mut delays = Delays {
        board,
        settings,
        job,
        stackup_settings: StackupSettings::default(),
        cache: BTreeMap::new(),
        told: BTreeSet::new(),
        findings: Vec::new(),
    };
    #[derive(Default)]
    struct Sum {
        tracks_nm: i128,
        vias: usize,
        vias_nm: i128,
        weighted: i128,
        arcs: usize,
        by_layer: BTreeMap<Id, (i128, i128)>,
    }
    let mut sums: BTreeMap<Id, Sum> = BTreeMap::new();
    for track in board.tracks.values() {
        let width = track.width.get();
        let rate = i128::from(delays.rate(track.layer, width));
        let sum = sums.entry(track.net).or_default();
        for segment in &track.path {
            let (a, b) = (segment.start(), segment.end());
            let (a, b) = ((a.x_nm.get(), a.y_nm.get()), (b.x_nm.get(), b.y_nm.get()));
            let len = match segment {
                Segment::Line { .. } => seg_len(a, b),
                Segment::Arc { sweep, .. } => {
                    sum.arcs += 1;
                    arc_len(a, b, sweep.get())
                }
            };
            let len = i128::from(len);
            sum.tracks_nm += len;
            sum.weighted += len * rate;
            let entry = sum.by_layer.entry(track.layer).or_insert((0, 0));
            entry.0 += len;
            entry.1 += len * rate;
        }
    }
    let via_rate = i128::from(to_fs(settings.via_delay_ps_per_mm));
    let stackup = job.map(|j| &j.stackup);
    for via in board.vias.values() {
        let len = i128::from(via_length(
            board,
            via,
            stackup,
            settings.default_via_length_nm,
        ));
        let sum = sums.entry(via.net).or_default();
        sum.vias += 1;
        sum.vias_nm += len;
        sum.weighted += len * via_rate;
    }
    let mut out: Vec<NetLength> = Vec::new();
    for (net, sum) in sums {
        let name = board
            .nets
            .get(&net)
            .map_or_else(|| net.to_string(), |n| n.name.clone());
        let layers: Vec<LayerLength> = sum
            .by_layer
            .iter()
            .map(|(layer, (len, weighted))| LayerLength {
                layer: delays.layer_name(*layer),
                length_nm: to_i64(*len),
                delay_fs: to_i64(round_div(*weighted, 1_000_000)),
            })
            .collect();
        out.push(NetLength {
            net,
            name,
            tracks_nm: to_i64(sum.tracks_nm),
            vias: sum.vias,
            vias_nm: to_i64(sum.vias_nm),
            total_nm: to_i64(sum.tracks_nm + sum.vias_nm),
            delay_fs: to_i64(round_div(sum.weighted, 1_000_000)),
            arcs: sum.arcs,
            layers,
        });
    }
    out.sort_by(|a, b| (&a.name, a.net).cmp(&(&b.name, b.net)));
    let mut findings = delays.findings;
    let arcs: usize = out.iter().map(|n| n.arcs).sum();
    if arcs > 0 {
        findings.push(Finding::new(
            ARC_ESTIMATED,
            Severity::Info,
            None,
            format!("{arcs} arc pieces are measured with floating-point trigonometry"),
        ));
    }
    Ok((out, findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn division_is_rounded_to_the_nearest_with_halves_up() {
        assert_eq!(round_div(7, 2), 4);
        assert_eq!(round_div(6, 4), 2);
        assert_eq!(round_div(5, 4), 1);
        assert_eq!(round_div(0, 5), 0);
    }

    #[test]
    fn an_arc_is_longer_than_its_chord_and_a_flat_one_is_its_chord() {
        // a half turn of radius 5 mm: 5 mm times pi, 15 707 963 nm to the nearest nanometre
        let half_turn = arc_len((-5_000_000, 0), (5_000_000, 0), 180_000_000);
        assert_eq!(half_turn, 15_707_963);
        // a quarter turn of radius 10 mm: 10 mm times pi over 2, 15 707 963 nm
        let quarter = arc_len((10_000_000, 0), (0, 10_000_000), 90_000_000);
        assert!((quarter - 15_707_963).abs() <= 2, "{quarter}");
        assert_eq!(arc_len((0, 0), (1_000, 0), 0), 1_000);
        assert_eq!(arc_len((5, 5), (5, 5), 90_000_000), 0);
    }
}
