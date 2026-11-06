// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the viewer. Every limit, tolerance, size and colour is read from a settings
//! text, `config/viewer.cfg`, and not buried in the code.

use std::collections::BTreeMap;

use crate::appearance::{Theme, FONT_MAX, FONT_MIN, SCALE_MAX, SCALE_MIN};
use crate::codes::{ViewError, SETTINGS_INVALID};
use crate::keymap::Keymap;
use crate::measure_set::{format_length, Unit};

/// The colours that a settings text may name, with their defaults in the dark theme (red, green, blue,
/// alpha): the colours of the drawing and, after them, the colours of the controls of the window.
pub const DEFAULT_COLOURS: [(&str, [u8; 4]); 41] = [
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
    ("gerber_drill_plated", [0xe8, 0xe8, 0xf0, 0xff]),
    ("gerber_drill_unplated", [0xff, 0x8c, 0x42, 0xff]),
    ("gerber_only_files", [0xff, 0x54, 0x70, 0xe6]),
    ("gerber_only_board", [0x4d, 0xd0, 0xe1, 0xe6]),
    ("view3d_board", [0x1f, 0x6f, 0x45, 0xff]),
    ("view3d_top_part", [0x6b, 0x7f, 0x95, 0xff]),
    ("view3d_bottom_part", [0x95, 0x6b, 0x7f, 0xff]),
    ("view3d_collision", [0xff, 0x54, 0x70, 0xff]),
    ("view3d_too_tall", [0xff, 0xb0, 0x00, 0xff]),
    ("view3d_outside", [0xb4, 0x3c, 0xc8, 0xff]),
    ("view3d_selected", [0xf4, 0xec, 0xe0, 0xff]),
    ("ui_panel", [0x17, 0x13, 0x10, 0xff]),
    ("ui_raised", [0x22, 0x1c, 0x17, 0xff]),
    ("ui_text", [0xf4, 0xec, 0xe0, 0xff]),
    ("ui_dim", [0x9c, 0x8d, 0x7a, 0xff]),
    ("ui_accent", [0xff, 0xb0, 0x00, 0xff]),
    ("ui_ok", [0x9b, 0xe3, 0x4a, 0xff]),
    ("ui_danger", [0xff, 0x54, 0x70, 0xff]),
];

