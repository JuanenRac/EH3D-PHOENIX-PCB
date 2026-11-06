// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing that the writers turn into SVG and PDF: shapes in millimetres on layers of a
//! kind, with a colour that may be transparent. The vertical axis points up, as on paper.

/// A point in millimetres.
pub type Pt = (f64, f64);

/// A colour with transparency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Opacity, 255 for a colour that is not transparent.
    pub a: u8,
}

impl Rgba {
    /// A colour.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }

    /// Reads `#rrggbb` or `#rrggbbaa`.
    pub fn parse(text: &str) -> Option<Rgba> {
        let hex = text.trim().strip_prefix('#')?;
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) || (hex.len() != 6 && hex.len() != 8) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        let a = if hex.len() == 8 { byte(6)? } else { 255 };
        Some(Rgba::new(byte(0)?, byte(2)?, byte(4)?, a))
    }

    /// The colour as `#rrggbb`, without the transparency.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// The opacity from 0 to 1.
    pub fn opacity(self) -> f64 {
        f64::from(self.a) / 255.0
    }
}

/// What a layer of a drawing is, which decides its name in the settings and its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// The border and the title block.
    Frame,
    /// The dimension lines.
    Dimension,
    /// Documentation layers of a board.
    Documentation,
    /// The top copper layer.
    TopCopper,
    /// The bottom copper layer.
    BottomCopper,
    /// An inner copper layer.
    InnerCopper,
    /// Solder mask openings.
    Mask,
    /// Solder paste openings.
    Paste,
    /// Silkscreen of the top side.
    SilkTop,
    /// Silkscreen of the bottom side.
    SilkBottom,
    /// The reference designators.
    Designator,
    /// The board outline.
    Outline,
    /// The drill holes.
    Drill,
    /// The silhouettes of the footprints of an assembly plan.
    Assembly,
    /// The mark of the first pin of a footprint.
    Pin1,
    /// The fiducials of a board.
    Fiducial,
    /// The frame of a sheet of a schematic.
    Sheet,
    /// The wires of a schematic.
    Wire,
    /// The junctions of a schematic.
    Junction,
    /// The buses of a schematic.
    Bus,
    /// The bodies of the symbols of a schematic.
    Symbol,
    /// The pins of a schematic.
    Pin,
    /// The texts of a schematic.
    Label,
    /// The power ports and the ports of a sheet of a schematic.
    Port,
    /// The rails of the frame of a panel.
    PanelFrame,
    /// The copies of the boards of a panel.
    PanelBoards,
    /// The routed outline of a panel.
    PanelOutline,
    /// The tabs of a panel.
    PanelTabs,
    /// The V-score lines of a panel.
    PanelVScore,
    /// The mouse bite holes of a panel.
    PanelMouseBites,
    /// The fiducials of a panel.
    PanelFiducials,
    /// The tooling holes of a panel.
    PanelTooling,
    /// The holes of the boards of a panel.
    PanelHoles,
}

impl LayerKind {
    /// Every kind, in the order of the settings file.
    pub const ALL: [LayerKind; 33] = [
        LayerKind::Frame,
        LayerKind::Dimension,
        LayerKind::Documentation,
        LayerKind::TopCopper,
        LayerKind::BottomCopper,
        LayerKind::InnerCopper,
        LayerKind::Mask,
        LayerKind::Paste,
        LayerKind::SilkTop,
        LayerKind::SilkBottom,
        LayerKind::Designator,
        LayerKind::Outline,
        LayerKind::Drill,
        LayerKind::Assembly,
        LayerKind::Pin1,
        LayerKind::Fiducial,
        LayerKind::Sheet,
        LayerKind::Wire,
        LayerKind::Junction,
        LayerKind::Bus,
        LayerKind::Symbol,
        LayerKind::Pin,
        LayerKind::Label,
        LayerKind::Port,
        LayerKind::PanelFrame,
        LayerKind::PanelBoards,
        LayerKind::PanelOutline,
        LayerKind::PanelTabs,
        LayerKind::PanelVScore,
        LayerKind::PanelMouseBites,
        LayerKind::PanelFiducials,
        LayerKind::PanelTooling,
        LayerKind::PanelHoles,
    ];

