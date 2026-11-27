// SPDX-License-Identifier: GPL-3.0-or-later
//! The netlist of a board as an IPC-D-356A test file: one line of 80 columns for each pad of a
//! part, with the net, the reference designator, the pin, the hole, the side it can be reached
//! from, the place and the size of the pad. Bare-board testers read this file to find out which
//! pads are joined.
//!
//! **State: prototype.** The columns follow the layout that is described below, which has been
//! written from the published description as it is remembered and has been read back by nothing
//! but the tests of this repository: it has not been compared with the standard, with a CAM tool
//! or with a tester. The units are micrometres (`P  UNITS CUST 1`).
//!
//! ```text
//! cols  1- 3  operation: 317 a pad with a hole, 327 a pad on the surface
//! cols  4-17  net, 14 characters (a longer name is cut and told in the notes)
//! col  18     blank
//! cols 19-24  reference designator, 6 characters
//! col  25     -
//! cols 26-29  pin, 4 characters
//! col  30     blank (the midpoint mark, never set)
//! cols 31-36  D, the diameter of the hole in micrometres (4 digits), P plated or U not plated;
//!             blanks for a pad without a hole
//! cols 37-39  A and the side the pad is reached from: 00 both (a hole), 01 top, 02 bottom
//! cols 40-47  X and the place, in micrometres with a sign and 6 digits
//! cols 48-55  Y and the same
//! cols 56-60  X and the size of the pad in x, in micrometres, 4 digits
//! cols 61-65  Y and the size in y
//! cols 66-69  R and the turn of the pad in degrees, 3 digits
//! cols 70-72  blank, S and the solder mask: 0 both sides, 1 top, 2 bottom, 3 covered (a via)
//! ```
//!
//! The file starts with a comment, the name of the board and the units, ends with `999`, and
//! every line is padded with blanks to 80 columns. The records are in the order of the net, the
//! reference designator and the pin, so the same board always gives the same file. A pad on no
//! net takes the name of `d356_unconnected_name`. A name is written in printable ASCII (a blank
//! becomes `_`, any other character `?`). What does not fit (a name that is cut, a place beyond
//! 999.999 mm, a size or a hole beyond 9.999 mm, a pad whose place is not known) is told in the
//! notes, and a pad that cannot be placed is left out.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Side};

use crate::codes::{Ipc2581Error, LIMIT_EXCEEDED};
use crate::settings::Ipc2581Settings;

/// The width of every line of the file, in columns.
pub const RECORD_WIDTH: usize = 80;

/// The largest distance from the origin that fits the six digits of a place, in micrometres.
const MAX_PLACE_UM: i128 = 999_999;

/// The largest size of a pad or hole that fits four digits, in micrometres.
const MAX_SIZE_UM: i128 = 9_999;

/// The file of a board and what is told about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D356 {
    /// The text of the file, one line of 80 columns after another, each ended by a line feed.
    pub text: String,
    /// How many pad records it holds.
    pub records: usize,
    /// What does not fit or was left out, in English, one line each, in a fixed order.
    pub notes: Vec<String>,
}

/// Nanometres as micrometres, rounded half away from zero.
fn um(nm: i64) -> i128 {
    let nm = i128::from(nm);
    let half = if nm >= 0 { 500 } else { -500 };
    (nm + half) / 1000
}

/// A word in printable ASCII: a blank becomes `_`, any other character `?`.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' ' => '_',
            c if c.is_ascii_graphic() => c,
            _ => '?',
        })
        .collect()
}

