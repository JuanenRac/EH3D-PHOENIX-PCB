// SPDX-License-Identifier: GPL-3.0-or-later
//! Net classes: a name for a group of nets with limits of its own.
//!
//! A class belongs to the rule file of the second format (`phoenix-rules/2`) and lists the nets it
//! holds by name. A length of zero leaves that limit to the general rules. A class only adds a
//! limit of its own: it never relaxes the general ones, which [`crate::check_board`] still checks
//! for every net, so a power net can ask for wider tracks than the rest without any risk of
//! hiding a general finding.
//!
//! Checked per class: the narrowest track, the smallest via diameter and the smallest via drill.
//! The clearance of a class is read and kept but not checked yet (a clearance between two nets of
//! different classes needs the geometry of the board checks); the check says so in the
//! inconclusive list and never passes it silently.

use std::collections::BTreeSet;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_model::Nm;

use crate::codes::{CLASS_TRACK_NARROW, CLASS_VIA_SMALL, NOT_JUDGED};
use crate::report::{Finding, Report, Severity};
use crate::rulefile::RuleFileError;

/// The place of the class checks in the fixed order of a report (after the general rules).
const RANK_CLASS_TRACK: u8 = 14;
const RANK_CLASS_VIA: u8 = 15;
const RANK_NOT_JUDGED: u8 = 13;

/// One net class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetClass {
    /// The name of the class.
    pub name: String,
    /// The names of the nets it holds.
    pub nets: Vec<String>,
    /// Narrowest track of its nets; zero leaves it to the general rules.
    pub min_track_width: Nm,
    /// Closest approach of its copper to other copper; read but not checked yet.
    pub min_clearance: Nm,
    /// Smallest via diameter of its nets; zero leaves it to the general rules.
    pub min_via_diameter: Nm,
    /// Smallest via drill of its nets; zero leaves it to the general rules.
    pub min_via_drill: Nm,
}

/// The net classes of a rule file, in the order of the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetClasses {
    classes: Vec<NetClass>,
}

impl NetClasses {
    /// Gathers classes. A class needs a name, no two classes share a name and a net belongs to at
    /// most one class.
    pub fn new(classes: Vec<NetClass>) -> Result<NetClasses, RuleFileError> {
        let mut names: BTreeSet<&str> = BTreeSet::new();
        let mut nets: BTreeSet<&str> = BTreeSet::new();
        for class in &classes {
            let bad = |why: &'static str| RuleFileError::BadClass {
                name: class.name.clone(),
                why,
            };
            if class.name.trim().is_empty() {
                return Err(bad("the class has no name"));
            }
            if !names.insert(class.name.as_str()) {
                return Err(bad("two classes have this name"));
            }
            for net in &class.nets {
                if !nets.insert(net.as_str()) {
                    return Err(bad("a net of the class is already in another class"));
                }
            }
        }
        Ok(NetClasses { classes })
    }

    /// True when there is no class.
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// The classes, in the order of the file.
    pub fn classes(&self) -> &[NetClass] {
        &self.classes
    }

    /// The class that holds the net of this name, if any.
    pub fn class_of(&self, net_name: &str) -> Option<&NetClass> {
        self.classes
            .iter()
            .find(|c| c.nets.iter().any(|n| n == net_name))
    }
}

fn on(value: Nm) -> bool {
    value.get() > 0
}

