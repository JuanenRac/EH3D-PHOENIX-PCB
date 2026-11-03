//! The consistency between a schematic and a board (`PHOENIX-LINK-<NNN>`).
//!
//! A component of the board is linked to the part it was made from by `Component::symbol`, which
//! holds the id of the part. A pin of a part goes to the pad of the same number in the footprint of
//! the linked component. The check reports every difference, in a fixed order, and changes
//! nothing: which side is right is the decision of the person who reads the report.
//!
//! The nets are compared as groups: the pins of one schematic net must all be on one net of the
//! board, and one net of the board must not hold pins of two schematic nets. The names of the nets
//! are attributes and are not compared.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_board::{Board, PadRef};

use crate::model::{Id, NetPin, Schematic};

/// A part of the schematic that no component of the board was made from.
pub const PART_WITHOUT_COMPONENT: &str = "PHOENIX-LINK-001";
/// A component names a part that the schematic does not have.
pub const COMPONENT_WITHOUT_PART: &str = "PHOENIX-LINK-002";
/// A part and its component have different reference designators.
pub const REFDES_DIFFERS: &str = "PHOENIX-LINK-003";
/// A part and its component have different values.
pub const VALUE_DIFFERS: &str = "PHOENIX-LINK-004";
/// A part and its component have different footprints.
pub const FOOTPRINT_DIFFERS: &str = "PHOENIX-LINK-005";
/// A net of the schematic and the nets of the board do not agree.
pub const NET_DIFFERS: &str = "PHOENIX-LINK-006";
/// Two components were made from one part.
pub const PART_DRAWN_TWICE: &str = "PHOENIX-LINK-007";
/// A pin has no pad of its number in the footprint.
pub const PIN_WITHOUT_PAD: &str = "PHOENIX-LINK-008";

/// Wires of the drawing join pins whose pads are on different nets of the board.
pub const DRAWN_JOINS_OTHER_NETS: &str = "PHOENIX-LINK-009";
/// The pads of one net of the board are in several pieces of the drawing (information).
pub const BOARD_NET_NOT_DRAWN: &str = "PHOENIX-LINK-010";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (PART_WITHOUT_COMPONENT, "Part has no component on the board"),
    (
        COMPONENT_WITHOUT_PART,
        "Component names a part that does not exist",
    ),
    (REFDES_DIFFERS, "Reference designators differ"),
    (VALUE_DIFFERS, "Values differ"),
    (FOOTPRINT_DIFFERS, "Footprints differ"),
    (NET_DIFFERS, "Nets differ between schematic and board"),
    (PART_DRAWN_TWICE, "Part used by two components"),
    (PIN_WITHOUT_PAD, "Pin has no pad"),
    (
        DRAWN_JOINS_OTHER_NETS,
        "Wires join pads that are on different nets of the board",
    ),
    (
        BOARD_NET_NOT_DRAWN,
        "Net of the board is not joined by wires",
    ),
];

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

/// What the check found: the differences that are certain, and warnings that may be on purpose
/// (a value that was changed on one side).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkReport {
    /// Differences between the two documents.
    pub errors: Vec<Diagnostic>,
    /// Differences that may be intended.
    pub warnings: Vec<Diagnostic>,
}

