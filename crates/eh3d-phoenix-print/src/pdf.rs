// SPDX-License-Identifier: GPL-3.0-or-later
//! The writer of PDF, made by hand and as small as it can be: a header, the objects, the table of
//! offsets and the trailer, with the length of every stream and every offset worked out while
//! writing. The streams are not compressed. The fonts are Helvetica and Courier with the
//! encoding WinAnsi, which every reader has; a character that they cannot show is drawn as an
//! empty box and counted. The identifier of the file is a constant and there is no date, so the
//! same drawings give the same bytes. The drawings are vectors: lines, closed and open paths,
//! circles made of four curves and text.

use crate::draw::{num, Anchor, Drawing, Item, Rgba, Shape};
use crate::font::{advance, text_width, winansi};

/// Points in a millimetre.
const PT: f64 = 72.0 / 25.4;

/// The control point of a Bezier curve that makes a quarter of a circle.
const KAPPA: f64 = 0.552_284_749_8;

/// The bytes of a PDF and what had to be drawn in another way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfOutput {
    /// The file.
    pub bytes: Vec<u8>,
    /// How many characters could not be shown by the fonts and were drawn as boxes.
    pub boxed_chars: usize,
}

fn pt(value: f64) -> String {
    num(value * PT)
}

fn alpha_levels(pages: &[Drawing]) -> Vec<u8> {
    let mut levels: Vec<u8> = Vec::new();
    for page in pages {
        let all = std::iter::once(page.background).chain(
            page.layers
                .iter()
                .flat_map(|l| l.items.iter().map(|i| i.colour)),
        );
        for colour in all {
            if colour.a != 255 && !levels.contains(&colour.a) {
                levels.push(colour.a);
            }
        }
    }
    levels.sort_unstable();
    levels
}

fn colour_ops(c: Rgba) -> String {
    let part = |v: u8| num(f64::from(v) / 255.0);
    let (r, g, b) = (part(c.r), part(c.g), part(c.b));
    format!("{r} {g} {b} rg {r} {g} {b} RG\n")
}

fn paint_op(fill: bool, width: f64, closed: bool) -> &'static str {
    match (fill, width > 0.0, closed) {
        (true, true, true) => "B",
        (true, false, true) => "f",
        (_, _, true) => "s",
        _ => "S",
    }
}

fn string_literal(bytes: &[u8]) -> String {
    let mut out = String::from("(");
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(char::from(b));
            }
            0x20..=0x7e => out.push(char::from(b)),
            _ => out.push_str(&format!("\\{b:03o}")),
        }
    }
    out.push(')');
    out
}

fn path_points(out: &mut String, points: &[(f64, f64)]) {
    for (i, (x, y)) in points.iter().enumerate() {
        out.push_str(&format!(
            "{} {} {}\n",
            pt(*x),
            pt(*y),
            if i == 0 { "m" } else { "l" }
        ));
    }
}

fn circle_path(out: &mut String, centre: (f64, f64), r: f64) {
    let (cx, cy) = centre;
    let k = KAPPA * r;
    let p = |x: f64, y: f64| format!("{} {}", pt(x), pt(y));
    out.push_str(&format!("{} m\n", p(cx + r, cy)));
    out.push_str(&format!(
        "{} {} {} c\n",
        p(cx + r, cy + k),
        p(cx + k, cy + r),
        p(cx, cy + r)
    ));
    out.push_str(&format!(
        "{} {} {} c\n",
        p(cx - k, cy + r),
        p(cx - r, cy + k),
        p(cx - r, cy)
    ));
    out.push_str(&format!(
        "{} {} {} c\n",
        p(cx - r, cy - k),
        p(cx - k, cy - r),
        p(cx, cy - r)
    ));
    out.push_str(&format!(
        "{} {} {} c\nh\n",
        p(cx + k, cy - r),
        p(cx + r, cy - k),
        p(cx + r, cy)
    ));
}

/// Writes a text as runs of characters of the font and boxes for the characters that it lacks.
fn text_ops(
    out: &mut String,
    at: (f64, f64),
    size: f64,
    anchor: Anchor,
    mono: bool,
    text: &str,
    boxed: &mut usize,
) {
    let width = text_width(text, size, mono);
    let mut x = match anchor {
        Anchor::Start => at.0,
        Anchor::Middle => at.0 - width / 2.0,
        Anchor::End => at.0 - width,
    };
    let font = if mono { "F2" } else { "F1" };
    let mut run: Vec<u8> = Vec::new();
    let mut run_x = x;
    let flush = |out: &mut String, run: &mut Vec<u8>, run_x: f64| {
        if !run.is_empty() {
            out.push_str(&format!(
                "BT /{font} {} Tf {} {} Td {} Tj ET\n",
                num(size * PT),
                pt(run_x),
                pt(at.1),
                string_literal(run)
            ));
            run.clear();
        }
    };
    for c in text.chars() {
        let step = f64::from(advance(c, mono)) * size / 1000.0;
        match winansi(c) {
            Some(byte) => {
                if run.is_empty() {
                    run_x = x;
                }
                run.push(byte);
            }
            None => {
                flush(out, &mut run, run_x);
                *boxed += 1;
                let (left, right) = (x + step * 0.1, x + step * 0.9);
                let (bottom, top) = (at.1, at.1 + size * 0.7);
                out.push_str("0.5 w\n");
                path_points(
                    out,
                    &[(left, bottom), (right, bottom), (right, top), (left, top)],
                );
                out.push_str("s\n");
            }
        }
        x += step;
    }
    flush(out, &mut run, run_x);
}