/// The colours of the light theme, with the colours of the controls.
pub const LIGHT_COLOURS: [(&str, [u8; 4]); 41] = [
    ("background", [0xf4, 0xf1, 0xea, 0xff]),
    ("top_copper", [0xa8, 0x74, 0x1a, 0xff]),
    ("bottom_copper", [0x1f, 0x5f, 0xa8, 0xff]),
    ("inner_copper_1", [0x1f, 0x8f, 0x4e, 0xff]),
    ("inner_copper_2", [0x8f, 0x2f, 0xa8, 0xff]),
    ("inner_copper_3", [0xa8, 0x54, 0x1f, 0xff]),
    ("inner_copper_4", [0x1f, 0x8f, 0x8f, 0xff]),
    ("mask", [0x2c, 0xa0, 0x2c, 0x46]),
    ("paste", [0x70, 0x70, 0x70, 0x78]),
    ("silk_top", [0x2b, 0x2b, 0x2b, 0xff]),
    ("silk_bottom", [0x6b, 0x6b, 0x6b, 0xff]),
    ("outline", [0x0a, 0x7f, 0x95, 0xff]),
    ("drill", [0x20, 0x20, 0x20, 0xff]),
    ("documentation", [0x6b, 0x6b, 0x6b, 0xff]),
    ("designator", [0x2b, 0x2b, 0x2b, 0xff]),
    ("sheet", [0x6b, 0x6b, 0x6b, 0xff]),
    ("symbol", [0x8a, 0x5a, 0x00, 0xff]),
    ("wire", [0x2f, 0x7d, 0x1a, 0xff]),
    ("pin", [0x30, 0x30, 0x40, 0xff]),
    ("label", [0x2b, 0x2b, 0x2b, 0xff]),
    ("highlight", [0xd0, 0x1a, 0x40, 0xff]),
    ("measure", [0xc4, 0x6a, 0x00, 0xff]),
    ("ratsnest", [0x30, 0x30, 0x40, 0xb4]),
    ("gerber_drill_plated", [0x30, 0x30, 0x40, 0xff]),
    ("gerber_drill_unplated", [0xd0, 0x60, 0x1a, 0xff]),
    ("gerber_only_files", [0xd0, 0x1a, 0x40, 0xe6]),
    ("gerber_only_board", [0x0a, 0x7f, 0x95, 0xe6]),
    ("view3d_board", [0x2f, 0x8f, 0x5a, 0xff]),
    ("view3d_top_part", [0x5a, 0x6f, 0x85, 0xff]),
    ("view3d_bottom_part", [0x85, 0x5a, 0x6f, 0xff]),
    ("view3d_collision", [0xd0, 0x1a, 0x40, 0xff]),
    ("view3d_too_tall", [0xc4, 0x6a, 0x00, 0xff]),
    ("view3d_outside", [0x8f, 0x2f, 0xa8, 0xff]),
    ("view3d_selected", [0x10, 0x10, 0x10, 0xff]),
    ("ui_panel", [0xec, 0xe8, 0xde, 0xff]),
    ("ui_raised", [0xfb, 0xf9, 0xf4, 0xff]),
    ("ui_text", [0x1c, 0x1a, 0x17, 0xff]),
    ("ui_dim", [0x5c, 0x56, 0x4c, 0xff]),
    ("ui_accent", [0x8a, 0x4a, 0x00, 0xff]),
    ("ui_ok", [0x1f, 0x6e, 0x12, 0xff]),
    ("ui_danger", [0xb0, 0x10, 0x2f, 0xff]),
];

