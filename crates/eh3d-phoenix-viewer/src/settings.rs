// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the viewer. Every limit, tolerance, size and colour is read from a settings
//! text, `config/viewer.cfg`, and not buried in the code.

use std::collections::BTreeMap;

use crate::codes::{ViewError, SETTINGS_INVALID};

/// The colours that a settings text may name, with their defaults (red, green, blue, alpha).
pub const DEFAULT_COLOURS: [(&str, [u8; 4]); 23] = [
    ("background", [0x0f, 0x0d, 0x0b, 0xff]),
    ("top_copper", [0xc8, 0xa3, 0x3c, 0xff]),
    ("bottom_copper", [0x3c, 0x78, 0xc8, 0xff]),
    ("inner_copper_1", [0x3c, 0xc8, 0x6e, 0xff]),
    ("inner_copper_2", [0xb4, 0x3c, 0xc8, 0xff]),
    ("inner_copper_3", [0xc8, 0x64, 0x3c, 0xff]),
    ("inner_copper_4", [0x3c, 0xc8, 0xc8, 0xff]),
    ("mask", [0x2c, 0xa0, 0x2c, 0x46]),
    ("paste", [0xb0, 0xb0, 0xb0, 0x5a]),
    ("silk_top", [0xf4, 0xec, 0xe0, 0xff]),
    ("silk_bottom", [0x9c, 0x8d, 0x7a, 0xff]),
    ("outline", [0x4d, 0xd0, 0xe1, 0xff]),
    ("drill", [0x10, 0x10, 0x10, 0xff]),
    ("documentation", [0x9c, 0x8d, 0x7a, 0xff]),
    ("designator", [0xf4, 0xec, 0xe0, 0xff]),
    ("sheet", [0x9c, 0x8d, 0x7a, 0xff]),
    ("symbol", [0xe0, 0xb0, 0x4c, 0xff]),
    ("wire", [0x9b, 0xe3, 0x4a, 0xff]),
    ("pin", [0xe8, 0xe8, 0xf0, 0xff]),
    ("label", [0xf4, 0xec, 0xe0, 0xff]),
    ("highlight", [0xff, 0x54, 0x70, 0xff]),
    ("measure", [0xff, 0xb0, 0x00, 0xff]),
    ("ratsnest", [0xe8, 0xe8, 0xf0, 0xb4]),
];

/// The limits, tolerances, sizes and colours of the viewer.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerSettings {
    /// The largest file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The most drawing primitives of one scene.
    pub max_primitives: usize,
    /// The smallest zoom, in pixels per millimetre.
    pub zoom_min_px_per_mm: f64,
    /// The largest zoom, in pixels per millimetre.
    pub zoom_max_px_per_mm: f64,
    /// The wheel multiplies the zoom by `exp(scroll * zoom_wheel_rate)`.
    pub zoom_wheel_rate: f64,
    /// The free space around a drawing fitted to the window, as a fraction of each side.
    pub fit_margin_fraction: f64,
    /// How far from an item a click still selects it, in pixels.
    pub pick_tolerance_px: f64,
    /// The error allowed when an arc is drawn as short lines, in pixels.
    pub arc_max_error_px: f64,
    /// The fewest lines an arc is drawn with.
    pub arc_min_segments: usize,
    /// The most lines an arc is drawn with.
    pub arc_max_segments: usize,
    /// A filled polygon with more points than this is drawn as its outline only.
    pub max_fill_points: usize,
    /// The width of the board outline, in millimetres.
    pub outline_width_mm: f64,
    /// The width of the lines of a symbol, in millimetres.
    pub symbol_line_width_mm: f64,
    /// The width of a wire of a schematic, in millimetres.
    pub wire_width_mm: f64,
    /// The height of the reference designators of the board, in millimetres.
    pub designator_height_mm: f64,
    /// The height of the labels of a schematic, in millimetres.
    pub label_height_mm: f64,
    /// The height of the pin numbers of a schematic, in millimetres.
    pub pin_number_height_mm: f64,
    /// The radius of a pin of a symbol, in millimetres.
    pub pin_radius_mm: f64,
    /// The space around the pins of a symbol, in millimetres.
    pub symbol_padding_mm: f64,
    /// The gap between two sheets laid side by side, in millimetres.
    pub sheet_gap_mm: f64,
    /// Text smaller than this on the screen is not drawn, in pixels.
    pub text_min_px: f64,
    /// The primitives per cell that the spatial index aims at.
    pub grid_target_per_cell: f64,
    /// A primitive that covers more cells than this is kept in a list of large ones.
    pub grid_max_cells_per_prim: usize,
    /// The most cells of the spatial index.
    pub grid_max_cells: usize,
    /// The size of the small benchmark board, in primitives.
    pub bench_small: usize,
    /// The size of the medium benchmark board, in primitives.
    pub bench_medium: usize,
    /// The size of the large benchmark board, in primitives.
    pub bench_large: usize,
    /// The picks made on each benchmark board.
    pub bench_picks: usize,
    /// How many of those picks are also made by a scan of every primitive.
    pub bench_linear_picks: usize,
    /// The distance between the generated copper items, in millimetres.
    pub bench_pitch_mm: f64,
    /// How far from an item a benchmark pick still hits it, in millimetres.
    pub bench_pick_tolerance_mm: f64,
    /// The seed of the generator of the benchmark boards.
    pub bench_seed: u64,
    /// The grid that the positions of an edit snap to, in millimetres.
    pub snap_mm: f64,
    /// How far one turn of a component goes, in degrees.
    pub rotate_step_deg: f64,
    /// How far from a vertex of a track a press still takes it, in pixels.
    pub vertex_pick_px: f64,
    /// How much the keys plus and minus change the width of a track, in millimetres.
    pub width_step_mm: f64,
    /// The width of the lines of the ratsnest, in pixels.
    pub ratsnest_width_px: f64,
    /// The radius of the ring that marks the place of a finding, in pixels.
    pub marker_radius_px: f64,
    /// The most findings that a panel lists.
    pub findings_shown: usize,
    /// The ending of the name of the copy that is made before a saved file is replaced.
    pub backup_suffix: String,
    /// How many recent files the start screen of the window keeps.
    pub recent_max: usize,
    /// The name of the file that holds the recent files, in the settings folder of the user.
    pub recent_file: String,
    /// The colours by name, each red, green, blue and alpha.
    pub colours: BTreeMap<String, [u8; 4]>,
}

