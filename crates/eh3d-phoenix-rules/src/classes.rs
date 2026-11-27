// SPDX-License-Identifier: GPL-3.0-or-later
//! Net classes: a name for a group of nets with limits of its own.
//!
//! A class belongs to the rule file of the second format (`phoenix-rules/2`) and lists the nets it
//! holds by name. A length of zero leaves that limit to the general rules. A class only adds a
//! limit of its own: it never relaxes the general ones, which [`crate::check_board`] still checks
//! for every net, so a power net can ask for wider tracks than the rest without any risk of
//! hiding a general finding.
//!
//! Checked per class: the narrowest track, the smallest via diameter, the smallest via drill and
//! the clearance. Between two pieces of copper of different nets the clearance asked for is the
//! larger of those of their classes; a pair is reported by this check only when it keeps the
//! general clearance, so that no pair is reported twice. Pads that are not round are judged by the
//! circles inside and around them, as the general check judges them: in between the pair is
//! inconclusive. A pour is judged as the manufacturing files write it, its outline kept the
//! clearance of the zone away from other copper: where that is less than a class asks, or where two
//! pours of different nets are nearer than their classes allow, the finding is a warning.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, Id, LayerKind};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_spatial::{inflate, Bx, Index, SpatialConfig};

use crate::codes::{CLASS_CLEARANCE, CLASS_TRACK_NARROW, CLASS_VIA_SMALL, NOT_JUDGED};
use crate::engine::why_unjudged;
use crate::geometry::{polygons_touch, Area, Near, Prim};
use crate::items::{pad_prim, placed_pads, track_prim, via_prim};
use crate::report::{Finding, Report, Severity};
use crate::rulefile::RuleFileError;

/// The place of the class checks in the fixed order of a report (after the general rules).
const RANK_CLASS_TRACK: u8 = 14;
const RANK_CLASS_VIA: u8 = 15;
const RANK_CLASS_CLEARANCE: u8 = 16;
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
    /// Closest approach of its copper to the copper of other nets.
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

/// A piece of copper as the clearance of the classes sees it.
struct Item {
    label: String,
    /// The net, its clearance by class and the name of that class.
    net: Option<Id>,
    asks: i64,
    class: String,
    layers: Vec<Id>,
    prim: Prim,
}

/// Every track, via and pad that shows copper.
fn copper_items(board: &Board, classes: &NetClasses) -> Vec<Item> {
    let item = |label: String, net: Option<Id>, layers: Vec<Id>, prim: Prim| {
        let class = net
            .and_then(|n| board.nets.get(&n))
            .and_then(|n| classes.class_of(&n.name));
        Item {
            label,
            net,
            asks: class.map_or(0, |c| c.min_clearance.get()),
            class: class.map_or_else(String::new, |c| c.name.clone()),
            layers,
            prim,
        }
    };
    let mut out = Vec::new();
    for (id, track) in &board.tracks {
        if let Some(prim) = track_prim(track) {
            out.push(item(
                format!("track {id}"),
                Some(track.net),
                vec![track.layer],
                prim,
            ));
        }
    }
    for (id, via) in &board.vias {
        if let Some(prim) = via_prim(via) {
            out.push(item(
                format!("via {id}"),
                Some(via.net),
                board.via_layers(via),
                prim,
            ));
        }
    }
    for pad in placed_pads(board) {
        if pad.drill.is_some() && !pad.plated {
            continue;
        }
        let layers: Vec<Id> = pad
            .layers
            .iter()
            .copied()
            .filter(|l| {
                board
                    .layers
                    .get(l)
                    .is_some_and(|x| x.kind == LayerKind::Copper)
            })
            .collect();
        if let Some(prim) = pad_prim(&pad) {
            out.push(item(pad.label.clone(), pad.net, layers, prim));
        }
    }
    out
}

