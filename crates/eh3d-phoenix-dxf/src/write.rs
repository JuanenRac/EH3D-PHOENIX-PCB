// SPDX-License-Identifier: GPL-3.0-or-later
//! The writer of DXF release 12 in ASCII, the dialect the most programs read. The text is a pure
//! function of the drawing: no date, no host, no program version, the layers in the order of
//! their names, the entities in the order they were made, lengths in millimetres with up to six
//! decimals. A path is a `POLYLINE` with `VERTEX` entities and the bulge of an arc on the vertex
//! it starts at; a circle, a point and a text are the entities of the same names. No program has
//! opened the files it writes; the reader of this crate reads them back.

use std::fmt::Write as _;

use crate::geom::{sweep_to_bulge, P};
use crate::model::{Drawing, Shape};

/// The colour numbers that the layers take in turn.
const COLOURS: [u8; 7] = [7, 1, 2, 3, 4, 5, 6];

/// Nanometres as millimetres: up to six decimals, at least one.
fn mm(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    let digits = format!("{:06}", a % 1_000_000);
    let trimmed = digits.trim_end_matches('0');
    let fraction = if trimmed.is_empty() { "0" } else { trimmed };
    format!("{sign}{}.{fraction}", a / 1_000_000)
}

/// A bulge with up to twelve decimals, at least one.
fn real(value: f64) -> String {
    let text = format!("{value:.12}");
    let trimmed = text.trim_end_matches('0');
    let text = if trimmed.ends_with('.') {
        format!("{trimmed}0")
    } else {
        trimmed.to_owned()
    };
    if text == "-0.0" {
        "0.0".to_owned()
    } else {
        text
    }
}

/// A name that DXF accepts for a layer: the characters it forbids become `_`, a name that is
/// empty becomes `0`.
pub fn layer_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>/\\\":;?*|=`,".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    if cleaned.is_empty() {
        "0".to_owned()
    } else {
        cleaned
    }
}

/// A text on one line: control characters become blanks.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn pair(out: &mut String, code: i32, value: &str) {
    let _ = writeln!(out, "{code:>3}\n{value}");
}

fn point3(out: &mut String, base: i32, p: P) {
    pair(out, base, &mm(p.0));
    pair(out, base + 10, &mm(p.1));
    pair(out, base + 20, "0.0");
}

/// The drawing as the text of a DXF file of release 12.
pub fn write_dxf(drawing: &Drawing) -> String {
    let mut out = String::new();
    pair(&mut out, 0, "SECTION");
    pair(&mut out, 2, "HEADER");
    pair(&mut out, 9, "$ACADVER");
    pair(&mut out, 1, "AC1009");
    pair(&mut out, 9, "$INSUNITS");
    pair(&mut out, 70, "     4");
    pair(&mut out, 9, "$MEASUREMENT");
    pair(&mut out, 70, "     1");
    pair(&mut out, 0, "ENDSEC");

    let mut names: Vec<String> = drawing
        .entities
        .iter()
        .map(|e| layer_name(&e.layer))
        .collect();
    names.sort();
    names.dedup();
    pair(&mut out, 0, "SECTION");
    pair(&mut out, 2, "TABLES");
    pair(&mut out, 0, "TABLE");
    pair(&mut out, 2, "LAYER");
    pair(&mut out, 70, &format!("{:>6}", names.len()));
    for (i, name) in names.iter().enumerate() {
        pair(&mut out, 0, "LAYER");
        pair(&mut out, 2, name);
        pair(&mut out, 70, "     0");
        pair(&mut out, 62, &format!("{:>6}", COLOURS[i % COLOURS.len()]));
        pair(&mut out, 6, "CONTINUOUS");
    }
    pair(&mut out, 0, "ENDTAB");
    pair(&mut out, 0, "ENDSEC");

    pair(&mut out, 0, "SECTION");
    pair(&mut out, 2, "ENTITIES");
    for entity in &drawing.entities {
        let layer = layer_name(&entity.layer);
        match &entity.shape {
            Shape::Path { segs, closed } => {
                let (Some(first), Some(last)) = (segs.first(), segs.last()) else {
                    continue;
                };
                pair(&mut out, 0, "POLYLINE");
                pair(&mut out, 8, &layer);
                pair(&mut out, 66, "     1");
                point3(&mut out, 10, (0, 0));
                pair(&mut out, 70, if *closed { "     1" } else { "     0" });
                for seg in segs {
                    vertex(&mut out, &layer, seg.a, seg.sweep_udeg);
                }
                // an open path ends with its last point; a path that says it is closed but does
                // not end where it starts keeps its last point too, and the flag closes it
                if !*closed || first.a != last.b {
                    vertex(&mut out, &layer, last.b, 0);
                }
                pair(&mut out, 0, "SEQEND");
                pair(&mut out, 8, &layer);
            }
            Shape::Circle { centre, radius } => {
                pair(&mut out, 0, "CIRCLE");
                pair(&mut out, 8, &layer);
                point3(&mut out, 10, *centre);
                pair(&mut out, 40, &mm(*radius));
            }
            Shape::Point { at } => {
                pair(&mut out, 0, "POINT");
                pair(&mut out, 8, &layer);
                point3(&mut out, 10, *at);
            }
            Shape::Text { at, height, text } => {
                pair(&mut out, 0, "TEXT");
                pair(&mut out, 8, &layer);
                point3(&mut out, 10, *at);
                pair(&mut out, 40, &mm(*height));
                pair(&mut out, 1, &one_line(text));
            }
        }
    }
    pair(&mut out, 0, "ENDSEC");
    pair(&mut out, 0, "EOF");
    out
}

