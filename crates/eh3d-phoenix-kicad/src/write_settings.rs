// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the KiCad writers. Every value that decides what is written is read from a
//! settings text, `config/kicad_export.cfg`, and not buried in the writers.

/// The values the writers use. Lengths are in nanometres.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteSettings {
    /// The version number written in a board and in a footprint (the KiCad 8 format).
    pub pcb_version: u64,
    /// The version number written in a schematic and in a symbol library (the KiCad 8 format).
    pub sch_version: u64,
    /// The program name written as `generator`.
    pub generator: String,
    /// The library name put before the name of a footprint or a symbol (`Library:Name`).
    pub lib_nickname: String,
    /// The paper written when the schematic has no sheet.
    pub paper: String,
    /// The height of the text of a reference, a value and a pin name.
    pub text_size_nm: i64,
    /// The thickness of the strokes of that text.
    pub text_thickness_nm: i64,
    /// The width of the lines of the board outline.
    pub edge_width_nm: i64,
    /// The width of the lines of the body of a symbol.
    pub symbol_line_nm: i64,
    /// The smallest width of the copper of a zone.
    pub zone_min_thickness_nm: i64,
    /// The pitch of the hatch drawn on the outline of a zone.
    pub zone_hatch_nm: i64,
    /// The distance between two parts that have no place in the drawing.
    pub grid_nm: i64,
    /// The length of the pin of a symbol.
    pub pin_length_nm: i64,
    /// The distance between two pins that have no position.
    pub pin_pitch_nm: i64,
    /// The margin of the body of a symbol around its pins.
    pub body_margin_nm: i64,
    /// The most items of one kind that are written.
    pub max_items: usize,
}

impl Default for WriteSettings {
    fn default() -> Self {
        WriteSettings {
            pcb_version: 20_240_108,
            sch_version: 20_231_120,
            generator: "phoenix".to_owned(),
            lib_nickname: "phoenix".to_owned(),
            paper: "A4".to_owned(),
            text_size_nm: 1_000_000,
            text_thickness_nm: 150_000,
            edge_width_nm: 100_000,
            symbol_line_nm: 254_000,
            zone_min_thickness_nm: 250_000,
            zone_hatch_nm: 500_000,
            grid_nm: 25_400_000,
            pin_length_nm: 2_540_000,
            pin_pitch_nm: 2_540_000,
            body_margin_nm: 1_270_000,
            max_items: 2_000_000,
        }
    }
}

fn whole(value: &str, line: usize, key: &str) -> Result<i64, String> {
    let parsed: i64 = value
        .parse()
        .map_err(|_| format!("line {line}: {key}: not a whole number"))?;
    if parsed <= 0 {
        return Err(format!("line {line}: {key}: must be above zero"));
    }
    if parsed >= 1_073_741_824 && key != "max_items" && !key.ends_with("version") {
        return Err(format!("line {line}: {key}: too large"));
    }
    Ok(parsed)
}

fn word(value: &str, line: usize, key: &str) -> Result<String, String> {
    if value.is_empty() || value.len() > 64 || value.chars().any(|c| c.is_control()) {
        return Err(format!("line {line}: {key}: not a usable word"));
    }
    Ok(value.to_owned())
}

impl WriteSettings {
    /// Parses a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error that names the line.
    pub fn parse(text: &str) -> Result<WriteSettings, String> {
        let mut s = WriteSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {line}: expected key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "pcb_version" => s.pcb_version = whole(value, line, key)?.unsigned_abs(),
                "sch_version" => s.sch_version = whole(value, line, key)?.unsigned_abs(),
                "generator" => s.generator = word(value, line, key)?,
                "lib_nickname" => {
                    s.lib_nickname = word(value, line, key)?;
                    if s.lib_nickname.contains(':') {
                        return Err(format!("line {line}: {key}: must not contain a colon"));
                    }
                }
                "paper" => s.paper = word(value, line, key)?,
                "text_size_nm" => s.text_size_nm = whole(value, line, key)?,
                "text_thickness_nm" => s.text_thickness_nm = whole(value, line, key)?,
                "edge_width_nm" => s.edge_width_nm = whole(value, line, key)?,
                "symbol_line_nm" => s.symbol_line_nm = whole(value, line, key)?,
                "zone_min_thickness_nm" => s.zone_min_thickness_nm = whole(value, line, key)?,
                "zone_hatch_nm" => s.zone_hatch_nm = whole(value, line, key)?,
                "grid_nm" => s.grid_nm = whole(value, line, key)?,
                "pin_length_nm" => s.pin_length_nm = whole(value, line, key)?,
                "pin_pitch_nm" => s.pin_pitch_nm = whole(value, line, key)?,
                "body_margin_nm" => s.body_margin_nm = whole(value, line, key)?,
                "max_items" => {
                    s.max_items = usize::try_from(whole(value, line, key)?)
                        .map_err(|_| format!("line {line}: {key}: too large"))?;
                }
                _ => return Err(format!("line {line}: unknown key {key}")),
            }
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_settings_text_is_the_default() {
        let text = include_str!("../../../config/kicad_export.cfg");
        assert_eq!(WriteSettings::parse(text), Ok(WriteSettings::default()));
    }

    #[test]
    fn a_key_overrides_its_default_and_the_rest_stay() {
        let s = WriteSettings::parse("# note\n\nlib_nickname = mine\ngrid_nm=5\n").unwrap();
        assert_eq!(s.lib_nickname, "mine");
        assert_eq!(s.grid_nm, 5);
        assert_eq!(s.text_size_nm, WriteSettings::default().text_size_nm);
    }

    #[test]
    fn bad_lines_are_errors_with_their_line() {
        for (text, line) in [
            ("grid_nm", 1),
            ("\ngrid_nm = x", 2),
            ("grid_nm = 0", 1),
            ("\n\nwhat = 3", 3),
            ("grid_nm = -4", 1),
            ("lib_nickname = a:b", 1),
            ("paper =", 1),
            ("grid_nm = 99999999999", 1),
        ] {
            let error = WriteSettings::parse(text).unwrap_err();
            assert!(
                error.starts_with(&format!("line {line}:")),
                "{text}: {error}"
            );
        }
    }
}
