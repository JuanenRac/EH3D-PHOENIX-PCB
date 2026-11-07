// SPDX-License-Identifier: GPL-3.0-or-later
//! The panel of the problems found by the design rules, without a screen: which of them are shown
//! (by severity and by category), in what order, which one is the current one and the step to the
//! next or the previous one that has a place on the board. The categories are the ones of the
//! reports of the checks (`eh3d_phoenix_rules::report_export`), so the panel, the CSV and the page
//! of HTML group the codes in the same way. The window draws this state in `drc_gui`, behind the
//! `gui` feature, and `Workspace` goes to the place of the current problem.

use std::collections::BTreeSet;

use eh3d_phoenix_rules::report_export::{category, CATEGORIES};
use eh3d_phoenix_rules::{Finding, Severity};

use crate::settings::ViewerSettings;

/// How the list of problems is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrcSort {
    /// Errors first, then warnings, then information; inside each, as the check found them.
    #[default]
    Severity,
    /// By category, in the order of the reports; inside each, as the check found them.
    Category,
    /// As the check found them.
    Found,
}

impl DrcSort {
    /// Every order, in the order the window lists them.
    pub const ALL: [DrcSort; 3] = [DrcSort::Severity, DrcSort::Category, DrcSort::Found];

    /// The word of the settings file.
    pub fn key(self) -> &'static str {
        match self {
            DrcSort::Severity => "severity",
            DrcSort::Category => "category",
            DrcSort::Found => "found",
        }
    }

    /// The order a word of the settings file names.
    pub fn from_key(text: &str) -> Option<DrcSort> {
        DrcSort::ALL.into_iter().find(|s| s.key() == text)
    }
}

/// Which problems are shown: the severities that are on and the categories that are hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrcFilter {
    /// Whether the errors are shown.
    pub errors: bool,
    /// Whether the warnings are shown.
    pub warnings: bool,
    /// Whether the information is shown.
    pub information: bool,
    /// The categories (one of `CATEGORIES`) that are hidden.
    pub hidden: BTreeSet<String>,
}

impl Default for DrcFilter {
    fn default() -> Self {
        DrcFilter {
            errors: true,
            warnings: true,
            information: true,
            hidden: BTreeSet::new(),
        }
    }
}

impl DrcFilter {
    /// Whether a severity is shown.
    pub fn severity_shown(&self, severity: Severity) -> bool {
        match severity {
            Severity::Error => self.errors,
            Severity::Warning => self.warnings,
            Severity::Info => self.information,
        }
    }

    /// Whether a category is shown.
    pub fn category_shown(&self, name: &str) -> bool {
        !self.hidden.contains(name)
    }

    /// Whether a problem is shown: its severity is on and its category is not hidden.
    pub fn shows(&self, finding: &Finding) -> bool {
        self.severity_shown(finding.severity) && self.category_shown(category(finding.code))
    }

    /// Hides a category that is shown and shows one that is hidden; a name that is not a category
    /// changes nothing.
    pub fn toggle_category(&mut self, name: &str) {
        if !CATEGORIES.contains(&name) {
            return;
        }
        if !self.hidden.remove(name) {
            self.hidden.insert(name.to_owned());
        }
    }

    /// Hides every category but one; a name that is not a category changes nothing.
    pub fn only_category(&mut self, name: &str) {
        if !CATEGORIES.contains(&name) {
            return;
        }
        self.hidden = CATEGORIES
            .iter()
            .filter(|c| **c != name)
            .map(|c| (*c).to_owned())
            .collect();
    }

    /// Turns every severity on and shows every category.
    pub fn show_all(&mut self) {
        *self = DrcFilter::default();
    }

    /// Whether nothing is filtered out.
    pub fn is_all(&self) -> bool {
        *self == DrcFilter::default()
    }
}

/// What identifies a problem from one check to the next: its code, what it is about and where it
/// is. The current problem is kept by this key, so it stays the current one when the list is made
/// again after an edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingKey {
    /// The code of the finding.
    pub code: &'static str,
    /// What the finding is about.
    pub location: String,
    /// Where it is, in nanometres.
    pub at_nm: Option<[i64; 2]>,
}

impl FindingKey {
    /// The key of a finding.
    pub fn of(finding: &Finding) -> FindingKey {
        FindingKey {
            code: finding.code,
            location: finding.location.clone(),
            at_nm: finding.at_nm,
        }
    }

    /// Whether a finding has this key.
    pub fn matches(&self, finding: &Finding) -> bool {
        self.code == finding.code
            && self.at_nm == finding.at_nm
            && self.location == finding.location
    }
}