impl Default for ViewerSettings {
    fn default() -> Self {
        ViewerSettings {
            max_file_bytes: 256 * 1024 * 1024,
            max_primitives: 4_000_000,
            zoom_min_px_per_mm: 0.05,
            zoom_max_px_per_mm: 4000.0,
            zoom_wheel_rate: 0.0015,
            fit_margin_fraction: 0.05,
            pick_tolerance_px: 5.0,
            arc_max_error_px: 0.3,
            arc_min_segments: 6,
            arc_max_segments: 256,
            max_fill_points: 2000,
            outline_width_mm: 0.1,
            symbol_line_width_mm: 0.2,
            wire_width_mm: 0.15,
            designator_height_mm: 1.0,
            label_height_mm: 1.2,
            pin_number_height_mm: 0.8,
            pin_radius_mm: 0.4,
            symbol_padding_mm: 2.54,
            sheet_gap_mm: 20.0,
            text_min_px: 5.0,
            grid_target_per_cell: 4.0,
            grid_max_cells_per_prim: 64,
            grid_max_cells: 1_000_000,
            bench_small: 1000,
            bench_medium: 20_000,
            bench_large: 200_000,
            bench_picks: 2000,
            bench_linear_picks: 200,
            bench_pitch_mm: 0.4,
            bench_pick_tolerance_mm: 0.25,
            bench_seed: 1,
            snap_mm: 0.05,
            rotate_step_deg: 90.0,
            vertex_pick_px: 8.0,
            width_step_mm: 0.05,
            ratsnest_width_px: 1.0,
            marker_radius_px: 14.0,
            findings_shown: 500,
            backup_suffix: ".bak".to_string(),
            recent_max: 10,
            recent_file: "recent.cfg".to_string(),
            colours: DEFAULT_COLOURS
                .iter()
                .map(|(name, rgba)| ((*name).to_string(), *rgba))
                .collect(),
        }
    }
}

fn invalid(line: usize, reason: String) -> ViewError {
    ViewError::new(SETTINGS_INVALID, format!("line {line}: {reason}"))
}

fn number(value: &str, line: usize, key: &str) -> Result<f64, ViewError> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| invalid(line, format!("{key}: not a number")))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(invalid(
            line,
            format!("{key}: must be a finite number above zero"),
        ));
    }
    Ok(parsed)
}

fn whole(value: &str, line: usize, key: &str) -> Result<u64, ViewError> {
    let parsed: u64 = value
        .parse()
        .map_err(|_| invalid(line, format!("{key}: not a whole number")))?;
    if parsed == 0 {
        return Err(invalid(line, format!("{key}: must be above zero")));
    }
    Ok(parsed)
}

fn count(value: &str, line: usize, key: &str) -> Result<usize, ViewError> {
    usize::try_from(whole(value, line, key)?)
        .map_err(|_| invalid(line, format!("{key}: too large")))
}

