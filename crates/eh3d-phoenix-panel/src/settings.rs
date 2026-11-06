// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the panel. Every size and count is read from a plain `key = value` text,
//! `config/panel.cfg`, and not buried in the code. A missing line keeps the default; an unknown key
//! or a value out of range is refused with its line number. Lengths are in nanometres.

use crate::codes::{PanelError, SETTINGS_INVALID};

/// The largest length a setting may take (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// Which sides of the matrix have a rail of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// No frame.
    None,
    /// A rail above and a rail below the boards.
    TopBottom,
    /// A rail left and a rail right of the boards.
    LeftRight,
    /// A rail on all four sides.
    All,
}

impl Frame {
    /// True when there is a rail above and below.
    pub fn has_top_bottom(self) -> bool {
        matches!(self, Frame::TopBottom | Frame::All)
    }

    /// True when there is a rail left and right.
    pub fn has_left_right(self) -> bool {
        matches!(self, Frame::LeftRight | Frame::All)
    }
}

/// How the boards are held together and then separated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joint {
    /// Tabs across a routed gap, with a row of small holes to break them.
    MouseBites,
    /// Boards side by side, cut by V-shaped grooves along the lines between them.
    VScore,
}

/// Where one board of a list goes and how it is turned: the lower left corner of its box, counted
/// from the lower left corner of the area inside the frame, a turn of 0, 90, 180 or 270 degrees
/// (counter-clockwise) and whether it is mirrored about its vertical axis first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// The number of the board in the list, from 0.
    pub board: usize,
    /// The horizontal position.
    pub x_nm: i64,
    /// The vertical position.
    pub y_nm: i64,
    /// The turn in degrees.
    pub rotation_deg: u32,
    /// True when the board is mirrored (and so seen from its other side).
    pub mirror: bool,
}

/// The sizes and counts of a panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelSettings {
    /// The boards across.
    pub columns: u32,
    /// The boards up.
    pub rows: u32,
    /// The routed gap between boards, and between a board and the frame.
    pub gap_nm: i64,
    /// The rails of the frame.
    pub frame: Frame,
    /// The width of a rail.
    pub frame_width_nm: i64,
    /// How the boards are separated.
    pub joint: Joint,
    /// The width of a tab, along the edge.
    pub tab_width_nm: i64,
    /// The tabs on each edge that has a neighbour.
    pub tabs_per_edge: u32,
    /// The diameter of a mouse bite hole.
    pub mouse_bite_drill_nm: i64,
    /// The distance between the centres of two mouse bite holes.
    pub mouse_bite_pitch_nm: i64,
    /// The mouse bite holes in a tab.
    pub mouse_bite_holes: u32,
    /// The diameter of the copper of a fiducial.
    pub fiducial_diameter_nm: i64,
    /// The diameter of the solder mask opening around a fiducial.
    pub fiducial_mask_diameter_nm: i64,
    /// The fiducials, at the corners of the frame: 0, 3 or 4.
    pub fiducial_count: u32,
    /// How far a fiducial is from the tooling hole of its corner, along the rail.
    pub fiducial_step_nm: i64,
    /// Put the fiducials on the bottom side too.
    pub fiducials_both_sides: bool,
    /// The diameter of a tooling hole.
    pub tooling_hole_diameter_nm: i64,
    /// The tooling holes, at the corners of the frame: 0, 2, 3 or 4.
    pub tooling_hole_count: u32,
    /// The distance of the centre of a tooling hole from the two edges of its corner.
    pub tooling_hole_margin_nm: i64,
    /// The width of the line that draws the outline and the V-score lines in the Gerber files.
    pub outline_width_nm: i64,
    /// The most boards of a panel.
    pub max_boards: u32,
    /// The largest side of a panel.
    pub max_panel_nm: i64,
    /// The turn of every board of a matrix, in degrees: 0, 90, 180 or 270.
    pub copy_rotation_deg: u32,
    /// Mirror every board of a matrix about its vertical axis.
    pub copy_mirror: bool,
    /// Add the number of the copy to the reference designators and the net names of each copy.
    pub copy_suffix: bool,
    /// What goes before the number of the copy.
    pub copy_suffix_separator: String,
    /// The widest row of boards when a list of boards is packed by rows.
    pub pack_row_width_nm: i64,
    /// How close a V-score line may pass to copper; 0 turns the check off.
    pub v_score_clearance_nm: i64,
    /// The fiducials on each board: 0, 2 or 3.
    pub local_fiducial_count: u32,
    /// The distance of a local fiducial from the two edges of its corner of the board.
    pub local_fiducial_inset_nm: i64,
    /// The places of the boards of a list (lines `place = ...`); empty to pack them by rows.
    pub placements: Vec<Placement>,
}