/// The colours of the theme of high contrast, with the colours of the controls.
pub const CONTRAST_COLOURS: [(&str, [u8; 4]); 41] = [
    ("background", [0x00, 0x00, 0x00, 0xff]),
    ("top_copper", [0xff, 0xd4, 0x00, 0xff]),
    ("bottom_copper", [0x4d, 0xa6, 0xff, 0xff]),
    ("inner_copper_1", [0x00, 0xff, 0x80, 0xff]),
    ("inner_copper_2", [0xff, 0x4d, 0xff, 0xff]),
    ("inner_copper_3", [0xff, 0x8a, 0x3c, 0xff]),
    ("inner_copper_4", [0x00, 0xff, 0xff, 0xff]),
    ("mask", [0x00, 0xff, 0x00, 0x50]),
    ("paste", [0xff, 0xff, 0xff, 0x70]),
    ("silk_top", [0xff, 0xff, 0xff, 0xff]),
    ("silk_bottom", [0xc8, 0xc8, 0xc8, 0xff]),
    ("outline", [0x00, 0xff, 0xff, 0xff]),
    ("drill", [0x30, 0x30, 0x30, 0xff]),
    ("documentation", [0xc8, 0xc8, 0xc8, 0xff]),
    ("designator", [0xff, 0xff, 0xff, 0xff]),
    ("sheet", [0xc8, 0xc8, 0xc8, 0xff]),
    ("symbol", [0xff, 0xd4, 0x00, 0xff]),
    ("wire", [0x00, 0xff, 0x00, 0xff]),
    ("pin", [0xff, 0xff, 0xff, 0xff]),
    ("label", [0xff, 0xff, 0xff, 0xff]),
    ("highlight", [0xff, 0x2a, 0x2a, 0xff]),
    ("measure", [0xff, 0xff, 0x00, 0xff]),
    ("ratsnest", [0xff, 0xff, 0xff, 0xc8]),
    ("gerber_drill_plated", [0xff, 0xff, 0xff, 0xff]),
    ("gerber_drill_unplated", [0xff, 0x8a, 0x3c, 0xff]),
    ("gerber_only_files", [0xff, 0x2a, 0x2a, 0xf0]),
    ("gerber_only_board", [0x00, 0xff, 0xff, 0xff]),
    ("view3d_board", [0x00, 0xa0, 0x50, 0xff]),
    ("view3d_top_part", [0x80, 0xb0, 0xff, 0xff]),
    ("view3d_bottom_part", [0xff, 0x80, 0xb0, 0xff]),
    ("view3d_collision", [0xff, 0x2a, 0x2a, 0xff]),
    ("view3d_too_tall", [0xff, 0xff, 0x00, 0xff]),
    ("view3d_outside", [0xff, 0x4d, 0xff, 0xff]),
    ("view3d_selected", [0xff, 0xff, 0xff, 0xff]),
    ("ui_panel", [0x00, 0x00, 0x00, 0xff]),
    ("ui_raised", [0x14, 0x14, 0x14, 0xff]),
    ("ui_text", [0xff, 0xff, 0xff, 0xff]),
    ("ui_dim", [0xd0, 0xd0, 0xd0, 0xff]),
    ("ui_accent", [0xff, 0xff, 0x00, 0xff]),
    ("ui_ok", [0x00, 0xff, 0x00, 0xff]),
    ("ui_danger", [0xff, 0x5c, 0x5c, 0xff]),
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
    /// The number of sides of a circle of an imported Gerber layer (an aperture, the end of a
    /// line, a hole).
    pub gerber_circle_segments: usize,
    /// The angle of one straight piece of an arc of an imported Gerber layer, in degrees.
    pub gerber_arc_step_deg: f64,
    /// The most polygons of the scene of an imported Gerber set.
    pub gerber_max_polys: usize,
    /// The width of a line that the file gives no width, in millimetres.
    pub gerber_hairline_mm: f64,
    /// The opacity a layer of an imported Gerber set starts with, from above 0 to 1.
    pub gerber_default_opacity: f64,
    /// The side of a cell of the comparison of two layers, in millimetres.
    pub gerber_compare_cell_mm: f64,
    /// The most cells of the comparison of two layers; a larger area gets larger cells.
    pub gerber_compare_max_cells: usize,
    /// How far from a corner or a centre a click of the measure still snaps to it, in pixels.
    pub gerber_snap_px: f64,
    /// The field of view of the camera of the 3D window, in degrees.
    pub view3d_fov_deg: f64,
    /// The turn of the camera around the board when the window opens, in degrees.
    pub view3d_start_yaw_deg: f64,
    /// The rise of the camera above the board when the window opens, in degrees.
    pub view3d_start_pitch_deg: f64,
    /// The turn of the camera for each pixel the pointer moves, in degrees.
    pub view3d_rotate_deg_per_px: f64,
    /// The speed of the pan: 1 moves the scene exactly with the pointer.
    pub view3d_pan_speed: f64,
    /// The wheel multiplies the distance of the camera by `exp(-scroll * this)`.
    pub view3d_zoom_wheel_rate: f64,
    /// The closest the camera may come to the point it looks at, in millimetres.
    pub view3d_min_distance_mm: f64,
    /// The highest or lowest the camera may go, in degrees below 90.
    pub view3d_max_pitch_deg: f64,
    /// The factor that the heights are multiplied by when drawn (1 is true to scale).
    pub view3d_height_scale: f64,
    /// How much room the first view leaves around the scene (1 touches the edges).
    pub view3d_fit_factor: f64,
    /// The look of the window: dark, light or high contrast.
    pub theme: Theme,
    /// The scale of the interface, in percent.
    pub ui_scale_percent: u32,
    /// The size of the letters of the body text, in points.
    pub font_size_pt: f64,
    /// The unit of the readings of the measures.
    pub measure_unit: Unit,
    /// The decimals of the readings of the measures.
    pub measure_decimals: usize,
    /// Whether a click of the measure tool goes to the notable point near it.
    pub measure_snap: bool,
    /// How far from a notable point a click of the measure tool still snaps to it, in pixels.
    pub measure_snap_px: f64,
    /// The most measures that are kept besides the one in progress.
    pub measure_max_kept: usize,
    /// The most results that the search lists.
    pub search_max_results: usize,
    /// The least stretch of the drawing that the view shows around a result, in millimetres.
    pub search_context_mm: f64,
    /// The most steps of the history that the list shows.
    pub history_shown: usize,
    /// The most layer presets that are kept (never fewer than nine, one for each number key).
    pub preset_max: usize,
    /// The name of the file of the preferences, in the settings folder of the user.
    pub prefs_file: String,
    /// The largest file of preferences that is read, in bytes.
    pub prefs_max_bytes: u64,
    /// How many times the rename that completes the writing of the preferences is tried again.
    pub prefs_write_retries: usize,
    /// How long to wait between those tries, in milliseconds.
    pub prefs_write_retry_ms: u64,
    /// The least time between two writings of the preferences, in seconds.
    pub prefs_save_interval_s: f64,
    /// How much a zoom must change, as a fraction of itself, to be remembered.
    pub zoom_note_step: f64,
    /// The keys of the actions of the window.
    pub keymap: Keymap,
    /// The colours of the dark theme by name, each red, green, blue and alpha.
    pub dark_colours: BTreeMap<String, [u8; 4]>,
    /// The colours of the light theme.
    pub light_colours: BTreeMap<String, [u8; 4]>,
    /// The colours of the theme of high contrast.
    pub contrast_colours: BTreeMap<String, [u8; 4]>,
    /// The colours by name in use: the ones of the theme.
    pub colours: BTreeMap<String, [u8; 4]>,
}