/// The problems of one category, by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryCount {
    /// The category, one of `CATEGORIES`.
    pub name: &'static str,
    /// How many errors.
    pub errors: usize,
    /// How many warnings.
    pub warnings: usize,
    /// How many items of information.
    pub information: usize,
}

impl CategoryCount {
    /// All the problems of the category.
    pub fn total(&self) -> usize {
        self.errors + self.warnings + self.information
    }
}

/// The state of the panel of problems.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrcPanel {
    /// Which problems are shown.
    pub filter: DrcFilter,
    /// The order of the list.
    pub sort: DrcSort,
    /// Whether the step past the last problem goes to the first, and the other way round.
    pub wrap: bool,
    /// The problem that was gone to last.
    pub current: Option<FindingKey>,
}

/// The place of a category in the order of the reports.
fn category_rank(code: &str) -> usize {
    let name = category(code);
    CATEGORIES
        .iter()
        .position(|c| *c == name)
        .unwrap_or(CATEGORIES.len())
}

impl DrcPanel {
    /// The panel as the settings file says it starts (`drc_*` of `config/viewer.cfg`).
    pub fn from_settings(settings: &ViewerSettings) -> DrcPanel {
        DrcPanel {
            filter: DrcFilter {
                errors: settings.drc_show_errors,
                warnings: settings.drc_show_warnings,
                information: settings.drc_show_information,
                hidden: settings.drc_hidden_categories.iter().cloned().collect(),
            },
            sort: settings.drc_sort,
            wrap: settings.drc_wrap,
            current: None,
        }
    }