impl LinkReport {
    /// True when the schematic and the board agree (warnings do not count).
    pub fn agrees(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Compares a schematic and a board.
pub fn check_consistency(schematic: &Schematic, board: &Board) -> LinkReport {
    let mut report = LinkReport::default();

    // part -> component
    let mut component_of: BTreeMap<Id, Id> = BTreeMap::new();
    for (cid, component) in &board.components {
        let Some(part_id) = component.symbol else {
            continue; // a component made on the board only has nothing to compare
        };
        if !schematic.parts.contains_key(&part_id) {
            report.errors.push(make(
                COMPONENT_WITHOUT_PART,
                Severity::Error,
                format!("component {} names part {part_id}", component.refdes),
            ));
            continue;
        }
        if let Some(first) = component_of.insert(part_id, *cid) {
            report.errors.push(make(
                PART_DRAWN_TWICE,
                Severity::Error,
                format!("part {part_id} by components {first} and {cid}"),
            ));
        }
    }

    for (part_id, part) in &schematic.parts {
        let Some(cid) = component_of.get(part_id) else {
            report.errors.push(make(
                PART_WITHOUT_COMPONENT,
                Severity::Error,
                part.refdes.clone(),
            ));
            continue;
        };
        let component = &board.components[cid];
        if component.refdes != part.refdes {
            report.errors.push(make(
                REFDES_DIFFERS,
                Severity::Error,
                format!(
                    "{} in the schematic, {} on the board",
                    part.refdes, component.refdes
                ),
            ));
        }
        if component.value != part.value {
            report.warnings.push(make(
                VALUE_DIFFERS,
                Severity::Warning,
                format!(
                    "{}: {} against {}",
                    part.refdes, part.value, component.value
                ),
            ));
        }
        if let Some(footprint) = part.footprint {
            if footprint != component.footprint {
                report.errors.push(make(
                    FOOTPRINT_DIFFERS,
                    Severity::Error,
                    part.refdes.clone(),
                ));
            }
        }
    }

    // pins -> pads -> nets of the board
    let mut schematic_net_of_pad: BTreeMap<PadRef, Id> = BTreeMap::new();
    for (net_id, net) in &schematic.nets {
        let mut board_nets: BTreeSet<Option<Id>> = BTreeSet::new();
        let mut pads_seen = 0usize;
        for pin in &net.pins {
            let (Some(cid), Some(number)) =
                (component_of.get(&pin.part), schematic.pin_number(*pin))
            else {
                continue; // the missing part is already reported
            };
            let component = &board.components[cid];
            let pad = board.footprints.get(&component.footprint).and_then(|f| {
                f.pads
                    .iter()
                    .find(|(_, pad)| pad.number == number)
                    .map(|(pid, _)| *pid)
            });
            let Some(pad) = pad else {
                report.errors.push(make(
                    PIN_WITHOUT_PAD,
                    Severity::Error,
                    format!("{} pin {number}", component.refdes),
                ));
                continue;
            };
            let pad_ref = PadRef {
                component: *cid,
                pad,
            };
            schematic_net_of_pad.insert(pad_ref, *net_id);
            board_nets.insert(board.net_of(pad_ref));
            pads_seen += 1;
        }
        if pads_seen >= 2 && (board_nets.len() > 1 || board_nets.contains(&None)) {
            report.errors.push(make(
                NET_DIFFERS,
                Severity::Error,
                format!(
                    "net {} is on {} nets of the board or has pads without one",
                    net.name,
                    board_nets.len()
                ),
            ));
        }
    }

    // the other way: a board net must not join pins of two schematic nets
    let mut schematic_nets_of_board_net: BTreeMap<Id, BTreeSet<Id>> = BTreeMap::new();
    for (pad_ref, schematic_net) in &schematic_net_of_pad {
        if let Some(board_net) = board.net_of(*pad_ref) {
            schematic_nets_of_board_net
                .entry(board_net)
                .or_default()
                .insert(*schematic_net);
        }
    }
    for (board_net, nets) in &schematic_nets_of_board_net {
        if nets.len() > 1 {
            let name = board.nets.get(board_net).map_or("?", |n| n.name.as_str());
            report.errors.push(make(
                NET_DIFFERS,
                Severity::Error,
                format!(
                    "net {name} of the board joins {} nets of the schematic",
                    nets.len()
                ),
            ));
        }
    }
    report
}

/// The pad of the component made from a part that a pin goes to, by the number of the pin.
fn pad_of_pin(
    schematic: &Schematic,
    board: &Board,
    component_of: &BTreeMap<Id, Id>,
    pin: NetPin,
) -> Option<PadRef> {
    let cid = component_of.get(&pin.part)?;
    let number = schematic.pin_number(pin)?;
    let component = board.components.get(cid)?;
    let footprint = board.footprints.get(&component.footprint)?;
    let (pad, _) = footprint.pads.iter().find(|(_, p)| p.number == number)?;
    Some(PadRef {
        component: *cid,
        pad: *pad,
    })
}

/// Compares what the wires of the drawing join with the nets of the board.
///
/// Two pins that wires join must go to pads of one net of the board (`PHOENIX-LINK-009`, an error);
/// pads of one net of the board that are drawn in several pieces get a warning
/// (`PHOENIX-LINK-010`), because a connection may be drawn later. Only the pins of placed parts take
/// part, and a part is matched to its component the same way as in [`check_consistency`], which
/// reports the parts and pins that do not match.
pub fn check_drawn_against_board(schematic: &Schematic, board: &Board) -> LinkReport {
    let mut report = LinkReport::default();
    let component_of: BTreeMap<Id, Id> = board
        .components
        .iter()
        .filter_map(|(cid, c)| c.symbol.map(|part| (part, *cid)))
        .filter(|(part, _)| schematic.parts.contains_key(part))
        .collect();
    let drawn = schematic.drawn();
    let mut group_of_pad: BTreeMap<PadRef, usize> = BTreeMap::new();
    for (index, group) in drawn.groups.iter().enumerate() {
        let pads: Vec<(NetPin, PadRef)> = group
            .iter()
            .filter_map(|p| Some((*p, pad_of_pin(schematic, board, &component_of, *p)?)))
            .collect();
        for (_, pad) in &pads {
            group_of_pad.insert(*pad, index);
        }
        let nets: BTreeSet<Option<Id>> = pads.iter().map(|(_, pad)| board.net_of(*pad)).collect();
        if pads.len() >= 2 && (nets.len() > 1 || nets.contains(&None)) {
            let names: Vec<String> = pads
                .iter()
                .filter_map(|(pin, _)| {
                    let part = schematic.parts.get(&pin.part)?;
                    Some(format!(
                        "{} pin {}",
                        part.refdes,
                        schematic.pin_number(*pin)?
                    ))
                })
                .collect();
            report.errors.push(make(
                DRAWN_JOINS_OTHER_NETS,
                Severity::Error,
                names.join(", "),
            ));
        }
    }
    let mut groups_of_net: BTreeMap<Id, BTreeSet<usize>> = BTreeMap::new();
    for (pad, group) in &group_of_pad {
        if let Some(net) = board.net_of(*pad) {
            groups_of_net.entry(net).or_default().insert(*group);
        }
    }
    for (net, groups) in &groups_of_net {
        if groups.len() > 1 {
            let name = board.nets.get(net).map_or("?", |n| n.name.as_str());
            report.warnings.push(make(
                BOARD_NET_NOT_DRAWN,
                Severity::Warning,
                format!("net {name} is drawn in {} pieces", groups.len()),
            ));
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NetPin, Schematic};
    use crate::sample::{id, schematic};
    use eh3d_phoenix_board::{Component, Footprint, Net, Pad, PadNet, PadShape, Side};
    use eh3d_phoenix_model::{Nm, Point, Udeg};

    /// A board made from the sample schematic: two resistors of one footprint with pads 1 and 2,
    /// one net joining pad 2 of R1 and pad 1 of R2.
    fn board_for(s: &Schematic, ids: &[Id; 6]) -> (Board, Id, [Id; 2], [Id; 2]) {
        let fp = id(60);
        let (pad1, pad2) = (id(61), id(62));
        let (c1, c2) = (id(63), id(64));
        let net = id(65);
        let mut b = Board::default();
        let mut pads = std::collections::BTreeMap::new();
        for (pid, number) in [(pad1, "1"), (pad2, "2")] {
            pads.insert(
                pid,
                Pad {
                    number: number.into(),
                    shape: PadShape::Circle {
                        diameter: Nm::new(1_000_000).unwrap(),
                    },
                    at: Point::from_nm(0, 0).unwrap(),
                    rotation: Udeg::ZERO,
                    drill: None,
                    layers: vec![],
                },
            );
        }
        b.footprints.insert(
            fp,
            Footprint {
                name: "F".into(),
                pads,
            },
        );
        for (cid, part_id) in [(c1, ids[1]), (c2, ids[2])] {
            let part = &s.parts[&part_id];
            b.components.insert(
                cid,
                Component {
                    refdes: part.refdes.clone(),
                    value: part.value.clone(),
                    symbol: Some(part_id),
                    footprint: fp,
                    at: Point::from_nm(0, 0).unwrap(),
                    rotation: Udeg::ZERO,
                    side: Side::Top,
                },
            );
        }
        b.nets.insert(net, Net { name: "N".into() });
        b.pad_nets = vec![
            PadNet {
                component: c1,
                pad: pad2,
                net,
            },
            PadNet {
                component: c2,
                pad: pad1,
                net,
            },
        ];
        b.pad_nets.sort_by_key(|p| p.pad_ref());
        (b, net, [c1, c2], [pad1, pad2])
    }

    fn codes(r: &LinkReport) -> Vec<String> {
        r.errors
            .iter()
            .chain(r.warnings.iter())
            .filter_map(|d| d.code.as_deref().map(str::to_owned))
            .collect()
    }

    #[test]
    fn a_board_made_from_the_schematic_agrees() {
        let (s, ids) = schematic();
        let (b, ..) = board_for(&s, &ids);
        let r = check_consistency(&s, &b);
        assert!(r.agrees(), "{:?}", r.errors);
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn a_missing_component_a_stray_link_and_a_double_link_are_found() {
        let (s, ids) = schematic();
        let (mut b, _, cs, _) = board_for(&s, &ids);
        // R2 deleted from the board
        b.components.remove(&cs[1]);
        b.pad_nets.retain(|p| p.component != cs[1]);
        assert_eq!(
            codes(&check_consistency(&s, &b)),
            vec![PART_WITHOUT_COMPONENT]
        );
        // a component that names a part that is not there
        let (mut b, _, cs, _) = board_for(&s, &ids);
        b.components.get_mut(&cs[0]).unwrap().symbol = Some(id(99));
        let c = codes(&check_consistency(&s, &b));
        assert!(c.contains(&COMPONENT_WITHOUT_PART.to_string()));
        assert!(c.contains(&PART_WITHOUT_COMPONENT.to_string()));
        // two components made from one part
        let (mut b, _, cs, _) = board_for(&s, &ids);
        b.components.get_mut(&cs[1]).unwrap().symbol = Some(ids[1]);
        assert!(codes(&check_consistency(&s, &b)).contains(&PART_DRAWN_TWICE.to_string()));
    }

    #[test]
    fn a_changed_designator_value_or_footprint_is_reported_by_kind() {
        let (s, ids) = schematic();
        let (mut b, _, cs, _) = board_for(&s, &ids);
        b.components.get_mut(&cs[0]).unwrap().refdes = "R9".into();
        b.components.get_mut(&cs[1]).unwrap().value = "22k".into();
        let r = check_consistency(&s, &b);
        assert_eq!(codes(&r), vec![REFDES_DIFFERS, VALUE_DIFFERS]);
        assert!(!r.agrees());
        // the value alone is a warning and the documents still agree
        let (mut b, _, cs, _) = board_for(&s, &ids);
        b.components.get_mut(&cs[1]).unwrap().value = "22k".into();
        assert!(check_consistency(&s, &b).agrees());
        // a footprint chosen in the schematic that the component does not use
        let mut s2 = s.clone();
        s2.parts.get_mut(&ids[1]).unwrap().footprint = Some(id(77));
        let (b, ..) = board_for(&s2, &ids);
        assert_eq!(codes(&check_consistency(&s2, &b)), vec![FOOTPRINT_DIFFERS]);
    }

    #[test]
    fn nets_must_match_as_groups() {
        let (s, ids) = schematic();
        // a pad of the net left without a net on the board
        let (mut b, _, cs, pads) = board_for(&s, &ids);
        b.pad_nets
            .retain(|p| !(p.component == cs[0] && p.pad == pads[1]));
        assert_eq!(codes(&check_consistency(&s, &b)), vec![NET_DIFFERS]);
        // the two pads on two different nets of the board
        let (mut b, _, cs, pads) = board_for(&s, &ids);
        let other = id(66);
        b.nets.insert(other, Net { name: "O".into() });
        for p in &mut b.pad_nets {
            if p.component == cs[0] && p.pad == pads[1] {
                p.net = other;
            }
        }
        assert_eq!(codes(&check_consistency(&s, &b)), vec![NET_DIFFERS]);
        // a board net that joins pins of two schematic nets: add a second net to the schematic
        // with the free pins (pin 1 of R1 and pin 2 of R2) and put them on the first board net
        let mut s2 = s.clone();
        let sym = &s2.symbols[&ids[0]];
        let pin_ids: Vec<Id> = sym.pins.keys().copied().collect();
        let mut pins = vec![
            NetPin {
                part: ids[1],
                pin: pin_ids[0],
            },
            NetPin {
                part: ids[2],
                pin: pin_ids[1],
            },
        ];
        pins.sort();
        s2.nets.insert(
            id(70),
            crate::model::SchematicNet {
                name: "M".into(),
                pins,
            },
        );
        let (mut b, net, cs, pads) = board_for(&s2, &ids);
        b.pad_nets.push(PadNet {
            component: cs[0],
            pad: pads[0],
            net,
        });
        b.pad_nets.push(PadNet {
            component: cs[1],
            pad: pads[1],
            net,
        });
        b.pad_nets.sort_by_key(|p| p.pad_ref());
        let r = check_consistency(&s2, &b);
        assert!(
            codes(&r).contains(&NET_DIFFERS.to_string()),
            "{:?}",
            r.errors
        );
        // with the original board the free pins of the second net have no net on the board
        let (b, ..) = board_for(&s2, &ids);
        let r = check_consistency(&s2, &b);
        assert!(r
            .errors
            .iter()
            .all(|d| d.code.as_deref() == Some(NET_DIFFERS)));
    }

    #[test]
    fn a_pin_without_a_pad_is_reported() {
        let (s, ids) = schematic();
        let (mut b, _, _, pads) = board_for(&s, &ids);
        for f in b.footprints.values_mut() {
            f.pads.remove(&pads[1]);
        }
        b.pad_nets.retain(|p| p.pad != pads[1]);
        let r = check_consistency(&s, &b);
        assert!(codes(&r).contains(&PIN_WITHOUT_PAD.to_string()));
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

    #[test]
    fn a_drawing_that_agrees_with_the_board_has_nothing_to_report() {
        let (s, ids, ..) = crate::sample::drawn_schematic();
        let (board, ..) = board_for(&s, &ids);
        let r = check_drawn_against_board(&s, &board);
        assert!(r.agrees() && r.warnings.is_empty(), "{:?}", codes(&r));
    }

    #[test]
    fn wires_that_join_pads_of_different_nets_of_the_board_are_an_error() {
        let (s, ids, ..) = crate::sample::drawn_schematic();
        let (mut board, _, comps, pads) = board_for(&s, &ids);
        // pad 1 of the second component goes to another net of the board
        let other = id(90);
        board.nets.insert(
            other,
            eh3d_phoenix_board::Net {
                name: "Other".into(),
            },
        );
        for p in board.pad_nets.iter_mut() {
            if p.component == comps[1] && p.pad == pads[0] {
                p.net = other;
            }
        }
        let r = check_drawn_against_board(&s, &board);
        assert_eq!(codes(&r), vec![DRAWN_JOINS_OTHER_NETS]);
        assert!(!r.agrees());
        // a pad without a net counts as a different net too
        board
            .pad_nets
            .retain(|p| !(p.component == comps[1] && p.pad == pads[0]));
        assert_eq!(
            codes(&check_drawn_against_board(&s, &board)),
            vec![DRAWN_JOINS_OTHER_NETS]
        );
    }

    #[test]
    fn a_net_of_the_board_drawn_in_pieces_is_a_warning() {
        let (mut s, ids, _, wire) = crate::sample::drawn_schematic();
        let (board, ..) = board_for(&s, &ids);
        s.wires.remove(&wire);
        let r = check_drawn_against_board(&s, &board);
        assert_eq!(codes(&r), vec![BOARD_NET_NOT_DRAWN]);
        assert!(r.agrees());
        // a schematic that is not drawn has nothing to compare
        let (plain, plain_ids) = schematic();
        let (board, ..) = board_for(&plain, &plain_ids);
        let r = check_drawn_against_board(&plain, &board);
        assert!(r.agrees() && r.warnings.is_empty());
    }
}
