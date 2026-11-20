// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of the report panel, without a screen: what the document is made of (the layers, the nets, the
//! counts of the objects), how many problems the rules found by severity and how many connections are
//! still to be routed. The same document always gives the same text, with no date, no name of a file and
//! no name of an object, so the text can be copied into a message without telling anything about the design.

use eh3d_phoenix_rules::{Finding, Lang, Severity};

use crate::dock_text::DockMsg;
use crate::editmsg::EditMsg;
use crate::msg::Msg;
use crate::object_list::{counts, CountKind};
use crate::scene::{Scene, SceneKind};

/// The numbers of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Whether the document is a board or a schematic.
    pub kind: SceneKind,
    /// How many layers the scene has.
    pub layers: usize,
    /// How many nets the scene has.
    pub nets: usize,
    /// How many objects of each kind the scene has, for the kinds that it has.
    pub objects: Vec<(CountKind, usize)>,
    /// How many things could not be drawn.
    pub skipped: usize,
    /// The problems of the rules by severity (errors, warnings, information), when the rules were run.
    pub findings: Option<[usize; 3]>,
    /// The connections still to be routed, for a board.
    pub unrouted: Option<usize>,
}

impl Report {
    /// The report of a scene alone: no rules were run.
    pub fn of_scene(scene: &Scene) -> Report {
        Report {
            kind: scene.kind,
            layers: scene.layers.len(),
            nets: scene.nets.len(),
            objects: counts(scene),
            skipped: scene.skipped,
            findings: None,
            unrouted: None,
        }
    }

    /// The report with the problems that the rules found.
    pub fn with_findings(self, findings: &[Finding]) -> Report {
        self.with_severities(findings.iter().map(|f| f.severity))
    }

    /// The report with problems given by their severity (the findings of the electrical rule check have
    /// their own type, with the same severities).
    pub fn with_severities(mut self, severities: impl IntoIterator<Item = Severity>) -> Report {
        let mut by = [0usize; 3];
        for severity in severities {
            match severity {
                Severity::Error => by[0] += 1,
                Severity::Warning => by[1] += 1,
                Severity::Info => by[2] += 1,
            }
        }
        self.findings = Some(by);
        self
    }

    /// The report with the connections that are still to be routed.
    pub fn with_unrouted(mut self, unrouted: usize) -> Report {
        self.unrouted = Some(unrouted);
        self
    }

    /// The lines of the report in a language, each a label and a number: `Layers: 4`.
    pub fn lines(&self, lang: Lang) -> Vec<String> {
        let mut out = vec![
            format!("{}: {}", Msg::Layers.tr(lang), self.layers),
            format!("{}: {}", Msg::Nets.tr(lang), self.nets),
        ];
        for (kind, n) in &self.objects {
            out.push(format!("{}: {n}", kind.name(lang)));
        }
        if self.skipped > 0 {
            out.push(format!("{}: {}", Msg::Skipped.tr(lang), self.skipped));
        }
        if let Some([errors, warnings, information]) = self.findings {
            out.push(format!(
                "{}: {} {} · {} {} · {} {}",
                DockMsg::ReportFindings.tr(lang),
                errors,
                EditMsg::SevError.tr(lang),
                warnings,
                EditMsg::SevWarning.tr(lang),
                information,
                EditMsg::SevInfo.tr(lang),
            ));
        }
        if let Some(n) = self.unrouted {
            out.push(format!("{}: {n}", EditMsg::Unrouted.tr(lang)));
        }
        out
    }