fn item_ops(out: &mut String, it: &Item, alphas: &[u8], boxed: &mut usize) {
    out.push_str("q\n");
    if it.colour.a != 255 {
        if let Some(index) = alphas.iter().position(|a| *a == it.colour.a) {
            out.push_str(&format!("/G{index} gs\n"));
        }
    }
    out.push_str(&colour_ops(it.colour));
    if it.width > 0.0 {
        out.push_str(&format!("{} w\n", pt(it.width)));
    }
    match &it.shape {
        Shape::Line { a, b } => {
            path_points(out, &[*a, *b]);
            out.push_str("S\n");
        }
        Shape::Poly { points, closed } => {
            if points.len() >= 2 {
                path_points(out, points);
                out.push_str(paint_op(it.fill, it.width, *closed));
                out.push('\n');
            }
        }
        Shape::Circle { centre, radius } => {
            circle_path(out, *centre, *radius);
            out.push_str(paint_op(it.fill, it.width, true));
            out.push('\n');
        }
        Shape::Text {
            at,
            size,
            anchor,
            mono,
            text,
        } => text_ops(out, *at, *size, *anchor, *mono, text, boxed),
    }
    out.push_str("Q\n");
}

fn page_stream(page: &Drawing, alphas: &[u8], boxed: &mut usize) -> Vec<u8> {
    let mut out = String::from("1 J 1 j\n");
    // the paper
    out.push_str("q\n");
    if page.background.a != 255 {
        if let Some(index) = alphas.iter().position(|a| *a == page.background.a) {
            out.push_str(&format!("/G{index} gs\n"));
        }
    }
    out.push_str(&colour_ops(page.background));
    out.push_str(&format!(
        "0 0 {} {} re\nf\nQ\n",
        pt(page.width),
        pt(page.height)
    ));
    for layer in &page.layers {
        for it in &layer.items {
            item_ops(&mut out, it, alphas, boxed);
        }
    }
    out.into_bytes()
}

/// A text for the information of the file: the characters that WinAnsi lacks become `?`.
fn info_text(text: &str) -> String {
    let bytes: Vec<u8> = text
        .chars()
        .map(|c| winansi(c).filter(|b| *b >= 0x20).unwrap_or(b'?'))
        .collect();
    string_literal(&bytes)
}

fn push_object(file: &mut Vec<u8>, offsets: &mut Vec<usize>, id: usize, body: &[u8]) {
    debug_assert_eq!(offsets.len() + 1, id);
    offsets.push(file.len());
    file.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    file.extend_from_slice(body);
    file.extend_from_slice(b"\nendobj\n");
}

