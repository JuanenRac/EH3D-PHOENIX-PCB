//! The rule check: sizes and clearances against a rule set read from a plain text file.
//!
//! The rule set is data, not code: a file of `name = value` lines (values in millimetres, `#`
//! starts a comment) that the program reads from its configuration. The checks run in a fixed
//! order (track widths, vias, pads, clearances), so the same board and rules always give the same
//! list, and the report keeps two lists apart:
//!
//! * `confirmed`: a rule that is broken, proven with exact integer arithmetic;
//! * `inconclusive`: what this check cannot judge (the clearance of a track with an arc, or numbers
//!   too large for the exact test). Nothing in this list is a failure and nothing is hidden in it.
//!
//! Clearances are checked between straight track pieces of different nets on one layer, and
//! between those pieces and the vias of other nets that reach the layer. Pads, zones and arcs are
//! not part of the clearance check yet.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{segments_intersect, ModelError, Nm};

use crate::connectivity::near_segment_exact;
use crate::model::{Board, PadShape, Segment};

/// A line of the rule file that is not understood.
pub const RULE_LINE_INVALID: &str = "PHOENIX-RULE-001";
/// A track narrower than the minimum width.
pub const TRACK_TOO_NARROW: &str = "PHOENIX-RULE-002";
/// A via or pad drill smaller than the minimum, or a via smaller than its minimum diameter.
pub const DRILL_OR_VIA_TOO_SMALL: &str = "PHOENIX-RULE-003";
/// An annular ring thinner than the minimum.
pub const RING_TOO_THIN: &str = "PHOENIX-RULE-004";
/// Copper of different nets closer than the minimum clearance.
pub const CLEARANCE_VIOLATED: &str = "PHOENIX-RULE-005";
/// A clearance that could not be judged.
pub const CLEARANCE_INCONCLUSIVE: &str = "PHOENIX-RULE-006";
/// Copper closer to the edge of the board than the minimum.
pub const EDGE_TOO_CLOSE: &str = "PHOENIX-RULE-007";
/// Less solder mask between two pad openings than the minimum.
pub const MASK_BRIDGE_THIN: &str = "PHOENIX-RULE-008";
/// A pad whose shape is too coarse to be judged against a limit.
pub const PAD_SHAPE_UNSURE: &str = "PHOENIX-RULE-009";
/// A rule that asks for something that the board does not say well enough to judge.
pub const NOT_CHECKABLE: &str = "PHOENIX-RULE-010";
/// A zone whose own clearance is smaller than the minimum.
pub const ZONE_CLEARANCE_SMALL: &str = "PHOENIX-RULE-011";
/// Zones of different nets on one layer that overlap.
pub const ZONES_OVERLAP: &str = "PHOENIX-RULE-012";
/// Silkscreen printed over a pad that shows on the same side.
pub const SILK_OVER_PAD: &str = "PHOENIX-RULE-013";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (RULE_LINE_INVALID, "Rule file line not understood"),
    (TRACK_TOO_NARROW, "Track narrower than the minimum"),
    (
        DRILL_OR_VIA_TOO_SMALL,
        "Drill or via smaller than the minimum",
    ),
    (RING_TOO_THIN, "Annular ring thinner than the minimum"),
    (CLEARANCE_VIOLATED, "Clearance violated"),
    (CLEARANCE_INCONCLUSIVE, "Clearance could not be judged"),
    (EDGE_TOO_CLOSE, "Copper too close to the edge of the board"),
    (MASK_BRIDGE_THIN, "Solder mask bridge too thin"),
    (PAD_SHAPE_UNSURE, "Pad shape too coarse to judge"),
    (NOT_CHECKABLE, "Rule asks for what the board does not say"),
    (
        ZONE_CLEARANCE_SMALL,
        "Zone clearance smaller than the minimum",
    ),
    (ZONES_OVERLAP, "Zones of different nets overlap"),
    (SILK_OVER_PAD, "Silkscreen printed over a pad"),
];