fn table(list: &[(&str, [u8; 4])]) -> BTreeMap<String, [u8; 4]> {
    list.iter()
        .map(|(name, rgba)| ((*name).to_string(), *rgba))
        .collect()
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
            gerber_circle_segments: 32,
            gerber_arc_step_deg: 6.0,
            gerber_max_polys: 2_000_000,
            gerber_hairline_mm: 0.02,
            gerber_default_opacity: 1.0,
            gerber_compare_cell_mm: 0.05,
            gerber_compare_max_cells: 4_000_000,
            gerber_snap_px: 8.0,
            view3d_fov_deg: 40.0,
            view3d_start_yaw_deg: 35.0,
            view3d_start_pitch_deg: 35.0,
            view3d_rotate_deg_per_px: 0.4,
            view3d_pan_speed: 1.0,
            view3d_zoom_wheel_rate: 0.0015,
            view3d_min_distance_mm: 5.0,
            view3d_max_pitch_deg: 89.0,
            view3d_height_scale: 1.0,
            view3d_fit_factor: 1.15,
            theme: Theme::Dark,
            ui_scale_percent: 100,
            font_size_pt: 14.0,
            measure_unit: Unit::Mm,
            measure_decimals: 3,
            measure_snap: true,
            measure_snap_px: 10.0,
            measure_max_kept: 20,
            search_max_results: 200,
            search_context_mm: 10.0,
            history_shown: 200,
            preset_max: 30,
            prefs_file: "prefs.cfg".to_string(),
            prefs_max_bytes: 1_048_576,
            prefs_write_retries: 5,
            prefs_write_retry_ms: 50,
            prefs_save_interval_s: 1.0,
            zoom_note_step: 0.05,
            keymap: Keymap::default(),
            dark_colours: table(&DEFAULT_COLOURS),
            light_colours: table(&LIGHT_COLOURS),
            contrast_colours: table(&CONTRAST_COLOURS),
            colours: table(&DEFAULT_COLOURS),
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

fn signed(value: &str, line: usize, key: &str) -> Result<f64, ViewError> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| invalid(line, format!("{key}: not a number")))?;
    if !parsed.is_finite() {
        return Err(invalid(line, format!("{key}: must be a finite number")));
    }
    Ok(parsed)
}