    /// The name in the settings and in `--layers`.
    pub fn key(self) -> &'static str {
        match self {
            LayerKind::Frame => "frame",
            LayerKind::Dimension => "dimensions",
            LayerKind::Documentation => "documentation",
            LayerKind::TopCopper => "top_copper",
            LayerKind::BottomCopper => "bottom_copper",
            LayerKind::InnerCopper => "inner_copper",
            LayerKind::Mask => "mask",
            LayerKind::Paste => "paste",
            LayerKind::SilkTop => "silk_top",
            LayerKind::SilkBottom => "silk_bottom",
            LayerKind::Designator => "designator",
            LayerKind::Outline => "outline",
            LayerKind::Drill => "drill",
            LayerKind::Assembly => "assembly",
            LayerKind::Pin1 => "pin1",
            LayerKind::Fiducial => "fiducial",
            LayerKind::Sheet => "sheet",
            LayerKind::Wire => "wire",
            LayerKind::Junction => "junction",
            LayerKind::Bus => "bus",
            LayerKind::Symbol => "symbol",
            LayerKind::Pin => "pin",
            LayerKind::Label => "label",
            LayerKind::Port => "port",
            LayerKind::PanelFrame => "panel_frame",
            LayerKind::PanelBoards => "panel_boards",
            LayerKind::PanelOutline => "panel_outline",
            LayerKind::PanelTabs => "panel_tabs",
            LayerKind::PanelVScore => "panel_vscore",
            LayerKind::PanelMouseBites => "panel_mouse_bites",
            LayerKind::PanelFiducials => "panel_fiducials",
            LayerKind::PanelTooling => "panel_tooling",
            LayerKind::PanelHoles => "panel_holes",
        }
    }

    /// The kind with a name, or `None`.
    pub fn from_key(key: &str) -> Option<LayerKind> {
        LayerKind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// The position of the kind in [`LayerKind::ALL`].
    pub fn index(self) -> usize {
        LayerKind::ALL
            .iter()
            .position(|k| *k == self)
            .unwrap_or_default()
    }

    /// The colour that the settings give when they say nothing: dark on white, as it is printed.
    pub fn default_colour(self) -> Rgba {
        let c = |r: u8, g: u8, b: u8, a: u8| Rgba::new(r, g, b, a);
        match self {
            LayerKind::Frame | LayerKind::Dimension => c(0x20, 0x20, 0x20, 0xff),
            LayerKind::Documentation => c(0x80, 0x80, 0x80, 0xff),
            LayerKind::TopCopper => c(0xc0, 0x30, 0x30, 0xb0),
            LayerKind::BottomCopper => c(0x30, 0x50, 0xc0, 0xb0),
            LayerKind::InnerCopper => c(0x30, 0x90, 0x50, 0x80),
            LayerKind::Mask => c(0x20, 0x80, 0x40, 0x50),
            LayerKind::Paste => c(0x90, 0x90, 0x90, 0x80),
            LayerKind::SilkTop => c(0x10, 0x10, 0x10, 0xff),
            LayerKind::SilkBottom => c(0x60, 0x40, 0x10, 0xff),
            LayerKind::Designator => c(0x10, 0x10, 0x10, 0xff),
            LayerKind::Outline => c(0x00, 0x00, 0x00, 0xff),
            LayerKind::Drill => c(0x00, 0x00, 0x00, 0xff),
            LayerKind::Assembly => c(0x30, 0x30, 0x30, 0xff),
            LayerKind::Pin1 => c(0xc0, 0x00, 0x00, 0xff),
            LayerKind::Fiducial => c(0x00, 0x60, 0xa0, 0xff),
            LayerKind::Sheet => c(0x30, 0x30, 0x30, 0xff),
            LayerKind::Wire => c(0x00, 0x60, 0x00, 0xff),
            LayerKind::Junction => c(0x00, 0x60, 0x00, 0xff),
            LayerKind::Bus => c(0x00, 0x00, 0x90, 0xff),
            LayerKind::Symbol => c(0x80, 0x00, 0x00, 0xff),
            LayerKind::Pin => c(0x80, 0x00, 0x00, 0xff),
            LayerKind::Label => c(0x10, 0x10, 0x10, 0xff),
            LayerKind::Port => c(0x60, 0x00, 0x60, 0xff),
            LayerKind::PanelFrame => c(0x4a, 0x55, 0x68, 0xff),
            LayerKind::PanelBoards => c(0x2f, 0x8f, 0x5b, 0x60),
            LayerKind::PanelOutline => c(0x20, 0x20, 0x20, 0xff),
            LayerKind::PanelTabs => c(0xc0, 0x90, 0x10, 0xff),
            LayerKind::PanelVScore => c(0xc0, 0x20, 0x20, 0xff),
            LayerKind::PanelMouseBites => c(0xd0, 0x70, 0x10, 0xff),
            LayerKind::PanelFiducials => c(0x00, 0x80, 0xb0, 0xff),
            LayerKind::PanelTooling => c(0x70, 0x40, 0xb0, 0xff),
            LayerKind::PanelHoles => c(0x10, 0x18, 0x20, 0xff),
        }
    }
}

/// Where the text is held by its point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// The text starts at the point.
    Start,
    /// The text is centred on the point.
    Middle,
    /// The text ends at the point.
    End,
}