/// The name each of `names` is written as in a column of `width`. Two different names must not
/// come out the same, or a tester takes two nets (or parts, or pins) for one: a name whose cut is
/// already another one ends in `~1`, `~2` and so on, and is noted.
fn short_names(
    names: impl Iterator<Item = String>,
    width: usize,
    what: &str,
    notes: &mut BTreeSet<String>,
) -> BTreeMap<String, String> {
    let names: BTreeSet<String> = names.collect();
    // the names that fit as they are keep them
    let mut taken: BTreeSet<String> = names
        .iter()
        .map(|n| printable(n))
        .filter(|n| n.len() <= width)
        .collect();
    let mut whole: BTreeSet<String> = BTreeSet::new();
    let mut out = BTreeMap::new();
    for name in names {
        let clean = printable(&name);
        let fits = clean.len() <= width;
        let mut short: String = clean.chars().take(width).collect();
        let clashes = if fits {
            !whole.insert(short.clone())
        } else {
            taken.contains(&short)
        };
        if clashes {
            let mut n = 1_usize;
            loop {
                let tail = format!("~{n}");
                let keep = width.saturating_sub(tail.len());
                short = clean.chars().take(keep).collect::<String>() + &tail;
                if !taken.contains(&short) || n > 99_999 {
                    break;
                }
                n += 1;
            }
            notes.insert(format!("the {what} {clean} is written as {short}"));
        } else if !fits {
            notes.insert(format!("the {what} {clean} is cut to {width} characters"));
        }
        taken.insert(short.clone());
        out.insert(name, short);
    }
    out
}

/// A word in printable ASCII, cut to `width` and padded with blanks to it. A cut is noted once.
fn field(text: &str, width: usize, what: &str, notes: &mut BTreeSet<String>) -> String {
    let clean = printable(text);
    let mut out: String = clean.chars().take(width).collect();
    if clean.chars().count() > width {
        notes.insert(format!("the {what} {clean} is cut to {width} characters"));
    }
    while out.chars().count() < width {
        out.push(' ');
    }
    out
}

/// A line padded with blanks to the width of a record.
fn padded(mut line: String) -> String {
    while line.len() < RECORD_WIDTH {
        line.push(' ');
    }
    line.push('\n');
    line
}

/// What is known of a pad that goes into a record.
struct Row {
    net: String,
    refdes: String,
    pin: String,
    hole_um: Option<i128>,
    plated: bool,
    side: Side,
    x_um: i128,
    y_um: i128,
    size_um: (i128, i128),
    turn: i64,
    /// A via: reached from both sides, with the solder mask over it.
    via: bool,
}

/// The size of a pad in x and y, in nanometres.
fn size_nm(shape: &PadShape) -> (i64, i64) {
    match shape {
        PadShape::Circle { diameter } => (diameter.get(), diameter.get()),
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            (width.get(), height.get())
        }
    }
}

