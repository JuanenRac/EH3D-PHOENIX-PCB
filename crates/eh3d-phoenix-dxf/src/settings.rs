// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the DXF reader and writer. The names of the layers, the tolerances and the
//! limits are read from a plain `key = value` text, `config/dxf.cfg`, and not buried in the code.
//! A missing line keeps the default; an unknown key or a value out of range is refused with its
//! line number.

use crate::codes::{DxfError, SETTINGS_INVALID};

const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The unit that a drawing without a unit is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultUnit {
    /// Millimetres.
    Millimetre,
    /// Inches.
    Inch,
}

impl DefaultUnit {
    /// Millimetres in one unit.
    pub fn millimetres(self) -> f64 {
        match self {
            DefaultUnit::Millimetre => 1.0,
            DefaultUnit::Inch => 25.4,
        }
    }
}

/// The settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxfSettings {
    /// The layer of the board outline.
    pub layer_outline: String,
    /// The layer of the drilled holes of a board.
    pub layer_drills: String,
    /// The layer of the outlines of the pads.
    pub layer_pads: String,
    /// The layer of the silkscreen.
    pub layer_silk: String,
    /// The layer of the V-score lines of a panel.
    pub layer_vscore: String,
    /// The layer of the fiducials of a panel.
    pub layer_fiducials: String,
    /// The layer of the tabs of a panel.
    pub layer_tabs: String,
    /// The layer of the tooling holes of a panel.
    pub layer_tooling: String,
    /// The layer of the mouse bites of a panel.
    pub layer_mouse_bites: String,
    /// The layer of the texts.
    pub layer_text: String,
    /// Write the names of the boards as texts.
    pub write_text: bool,
    /// The height of a text, in nanometres.
    pub text_height_nm: i64,
    /// The length of an arm of a cross that marks a point, in nanometres.
    pub cross_size_nm: i64,
    /// How far apart two ends may be and still be joined into a chain, in nanometres.
    pub join_tolerance_nm: i64,
    /// How far a straight piece may be from the spline it replaces, in nanometres.
    pub spline_tolerance_nm: i64,
    /// How far a straight piece may be from the arc it replaces in an outline, in nanometres.
    pub arc_tolerance_nm: i64,
    /// The unit of a drawing that does not say.
    pub default_unit: DefaultUnit,
    /// The largest file read, in bytes.
    pub max_file_bytes: u64,
    /// The most entities a drawing may hold once the blocks are expanded.
    pub max_entities: usize,
    /// How deep blocks may hold blocks.
    pub max_block_depth: usize,
    /// The most blocks a drawing may define.
    pub max_blocks: usize,
    /// The most points of one spline.
    pub max_spline_points: usize,
    /// The most findings of a run that are listed (the rest are counted).
    pub max_findings: usize,
    /// The most edges of a contour that are checked for crossings.
    pub max_check_edges: usize,
    /// The most closed contours that are put in order by containment; with more the outline is
    /// refused (the work grows with the square of the number).
    pub max_contours: usize,
}

impl Default for DxfSettings {
    fn default() -> Self {
        DxfSettings {
            layer_outline: "OUTLINE".to_owned(),
            layer_drills: "DRILLS".to_owned(),
            layer_pads: "PADS".to_owned(),
            layer_silk: "SILKSCREEN".to_owned(),
            layer_vscore: "VSCORE".to_owned(),
            layer_fiducials: "FIDUCIALS".to_owned(),
            layer_tabs: "TABS".to_owned(),
            layer_tooling: "TOOLING".to_owned(),
            layer_mouse_bites: "MOUSEBITES".to_owned(),
            layer_text: "TEXT".to_owned(),
            write_text: false,
            text_height_nm: 1_000_000,
            cross_size_nm: 1_000_000,
            join_tolerance_nm: 10_000,
            spline_tolerance_nm: 5_000,
            arc_tolerance_nm: 5_000,
            default_unit: DefaultUnit::Millimetre,
            max_file_bytes: 16 * 1024 * 1024,
            max_entities: 500_000,
            max_block_depth: 8,
            max_blocks: 10_000,
            max_spline_points: 10_000,
            max_findings: 1_000,
            max_check_edges: 4_000,
            max_contours: 5_000,
        }
    }
}