/// The numbers a board is checked against. A value of zero turns its rule off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSet {
    /// Narrowest track.
    pub min_track_width: Nm,
    /// Closest approach of copper of different nets, edge to edge.
    pub min_clearance: Nm,
    /// Smallest drill of a via or a pad.
    pub min_drill: Nm,
    /// Smallest via pad diameter.
    pub min_via_diameter: Nm,
    /// Thinnest ring of copper around a drill.
    pub min_annular_ring: Nm,
    /// Closest approach of copper to the edge of the board.
    pub min_edge_clearance: Nm,
    /// Least solder mask between two pad openings.
    pub min_mask_bridge: Nm,
    /// Ask for the check of silkscreen over pads.
    pub silkscreen_over_pads: bool,
}

impl Default for RuleSet {
    /// Every rule off: a board without rules has nothing to break.
    fn default() -> Self {
        RuleSet {
            min_track_width: Nm::ZERO,
            min_clearance: Nm::ZERO,
            min_drill: Nm::ZERO,
            min_via_diameter: Nm::ZERO,
            min_annular_ring: Nm::ZERO,
            min_edge_clearance: Nm::ZERO,
            min_mask_bridge: Nm::ZERO,
            silkscreen_over_pads: false,
        }
    }
}

impl RuleSet {
    /// Reads a rule file. Lines that are not understood become diagnostics and the rest is used.
    pub fn parse(text: &str) -> (RuleSet, Vec<Diagnostic>) {
        let mut rules = RuleSet::default();
        let mut diagnostics = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let bad = |why: &str| {
                make(
                    RULE_LINE_INVALID,
                    Severity::Error,
                    format!("line {}: {why}", index + 1),
                )
            };
            let Some((key, value)) = line.split_once('=') else {
                diagnostics.push(bad("expected name = value"));
                continue;
            };
            if key.trim() == "silkscreen_over_pads" {
                match value.trim() {
                    "on" => rules.silkscreen_over_pads = true,
                    "off" => rules.silkscreen_over_pads = false,
                    _ => diagnostics.push(bad("the value is not on or off")),
                }
                continue;
            }
            let value = match Nm::parse_mm(value.trim()) {
                Ok((nm, _exact)) if nm.get() >= 0 => nm,
                Ok(_) => {
                    diagnostics.push(bad("the value is negative"));
                    continue;
                }
                Err(e) => {
                    let ModelError::InvalidDecimal { .. } = e else {
                        diagnostics.push(bad("the value is out of range"));
                        continue;
                    };
                    diagnostics.push(bad("the value is not a number in millimetres"));
                    continue;
                }
            };
            match key.trim() {
                "min_track_width" => rules.min_track_width = value,
                "min_clearance" => rules.min_clearance = value,
                "min_drill" => rules.min_drill = value,
                "min_via_diameter" => rules.min_via_diameter = value,
                "min_annular_ring" => rules.min_annular_ring = value,
                "min_edge_clearance" => rules.min_edge_clearance = value,
                "min_mask_bridge" => rules.min_mask_bridge = value,
                _ => diagnostics.push(bad("unknown rule name")),
            }
        }
        (rules, diagnostics)
    }
}

/// The two lists of a rule check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleReport {
    /// Rules that are broken, proven.
    pub confirmed: Vec<Diagnostic>,
    /// What could not be judged.
    pub inconclusive: Vec<Diagnostic>,
}

impl RuleReport {
    /// True when nothing is broken. It says nothing about the inconclusive list.
    pub fn passes(&self) -> bool {
        self.confirmed.is_empty()
    }
}

fn title(code: &str) -> &'static str {
    REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t)
}