/// Writes the netlist of a board as an IPC-D-356A test file. The records are the pads of the
/// parts and, with `d356_include_vias`, the vias that go through the board; tracks are not
/// written. With `d356_include_smd` off the pads without a hole are left out. More records than `max_items`
/// is refused with `PHOENIX-IPC2581-005`.
pub fn d356(board: &Board, settings: &Ipc2581Settings) -> Result<D356, Ipc2581Error> {
    let mut notes: BTreeSet<String> = BTreeSet::new();
    let net_of: BTreeMap<PadRef, Id> = board
        .pad_nets
        .iter()
        .map(|a| (a.pad_ref(), a.net))
        .collect();
    let mut rows: Vec<Row> = Vec::new();
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            if pad.drill.is_none() && !settings.d356_include_smd {
                continue;
            }
            let reference = PadRef {
                component: *component_id,
                pad: *pad_id,
            };
            let Some(at) = pad_position(board, reference) else {
                notes.insert(format!(
                    "the pad {} of {} has no known place and is left out",
                    pad.number, component.refdes
                ));
                continue;
            };
            let (x_um, y_um) = (um(at.x_nm.get()), um(at.y_nm.get()));
            if x_um.abs() > MAX_PLACE_UM || y_um.abs() > MAX_PLACE_UM {
                notes.insert(format!(
                    "the pad {} of {} is beyond 999.999 mm from the origin and is left out",
                    pad.number, component.refdes
                ));
                continue;
            }
            let net = net_of
                .get(&reference)
                .and_then(|n| board.nets.get(n))
                .map_or_else(
                    || settings.d356_unconnected_name.clone(),
                    |n| n.name.clone(),
                );
            let (sx, sy) = size_nm(&pad.shape);
            let turn = component.pad_angle_udeg(pad.rotation);
            rows.push(Row {
                net,
                refdes: component.refdes.clone(),
                pin: pad.number.clone(),
                hole_um: pad.drill.map(|d| um(d.get())),
                plated: pad.plated,
                side: component.side,
                x_um,
                y_um,
                size_um: (um(sx), um(sy)),
                turn: (turn + 500_000) / 1_000_000 % 360,
                via: false,
            });
        }
    }
    if settings.d356_include_vias {
        let stack = &board.copper_stack;
        let mut inner = 0_usize;
        for via in board.vias.values() {
            let (a, b) = (
                stack.iter().position(|l| *l == via.from_layer),
                stack.iter().position(|l| *l == via.to_layer),
            );
            // a via that does not reach both faces cannot be probed from both
            if let (Some(a), Some(b)) = (a, b) {
                if a != b && (a.min(b) != 0 || a.max(b) != stack.len() - 1) {
                    inner += 1;
                    continue;
                }
            }
            let (x_um, y_um) = (um(via.at.x_nm.get()), um(via.at.y_nm.get()));
            if x_um.abs() > MAX_PLACE_UM || y_um.abs() > MAX_PLACE_UM {
                notes.insert("a via is beyond 999.999 mm from the origin and is left out".into());
                continue;
            }
            let size = um(via.diameter.get());
            rows.push(Row {
                net: board.nets.get(&via.net).map_or_else(
                    || settings.d356_unconnected_name.clone(),
                    |n| n.name.clone(),
                ),
                refdes: "VIA".to_owned(),
                pin: String::new(),
                hole_um: Some(um(via.drill.get())),
                plated: true,
                side: Side::Top,
                x_um,
                y_um,
                size_um: (size, size),
                turn: 0,
                via: true,
            });
        }
        if inner > 0 {
            notes.insert(format!("{inner} blind or buried vias are left out"));
        }
    }
    if rows.len() > settings.max_items {
        return Err(Ipc2581Error::new(
            LIMIT_EXCEEDED,
            format!("{} pads, the limit is {}", rows.len(), settings.max_items),
        ));
    }
    rows.sort_by(|a, b| {
        (&a.net, &a.refdes, &a.pin, a.x_um, a.y_um)
            .cmp(&(&b.net, &b.refdes, &b.pin, b.x_um, b.y_um))
    });
    let nets = short_names(rows.iter().map(|r| r.net.clone()), 14, "net", &mut notes);
    let refs = short_names(
        rows.iter().map(|r| r.refdes.clone()),
        6,
        "reference designator",
        &mut notes,
    );
    let pins = short_names(rows.iter().map(|r| r.pin.clone()), 4, "pin", &mut notes);
    for row in &mut rows {
        row.net = nets.get(&row.net).cloned().unwrap_or_default();
        row.refdes = refs.get(&row.refdes).cloned().unwrap_or_default();
        row.pin = pins.get(&row.pin).cloned().unwrap_or_default();
    }
    let name = field(&board.name, 60, "name of the board", &mut notes);
    let name = if name.trim().is_empty() {
        "BOARD".to_owned()
    } else {
        name.trim_end().to_owned()
    };
    let mut text = String::new();
    text.push_str(&padded(
        "C  IPC-D-356A netlist, written by phoenix ipc2581 --d356".to_owned(),
    ));
    text.push_str(&padded(format!("P  JOB   {name}")));
    text.push_str(&padded("P  UNITS CUST 1".to_owned()));
    for row in &rows {
        text.push_str(&padded(record(row, &mut notes)));
    }
    text.push_str(&padded("999".to_owned()));
    Ok(D356 {
        text,
        records: rows.len(),
        notes: notes.into_iter().collect(),
    })
}

