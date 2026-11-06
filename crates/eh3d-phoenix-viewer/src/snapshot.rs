// SPDX-License-Identifier: GPL-3.0-or-later
//! A picture of the scene of the viewer as SVG, made without a window: one group for each visible
//! layer in the order of drawing, with the colour the window gives the layer, on the background
//! colour of the window. It is the same data the window draws (`Scene`), so the drawing of a
//! board or a schematic can be looked at without a screen. The text is deterministic: the same
//! scene and settings give the same bytes.

use std::f64::consts::PI;
use std::fmt::Write as _;

use crate::prim::{arc_point, Bounds, Prim, Shape, P};
use crate::scene::Scene;
use crate::settings::ViewerSettings;

/// The margin around the drawing, as a fraction of its larger side.
const MARGIN: f64 = 0.04;
/// The width of a thin line (a width of zero), as a fraction of the larger side of the drawing.
const HAIRLINE: f64 = 0.0015;
/// The height of a capital letter as a fraction of the size of the font.
const CAP_HEIGHT: f64 = 0.7;

/// A number with at most four decimals and no trailing zeros; the same text on every system.
fn num(value: f64) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let mut text = format!("{value:.4}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" || text.is_empty() {
        text = "0".to_owned();
    }
    text
}

/// The text made safe for an attribute or for the content of an element.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// A colour as `#rrggbb`.
fn hex(rgba: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
}

/// The opacity of a colour, from its alpha.
fn opacity(rgba: [u8; 4]) -> String {
    num(f64::from(rgba[3]) / 255.0)
}

/// A point of the drawing as `x y` of the picture, whose y runs down.
fn pt(p: P) -> String {
    format!("{} {}", num(p.x), num(-p.y))
}

/// The path of an arc, cut in pieces of at most half a turn so that every piece is unambiguous.
fn arc_path(centre: P, radius: f64, start: f64, sweep: f64) -> String {
    let pieces = ((sweep.abs() / PI).ceil() as usize).max(1);
    let step = sweep / pieces as f64;
    let mut d = format!("M{}", pt(arc_point(centre, radius, start)));
    for k in 1..=pieces {
        let end = arc_point(centre, radius, start + step * k as f64);
        // the picture runs y down, so a counter-clockwise sweep of the drawing is the
        // positive-angle direction (flag 1) of the picture
        let flag = u8::from(step > 0.0);
        let _ = write!(d, "A{} {} 0 0 {flag} {}", num(radius), num(radius), pt(end));
    }
    d
}

/// The element of a primitive, with the stroke width and the fill that it asks for.
fn element(prim: &Prim, hairline: f64) -> String {
    let stroke = if prim.width > 0.0 {
        prim.width
    } else {
        hairline
    };
    match &prim.shape {
        Shape::Line { a, b } => format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"{}\"/>",
            num(a.x),
            num(-a.y),
            num(b.x),
            num(-b.y),
            num(stroke)
        ),
        Shape::Arc {
            centre,
            radius,
            start,
            sweep,
        } => format!(
            "<path d=\"{}\" fill=\"none\" stroke-width=\"{}\"/>",
            arc_path(*centre, *radius, *start, *sweep),
            num(stroke)
        ),
        Shape::Polygon { points } => {
            let mut d = String::new();
            for (k, p) in points.iter().enumerate() {
                let _ = write!(d, "{}{}", if k == 0 { "M" } else { "L" }, pt(*p));
            }
            d.push('Z');
            if prim.filled {
                format!("<path d=\"{d}\" stroke=\"none\"/>")
            } else {
                format!(
                    "<path d=\"{d}\" fill=\"none\" stroke-width=\"{}\"/>",
                    num(stroke)
                )
            }
        }
        Shape::Circle { centre, radius } => {
            if prim.filled {
                format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"none\"/>",
                    num(centre.x),
                    num(-centre.y),
                    num(*radius)
                )
            } else {
                format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke-width=\"{}\"/>",
                    num(centre.x),
                    num(-centre.y),
                    num(*radius),
                    num(stroke)
                )
            }
        }
        Shape::Text { at, height, text } => format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" stroke=\"none\">{}</text>",
            num(at.x),
            num(-at.y),
            num(height / CAP_HEIGHT),
            escape(text)
        ),
    }
}

