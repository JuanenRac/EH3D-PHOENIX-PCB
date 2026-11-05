//! The drawn schematic: sheets, placed parts and wires, and what they say about connections.
//!
//! The drawing is an addition to the schematic and changes nothing for one that has none: sheets,
//! wires, the placement of a part and the position of a pin are not written when they are not
//! given. All lengths are integer nanometres.
//!
//! The rules of the drawing, kept simple on purpose:
//!
//! * a part is placed on a sheet at a point with a rotation that is a multiple of 90 degrees; the
//!   position of a pin is the placement point plus the pin offset of its symbol, turned;
//! * a wire is a path of at least two points on one sheet; the points of a wire are connected;
//! * two wires, or a wire and a pin, are connected where they have a point in common (an end point,
//!   a bend or a pin). Crossing wires are not connected, and a point that lies in the middle of
//!   another wire's piece is not connected either: it is reported as a warning because it looks
//!   connected (`PHOENIX-SCH-008`);
//! * what the wires join is compared with the net list of the schematic: pins joined by wires must
//!   be on one net (`PHOENIX-SCH-005`, an error); a net whose pins are not all joined is only
//!   information (`PHOENIX-SCH-006`) because a connection may be given by the net list alone.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{orient, Nm, Orientation, Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::model::{Id, NetPin, Schematic, DANGLING};

/// A wire with fewer than two points or two equal points in a row.
pub const WIRE_INVALID: &str = "PHOENIX-SCH-004";
/// Pins joined by wires that the net list puts on different nets (or on none).
pub const DRAWN_NOT_IN_NETLIST: &str = "PHOENIX-SCH-005";
/// A net whose pins are not all joined by wires (information).
pub const NET_NOT_DRAWN: &str = "PHOENIX-SCH-006";
/// A placed part with a pin whose position is not given, so its connections cannot be judged.
pub const PIN_POSITION_MISSING: &str = "PHOENIX-SCH-007";
/// A point that lies in the middle of a wire's piece without being a point of that wire.
pub const LOOKS_CONNECTED: &str = "PHOENIX-SCH-008";
/// A placement whose rotation is not a multiple of 90 degrees, or whose pins fall out of range.
pub const PLACEMENT_INVALID: &str = "PHOENIX-SCH-009";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (WIRE_INVALID, "Wire is not a path of two or more points"),
    (
        DRAWN_NOT_IN_NETLIST,
        "Wires join pins that are on different nets",
    ),
    (NET_NOT_DRAWN, "Net is not joined by wires"),
    (PIN_POSITION_MISSING, "Pin position is not given"),
    (
        LOOKS_CONNECTED,
        "Point in the middle of a wire looks connected",
    ),
    (PLACEMENT_INVALID, "Placement is not valid"),
];

/// A sheet of the drawing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sheet {
    /// The name shown to the user.
    pub name: String,
    /// Width of the drawing area.
    pub width: Nm,
    /// Height of the drawing area.
    pub height: Nm,
}

/// Where a part is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// The sheet.
    pub sheet: Id,
    /// The origin of the symbol.
    pub at: Point,
    /// Rotation about the origin; a multiple of 90 degrees.
    pub rotation: Udeg,
}

/// A wire: a path of points on one sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    /// The sheet.
    pub sheet: Id,
    /// The points; each consecutive pair is a piece of wire.
    pub path: Vec<Point>,
}

fn make(code: &'static str, severity: Severity, detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, severity, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

/// Turns an offset by a multiple of 90 degrees.
fn turn(x: i64, y: i64, rotation: Udeg) -> Option<(i64, i64)> {
    let a = rotation.get();
    if a % 90_000_000 != 0 {
        return None;
    }
    Some(match a.rem_euclid(360_000_000) / 90_000_000 {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    })
}

/// A pin on a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlacedPin {
    pin: NetPin,
    sheet: Id,
    at: Point,
}

pub(crate) fn key(p: Point) -> (i64, i64) {
    (p.x_nm.get(), p.y_nm.get())
}