    /// The report as text, one line for each line.
    pub fn text(&self, lang: Lang) -> String {
        let mut text = self.lines(lang).join("\n");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim::{Shape, P};
    use crate::sample;
    use crate::scene::ItemKind;

    fn finding(severity: Severity) -> Finding {
        Finding {
            rank: 1,
            rule: "copper_clearance",
            code: "PHOENIX-CHECK-005",
            severity,
            location: "R1 and R2".to_string(),
            at_nm: None,
            measured_nm: None,
            limit_nm: None,
            detail: String::new(),
        }
    }

    fn scene() -> Scene {
        let dot = Shape::Circle {
            centre: P::new(0.0, 0.0),
            radius: 0.3,
        };
        sample::scene(
            &["GND", "VCC"],
            &[
                (ItemKind::Designator, "R1 10k", None, dot.clone()),
                (ItemKind::Pad, "R1 pad 1", Some(0), dot.clone()),
                (ItemKind::Track, "track", Some(0), dot),
            ],
        )
    }

    #[test]
    fn the_report_of_a_scene_counts_what_is_in_it() {
        let r = Report::of_scene(&scene());
        assert_eq!(r.nets, 2);
        assert_eq!(
            r.objects,
            vec![
                (CountKind::Component, 1),
                (CountKind::Pad, 1),
                (CountKind::Track, 1)
            ]
        );
        assert_eq!(r.findings, None);
        assert_eq!(r.unrouted, None);
        let text = r.text(Lang::En);
        assert!(text.contains("Nets: 2\n"), "{text}");
        assert!(text.contains("Tracks: 1\n"), "{text}");
        assert!(!text.contains(&EditMsg::Unrouted.tr(Lang::En).to_string()));
    }

    #[test]
    fn the_problems_are_counted_by_severity() {
        let findings = [
            finding(Severity::Error),
            finding(Severity::Error),
            finding(Severity::Warning),
            finding(Severity::Info),
            finding(Severity::Info),
            finding(Severity::Info),
        ];
        let r = Report::of_scene(&scene()).with_findings(&findings);
        assert_eq!(r.findings, Some([2, 1, 3]));
        let r = Report::of_scene(&scene()).with_findings(&[]);
        assert_eq!(r.findings, Some([0, 0, 0]));
    }

    #[test]
    fn severities_alone_give_the_same_counts_as_findings() {
        let findings = [
            finding(Severity::Error),
            finding(Severity::Warning),
            finding(Severity::Warning),
        ];
        let a = Report::of_scene(&scene()).with_findings(&findings);
        let b = Report::of_scene(&scene()).with_severities(findings.iter().map(|f| f.severity));
        assert_eq!(a, b);
        assert_eq!(b.findings, Some([1, 2, 0]));
        let none = Report::of_scene(&scene()).with_severities(std::iter::empty());
        assert_eq!(none.findings, Some([0, 0, 0]));
    }

    #[test]
    fn the_unrouted_connections_are_a_line_of_the_report() {
        let r = Report::of_scene(&scene()).with_unrouted(7);
        assert!(r.lines(Lang::En).last().is_some_and(|l| l.ends_with(": 7")));
    }

    #[test]
    fn the_text_is_the_same_every_time_and_says_nothing_of_the_design() {
        let findings = [finding(Severity::Error)];
        let build = || {
            Report::of_scene(&scene())
                .with_findings(&findings)
                .with_unrouted(1)
        };
        for lang in Lang::ALL {
            let a = build().text(lang);
            assert_eq!(a, build().text(lang));
            // no reference designator, net name or place of a finding reaches the text
            for secret in ["R1", "GND", "VCC", "copper_clearance", "PHOENIX-CHECK-005"] {
                assert!(!a.contains(secret), "{lang:?}: {secret} in {a}");
            }
        }
    }

    #[test]
    fn every_line_exists_in_the_eight_languages_and_has_a_number() {
        let r = Report::of_scene(&scene())
            .with_findings(&[finding(Severity::Warning)])
            .with_unrouted(2);
        for lang in Lang::ALL {
            let lines = r.lines(lang);
            assert!(lines.len() >= 6);
            for l in lines {
                assert!(l.chars().any(|c| c.is_ascii_digit()), "{lang:?}: {l}");
            }
        }
    }

    #[test]
    fn a_scene_that_skipped_something_says_so() {
        let mut s = scene();
        s.skipped = 3;
        let mut quiet = scene();
        quiet.skipped = 0;
        let with = Report::of_scene(&s).lines(Lang::En);
        let without = Report::of_scene(&quiet).lines(Lang::En);
        assert_eq!(with.len(), without.len() + 1);
        assert!(with.iter().any(|l| l.ends_with(": 3")), "{with:?}");
    }
}
