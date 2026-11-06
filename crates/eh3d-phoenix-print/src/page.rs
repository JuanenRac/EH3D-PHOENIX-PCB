// SPDX-License-Identifier: GPL-3.0-or-later
//! The page: the design (a sheet of a schematic, a side of a board, a panel) put on a sheet of
//! paper at a scale, with a border, a title block with the name of the project, the sheet, the
//! revision, the author, the scale and the paper in the language chosen, a scale bar and, when
//! asked, the dimension lines of the width and the height.

use eh3d_phoenix_rules::Lang;

use crate::codes::{PrintError, DOES_NOT_FIT, NOTHING_TO_PRINT, PAPER_INVALID};
use crate::config::{FontFace, Orientation, PrintConfig, Rounding};
use crate::draw::{num, Anchor, Drawing, Item, Layer, LayerKind, Pt, Shape};
use crate::font::{fit_text, text_width};
use crate::text::PrintText;

/// The largest scale that the automatic choice picks.
const MAX_AUTO_SCALE: f64 = 50.0;

/// The usual scales inside a power of ten.
const NICE: [f64; 11] = [1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 7.5, 8.0];

/// The room between the border and the drawing, in millimetres.
const PAD_MM: f64 = 4.0;

/// The room that the dimension lines need on the right, at the bottom and at the top.
const DIMENSION_ROOM_MM: f64 = 14.0;

/// What is drawn on one page, in the units of the design (millimetres, the vertical axis up).
#[derive(Debug, Clone, PartialEq)]
pub struct Content {
    /// The name of the file, without the extension.
    pub name: String,
    /// The name of the document, for the title block when the settings give no project name.
    pub project: String,
    /// The title of the page.
    pub title: String,
    /// The number of this page among the pages of the document, from 1.
    pub sheet_no: usize,
    /// How many pages the document has.
    pub sheet_count: usize,
    /// The layers, from the lowest.
    pub layers: Vec<Layer>,
    /// The box that the dimension lines measure (the outline of a board), or `None` for the box
    /// that holds everything.
    pub dimension_box: Option<(Pt, Pt)>,
}

fn grow(bounds: &mut Option<(Pt, Pt)>, p: Pt) {
    match bounds {
        Some((min, max)) => {
            min.0 = min.0.min(p.0);
            min.1 = min.1.min(p.1);
            max.0 = max.0.max(p.0);
            max.1 = max.1.max(p.1);
        }
        None => *bounds = Some((p, p)),
    }
}

/// The box that holds every item of the layers, or `None` when they hold nothing.
pub fn layers_bounds<'a>(layers: impl Iterator<Item = &'a Layer>) -> Option<(Pt, Pt)> {
    let mut bounds: Option<(Pt, Pt)> = None;
    for item in layers.flat_map(|l| l.items.iter()) {
        match &item.shape {
            Shape::Line { a, b } => {
                grow(&mut bounds, *a);
                grow(&mut bounds, *b);
            }
            Shape::Poly { points, .. } => {
                for p in points {
                    grow(&mut bounds, *p);
                }
            }
            Shape::Circle { centre, radius } => {
                grow(&mut bounds, (centre.0 - radius, centre.1 - radius));
                grow(&mut bounds, (centre.0 + radius, centre.1 + radius));
            }
            Shape::Text {
                at,
                size,
                anchor,
                mono,
                text,
            } => {
                let width = text_width(text, *size, *mono);
                let left = match anchor {
                    Anchor::Start => at.0,
                    Anchor::Middle => at.0 - width / 2.0,
                    Anchor::End => at.0 - width,
                };
                grow(&mut bounds, (left, at.1));
                grow(&mut bounds, (left + width, at.1 + size));
            }
        }
    }
    bounds
}

/// The largest usual scale that does not exceed `fit`.
pub fn nice_scale(fit: f64) -> f64 {
    if !(fit.is_finite() && fit > 0.0) {
        return fit;
    }
    let decade = 10f64.powf(fit.log10().floor());
    for d in [decade, decade / 10.0] {
        for m in NICE.iter().rev() {
            let value = m * d;
            if value <= fit * (1.0 + 1e-9) {
                return value;
            }
        }
    }
    fit
}