/// The geometry of an item.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A straight line.
    Line {
        /// Start.
        a: Pt,
        /// End.
        b: Pt,
    },
    /// A chain of lines, closed into a polygon or open.
    Poly {
        /// The corners.
        points: Vec<Pt>,
        /// True when the last corner joins the first.
        closed: bool,
    },
    /// A circle.
    Circle {
        /// The centre.
        centre: Pt,
        /// The radius.
        radius: f64,
    },
    /// A text on one line.
    Text {
        /// The point on the baseline that the anchor refers to.
        at: Pt,
        /// The size of the font in millimetres.
        size: f64,
        /// How the point holds the text.
        anchor: Anchor,
        /// True for the font of fixed width.
        mono: bool,
        /// The characters.
        text: String,
    },
}

/// One thing to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The geometry.
    pub shape: Shape,
    /// The colour of the stroke and of the fill.
    pub colour: Rgba,
    /// The width of the stroke in millimetres; zero for a shape that is only filled.
    pub width: f64,
    /// True when the inside of a polygon or a circle is painted.
    pub fill: bool,
}

impl Item {
    /// A line.
    pub fn line(a: Pt, b: Pt, colour: Rgba, width: f64) -> Item {
        Item {
            shape: Shape::Line { a, b },
            colour,
            width,
            fill: false,
        }
    }

    /// A polygon, painted when `fill` is true.
    pub fn polygon(points: Vec<Pt>, colour: Rgba, width: f64, fill: bool) -> Item {
        Item {
            shape: Shape::Poly {
                points,
                closed: true,
            },
            colour,
            width,
            fill,
        }
    }

    /// A chain of lines that is not closed.
    pub fn polyline(points: Vec<Pt>, colour: Rgba, width: f64) -> Item {
        Item {
            shape: Shape::Poly {
                points,
                closed: false,
            },
            colour,
            width,
            fill: false,
        }
    }

    /// A rectangle from its lower left and upper right corners.
    pub fn rect(min: Pt, max: Pt, colour: Rgba, width: f64, fill: bool) -> Item {
        Item::polygon(
            vec![min, (max.0, min.1), max, (min.0, max.1)],
            colour,
            width,
            fill,
        )
    }

    /// A circle, painted when `fill` is true.
    pub fn circle(centre: Pt, radius: f64, colour: Rgba, width: f64, fill: bool) -> Item {
        Item {
            shape: Shape::Circle { centre, radius },
            colour,
            width,
            fill,
        }
    }

    /// A text.
    pub fn text(at: Pt, size: f64, anchor: Anchor, mono: bool, text: &str, colour: Rgba) -> Item {
        Item {
            shape: Shape::Text {
                at,
                size,
                anchor,
                mono,
                text: text.to_owned(),
            },
            colour,
            width: 0.0,
            fill: true,
        }
    }
}

/// The items of one kind, drawn in the order of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// What the layer is.
    pub kind: LayerKind,
    /// The items.
    pub items: Vec<Item>,
}

/// A page ready to be written: its size, its scale and its layers, from the lowest.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// The width of the page in millimetres.
    pub width: f64,
    /// The height of the page in millimetres.
    pub height: f64,
    /// The title of the page.
    pub title: String,
    /// The scale: millimetres on paper for each millimetre of the design.
    pub scale: f64,
    /// The colour of the paper.
    pub background: Rgba,
    /// The layers.
    pub layers: Vec<Layer>,
}

/// A number with at most three decimals and no trailing zeros, the same on every machine.
pub fn num(value: f64) -> String {
    let v = if value.is_finite() { value } else { 0.0 };
    let text = format!("{v:.3}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-0" || trimmed == "-" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_have_the_fewest_exact_decimals() {
        assert_eq!(num(0.0), "0");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(10.0), "10");
        assert_eq!(num(100.0), "100");
        assert_eq!(num(10.5), "10.5");
        assert_eq!(num(0.125), "0.125");
        assert_eq!(num(2.0 / 3.0), "0.667");
        assert_eq!(num(f64::NAN), "0");
        assert_eq!(num(-1.25), "-1.25");
    }

    #[test]
    fn a_colour_is_read_with_or_without_transparency() {
        assert_eq!(Rgba::parse("#ff8000"), Some(Rgba::new(255, 128, 0, 255)));
        assert_eq!(Rgba::parse(" #00000080 "), Some(Rgba::new(0, 0, 0, 128)));
        for bad in ["", "ff8000", "#ff80", "#gg0000", "#ff80000", "#ééé"] {
            assert_eq!(Rgba::parse(bad), None, "{bad}");
        }
        assert_eq!(Rgba::new(1, 2, 255, 7).hex(), "#0102ff");
        assert!((Rgba::new(0, 0, 0, 255).opacity() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn every_kind_has_its_own_key_that_reads_back() {
        for (i, kind) in LayerKind::ALL.iter().enumerate() {
            assert_eq!(LayerKind::from_key(kind.key()), Some(*kind));
            assert_eq!(kind.index(), i);
        }
        assert_eq!(LayerKind::from_key("nothing"), None);
        let mut keys: Vec<&str> = LayerKind::ALL.iter().map(|k| k.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), LayerKind::ALL.len());
    }
}