/// The pairs of copper that keep the general clearance `general` and not the one of a class.
fn check_clearances(board: &Board, classes: &NetClasses, general: Nm, report: &mut Report) {
    let widest = classes
        .classes()
        .iter()
        .map(|c| c.min_clearance.get())
        .max()
        .unwrap_or(0);
    if widest <= general.get() {
        return;
    }
    let items = copper_items(board, classes);
    let boxes: Vec<Bx> = items
        .iter()
        .map(|i| inflate(&i.prim.bbox(), i.prim.hi / 2 + 1))
        .collect();
    let index = Index::grid(&boxes, &SpatialConfig::default())
        .unwrap_or_else(|_| Index::naive(boxes.len()));
    for (i, a) in items.iter().enumerate() {
        // only the items of a class look around: a pair needs at least one of them
        if a.asks <= general.get() {
            continue;
        }
        for j in index.candidates(&inflate(&boxes[i], i128::from(a.asks))) {
            let b = &items[j];
            // a pair of two items of classes is met from both: judge it from the one that asks
            // for more, or from the first one when they ask for the same
            if j == i || b.asks > a.asks || (b.asks == a.asks && j < i) {
                continue;
            }
            if (a.net.is_some() && a.net == b.net) || !a.layers.iter().any(|l| b.layers.contains(l))
            {
                continue;
            }
            let limit = i128::from(a.asks);
            if !a.prim.may_be_within(&b.prim, limit) {
                continue;
            }
            if general.get() > 0
                && a.prim.within(&b.prim, i128::from(general.get()), false) == Near::Yes
            {
                continue;
            }
            let near = a.prim.within(&b.prim, limit, false);
            if near == Near::No {
                continue;
            }
            let (first, second) = if i <= j { (a, b) } else { (b, a) };
            let location = format!("{} and {}", first.label, second.label);
            let at: Point = first.prim.first_point();
            let (x, y) = (at.x_nm.get(), at.y_nm.get());
            match (near, a.prim.distance(&b.prim)) {
                (Near::Yes, Some(d)) => {
                    let gap = d.nm_f64() - (a.prim.lo + b.prim.lo) as f64 / 2.0;
                    report.confirmed.push(
                        Finding::new(
                            RANK_CLASS_CLEARANCE,
                            "class_clearance",
                            CLASS_CLEARANCE,
                            Severity::Error,
                            location,
                        )
                        .at(x, y)
                        .values(gap.round().max(0.0) as i64, a.asks)
                        .why(&format!("class {}", a.class)),
                    );
                }
                (near, _) => report.inconclusive.push(
                    Finding::new(
                        RANK_NOT_JUDGED,
                        "not_judged",
                        NOT_JUDGED,
                        Severity::Info,
                        location,
                    )
                    .at(x, y)
                    .limit(a.asks)
                    .why(why_unjudged(near)),
                ),
            }
        }
    }
}

/// The pours that the clearance of a class is not kept from: the copper of other nets near a pour
/// whose own clearance is smaller than the class asks, and two pours of different nets nearer than
/// their classes allow.
fn check_zone_clearances(board: &Board, classes: &NetClasses, general: Nm, report: &mut Report) {
    let asked = |net: Id| -> (i64, String) {
        board
            .nets
            .get(&net)
            .and_then(|n| classes.class_of(&n.name))
            .map_or((0, String::new()), |c| {
                (c.min_clearance.get(), c.name.clone())
            })
    };
    let finding = |location: String, at: Point, gap: i64, limit: i64, class: &str| {
        Finding::new(
            RANK_CLASS_CLEARANCE,
            "class_clearance",
            CLASS_CLEARANCE,
            Severity::Warning,
            location,
        )
        .at(at.x_nm.get(), at.y_nm.get())
        .values(gap, limit)
        .why(&format!("class {class}"))
    };
    let items = copper_items(board, classes);
    let zones: Vec<_> = board.zones.iter().collect();
    for (i, (id, zone)) in zones.iter().enumerate() {
        let Some(area) = Area::new(&zone.outline, &zone.cutouts) else {
            continue;
        };
        let (own, own_class) = asked(zone.net);
        let kept = zone.clearance.get();
        for item in &items {
            if item.net == Some(zone.net) || !item.layers.contains(&zone.layer) {
                continue;
            }
            let (limit, class) = if item.asks > own {
                (item.asks, item.class.as_str())
            } else {
                (own, own_class.as_str())
            };
            if limit <= general.get() || limit <= kept {
                continue;
            }
            let (near, gap) = area.near(&item.prim, i128::from(limit));
            if near == Near::Yes {
                report.confirmed.push(finding(
                    format!("zone {id} and {}", item.label),
                    item.prim.first_point(),
                    gap.max(kept),
                    limit,
                    class,
                ));
            }
        }
        for (other_id, other) in &zones[i + 1..] {
            if other.net == zone.net
                || other.layer != zone.layer
                || polygons_touch(&zone.outline, &other.outline)
            {
                continue;
            }
            let (theirs, their_class) = asked(other.net);
            let (limit, class) = if theirs > own {
                (theirs, their_class.as_str())
            } else {
                (own, own_class.as_str())
            };
            let Some(second) = Area::new(&other.outline, &other.cutouts) else {
                continue;
            };
            if limit <= general.get() {
                continue;
            }
            let (near, gap) = area.near(&second.border, i128::from(limit));
            // nearer than the general clearance is the finding of the general check
            if near == Near::Yes && gap >= general.get() {
                report.confirmed.push(finding(
                    format!("zone {id} and zone {other_id}"),
                    zone.outline[0],
                    gap,
                    limit,
                    class,
                ));
            }
        }
    }
}

