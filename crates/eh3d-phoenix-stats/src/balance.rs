// SPDX-License-Identifier: GPL-3.0-or-later
//! The balance of the copper of a board. A board whose copper is not the same on the two sides of
//! the middle of the stack bends when it is heated, so a maker wants the layers that mirror each
//! other (the top and the bottom, the first inner and the last inner) to carry about the same
//! share of copper.
//!
//! The layers are paired from the outside in: the first with the last, the second with the one
//! before the last, and so on; the layer in the middle of an odd stack has no pair. The difference
//! of a pair is the difference of the coverage of its two layers, in hundredths of a percentage
//! point. A pair whose difference is above `balance_max_difference_x100` of the settings gives
//! `PHOENIX-STATS-008`, with the severity that `balance_severity` names (a warning by default).
//!
//! Without the order of the copper layers (`copper_stack`) the pairs follow the order of the layer
//! ids and the report says so (`PHOENIX-STATS-009`).

use serde::Serialize;

use crate::codes::{Finding, LAYER_UNBALANCED, STACK_ORDER_UNKNOWN};
use crate::report::hundredths;
use crate::settings::StatsSettings;
use crate::stats::LayerStats;

/// Two layers that mirror each other and the difference of their copper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BalancePair {
    /// The name of the outer layer of the pair.
    pub layer_a: String,
    /// The name of the inner layer of the pair.
    pub layer_b: String,
    /// The coverage of the first, in hundredths of a percent.
    pub coverage_a_x100: i64,
    /// The coverage of the second, in hundredths of a percent.
    pub coverage_b_x100: i64,
    /// The difference of the two, in hundredths of a percentage point.
    pub difference_x100: i64,
    /// The largest difference the settings allow, in hundredths of a percentage point.
    pub limit_x100: i64,
    /// True when the difference is within the limit.
    pub balanced: bool,
}

/// The pairs of a stack of layers and what is wrong with them. `ordered` is true when the board
/// gives the order of its copper layers. Nothing is judged when the check is off or when there
/// are fewer than two layers.
pub(crate) fn judge(
    layers: &[LayerStats],
    ordered: bool,
    settings: &StatsSettings,
) -> (Vec<BalancePair>, Vec<Finding>) {
    let mut pairs = Vec::new();
    let mut findings = Vec::new();
    if !settings.balance_check || layers.len() < 2 {
        return (pairs, findings);
    }
    if !ordered {
        findings.push(Finding::new(
            STACK_ORDER_UNKNOWN,
            crate::codes::Severity::Info,
            None,
            format!("{} layers paired in the order of their ids", layers.len()),
        ));
    }
    let limit = settings.balance_max_difference_x100;
    for i in 0..layers.len() / 2 {
        let a = &layers[i];
        let b = &layers[layers.len() - 1 - i];
        let difference = (a.coverage_x100 - b.coverage_x100).abs();
        let balanced = difference <= limit;
        if !balanced {
            findings.push(Finding::new(
                LAYER_UNBALANCED,
                settings.balance_severity,
                Some(&format!("{} / {}", a.layer, b.layer)),
                format!(
                    "coverage {} % against {} %, difference {} points, limit {}",
                    hundredths(a.coverage_x100),
                    hundredths(b.coverage_x100),
                    hundredths(difference),
                    hundredths(limit)
                ),
            ));
        }
        pairs.push(BalancePair {
            layer_a: a.layer.clone(),
            layer_b: b.layer.clone(),
            coverage_a_x100: a.coverage_x100,
            coverage_b_x100: b.coverage_x100,
            difference_x100: difference,
            limit_x100: limit,
            balanced,
        });
    }
    (pairs, findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::Severity;

    fn layer(name: &str, coverage_x100: i64) -> LayerStats {
        LayerStats {
            layer: name.into(),
            position: 0,
            pads: 0,
            tracks: 0,
            track_length_nm: 0,
            vias: 0,
            zones: 0,
            pads_um2: 0,
            tracks_um2: 0,
            vias_um2: 0,
            zones_um2: 0,
            copper_um2: 0,
            coverage_x100,
        }
    }

    #[test]
    fn the_layers_are_paired_from_the_outside_in() {
        let stack = [
            layer("Top", 4000),
            layer("In1", 8000),
            layer("In2", 7000),
            layer("Bottom", 3000),
        ];
        let (pairs, findings) = judge(&stack, true, &StatsSettings::default());
        let names: Vec<(&str, &str)> = pairs
            .iter()
            .map(|p| (p.layer_a.as_str(), p.layer_b.as_str()))
            .collect();
        assert_eq!(names, vec![("Top", "Bottom"), ("In1", "In2")]);
        assert_eq!(pairs[0].difference_x100, 1000);
        assert_eq!(pairs[1].difference_x100, 1000);
        assert!(pairs.iter().all(|p| p.balanced));
        assert!(findings.is_empty());
    }

    #[test]
    fn the_layer_in_the_middle_of_an_odd_stack_has_no_pair() {
        let stack = [layer("Top", 100), layer("Mid", 9000), layer("Bottom", 100)];
        let (pairs, _) = judge(&stack, true, &StatsSettings::default());
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].layer_b, "Bottom");
    }

    #[test]
    fn a_pair_above_the_limit_is_reported_with_the_severity_of_the_settings() {
        let stack = [layer("Top", 500), layer("Bottom", 9000)];
        let (pairs, findings) = judge(&stack, true, &StatsSettings::default());
        assert!(!pairs[0].balanced);
        assert_eq!(pairs[0].difference_x100, 8500);
        let [f] = findings.as_slice() else {
            panic!("{findings:?}");
        };
        assert_eq!(f.code, LAYER_UNBALANCED);
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.subject.as_deref(), Some("Top / Bottom"));
        assert!(f.detail.contains("5.00 % against 90.00 %"), "{}", f.detail);
        assert!(f.detail.contains("difference 85.00 points"), "{}", f.detail);
        let strict = StatsSettings {
            balance_severity: Severity::Error,
            ..StatsSettings::default()
        };
        assert_eq!(judge(&stack, true, &strict).1[0].severity, Severity::Error);
    }

    #[test]
    fn a_difference_equal_to_the_limit_is_balanced() {
        let stack = [layer("Top", 1000), layer("Bottom", 2500)];
        let (pairs, findings) = judge(&stack, true, &StatsSettings::default());
        assert_eq!(pairs[0].difference_x100, 1500);
        assert!(pairs[0].balanced && findings.is_empty());
    }

    #[test]
    fn the_check_can_be_switched_off_and_needs_two_layers() {
        let stack = [layer("Top", 0), layer("Bottom", 9000)];
        let off = StatsSettings {
            balance_check: false,
            ..StatsSettings::default()
        };
        let (pairs, findings) = judge(&stack, true, &off);
        assert!(pairs.is_empty() && findings.is_empty());
        let (pairs, findings) = judge(&stack[..1], true, &StatsSettings::default());
        assert!(pairs.is_empty() && findings.is_empty());
    }

    #[test]
    fn a_stack_without_an_order_says_how_the_layers_were_paired() {
        let stack = [layer("A", 100), layer("B", 100)];
        let (pairs, findings) = judge(&stack, false, &StatsSettings::default());
        assert_eq!(pairs.len(), 1);
        let [f] = findings.as_slice() else {
            panic!("{findings:?}");
        };
        assert_eq!(f.code, STACK_ORDER_UNKNOWN);
        assert_eq!(f.severity, Severity::Info);
    }
}