/// A scale as it is written: `2:1` for double size, `1:2` for half size.
pub fn scale_text(scale: f64) -> String {
    if scale >= 1.0 {
        format!("{}:1", num(scale))
    } else {
        format!("1:{}", num(1.0 / scale))
    }
}

/// The longest length of 1, 2 or 5 times a power of ten that does not exceed `max`.
fn bar_length(max: f64) -> f64 {
    if !(max.is_finite() && max > 0.0) {
        return 0.0;
    }
    let decade = 10f64.powf(max.log10().floor());
    for d in [decade, decade / 10.0] {
        for m in [5.0, 2.0, 1.0] {
            let value = m * d;
            if value <= max * (1.0 + 1e-9) {
                return value;
            }
        }
    }
    0.0
}

fn mono(face: FontFace) -> bool {
    face == FontFace::Courier
}

/// The item mapped onto the paper.
fn place(item: &Item, scale: f64, offset: Pt, cfg: &PrintConfig) -> Item {
    let at = |p: &Pt| (p.0 * scale + offset.0, p.1 * scale + offset.1);
    let stroke = |width: f64, fill: bool| {
        if width > 0.0 {
            (width * scale).max(cfg.thin_line_mm)
        } else if fill {
            0.0
        } else {
            cfg.thin_line_mm
        }
    };
    let (shape, width) = match &item.shape {
        Shape::Line { a, b } => (
            Shape::Line { a: at(a), b: at(b) },
            stroke(item.width, false),
        ),
        Shape::Poly { points, closed } => (
            Shape::Poly {
                points: points.iter().map(at).collect(),
                closed: *closed,
            },
            stroke(item.width, item.fill && *closed),
        ),
        Shape::Circle { centre, radius } => (
            Shape::Circle {
                centre: at(centre),
                radius: radius * scale,
            },
            stroke(item.width, item.fill),
        ),
        Shape::Text {
            at: p,
            size,
            anchor,
            mono,
            text,
        } => (
            Shape::Text {
                at: at(p),
                size: (size * scale).max(cfg.min_text_mm),
                anchor: *anchor,
                mono: *mono,
                text: text.clone(),
            },
            0.0,
        ),
    };
    Item {
        shape,
        colour: item.colour,
        width,
        fill: item.fill,
    }
}

fn arrow(items: &mut Vec<Item>, tip: Pt, toward: Pt, cfg: &PrintConfig) {
    let colour = cfg.colour(LayerKind::Dimension);
    let (dx, dy) = (toward.0 - tip.0, toward.1 - tip.1);
    let length = dx.hypot(dy);
    if length <= 0.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let (back, side) = (1.8, 0.6);
    for sign in [1.0, -1.0] {
        let end = (
            tip.0 + ux * back - uy * side * sign,
            tip.1 + uy * back + ux * side * sign,
        );
        items.push(Item::line(tip, end, colour, cfg.title_line_mm));
    }
}

/// The dimension lines of the width, below the box, and of the height, to its right.
fn dimension_items(low: Pt, high: Pt, real: (f64, f64), cfg: &PrintConfig) -> Vec<Item> {
    let colour = cfg.colour(LayerKind::Dimension);
    let line = cfg.title_line_mm;
    let size = cfg.label_text_mm * 1.2;
    let m = mono(cfg.font_labels);
    let mut items = Vec::new();
    // the width
    let y = low.1 - 6.0;
    for x in [low.0, high.0] {
        items.push(Item::line((x, low.1 - 1.0), (x, y - 1.5), colour, line));
    }
    items.push(Item::line((low.0, y), (high.0, y), colour, line));
    arrow(&mut items, (low.0, y), (high.0, y), cfg);
    arrow(&mut items, (high.0, y), (low.0, y), cfg);
    items.push(Item::text(
        ((low.0 + high.0) / 2.0, y + 1.0),
        size,
        Anchor::Middle,
        m,
        &format!("{} mm", num(real.0)),
        colour,
    ));
    // the height
    let x = high.0 + 6.0;
    for y in [low.1, high.1] {
        items.push(Item::line((high.0 + 1.0, y), (x + 1.5, y), colour, line));
    }
    items.push(Item::line((x, low.1), (x, high.1), colour, line));
    arrow(&mut items, (x, low.1), (x, high.1), cfg);
    arrow(&mut items, (x, high.1), (x, low.1), cfg);
    items.push(Item::text(
        (x - 1.0, high.1 + 2.5),
        size,
        Anchor::End,
        m,
        &format!("{} mm", num(real.1)),
        colour,
    ));
    items
}