impl Default for PanelSettings {
    fn default() -> Self {
        PanelSettings {
            columns: 2,
            rows: 2,
            gap_nm: 2_000_000,
            frame: Frame::All,
            frame_width_nm: 8_000_000,
            joint: Joint::MouseBites,
            tab_width_nm: 3_000_000,
            tabs_per_edge: 2,
            mouse_bite_drill_nm: 500_000,
            mouse_bite_pitch_nm: 800_000,
            mouse_bite_holes: 3,
            fiducial_diameter_nm: 1_000_000,
            fiducial_mask_diameter_nm: 2_000_000,
            fiducial_count: 3,
            fiducial_step_nm: 12_000_000,
            fiducials_both_sides: false,
            tooling_hole_diameter_nm: 3_000_000,
            tooling_hole_count: 4,
            tooling_hole_margin_nm: 4_000_000,
            outline_width_nm: 100_000,
            max_boards: 400,
            max_panel_nm: 600_000_000,
            copy_rotation_deg: 0,
            copy_mirror: false,
            copy_suffix: false,
            copy_suffix_separator: "_".to_owned(),
            pack_row_width_nm: 150_000_000,
            v_score_clearance_nm: 300_000,
            local_fiducial_count: 0,
            local_fiducial_inset_nm: 3_000_000,
            placements: Vec::new(),
        }
    }
}