fn vertex(out: &mut String, layer: &str, at: P, sweep_udeg: i64) {
    pair(out, 0, "VERTEX");
    pair(out, 8, layer);
    point3(out, 10, at);
    if sweep_udeg != 0 {
        pair(out, 42, &real(sweep_to_bulge(sweep_udeg)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Seg;

    #[test]
    fn lengths_are_millimetres_with_the_digits_they_have() {
        assert_eq!(mm(0), "0.0");
        assert_eq!(mm(1_000_000), "1.0");
        assert_eq!(mm(-2_500_000), "-2.5");
        assert_eq!(mm(1), "0.000001");
        assert_eq!(mm(-1_234_567), "-1.234567");
        assert_eq!(real(0.0), "0.0");
        assert_eq!(real(1.0), "1.0");
        assert_eq!(real(-0.0000000000001), "0.0");
        assert_eq!(real(0.41421356237), "0.41421356237");
    }

    #[test]
    fn a_layer_name_loses_what_dxf_forbids() {
        assert_eq!(layer_name("A/B:C"), "A_B_C");
        assert_eq!(layer_name(""), "0");
        assert_eq!(layer_name("EDGE"), "EDGE");
    }

    #[test]
    fn a_closed_square_is_written_the_way_release_12_writes_a_polyline() {
        let mut d = Drawing::new();
        d.polygon("OUTLINE", &[(0, 0), (2_000_000, 0), (2_000_000, 1_000_000)]);
        let text = write_dxf(&d);
        let expected_polyline = "  0\nPOLYLINE\n  8\nOUTLINE\n 66\n     1\n 10\n0.0\n 20\n0.0\n 30\n0.0\n 70\n     1\n  0\nVERTEX\n  8\nOUTLINE\n 10\n0.0\n 20\n0.0\n 30\n0.0\n  0\nVERTEX\n  8\nOUTLINE\n 10\n2.0\n 20\n0.0\n 30\n0.0\n  0\nVERTEX\n  8\nOUTLINE\n 10\n2.0\n 20\n1.0\n 30\n0.0\n  0\nSEQEND\n  8\nOUTLINE\n";
        assert!(text.contains(expected_polyline), "{text}");
        assert!(text.starts_with("  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1009\n"));
        assert!(text.ends_with("  0\nENDSEC\n  0\nEOF\n"));
        // the same drawing gives the same bytes
        assert_eq!(text, write_dxf(&d));
    }

    #[test]
    fn an_arc_is_the_bulge_of_the_vertex_it_starts_at() {
        let mut d = Drawing::new();
        d.path(
            "A",
            vec![Seg {
                a: (0, 0),
                b: (2_000_000, 0),
                sweep_udeg: 180_000_000,
            }],
            false,
        );
        let text = write_dxf(&d);
        assert!(text.contains(" 42\n1.0\n"), "{text}");
        assert!(text.contains("  0\nSEQEND\n"));
    }

    #[test]
    fn circles_points_and_texts_are_written_and_the_layers_are_in_order_of_name() {
        let mut d = Drawing::new();
        d.circle("Z", (1_000_000, 2_000_000), 500_000);
        d.point("A", (0, 0));
        d.text("A", (0, 0), 1_000_000, "R1\nx");
        let text = write_dxf(&d);
        assert!(text.contains("  0\nCIRCLE\n  8\nZ\n 10\n1.0\n 20\n2.0\n 30\n0.0\n 40\n0.5\n"));
        assert!(text.contains("  0\nPOINT\n  8\nA\n"));
        assert!(
            text.contains("  0\nTEXT\n  8\nA\n 10\n0.0\n 20\n0.0\n 30\n0.0\n 40\n1.0\n  1\nR1 x\n")
        );
        let a = text.find("  2\nA\n").unwrap();
        let z = text.find("  2\nZ\n").unwrap();
        assert!(a < z);
        assert!(text.contains(" 70\n     2\n  0\nLAYER"));
    }
}