/// Checks the limits of the net classes on a board: the width of the tracks and the diameter and
/// drill of the vias of every net that belongs to a class, and the clearance of the copper of those
/// nets where it is larger than the `general` clearance of the rules. The report is in the same
/// fixed order as the general check and can be merged with it.
pub fn check_classes(board: &Board, classes: &NetClasses, general: Nm) -> Report {
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
    check_clearances(board, classes, general, &mut report);
    check_zone_clearances(board, classes, general, &mut report);
    report.finish();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::CLASS_CLEARANCE;
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
        let report = check_classes(&board(), &classes, Nm::ZERO);
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
        let none = check_classes(&board(), &NetClasses::default(), Nm::ZERO);
        assert!(none.confirmed.is_empty() && none.inconclusive.is_empty());
    }

    /// A track of SIG whose edge is `gap` nanometres above the edge of the track of VCC.
    fn close(gap: i64) -> Board {
        let mut b = board();
        if let Some(track) = b.tracks.get_mut(&id(21)) {
            let y = 20_000_000 + 200_000 + gap;
            track.path = vec![Segment::Line {
                start: pt(0, y),
                end: pt(5_000_000, y),
            }];
        }
        b
    }

    fn with_clearance(nm: i64) -> NetClasses {
        let mut c = class("power", &["VCC"], 0, 0, 0);
        c.min_clearance = Nm::new(nm).unwrap();
        NetClasses::new(vec![c]).unwrap()
    }

    #[test]
    fn the_clearance_of_a_class_is_checked_between_its_nets_and_the_others() {
        let classes = with_clearance(500_000);
        let general = Nm::new(200_000).unwrap();
        // 0.3 mm apart: enough for the general rules, not for the class
        let report = check_classes(&close(300_000), &classes, general);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let found = &report.confirmed[0];
        assert_eq!(found.code, CLASS_CLEARANCE);
        assert_eq!(
            (found.measured_nm, found.limit_nm),
            (Some(300_000), Some(500_000))
        );
        assert!(found.detail.contains("class power"));
        // exactly the clearance of the class is enough
        assert!(check_classes(&close(500_000), &classes, general)
            .confirmed
            .is_empty());
        // closer than the general clearance is a finding of the general check, not a second one
        assert!(check_classes(&close(100_000), &classes, general)
            .confirmed
            .is_empty());
        // and of this check when there is no general clearance
        assert_eq!(
            check_classes(&close(100_000), &classes, Nm::ZERO)
                .confirmed
                .len(),
            1
        );
        // a class that asks for no more than the general rules adds nothing
        assert!(
            check_classes(&close(300_000), &with_clearance(150_000), general)
                .confirmed
                .is_empty()
        );
        // two items of classes are judged once, by the larger clearance
        let mut sig = class("signal", &["SIG"], 0, 0, 0);
        sig.min_clearance = Nm::new(400_000).unwrap();
        let mut power = class("power", &["VCC"], 0, 0, 0);
        power.min_clearance = Nm::new(350_000).unwrap();
        let both = NetClasses::new(vec![power, sig]).unwrap();
        let report = check_classes(&close(375_000), &both, general);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        assert_eq!(report.confirmed[0].limit_nm, Some(400_000));
        assert!(report.confirmed[0].detail.contains("class signal"));
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
