// SPDX-License-Identifier: GPL-3.0-or-later
//! The writer of SVG: the same drawing gives the same bytes every time, with no date and nothing
//! random, numbers with the fewest exact decimals and text escaped. The units are millimetres
//! and the vertical axis is turned over, as SVG counts from the top.

use crate::draw::{num, Anchor, Drawing, Item, Rgba, Shape};

/// A text safe to put in an XML text or in the value of an attribute: the characters that XML
/// does not allow become a space and the markup characters are escaped.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(' '),
            c if u32::from(c) < 0x20 || c == '\u{7f}' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn paint(colour: Rgba, attribute: &str) -> String {
    let mut out = format!(" {attribute}=\"{}\"", colour.hex());
    if colour.a != 255 {
        out.push_str(&format!(
            " {attribute}-opacity=\"{}\"",
            num(colour.opacity())
        ));
    }
    out
}

fn points(list: &[(f64, f64)], height: f64) -> String {
    let parts: Vec<String> = list
        .iter()
        .map(|(x, y)| format!("{},{}", num(*x), num(height - *y)))
        .collect();
    parts.join(" ")
}

fn item(out: &mut String, it: &Item, height: f64) {
    let stroke = if it.width > 0.0 {
        format!(
            "{} stroke-width=\"{}\"",
            paint(it.colour, "stroke"),
            num(it.width)
        )
    } else {
        " stroke=\"none\"".to_owned()
    };
    match &it.shape {
        Shape::Line { a, b } => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{}/>\n",
                num(a.0),
                num(height - a.1),
                num(b.0),
                num(height - b.1),
                stroke
            ));
        }
        Shape::Poly {
            points: list,
            closed,
        } => {
            let fill = if it.fill && *closed {
                paint(it.colour, "fill")
            } else {
                " fill=\"none\"".to_owned()
            };
            let tag = if *closed { "polygon" } else { "polyline" };
            out.push_str(&format!(
                "<{tag} points=\"{}\"{fill}{stroke}/>\n",
                points(list, height)
            ));
        }
        Shape::Circle { centre, radius } => {
            let fill = if it.fill {
                paint(it.colour, "fill")
            } else {
                " fill=\"none\"".to_owned()
            };
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{fill}{stroke}/>\n",
                num(centre.0),
                num(height - centre.1),
                num(*radius)
            ));
        }
        Shape::Text {
            at,
            size,
            anchor,
            mono,
            text,
        } => {
            let anchor = match anchor {
                Anchor::Start => "start",
                Anchor::Middle => "middle",
                Anchor::End => "end",
            };
            let family = if *mono {
                "Courier New, Courier, monospace"
            } else {
                "Helvetica, Arial, sans-serif"
            };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-family=\"{family}\" text-anchor=\"{anchor}\"{}>{}</text>\n",
                num(at.0),
                num(height - at.1),
                num(*size),
                paint(it.colour, "fill"),
                escape(text)
            ));
        }
    }
}

/// The SVG of a page.
pub fn to_svg(drawing: &Drawing) -> String {
    let (w, h) = (drawing.width, drawing.height);
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\">\n",
        num(w),
        num(h),
        num(w),
        num(h)
    ));
    out.push_str(&format!("<title>{}</title>\n", escape(&drawing.title)));
    out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"{}/>\n",
        num(w),
        num(h),
        paint(drawing.background, "fill")
    ));
    for (index, layer) in drawing.layers.iter().enumerate() {
        out.push_str(&format!(
            "<g id=\"layer-{index}-{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n",
            layer.kind.key()
        ));
        for it in &layer.items {
            item(&mut out, it, h);
        }
        out.push_str("</g>\n");
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Layer, LayerKind};

    fn page() -> Drawing {
        let black = Rgba::new(0, 0, 0, 255);
        let glass = Rgba::new(255, 0, 0, 128);
        Drawing {
            width: 100.0,
            height: 50.0,
            title: "A <b> & \"c\"".to_owned(),
            scale: 1.0,
            background: Rgba::new(255, 255, 255, 255),
            layers: vec![
                Layer {
                    kind: LayerKind::Outline,
                    items: vec![
                        Item::line((0.0, 0.0), (10.0, 5.0), black, 0.25),
                        Item::rect((1.0, 1.0), (4.0, 3.0), glass, 0.0, true),
                        Item::polyline(vec![(0.0, 0.0), (1.0, 1.0)], black, 0.1),
                        Item::circle((5.0, 5.0), 1.5, black, 0.0, true),
                    ],
                },
                Layer {
                    kind: LayerKind::Label,
                    items: vec![Item::text(
                        (2.0, 2.0),
                        3.0,
                        Anchor::Middle,
                        true,
                        "R1 <10k> & \u{1}x",
                        black,
                    )],
                },
            ],
        }
    }

    #[test]
    fn the_same_drawing_gives_the_same_bytes_and_no_date() {
        let a = to_svg(&page());
        assert_eq!(a, to_svg(&page()));
        assert!(!a.to_lowercase().contains("date"), "{a}");
    }

    #[test]
    fn the_page_has_the_size_the_title_and_the_layers() {
        let svg = to_svg(&page());
        assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg "));
        assert!(svg.contains("width=\"100mm\" height=\"50mm\" viewBox=\"0 0 100 50\""));
        assert!(svg.contains("<title>A &lt;b&gt; &amp; &quot;c&quot;</title>"));
        assert!(svg.contains("<g id=\"layer-0-outline\""));
        assert!(svg.contains("<g id=\"layer-1-label\""));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn the_vertical_axis_is_turned_and_transparency_is_kept() {
        let svg = to_svg(&page());
        assert!(svg.contains("<line x1=\"0\" y1=\"50\" x2=\"10\" y2=\"45\" stroke=\"#000000\" stroke-width=\"0.25\"/>"));
        assert!(svg.contains("<polygon points=\"1,49 4,49 4,47 1,47\" fill=\"#ff0000\" fill-opacity=\"0.502\" stroke=\"none\"/>"));
        assert!(svg.contains("<polyline points=\"0,50 1,49\" fill=\"none\""));
        assert!(
            svg.contains("<circle cx=\"5\" cy=\"45\" r=\"1.5\" fill=\"#000000\" stroke=\"none\"/>")
        );
    }

    #[test]
    fn the_text_is_escaped_and_anchored() {
        let svg = to_svg(&page());
        assert!(svg.contains("text-anchor=\"middle\""));
        assert!(svg.contains("font-family=\"Courier New, Courier, monospace\""));
        assert!(svg.contains(">R1 &lt;10k&gt; &amp;  x</text>"));
        assert_eq!(escape("a\tb\u{7f}"), "a b ");
        assert_eq!(escape("it's"), "it&apos;s");
    }
}