    /// The problems that pass the filter, in the order chosen. The sorts are stable, so problems
    /// that rank the same keep the order in which the check found them.
    pub fn visible<'a>(&self, findings: &'a [Finding]) -> Vec<&'a Finding> {
        let mut list: Vec<&Finding> = findings.iter().filter(|f| self.filter.shows(f)).collect();
        match self.sort {
            DrcSort::Found => {}
            DrcSort::Severity => list.sort_by_key(|f| std::cmp::Reverse(f.severity)),
            DrcSort::Category => list.sort_by_key(|f| category_rank(f.code)),
        }
        list
    }

    /// The problems of each category that has any, by severity, whatever the filter says, in the
    /// order of the reports.
    pub fn counts(findings: &[Finding]) -> Vec<CategoryCount> {
        CATEGORIES
            .iter()
            .filter_map(|&name| {
                let mut count = CategoryCount {
                    name,
                    errors: 0,
                    warnings: 0,
                    information: 0,
                };
                for f in findings.iter().filter(|f| category(f.code) == name) {
                    match f.severity {
                        Severity::Error => count.errors += 1,
                        Severity::Warning => count.warnings += 1,
                        Severity::Info => count.information += 1,
                    }
                }
                (count.total() > 0).then_some(count)
            })
            .collect()
    }

    /// Whether a finding is the current problem.
    pub fn is_current(&self, finding: &Finding) -> bool {
        self.current.as_ref().is_some_and(|k| k.matches(finding))
    }

    /// Makes a finding the current problem.
    pub fn select(&mut self, finding: &Finding) {
        self.current = Some(FindingKey::of(finding));
    }

    /// Steps to the next problem (or the previous one) of the list that is shown and has a place
    /// on the board, makes it the current one and gives its place in nanometres. With no current
    /// problem in the list it starts at the first (or the last). At the end of the list it goes
    /// round when `wrap` is on and stays where it is when it is off. `None` when no problem shown
    /// has a place.
    pub fn step(&mut self, findings: &[Finding], forward: bool) -> Option<[i64; 2]> {
        let placed: Vec<&Finding> = self
            .visible(findings)
            .into_iter()
            .filter(|f| f.at_nm.is_some())
            .collect();
        let n = placed.len();
        if n == 0 {
            return None;
        }
        let at = self
            .current
            .as_ref()
            .and_then(|k| placed.iter().position(|f| k.matches(f)));
        let target = match (at, forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) if i + 1 < n => i + 1,
            (Some(_), true) if self.wrap => 0,
            (Some(i), false) if i > 0 => i - 1,
            (Some(_), false) if self.wrap => n - 1,
            (Some(i), _) => i,
        };
        let finding = placed[target];
        self.current = Some(FindingKey::of(finding));
        finding.at_nm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rules::codes;

    fn finding(
        code: &'static str,
        severity: Severity,
        name: &str,
        at: Option<[i64; 2]>,
    ) -> Finding {
        Finding {
            rank: 0,
            rule: "x",
            code,
            severity,
            location: name.to_owned(),
            at_nm: at,
            measured_nm: None,
            limit_nm: None,
            detail: String::new(),
        }
    }

    /// Six problems of four categories and the three severities, one of them with no place.
    fn sample() -> Vec<Finding> {
        vec![
            finding(codes::CLEARANCE, Severity::Error, "a", Some([1, 1])),
            finding(codes::TRACK_NARROW, Severity::Warning, "b", Some([2, 2])),
            finding(codes::SILK_OVER_PAD, Severity::Info, "c", Some([3, 3])),
            finding(codes::CLEARANCE, Severity::Warning, "d", None),
            finding(codes::EDGE_CLOSE, Severity::Error, "e", Some([5, 5])),
            finding(codes::TRACK_NARROW, Severity::Error, "f", Some([6, 6])),
        ]
    }

    fn names<'a>(list: &[&'a Finding]) -> Vec<&'a str> {
        list.iter().map(|f| f.location.as_str()).collect()
    }

    #[test]
    fn the_sorts_are_by_severity_by_category_or_as_found_and_keep_the_order_of_ties() {
        let findings = sample();
        let mut panel = DrcPanel::default();
        assert_eq!(panel.sort, DrcSort::Severity);
        assert_eq!(
            names(&panel.visible(&findings)),
            ["a", "e", "f", "b", "d", "c"]
        );
        panel.sort = DrcSort::Category;
        // size (the two narrow tracks), clearance, edge, mask and silk
        assert_eq!(
            names(&panel.visible(&findings)),
            ["b", "f", "a", "d", "e", "c"]
        );
        panel.sort = DrcSort::Found;
        assert_eq!(
            names(&panel.visible(&findings)),
            ["a", "b", "c", "d", "e", "f"]
        );
    }

    #[test]
    fn the_filter_hides_by_severity_and_by_category() {
        let findings = sample();
        let mut panel = DrcPanel {
            sort: DrcSort::Found,
            ..DrcPanel::default()
        };
        panel.filter.warnings = false;
        assert_eq!(names(&panel.visible(&findings)), ["a", "c", "e", "f"]);
        panel.filter.information = false;
        assert_eq!(names(&panel.visible(&findings)), ["a", "e", "f"]);
        panel.filter.toggle_category("clearance");
        assert_eq!(names(&panel.visible(&findings)), ["e", "f"]);
        // toggling again shows it
        panel.filter.toggle_category("clearance");
        assert!(panel.filter.category_shown("clearance"));
        // a name that is not a category changes nothing
        let before = panel.filter.clone();
        panel.filter.toggle_category("nothing");
        panel.filter.only_category("nothing");
        assert_eq!(panel.filter, before);
        assert!(!panel.filter.is_all());
        panel.filter.show_all();
        assert!(panel.filter.is_all());
        assert_eq!(panel.visible(&findings).len(), 6);
    }

    #[test]
    fn only_one_category_hides_all_the_others() {
        let findings = sample();
        let mut panel = DrcPanel::default();
        panel.filter.only_category("size");
        assert_eq!(panel.filter.hidden.len(), CATEGORIES.len() - 1);
        assert!(!panel.filter.hidden.contains("size"));
        assert_eq!(names(&panel.visible(&findings)), ["f", "b"]);
    }

    #[test]
    fn the_counts_are_by_category_and_severity_whatever_the_filter_says() {
        let findings = sample();
        let counts = DrcPanel::counts(&findings);
        let seen: Vec<(&str, usize, usize, usize)> = counts
            .iter()
            .map(|c| (c.name, c.errors, c.warnings, c.information))
            .collect();
        assert_eq!(
            seen,
            [
                ("size", 1, 1, 0),
                ("clearance", 1, 1, 0),
                ("edge", 1, 0, 0),
                ("mask_silk", 0, 0, 1),
            ]
        );
        assert_eq!(counts[0].total(), 2);
        assert!(DrcPanel::counts(&[]).is_empty());
    }

    #[test]
    fn the_step_goes_through_the_problems_with_a_place_and_goes_round_when_it_wraps() {
        let findings = sample();
        let mut panel = DrcPanel {
            sort: DrcSort::Found,
            wrap: true,
            ..DrcPanel::default()
        };
        // "d" has no place, so the steps are a, b, c, e, f
        let mut seen = Vec::new();
        for _ in 0..6 {
            seen.push(panel.step(&findings, true).unwrap());
        }
        assert_eq!(seen, [[1, 1], [2, 2], [3, 3], [5, 5], [6, 6], [1, 1]]);
        // backwards from the first goes to the last
        assert_eq!(panel.step(&findings, false), Some([6, 6]));
        assert_eq!(panel.step(&findings, false), Some([5, 5]));
        assert!(panel.is_current(&findings[4]));
        assert!(!panel.is_current(&findings[0]));
    }

    #[test]
    fn without_wrap_the_step_stays_at_the_ends() {
        let findings = sample();
        let mut panel = DrcPanel {
            sort: DrcSort::Found,
            wrap: false,
            ..DrcPanel::default()
        };
        // nothing is current: backwards starts at the last
        assert_eq!(panel.step(&findings, false), Some([6, 6]));
        assert_eq!(panel.step(&findings, true), Some([6, 6]));
        panel.select(&findings[0]);
        assert_eq!(panel.step(&findings, false), Some([1, 1]));
        assert_eq!(panel.step(&findings, true), Some([2, 2]));
    }

    #[test]
    fn the_step_follows_the_filter_and_the_sort() {
        let findings = sample();
        let mut panel = DrcPanel::default();
        panel.filter.warnings = false;
        panel.filter.information = false;
        // by severity the errors are a, e, f
        assert_eq!(panel.step(&findings, true), Some([1, 1]));
        assert_eq!(panel.step(&findings, true), Some([5, 5]));
        assert_eq!(panel.step(&findings, true), Some([6, 6]));
        // the current problem is hidden: the step starts again at the first
        panel.filter.errors = false;
        panel.filter.warnings = true;
        assert_eq!(panel.step(&findings, true), Some([2, 2]));
        // nothing shown has a place, or nothing is shown
        panel.filter.warnings = false;
        assert_eq!(panel.step(&findings, true), None);
        let only_unplaced = [finding(codes::CLEARANCE, Severity::Error, "x", None)];
        let mut open = DrcPanel::default();
        assert_eq!(open.step(&only_unplaced, true), None);
        assert_eq!(open.step(&[], false), None);
    }

    #[test]
    fn the_current_problem_survives_a_new_list_by_its_key() {
        let mut findings = sample();
        let mut panel = DrcPanel {
            sort: DrcSort::Found,
            ..DrcPanel::default()
        };
        panel.step(&findings, true);
        panel.step(&findings, true);
        assert!(panel.is_current(&findings[1]));
        // the check ran again and put a new problem first
        findings.insert(
            0,
            finding(codes::DRILL_SMALL, Severity::Error, "new", Some([9, 9])),
        );
        assert!(panel.is_current(&findings[2]));
        assert_eq!(panel.step(&findings, true), Some([3, 3]));
        let key = FindingKey::of(&findings[0]);
        assert!(key.matches(&findings[0]));
        assert!(!key.matches(&findings[1]));
    }

    #[test]
    fn the_panel_starts_as_the_settings_say() {
        let s = ViewerSettings::parse(
            "drc_show_warnings = false\ndrc_hidden_categories = zones|edge\ndrc_sort = category\ndrc_wrap = false\n",
        )
        .unwrap();
        let panel = DrcPanel::from_settings(&s);
        assert!(panel.filter.errors && !panel.filter.warnings && panel.filter.information);
        assert_eq!(
            panel.filter.hidden,
            BTreeSet::from(["zones".to_owned(), "edge".to_owned()])
        );
        assert_eq!(panel.sort, DrcSort::Category);
        assert!(!panel.wrap);
        assert!(panel.current.is_none());
        // the defaults of the settings are the panel that shows everything
        let plain = DrcPanel::from_settings(&ViewerSettings::default());
        assert!(plain.filter.is_all());
        assert_eq!(plain.sort, DrcSort::Severity);
        assert!(plain.wrap);
    }

    #[test]
    fn the_words_of_the_orders_are_read_back() {
        for sort in DrcSort::ALL {
            assert_eq!(DrcSort::from_key(sort.key()), Some(sort));
        }
        assert_eq!(DrcSort::from_key("Severity"), None);
        assert_eq!(DrcSort::from_key(""), None);
    }

    #[test]
    fn every_code_of_the_checks_has_a_category_that_the_panel_lists() {
        for code in [
            codes::TRACK_NARROW,
            codes::VIA_SMALL,
            codes::DRILL_SMALL,
            codes::RING_THIN,
            codes::CLEARANCE,
            codes::EDGE_CLOSE,
            codes::DRILL_SPACING,
            codes::MASK_BRIDGE,
            codes::ZONE_CLEARANCE,
            codes::ZONES_OVERLAP,
            codes::SILK_OVER_PAD,
            codes::SILK_OVER_MASK,
            codes::NOT_JUDGED,
            codes::CLASS_TRACK_NARROW,
            codes::CLASS_VIA_SMALL,
            "PHOENIX-DRC-001",
            "PHOENIX-DRC-013",
        ] {
            assert!(CATEGORIES.contains(&category(code)), "{code}");
            assert!(category_rank(code) < CATEGORIES.len(), "{code}");
        }
    }
}