/// The values of the title block.
struct Block<'a> {
    project: &'a str,
    title: &'a str,
    sheet: String,
    scale: String,
    paper: String,
}

/// The border, the title block and the scale bar.
fn frame_items(
    page: (f64, f64),
    scale: f64,
    block: &Block,
    cfg: &PrintConfig,
    lang: Lang,
) -> Vec<Item> {
    let colour = cfg.colour(LayerKind::Frame);
    let (x0, y0) = (cfg.margin_mm, cfg.margin_mm);
    let (x1, y1) = (page.0 - cfg.margin_mm, page.1 - cfg.margin_mm);
    let mut items = vec![Item::rect(
        (x0, y0),
        (x1, y1),
        colour,
        cfg.frame_line_mm,
        false,
    )];
    let (tw, th) = (cfg.title_width_mm, cfg.title_height_mm);
    let bx = x1 - tw;
    items.push(Item::rect(
        (bx, y0),
        (x1, y0 + th),
        colour,
        cfg.frame_line_mm,
        false,
    ));
    let row = th / 3.0;
    for k in 1..3 {
        let y = y0 + row * f64::from(k);
        items.push(Item::line((bx, y), (x1, y), colour, cfg.title_line_mm));
    }
    let cell = tw / 5.0;
    for k in 1..5 {
        let x = bx + cell * f64::from(k);
        items.push(Item::line(
            (x, y0),
            (x, y0 + row),
            colour,
            cfg.title_line_mm,
        ));
    }
    // a cell: its name small at the top and its value below
    let (lm, vm) = (mono(cfg.font_labels), mono(cfg.font_title));
    let mut put = |left: f64, bottom: f64, width: f64, name: PrintText, value: &str| {
        items.push(Item::text(
            (left + 1.0, bottom + row - cfg.label_text_mm - 0.4),
            cfg.label_text_mm,
            Anchor::Start,
            lm,
            name.tr(lang),
            colour,
        ));
        let shown = fit_text(value, cfg.title_text_mm, vm, width - 2.0);
        items.push(Item::text(
            (left + 1.5, bottom + 1.6),
            cfg.title_text_mm,
            Anchor::Start,
            vm,
            &shown,
            colour,
        ));
    };
    put(bx, y0 + 2.0 * row, tw, PrintText::Project, block.project);
    put(bx, y0 + row, tw, PrintText::Title, block.title);
    put(bx, y0, cell, PrintText::Sheet, &block.sheet);
    put(bx + cell, y0, cell, PrintText::Revision, &cfg.revision);
    put(bx + 2.0 * cell, y0, cell, PrintText::Author, &cfg.author);
    put(bx + 3.0 * cell, y0, cell, PrintText::Scale, &block.scale);
    put(bx + 4.0 * cell, y0, cell, PrintText::Paper, &block.paper);

    // the scale bar, in the strip to the left of the title block
    let strip = (bx - x0) - 2.0 * PAD_MM;
    if cfg.scale_bar && strip >= 25.0 {
        let longest = (strip - 10.0).min(80.0);
        let real = bar_length(longest / scale);
        if real > 0.0 {
            let (xs, y) = (x0 + PAD_MM + 3.0, y0 + th * 0.45);
            let xe = xs + real * scale;
            items.push(Item::line(
                (xs, y),
                (xe, y),
                colour,
                cfg.title_line_mm * 2.0,
            ));
            for (x, h) in [(xs, 1.5), (xe, 1.5), ((xs + xe) / 2.0, 0.9)] {
                items.push(Item::line(
                    (x, y - h),
                    (x, y + h),
                    colour,
                    cfg.title_line_mm,
                ));
            }
            items.push(Item::text(
                (xs, y - 2.0 - cfg.label_text_mm),
                cfg.label_text_mm * 1.2,
                Anchor::Start,
                lm,
                &format!("{} mm", num(real)),
                colour,
            ));
            items.push(Item::text(
                (xs, y + 3.0),
                cfg.label_text_mm * 1.2,
                Anchor::Start,
                lm,
                &format!("{} {}", PrintText::Scale.tr(lang), scale_text(scale)),
                colour,
            ));
        }
    }
    items
}