/// Whether `p` lies on the segment `a`-`b` strictly between its end points, in exact arithmetic.
pub(crate) fn inside_piece(p: Point, a: Point, b: Point) -> bool {
    if p == a || p == b || orient(a, b, p) != Orientation::Collinear {
        return false;
    }
    let between = |v: i64, x: i64, y: i64| v >= x.min(y) && v <= x.max(y);
    between(p.x_nm.get(), a.x_nm.get(), b.x_nm.get())
        && between(p.y_nm.get(), a.y_nm.get(), b.y_nm.get())
}

struct Union {
    parent: Vec<usize>,
}

impl Union {
    fn new(n: usize) -> Union {
        Union {
            parent: (0..n).collect(),
        }
    }
    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }
    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.parent[a.max(b)] = a.min(b);
        }
    }
}

/// What the drawing says: groups of pins joined by wires, and the problems found while reading it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Drawn {
    /// The pins of each group that wires or a shared point join, in a fixed order; a group has at
    /// least one pin.
    pub groups: Vec<BTreeSet<NetPin>>,
    /// The names that net labels and power ports give to each group, in the order of `groups`.
    pub group_names: Vec<BTreeSet<String>>,
    /// The problems of the drawing, in a fixed order.
    pub problems: Vec<Diagnostic>,
}

impl Schematic {
    /// The position of every pin of every placed part whose symbol gives it.
    fn placed_pins(&self, out: &mut Vec<Diagnostic>) -> Vec<PlacedPin> {
        let mut pins = Vec::new();
        for (part_id, part) in &self.parts {
            let Some(place) = part.placement else {
                continue;
            };
            let Some(symbol) = self.symbols.get(&part.symbol) else {
                continue;
            };
            if !self.sheets.contains_key(&place.sheet) {
                out.push(make(
                    DANGLING,
                    Severity::Error,
                    format!("part {part_id} is placed on a sheet that does not exist"),
                ));
                continue;
            }
            for (pin_id, pin) in &symbol.pins {
                let Some(offset) = pin.at else {
                    out.push(make(
                        PIN_POSITION_MISSING,
                        Severity::Info,
                        format!("{} pin {}", part.refdes, pin.number),
                    ));
                    continue;
                };
                let ox = if self.mirrored.contains(part_id) {
                    -offset.x_nm.get()
                } else {
                    offset.x_nm.get()
                };
                let at = turn(ox, offset.y_nm.get(), place.rotation).and_then(|(dx, dy)| {
                    Point::from_nm(
                        place.at.x_nm.get().checked_add(dx)?,
                        place.at.y_nm.get().checked_add(dy)?,
                    )
                    .ok()
                });
                match at {
                    Some(at) => pins.push(PlacedPin {
                        pin: NetPin {
                            part: *part_id,
                            pin: *pin_id,
                        },
                        sheet: place.sheet,
                        at,
                    }),
                    None => out.push(make(
                        PLACEMENT_INVALID,
                        Severity::Error,
                        format!("{} pin {}", part.refdes, pin.number),
                    )),
                }
            }
        }
        pins
    }

    /// The position of every pin of every placed part whose symbol gives it: the pin, the sheet
    /// and the point.
    pub fn pin_positions(&self) -> Vec<(NetPin, Id, Point)> {
        let mut ignored = Vec::new();
        self.placed_pins(&mut ignored)
            .into_iter()
            .map(|p| (p.pin, p.sheet, p.at))
            .collect()
    }