pub(crate) fn make(code: &'static str, severity: Severity, detail: String) -> Diagnostic {
    Diagnostic::new(0, severity, format!("{}: {detail}", title(code)))
        .with_code(code)
        .with_category(Category::Geometric)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

fn is_off(value: Nm) -> bool {
    value.get() == 0
}

impl Board {
    /// Checks the board against `rules`, in a fixed order.
    pub fn check_rules(&self, rules: &RuleSet) -> RuleReport {
        let mut report = RuleReport::default();

        if !is_off(rules.min_track_width) {
            for (id, t) in &self.tracks {
                if t.width < rules.min_track_width {
                    report.confirmed.push(make(
                        TRACK_TOO_NARROW,
                        Severity::Error,
                        format!(
                            "track {id}: {} nm, minimum {} nm",
                            t.width.get(),
                            rules.min_track_width.get()
                        ),
                    ));
                }
            }
        }

        for (id, v) in &self.vias {
            if !is_off(rules.min_via_diameter) && v.diameter < rules.min_via_diameter {
                report.confirmed.push(make(
                    DRILL_OR_VIA_TOO_SMALL,
                    Severity::Error,
                    format!("via {id}: diameter {} nm", v.diameter.get()),
                ));
            }
            if !is_off(rules.min_drill) && v.drill < rules.min_drill {
                report.confirmed.push(make(
                    DRILL_OR_VIA_TOO_SMALL,
                    Severity::Error,
                    format!("via {id}: drill {} nm", v.drill.get()),
                ));
            }
            if !is_off(rules.min_annular_ring) {
                let ring2 = v.diameter.get() - v.drill.get();
                if ring2 < 2 * rules.min_annular_ring.get() {
                    report.confirmed.push(make(
                        RING_TOO_THIN,
                        Severity::Error,
                        format!("via {id}: ring {} nm on each side", ring2 / 2),
                    ));
                }
            }
        }

        for (fid, fp) in &self.footprints {
            for (pid, pad) in &fp.pads {
                let Some(drill) = pad.drill else {
                    continue;
                };
                if !is_off(rules.min_drill) && drill < rules.min_drill {
                    report.confirmed.push(make(
                        DRILL_OR_VIA_TOO_SMALL,
                        Severity::Error,
                        format!("pad {pid} of footprint {fid}: drill {} nm", drill.get()),
                    ));
                }
                if !is_off(rules.min_annular_ring) {
                    let size = match pad.shape {
                        PadShape::Circle { diameter } => diameter.get(),
                        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
                            width.get().min(height.get())
                        }
                    };
                    if size - drill.get() < 2 * rules.min_annular_ring.get() {
                        report.confirmed.push(make(
                            RING_TOO_THIN,
                            Severity::Error,
                            format!(
                                "pad {pid} of footprint {fid}: ring {} nm on each side",
                                (size - drill.get()) / 2
                            ),
                        ));
                    }
                }
            }
        }

        if !is_off(rules.min_clearance) {
            self.check_clearances(rules, &mut report);
        }
        self.check_more_rules(rules, &mut report);
        report
    }

    fn check_clearances(&self, rules: &RuleSet, report: &mut RuleReport) {
        let c = i128::from(rules.min_clearance.get());
        let tracks: Vec<_> = self.tracks.iter().collect();
        for i in 0..tracks.len() {
            let (ida, a) = tracks[i];
            if a.path.iter().any(|s| matches!(s, Segment::Arc { .. })) {
                report.inconclusive.push(make(
                    CLEARANCE_INCONCLUSIVE,
                    Severity::Info,
                    format!("track {ida} has an arc"),
                ));
            }
            for (idb, b) in tracks.iter().skip(i + 1).copied() {
                if a.net == b.net || a.layer != b.layer {
                    continue;
                }
                // the centre lines must stay at least c + (wa + wb) / 2 apart; times two:
                let t = 2 * c + i128::from(a.width.get()) + i128::from(b.width.get());
                let mut unknown = false;
                let mut broken = false;
                for sa in a.path.iter().filter(|s| matches!(s, Segment::Line { .. })) {
                    for sb in b.path.iter().filter(|s| matches!(s, Segment::Line { .. })) {
                        if segments_intersect(sa.start(), sa.end(), sb.start(), sb.end()) {
                            broken = true;
                            continue;
                        }
                        for (p, s, e) in [
                            (sa.start(), sb.start(), sb.end()),
                            (sa.end(), sb.start(), sb.end()),
                            (sb.start(), sa.start(), sa.end()),
                            (sb.end(), sa.start(), sa.end()),
                        ] {
                            match near_segment_exact(p, s, e, t * t, true) {
                                Some(true) => broken = true,
                                Some(false) => {}
                                None => unknown = true,
                            }
                        }
                    }
                }
                if broken {
                    report.confirmed.push(make(
                        CLEARANCE_VIOLATED,
                        Severity::Error,
                        format!("tracks {ida} and {idb}"),
                    ));
                } else if unknown {
                    report.inconclusive.push(make(
                        CLEARANCE_INCONCLUSIVE,
                        Severity::Info,
                        format!("tracks {ida} and {idb}: numbers too large for the exact test"),
                    ));
                }
            }
            for (idv, v) in &self.vias {
                if a.net == v.net || (v.from_layer != a.layer && v.to_layer != a.layer) {
                    continue;
                }
                let t = 2 * c + i128::from(a.width.get()) + i128::from(v.diameter.get());
                let mut broken = false;
                let mut unknown = false;
                for s in a.path.iter().filter(|s| matches!(s, Segment::Line { .. })) {
                    match near_segment_exact(v.at, s.start(), s.end(), t * t, true) {
                        Some(true) => broken = true,
                        Some(false) => {}
                        None => unknown = true,
                    }
                }
                if broken {
                    report.confirmed.push(make(
                        CLEARANCE_VIOLATED,
                        Severity::Error,
                        format!("track {ida} and via {idv}"),
                    ));
                } else if unknown {
                    report.inconclusive.push(make(
                        CLEARANCE_INCONCLUSIVE,
                        Severity::Info,
                        format!("track {ida} and via {idv}: numbers too large for the exact test"),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Id, Layer, LayerKind, Net, Track, Via};
    use eh3d_phoenix_model::{ObjectId, Point};

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }
    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }
    fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
        Segment::Line {
            start: pt(a.0, a.1),
            end: pt(b.0, b.1),
        }
    }

    fn two_nets() -> (Board, Id, Id, Id) {
        let (top, n1, n2) = (id(1), id(2), id(3));
        let mut b = Board::default();
        b.layers.insert(
            top,
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.nets.insert(n1, Net { name: "A".into() });
        b.nets.insert(n2, Net { name: "B".into() });
        (b, top, n1, n2)
    }

    fn rules(text: &str) -> RuleSet {
        let (r, d) = RuleSet::parse(text);
        assert!(d.is_empty(), "{d:?}");
        r
    }

    #[test]
    fn the_rule_file_is_read_in_millimetres_and_bad_lines_are_reported() {
        let r = rules("# comment\nmin_track_width = 0.15\nmin_clearance=0.2 # inline\nmin_drill = 0.3\n\nmin_via_diameter = 0.6\nmin_annular_ring = 0.125\n");
        assert_eq!(r.min_track_width.get(), 150_000);
        assert_eq!(r.min_clearance.get(), 200_000);
        assert_eq!(r.min_annular_ring.get(), 125_000);
        let (_, d) =
            RuleSet::parse("min_track_width\nmin_drill = abc\nspeed = 1\nmin_clearance = -1\n");
        assert_eq!(d.len(), 4);
        assert!(d
            .iter()
            .all(|x| x.code.as_deref() == Some(RULE_LINE_INVALID)));
        assert_eq!(RuleSet::default().min_clearance, Nm::ZERO);
    }

    #[test]
    fn sizes_are_checked_against_the_minimums() {
        let (mut b, top, n1, _) = two_nets();
        b.tracks.insert(
            id(10),
            Track {
                net: n1,
                layer: top,
                width: nm(100_000),
                path: vec![line((0, 0), (1_000_000, 0))],
            },
        );
        b.vias.insert(
            id(11),
            Via {
                net: n1,
                at: pt(0, 0),
                drill: nm(250_000),
                diameter: nm(500_000),
                from_layer: top,
                to_layer: top,
            },
        );
        let rep = b.check_rules(&rules(
            "min_track_width = 0.15\nmin_drill = 0.3\nmin_via_diameter = 0.6\nmin_annular_ring = 0.2\n",
        ));
        let codes: Vec<&str> = rep
            .confirmed
            .iter()
            .filter_map(|d| d.code.as_deref())
            .collect();
        assert_eq!(
            codes,
            vec![
                TRACK_TOO_NARROW,
                DRILL_OR_VIA_TOO_SMALL,
                DRILL_OR_VIA_TOO_SMALL,
                RING_TOO_THIN
            ]
        );
        assert!(!rep.passes());
        assert!(b.check_rules(&RuleSet::default()).passes());
    }

    #[test]
    fn clearance_is_edge_to_edge_and_only_between_different_nets_on_one_layer() {
        let (mut b, top, n1, n2) = two_nets();
        // two parallel tracks 0.25 mm wide, centres 0.45 mm apart: edges 0.20 mm apart
        let t = |net, y| Track {
            net,
            layer: top,
            width: nm(250_000),
            path: vec![line((0, y), (10_000_000, y))],
        };
        b.tracks.insert(id(10), t(n1, 0));
        b.tracks.insert(id(11), t(n2, 450_000));
        let ok = b.check_rules(&rules("min_clearance = 0.2\n"));
        assert!(ok.passes(), "{:?}", ok.confirmed);
        let bad = b.check_rules(&rules("min_clearance = 0.21\n"));
        assert_eq!(bad.confirmed.len(), 1);
        assert_eq!(bad.confirmed[0].code.as_deref(), Some(CLEARANCE_VIOLATED));
        // the same net may touch itself
        b.tracks.get_mut(&id(11)).unwrap().net = n1;
        assert!(b.check_rules(&rules("min_clearance = 1\n")).passes());
    }

    #[test]
    fn crossing_tracks_of_different_nets_break_the_clearance() {
        let (mut b, top, n1, n2) = two_nets();
        b.tracks.insert(
            id(10),
            Track {
                net: n1,
                layer: top,
                width: nm(200_000),
                path: vec![line((0, 0), (10_000_000, 10_000_000))],
            },
        );
        b.tracks.insert(
            id(11),
            Track {
                net: n2,
                layer: top,
                width: nm(200_000),
                path: vec![line((0, 10_000_000), (10_000_000, 0))],
            },
        );
        assert_eq!(
            b.check_rules(&rules("min_clearance = 0.1\n"))
                .confirmed
                .len(),
            1
        );
    }

    #[test]
    fn a_via_too_close_to_a_track_of_another_net_is_found() {
        let (mut b, top, n1, n2) = two_nets();
        b.tracks.insert(
            id(10),
            Track {
                net: n1,
                layer: top,
                width: nm(200_000),
                path: vec![line((0, 0), (10_000_000, 0))],
            },
        );
        b.vias.insert(
            id(11),
            Via {
                net: n2,
                at: pt(5_000_000, 500_000),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: top,
            },
        );
        // edges: 0.5 - 0.1 - 0.3 = 0.1 mm apart
        assert!(b.check_rules(&rules("min_clearance = 0.1\n")).passes());
        assert_eq!(
            b.check_rules(&rules("min_clearance = 0.11\n"))
                .confirmed
                .len(),
            1
        );
    }

    #[test]
    fn arcs_make_the_clearance_inconclusive_and_not_a_failure() {
        let (mut b, top, n1, n2) = two_nets();
        b.tracks.insert(
            id(10),
            Track {
                net: n1,
                layer: top,
                width: nm(200_000),
                path: vec![Segment::Arc {
                    start: pt(0, 0),
                    end: pt(2_000_000, 0),
                    sweep: eh3d_phoenix_model::Udeg::new(90_000_000).unwrap(),
                }],
            },
        );
        b.tracks.insert(
            id(11),
            Track {
                net: n2,
                layer: top,
                width: nm(200_000),
                path: vec![line((0, 100_000), (2_000_000, 100_000))],
            },
        );
        let rep = b.check_rules(&rules("min_clearance = 0.5\n"));
        assert!(rep.passes());
        assert!(!rep.inconclusive.is_empty());
        assert_eq!(rep.inconclusive[0].severity, Severity::Info);
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