/// Puts the content on a page of the paper of the settings.
pub fn compose(content: &Content, cfg: &PrintConfig, lang: Lang) -> Result<Drawing, PrintError> {
    cfg.validate()?;
    let chosen: Vec<&Layer> = content
        .layers
        .iter()
        .filter(|l| cfg.wants(l.kind) && !l.items.is_empty())
        .collect();
    let (min, max) = layers_bounds(chosen.iter().copied()).ok_or_else(|| {
        PrintError::new(
            NOTHING_TO_PRINT,
            format!("{}: no layer holds anything", content.name),
        )
    })?;
    let (cw, ch) = ((max.0 - min.0).max(0.01), (max.1 - min.1).max(0.01));

    let (short, long) = cfg.paper.sides();
    let landscape = match cfg.orientation {
        Orientation::Landscape => true,
        Orientation::Portrait => false,
        Orientation::Auto => cw >= ch,
    };
    let (pw, ph) = if landscape {
        (long, short)
    } else {
        (short, long)
    };

    let room = if cfg.dimensions {
        DIMENSION_ROOM_MM
    } else {
        0.0
    };
    let top = if cfg.dimensions { 6.0 } else { 0.0 };
    let (ax0, ax1) = (cfg.margin_mm + PAD_MM, pw - cfg.margin_mm - PAD_MM - room);
    let (ay0, ay1) = (
        cfg.margin_mm + cfg.title_height_mm + PAD_MM + room,
        ph - cfg.margin_mm - PAD_MM - top,
    );
    let (avail_w, avail_h) = (ax1 - ax0, ay1 - ay0);
    if avail_w < 10.0 || avail_h < 10.0 {
        return Err(PrintError::new(
            PAPER_INVALID,
            format!("{} leaves no room for the drawing", cfg.paper.name()),
        ));
    }

    let scale = if cfg.scale > 0.0 {
        if cw * cfg.scale > avail_w + 1e-9 || ch * cfg.scale > avail_h + 1e-9 {
            return Err(PrintError::new(
                DOES_NOT_FIT,
                format!(
                    "{}: the scale {} needs {} x {} mm and the paper leaves {} x {} mm",
                    content.name,
                    scale_text(cfg.scale),
                    num(cw * cfg.scale),
                    num(ch * cfg.scale),
                    num(avail_w),
                    num(avail_h)
                ),
            ));
        }
        cfg.scale
    } else {
        let fit = (avail_w / cw).min(avail_h / ch).min(MAX_AUTO_SCALE);
        match cfg.rounding {
            Rounding::Nice => nice_scale(fit),
            Rounding::Exact => fit,
        }
    };
    let offset = (
        ax0 + (avail_w - cw * scale) / 2.0 - min.0 * scale,
        ay0 + (avail_h - ch * scale) / 2.0 - min.1 * scale,
    );

    let mut layers: Vec<Layer> = chosen
        .iter()
        .map(|l| Layer {
            kind: l.kind,
            items: l
                .items
                .iter()
                .map(|i| place(i, scale, offset, cfg))
                .collect(),
        })
        .collect();

    if cfg.dimensions {
        let (dmin, dmax) = content.dimension_box.unwrap_or((min, max));
        let low = (dmin.0 * scale + offset.0, dmin.1 * scale + offset.1);
        let high = (dmax.0 * scale + offset.0, dmax.1 * scale + offset.1);
        layers.push(Layer {
            kind: LayerKind::Dimension,
            items: dimension_items(low, high, (dmax.0 - dmin.0, dmax.1 - dmin.1), cfg),
        });
    }

    let project = if cfg.project.is_empty() {
        content.project.as_str()
    } else {
        cfg.project.as_str()
    };
    let block = Block {
        project,
        title: &content.title,
        sheet: format!("{}/{}", content.sheet_no, content.sheet_count),
        scale: scale_text(scale),
        paper: cfg.paper.name(),
    };
    layers.push(Layer {
        kind: LayerKind::Frame,
        items: frame_items((pw, ph), scale, &block, cfg, lang),
    });

    Ok(Drawing {
        width: pw,
        height: ph,
        title: content.title.clone(),
        scale,
        background: cfg.background,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Rgba;
    use crate::paper::Paper;

    fn content() -> Content {
        let black = Rgba::new(0, 0, 0, 255);
        Content {
            name: "plan".to_owned(),
            project: "Board".to_owned(),
            title: "Assembly".to_owned(),
            sheet_no: 2,
            sheet_count: 3,
            layers: vec![
                Layer {
                    kind: LayerKind::Outline,
                    items: vec![Item::rect((0.0, 0.0), (100.0, 50.0), black, 0.1, false)],
                },
                Layer {
                    kind: LayerKind::Label,
                    items: vec![Item::text(
                        (10.0, 10.0),
                        2.0,
                        Anchor::Start,
                        false,
                        "R1",
                        black,
                    )],
                },
            ],
            dimension_box: None,
        }
    }

    fn texts(drawing: &Drawing) -> Vec<String> {
        drawing
            .layers
            .iter()
            .flat_map(|l| l.items.iter())
            .filter_map(|i| match &i.shape {
                Shape::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_wide_drawing_goes_on_a_landscape_page_at_a_usual_scale() {
        let d = compose(&content(), &PrintConfig::default(), Lang::En).unwrap();
        assert_eq!((d.width, d.height), (297.0, 210.0));
        assert_eq!(d.scale, 2.5);
        let outline = &d.layers[0];
        assert_eq!(outline.kind, LayerKind::Outline);
        let Shape::Poly { points, .. } = &outline.items[0].shape else {
            panic!("not a polygon");
        };
        assert!((points[0].0 - 23.5).abs() < 1e-9, "{points:?}");
        assert!((points[0].1 - 54.5).abs() < 1e-9, "{points:?}");
        // the line of 0.1 mm is 0.25 mm on paper
        assert!((outline.items[0].width - 0.25).abs() < 1e-9);
        assert_eq!(d.layers.last().unwrap().kind, LayerKind::Frame);
    }

    #[test]
    fn the_title_block_is_in_the_language_chosen() {
        let cfg = PrintConfig {
            revision: "B".to_owned(),
            author: "Ana".to_owned(),
            ..PrintConfig::default()
        };
        let en = texts(&compose(&content(), &cfg, Lang::En).unwrap());
        for want in [
            "Project", "Board", "Title", "Assembly", "Sheet", "2/3", "B", "Ana",
        ] {
            assert!(en.iter().any(|t| t == want), "{want} in {en:?}");
        }
        assert!(en.iter().any(|t| t == "2.5:1"), "{en:?}");
        assert!(en.iter().any(|t| t == "A4"));
        let pt = texts(&compose(&content(), &cfg, Lang::Pt).unwrap());
        for want in ["Projeto", "Título", "Folha", "Revisão", "Escala", "Papel"] {
            assert!(pt.iter().any(|t| t == want), "{want} in {pt:?}");
        }
        let named = PrintConfig {
            project: "My project".to_owned(),
            ..PrintConfig::default()
        };
        assert!(texts(&compose(&content(), &named, Lang::En).unwrap())
            .iter()
            .any(|t| t == "My project"));
    }

    #[test]
    fn the_scale_can_be_given_and_must_fit() {
        let ok = PrintConfig {
            scale: 2.0,
            ..PrintConfig::default()
        };
        assert_eq!(compose(&content(), &ok, Lang::En).unwrap().scale, 2.0);
        let too_big = PrintConfig {
            scale: 3.0,
            ..PrintConfig::default()
        };
        assert_eq!(
            compose(&content(), &too_big, Lang::En).unwrap_err().code,
            DOES_NOT_FIT
        );
        let exact = PrintConfig {
            rounding: Rounding::Exact,
            ..PrintConfig::default()
        };
        let d = compose(&content(), &exact, Lang::En).unwrap();
        assert!((d.scale - 2.69).abs() < 1e-9, "{}", d.scale);
    }

    #[test]
    fn the_orientation_and_the_paper_follow_the_settings() {
        let portrait = PrintConfig {
            orientation: Orientation::Portrait,
            paper: Paper::A3,
            ..PrintConfig::default()
        };
        let d = compose(&content(), &portrait, Lang::En).unwrap();
        assert_eq!((d.width, d.height), (297.0, 420.0));
        let mut tall = content();
        tall.layers[0].items = vec![Item::rect(
            (0.0, 0.0),
            (30.0, 90.0),
            Rgba::new(0, 0, 0, 255),
            0.1,
            false,
        )];
        let d = compose(&tall, &PrintConfig::default(), Lang::En).unwrap();
        assert_eq!((d.width, d.height), (210.0, 297.0));
    }

    #[test]
    fn layers_are_chosen_and_an_empty_choice_is_refused() {
        let only = PrintConfig {
            layers: vec![LayerKind::Label],
            ..PrintConfig::default()
        };
        let d = compose(&content(), &only, Lang::En).unwrap();
        let kinds: Vec<LayerKind> = d.layers.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, vec![LayerKind::Label, LayerKind::Frame]);
        let none = PrintConfig {
            layers: vec![LayerKind::Drill],
            ..PrintConfig::default()
        };
        assert_eq!(
            compose(&content(), &none, Lang::En).unwrap_err().code,
            NOTHING_TO_PRINT
        );
    }

    #[test]
    fn dimensions_and_the_scale_bar_are_optional() {
        let with = PrintConfig {
            dimensions: true,
            ..PrintConfig::default()
        };
        let d = compose(&content(), &with, Lang::En).unwrap();
        let t = texts(&d);
        assert!(t.iter().any(|x| x == "100 mm"), "{t:?}");
        assert!(t.iter().any(|x| x == "50 mm"), "{t:?}");
        assert!(d.layers.iter().any(|l| l.kind == LayerKind::Dimension));
        let without = PrintConfig {
            scale_bar: false,
            ..PrintConfig::default()
        };
        let t = texts(&compose(&content(), &without, Lang::En).unwrap());
        assert!(!t.iter().any(|x| x.ends_with(" mm")), "{t:?}");
        let t = texts(&compose(&content(), &PrintConfig::default(), Lang::En).unwrap());
        assert!(t.iter().any(|x| x.ends_with(" mm")), "{t:?}");
    }

    #[test]
    fn the_usual_scales_and_the_bar_lengths_are_worked_out() {
        assert_eq!(nice_scale(2.69), 2.5);
        assert_eq!(nice_scale(1.0), 1.0);
        assert!((nice_scale(0.9) - 0.8).abs() < 1e-12);
        assert!((nice_scale(0.07) - 0.06).abs() < 1e-12);
        assert_eq!(scale_text(2.0), "2:1");
        assert_eq!(scale_text(0.5), "1:2");
        assert_eq!(scale_text(1.0), "1:1");
        assert_eq!(bar_length(37.0), 20.0);
        assert_eq!(bar_length(100.0), 100.0);
        assert_eq!(bar_length(4.9), 2.0);
        assert_eq!(bar_length(0.0), 0.0);
    }

    #[test]
    fn the_bounds_hold_the_text_too() {
        let c = content();
        let (min, max) = layers_bounds(c.layers.iter()).unwrap();
        assert_eq!(min, (0.0, 0.0));
        assert_eq!(max, (100.0, 50.0));
        assert!(layers_bounds(std::iter::empty()).is_none());
    }
}