/// A line `place = BOARD,X_nm,Y_nm,ROTATION_DEG,MIRROR`.
fn parse_place(value: &str) -> Option<Placement> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    let [board, x, y, rotation, mirror] = parts.as_slice() else {
        return None;
    };
    Some(Placement {
        board: board.parse().ok()?,
        x_nm: x.parse().ok()?,
        y_nm: y.parse().ok()?,
        rotation_deg: rotation.parse().ok()?,
        mirror: parse_bool(mirror)?,
    })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl PanelSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<PanelSettings, PanelError> {
        let mut s = PanelSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                PanelError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                PanelError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "columns" => s.columns = value.parse().map_err(|_| bad())?,
                "rows" => s.rows = value.parse().map_err(|_| bad())?,
                "gap_nm" => s.gap_nm = value.parse().map_err(|_| bad())?,
                "frame" => {
                    s.frame = match value {
                        "none" => Frame::None,
                        "top_bottom" => Frame::TopBottom,
                        "left_right" => Frame::LeftRight,
                        "all" => Frame::All,
                        _ => return Err(bad()),
                    }
                }
                "frame_width_nm" => s.frame_width_nm = value.parse().map_err(|_| bad())?,
                "joint" => {
                    s.joint = match value {
                        "mouse_bites" => Joint::MouseBites,
                        "v_score" => Joint::VScore,
                        _ => return Err(bad()),
                    }
                }
                "tab_width_nm" => s.tab_width_nm = value.parse().map_err(|_| bad())?,
                "tabs_per_edge" => s.tabs_per_edge = value.parse().map_err(|_| bad())?,
                "mouse_bite_drill_nm" => {
                    s.mouse_bite_drill_nm = value.parse().map_err(|_| bad())?
                }
                "mouse_bite_pitch_nm" => {
                    s.mouse_bite_pitch_nm = value.parse().map_err(|_| bad())?
                }
                "mouse_bite_holes" => s.mouse_bite_holes = value.parse().map_err(|_| bad())?,
                "fiducial_diameter_nm" => {
                    s.fiducial_diameter_nm = value.parse().map_err(|_| bad())?
                }
                "fiducial_mask_diameter_nm" => {
                    s.fiducial_mask_diameter_nm = value.parse().map_err(|_| bad())?
                }
                "fiducial_count" => s.fiducial_count = value.parse().map_err(|_| bad())?,
                "fiducial_step_nm" => s.fiducial_step_nm = value.parse().map_err(|_| bad())?,
                "fiducials_both_sides" => {
                    s.fiducials_both_sides = parse_bool(value).ok_or_else(bad)?
                }
                "tooling_hole_diameter_nm" => {
                    s.tooling_hole_diameter_nm = value.parse().map_err(|_| bad())?
                }
                "tooling_hole_count" => s.tooling_hole_count = value.parse().map_err(|_| bad())?,
                "tooling_hole_margin_nm" => {
                    s.tooling_hole_margin_nm = value.parse().map_err(|_| bad())?
                }
                "outline_width_nm" => s.outline_width_nm = value.parse().map_err(|_| bad())?,
                "max_boards" => s.max_boards = value.parse().map_err(|_| bad())?,
                "max_panel_nm" => s.max_panel_nm = value.parse().map_err(|_| bad())?,
                "copy_rotation_deg" => s.copy_rotation_deg = value.parse().map_err(|_| bad())?,
                "copy_mirror" => s.copy_mirror = parse_bool(value).ok_or_else(bad)?,
                "copy_suffix" => s.copy_suffix = parse_bool(value).ok_or_else(bad)?,
                "copy_suffix_separator" => s.copy_suffix_separator = value.to_owned(),
                "pack_row_width_nm" => s.pack_row_width_nm = value.parse().map_err(|_| bad())?,
                "v_score_clearance_nm" => {
                    s.v_score_clearance_nm = value.parse().map_err(|_| bad())?
                }
                "local_fiducial_count" => {
                    s.local_fiducial_count = value.parse().map_err(|_| bad())?
                }
                "local_fiducial_inset_nm" => {
                    s.local_fiducial_inset_nm = value.parse().map_err(|_| bad())?
                }
                "place" => s.placements.push(parse_place(value).ok_or_else(bad)?),
                _ => {
                    return Err(PanelError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the layout relies on.
    pub fn validate(&self) -> Result<(), PanelError> {
        let fail = |what: &str| Err(PanelError::new(SETTINGS_INVALID, what.to_string()));
        let length = |v: i64| (0..LENGTH_LIMIT_NM).contains(&v);
        let size = |v: i64| (1..LENGTH_LIMIT_NM).contains(&v);
        if !(1..=64).contains(&self.columns) || !(1..=64).contains(&self.rows) {
            return fail("columns and rows must be between 1 and 64");
        }
        if !length(self.gap_nm) || !length(self.frame_width_nm) {
            return fail("gap_nm and frame_width_nm must be between 0 and 2^30");
        }
        if self.frame != Frame::None && self.frame_width_nm == 0 {
            return fail("frame_width_nm must be at least 1 when there is a frame");
        }
        if !size(self.outline_width_nm) {
            return fail("outline_width_nm must be at least 1 and below 2^30");
        }
        if !size(self.fiducial_diameter_nm)
            || !size(self.fiducial_mask_diameter_nm)
            || self.fiducial_mask_diameter_nm < self.fiducial_diameter_nm
        {
            return fail(
                "fiducial_diameter_nm must be at least 1 and not above fiducial_mask_diameter_nm",
            );
        }
        if !matches!(self.fiducial_count, 0 | 3 | 4) {
            return fail("fiducial_count must be 0, 3 or 4");
        }
        if !length(self.fiducial_step_nm) {
            return fail("fiducial_step_nm must be between 0 and 2^30");
        }
        if !size(self.tooling_hole_diameter_nm) || !length(self.tooling_hole_margin_nm) {
            return fail(
                "tooling_hole_diameter_nm must be at least 1 and tooling_hole_margin_nm 0 or more",
            );
        }
        if !matches!(self.tooling_hole_count, 0 | 2..=4) {
            return fail("tooling_hole_count must be 0, 2, 3 or 4");
        }
        if self.max_boards == 0 || !size(self.max_panel_nm) {
            return fail("max_boards must be at least 1 and max_panel_nm at least 1");
        }
        if !matches!(self.copy_rotation_deg, 0 | 90 | 180 | 270) {
            return fail("copy_rotation_deg must be 0, 90, 180 or 270");
        }
        let separator = &self.copy_suffix_separator;
        if separator.len() > 8
            || !separator
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        {
            return fail("copy_suffix_separator must be up to 8 letters, digits, _, - or .");
        }
        if !length(self.pack_row_width_nm) || !length(self.v_score_clearance_nm) {
            return fail("pack_row_width_nm and v_score_clearance_nm must be between 0 and 2^30");
        }
        if !matches!(self.local_fiducial_count, 0 | 2 | 3) {
            return fail("local_fiducial_count must be 0, 2 or 3");
        }
        if !length(self.local_fiducial_inset_nm) {
            return fail("local_fiducial_inset_nm must be between 0 and 2^30");
        }
        for place in &self.placements {
            if !matches!(place.rotation_deg, 0 | 90 | 180 | 270) {
                return fail("a place line needs a rotation of 0, 90, 180 or 270");
            }
            if !length(place.x_nm) || !length(place.y_nm) {
                return fail("a place line needs a position between 0 and 2^30");
            }
        }
        if self.joint == Joint::MouseBites {
            if !size(self.tab_width_nm) || !(1..=1000).contains(&self.tabs_per_edge) {
                return fail("tab_width_nm must be at least 1 and tabs_per_edge 1 to 1000");
            }
            if !size(self.mouse_bite_drill_nm)
                || !size(self.mouse_bite_pitch_nm)
                || !(1..=64).contains(&self.mouse_bite_holes)
            {
                return fail(
                    "mouse_bite_drill_nm and mouse_bite_pitch_nm must be at least 1 and mouse_bite_holes 1 to 64",
                );
            }
            if self.mouse_bite_pitch_nm < self.mouse_bite_drill_nm {
                return fail("mouse_bite_pitch_nm must not be below mouse_bite_drill_nm");
            }
            if self.gap_nm < self.mouse_bite_drill_nm {
                return fail("gap_nm must not be below mouse_bite_drill_nm");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(
            PanelSettings::parse("# nothing\n").unwrap(),
            PanelSettings::default()
        );
        let s = PanelSettings::parse(
            "columns = 3\nrows = 1\nframe = top_bottom\njoint = v_score\nfiducials_both_sides = true\n",
        )
        .unwrap();
        assert_eq!((s.columns, s.rows), (3, 1));
        assert_eq!(s.frame, Frame::TopBottom);
        assert!(s.frame.has_top_bottom() && !s.frame.has_left_right());
        assert_eq!(s.joint, Joint::VScore);
        assert!(s.fiducials_both_sides);
        assert_eq!(s.gap_nm, PanelSettings::default().gap_nm);
        assert!(Frame::All.has_left_right() && Frame::All.has_top_bottom());
        assert!(!Frame::None.has_left_right() && Frame::LeftRight.has_left_right());
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, needle) in [
            ("columns = many", "columns"),
            ("columns = 0", "columns"),
            ("rows = 65", "columns and rows"),
            ("colour = red", "unknown key"),
            ("oops", "key = value"),
            ("frame = round", "frame"),
            ("joint = glue", "joint"),
            ("fiducials_both_sides = 1", "fiducials_both_sides"),
            ("frame_width_nm = 0", "frame_width_nm"),
            ("fiducial_count = 2", "fiducial_count"),
            ("tooling_hole_count = 1", "tooling_hole_count"),
            ("fiducial_mask_diameter_nm = 100", "fiducial_diameter_nm"),
            ("tabs_per_edge = 0", "tabs_per_edge"),
            ("mouse_bite_pitch_nm = 100", "mouse_bite_pitch_nm"),
            ("gap_nm = 100000", "gap_nm"),
            ("max_boards = 0", "max_boards"),
            ("copy_rotation_deg = 45", "copy_rotation_deg"),
            ("copy_mirror = yes", "copy_mirror"),
            ("copy_suffix_separator = a b", "copy_suffix_separator"),
            ("local_fiducial_count = 4", "local_fiducial_count"),
            ("place = 0,1,2", "place"),
            ("place = 0,1,2,45,false", "rotation"),
            ("place = 0,-1,2,0,false", "position"),
        ] {
            let e = PanelSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let e = PanelSettings::parse("\n\ncolumns = x").unwrap_err();
        assert!(e.detail.starts_with("line 3"), "{}", e.detail);
    }

    #[test]
    fn the_places_of_a_list_are_read_one_line_each() {
        let s = PanelSettings::parse(
            "place = 0, 0, 0, 0, false\nplace = 1,40000000,0,90,true\ncopy_suffix = true\ncopy_suffix_separator = -\n",
        )
        .unwrap();
        assert_eq!(s.placements.len(), 2);
        assert_eq!(
            s.placements[1],
            Placement {
                board: 1,
                x_nm: 40_000_000,
                y_nm: 0,
                rotation_deg: 90,
                mirror: true,
            }
        );
        assert!(s.copy_suffix);
        assert_eq!(s.copy_suffix_separator, "-");
    }

    #[test]
    fn v_score_does_not_check_the_tabs() {
        let s = PanelSettings::parse("joint = v_score\ntabs_per_edge = 0\ngap_nm = 0\n").unwrap();
        assert_eq!(s.gap_nm, 0);
    }

    #[test]
    fn the_shipped_settings_file_is_the_defaults() {
        let text = include_str!("../../../config/panel.cfg");
        assert_eq!(
            PanelSettings::parse(text).unwrap(),
            PanelSettings::default()
        );
    }
}