fn flag(value: &str, line: usize, key: &str) -> Result<bool, ViewError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(invalid(line, format!("{key}: expected true or false"))),
    }
}

fn file_name(value: &str, line: usize, key: &str) -> Result<String, ViewError> {
    if value.is_empty() || value.contains(['/', '\\']) {
        return Err(invalid(
            line,
            format!("{key}: must be a non-empty file name without a slash"),
        ));
    }
    Ok(value.to_string())
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

    /// The colours of a theme by name.
    pub fn palette(&self, theme: Theme) -> &BTreeMap<String, [u8; 4]> {
        match theme {
            Theme::Dark => &self.dark_colours,
            Theme::Light => &self.light_colours,
            Theme::Contrast => &self.contrast_colours,
        }
    }

    /// Changes the theme: the colours in use become the ones of the theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.colours = self.palette(theme).clone();
    }

    /// A length in millimetres written in the unit and with the decimals of the settings.
    pub fn format_length(&self, mm: f64) -> String {
        format_length(mm, self.measure_unit, self.measure_decimals)
    }

    /// Parse a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error. Colours are
    /// `colour.<name> = #rrggbb` or `#rrggbbaa`.
    pub fn parse(text: &str) -> Result<ViewerSettings, ViewError> {
        let mut s = ViewerSettings::default();
        let mut key_changes: Vec<(usize, String, String)> = Vec::new();
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
            if let Some(rest) = key.strip_prefix("colour.") {
                let (palette, name) = if let Some(name) = rest.strip_prefix("light.") {
                    (&mut s.light_colours, name)
                } else if let Some(name) = rest.strip_prefix("contrast.") {
                    (&mut s.contrast_colours, name)
                } else {
                    (&mut s.dark_colours, rest)
                };
                if !DEFAULT_COLOURS.iter().any(|(n, _)| *n == name) {
                    return Err(invalid(line, format!("unknown colour {name}")));
                }
                let rgba = parse_colour(value).ok_or_else(|| {
                    invalid(line, format!("{key}: expected #rrggbb or #rrggbbaa"))
                })?;
                palette.insert(name.to_string(), rgba);
                continue;
            }
            if let Some(action) = key.strip_prefix("key.") {
                key_changes.push((line, action.to_string(), value.to_string()));
                continue;
            }
            match key {
                "theme" => {
                    s.theme = Theme::parse(value)
                        .ok_or_else(|| invalid(line, format!("{key}: dark, light or contrast")))?;
                }
                "ui_scale_percent" => {
                    let percent = u32::try_from(whole(value, line, key)?).unwrap_or(u32::MAX);
                    if !(SCALE_MIN..=SCALE_MAX).contains(&percent) {
                        return Err(invalid(
                            line,
                            format!("{key}: must be from {SCALE_MIN} to {SCALE_MAX}"),
                        ));
                    }
                    s.ui_scale_percent = percent;
                }
                "font_size_pt" => {
                    let points = number(value, line, key)?;
                    if !(FONT_MIN..=FONT_MAX).contains(&points) {
                        return Err(invalid(
                            line,
                            format!("{key}: must be from {FONT_MIN} to {FONT_MAX}"),
                        ));
                    }
                    s.font_size_pt = points;
                }
                "measure_unit" => {
                    s.measure_unit = Unit::parse(value)
                        .ok_or_else(|| invalid(line, format!("{key}: mm or mil")))?;
                }
                "measure_decimals" => {
                    let decimals = count(value, line, key)?;
                    if decimals > 6 {
                        return Err(invalid(line, format!("{key}: must not exceed 6")));
                    }
                    s.measure_decimals = decimals;
                }
                "measure_snap" => s.measure_snap = flag(value, line, key)?,
                "measure_snap_px" => s.measure_snap_px = number(value, line, key)?,
                "measure_max_kept" => s.measure_max_kept = count(value, line, key)?,
                "search_max_results" => s.search_max_results = count(value, line, key)?,
                "search_context_mm" => s.search_context_mm = number(value, line, key)?,
                "history_shown" => s.history_shown = count(value, line, key)?,
                "preset_max" => {
                    let most = count(value, line, key)?;
                    if most < 9 {
                        return Err(invalid(line, format!("{key}: must be at least 9")));
                    }
                    s.preset_max = most;
                }
                "prefs_file" => s.prefs_file = file_name(value, line, key)?,
                "prefs_max_bytes" => s.prefs_max_bytes = whole(value, line, key)?,
                "prefs_write_retries" => s.prefs_write_retries = count(value, line, key)?,
                "prefs_write_retry_ms" => s.prefs_write_retry_ms = whole(value, line, key)?,
                "prefs_save_interval_s" => s.prefs_save_interval_s = number(value, line, key)?,
                "zoom_note_step" => s.zoom_note_step = number(value, line, key)?,
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
                "gerber_circle_segments" => s.gerber_circle_segments = count(value, line, key)?,
                "gerber_arc_step_deg" => s.gerber_arc_step_deg = number(value, line, key)?,
                "gerber_max_polys" => s.gerber_max_polys = count(value, line, key)?,
                "gerber_hairline_mm" => s.gerber_hairline_mm = number(value, line, key)?,
                "gerber_default_opacity" => s.gerber_default_opacity = number(value, line, key)?,
                "gerber_compare_cell_mm" => s.gerber_compare_cell_mm = number(value, line, key)?,
                "gerber_compare_max_cells" => {
                    s.gerber_compare_max_cells = count(value, line, key)?;
                }
                "gerber_snap_px" => s.gerber_snap_px = number(value, line, key)?,
                "view3d_fov_deg" => s.view3d_fov_deg = number(value, line, key)?,
                "view3d_start_yaw_deg" => s.view3d_start_yaw_deg = signed(value, line, key)?,
                "view3d_start_pitch_deg" => s.view3d_start_pitch_deg = signed(value, line, key)?,
                "view3d_rotate_deg_per_px" => {
                    s.view3d_rotate_deg_per_px = number(value, line, key)?;
                }
                "view3d_pan_speed" => s.view3d_pan_speed = number(value, line, key)?,
                "view3d_zoom_wheel_rate" => s.view3d_zoom_wheel_rate = number(value, line, key)?,
                "view3d_min_distance_mm" => s.view3d_min_distance_mm = number(value, line, key)?,
                "view3d_max_pitch_deg" => s.view3d_max_pitch_deg = number(value, line, key)?,
                "view3d_height_scale" => s.view3d_height_scale = number(value, line, key)?,
                "view3d_fit_factor" => s.view3d_fit_factor = number(value, line, key)?,
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
        if s.gerber_circle_segments < 6 {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "gerber_circle_segments must be at least 6",
            ));
        }
        if s.gerber_default_opacity > 1.0 {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "gerber_default_opacity must not exceed 1",
            ));
        }
        if s.view3d_fov_deg >= 170.0 {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "view3d_fov_deg must be below 170",
            ));
        }
        if s.view3d_max_pitch_deg >= 90.0 {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "view3d_max_pitch_deg must be below 90",
            ));
        }
        if s.view3d_start_pitch_deg.abs() > s.view3d_max_pitch_deg {
            return Err(ViewError::new(
                SETTINGS_INVALID,
                "view3d_start_pitch_deg must lie within view3d_max_pitch_deg",
            ));
        }
        s.keymap = Keymap::with_changes(&key_changes).map_err(|(line, why)| invalid(line, why))?;
        s.colours = s.palette(s.theme).clone();
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{Action, Binding, KeyName};

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
            "gerber_circle_segments = 5",
            "gerber_arc_step_deg = 0",
            "gerber_default_opacity = 1.5",
            "gerber_compare_cell_mm = -1",
            "view3d_fov_deg = 170",
            "view3d_fov_deg = abc",
            "view3d_max_pitch_deg = 90",
            "view3d_start_yaw_deg = nan",
            "view3d_start_pitch_deg = 120",
            "view3d_min_distance_mm = 0",
            "colour.view3d_nothing = #000000",
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
    fn the_settings_of_the_gerber_scene_and_of_the_3d_window_are_read_and_checked() {
        let s = ViewerSettings::parse(
            "gerber_circle_segments = 12\ngerber_default_opacity = 0.5\nview3d_fov_deg = 60\nview3d_start_yaw_deg = -45\nview3d_start_pitch_deg = -10\nview3d_height_scale = 2\ncolour.view3d_collision = #112233\n",
        )
        .unwrap();
        assert_eq!(s.gerber_circle_segments, 12);
        assert_eq!(s.gerber_default_opacity, 0.5_f64);
        assert_eq!(s.view3d_fov_deg, 60.0_f64);
        assert_eq!(s.view3d_start_yaw_deg, -45.0_f64);
        assert_eq!(s.view3d_start_pitch_deg, -10.0_f64);
        assert_eq!(s.view3d_height_scale, 2.0_f64);
        assert_eq!(s.colour("view3d_collision"), [0x11, 0x22, 0x33, 0xff]);
        let d = ViewerSettings::default();
        assert_eq!(d.gerber_circle_segments, 32);
        assert_eq!(d.view3d_fov_deg, 40.0_f64);
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

    #[test]
    fn the_settings_of_the_window_are_read_and_checked() {
        let s = ViewerSettings::parse(
            "theme = light\nui_scale_percent = 150\nfont_size_pt = 16\nmeasure_unit = MIL\nmeasure_decimals = 2\nmeasure_snap = false\nmeasure_snap_px = 12\nmeasure_max_kept = 5\nsearch_max_results = 50\nsearch_context_mm = 4\nhistory_shown = 30\npreset_max = 12\nprefs_file = mine.cfg\nprefs_max_bytes = 2048\nprefs_write_retries = 2\nprefs_write_retry_ms = 10\nprefs_save_interval_s = 2.5\nzoom_note_step = 0.1\n",
        )
        .unwrap();
        assert_eq!(s.theme, Theme::Light);
        assert_eq!(s.ui_scale_percent, 150);
        assert_eq!(s.font_size_pt, 16.0_f64);
        assert_eq!(s.measure_unit, Unit::Mil);
        assert_eq!(s.measure_decimals, 2);
        assert!(!s.measure_snap);
        assert_eq!(s.measure_snap_px, 12.0_f64);
        assert_eq!(s.measure_max_kept, 5);
        assert_eq!(s.search_max_results, 50);
        assert_eq!(s.search_context_mm, 4.0_f64);
        assert_eq!(s.history_shown, 30);
        assert_eq!(s.preset_max, 12);
        assert_eq!(s.prefs_file, "mine.cfg");
        assert_eq!(s.prefs_max_bytes, 2048);
        assert_eq!(s.prefs_write_retries, 2);
        assert_eq!(s.prefs_write_retry_ms, 10);
        assert_eq!(s.prefs_save_interval_s, 2.5_f64);
        assert_eq!(s.zoom_note_step, 0.1_f64);
        assert_eq!(s.format_length(2.54), "100.00 mil");
        let d = ViewerSettings::default();
        assert_eq!(d.theme, Theme::Dark);
        assert_eq!(d.ui_scale_percent, 100);
        assert_eq!(d.format_length(1.5), "1.500 mm");
    }

    #[test]
    fn a_bad_value_of_the_window_settings_is_an_error_with_its_line() {
        for text in [
            "theme = blue",
            "ui_scale_percent = 49",
            "ui_scale_percent = 301",
            "ui_scale_percent = 1.5",
            "font_size_pt = 5",
            "font_size_pt = 49",
            "measure_unit = inch",
            "measure_decimals = 0",
            "measure_decimals = 7",
            "measure_snap = yes",
            "measure_snap_px = 0",
            "search_max_results = 0",
            "preset_max = 8",
            "prefs_file = ",
            "prefs_file = a/b",
            "prefs_save_interval_s = -1",
            "zoom_note_step = 0",
            "key.nothing = z",
            "key.undo = alt+z",
            "key.fit = v",
        ] {
            let error = ViewerSettings::parse(&format!("\n{text}")).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
            assert!(
                error.detail.starts_with("line 2"),
                "{text}: {}",
                error.detail
            );
        }
    }

    #[test]
    fn the_keys_come_from_the_settings_and_a_clash_names_the_line() {
        let s = ViewerSettings::parse("# keys\nkey.undo = ctrl+u\nkey.help = none\n").unwrap();
        assert_eq!(
            s.keymap.keys(Action::Undo),
            [Binding {
                key: KeyName::U,
                ctrl: true,
                shift: false
            }]
        );
        assert!(s.keymap.keys(Action::Help).is_empty());
        assert_eq!(ViewerSettings::default().keymap, Keymap::default());
        let error = ViewerSettings::parse("a = 1").unwrap_err();
        assert!(error.detail.starts_with("line 1"));
        let clash = ViewerSettings::parse("\n\nkey.redo = ctrl+z\n").unwrap_err();
        assert!(clash.detail.starts_with("line 3"), "{}", clash.detail);
        assert!(clash.detail.contains("undo"), "{}", clash.detail);
    }

    #[test]
    fn each_theme_has_its_own_colours_and_the_theme_chooses_the_ones_in_use() {
        let dark = ViewerSettings::default();
        let light = ViewerSettings::parse("theme = light\n").unwrap();
        let contrast = ViewerSettings::parse("theme = contrast\n").unwrap();
        assert_eq!(dark.theme, Theme::Dark);
        assert_ne!(light.colour("background"), dark.colour("background"));
        assert_eq!(contrast.colour("background"), [0, 0, 0, 255]);
        // the three palettes are always there
        assert_eq!(light.palette(Theme::Dark), dark.palette(Theme::Dark));
        assert_eq!(light.palette(Theme::Light), &light.colours);
        // a colour of a theme can be changed without being in use
        let s = ViewerSettings::parse("colour.light.wire = #010203\n").unwrap();
        assert_eq!(s.colour("wire"), dark.colour("wire"));
        assert_eq!(s.palette(Theme::Light)["wire"], [1, 2, 3, 255]);
        let s = ViewerSettings::parse("theme = light\ncolour.light.wire = #010203\n").unwrap();
        assert_eq!(s.colour("wire"), [1, 2, 3, 255]);
        for text in [
            "colour.light.nothing = #000000",
            "colour.contrast.wire = red",
        ] {
            assert!(ViewerSettings::parse(text).is_err(), "{text}");
        }
        // a theme set later brings its colours
        let mut s = ViewerSettings::default();
        s.set_theme(Theme::Contrast);
        assert_eq!(s.colour("background"), [0, 0, 0, 255]);
    }

    #[test]
    fn every_colour_of_every_theme_and_every_key_is_in_the_settings_file() {
        let text = include_str!("../../../config/viewer.cfg");
        for (name, _) in DEFAULT_COLOURS {
            for prefix in ["colour.light.", "colour.contrast."] {
                assert!(
                    text.lines()
                        .any(|l| l.trim_start().starts_with(&format!("{prefix}{name} "))),
                    "{prefix}{name}"
                );
            }
        }
        for action in Action::ALL {
            assert!(
                text.lines().any(|l| l
                    .trim_start()
                    .starts_with(&format!("key.{} = ", action.id()))),
                "{action:?}"
            );
        }
        assert_eq!(LIGHT_COLOURS.len(), DEFAULT_COLOURS.len());
        assert_eq!(CONTRAST_COLOURS.len(), DEFAULT_COLOURS.len());
        for (i, (name, _)) in DEFAULT_COLOURS.iter().enumerate() {
            assert_eq!(LIGHT_COLOURS[i].0, *name);
            assert_eq!(CONTRAST_COLOURS[i].0, *name);
        }
    }
}
