// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the script interpreter. Every limit, default, permission and list is read from a
//! plain `key = value` text, `config/script.cfg`, and not buried in the code. A missing line keeps
//! the default; an unknown key or a value out of range is refused with its line number. The
//! permissions to read and write files are off by default, and the list of folders they may use is
//! empty.

use crate::codes::{Diag, SETTINGS_INVALID};
use crate::values::Unit;

/// The settings of one run, of a report or of the reader of an output job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSettings {
    /// The largest script, in bytes.
    pub max_script_bytes: usize,
    /// The most lines of a script.
    pub max_lines: usize,
    /// The most commands of a script (and of all the scripts it includes).
    pub max_commands: usize,
    /// The most arguments of one command.
    pub max_args: usize,
    /// The longest text in quotes, in characters.
    pub max_string_chars: usize,
    /// The deepest nesting of scripts that include scripts.
    pub max_depth: usize,
    /// The most operations one run may make.
    pub max_operations: usize,
    /// The most script files a report reads.
    pub max_files: usize,
    /// The deepest folder a report walks into.
    pub max_walk_depth: usize,
    /// The largest file read by `OPEN`, in bytes.
    pub max_file_bytes: u64,
    /// The largest output job, in bytes.
    pub max_cam_bytes: usize,
    /// The most sections of an output job.
    pub max_cam_sections: usize,
    /// A line break ends a command like a semicolon does; a backslash at the end of a line joins it
    /// to the next one.
    pub newline_ends_command: bool,
    /// A command that is pending is skipped with a warning instead of stopping the run.
    pub skip_pending: bool,
    /// `SCRIPT` may read another script.
    pub allow_script: bool,
    /// `WRITE` may write a board.
    pub allow_write: bool,
    /// `OPEN` may read a board.
    pub allow_open: bool,
    /// The folders, relative to the folder of the script, that the file commands may use; `.`
    /// is the folder of the script itself. Empty: none.
    pub allowed_dirs: Vec<String>,
    /// The unit of a number without a unit when a script has not set one.
    pub default_unit: Unit,
    /// The grid at the start, in nanometres.
    pub default_grid_nm: i64,
    /// The width of a wire at the start, in nanometres.
    pub default_width_nm: i64,
    /// The drill of a via at the start, in nanometres.
    pub default_drill_nm: i64,
    /// The diameter of a via at the start, in nanometres.
    pub default_via_diameter_nm: i64,
    /// The height of a text at the start, in nanometres.
    pub default_text_height_nm: i64,
    /// The width of the stroke of a text, as a percentage of its height.
    pub text_ratio_percent: i64,
    /// The seed of the identities of new objects.
    pub id_seed: u64,
    /// The start of the clock of the identities of new objects, in milliseconds.
    pub id_epoch_ms: u64,
    /// The layer numbers of the original workflow and the names they stand for on the board.
    pub layer_numbers: Vec<(u32, String)>,
    /// The names of commands of the original language that are known and have no equivalent yet.
    pub known_pending: Vec<String>,
    /// The most steps of the interpreter (statements and expression nodes) a script of the second
    /// version may take before it is stopped.
    pub max_steps: u64,
    /// The most turns of one loop of a script of the second version.
    pub max_loop_iterations: u64,
    /// The deepest nesting of blocks (`if`, `for`, `while`) in a script of the second version.
    pub max_block_depth: usize,
    /// The deepest nesting of an expression in a script of the second version.
    pub max_expr_depth: usize,
    /// The most cells the values of the variables may hold together (a number or an object is one
    /// cell, a list is one cell and the cells of its items, a text is one cell and one more for
    /// every 16 characters).
    pub max_value_cells: usize,
    /// The most items of one list.
    pub max_list_len: usize,
    /// The most bytes a text made while running (a value or a data file) may have.
    pub max_text_bytes: usize,
    /// The most bytes of output (`print`) of one run.
    pub max_output_bytes: usize,
    /// The folder, relative to the folder of the script, that the data files of a script of the
    /// second version are read from and written to; empty: no file is reachable.
    pub data_dir: String,
    /// The most data files one run may read or write.
    pub max_data_files: usize,
    /// The largest data file one run may read, in bytes.
    pub max_data_bytes: u64,
    /// The options of `SET` that only change the original interface and are accepted and ignored.
    pub set_ignored: Vec<String>,
}

