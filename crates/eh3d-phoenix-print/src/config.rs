// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the printing. Every size, colour, font and text of the title block is read
//! from a plain `key = value` text, `config/print.cfg`, and not buried in the code. A missing line
//! keeps the default; an unknown key or a value out of range is refused with its line number. A
//! line that starts with `#` is a comment; a `#` inside a value is part of it (a colour).

use crate::codes::{PrintError, SETTINGS_INVALID};
use crate::draw::{LayerKind, Rgba};
use crate::paper::Paper;

/// The largest settings file that is read, in bytes.
pub const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

/// How the paper is turned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Landscape for a drawing that is wider than it is tall, else portrait.
    Auto,
    /// The long side is horizontal.
    Landscape,
    /// The long side is vertical.
    Portrait,
}

/// The font of the texts: one of the standard fonts of a PDF reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFace {
    /// Helvetica, of variable width.
    Helvetica,
    /// Courier, of fixed width.
    Courier,
}

/// Which side of a board is printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideChoice {
    /// The top side.
    Top,
    /// The bottom side.
    Bottom,
    /// Both sides.
    Both,
}

/// How the automatic scale is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// The largest of the usual scales (1, 1.25, 1.5, 2, 2.5, 3, 4, 5, 6, 7.5 and 8 times a power
    /// of ten) that fits.
    Nice,
    /// The exact scale that fills the paper.
    Exact,
}

/// The settings of the printing.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintConfig {
    /// The paper.
    pub paper: Paper,
    /// How the paper is turned.
    pub orientation: Orientation,
    /// The margin between the edge of the paper and the border, in millimetres.
    pub margin_mm: f64,
    /// The scale (paper over design); zero for the automatic one.
    pub scale: f64,
    /// How the automatic scale is chosen.
    pub rounding: Rounding,
    /// The width of the border line, in millimetres.
    pub frame_line_mm: f64,
    /// The width of the lines of the title block and of the dimensions, in millimetres.
    pub title_line_mm: f64,
    /// The width of the thinnest line that is drawn, in millimetres.
    pub thin_line_mm: f64,
    /// The width of the title block, in millimetres.
    pub title_width_mm: f64,
    /// The height of the title block, in millimetres.
    pub title_height_mm: f64,
    /// The size of the font of the values of the title block, in millimetres.
    pub title_text_mm: f64,
    /// The size of the font of the names of the cells of the title block, in millimetres.
    pub label_text_mm: f64,
    /// The smallest size of a font of the drawing, in millimetres.
    pub min_text_mm: f64,
    /// The font of the values of the title block.
    pub font_title: FontFace,
    /// The font of the names of the cells and of the dimensions.
    pub font_labels: FontFace,
    /// The name of the project in the title block; empty to use the name of the document.
    pub project: String,
    /// The revision in the title block.
    pub revision: String,
    /// The author in the title block.
    pub author: String,
    /// Draw the scale bar.
    pub scale_bar: bool,
    /// Draw the dimension lines of the width and the height.
    pub dimensions: bool,
    /// The side of a board that is printed.
    pub side: SideChoice,
    /// The layers that are printed; empty for all of them.
    pub layers: Vec<LayerKind>,
    /// The colour of the paper.
    pub background: Rgba,
    /// The colour of each kind of layer, in the order of [`LayerKind::ALL`].
    pub colours: [Rgba; 33],
}

impl Default for PrintConfig {
    fn default() -> Self {
        let mut colours = [Rgba::new(0, 0, 0, 255); 33];
        for kind in LayerKind::ALL {
            colours[kind.index()] = kind.default_colour();
        }
        PrintConfig {
            paper: Paper::A4,
            orientation: Orientation::Auto,
            margin_mm: 10.0,
            scale: 0.0,
            rounding: Rounding::Nice,
            frame_line_mm: 0.5,
            title_line_mm: 0.25,
            thin_line_mm: 0.1,
            title_width_mm: 150.0,
            title_height_mm: 24.0,
            title_text_mm: 3.2,
            label_text_mm: 2.0,
            min_text_mm: 1.0,
            font_title: FontFace::Helvetica,
            font_labels: FontFace::Helvetica,
            project: String::new(),
            revision: String::new(),
            author: String::new(),
            scale_bar: true,
            dimensions: false,
            side: SideChoice::Both,
            layers: Vec::new(),
            background: Rgba::new(255, 255, 255, 255),
            colours,
        }
    }
}

/// The file that is read when no other is named and it exists.
pub const DEFAULT_FILE: &str = "config/print.cfg";

/// Reads the settings from a file, refusing one larger than [`MAX_SETTINGS_BYTES`].
pub fn read_file(path: &std::path::Path) -> Result<PrintConfig, PrintError> {
    let bad =
        |e: std::io::Error| PrintError::new(SETTINGS_INVALID, format!("{}: {e}", path.display()));
    let size = std::fs::metadata(path).map_err(bad)?.len();
    if size > MAX_SETTINGS_BYTES {
        return Err(PrintError::new(
            SETTINGS_INVALID,
            format!(
                "{} is {size} bytes, the limit is {MAX_SETTINGS_BYTES}",
                path.display()
            ),
        ));
    }
    let text = std::fs::read_to_string(path).map_err(bad)?;
    let config = PrintConfig::parse(&text)?;
    config.validate()?;
    Ok(config)
}