/// Writes pages as a PDF. The pages have the size of their drawings.
pub fn to_pdf(pages: &[Drawing]) -> PdfOutput {
    let alphas = alpha_levels(pages);
    let first_page = 7 + alphas.len();
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    let mut offsets: Vec<usize> = Vec::new();
    let mut boxed = 0;

    push_object(
        &mut file,
        &mut offsets,
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>",
    );
    let kids: Vec<String> = (0..pages.len())
        .map(|k| format!("{} 0 R", first_page + 2 * k))
        .collect();
    push_object(
        &mut file,
        &mut offsets,
        2,
        format!(
            "<< /Type /Pages /Count {} /Kids [{}] >>",
            pages.len(),
            kids.join(" ")
        )
        .as_bytes(),
    );
    for (id, name) in [(3, "Helvetica"), (4, "Courier")] {
        push_object(
            &mut file,
            &mut offsets,
            id,
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>"
            )
            .as_bytes(),
        );
    }
    let title = pages.first().map_or(String::new(), |p| p.title.clone());
    push_object(
        &mut file,
        &mut offsets,
        5,
        format!(
            "<< /Title {} /Producer (EH3D-PHOENIX-PCB) >>",
            info_text(&title)
        )
        .as_bytes(),
    );
    let gs_refs: Vec<String> = (0..alphas.len())
        .map(|i| format!("/G{i} {} 0 R", 7 + i))
        .collect();
    let states = if gs_refs.is_empty() {
        String::new()
    } else {
        format!(" /ExtGState << {} >>", gs_refs.join(" "))
    };
    push_object(
        &mut file,
        &mut offsets,
        6,
        format!("<< /Font << /F1 3 0 R /F2 4 0 R >>{states} /ProcSet [/PDF /Text] >>").as_bytes(),
    );
    for (i, level) in alphas.iter().enumerate() {
        let a = num(f64::from(*level) / 255.0);
        push_object(
            &mut file,
            &mut offsets,
            7 + i,
            format!("<< /Type /ExtGState /ca {a} /CA {a} >>").as_bytes(),
        );
    }
    for (k, page) in pages.iter().enumerate() {
        let id = first_page + 2 * k;
        push_object(
            &mut file,
            &mut offsets,
            id,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources 6 0 R /Contents {} 0 R >>",
                pt(page.width),
                pt(page.height),
                id + 1
            )
            .as_bytes(),
        );
        let stream = page_stream(page, &alphas, &mut boxed);
        let mut body: Vec<u8> = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
        body.extend_from_slice(&stream);
        body.extend_from_slice(b"\nendstream");
        push_object(&mut file, &mut offsets, id + 1, &body);
    }

    let xref = file.len();
    let count = offsets.len() + 1;
    file.extend_from_slice(format!("xref\n0 {count}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let zeros = "0".repeat(32);
    file.extend_from_slice(
        format!(
            "trailer\n<< /Size {count} /Root 1 0 R /Info 5 0 R /ID [<{zeros}> <{zeros}>] >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    PdfOutput {
        bytes: file,
        boxed_chars: boxed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Layer, LayerKind};

    fn page(text: &str) -> Drawing {
        let red = Rgba::new(255, 0, 0, 128);
        let black = Rgba::new(0, 0, 0, 255);
        Drawing {
            width: 100.0,
            height: 50.0,
            title: "Title (one)".to_owned(),
            scale: 1.0,
            background: Rgba::new(255, 255, 255, 255),
            layers: vec![Layer {
                kind: LayerKind::Outline,
                items: vec![
                    Item::line((0.0, 0.0), (10.0, 5.0), black, 0.25),
                    Item::rect((1.0, 1.0), (4.0, 3.0), red, 0.0, true),
                    Item::circle((5.0, 5.0), 1.5, black, 0.1, true),
                    Item::text((2.0, 2.0), 3.0, Anchor::End, false, text, black),
                ],
            }],
        }
    }

    #[test]
    fn the_same_pages_give_the_same_bytes_without_a_date() {
        let a = to_pdf(&[page("R1"), page("R2")]);
        assert_eq!(a, to_pdf(&[page("R1"), page("R2")]));
        let text = String::from_utf8_lossy(&a.bytes).into_owned();
        assert!(text.starts_with("%PDF-1.4\n"));
        assert!(text.ends_with("%%EOF\n"));
        assert!(!text.contains("CreationDate"));
        assert!(text.contains(
            "/ID [<00000000000000000000000000000000> <00000000000000000000000000000000>]"
        ));
        assert!(text.contains("/Count 2"));
    }

    #[test]
    fn transparency_uses_a_graphics_state() {
        let out = to_pdf(&[page("R1")]);
        let text = String::from_utf8_lossy(&out.bytes).into_owned();
        assert!(text.contains("/Type /ExtGState /ca 0.502 /CA 0.502"));
        assert!(text.contains("/G0 gs"));
    }

    #[test]
    fn a_character_the_fonts_lack_is_drawn_as_a_box_and_counted() {
        let out = to_pdf(&[page("R1 縮")]);
        assert_eq!(out.boxed_chars, 1);
        let plain = to_pdf(&[page("R1 é")]);
        assert_eq!(plain.boxed_chars, 0);
        let text = String::from_utf8_lossy(&plain.bytes).into_owned();
        assert!(text.contains("(R1 \\351) Tj"));
    }

    #[test]
    fn the_strings_are_escaped() {
        assert_eq!(string_literal(b"a(b)c\\"), "(a\\(b\\)c\\\\)");
        assert_eq!(string_literal(&[0x0a, 0xe9]), "(\\012\\351)");
        assert_eq!(info_text("縮a"), "(?a)");
    }

    #[test]
    fn a_closed_path_is_filled_or_stroked_by_its_paint() {
        assert_eq!(paint_op(true, 0.0, true), "f");
        assert_eq!(paint_op(true, 1.0, true), "B");
        assert_eq!(paint_op(false, 1.0, true), "s");
        assert_eq!(paint_op(false, 1.0, false), "S");
    }
}