const KNOWN_PENDING: &str = "ADD, ARC, ASSIGN, ATTRIBUTE, AUTO, BOARD, BUS, CLASS, CLOSE, COPY, CUT, DESCRIPTION, DIMENSION, EDIT, ERRORS, EXPORT, FRAME, GATESWAP, HOLE, INFO, INVOKE, JUNCTION, LABEL, LOCK, MARK, MENU, MIRROR, MITER, NET, OPTIMIZE, PACKAGE, PAD, PASTE, PIN, PINSWAP, PLAY, POLYGON, PREFIX, PRINT, QUIT, REDO, REMOVE, RENAME, REPLACE, RIPUP, ROUTE, RUN, SCAN, SHOW, SIGNAL, SMASH, SPLIT, SYMBOL, TECHNOLOGY, UNDO, UPDATE, USE, WINDOW";
const SET_IGNORED: &str = "BEEP, CONFIRM, DISPLAY_MODE, PALETTE, COLOR_LAYER, FILL_LAYER, USED_LAYERS, UNDO_LOG, WIRE_BEND, AUTO_JUNCTION, POLYGON_RATSNEST, VECTOR_FONT";
const LAYER_NUMBERS: &str = "1:Top, 16:Bottom, 20:Dimension, 21:tPlace, 22:bPlace, 25:tNames, 26:bNames, 27:tValues, 28:bValues, 29:tStop, 30:bStop, 31:tCream, 32:bCream, 51:tDocu, 52:bDocu";

fn words(text: &str, upper: bool) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(|w| {
            if upper {
                w.to_ascii_uppercase()
            } else {
                w.to_owned()
            }
        })
        .collect()
}

fn layer_pairs(text: &str) -> Option<Vec<(u32, String)>> {
    let mut out = Vec::new();
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (number, name) = part.split_once(':')?;
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        out.push((number.trim().parse().ok()?, name.to_owned()));
    }
    Some(out)
}