/// One record, without the padding.
fn record(row: &Row, notes: &mut BTreeSet<String>) -> String {
    let through = row.hole_um.is_some();
    let operation = if through { 317 } else { 327 };
    let net = field(&row.net, 14, "net", notes);
    let refdes = field(&row.refdes, 6, "reference designator", notes);
    let pin = field(&row.pin, 4, "pin", notes);
    let hole = match row.hole_um {
        Some(h) => {
            if h > MAX_SIZE_UM {
                notes.insert(format!(
                    "the hole of {}-{} is cut to 9.999 mm",
                    row.refdes, row.pin
                ));
            }
            format!(
                "D{:04}{}",
                h.clamp(0, MAX_SIZE_UM),
                if row.plated { 'P' } else { 'U' }
            )
        }
        None => "      ".to_owned(),
    };
    let (access, mask) = match (through, row.side) {
        _ if row.via => (0, 3),
        (true, _) => (0, 0),
        (false, Side::Top) => (1, 1),
        (false, Side::Bottom) => (2, 2),
    };
    let (sx, sy) = row.size_um;
    if sx > MAX_SIZE_UM || sy > MAX_SIZE_UM {
        notes.insert(format!(
            "the size of the pad {}-{} is cut to 9.999 mm",
            row.refdes, row.pin
        ));
    }
    format!(
        "{operation:03}{net} {refdes}-{pin} {hole}A{access:02}X{x:+07}Y{y:+07}X{sx:04}Y{sy:04}R{turn:03} S{mask}",
        x = row.x_um,
        y = row.y_um,
        sx = sx.clamp(0, MAX_SIZE_UM),
        sy = sy.clamp(0, MAX_SIZE_UM),
        turn = row.turn,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{Component, Footprint, Net, Pad, PadNet};
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

    const MM: i64 = 1_000_000;

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn pad(number: &str, at: Point, diameter: i64, drill: Option<i64>) -> Pad {
        Pad {
            number: number.into(),
            shape: PadShape::Circle {
                diameter: Nm::new(diameter).unwrap(),
            },
            at,
            rotation: Udeg::ZERO,
            drill: drill.map(|d| Nm::new(d).unwrap()),
            layers: vec![id(1)],
            roundness: 0,
            plated: true,
        }
    }

    /// A board with a footprint `F` (id 10) of the pads given and the parts `(id, reference, x,
    /// y, side)`.
    fn board(
        pads: Vec<(u32, Pad)>,
        parts: &[(u32, &str, i64, i64, Side)],
        nets: &[(u32, &str)],
    ) -> Board {
        let mut b = Board {
            name: "Demo board".into(),
            ..Board::default()
        };
        for (n, name) in nets {
            b.nets.insert(
                id(*n),
                Net {
                    name: (*name).into(),
                },
            );
        }
        let mut footprint = Footprint {
            name: "F".into(),
            pads: Default::default(),
        };
        for (n, p) in pads {
            footprint.pads.insert(id(n), p);
        }
        b.footprints.insert(id(10), footprint);
        for (n, refdes, x, y, side) in parts {
            b.components.insert(
                id(*n),
                Component {
                    refdes: (*refdes).into(),
                    value: String::new(),
                    symbol: None,
                    footprint: id(10),
                    at: pt(*x, *y),
                    rotation: Udeg::ZERO,
                    side: *side,
                },
            );
        }
        b
    }

    fn assign(b: &mut Board, component: u32, pad: u32, net: u32) {
        b.pad_nets.push(PadNet {
            component: id(component),
            pad: id(pad),
            net: id(net),
        });
        b.pad_nets.sort_by_key(PadNet::pad_ref);
    }

    fn settings() -> Ipc2581Settings {
        Ipc2581Settings::default()
    }

    fn lines(file: &D356) -> Vec<&str> {
        file.text.lines().collect()
    }

    #[test]
    fn a_pad_with_a_hole_is_written_in_the_columns_of_the_layout() {
        let mut b = board(
            vec![(11, pad("1", pt(-MM, 0), 1_600_000, Some(800_000)))],
            &[(20, "R1", 10 * MM, 20 * MM, Side::Top)],
            &[(2, "GND")],
        );
        assign(&mut b, 20, 11, 2);
        let file = d356(&b, &settings()).unwrap();
        assert_eq!(file.records, 1);
        assert!(file.notes.is_empty(), "{:?}", file.notes);
        let all = lines(&file);
        assert_eq!(all.len(), 5);
        assert!(all[0].starts_with("C  IPC-D-356A"));
        assert!(all[1].starts_with("P  JOB   Demo_board"));
        assert!(all[2].starts_with("P  UNITS CUST 1"));
        assert!(all[4].starts_with("999"));
        let r = all[3];
        assert_eq!(r.len(), 80);
        assert_eq!(&r[0..3], "317");
        assert_eq!(&r[3..17], "GND           ");
        assert_eq!(&r[17..18], " ");
        assert_eq!(&r[18..24], "R1    ");
        assert_eq!(&r[24..25], "-");
        assert_eq!(&r[25..29], "1   ");
        assert_eq!(&r[29..30], " ");
        assert_eq!(&r[30..36], "D0800P");
        assert_eq!(&r[36..39], "A00");
        assert_eq!(&r[39..47], "X+009000");
        assert_eq!(&r[47..55], "Y+020000");
        assert_eq!(&r[55..60], "X1600");
        assert_eq!(&r[60..65], "Y1600");
        assert_eq!(&r[65..69], "R000");
        assert_eq!(&r[69..72], " S0");
        assert!(r[72..].trim().is_empty());
    }

    #[test]
    fn a_surface_pad_on_the_bottom_is_mirrored_and_reached_from_the_bottom() {
        let mut b = board(
            vec![(11, pad("2", pt(MM, 0), 800_000, None))],
            &[(20, "C1", 10 * MM, 5 * MM, Side::Bottom)],
            &[(2, "VCC")],
        );
        assign(&mut b, 20, 11, 2);
        let file = d356(&b, &settings()).unwrap();
        let r = lines(&file)[3].to_owned();
        assert_eq!(&r[0..3], "327");
        assert_eq!(&r[30..36], "      ");
        assert_eq!(&r[36..39], "A02");
        // a part on the bottom is mirrored about the vertical axis: x is 10 mm less 1 mm
        assert_eq!(&r[39..47], "X+009000");
        assert_eq!(&r[47..55], "Y+005000");
        assert_eq!(&r[55..60], "X0800");
        assert_eq!(&r[69..72], " S2");
        let top = board(
            vec![(11, pad("2", pt(MM, 0), 800_000, None))],
            &[(20, "C1", 10 * MM, 5 * MM, Side::Top)],
            &[(2, "VCC")],
        );
        let r = lines(&d356(&top, &settings()).unwrap())[3].to_owned();
        assert_eq!(&r[36..39], "A01");
        assert_eq!(&r[39..47], "X+011000");
        assert_eq!(&r[69..72], " S1");
    }

    #[test]
    fn the_records_are_in_the_order_of_the_net_the_part_and_the_pin() {
        let mut b = board(
            vec![
                (11, pad("2", pt(0, 0), 800_000, None)),
                (12, pad("1", pt(MM, 0), 800_000, None)),
            ],
            &[
                (20, "R2", 0, 0, Side::Top),
                (21, "R1", 10 * MM, 0, Side::Top),
            ],
            &[(2, "SIG"), (3, "GND")],
        );
        assign(&mut b, 20, 11, 2);
        assign(&mut b, 20, 12, 3);
        assign(&mut b, 21, 11, 3);
        // the pin 1 of R1 is on no net
        let file = d356(&b, &settings()).unwrap();
        let keys: Vec<String> = lines(&file)[3..7]
            .iter()
            .map(|l| {
                format!(
                    "{}|{}|{}",
                    l[3..17].trim(),
                    l[18..24].trim(),
                    l[25..29].trim()
                )
            })
            .collect();
        assert_eq!(keys, ["GND|R1|2", "GND|R2|1", "N/C|R1|1", "SIG|R2|2"]);
        assert_eq!(d356(&b, &settings()).unwrap(), file);
    }

    #[test]
    fn a_name_is_made_printable_and_a_cut_is_told_once() {
        let mut b = board(
            vec![
                (11, pad("1", pt(0, 0), 800_000, None)),
                (12, pad("2", pt(MM, 0), 800_000, None)),
            ],
            &[(20, "REFERENCE1", 0, 0, Side::Top)],
            &[(2, "A very long net name"), (3, "caf\u{e9}")],
        );
        assign(&mut b, 20, 11, 2);
        assign(&mut b, 20, 12, 3);
        let file = d356(&b, &settings()).unwrap();
        let rows = lines(&file);
        assert_eq!(&rows[3][3..17], "A_very_long_ne");
        assert_eq!(&rows[4][3..17], "caf?          ");
        assert_eq!(&rows[3][18..24], "REFERE");
        assert!(file.text.is_ascii());
        assert!(file
            .notes
            .contains(&"the net A_very_long_net_name is cut to 14 characters".to_owned()));
        assert!(file
            .notes
            .contains(&"the reference designator REFERENCE1 is cut to 6 characters".to_owned()));
        // told once, though the part has two pads
        assert_eq!(
            file.notes
                .iter()
                .filter(|n| n.contains("REFERENCE1"))
                .count(),
            1
        );
    }

    #[test]
    fn two_names_that_cut_to_the_same_text_are_written_apart() {
        let mut b = board(
            vec![
                (11, pad("1", pt(0, 0), 800_000, None)),
                (12, pad("2", pt(MM, 0), 800_000, None)),
                (13, pad("3", pt(2 * MM, 0), 800_000, None)),
            ],
            &[(20, "R1", 0, 0, Side::Top)],
            &[
                (2, "ADDRESS_BUS_LINE_1"),
                (3, "ADDRESS_BUS_LINE_2"),
                // a name that is already what the second would be written as
                (4, "ADDRESS_BUS_~1"),
            ],
        );
        assign(&mut b, 20, 11, 2);
        assign(&mut b, 20, 12, 3);
        assign(&mut b, 20, 13, 4);
        let file = d356(&b, &settings()).unwrap();
        let mut nets: Vec<String> = lines(&file)[3..6]
            .iter()
            .map(|r| r[3..17].trim_end().to_owned())
            .collect();
        nets.sort();
        assert_eq!(
            nets,
            vec!["ADDRESS_BUS_LI", "ADDRESS_BUS_~1", "ADDRESS_BUS_~2"]
        );
        assert!(file
            .notes
            .contains(&"the net ADDRESS_BUS_LINE_2 is written as ADDRESS_BUS_~2".to_owned()));
        // two names that differ only in characters that are written as ? stay apart too
        let mut b = board(
            vec![
                (11, pad("1", pt(0, 0), 800_000, None)),
                (12, pad("2", pt(MM, 0), 800_000, None)),
            ],
            &[(20, "R1", 0, 0, Side::Top)],
            &[(2, "caf\u{e9}"), (3, "caf\u{e8}")],
        );
        assign(&mut b, 20, 11, 2);
        assign(&mut b, 20, 12, 3);
        let file = d356(&b, &settings()).unwrap();
        let rows = lines(&file);
        assert_ne!(&rows[3][3..17], &rows[4][3..17]);
    }

    #[test]
    fn the_turn_of_a_bottom_pad_is_taken_off_the_turn_of_its_part() {
        let mut turned = pad("1", pt(0, 0), 800_000, None);
        turned.rotation = Udeg::new(30_000_000).unwrap();
        let mut b = board(vec![(11, turned)], &[(20, "C1", 0, 0, Side::Bottom)], &[]);
        b.components.get_mut(&id(20)).unwrap().rotation = Udeg::new(90_000_000).unwrap();
        let file = d356(&b, &settings()).unwrap();
        assert_eq!(&lines(&file)[3][65..69], "R060");
        b.components.get_mut(&id(20)).unwrap().side = Side::Top;
        let file = d356(&b, &settings()).unwrap();
        assert_eq!(&lines(&file)[3][65..69], "R120");
    }

    #[test]
    fn the_vias_through_the_board_are_written_when_asked_for() {
        let mut b = board(
            vec![(11, pad("1", pt(0, 0), 800_000, None))],
            &[(20, "R1", 0, 0, Side::Top)],
            &[(2, "GND")],
        );
        assign(&mut b, 20, 11, 2);
        b.copper_stack = vec![id(1), id(8), id(9)];
        for (n, to, x) in [(30, 9, 5), (31, 8, 6)] {
            b.vias.insert(
                id(n),
                eh3d_phoenix_board::Via {
                    net: id(2),
                    at: pt(x * MM, 2 * MM),
                    drill: Nm::new(300_000).unwrap(),
                    diameter: Nm::new(600_000).unwrap(),
                    from_layer: id(1),
                    to_layer: id(to),
                },
            );
        }
        assert_eq!(d356(&b, &settings()).unwrap().records, 1);
        let asked = Ipc2581Settings::parse("d356_include_vias = true\n").unwrap();
        let file = d356(&b, &asked).unwrap();
        // the pad and the through via; the blind via is left out and told
        assert_eq!(file.records, 2);
        assert!(file
            .notes
            .contains(&"1 blind or buried vias are left out".to_owned()));
        let via = lines(&file)
            .into_iter()
            .find(|l| l[18..24].starts_with("VIA"))
            .unwrap()
            .to_owned();
        assert_eq!(via.len(), 80);
        assert_eq!(&via[0..17], "317GND           ");
        assert_eq!(&via[30..39], "D0300PA00");
        assert_eq!(&via[39..55], "X+005000Y+002000");
        assert_eq!(&via[55..65], "X0600Y0600");
        assert_eq!(&via[69..72], " S3");
    }

    #[test]
    fn surface_pads_can_be_left_out_and_the_unconnected_name_is_a_setting() {
        let b = board(
            vec![
                (11, pad("1", pt(0, 0), 800_000, None)),
                (12, pad("2", pt(MM, 0), 800_000, Some(500_000))),
            ],
            &[(20, "R1", 0, 0, Side::Top)],
            &[],
        );
        let mut s = settings();
        assert_eq!(d356(&b, &s).unwrap().records, 2);
        s.d356_include_smd = false;
        s.d356_unconnected_name = "NC".to_owned();
        let file = d356(&b, &s).unwrap();
        assert_eq!(file.records, 1);
        assert_eq!(&lines(&file)[3][0..3], "317");
        assert_eq!(&lines(&file)[3][3..17], "NC            ");
    }

    #[test]
    fn a_place_that_does_not_fit_is_left_out_and_sizes_are_cut_and_told() {
        let b = board(
            vec![
                (11, pad("1", pt(0, 0), 12 * MM, Some(11 * MM))),
                (12, pad("2", pt(1_000 * MM, 0), 800_000, None)),
            ],
            &[(20, "R1", 0, 0, Side::Top)],
            &[],
        );
        let file = d356(&b, &settings()).unwrap();
        assert_eq!(file.records, 1);
        let r = lines(&file)[3].to_owned();
        assert_eq!(&r[30..36], "D9999P");
        assert_eq!(&r[55..60], "X9999");
        assert_eq!(file.notes.len(), 3, "{:?}", file.notes);
        assert!(file.notes.iter().any(|n| n.contains("beyond 999.999 mm")));
        assert!(file.notes.iter().any(|n| n.contains("hole of R1-1")));
        assert!(file
            .notes
            .iter()
            .any(|n| n.contains("size of the pad R1-1")));
    }

    #[test]
    fn more_pads_than_the_limit_are_refused() {
        let b = board(
            vec![
                (11, pad("1", pt(0, 0), 800_000, None)),
                (12, pad("2", pt(MM, 0), 800_000, None)),
            ],
            &[(20, "R1", 0, 0, Side::Top)],
            &[],
        );
        let mut s = settings();
        s.max_items = 1;
        let error = d356(&b, &s).unwrap_err();
        assert_eq!(error.code, LIMIT_EXCEEDED);
        s.max_items = 2;
        assert!(d356(&b, &s).is_ok());
    }

    #[test]
    fn a_board_with_no_part_has_a_header_and_the_end_record() {
        let file = d356(&Board::default(), &settings()).unwrap();
        assert_eq!(file.records, 0);
        let all = lines(&file);
        assert_eq!(all.len(), 4);
        assert!(all[1].starts_with("P  JOB   BOARD"));
        assert!(all[3].starts_with("999"));
    }

    /// A small linear congruential generator, so the cases are the same on every run.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn range(&mut self, low: i64, high: i64) -> i64 {
            low + (self.next() % (high - low + 1) as u64) as i64
        }
    }

    #[test]
    fn generated_boards_give_lines_of_80_columns_in_order_with_one_record_for_each_pad() {
        let mut rng = Lcg(0x356a);
        for _round in 0..30 {
            let mut pads = Vec::new();
            for n in 0..rng.range(1, 4) {
                let drill = rng
                    .next()
                    .is_multiple_of(2)
                    .then(|| rng.range(300_000, 1_200_000));
                pads.push((
                    11 + n as u32,
                    pad(
                        &format!("{}", n + 1),
                        pt(rng.range(-3, 3) * MM, rng.range(-3, 3) * MM),
                        rng.range(600_000, 2_000_000),
                        drill,
                    ),
                ));
            }
            let pad_ids: Vec<u32> = pads.iter().map(|(n, _)| *n).collect();
            let mut parts = Vec::new();
            let names = ["R1", "R2", "C10", "U1", "J1", "D3"];
            let count = rng.range(1, 5) as usize;
            for (n, name) in names.iter().enumerate().take(count) {
                let side = if rng.next().is_multiple_of(3) {
                    Side::Bottom
                } else {
                    Side::Top
                };
                parts.push((
                    20 + n as u32,
                    *name,
                    rng.range(-300, 300) * MM,
                    rng.range(-300, 300) * MM,
                    side,
                ));
            }
            let mut b = board(pads, &parts, &[(2, "GND"), (3, "VCC"), (4, "SIG 1")]);
            for (part, ..) in &parts {
                for pad_id in &pad_ids {
                    if !rng.next().is_multiple_of(4) {
                        assign(&mut b, *part, *pad_id, 2 + (rng.next() % 3) as u32);
                    }
                }
            }
            let file = d356(&b, &settings()).unwrap();
            assert!(file.notes.is_empty(), "{:?}", file.notes);
            assert_eq!(file.records, parts.len() * pad_ids.len());
            let all = lines(&file);
            assert!(file.text.is_ascii() && file.text.ends_with('\n'));
            assert!(all.iter().all(|l| l.len() == 80));
            let records = &all[3..all.len() - 1];
            assert_eq!(records.len(), file.records);
            let keys: Vec<(String, String, String)> = records
                .iter()
                .map(|l| {
                    (
                        l[3..17].trim_end().to_owned(),
                        l[18..24].trim_end().to_owned(),
                        l[25..29].trim_end().to_owned(),
                    )
                })
                .collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted);
        }
    }
}
