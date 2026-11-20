// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the IPC-2581 writer. What goes into the file and every limit are read from a
//! plain `key = value` text, `config/ipc2581.cfg`, and not buried in the code. A missing line keeps
//! the default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{Ipc2581Error, SETTINGS_INVALID};

/// The settings of one export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipc2581Settings {
    /// The date and time the history record of the file carries, as `YYYY-MM-DDTHH:MM:SS`. It is
    /// a setting and not the clock so that the same board always gives the same file.
    pub origination: String,
    /// Write the bill of materials (the parts grouped by value and footprint).
    pub include_bom: bool,
    /// Write the tracks.
    pub include_tracks: bool,
    /// Write the zones, as their outlines.
    pub include_zones: bool,
    /// Write the lines of the dimensions on a layer of the function DOCUMENT.
    pub include_dimensions: bool,
    /// Write the texts of the dimensions (their measures) on that layer too; with
    /// `include_dimensions` off nothing of the dimensions is written.
    pub include_dimension_texts: bool,
    /// The height of the texts of the dimensions, in nanometres; zero keeps the height each
    /// dimension has.
    pub dimension_text_height_nm: i64,
    /// The most objects (parts, connected pads, tracks, vias and zones) a board may hold for the
    /// file to be made; it also limits the pads of the IPC-D-356A file.
    pub max_items: usize,
    /// Write the pads without a hole in the IPC-D-356A file (`phoenix ipc2581 --d356`) as well as
    /// the pads with one.
    pub d356_include_smd: bool,
    /// The name of the net that a pad on no net has in the IPC-D-356A file: from 1 to 14 printable
    /// characters without a blank.
    pub d356_unconnected_name: String,
    /// The largest IPC-2581 file the reader accepts, in bytes (`phoenix ipc2581-read`).
    pub read_max_bytes: usize,
    /// The name of the writer profile that the file carries as a comment (`phoenix-ipc2581/1:default`): from
    /// 1 to 64 letters, digits, `.`, `_` and `-`.
    pub profile: String,
    /// The deepest nesting of elements the reader accepts.
    pub read_max_depth: usize,
    /// The most elements a file may hold for the reader to accept it.
    pub read_max_elements: usize,
    /// The most items (holes, parts, nets, pads, tracks, regions) the reader keeps counting
    /// before it refuses the file.
    pub read_max_items: usize,
    /// Keep the geometry of the tracks, arcs, polylines and regions (the counts are always kept).
    pub read_keep_geometry: bool,
    /// The most attributes the reader accepts on one element.
    pub read_max_attrs: usize,
    /// The most findings the reader lists for a file; the rest are counted in one last finding.
    pub read_max_findings: usize,
    /// The frequency, in MHz, of the single point of a material that is made from a file (the
    /// file gives a dielectric constant and a loss tangent but not the frequency).
    pub read_reference_mhz: usize,
    /// The most bytes that all the entries of a ZIP archive may hold once unpacked (the archive
    /// is unpacked in memory and never written to a folder).
    pub zip_max_total_bytes: usize,
    /// The most entries a ZIP archive may list.
    pub zip_max_entries: usize,
    /// The most that an entry may grow when unpacked, as a multiple of its packed size.
    pub zip_max_ratio: usize,
}

/// The tag of the format of these settings, for the identity of the writer profile.
pub const WRITER_FORMAT: &str = "phoenix-ipc2581/1";