impl Default for ScriptSettings {
    fn default() -> Self {
        ScriptSettings {
            max_script_bytes: 1_048_576,
            max_lines: 20_000,
            max_commands: 10_000,
            max_args: 512,
            max_string_chars: 256,
            max_depth: 4,
            max_operations: 100_000,
            max_files: 2_000,
            max_walk_depth: 16,
            max_file_bytes: 16_777_216,
            max_cam_bytes: 262_144,
            max_cam_sections: 256,
            newline_ends_command: true,
            skip_pending: false,
            allow_script: false,
            allow_write: false,
            allow_open: false,
            allowed_dirs: Vec::new(),
            default_unit: Unit::Mm,
            default_grid_nm: 1_270_000,
            default_width_nm: 254_000,
            default_drill_nm: 300_000,
            default_via_diameter_nm: 600_000,
            default_text_height_nm: 1_270_000,
            text_ratio_percent: 8,
            id_seed: 0x7363_7269_7074,
            id_epoch_ms: 1_700_000_000_000,
            layer_numbers: layer_pairs(LAYER_NUMBERS).unwrap_or_default(),
            known_pending: words(KNOWN_PENDING, true),
            max_steps: 1_000_000,
            max_loop_iterations: 10_000,
            max_block_depth: 16,
            max_expr_depth: 32,
            max_value_cells: 20_000,
            max_list_len: 5_000,
            max_text_bytes: 262_144,
            max_output_bytes: 65_536,
            data_dir: String::new(),
            max_data_files: 8,
            max_data_bytes: 1_048_576,
            set_ignored: words(SET_IGNORED, true),
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl ScriptSettings {
    /// Parse a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<ScriptSettings, Diag> {
        let mut s = ScriptSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| Diag::new(SETTINGS_INVALID, line, 1, "not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                Diag::new(
                    SETTINGS_INVALID,
                    line,
                    1,
                    format!("the value of {key} is not valid"),
                )
            };
            match key {
                "max_script_bytes" => s.max_script_bytes = value.parse().map_err(|_| bad())?,
                "max_lines" => s.max_lines = value.parse().map_err(|_| bad())?,
                "max_commands" => s.max_commands = value.parse().map_err(|_| bad())?,
                "max_args" => s.max_args = value.parse().map_err(|_| bad())?,
                "max_string_chars" => s.max_string_chars = value.parse().map_err(|_| bad())?,
                "max_depth" => s.max_depth = value.parse().map_err(|_| bad())?,
                "max_operations" => s.max_operations = value.parse().map_err(|_| bad())?,
                "max_files" => s.max_files = value.parse().map_err(|_| bad())?,
                "max_walk_depth" => s.max_walk_depth = value.parse().map_err(|_| bad())?,
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "max_cam_bytes" => s.max_cam_bytes = value.parse().map_err(|_| bad())?,
                "max_cam_sections" => s.max_cam_sections = value.parse().map_err(|_| bad())?,
                "newline_ends_command" => {
                    s.newline_ends_command = parse_bool(value).ok_or_else(bad)?
                }
                "skip_pending" => s.skip_pending = parse_bool(value).ok_or_else(bad)?,
                "allow_script" => s.allow_script = parse_bool(value).ok_or_else(bad)?,
                "allow_write" => s.allow_write = parse_bool(value).ok_or_else(bad)?,
                "allow_open" => s.allow_open = parse_bool(value).ok_or_else(bad)?,
                "allowed_dirs" => s.allowed_dirs = words(value, false),
                "default_unit" => {
                    s.default_unit = match Unit::from_word(value) {
                        Some(u) => u,
                        None => return Err(bad()),
                    }
                }
                "default_grid_nm" => s.default_grid_nm = value.parse().map_err(|_| bad())?,
                "default_width_nm" => s.default_width_nm = value.parse().map_err(|_| bad())?,
                "default_drill_nm" => s.default_drill_nm = value.parse().map_err(|_| bad())?,
                "default_via_diameter_nm" => {
                    s.default_via_diameter_nm = value.parse().map_err(|_| bad())?
                }
                "default_text_height_nm" => {
                    s.default_text_height_nm = value.parse().map_err(|_| bad())?
                }
                "text_ratio_percent" => s.text_ratio_percent = value.parse().map_err(|_| bad())?,
                "id_seed" => s.id_seed = value.parse().map_err(|_| bad())?,
                "id_epoch_ms" => s.id_epoch_ms = value.parse().map_err(|_| bad())?,
                "layer_numbers" => s.layer_numbers = layer_pairs(value).ok_or_else(bad)?,
                "known_pending" => s.known_pending = words(value, true),
                "max_steps" => s.max_steps = value.parse().map_err(|_| bad())?,
                "max_loop_iterations" => {
                    s.max_loop_iterations = value.parse().map_err(|_| bad())?
                }
                "max_block_depth" => s.max_block_depth = value.parse().map_err(|_| bad())?,
                "max_expr_depth" => s.max_expr_depth = value.parse().map_err(|_| bad())?,
                "max_value_cells" => s.max_value_cells = value.parse().map_err(|_| bad())?,
                "max_list_len" => s.max_list_len = value.parse().map_err(|_| bad())?,
                "max_text_bytes" => s.max_text_bytes = value.parse().map_err(|_| bad())?,
                "max_output_bytes" => s.max_output_bytes = value.parse().map_err(|_| bad())?,
                "data_dir" => s.data_dir = value.to_owned(),
                "max_data_files" => s.max_data_files = value.parse().map_err(|_| bad())?,
                "max_data_bytes" => s.max_data_bytes = value.parse().map_err(|_| bad())?,
                "set_ignored" => s.set_ignored = words(value, true),
                _ => {
                    return Err(Diag::new(
                        SETTINGS_INVALID,
                        line,
                        1,
                        format!("unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks every value against its range.
    pub fn validate(&self) -> Result<(), Diag> {
        let bad = |what: &str| Err(Diag::plain(SETTINGS_INVALID, what.to_owned()));
        for (name, value) in [
            ("max_script_bytes", self.max_script_bytes),
            ("max_lines", self.max_lines),
            ("max_commands", self.max_commands),
            ("max_args", self.max_args),
            ("max_string_chars", self.max_string_chars),
            ("max_operations", self.max_operations),
            ("max_files", self.max_files),
            ("max_cam_bytes", self.max_cam_bytes),
            ("max_cam_sections", self.max_cam_sections),
            ("max_block_depth", self.max_block_depth),
            ("max_expr_depth", self.max_expr_depth),
            ("max_value_cells", self.max_value_cells),
            ("max_list_len", self.max_list_len),
            ("max_text_bytes", self.max_text_bytes),
            ("max_output_bytes", self.max_output_bytes),
        ] {
            if value == 0 {
                return bad(&format!("{name} must be at least 1"));
            }
        }
        if self.max_depth > 32 {
            return bad("max_depth is beyond 32");
        }
        if self.max_walk_depth > 256 {
            return bad("max_walk_depth is beyond 256");
        }
        if self.max_file_bytes == 0 {
            return bad("max_file_bytes must be at least 1");
        }
        if self.max_steps == 0 || self.max_loop_iterations == 0 || self.max_data_bytes == 0 {
            return bad("max_steps, max_loop_iterations and max_data_bytes must be at least 1");
        }
        if self.max_block_depth > 64 || self.max_expr_depth > 256 {
            return bad("max_block_depth is beyond 64 or max_expr_depth beyond 256");
        }
        if self.max_data_files > 1024 {
            return bad("max_data_files is beyond 1024");
        }
        for (name, value) in [
            ("default_grid_nm", self.default_grid_nm),
            ("default_width_nm", self.default_width_nm),
            ("default_drill_nm", self.default_drill_nm),
            ("default_via_diameter_nm", self.default_via_diameter_nm),
            ("default_text_height_nm", self.default_text_height_nm),
        ] {
            if !(1..=100_000_000).contains(&value) {
                return bad(&format!("{name} is not 1 to 100000000"));
            }
        }
        if !(1..=100).contains(&self.text_ratio_percent) {
            return bad("text_ratio_percent is not 1 to 100");
        }
        if self.default_drill_nm >= self.default_via_diameter_nm {
            return bad("default_drill_nm must be smaller than default_via_diameter_nm");
        }
        if self.id_epoch_ms > 0x0000_FFFF_FFFF_FFFF {
            return bad("id_epoch_ms is beyond the range of the identities");
        }
        Ok(())
    }

    /// True when `name` (any case) is a known command without an equivalent yet.
    pub fn is_known_pending(&self, name: &str) -> bool {
        self.known_pending
            .iter()
            .any(|k| k.eq_ignore_ascii_case(name))
    }

    /// True when `option` (any case) is an option of `SET` that is accepted and ignored.
    pub fn is_ignored_option(&self, option: &str) -> bool {
        self.set_ignored
            .iter()
            .any(|k| k.eq_ignore_ascii_case(option))
    }

    /// The name that a layer number stands for.
    pub fn layer_name(&self, number: u32) -> Option<&str> {
        self.layer_numbers
            .iter()
            .find(|(n, _)| *n == number)
            .map(|(_, name)| name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_have_their_lists() {
        let s = ScriptSettings::default();
        assert!(s.validate().is_ok());
        assert!(!s.allow_script && !s.allow_write && !s.allow_open);
        assert!(s.allowed_dirs.is_empty());
        assert_eq!(s.layer_name(21), Some("tPlace"));
        assert_eq!(s.layer_name(99), None);
        assert!(s.is_known_pending("route"));
        assert!(!s.is_known_pending("WIRE"));
        assert!(s.is_ignored_option("confirm"));
        assert!(!s.is_ignored_option("GRID"));
    }

    #[test]
    fn the_shipped_file_parses_to_the_defaults() {
        let text = include_str!("../../../config/script.cfg");
        let parsed = ScriptSettings::parse(text).unwrap();
        assert_eq!(parsed, ScriptSettings::default());
    }

    #[test]
    fn a_text_changes_only_the_keys_it_names() {
        let s = ScriptSettings::parse(
            "# note\nmax_lines = 10\nallow_write = true\nallowed_dirs = out, work/sub\nlayer_numbers = 7:Mine\ndefault_unit = mil\n",
        )
        .unwrap();
        assert_eq!(s.max_lines, 10);
        assert!(s.allow_write);
        assert_eq!(
            s.allowed_dirs,
            vec!["out".to_owned(), "work/sub".to_owned()]
        );
        assert_eq!(s.layer_name(7), Some("Mine"));
        assert_eq!(s.layer_name(1), None);
        assert_eq!(s.default_unit, Unit::Mil);
        assert_eq!(s.max_commands, ScriptSettings::default().max_commands);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        for (text, line) in [
            ("max_lines 10", 1),
            ("\nnope = 1", 2),
            ("max_lines = ten", 1),
            ("\n\nallow_write = yes", 3),
            ("default_unit = furlong", 1),
            ("layer_numbers = x:Top", 1),
            ("layer_numbers = 3:", 1),
        ] {
            let e = ScriptSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert_eq!(e.line, line, "{text}");
        }
    }

    #[test]
    fn a_value_out_of_range_is_refused() {
        for text in [
            "max_lines = 0",
            "max_depth = 33",
            "text_ratio_percent = 0",
            "text_ratio_percent = 101",
            "default_width_nm = 0",
            "default_drill_nm = 600000",
            "id_epoch_ms = 281474976710656",
            "max_steps = 0",
            "max_block_depth = 65",
            "max_expr_depth = 0",
            "max_data_files = 1025",
            "max_loop_iterations = ten",
        ] {
            let e = ScriptSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
        }
    }
}