    /// Reads the drawing: the pins that wires join, and every problem of the sheets, parts and
    /// wires. Nothing when nothing is drawn.
    pub fn drawn(&self) -> Drawn {
        let mut problems = Vec::new();
        let pins = self.placed_pins(&mut problems);

        // nodes are (sheet, point); the points of a wire and the pins at a point share a node
        let mut index: BTreeMap<(Id, (i64, i64)), usize> = BTreeMap::new();
        let mut node = |sheet: Id, p: Point| {
            let next = index.len();
            *index.entry((sheet, key(p))).or_insert(next)
        };
        let mut links: Vec<(usize, usize)> = Vec::new();
        let mut pieces: Vec<(Id, Id, Point, Point)> = Vec::new(); // wire id, sheet, a, b
        let mut wire_points: Vec<(Id, Id, Point)> = Vec::new(); // sheet, wire, point
        for (wire_id, wire) in &self.wires {
            if !self.sheets.contains_key(&wire.sheet) {
                problems.push(make(
                    DANGLING,
                    Severity::Error,
                    format!("wire {wire_id} is on a sheet that does not exist"),
                ));
                continue;
            }
            let bad_pair = wire.path.windows(2).any(|w| w[0] == w[1]);
            if wire.path.len() < 2 || bad_pair {
                problems.push(make(
                    WIRE_INVALID,
                    Severity::Error,
                    format!("wire {wire_id}"),
                ));
                continue;
            }
            for w in wire.path.windows(2) {
                let (a, b) = (node(wire.sheet, w[0]), node(wire.sheet, w[1]));
                links.push((a, b));
                pieces.push((*wire_id, wire.sheet, w[0], w[1]));
            }
            for p in &wire.path {
                wire_points.push((wire.sheet, *wire_id, *p));
            }
        }
        let pin_nodes: Vec<usize> = pins.iter().map(|p| node(p.sheet, p.at)).collect();
        // net labels and power ports: the same name joins the nets wherever they are
        let mut named: Vec<(&str, usize)> = Vec::new();
        for label in self.labels.values() {
            if self.sheets.contains_key(&label.sheet) {
                named.push((label.name.as_str(), node(label.sheet, label.at)));
            }
        }
        for port in self.power_ports.values() {
            if self.sheets.contains_key(&port.sheet) {
                named.push((port.name.as_str(), node(port.sheet, port.at)));
            }
        }

        // a point in the middle of a piece of another wire looks connected and is not
        let mut warned: BTreeSet<(Id, Id, (i64, i64))> = BTreeSet::new();
        for (sheet, wire_id, p) in &wire_points {
            for (other, other_sheet, a, b) in &pieces {
                if other_sheet == sheet
                    && other != wire_id
                    && inside_piece(*p, *a, *b)
                    && warned.insert((*sheet, *wire_id, key(*p)))
                {
                    problems.push(make(
                        LOOKS_CONNECTED,
                        Severity::Warning,
                        format!("wire {wire_id} ends in the middle of wire {other}"),
                    ));
                }
            }
        }
        for pin in &pins {
            for (wire_id, sheet, a, b) in &pieces {
                if *sheet == pin.sheet && inside_piece(pin.at, *a, *b) {
                    let part = self
                        .parts
                        .get(&pin.pin.part)
                        .map_or("?", |p| p.refdes.as_str());
                    let number = self.pin_number(pin.pin).unwrap_or("?");
                    problems.push(make(
                        LOOKS_CONNECTED,
                        Severity::Warning,
                        format!("{part} pin {number} lies in the middle of wire {wire_id}"),
                    ));
                }
            }
        }

        let mut union = Union::new(index.len());
        for (a, b) in links {
            union.join(a, b);
        }
        let mut first_of_name: BTreeMap<&str, usize> = BTreeMap::new();
        for (name, n) in &named {
            match first_of_name.entry(*name) {
                Entry::Occupied(first) => union.join(*first.get(), *n),
                Entry::Vacant(slot) => {
                    slot.insert(*n);
                }
            }
        }
        let mut groups: BTreeMap<usize, BTreeSet<NetPin>> = BTreeMap::new();
        for (pin, n) in pins.iter().zip(&pin_nodes) {
            groups.entry(union.find(*n)).or_default().insert(pin.pin);
        }
        let mut names_of_root: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        for (name, n) in &named {
            names_of_root
                .entry(union.find(*n))
                .or_default()
                .insert((*name).to_owned());
        }
        let mut pairs: Vec<(BTreeSet<NetPin>, BTreeSet<String>)> = groups
            .into_iter()
            .map(|(root, pins)| (pins, names_of_root.remove(&root).unwrap_or_default()))
            .collect();
        pairs.sort();
        let (groups, group_names): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();

        // the drawing against the net list
        let net_of: BTreeMap<NetPin, Id> = self
            .nets
            .iter()
            .flat_map(|(nid, n)| n.pins.iter().map(move |p| (*p, *nid)))
            .collect();
        for group in &groups {
            let nets: BTreeSet<Option<Id>> = group.iter().map(|p| net_of.get(p).copied()).collect();
            if group.len() >= 2 && nets.len() > 1 {
                let names: Vec<String> = group
                    .iter()
                    .filter_map(|p| {
                        let part = self.parts.get(&p.part)?;
                        Some(format!("{} pin {}", part.refdes, self.pin_number(*p)?))
                    })
                    .collect();
                problems.push(make(
                    DRAWN_NOT_IN_NETLIST,
                    Severity::Error,
                    names.join(", "),
                ));
            }
        }
        let group_of: BTreeMap<NetPin, usize> = groups
            .iter()
            .enumerate()
            .flat_map(|(i, g)| g.iter().map(move |p| (*p, i)))
            .collect();
        for (net_id, net) in &self.nets {
            if net.pins.len() < 2 || net.pins.iter().any(|p| !group_of.contains_key(p)) {
                continue; // a pin that is not placed: nothing to judge
            }
            let joined: BTreeSet<usize> = net.pins.iter().map(|p| group_of[p]).collect();
            if joined.len() > 1 {
                problems.push(make(
                    NET_NOT_DRAWN,
                    Severity::Info,
                    format!("net {} ({net_id}) is in {} pieces", net.name, joined.len()),
                ));
            }
        }
        Drawn {
            groups,
            group_names,
            problems,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SchematicNet;
    use crate::sample::{drawn_schematic, id, schematic};

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn codes(s: &Schematic) -> Vec<String> {
        s.drawn()
            .problems
            .iter()
            .filter_map(|d| d.code.as_deref().map(str::to_owned))
            .collect()
    }

    fn rotate_first(s: &mut Schematic, ids: &[Id; 6], micro_degrees: i64) {
        s.parts
            .get_mut(&ids[1])
            .unwrap()
            .placement
            .as_mut()
            .unwrap()
            .rotation = Udeg::new(micro_degrees).unwrap();
    }

    #[test]
    fn a_drawn_schematic_that_agrees_with_its_net_list_has_no_problem() {
        let (s, ids, _, _) = drawn_schematic();
        assert!(codes(&s).is_empty(), "{:?}", s.drawn().problems);
        assert!(s.is_consistent());
        let drawn = s.drawn();
        // the two pins of the net are joined; the two other pins are alone
        let sizes: Vec<usize> = drawn.groups.iter().map(BTreeSet::len).collect();
        assert_eq!(drawn.groups.len(), 3);
        assert_eq!(sizes.iter().sum::<usize>(), 4);
        let joined = drawn.groups.iter().any(|g| {
            g.contains(&NetPin {
                part: ids[1],
                pin: ids[4],
            }) && g.contains(&NetPin {
                part: ids[2],
                pin: ids[3],
            })
        });
        assert!(joined);
    }

    #[test]
    fn the_drawing_round_trips_byte_for_byte_and_is_additive() {
        let (s, ..) = drawn_schematic();
        let bytes = s.to_bytes().unwrap();
        let back = Schematic::from_bytes(&bytes).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.to_bytes().unwrap(), bytes);
        let (plain, _) = schematic();
        let text = String::from_utf8(plain.to_bytes().unwrap()).unwrap();
        for word in ["sheets", "wires", "placement", "\"at\""] {
            assert!(!text.contains(word), "{word}");
        }
        assert_eq!(Schematic::from_bytes(text.as_bytes()).unwrap(), plain);
    }

    #[test]
    fn a_net_that_is_not_joined_is_information_not_an_error() {
        let (mut s, _, _, wire) = drawn_schematic();
        s.wires.get_mut(&wire).unwrap().path.pop();
        assert_eq!(codes(&s), vec![NET_NOT_DRAWN]);
        assert_eq!(s.drawn().problems[0].severity, Severity::Info);
        assert!(s.is_consistent());
    }

    #[test]
    fn wires_that_join_pins_of_different_nets_are_an_error() {
        let (mut s, _, sheet, _) = drawn_schematic();
        // a short wire from pin 1 of the first resistor to the first point of the other wire
        s.wires.insert(
            id(72),
            Wire {
                sheet,
                path: vec![pt(17_460_000, 20_000_000), pt(22_540_000, 20_000_000)],
            },
        );
        assert_eq!(codes(&s), vec![DRAWN_NOT_IN_NETLIST]);
        assert!(!s.is_consistent());
    }

    #[test]
    fn crossing_wires_do_not_connect_and_a_point_in_the_middle_looks_connected() {
        let (mut s, _, sheet, _) = drawn_schematic();
        // a wire that ends in the middle of the first piece of the other wire
        s.wires.insert(
            id(72),
            Wire {
                sheet,
                path: vec![pt(26_000_000, 25_000_000), pt(26_000_000, 20_000_000)],
            },
        );
        assert_eq!(codes(&s), vec![LOOKS_CONNECTED]);
        assert_eq!(s.drawn().problems[0].severity, Severity::Warning);
        assert!(s.is_consistent());
        // a wire that crosses it from side to side is silent: nothing ends on the other wire
        s.wires.get_mut(&id(72)).unwrap().path =
            vec![pt(26_000_000, 25_000_000), pt(26_000_000, 15_000_000)];
        assert!(codes(&s).is_empty());
        // a pin in the middle of a wire looks connected too
        let (mut t, _, _, wire) = drawn_schematic();
        t.wires.get_mut(&wire).unwrap().path =
            vec![pt(10_000_000, 20_000_000), pt(30_000_000, 20_000_000)];
        let found = codes(&t);
        assert!(found.contains(&LOOKS_CONNECTED.to_owned()), "{found:?}");
    }

    #[test]
    fn rotation_turns_the_pins() {
        let (mut s, ids, _, _) = drawn_schematic();
        // turned by a quarter, the pins of the first resistor stand above and below its origin
        rotate_first(&mut s, &ids, 90_000_000);
        assert_eq!(codes(&s), vec![NET_NOT_DRAWN]);
        // half a turn swaps the pins: the wire now meets pin 1, which is on no net
        rotate_first(&mut s, &ids, 180_000_000);
        assert_eq!(codes(&s), vec![DRAWN_NOT_IN_NETLIST, NET_NOT_DRAWN]);
        // an angle that is not a right angle is an error, and nothing panics
        rotate_first(&mut s, &ids, 45_000_000);
        assert!(codes(&s).contains(&PLACEMENT_INVALID.to_owned()));
        assert!(!s.is_consistent());
    }

    #[test]
    fn each_kind_of_problem_has_its_code() {
        // references that do not exist
        let (mut s, ids, _, wire) = drawn_schematic();
        s.wires.get_mut(&wire).unwrap().sheet = id(99);
        s.parts
            .get_mut(&ids[1])
            .unwrap()
            .placement
            .as_mut()
            .unwrap()
            .sheet = id(98);
        assert_eq!(codes(&s), vec![DANGLING, DANGLING]);
        // wires that are not paths
        let (mut s, _, sheet, wire) = drawn_schematic();
        s.wires.get_mut(&wire).unwrap().path = vec![pt(0, 0)];
        s.wires.insert(
            id(72),
            Wire {
                sheet,
                path: vec![pt(1, 1), pt(1, 1)],
            },
        );
        assert_eq!(codes(&s), vec![WIRE_INVALID, WIRE_INVALID, NET_NOT_DRAWN]);
        // a pin whose position is missing
        let (mut s, ids, _, _) = drawn_schematic();
        s.symbols
            .get_mut(&ids[0])
            .unwrap()
            .pins
            .get_mut(&ids[3])
            .unwrap()
            .at = None;
        let found = codes(&s);
        assert_eq!(
            found.iter().filter(|c| *c == PIN_POSITION_MISSING).count(),
            2
        );
        assert!(s.is_consistent());
    }

    #[test]
    fn nets_with_one_pin_and_unplaced_parts_are_left_alone() {
        let (mut s, ids, _, _) = drawn_schematic();
        s.nets.insert(
            id(80),
            SchematicNet {
                name: "Alone".into(),
                pins: vec![NetPin {
                    part: ids[1],
                    pin: ids[3],
                }],
            },
        );
        assert!(codes(&s).is_empty());
        s.parts.get_mut(&ids[2]).unwrap().placement = None;
        assert!(codes(&s).is_empty());
    }

    #[test]
    fn labels_with_one_name_join_pins_without_a_wire() {
        let (mut s, ids, sheet, wire) = drawn_schematic();
        s.wires.remove(&wire);
        for (n, at) in [
            (80, pt(22_540_000, 20_000_000)),
            (81, pt(37_460_000, 20_000_000)),
        ] {
            s.labels.insert(
                id(n),
                crate::marks::NetLabel {
                    sheet,
                    at,
                    name: "SIG".into(),
                },
            );
        }
        let drawn = s.drawn();
        assert!(codes(&s).is_empty(), "{:?}", drawn.problems);
        let joined = drawn.groups.iter().position(|g| g.len() == 2).unwrap();
        assert!(drawn.groups[joined].contains(&NetPin {
            part: ids[1],
            pin: ids[4],
        }));
        assert!(drawn.groups[joined].contains(&NetPin {
            part: ids[2],
            pin: ids[3],
        }));
        let names: Vec<String> = drawn.group_names[joined].iter().cloned().collect();
        assert_eq!(names, vec!["SIG".to_string()]);
    }

    #[test]
    fn labels_with_different_names_and_power_ports_follow_their_names() {
        let (mut s, _, sheet, wire) = drawn_schematic();
        s.wires.remove(&wire);
        s.labels.insert(
            id(80),
            crate::marks::NetLabel {
                sheet,
                at: pt(22_540_000, 20_000_000),
                name: "A".into(),
            },
        );
        s.labels.insert(
            id(81),
            crate::marks::NetLabel {
                sheet,
                at: pt(37_460_000, 20_000_000),
                name: "B".into(),
            },
        );
        assert_eq!(s.drawn().groups.len(), 4);
        assert_eq!(codes(&s), vec![NET_NOT_DRAWN]);
        // a power port named like a label joins the same two pins
        s.power_ports.insert(
            id(82),
            crate::marks::PowerPort {
                sheet,
                at: pt(22_540_000, 20_000_000),
                name: "B".into(),
            },
        );
        assert_eq!(s.drawn().groups.len(), 3);
        assert!(codes(&s).is_empty());
    }

    #[test]
    fn a_mirrored_part_has_its_pins_on_the_other_side() {
        let (mut s, ids, _, _) = drawn_schematic();
        let first_pin = NetPin {
            part: ids[1],
            pin: ids[3],
        };
        let x_of = |s: &Schematic| {
            s.pin_positions()
                .into_iter()
                .find(|(p, _, _)| *p == first_pin)
                .map(|(_, _, at)| at.x_nm.get())
        };
        assert_eq!(x_of(&s), Some(17_460_000));
        s.mirrored.insert(ids[1]);
        assert_eq!(x_of(&s), Some(22_540_000));
        // the wire now touches pin 1 of the first part and the net list says pin 2
        assert_eq!(codes(&s), vec![DRAWN_NOT_IN_NETLIST, NET_NOT_DRAWN]);
    }

    #[test]
    fn marks_are_written_only_when_there_are_some_and_round_trip() {
        let (mut s, _, sheet, _) = drawn_schematic();
        let plain = String::from_utf8(s.to_bytes().unwrap()).unwrap();
        for word in ["labels", "junctions", "power_ports", "mirrored"] {
            assert!(!plain.contains(word), "{word}");
        }
        s.labels.insert(
            id(80),
            crate::marks::NetLabel {
                sheet,
                at: pt(0, 0),
                name: "X".into(),
            },
        );
        s.junctions.insert(
            id(81),
            crate::marks::Junction {
                sheet,
                at: pt(1_000_000, 0),
            },
        );
        s.mirrored.insert(id(2));
        let bytes = s.to_bytes().unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("labels") && text.contains("junctions") && text.contains("mirrored"));
        assert_eq!(Schematic::from_bytes(&bytes).unwrap(), s);
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