/// The scene as an SVG document in millimetres: the background, then each visible layer in the
/// order of drawing, in the colour of the layer. An empty scene gives a small empty picture.
pub fn scene_svg(scene: &Scene, settings: &ViewerSettings) -> String {
    let mut area: Bounds = scene.bounds;
    if area.is_empty() {
        area = Bounds {
            min: P::new(0.0, 0.0),
            max: P::new(10.0, 10.0),
        };
    }
    let side = area.width().max(area.height()).max(1e-3);
    let margin = side * MARGIN;
    let hairline = side * HAIRLINE;
    let x = area.min.x - margin;
    let y = -area.max.y - margin;
    let w = area.width() + 2.0 * margin;
    let h = area.height() + 2.0 * margin;
    let background = settings.colour("background");
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}mm\" height=\"{}mm\">",
        num(x),
        num(y),
        num(w),
        num(h),
        num(w),
        num(h)
    );
    let _ = writeln!(
        out,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        num(x),
        num(y),
        num(w),
        num(h),
        hex(background)
    );
    for (index, layer) in scene.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let prims: Vec<&Prim> = scene
            .prims
            .iter()
            .filter(|p| p.layer as usize == index)
            .collect();
        if prims.is_empty() {
            continue;
        }
        let _ = writeln!(
            out,
            "<g id=\"layer-{index}\" data-name=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-opacity=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">",
            escape(&layer.name),
            hex(layer.colour),
            opacity(layer.colour),
            hex(layer.colour),
            opacity(layer.colour)
        );
        for prim in prims {
            out.push_str(&element(prim, hairline));
            out.push('\n');
        }
        out.push_str("</g>\n");
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use crate::scene::Scene;

    fn board_scene() -> (Scene, ViewerSettings) {
        let settings = ViewerSettings::default();
        let scene = Scene::from_board(&sample::board(), &settings).unwrap();
        (scene, settings)
    }

    #[test]
    fn numbers_have_no_trailing_zeros_and_no_negative_zero() {
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(2.0), "2");
        assert_eq!(num(-0.00001), "0");
        assert_eq!(num(0.12346), "0.1235");
        assert_eq!(num(f64::NAN), "0");
    }

    #[test]
    fn text_is_escaped_for_xml() {
        assert_eq!(escape("a<b & \"c\">"), "a&lt;b &amp; &quot;c&quot;&gt;");
        assert_eq!(escape("x\ny"), "x y");
    }

    #[test]
    fn the_snapshot_of_a_board_has_one_group_for_each_layer_with_something_to_draw() {
        let (scene, settings) = board_scene();
        let svg = scene_svg(&scene, &settings);
        assert!(svg.starts_with("<?xml"), "{svg}");
        assert!(svg.trim_end().ends_with("</svg>"));
        let drawn = scene
            .layers
            .iter()
            .enumerate()
            .filter(|(i, l)| l.visible && scene.prims.iter().any(|p| p.layer as usize == *i))
            .count();
        assert_eq!(svg.matches("<g id=\"layer-").count(), drawn);
        assert!(drawn >= 2, "{drawn}");
        // the background is the colour of the window
        assert!(svg.contains(&hex(settings.colour("background"))));
    }

    #[test]
    fn the_snapshot_draws_every_primitive_of_a_visible_layer_once() {
        let (scene, settings) = board_scene();
        let svg = scene_svg(&scene, &settings);
        let elements = svg.matches("<line ").count()
            + svg.matches("<path ").count()
            + svg.matches("<circle ").count()
            + svg.matches("<text ").count();
        let expected = scene
            .prims
            .iter()
            .filter(|p| scene.layers[p.layer as usize].visible)
            .count();
        assert_eq!(elements, expected);
    }

    #[test]
    fn the_same_scene_gives_the_same_bytes() {
        let (scene, settings) = board_scene();
        assert_eq!(scene_svg(&scene, &settings), scene_svg(&scene, &settings));
    }

    #[test]
    fn a_hidden_layer_is_left_out() {
        let (mut scene, settings) = board_scene();
        let before = scene_svg(&scene, &settings);
        let first = scene
            .layers
            .iter()
            .enumerate()
            .position(|(i, l)| l.visible && scene.prims.iter().any(|p| p.layer as usize == i))
            .unwrap();
        scene.layers[first].visible = false;
        let after = scene_svg(&scene, &settings);
        assert!(after.len() < before.len());
        assert!(!after.contains(&format!("<g id=\"layer-{first}\"")));
    }

    #[test]
    fn the_snapshot_of_a_schematic_has_the_names_of_its_layers() {
        let settings = ViewerSettings::default();
        let scene = Scene::from_schematic(&sample::schematic(), &settings).unwrap();
        let svg = scene_svg(&scene, &settings);
        assert!(svg.contains("data-name=\""), "{svg}");
        assert!(svg.contains("<line ") || svg.contains("<path "));
    }

    #[test]
    fn an_empty_scene_gives_a_small_empty_picture() {
        let (mut scene, settings) = board_scene();
        scene.prims.clear();
        scene.bounds = Bounds::EMPTY;
        let svg = scene_svg(&scene, &settings);
        assert!(svg.contains("viewBox=\""), "{svg}");
        assert_eq!(svg.matches("<g ").count(), 0);
    }

    #[test]
    fn an_arc_is_cut_in_pieces_of_at_most_half_a_turn() {
        let centre = P::new(0.0, 0.0);
        let half = arc_path(centre, 1.0, 0.0, PI / 2.0);
        assert_eq!(half.matches('A').count(), 1);
        let circle = arc_path(centre, 1.0, 0.0, 2.0 * PI);
        assert_eq!(circle.matches('A').count(), 2);
        // a counter-clockwise sweep of the drawing is flag 1 of the picture, a clockwise one 0
        assert!(half.contains(" 0 0 1 "), "{half}");
        assert!(arc_path(centre, 1.0, 0.0, -PI / 2.0).contains(" 0 0 0 "));
    }

    #[test]
    fn the_y_axis_of_the_drawing_is_turned_into_the_one_of_the_picture() {
        assert_eq!(pt(P::new(2.0, 3.0)), "2 -3");
    }
}