/// Checks the limits of the net classes on a board: the width of the tracks and the diameter and
/// drill of the vias of every net that belongs to a class. The report is in the same fixed order
/// as the general check and can be merged with it.
pub fn check_classes(board: &Board, classes: &NetClasses) -> Report {
    let mut report = Report::new("check");
    for (id, track) in &board.tracks {
        let Some(net) = board.nets.get(&track.net) else {
            continue;
        };
        let Some(class) = classes.class_of(&net.name) else {
            continue;
        };
        if on(class.min_track_width) && track.width < class.min_track_width {
            let mut finding = Finding::new(
                RANK_CLASS_TRACK,
                "class_track_width",
                CLASS_TRACK_NARROW,
                Severity::Error,
                format!("track {id} (net {}, class {})", net.name, class.name),
            )
            .values(track.width.get(), class.min_track_width.get());
            if let Some(segment) = track.path.first() {
                let p = segment.start();
                finding = finding.at(p.x_nm.get(), p.y_nm.get());
            }
            report.confirmed.push(finding);
        }
    }
    for (id, via) in &board.vias {
        let Some(net) = board.nets.get(&via.net) else {
            continue;
        };
        let Some(class) = classes.class_of(&net.name) else {
            continue;
        };
        let (x, y) = (via.at.x_nm.get(), via.at.y_nm.get());
        let place = format!("via {id} (net {}, class {})", net.name, class.name);
        if on(class.min_via_diameter) && via.diameter < class.min_via_diameter {
            report.confirmed.push(
                Finding::new(
                    RANK_CLASS_VIA,
                    "class_via_size",
                    CLASS_VIA_SMALL,
                    Severity::Error,
                    place.clone(),
                )
                .at(x, y)
                .values(via.diameter.get(), class.min_via_diameter.get())
                .why("diameter"),
            );
        }
        if on(class.min_via_drill) && via.drill < class.min_via_drill {
            report.confirmed.push(
                Finding::new(
                    RANK_CLASS_VIA,
                    "class_via_size",
                    CLASS_VIA_SMALL,
                    Severity::Error,
                    place,
                )
                .at(x, y)
                .values(via.drill.get(), class.min_via_drill.get())
                .why("drill"),
            );
        }
    }
    for class in classes.classes() {
        if on(class.min_clearance) {
            report.inconclusive.push(
                Finding::new(
                    RANK_NOT_JUDGED,
                    "not_judged",
                    NOT_JUDGED,
                    Severity::Info,
                    format!("class {}", class.name),
                )
                .limit(class.min_clearance.get())
                .why("the clearance of a net class is read but not checked yet"),
            );
        }
    }
    report.finish();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rulefile::{RuleFile, RuleValues};
    use eh3d_phoenix_board::{Layer, LayerKind, Net, Segment, Track, Via};
    use eh3d_phoenix_model::Point;

    fn id(n: u16) -> eh3d_phoenix_board::Id {
        eh3d_phoenix_board::Id::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn class(name: &str, nets: &[&str], track: i64, via: i64, drill: i64) -> NetClass {
        NetClass {
            name: name.into(),
            nets: nets.iter().map(|n| (*n).to_owned()).collect(),
            min_track_width: Nm::new(track).unwrap(),
            min_clearance: Nm::ZERO,
            min_via_diameter: Nm::new(via).unwrap(),
            min_via_drill: Nm::new(drill).unwrap(),
        }
    }

    /// A board with a power net and a signal net, a thin track and a small via on each.
    fn board() -> Board {
        let mut b = Board::default();
        b.layers.insert(
            id(1),
            Layer {
                name: "Top".into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.nets.insert(id(10), Net { name: "VCC".into() });
        b.nets.insert(id(11), Net { name: "SIG".into() });
        for (n, net) in [(20u16, id(10)), (21, id(11))] {
            b.tracks.insert(
                id(n),
                Track {
                    net,
                    layer: id(1),
                    width: Nm::new(200_000).unwrap(),
                    path: vec![Segment::Line {
                        start: pt(0, i64::from(n) * 1_000_000),
                        end: pt(5_000_000, i64::from(n) * 1_000_000),
                    }],
                },
            );
        }
        for (n, net) in [(30u16, id(10)), (31, id(11))] {
            b.vias.insert(
                id(n),
                Via {
                    net,
                    at: pt(i64::from(n) * 1_000_000, 0),
                    drill: Nm::new(200_000).unwrap(),
                    diameter: Nm::new(400_000).unwrap(),
                    from_layer: id(1),
                    to_layer: id(1),
                },
            );
        }
        b
    }

    #[test]
    fn only_the_nets_of_a_class_are_held_to_its_limits() {
        let classes =
            NetClasses::new(vec![class("power", &["VCC"], 300_000, 600_000, 300_000)]).unwrap();
        let report = check_classes(&board(), &classes);
        // the track and the via of VCC, and the via has two things wrong (diameter and drill)
        assert_eq!(report.confirmed.len(), 3, "{:?}", report.confirmed);
        assert!(report
            .confirmed
            .iter()
            .all(|f| f.location.contains("net VCC") && f.location.contains("class power")));
        let track = &report.confirmed[0];
        assert_eq!(track.code, CLASS_TRACK_NARROW);
        assert_eq!(
            (track.measured_nm, track.limit_nm),
            (Some(200_000), Some(300_000))
        );
        assert!(!report.passes());
        // the general rules are not touched by a class
        let none = check_classes(&board(), &NetClasses::default());
        assert!(none.confirmed.is_empty() && none.inconclusive.is_empty());
    }

    #[test]
    fn a_class_with_a_clearance_says_that_it_is_not_checked() {
        let mut c = class("power", &["VCC"], 0, 0, 0);
        c.min_clearance = Nm::new(250_000).unwrap();
        let report = check_classes(&board(), &NetClasses::new(vec![c]).unwrap());
        assert!(report.confirmed.is_empty());
        assert_eq!(report.inconclusive.len(), 1);
        assert_eq!(report.inconclusive[0].code, NOT_JUDGED);
        assert_eq!(report.inconclusive[0].limit_nm, Some(250_000));
    }

    #[test]
    fn bad_classes_are_refused() {
        let twice = vec![class("a", &["X"], 0, 0, 0), class("a", &["Y"], 0, 0, 0)];
        let shared = vec![class("a", &["X"], 0, 0, 0), class("b", &["X"], 0, 0, 0)];
        let nameless = vec![class(" ", &["X"], 0, 0, 0)];
        for classes in [twice, shared, nameless] {
            let error = NetClasses::new(classes).unwrap_err();
            assert_eq!(error.code(), crate::codes::RULEFILE_CLASS);
        }
    }

    const V2: &str = r#"{"format":"phoenix-rules/2","name":"with classes","rules":{"min_clearance_nm":200000},
        "classes":[{"name":"power","nets":["VCC","GND"],"min_track_width_nm":300000,"min_via_diameter_nm":600000}]}"#;

    #[test]
    fn the_second_format_carries_classes_and_the_first_is_read_as_before() {
        let file = RuleFile::from_json(V2).unwrap();
        assert_eq!(file.values.min_clearance.get(), 200_000);
        assert_eq!(file.classes.classes().len(), 1);
        assert_eq!(
            file.classes.class_of("GND").map(|c| c.name.as_str()),
            Some("power")
        );
        assert!(file.classes.class_of("SIG").is_none());
        // writing and reading back gives the same file, in the second format
        let text = file.to_json();
        assert!(text.contains("phoenix-rules/2"));
        assert_eq!(RuleFile::from_json(&text).unwrap(), file);
        assert_eq!(file.to_json(), text);
        // the old entry point reads the values of a file of the second format
        let (name, values) = RuleValues::from_json(V2).unwrap();
        assert_eq!((name.as_str(), values), ("with classes", file.values));
        // a file of the first format has no classes and is written in the first format
        let v1 = RuleValues::default().to_json("plain");
        let plain = RuleFile::from_json(&v1).unwrap();
        assert!(plain.classes.is_empty());
        assert_eq!(plain.to_json(), v1);
        // classes need the second format
        let wrong = V2.replace("rules/2", "rules/1");
        assert!(RuleFile::from_json(&wrong).is_err());
    }

    #[test]
    fn a_net_in_two_classes_in_a_file_has_the_class_code() {
        let text = r#"{"format":"phoenix-rules/2","rules":{},"classes":[
            {"name":"a","nets":["X"]},{"name":"b","nets":["X"]}]}"#;
        let error = RuleFile::from_json(text).unwrap_err();
        assert_eq!(error.code(), crate::codes::RULEFILE_CLASS);
    }
}