/// Reads `#rrggbb` or `#rrggbbaa`.
pub fn parse_colour(text: &str) -> Option<[u8; 4]> {
    let hex = text.trim().strip_prefix('#')?;
    if !(hex.len() == 6 || hex.len() == 8) || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let alpha = if hex.len() == 8 { byte(6)? } else { 0xff };
    Some([byte(0)?, byte(2)?, byte(4)?, alpha])
}

impl ViewerSettings {
    /// The colour of a name; a grey for a name that is not known (the settings text refuses such
    /// names, so only code can ask for one).
    pub fn colour(&self, name: &str) -> [u8; 4] {
        self.colours
            .get(name)
            .copied()
            .unwrap_or([0x80, 0x80, 0x80, 0xff])
    }

    /// Parse a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error. Colours are
    /// `colour.<name> = #rrggbb` or `#rrggbbaa`.
    pub fn parse(text: &str) -> Result<ViewerSettings, ViewError> {
        let mut s = ViewerSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| invalid(line, "expected key = value".to_string()))?;
            let (key, value) = (key.trim(), value.trim());
            if let Some(name) = key.strip_prefix("colour.") {
                if !DEFAULT_COLOURS.iter().any(|(n, _)| *n == name) {
                    return Err(invalid(line, format!("unknown colour {name}")));
                }
                let rgba = parse_colour(value).ok_or_else(|| {
                    invalid(line, format!("{key}: expected #rrggbb or #rrggbbaa"))
                })?;
                s.colours.insert(name.to_string(), rgba);
                continue;
            }
            match key {
                "max_file_bytes" => s.max_file_bytes = whole(value, line, key)?,
                "max_primitives" => s.max_primitives = count(value, line, key)?,
                "zoom_min_px_per_mm" => s.zoom_min_px_per_mm = number(value, line, key)?,
                "zoom_max_px_per_mm" => s.zoom_max_px_per_mm = number(value, line, key)?,
                "zoom_wheel_rate" => s.zoom_wheel_rate = number(value, line, key)?,
                "fit_margin_fraction" => s.fit_margin_fraction = number(value, line, key)?,
                "pick_tolerance_px" => s.pick_tolerance_px = number(value, line, key)?,
                "arc_max_error_px" => s.arc_max_error_px = number(value, line, key)?,
                "arc_min_segments" => s.arc_min_segments = count(value, line, key)?,
                "arc_max_segments" => s.arc_max_segments = count(value, line, key)?,
                "max_fill_points" => s.max_fill_points = count(value, line, key)?,
                "outline_width_mm" => s.outline_width_mm = number(value, line, key)?,
                "symbol_line_width_mm" => s.symbol_line_width_mm = number(value, line, key)?,
                "wire_width_mm" => s.wire_width_mm = number(value, line, key)?,
                "designator_height_mm" => s.designator_height_mm = number(value, line, key)?,
                "label_height_mm" => s.label_height_mm = number(value, line, key)?,
                "pin_number_height_mm" => s.pin_number_height_mm = number(value, line, key)?,
                "pin_radius_mm" => s.pin_radius_mm = number(value, line, key)?,
                "symbol_padding_mm" => s.symbol_padding_mm = number(value, line, key)?,
                "sheet_gap_mm" => s.sheet_gap_mm = number(value, line, key)?,
                "text_min_px" => s.text_min_px = number(value, line, key)?,
                "grid_target_per_cell" => s.grid_target_per_cell = number(value, line, key)?,
                "grid_max_cells_per_prim" => s.grid_max_cells_per_prim = count(value, line, key)?,
                "grid_max_cells" => s.grid_max_cells = count(value, line, key)?,
                "bench_small" => s.bench_small = count(value, line, key)?,
                "bench_medium" => s.bench_medium = count(value, line, key)?,
                "bench_large" => s.bench_large = count(value, line, key)?,
                "bench_picks" => s.bench_picks = count(value, line, key)?,
                "bench_linear_picks" => s.bench_linear_picks = count(value, line, key)?,
                "bench_pitch_mm" => s.bench_pitch_mm = number(value, line, key)?,
                "bench_pick_tolerance_mm" => {
                    s.bench_pick_tolerance_mm = number(value, line, key)?;
                }
                "bench_seed" => s.bench_seed = whole(value, line, key)?,
                "snap_mm" => s.snap_mm = number(value, line, key)?,
                "rotate_step_deg" => s.rotate_step_deg = number(value, line, key)?,
                "vertex_pick_px" => s.vertex_pick_px = number(value, line, key)?,
                "width_step_mm" => s.width_step_mm = number(value, line, key)?,
                "ratsnest_width_px" => s.ratsnest_width_px = number(value, line, key)?,
                "marker_radius_px" => s.marker_radius_px = number(value, line, key)?,
                "findings_shown" => s.findings_shown = count(value, line, key)?,
                "backup_suffix" => {
                    if value.is_empty() || value.contains(['/', '\\']) {
                        return Err(invalid(
                            line,
                            format!("{key}: must be a non-empty ending without a slash"),
                        ));
                    }
                    s.backup_suffix = value.to_string();
                }
                "recent_max" => s.recent_max = count(value, line, key)?,
                "recent_file" => {
                    if value.is_empty() || value.contains(['/', '\\']) {
                        return Err(invalid(
                            line,
                            format!("{key}: must be a non-empty file name without a slash"),
                        ));
                    }
                    s.recent_file = value.to_string();
                }
                _ => return Err(invalid(line, format!("unknown key {key}"))),
            }
        }
        if s.zoom_min_px_per_mm >= s.zoom_max_px_per_mm {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "zoom_min_px_per_mm must be below zoom_max_px_per_mm",
            ));
        }
        if s.fit_margin_fraction >= 0.5 {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "fit_margin_fraction must be below 0.5",
            ));
        }
        if s.rotate_step_deg > 360.0 {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "rotate_step_deg must not exceed 360",
            ));
        }
        if s.arc_min_segments > s.arc_max_segments {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "arc_min_segments must not exceed arc_max_segments",
            ));
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_of_the_repository_holds_the_defaults() {
        let parsed = ViewerSettings::parse(include_str!("../../../config/viewer.cfg")).unwrap();
        assert_eq!(parsed, ViewerSettings::default());
    }

    #[test]
    fn a_value_replaces_its_default_and_the_rest_stays() {
        let s =
            ViewerSettings::parse("pick_tolerance_px = 9\n# a comment\n\ncolour.wire = #102030\n")
                .unwrap();
        assert_eq!(s.pick_tolerance_px, 9.0_f64);
        assert_eq!(s.colour("wire"), [0x10, 0x20, 0x30, 0xff]);
        assert_eq!(s.max_primitives, 4_000_000);
    }

    #[test]
    fn colours_take_an_optional_alpha_and_refuse_anything_else() {
        assert_eq!(parse_colour("#0a0b0c80"), Some([10, 11, 12, 0x80]));
        assert_eq!(parse_colour("#0a0b0c"), Some([10, 11, 12, 0xff]));
        assert_eq!(parse_colour("0a0b0c"), None);
        assert_eq!(parse_colour("#0a0b"), None);
        assert_eq!(parse_colour("#zz0b0c"), None);
        assert_eq!(parse_colour("#é0b0c0"), None);
    }

    #[test]
    fn a_bad_line_is_an_error_with_its_line_number_and_never_a_panic() {
        for text in [
            "nonsense",
            "unknown_key = 1",
            "pick_tolerance_px = abc",
            "pick_tolerance_px = -1",
            "max_primitives = 0",
            "max_primitives = 1.5",
            "colour.nothing = #000000",
            "colour.wire = red",
            "zoom_min_px_per_mm = 10\nzoom_max_px_per_mm = 5",
            "fit_margin_fraction = 0.5",
            "arc_min_segments = 10\narc_max_segments = 5",
            "snap_mm = 0",
            "rotate_step_deg = 400",
            "backup_suffix = ",
            "backup_suffix = a/b",
            "recent_max = 0",
            "recent_file = ",
            "recent_file = a/b",
        ] {
            let error = ViewerSettings::parse(text).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
        }
        let error = ViewerSettings::parse("\n\nbad").unwrap_err();
        assert!(error.detail.starts_with("line 3"), "{}", error.detail);
    }

    #[test]
    fn the_settings_of_the_editing_are_read_and_checked() {
        let s = ViewerSettings::parse(
            "snap_mm = 0.1\nrotate_step_deg = 45\nfindings_shown = 7\nbackup_suffix = .old\ncolour.ratsnest = #112233\n",
        )
        .unwrap();
        assert_eq!(s.snap_mm, 0.1_f64);
        assert_eq!(s.rotate_step_deg, 45.0_f64);
        assert_eq!(s.findings_shown, 7);
        assert_eq!(s.backup_suffix, ".old");
        assert_eq!(s.colour("ratsnest"), [0x11, 0x22, 0x33, 0xff]);
    }

    #[test]
    fn every_default_colour_is_in_the_settings_file() {
        let text = include_str!("../../../config/viewer.cfg");
        for (name, _) in DEFAULT_COLOURS {
            assert!(
                text.lines()
                    .any(|l| l.trim_start().starts_with(&format!("colour.{name} "))),
                "{name}"
            );
        }
    }
}