impl DxfSettings {
    /// Reads the settings from the text of a file.
    pub fn parse(text: &str) -> Result<DxfSettings, DxfError> {
        let mut s = DxfSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                DxfError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                DxfError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "layer_outline" => s.layer_outline = layer_name(value).ok_or_else(bad)?,
                "layer_drills" => s.layer_drills = layer_name(value).ok_or_else(bad)?,
                "layer_pads" => s.layer_pads = layer_name(value).ok_or_else(bad)?,
                "layer_silk" => s.layer_silk = layer_name(value).ok_or_else(bad)?,
                "layer_vscore" => s.layer_vscore = layer_name(value).ok_or_else(bad)?,
                "layer_fiducials" => s.layer_fiducials = layer_name(value).ok_or_else(bad)?,
                "layer_tabs" => s.layer_tabs = layer_name(value).ok_or_else(bad)?,
                "layer_tooling" => s.layer_tooling = layer_name(value).ok_or_else(bad)?,
                "layer_mouse_bites" => s.layer_mouse_bites = layer_name(value).ok_or_else(bad)?,
                "layer_text" => s.layer_text = layer_name(value).ok_or_else(bad)?,
                "write_text" => {
                    s.write_text = match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(bad()),
                    }
                }
                "text_height_nm" => s.text_height_nm = value.parse().map_err(|_| bad())?,
                "cross_size_nm" => s.cross_size_nm = value.parse().map_err(|_| bad())?,
                "join_tolerance_nm" => s.join_tolerance_nm = value.parse().map_err(|_| bad())?,
                "spline_tolerance_nm" => {
                    s.spline_tolerance_nm = value.parse().map_err(|_| bad())?
                }
                "arc_tolerance_nm" => s.arc_tolerance_nm = value.parse().map_err(|_| bad())?,
                "default_unit" => {
                    s.default_unit = match value {
                        "mm" => DefaultUnit::Millimetre,
                        "inch" => DefaultUnit::Inch,
                        _ => return Err(bad()),
                    }
                }
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "max_entities" => s.max_entities = value.parse().map_err(|_| bad())?,
                "max_block_depth" => s.max_block_depth = value.parse().map_err(|_| bad())?,
                "max_blocks" => s.max_blocks = value.parse().map_err(|_| bad())?,
                "max_spline_points" => s.max_spline_points = value.parse().map_err(|_| bad())?,
                "max_findings" => s.max_findings = value.parse().map_err(|_| bad())?,
                "max_check_edges" => s.max_check_edges = value.parse().map_err(|_| bad())?,
                "max_contours" => s.max_contours = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(DxfError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges.
    pub fn validate(&self) -> Result<(), DxfError> {
        let fail = |what: &str| Err(DxfError::new(SETTINGS_INVALID, what.to_string()));
        for (name, value) in [
            ("text_height_nm", self.text_height_nm),
            ("cross_size_nm", self.cross_size_nm),
            ("spline_tolerance_nm", self.spline_tolerance_nm),
            ("arc_tolerance_nm", self.arc_tolerance_nm),
        ] {
            if value <= 0 || value > LENGTH_LIMIT_NM {
                return fail(&format!("{name} must be above 0 and within the range"));
            }
        }
        if self.join_tolerance_nm < 0 || self.join_tolerance_nm > LENGTH_LIMIT_NM {
            return fail("join_tolerance_nm must be from 0 and within the range");
        }
        if self.max_file_bytes == 0
            || self.max_entities == 0
            || self.max_blocks == 0
            || self.max_spline_points < 2
            || self.max_findings == 0
            || self.max_check_edges < 3
            || self.max_contours == 0
        {
            return fail("a limit is below its least value");
        }
        let layers = [
            &self.layer_outline,
            &self.layer_drills,
            &self.layer_pads,
            &self.layer_silk,
            &self.layer_vscore,
            &self.layer_fiducials,
            &self.layer_tabs,
            &self.layer_tooling,
            &self.layer_mouse_bites,
            &self.layer_text,
        ];
        for (i, a) in layers.iter().enumerate() {
            if layers[..i].iter().any(|b| b.eq_ignore_ascii_case(a)) {
                return fail("two layers have the same name");
            }
        }
        Ok(())
    }
}

/// A layer name that DXF accepts: not empty, at most 255 characters, none of `<>/\":;?*|=` and
/// no control character.
fn layer_name(value: &str) -> Option<String> {
    let bad = |c: char| c.is_control() || "<>/\\\":;?*|=`,".contains(c);
    (!value.is_empty() && value.chars().count() <= 255 && !value.chars().any(bad))
        .then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_are_valid_and_the_shipped_file_parses_to_them() {
        let d = DxfSettings::default();
        assert!(d.validate().is_ok());
        let shipped = include_str!("../../../config/dxf.cfg");
        assert_eq!(DxfSettings::parse(shipped).unwrap(), d);
    }

    #[test]
    fn the_limit_of_contours_is_read_and_must_be_above_zero() {
        assert_eq!(DxfSettings::default().max_contours, 5_000);
        assert_eq!(
            DxfSettings::parse("max_contours = 12\n")
                .unwrap()
                .max_contours,
            12
        );
        assert!(DxfSettings::parse("max_contours = 0\n").is_err());
        assert!(DxfSettings::parse("max_contours = -1\n").is_err());
        assert!(DxfSettings::parse("max_contours = many\n").is_err());
    }

    #[test]
    fn a_line_changes_one_value_and_the_rest_keep_the_default() {
        let s = DxfSettings::parse(
            "# note\nlayer_outline = EDGE\nwrite_text = true\ndefault_unit = inch\njoin_tolerance_nm = 0\n",
        )
        .unwrap();
        assert_eq!(s.layer_outline, "EDGE");
        assert!(s.write_text);
        assert_eq!(s.default_unit, DefaultUnit::Inch);
        assert_eq!(s.default_unit.millimetres(), 25.4);
        assert_eq!(s.join_tolerance_nm, 0);
        assert_eq!(s.layer_drills, DxfSettings::default().layer_drills);
    }

    #[test]
    fn what_is_wrong_is_refused_with_its_line() {
        for (text, needle) in [
            ("max_findings\n", "line 1: not key = value"),
            ("\nfoo = 1\n", "line 2: unknown key foo"),
            ("layer_outline = A/B\n", "the value of layer_outline"),
            ("layer_pads = \n", "the value of layer_pads"),
            ("write_text = maybe\n", "the value of write_text"),
            ("default_unit = mile\n", "the value of default_unit"),
            ("text_height_nm = 0\n", "text_height_nm must be"),
            ("join_tolerance_nm = -1\n", "join_tolerance_nm must be"),
            ("max_entities = 0\n", "a limit is below"),
            ("layer_tabs = vscore\n", "two layers have the same name"),
            ("max_blocks = abc\n", "the value of max_blocks"),
        ] {
            let e = DxfSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }
}
