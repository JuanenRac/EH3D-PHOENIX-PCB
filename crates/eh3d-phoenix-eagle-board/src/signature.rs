//! A description of a board without its ids, and the round trip through an EAGLE file.
//!
//! The signature is a sorted list of text lines, one per layer, footprint, pad, component, net,
//! contact, wire piece, via, zone, silkscreen piece and the outline. Ids, the names of layers, the
//! name of the board, the grouping of wires into tracks and the difference between a silkscreen
//! text and the strokes it prints do not appear in it; everything an EAGLE file can say does.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_board::{Board, Id, PadShape, Segment};
use eh3d_phoenix_model::Point;

use crate::layers::{assign, effective_copper};
use crate::reader::read_board;
use crate::writer::write_board;

fn pt(p: Point) -> String {
    format!("{},{}", p.x_nm.get(), p.y_nm.get())
}

fn seg(s: &Segment) -> String {
    match s {
        Segment::Line { start, end } => format!("L {} {}", pt(*start), pt(*end)),
        Segment::Arc { start, end, sweep } => {
            format!("A {} {} {}", pt(*start), pt(*end), sweep.get())
        }
    }
}

fn shape(s: PadShape) -> String {
    match s {
        PadShape::Circle { diameter } => format!("circle {}", diameter.get()),
        PadShape::Rect { width, height } => format!("rect {} {}", width.get(), height.get()),
        PadShape::Oblong { width, height } => format!("oblong {} {}", width.get(), height.get()),
    }
}