impl Default for Ipc2581Settings {
    fn default() -> Self {
        Ipc2581Settings {
            origination: "1970-01-01T00:00:00".to_owned(),
            include_bom: true,
            include_tracks: true,
            include_zones: true,
            include_dimensions: true,
            include_dimension_texts: true,
            dimension_text_height_nm: 0,
            max_items: 1_000_000,
            d356_include_smd: true,
            d356_unconnected_name: "N/C".to_owned(),
            read_max_bytes: 128 * 1024 * 1024,
            profile: "default".to_owned(),
            read_max_depth: 64,
            read_max_elements: 50_000_000,
            read_max_items: 20_000_000,
            read_keep_geometry: true,
            read_max_attrs: 256,
            read_max_findings: 200,
            read_reference_mhz: 1000,
            zip_max_total_bytes: 1024 * 1024 * 1024,
            zip_max_entries: 10_000,
            zip_max_ratio: 1100,
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

/// True for a text of the form `YYYY-MM-DDTHH:MM:SS` with digits where they belong.
fn is_date_time(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 19
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            _ => c.is_ascii_digit(),
        })
}

impl Ipc2581Settings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<Ipc2581Settings, Ipc2581Error> {
        let mut s = Ipc2581Settings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                Ipc2581Error::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                Ipc2581Error::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "origination" => value.clone_into(&mut s.origination),
                "include_bom" => s.include_bom = parse_bool(value).ok_or_else(bad)?,
                "include_tracks" => s.include_tracks = parse_bool(value).ok_or_else(bad)?,
                "include_zones" => s.include_zones = parse_bool(value).ok_or_else(bad)?,
                "include_dimensions" => s.include_dimensions = parse_bool(value).ok_or_else(bad)?,
                "include_dimension_texts" => {
                    s.include_dimension_texts = parse_bool(value).ok_or_else(bad)?;
                }
                "dimension_text_height_nm" => {
                    s.dimension_text_height_nm = value.parse().map_err(|_| bad())?;
                }
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                "d356_include_smd" => s.d356_include_smd = parse_bool(value).ok_or_else(bad)?,
                "d356_unconnected_name" => value.clone_into(&mut s.d356_unconnected_name),
                "profile" => value.clone_into(&mut s.profile),
                "read_max_bytes" => s.read_max_bytes = value.parse().map_err(|_| bad())?,
                "read_max_depth" => s.read_max_depth = value.parse().map_err(|_| bad())?,
                "read_max_elements" => s.read_max_elements = value.parse().map_err(|_| bad())?,
                "read_max_items" => s.read_max_items = value.parse().map_err(|_| bad())?,
                "read_keep_geometry" => s.read_keep_geometry = parse_bool(value).ok_or_else(bad)?,
                "read_max_attrs" => s.read_max_attrs = value.parse().map_err(|_| bad())?,
                "read_max_findings" => s.read_max_findings = value.parse().map_err(|_| bad())?,
                "read_reference_mhz" => s.read_reference_mhz = value.parse().map_err(|_| bad())?,
                "zip_max_total_bytes" => {
                    s.zip_max_total_bytes = value.parse().map_err(|_| bad())?
                }
                "zip_max_entries" => s.zip_max_entries = value.parse().map_err(|_| bad())?,
                "zip_max_ratio" => s.zip_max_ratio = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(Ipc2581Error::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// The identity that the file carries: the tag of the format and the name of the profile.
    pub fn writer_id(&self) -> Result<String, Ipc2581Error> {
        eh3d_phoenix_model::writer_profile::identity(WRITER_FORMAT, &self.profile).map_err(|_| {
            Ipc2581Error::new(
                SETTINGS_INVALID,
                "profile must be 1 to 64 letters, digits, '.', '_' or '-' (no two '-' in a row)",
            )
        })
    }

    /// Checks the ranges that the writer relies on.
    pub fn validate(&self) -> Result<(), Ipc2581Error> {
        self.writer_id()?;
        if !is_date_time(&self.origination) {
            return Err(Ipc2581Error::new(
                SETTINGS_INVALID,
                "origination must be like 1970-01-01T00:00:00",
            ));
        }
        if !(0..=1_000_000_000).contains(&self.dimension_text_height_nm) {
            return Err(Ipc2581Error::new(
                SETTINGS_INVALID,
                "dimension_text_height_nm must be between 0 and 1000000000",
            ));
        }
        if !(1..=100_000_000).contains(&self.max_items) {
            return Err(Ipc2581Error::new(
                SETTINGS_INVALID,
                "max_items must be between 1 and 100000000",
            ));
        }
        let ranges: [(&str, usize, usize, usize); 10] = [
            ("read_max_bytes", self.read_max_bytes, 1, 2_000_000_000),
            ("read_max_depth", self.read_max_depth, 4, 1000),
            (
                "read_max_elements",
                self.read_max_elements,
                1,
                1_000_000_000,
            ),
            ("read_max_items", self.read_max_items, 1, 1_000_000_000),
            ("read_max_attrs", self.read_max_attrs, 1, 10_000),
            ("read_max_findings", self.read_max_findings, 1, 100_000),
            ("read_reference_mhz", self.read_reference_mhz, 1, 1_000_000),
            (
                "zip_max_total_bytes",
                self.zip_max_total_bytes,
                1,
                4_000_000_000,
            ),
            ("zip_max_entries", self.zip_max_entries, 1, 100_000),
            ("zip_max_ratio", self.zip_max_ratio, 1, 100_000),
        ];
        for (key, value, low, high) in ranges {
            if !(low..=high).contains(&value) {
                return Err(Ipc2581Error::new(
                    SETTINGS_INVALID,
                    format!("{key} must be between {low} and {high}"),
                ));
            }
        }
        let name = &self.d356_unconnected_name;
        if !(1..=14).contains(&name.len()) || !name.bytes().all(|c| c.is_ascii_graphic()) {
            return Err(Ipc2581Error::new(
                SETTINGS_INVALID,
                "d356_unconnected_name must be from 1 to 14 printable characters without a blank",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(Ipc2581Settings::default().validate().is_ok());
        assert_eq!(
            Ipc2581Settings::parse("# nothing\n\n").unwrap(),
            Ipc2581Settings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/ipc2581.cfg");
        assert_eq!(
            Ipc2581Settings::parse(text).unwrap(),
            Ipc2581Settings::default()
        );
    }

    #[test]
    fn lines_change_the_values() {
        let s = Ipc2581Settings::parse(
            "origination = 2000-02-03T04:05:06\ninclude_bom = false\ninclude_tracks = false\ninclude_zones = false\ninclude_dimensions = false\ninclude_dimension_texts = false\ndimension_text_height_nm = 1500000\nmax_items = 9\n",
        )
        .unwrap();
        assert_eq!(s.origination, "2000-02-03T04:05:06");
        assert!(!s.include_bom && !s.include_tracks && !s.include_zones && !s.include_dimensions);
        assert!(!s.include_dimension_texts);
        assert_eq!(s.dimension_text_height_nm, 1_500_000);
        assert_eq!(s.max_items, 9);
        let s = Ipc2581Settings::parse("d356_include_smd = false\nd356_unconnected_name = NONET\n")
            .unwrap();
        assert!(!s.d356_include_smd);
        assert_eq!(s.d356_unconnected_name, "NONET");
    }

    #[test]
    fn the_limits_of_the_zip_reader_are_settings_and_name_their_key() {
        let s = Ipc2581Settings::parse(
            "zip_max_total_bytes = 5000
zip_max_entries = 3
zip_max_ratio = 50
",
        )
        .unwrap();
        assert_eq!(
            (s.zip_max_total_bytes, s.zip_max_entries, s.zip_max_ratio),
            (5000, 3, 50)
        );
        for line in [
            "zip_max_total_bytes = 0",
            "zip_max_entries = 100001",
            "zip_max_ratio = 0",
        ] {
            let e = Ipc2581Settings::parse(line).unwrap_err();
            assert!(e.detail.contains(line.split(' ').next().unwrap()), "{line}");
        }
    }

    #[test]
    fn the_limits_of_the_reader_are_settings_and_name_their_key() {
        let s = Ipc2581Settings::parse(
            "read_max_bytes = 1000\nread_max_depth = 10\nread_max_elements = 99\nread_max_items = 7\nread_keep_geometry = false\n",
        )
        .unwrap();
        assert_eq!(
            (
                s.read_max_bytes,
                s.read_max_depth,
                s.read_max_elements,
                s.read_max_items
            ),
            (1000, 10, 99, 7)
        );
        assert!(!s.read_keep_geometry);
        let bad = Ipc2581Settings::parse("read_max_depth = 2\n").unwrap_err();
        assert!(bad.detail.contains("read_max_depth"), "{}", bad.detail);
        let bad = Ipc2581Settings::parse("read_max_items = 0\n").unwrap_err();
        assert!(bad.detail.contains("read_max_items"), "{}", bad.detail);
        let s = Ipc2581Settings::parse(
            "read_max_attrs = 8\nread_max_findings = 3\nread_reference_mhz = 2500\n",
        )
        .unwrap();
        assert_eq!(
            (s.read_max_attrs, s.read_max_findings, s.read_reference_mhz),
            (8, 3, 2500)
        );
        let bad = Ipc2581Settings::parse("read_reference_mhz = 0\n").unwrap_err();
        assert!(bad.detail.contains("read_reference_mhz"), "{}", bad.detail);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = Ipc2581Settings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"), "{}", unknown.detail);
        for text in [
            "origination = yesterday\n",
            "origination = 2000-02-03 04:05:06\n",
            "origination = 2000-02-03T04:05:0x\n",
            "include_bom = 1\n",
            "max_items = 0\n",
            "max_items = x\n",
            "include_zones\n",
            "include_dimensions = yes\n",
            "include_dimension_texts = 1\n",
            "dimension_text_height_nm = -1\n",
            "dimension_text_height_nm = 1000000001\n",
            "dimension_text_height_nm = tall\n",
            "d356_include_smd = 1\n",
            "d356_unconnected_name =\n",
            "d356_unconnected_name = THE_NAME_IS_TOO_LONG\n",
            "d356_unconnected_name = caf\u{e9}\n",
            "read_max_bytes = 0\n",
            "read_max_bytes = 2000000001\n",
            "read_max_depth = 3\n",
            "read_max_depth = 1001\n",
            "read_max_elements = 0\n",
            "read_max_items = x\n",
            "read_keep_geometry = 1\n",
            "read_max_attrs = 0\n",
            "read_max_attrs = 10001\n",
            "read_max_findings = 0\n",
            "read_reference_mhz = 0\n",
            "read_reference_mhz = 1000001\n",
        ] {
            assert!(Ipc2581Settings::parse(text).is_err(), "{text}");
        }
    }
}