/// Reads a scale: `0` or `auto` for the automatic one, `1:2` for half size, `2:1` for double size
/// or a plain number such as `0.5`. The scale must be from 0.001 to 1000.
pub fn parse_scale(text: &str) -> Option<f64> {
    let t = text.trim();
    if t == "0" || t.eq_ignore_ascii_case("auto") {
        return Some(0.0);
    }
    let value = match t.split_once(':') {
        Some((a, b)) => a.trim().parse::<f64>().ok()? / b.trim().parse::<f64>().ok()?,
        None => t.parse::<f64>().ok()?,
    };
    (value.is_finite() && (0.001..=1000.0).contains(&value)).then_some(value)
}

/// Reads a list of layer names separated by commas; `all` or nothing is every layer. The error is
/// the name that is not known.
pub fn parse_layers(text: &str) -> Result<Vec<LayerKind>, String> {
    let t = text.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("all") {
        return Ok(Vec::new());
    }
    let mut kinds = Vec::new();
    for name in t.split(',') {
        let name = name.trim();
        let kind = LayerKind::from_key(name).ok_or_else(|| name.to_owned())?;
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    Ok(kinds)
}

impl PrintConfig {
    /// The colour of a kind of layer.
    pub fn colour(&self, kind: LayerKind) -> Rgba {
        self.colours[kind.index()]
    }

    /// True when a kind of layer is printed: the border and the dimensions always are, the others
    /// when no list was given or the list holds them.
    pub fn wants(&self, kind: LayerKind) -> bool {
        matches!(kind, LayerKind::Frame | LayerKind::Dimension)
            || self.layers.is_empty()
            || self.layers.contains(&kind)
    }

    /// Reads the settings from the text of a file; a line that is missing keeps its default.
    pub fn parse(text: &str) -> Result<PrintConfig, PrintError> {
        let mut config = PrintConfig::default();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fail = |why: String| {
                PrintError::new(SETTINGS_INVALID, format!("line {}: {why}", index + 1))
            };
            let Some((key, value)) = line.split_once('=') else {
                return Err(fail("a line must be `key = value`".to_owned()));
            };
            config.set(key.trim(), value.trim()).map_err(fail)?;
        }
        Ok(config)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let number = |lo: f64, hi: f64| -> Result<f64, String> {
            let v: f64 = value
                .parse()
                .map_err(|_| format!("{key} needs a number, not `{value}`"))?;
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(v)
            } else {
                Err(format!("{key} must be from {lo} to {hi}"))
            }
        };
        let flag = || -> Result<bool, String> {
            match value {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(format!("{key} must be true or false")),
            }
        };
        let font = || -> Result<FontFace, String> {
            match value {
                "helvetica" => Ok(FontFace::Helvetica),
                "courier" => Ok(FontFace::Courier),
                _ => Err(format!("{key} must be helvetica or courier")),
            }
        };
        let words = || -> Result<String, String> {
            if value.chars().count() > 128 || value.chars().any(char::is_control) {
                Err(format!("{key} must be a text of at most 128 characters"))
            } else {
                Ok(value.to_owned())
            }
        };
        match key {
            "paper" => self.paper = Paper::parse(value).map_err(|e| e.detail)?,
            "orientation" => {
                self.orientation = match value {
                    "auto" => Orientation::Auto,
                    "landscape" => Orientation::Landscape,
                    "portrait" => Orientation::Portrait,
                    _ => return Err("orientation must be auto, landscape or portrait".to_owned()),
                }
            }
            "margin_mm" => self.margin_mm = number(0.0, 50.0)?,
            "scale" => {
                self.scale = parse_scale(value)
                    .ok_or_else(|| "scale must be auto, 1:2, 2:1 or a number".to_owned())?
            }
            "scale_rounding" => {
                self.rounding = match value {
                    "nice" => Rounding::Nice,
                    "exact" => Rounding::Exact,
                    _ => return Err("scale_rounding must be nice or exact".to_owned()),
                }
            }
            "frame_line_mm" => self.frame_line_mm = number(0.01, 5.0)?,
            "title_line_mm" => self.title_line_mm = number(0.01, 5.0)?,
            "thin_line_mm" => self.thin_line_mm = number(0.01, 5.0)?,
            "title_width_mm" => self.title_width_mm = number(60.0, 400.0)?,
            "title_height_mm" => self.title_height_mm = number(12.0, 60.0)?,
            "title_text_mm" => self.title_text_mm = number(1.0, 10.0)?,
            "label_text_mm" => self.label_text_mm = number(0.8, 6.0)?,
            "min_text_mm" => self.min_text_mm = number(0.3, 6.0)?,
            "font_title" => self.font_title = font()?,
            "font_labels" => self.font_labels = font()?,
            "project" => self.project = words()?,
            "revision" => self.revision = words()?,
            "author" => self.author = words()?,
            "scale_bar" => self.scale_bar = flag()?,
            "dimensions" => self.dimensions = flag()?,
            "side" => {
                self.side = match value {
                    "top" => SideChoice::Top,
                    "bottom" => SideChoice::Bottom,
                    "both" => SideChoice::Both,
                    _ => return Err("side must be top, bottom or both".to_owned()),
                }
            }
            "layers" => {
                self.layers = parse_layers(value)
                    .map_err(|name| format!("layers: `{name}` is not a layer"))?
            }
            "colour_background" => {
                self.background = Rgba::parse(value)
                    .ok_or_else(|| format!("{key} must be #rrggbb or #rrggbbaa"))?
            }
            other => {
                let kind = other
                    .strip_prefix("colour_")
                    .and_then(LayerKind::from_key)
                    .ok_or_else(|| format!("`{other}` is not a setting"))?;
                self.colours[kind.index()] = Rgba::parse(value)
                    .ok_or_else(|| format!("{key} must be #rrggbb or #rrggbbaa"))?;
            }
        }
        Ok(())
    }

    /// Checks the values that depend on each other.
    pub fn validate(&self) -> Result<(), PrintError> {
        let (short, long) = self.paper.sides();
        if short - 2.0 * self.margin_mm <= self.title_height_mm
            || short - 2.0 * self.margin_mm <= self.title_width_mm
            || long - 2.0 * self.margin_mm <= self.title_width_mm
        {
            return Err(PrintError::new(
                SETTINGS_INVALID,
                "the margin and the title block leave no room on the paper",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_example_file_is_the_default_settings() {
        let text = include_str!("../../../config/print.cfg");
        assert_eq!(
            PrintConfig::parse(text).unwrap(),
            PrintConfig::default(),
            "config/print.cfg must show the defaults"
        );
    }

    #[test]
    fn a_value_is_read_and_a_missing_one_keeps_its_default() {
        let c = PrintConfig::parse(
            "# a comment\n\npaper = A3\nmargin_mm = 5\nscale = 1:2\nfont_title = courier\n\
             colour_wire = #112233\nproject = A # B\nlayers = outline, top_copper\n",
        )
        .unwrap();
        assert_eq!(c.paper, Paper::A3);
        assert_eq!(c.margin_mm, 5.0);
        assert_eq!(c.scale, 0.5);
        assert_eq!(c.font_title, FontFace::Courier);
        assert_eq!(c.colour(LayerKind::Wire), Rgba::new(0x11, 0x22, 0x33, 255));
        assert_eq!(c.project, "A # B");
        assert_eq!(c.layers, vec![LayerKind::Outline, LayerKind::TopCopper]);
        assert_eq!(c.frame_line_mm, 0.5);
        assert!(c.wants(LayerKind::Outline));
        assert!(!c.wants(LayerKind::Mask));
        assert!(c.wants(LayerKind::Frame));
    }

    #[test]
    fn a_line_that_is_not_valid_is_refused_with_its_number() {
        for (text, line) in [
            ("nothing", 1),
            ("\nunknown = 1", 2),
            ("margin_mm = 99", 1),
            ("margin_mm = abc", 1),
            ("scale = 5:0", 1),
            ("scale = 0.0001", 1),
            ("font_title = comic", 1),
            ("scale_bar = yes", 1),
            ("orientation = sideways", 1),
            ("side = left", 1),
            ("layers = outline,foo", 1),
            ("colour_wire = red", 1),
            ("colour_nothing = #000000", 1),
            ("paper = A5", 1),
            ("project = a\u{7}b", 1),
        ] {
            let error = PrintConfig::parse(text).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
            assert!(error.detail.starts_with(&format!("line {line}:")), "{text}");
        }
    }

    #[test]
    fn scales_and_layer_lists_are_read() {
        assert_eq!(parse_scale("auto"), Some(0.0));
        assert_eq!(parse_scale("0"), Some(0.0));
        assert_eq!(parse_scale("2:1"), Some(2.0));
        assert_eq!(parse_scale("1:4"), Some(0.25));
        assert_eq!(parse_scale("1.5"), Some(1.5));
        assert_eq!(parse_scale("-1"), None);
        assert_eq!(parse_scale("1:"), None);
        assert_eq!(parse_layers("all").unwrap(), Vec::<LayerKind>::new());
        assert_eq!(
            parse_layers("wire,wire,symbol").unwrap(),
            vec![LayerKind::Wire, LayerKind::Symbol]
        );
        assert_eq!(parse_layers("wire,nope").unwrap_err(), "nope");
    }

    #[test]
    fn the_margin_and_the_title_block_must_leave_room() {
        let mut c = PrintConfig::default();
        assert!(c.validate().is_ok());
        c.margin_mm = 50.0;
        assert_eq!(c.validate().unwrap_err().code, SETTINGS_INVALID);
        let wide = PrintConfig {
            title_width_mm: 400.0,
            ..PrintConfig::default()
        };
        assert!(wide.validate().is_err());
    }
}