/// The signature of a board: sorted lines, see the module documentation.
pub fn signature(board: &Board) -> Vec<String> {
    let (numbers, _) = assign(board);
    let key = |id: &Id| numbers.get(id).map_or("L?".to_owned(), |n| format!("L{n}"));
    let copper = effective_copper(board);
    let net_name = |id: &Id| board.nets.get(id).map_or("?", |n| n.name.as_str());
    let mut lines: Vec<String> = Vec::new();

    for n in numbers.values().collect::<BTreeSet<_>>() {
        lines.push(format!("layer L{n}"));
    }
    if board.outline.len() >= 3 {
        lines.push(format!(
            "outline {}",
            board
                .outline
                .iter()
                .map(|p| pt(*p))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    for fp in board.footprints.values() {
        lines.push(format!("footprint {}", fp.name));
        for pad in fp.pads.values() {
            let layers = if pad.drill.is_some() {
                "all".to_owned()
            } else {
                copper
                    .iter()
                    .find(|l| pad.layers.contains(l))
                    .map_or("none".to_owned(), key)
            };
            lines.push(format!(
                "pad {} {} {} at {} rot {} drill {} on {}",
                fp.name,
                pad.number,
                shape(pad.shape),
                pt(pad.at),
                pad.rotation.normalised().get(),
                pad.drill.map_or(-1, |d| d.get()),
                layers
            ));
        }
    }
    for c in board.components.values() {
        let footprint = board
            .footprints
            .get(&c.footprint)
            .map_or("?", |f| f.name.as_str());
        lines.push(format!(
            "component {} value {} footprint {} at {} rot {} side {:?}",
            c.refdes,
            c.value,
            footprint,
            pt(c.at),
            c.rotation.normalised().get(),
            c.side
        ));
    }
    for n in board.nets.values() {
        lines.push(format!("net {}", n.name));
    }
    for pn in &board.pad_nets {
        let contact = board.components.get(&pn.component).and_then(|c| {
            let number = &board
                .footprints
                .get(&c.footprint)?
                .pads
                .get(&pn.pad)?
                .number;
            Some(format!("{} {}", c.refdes, number))
        });
        if let Some(contact) = contact {
            lines.push(format!("contact {} on {}", contact, net_name(&pn.net)));
        }
    }
    for t in board.tracks.values() {
        for s in &t.path {
            lines.push(format!(
                "wire {} {} {} {}",
                net_name(&t.net),
                key(&t.layer),
                t.width.get(),
                seg(s)
            ));
        }
    }
    for v in board.vias.values() {
        let (a, b) = (numbers.get(&v.from_layer), numbers.get(&v.to_layer));
        let extent = match (a, b) {
            (Some(a), Some(b)) => format!("{}-{}", a.min(b), a.max(b)),
            _ => "?".to_owned(),
        };
        lines.push(format!(
            "via {} at {} drill {} diameter {} extent {}",
            net_name(&v.net),
            pt(v.at),
            v.drill.get(),
            v.diameter.get(),
            extent
        ));
    }
    for z in board.zones.values() {
        lines.push(format!(
            "zone {} {} clearance {} {}",
            net_name(&z.net),
            key(&z.layer),
            z.clearance.get(),
            z.outline
                .iter()
                .map(|p| pt(*p))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    for item in board.silkscreen.values() {
        if let Ok(d) = item.drawn() {
            for s in &d.pieces {
                lines.push(format!(
                    "silk {} {} {}",
                    key(&item.layer()),
                    d.width.get(),
                    seg(s)
                ));
            }
        }
    }
    lines.sort();
    lines
}

/// The outcome of writing a board as an EAGLE file and reading the file back.
#[derive(Debug, Clone)]
pub struct RoundTrip {
    /// True when the signature of the board read back equals that of the original.
    pub identical: bool,
    /// Lines of the original that the copy does not have (a line repeated is counted).
    pub only_in_original: Vec<String>,
    /// Lines of the copy that the original does not have.
    pub only_in_copy: Vec<String>,
    /// The quality report of the writer and the reader together.
    pub diagnostics: Vec<Diagnostic>,
    /// The file text.
    pub text: String,
}

/// Writes `board` as an EAGLE file, reads it back with ids from `new_id` and compares the
/// signatures of the two boards.
pub fn round_trip(board: &Board, new_id: &mut dyn FnMut() -> Id) -> Result<RoundTrip, Diagnostic> {
    let written = write_board(board);
    let read = read_board(&written.text, new_id)?;
    let (before, after) = (signature(board), signature(&read.board));
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for l in &before {
        *counts.entry(l.as_str()).or_insert(0) += 1;
    }
    for l in &after {
        *counts.entry(l.as_str()).or_insert(0) -= 1;
    }
    let mut only_in_original = Vec::new();
    let mut only_in_copy = Vec::new();
    for (line, n) in counts {
        for _ in 0..n.max(0) {
            only_in_original.push(line.to_owned());
        }
        for _ in 0..(-n).max(0) {
            only_in_copy.push(line.to_owned());
        }
    }
    let mut diagnostics = written.diagnostics;
    diagnostics.extend(read.diagnostics);
    Ok(RoundTrip {
        identical: before == after,
        only_in_original,
        only_in_copy,
        diagnostics,
        text: written.text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{
        Component, Footprint, Layer, LayerKind, Net, Pad, PadNet, Side, SilkItem, Track, Via, Zone,
    };
    use eh3d_phoenix_eagle::{read_bytes, Limits};
    use eh3d_phoenix_model::{Nm, ObjectId, Udeg};

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }
    fn p(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }
    fn nm(v: i64) -> Nm {
        Nm::new(v).unwrap()
    }

    /// A counter that gives ids for the board read back.
    fn ids() -> impl FnMut() -> Id {
        let mut n = 1000u16;
        move || {
            n += 1;
            id(n)
        }
    }

    /// A board that uses everything an EAGLE file can say. `shift` moves every coordinate and
    /// `turn` changes the rotation of the second component, so that many different boards come
    /// from one description.
    fn board(shift: i64, turn: i64) -> Board {
        let (top, inner, bottom, silk, mask, outline) = (id(1), id(2), id(3), id(4), id(5), id(6));
        let mut b = Board {
            outline: vec![
                p(shift, 0),
                p(shift + 30_000_000, 0),
                p(shift + 30_000_000, 20_000_000),
                p(shift, 20_000_000),
            ],
            ..Board::default()
        };
        for (l, name, kind, side) in [
            (top, "Top", LayerKind::Copper, None),
            (inner, "In", LayerKind::Copper, None),
            (bottom, "Bot", LayerKind::Copper, None),
            (silk, "Silk", LayerKind::Silkscreen, Some(Side::Top)),
            (mask, "Mask", LayerKind::SolderMask, Some(Side::Bottom)),
            (outline, "Edge", LayerKind::Outline, None),
        ] {
            b.layers.insert(
                l,
                Layer {
                    name: name.into(),
                    kind,
                    side,
                },
            );
        }
        b.copper_stack = vec![top, inner, bottom];
        let (n1, n2) = (id(10), id(11));
        b.nets.insert(n1, Net { name: "GND".into() });
        b.nets.insert(
            n2,
            Net {
                name: "SIG 1".into(),
            },
        );

        let (fp_r, fp_j) = (id(20), id(21));
        let (r1, r2, j1, j2) = (id(22), id(23), id(24), id(25));
        let mut pads = BTreeMap::new();
        pads.insert(
            r1,
            Pad {
                number: "1".into(),
                shape: PadShape::Rect {
                    width: nm(900_000),
                    height: nm(1_300_000),
                },
                at: p(-1_000_000, 0),
                rotation: Udeg::ZERO,
                drill: None,
                layers: vec![top],
            },
        );
        pads.insert(
            r2,
            Pad {
                number: "2".into(),
                shape: PadShape::Oblong {
                    width: nm(900_000),
                    height: nm(1_300_000),
                },
                at: p(1_000_000, 0),
                rotation: Udeg::new(45_500_000).unwrap(),
                drill: None,
                layers: vec![bottom],
            },
        );
        b.footprints.insert(
            fp_r,
            Footprint {
                name: "R0805".into(),
                pads,
            },
        );
        let mut pads = BTreeMap::new();
        pads.insert(
            j1,
            Pad {
                number: "A1".into(),
                shape: PadShape::Circle {
                    diameter: nm(1_600_000),
                },
                at: p(0, 0),
                rotation: Udeg::ZERO,
                drill: Some(nm(800_000)),
                layers: vec![top, inner, bottom],
            },
        );
        pads.insert(
            j2,
            Pad {
                number: "A2".into(),
                shape: PadShape::Rect {
                    width: nm(1_600_000),
                    height: nm(1_600_000),
                },
                at: p(2_540_000, 0),
                rotation: Udeg::ZERO,
                drill: Some(nm(900_000)),
                layers: vec![top, inner, bottom],
            },
        );
        b.footprints.insert(
            fp_j,
            Footprint {
                name: "HDR".into(),
                pads,
            },
        );
        let (c1, c2, c3) = (id(30), id(31), id(32));
        for (cid, refdes, fp, x, rot, side) in [
            (c1, "R1", fp_r, 5_000_000, 0, Side::Top),
            (c2, "R2", fp_r, 12_000_000, turn, Side::Bottom),
            (c3, "J1", fp_j, 20_000_000, 90_000_000, Side::Top),
        ] {
            b.components.insert(
                cid,
                Component {
                    refdes: refdes.into(),
                    value: format!("val {refdes}"),
                    symbol: None,
                    footprint: fp,
                    at: p(shift + x, 8_000_000),
                    rotation: Udeg::new(rot).unwrap(),
                    side,
                },
            );
        }
        b.pad_nets = vec![
            PadNet {
                component: c1,
                pad: r1,
                net: n1,
            },
            PadNet {
                component: c1,
                pad: r2,
                net: n2,
            },
            PadNet {
                component: c3,
                pad: j1,
                net: n1,
            },
        ];
        b.pad_nets.sort_by_key(|x| x.pad_ref());
        b.tracks.insert(
            id(40),
            Track {
                net: n1,
                layer: top,
                width: nm(250_000),
                path: vec![
                    Segment::Line {
                        start: p(shift + 4_000_000, 8_000_000),
                        end: p(shift + 8_000_000, 8_000_000),
                    },
                    Segment::Arc {
                        start: p(shift + 8_000_000, 8_000_000),
                        end: p(shift + 10_000_000, 10_000_000),
                        sweep: Udeg::new(90_000_000).unwrap(),
                    },
                ],
            },
        );
        b.vias.insert(
            id(41),
            Via {
                net: n1,
                at: p(shift + 10_000_000, 10_000_000),
                drill: nm(300_000),
                diameter: nm(600_000),
                from_layer: top,
                to_layer: bottom,
            },
        );
        b.zones.insert(
            id(42),
            Zone {
                net: n2,
                layer: bottom,
                outline: vec![
                    p(shift + 1_000_000, 1_000_000),
                    p(shift + 9_000_000, 1_000_000),
                    p(shift + 9_000_000, 5_000_000),
                ],
                clearance: nm(300_000),
            },
        );
        b.silkscreen.insert(
            id(50),
            SilkItem::Text {
                layer: silk,
                at: p(shift + 2_000_000, 15_000_000),
                text: "R1 10K".into(),
                height: nm(1_000_000),
                stroke: nm(120_000),
                rotation: Udeg::ZERO,
                mirrored: false,
            },
        );
        b.silkscreen.insert(
            id(51),
            SilkItem::Stroke {
                layer: silk,
                width: nm(150_000),
                path: vec![
                    Segment::Line {
                        start: p(shift + 1_000_000, 18_000_000),
                        end: p(shift + 9_000_000, 18_000_000),
                    },
                    Segment::Line {
                        start: p(shift + 9_000_000, 18_000_000),
                        end: p(shift + 9_000_000, 19_000_000),
                    },
                ],
            },
        );
        assert!(b.is_consistent(), "{:?}", b.validate());
        b
    }

    #[test]
    fn a_board_with_everything_comes_back_identical() {
        let b = board(0, 30_000_000);
        // an oblong surface pad, a rectangular one, drilled round and square pads: all expressible
        let r = round_trip(&b, &mut ids()).unwrap();
        assert!(
            r.identical,
            "only in the original: {:#?}\nonly in the copy: {:#?}",
            r.only_in_original, r.only_in_copy
        );
        // the text is written as the strokes that are printed, and that is said
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some(crate::ITEM_SIMPLIFIED)));
        // a second writing of the same board is the same text
        assert_eq!(write_board(&b).text, r.text);
    }

    #[test]
    fn many_boards_come_back_identical() {
        for i in 0..60i64 {
            let b = board(i * 137_000, (i * 7_654_321) % 360_000_000);
            let r = round_trip(&b, &mut ids()).unwrap();
            assert!(
                r.identical,
                "board {i}: {:?} {:?}",
                r.only_in_original, r.only_in_copy
            );
        }
    }

    #[test]
    fn the_independent_reader_of_the_summary_agrees_with_the_file() {
        let b = board(0, 0);
        let text = write_board(&b).text;
        let reading = read_bytes(text.as_bytes(), &Limits::default());
        let summary = reading.summary.expect("the file is an EAGLE document");
        assert_eq!(summary.kind, eh3d_phoenix_eagle::DocumentKind::Board);
        assert_eq!(summary.version, "9.6.2");
        assert_eq!(summary.parts, 3);
        assert_eq!(summary.nets, 2);
        assert_eq!(summary.embedded_libraries, 1);
        // 2 wire pieces and a via in the signals, one polygon is counted apart
        assert!(summary.routed_items >= 3, "{}", summary.routed_items);
        assert!(reading
            .diagnostics
            .iter()
            .all(|d| d.severity != eh3d_diagnostic::Severity::Error));
    }

    #[test]
    fn what_the_format_cannot_say_is_reported_and_shows_in_the_comparison() {
        let mut b = board(0, 0);
        // a drilled oblong pad and a surface pad on two layers cannot be written as they are
        let fp = b.footprints.get_mut(&id(21)).unwrap();
        fp.pads.get_mut(&id(24)).unwrap().shape = PadShape::Oblong {
            width: nm(1_600_000),
            height: nm(2_400_000),
        };
        let fp = b.footprints.get_mut(&id(20)).unwrap();
        fp.pads.get_mut(&id(22)).unwrap().layers = vec![id(1), id(3)];
        let r = round_trip(&b, &mut ids()).unwrap();
        assert!(!r.identical);
        assert!(!r.only_in_original.is_empty() && !r.only_in_copy.is_empty());
        let simplified = r
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some(crate::ITEM_SIMPLIFIED))
            .count();
        assert!(simplified >= 2, "{simplified}");
    }

    #[test]
    fn items_without_a_place_in_the_file_are_left_out_with_a_warning() {
        let mut b = board(0, 0);
        // a component whose footprint is gone, and a track on a layer that does not exist
        b.components.get_mut(&id(30)).unwrap().footprint = id(99);
        b.tracks.get_mut(&id(40)).unwrap().layer = id(98);
        let w = write_board(&b);
        let left_out: Vec<_> = w
            .diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some(crate::ITEM_LEFT_OUT))
            .collect();
        assert!(left_out.len() >= 2, "{left_out:?}");
        assert!(left_out
            .iter()
            .all(|d| d.severity == eh3d_diagnostic::Severity::Warning));
    }

    #[test]
    fn a_file_that_is_not_a_board_is_an_error_and_damaged_text_never_panics() {
        for bad in [
            "not xml at all",
            "<eagle><drawing/></eagle>",
            "<other><drawing><board/></drawing></other>",
        ] {
            let e = read_board(bad, &mut ids()).unwrap_err();
            assert_eq!(e.code.as_deref(), Some(crate::UNREADABLE));
        }
        let text = write_board(&board(0, 0)).text;
        for cut in (0..text.len()).step_by(97) {
            if text.is_char_boundary(cut) {
                let _ = read_board(&text[..cut], &mut ids());
            }
        }
    }

    #[test]
    fn parts_not_read_are_counted_by_name() {
        let text = write_board(&board(0, 0)).text.replace(
            "<plain>",
            "<plain>\n<circle x=\"1\" y=\"1\" radius=\"1\" width=\"0.1\" layer=\"21\"/>\n<circle x=\"2\" y=\"1\" radius=\"1\" width=\"0.1\" layer=\"21\"/>",
        );
        let r = read_board(&text, &mut ids()).unwrap();
        let note = r
            .diagnostics
            .iter()
            .find(|d| d.code.as_deref() == Some(crate::NOT_READ))
            .expect("a note");
        assert!(note.message.contains("2 circle"), "{}", note.message);
    }
}
