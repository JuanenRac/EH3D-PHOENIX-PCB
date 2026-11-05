// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of Excellon drill files.
//!
//! Read: the units (`METRIC`, `INCH`, `M71`, `M72`), the zeros that are left out (`LZ`, `TZ`) and
//! the digits of the coordinates (`FILE_FORMAT=i:d` comments or a `000.000` pattern), the tools
//! (`T01C0.8`), holes, slots (`G85`) and routed paths (`G00`, `M15`, `G01`, `M16`) as slots, and
//! the `TF.FileFunction` attribute written in a comment. Skipped with a finding: arcs and repeats.

use std::collections::{BTreeMap, BTreeSet};

use crate::codes::{
    Finding, GbrError, Severity, DRILL_SYNTAX, DRILL_TOOL_INVALID, FEATURE_SKIPPED, FORMAT_INVALID,
    INPUT_TOO_LARGE, LIMIT_EXCEEDED, NO_END,
};
use crate::model::{DrillFile, Hit, Slot, Units, P};
use crate::parse::words;
use crate::settings::{ImportSettings, ZeroOmission};
use crate::units::{decimal_to_nm, integer_to_nm, split_sign};

/// The most findings that one file keeps.
const MAX_FINDINGS: usize = 1000;

struct Reader<'a> {
    settings: &'a ImportSettings,
    units: Option<Units>,
    omission: Option<ZeroOmission>,
    digits: Option<(u32, u32)>,
    tools: BTreeMap<u32, i64>,
    current: Option<u32>,
    at: P,
    route: bool,
    pen_down: bool,
    hits: Vec<Hit>,
    slots: Vec<Slot>,
    attributes: BTreeMap<String, Vec<String>>,
    findings: Vec<Finding>,
    noted: BTreeSet<&'static str>,
    ended: bool,
}

fn syntax(line: usize, detail: impl Into<String>) -> GbrError {
    GbrError::at(DRILL_SYNTAX, line, detail)
}

impl Reader<'_> {
    fn note(&mut self, code: &'static str, severity: Severity, line: usize, detail: &str) {
        if self.findings.len() < MAX_FINDINGS {
            self.findings
                .push(Finding::new(code, severity, line, detail));
        }
    }

    fn note_once(
        &mut self,
        key: &'static str,
        code: &'static str,
        severity: Severity,
        line: usize,
        detail: &str,
    ) {
        if self.noted.insert(key) {
            self.note(code, severity, line, detail);
        }
    }

    /// The units; a file that never said gets millimetres, with a finding.
    fn unit(&mut self, line: usize) -> Units {
        match self.units {
            Some(units) => units,
            None => {
                self.note_once(
                    "UNITS",
                    FORMAT_INVALID,
                    Severity::Warning,
                    line,
                    "the units are not stated; millimetres are assumed",
                );
                self.units = Some(Units::Millimetres);
                Units::Millimetres
            }
        }
    }

    fn check_range(&self, value: i64, line: usize) -> Result<i64, GbrError> {
        if value.abs() > self.settings.coordinate_limit_nm {
            Err(GbrError::at(
                crate::codes::COORDINATE_RANGE,
                line,
                format!("{value} nm is beyond the limit"),
            ))
        } else {
            Ok(value)
        }
    }

    fn coordinate(&mut self, raw: &str, line: usize) -> Result<i64, GbrError> {
        let units = self.unit(line);
        let value = if raw.contains('.') {
            decimal_to_nm(raw, units.nm())
        } else {
            let (integer, decimal) = match self.digits {
                Some(pair) => pair,
                None => {
                    self.note_once(
                        "FORMAT",
                        FORMAT_INVALID,
                        Severity::Warning,
                        line,
                        "the number of digits is not stated; the default of the settings is used",
                    );
                    match units {
                        Units::Inches => (
                            self.settings.excellon_inch_integer_digits,
                            self.settings.excellon_inch_decimal_digits,
                        ),
                        Units::Millimetres => (
                            self.settings.excellon_metric_integer_digits,
                            self.settings.excellon_metric_decimal_digits,
                        ),
                    }
                }
            };
            let omission = self.omission.unwrap_or(self.settings.excellon_omission);
            let (negative, digits) = split_sign(raw);
            integer_to_nm(digits, negative, integer, decimal, omission, units.nm())
        };
        let value = value.ok_or_else(|| syntax(line, format!("'{raw}' is not a coordinate")))?;
        self.check_range(value, line)
    }

    fn comment(&mut self, text: &str) {
        let text = text.trim();
        if let Some(index) = text.find("TF.") {
            let attribute = &text[index + 2..];
            let mut values = attribute.split(',');
            let name = values.next().unwrap_or("").trim().to_owned();
            if !name.is_empty() {
                let rest: Vec<String> = values.map(|v| v.trim().to_owned()).collect();
                self.attributes.insert(name, rest);
            }
        } else if let Some(format) = text.strip_prefix("FILE_FORMAT=") {
            if let Some((integer, decimal)) = format.split_once(':') {
                if let (Ok(i), Ok(d)) =
                    (integer.trim().parse::<u32>(), decimal.trim().parse::<u32>())
                {
                    if (1..=8).contains(&i) && (1..=8).contains(&d) {
                        self.digits = Some((i, d));
                    }
                }
            }
        } else if let Some(kind) = text.strip_prefix("TYPE=") {
            let value = match kind.trim() {
                "PLATED" => Some("Plated"),
                "NON_PLATED" => Some("NonPlated"),
                _ => None,
            };
            if let Some(value) = value {
                self.attributes
                    .entry(".FileFunction".to_owned())
                    .or_insert_with(|| vec![value.to_owned()]);
            }
        }
    }

    /// `METRIC,TZ`, `INCH,LZ,00.0000` and the like.
    fn unit_line(&mut self, text: &str, line: usize) -> Result<(), GbrError> {
        let mut parts = text.split(',');
        self.units = Some(match parts.next().unwrap_or("").trim() {
            "METRIC" => Units::Millimetres,
            _ => Units::Inches,
        });
        for part in parts {
            let part = part.trim();
            match part {
                "LZ" => self.omission = Some(ZeroOmission::Trailing),
                "TZ" => self.omission = Some(ZeroOmission::Leading),
                "" => {}
                pattern => {
                    let Some((integer, decimal)) = pattern.split_once('.') else {
                        return Err(syntax(
                            line,
                            format!("'{pattern}' is not LZ, TZ or a format"),
                        ));
                    };
                    let digits = |s: &str| s.bytes().all(|b| b == b'0') && !s.is_empty();
                    if !digits(integer) || !digits(decimal) {
                        return Err(syntax(line, format!("'{pattern}' is not a format")));
                    }
                    let (i, d) = (integer.len() as u32, decimal.len() as u32);
                    if i > 8 || d > 8 {
                        return Err(syntax(line, format!("'{pattern}' has too many digits")));
                    }
                    self.digits = Some((i, d));
                }
            }
        }
        Ok(())
    }

    fn tool_number(value: &str, line: usize) -> Result<u32, GbrError> {
        value
            .parse()
            .map_err(|_| GbrError::at(DRILL_TOOL_INVALID, line, "a tool without a number"))
    }

    fn read_line(
        &mut self,
        text: &str,
        line: usize,
        in_header: &mut bool,
    ) -> Result<bool, GbrError> {
        if let Some(comment) = text.strip_prefix(';') {
            self.comment(comment);
            return Ok(false);
        }
        match text {
            "M48" => {
                *in_header = true;
                return Ok(false);
            }
            "%" | "M95" => {
                *in_header = false;
                return Ok(false);
            }
            "M30" | "M00" => {
                self.ended = true;
                return Ok(true);
            }
            "M71" => {
                self.units = Some(Units::Millimetres);
                return Ok(false);
            }
            "M72" => {
                self.units = Some(Units::Inches);
                return Ok(false);
            }
            "ICI,OFF" | "G90" | "G05" | "G93" | "G40" | "FMAT,2" | "VER,1" | "VER,2" => {
                if text == "G05" {
                    self.route = false;
                }
                return Ok(false);
            }
            "ICI,ON" | "G91" => {
                return Err(GbrError::at(
                    FORMAT_INVALID,
                    line,
                    "incremental coordinates are not read",
                ))
            }
            _ => {}
        }
        if text.starts_with("METRIC") || text.starts_with("INCH") {
            return self.unit_line(text, line).map(|()| false);
        }
        if text.starts_with("M47") || text.starts_with("DETECT") || text.starts_with("FMAT") {
            return Ok(false);
        }
        let list = words(text).ok_or_else(|| syntax(line, format!("cannot read '{text}'")))?;
        if list.is_empty() {
            return Ok(false);
        }
        if list[0].0 == 'T' {
            self.tool_line(&list, line, *in_header)?;
            return Ok(false);
        }
        self.body_line(&list, line)?;
        Ok(false)
    }

    fn tool_line(
        &mut self,
        list: &[(char, &str)],
        line: usize,
        in_header: bool,
    ) -> Result<(), GbrError> {
        let number = Self::tool_number(list[0].1, line)?;
        let diameter = list.iter().find(|(l, _)| *l == 'C').map(|(_, v)| *v);
        if let Some(text) = diameter {
            let units = self.unit(line);
            let value = decimal_to_nm(text, units.nm())
                .filter(|v| *v > 0)
                .ok_or_else(|| {
                    GbrError::at(
                        DRILL_TOOL_INVALID,
                        line,
                        format!("T{number} has the diameter '{text}'"),
                    )
                })?;
            self.check_range(value, line)?;
            if self.tools.len() >= self.settings.max_apertures && !self.tools.contains_key(&number)
            {
                return Err(GbrError::at(
                    LIMIT_EXCEEDED,
                    line,
                    format!("more than {} tools", self.settings.max_apertures),
                ));
            }
            self.tools.insert(number, value);
            if !in_header {
                self.current = Some(number);
            }
            return Ok(());
        }
        if number == 0 {
            self.current = None;
            return Ok(());
        }
        if !self.tools.contains_key(&number) {
            return Err(GbrError::at(
                DRILL_TOOL_INVALID,
                line,
                format!("T{number} is used before it is defined"),
            ));
        }
        self.current = Some(number);
        Ok(())
    }

    fn push_check(&self, line: usize) -> Result<(), GbrError> {
        if self.hits.len() + self.slots.len() >= self.settings.max_objects {
            Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!("more than {} holes", self.settings.max_objects),
            ))
        } else {
            Ok(())
        }
    }

    fn body_line(&mut self, list: &[(char, &str)], line: usize) -> Result<(), GbrError> {
        let (mut x1, mut y1, mut x2, mut y2): (
            Option<&str>,
            Option<&str>,
            Option<&str>,
            Option<&str>,
        ) = (None, None, None, None);
        let mut slot_marker = false;
        let mut linear = false;
        let mut arc = false;
        let mut repeat = false;
        for (letter, value) in list {
            match letter {
                'G' => match Self::tool_number(value, line)
                    .map_err(|_| syntax(line, "G without a number"))?
                {
                    0 => {
                        self.route = true;
                        self.pen_down = false;
                    }
                    1 => {
                        self.route = true;
                        linear = true;
                    }
                    2 | 3 => {
                        arc = true;
                        self.route = true;
                        self.note_once(
                            "ARC",
                            FEATURE_SKIPPED,
                            Severity::Warning,
                            line,
                            "arcs in a route (G02, G03) are not read",
                        );
                    }
                    5 => self.route = false,
                    85 => slot_marker = true,
                    90 | 93 => {}
                    91 => {
                        return Err(GbrError::at(
                            FORMAT_INVALID,
                            line,
                            "incremental coordinates (G91) are not read",
                        ))
                    }
                    other => return Err(syntax(line, format!("G{other} is not known"))),
                },
                'M' => match Self::tool_number(value, line)
                    .map_err(|_| syntax(line, "M without a number"))?
                {
                    15 => self.pen_down = true,
                    16 => self.pen_down = false,
                    30 | 0 => self.ended = true,
                    71 => self.units = Some(Units::Millimetres),
                    72 => self.units = Some(Units::Inches),
                    _ => self.note_once(
                        "MCODE",
                        FEATURE_SKIPPED,
                        Severity::Info,
                        line,
                        "an M code that has no effect on the holes was ignored",
                    ),
                },
                'X' => {
                    if slot_marker {
                        x2 = Some(*value);
                    } else {
                        x1 = Some(*value);
                    }
                }
                'Y' => {
                    if slot_marker {
                        y2 = Some(*value);
                    } else {
                        y1 = Some(*value);
                    }
                }
                'R' => {
                    repeat = true;
                    self.note_once(
                        "REPEAT",
                        FEATURE_SKIPPED,
                        Severity::Warning,
                        line,
                        "repeated holes (R) are not read",
                    );
                }
                'C' | 'F' | 'S' | 'B' | 'H' | 'Z' | 'I' | 'J' | 'A' | 'N' => {}
                other => return Err(syntax(line, format!("the word '{other}' is not known"))),
            }
        }
        if self.ended || repeat || (x1.is_none() && y1.is_none() && x2.is_none() && y2.is_none()) {
            return Ok(());
        }
        let first = P {
            x: match x1 {
                Some(text) => self.coordinate(text, line)?,
                None => self.at.x,
            },
            y: match y1 {
                Some(text) => self.coordinate(text, line)?,
                None => self.at.y,
            },
        };
        if slot_marker {
            let tool = self.current.ok_or_else(|| {
                GbrError::at(DRILL_TOOL_INVALID, line, "a slot is drilled with no tool")
            })?;
            let end = P {
                x: match x2 {
                    Some(text) => self.coordinate(text, line)?,
                    None => first.x,
                },
                y: match y2 {
                    Some(text) => self.coordinate(text, line)?,
                    None => first.y,
                },
            };
            self.push_check(line)?;
            self.slots.push(Slot {
                tool,
                from: first,
                to: end,
            });
            self.at = end;
            return Ok(());
        }
        if self.route {
            if self.pen_down && !arc && (linear || x1.is_some() || y1.is_some()) {
                let tool = self.current.ok_or_else(|| {
                    GbrError::at(DRILL_TOOL_INVALID, line, "a route is cut with no tool")
                })?;
                self.push_check(line)?;
                self.slots.push(Slot {
                    tool,
                    from: self.at,
                    to: first,
                });
            }
            self.at = first;
            return Ok(());
        }
        let tool = self.current.ok_or_else(|| {
            GbrError::at(DRILL_TOOL_INVALID, line, "a hole is drilled with no tool")
        })?;
        self.push_check(line)?;
        self.hits.push(Hit { tool, at: first });
        self.at = first;
        Ok(())
    }
}

/// Reads an Excellon file. An error is a file that cannot be read at all; what is read but
/// doubtful is a finding of the result.
pub fn parse_drill(text: &str, settings: &ImportSettings) -> Result<DrillFile, GbrError> {
    if text.len() > settings.max_file_bytes {
        return Err(GbrError::new(
            INPUT_TOO_LARGE,
            format!(
                "{} bytes, the limit is {}",
                text.len(),
                settings.max_file_bytes
            ),
        ));
    }
    let mut reader = Reader {
        settings,
        units: None,
        omission: None,
        digits: None,
        tools: BTreeMap::new(),
        current: None,
        at: P { x: 0, y: 0 },
        route: false,
        pen_down: false,
        hits: Vec::new(),
        slots: Vec::new(),
        attributes: BTreeMap::new(),
        findings: Vec::new(),
        noted: BTreeSet::new(),
        ended: false,
    };
    let mut in_header = false;
    let mut seen = 0usize;
    let mut last_line = 0usize;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        seen += 1;
        if seen > settings.max_commands {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!("more than {} lines", settings.max_commands),
            ));
        }
        last_line = line;
        if reader.read_line(trimmed, line, &mut in_header)? {
            break;
        }
    }
    if seen == 0 {
        return Err(syntax(0, "the file has no line"));
    }
    if !reader.ended {
        reader.note(NO_END, Severity::Warning, last_line, "no M30");
    }
    let omission = reader.omission.unwrap_or(settings.excellon_omission);
    Ok(DrillFile {
        units: reader.units.unwrap_or(Units::Millimetres),
        tools: reader.tools,
        hits: reader.hits,
        slots: reader.slots,
        attributes: reader.attributes,
        findings: reader.findings,
        ended: reader.ended,
        omission,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> DrillFile {
        parse_drill(text, &ImportSettings::default()).unwrap()
    }

    fn fail(text: &str) -> GbrError {
        parse_drill(text, &ImportSettings::default()).unwrap_err()
    }

    #[test]
    fn tools_holes_slots_and_routes_of_the_writer_of_the_suite() {
        let file = read(
            "M48\n; #@! TF.FileFunction,Plated,1,2,PTH\n;FILE_FORMAT=3:3\nMETRIC,TZ\nT01C0.8F200S65\nT02C1.2\nT03C2\n%\nT01\nX1.5Y2.25\nX10.0Y-3.125\nT02\nX5.0Y5.0\nX12.0Y6.0G85X14.0Y6.0\nT03\nG00X20.0Y20.0\nM15\nG01X30.0Y20.0\nG01X30.0Y30.0\nM16\nM30\n",
        );
        assert!(file.ended);
        assert_eq!(file.findings, Vec::new());
        assert_eq!(file.units, Units::Millimetres);
        assert_eq!(
            file.tools.values().copied().collect::<Vec<_>>(),
            vec![800_000, 1_200_000, 2_000_000]
        );
        assert_eq!(
            file.attributes[".FileFunction"],
            vec!["Plated", "1", "2", "PTH"]
        );
        assert_eq!(file.hits.len(), 3);
        assert_eq!(
            file.hits[1],
            Hit {
                tool: 1,
                at: P {
                    x: 10_000_000,
                    y: -3_125_000
                }
            }
        );
        assert_eq!(file.slots.len(), 3);
        assert_eq!(
            file.slots[0],
            Slot {
                tool: 2,
                from: P {
                    x: 12_000_000,
                    y: 6_000_000
                },
                to: P {
                    x: 14_000_000,
                    y: 6_000_000
                }
            }
        );
        // the route is two slots of the third tool
        assert_eq!(
            file.slots[1].from,
            P {
                x: 20_000_000,
                y: 20_000_000
            }
        );
        assert_eq!(
            file.slots[2].to,
            P {
                x: 30_000_000,
                y: 30_000_000
            }
        );
        assert_eq!(file.diameter(3), 2_000_000);
        assert_eq!(file.diameter(9), 0);
        assert_eq!(
            file.centre_bounds(),
            Some((
                P {
                    x: 1_500_000,
                    y: -3_125_000
                },
                P {
                    x: 30_000_000,
                    y: 30_000_000
                }
            ))
        );
    }

    #[test]
    fn integer_coordinates_follow_the_zeros_and_the_format() {
        // leading zeros left out (TZ), 3.3 in millimetres
        let leading = read("M48\nMETRIC,TZ\nT1C0.5\n%\nT1\nX1500Y-20\nM30\n");
        assert_eq!(
            leading.hits[0].at,
            P {
                x: 1_500_000,
                y: -20_000
            }
        );
        // trailing zeros left out (LZ): 15 is 150.000 and 2 is 200.000 in the format 3.3
        let trailing = read("M48\nMETRIC,LZ\nT1C0.5\n%\nT1\nX15Y2\nM30\n");
        assert_eq!(
            trailing.hits[0].at,
            P {
                x: 150_000_000,
                y: 200_000_000
            }
        );
        // inches with a stated pattern
        let inches = read("M48\nINCH,TZ,00.0000\nT1C0.02\n%\nT1\nX10000Y5000\nM30\n");
        assert_eq!(
            inches.hits[0].at,
            P {
                x: 25_400_000,
                y: 12_700_000
            }
        );
        assert_eq!(inches.tools[&1], 508_000);
        // a format in a comment
        let commented = read("M48\n;FILE_FORMAT=2:5\nMETRIC,TZ\nT1C0.5\n%\nT1\nX100000\nM30\n");
        assert_eq!(commented.hits[0].at.x, 1_000_000);
        // M71 and M72
        let old = read("M48\nM72\nT1C0.02\n%\nT1\nX1.0Y1.0\nM71\nX1.0Y1.0\nM30\n");
        assert_eq!(old.hits[0].at.x, 25_400_000);
        assert_eq!(old.hits[1].at.x, 1_000_000);
    }

    #[test]
    fn a_file_that_does_not_say_gets_findings_and_the_defaults() {
        let file = read("T1C0.5\nT1\nX1500Y200\nM30\n");
        assert_eq!(file.units, Units::Millimetres);
        assert!(file
            .findings
            .iter()
            .any(|f| f.code == FORMAT_INVALID && f.detail.contains("units")));
        assert!(file
            .findings
            .iter()
            .any(|f| f.code == FORMAT_INVALID && f.detail.contains("digits")));
        assert_eq!(
            file.hits[0].at,
            P {
                x: 1_500_000,
                y: 200_000
            }
        );
        let trailing = ImportSettings {
            excellon_omission: ZeroOmission::Trailing,
            ..ImportSettings::default()
        };
        let other = parse_drill("T1C0.5\nT1\nX15\nM30\n", &trailing).unwrap();
        assert_eq!(other.hits[0].at.x, 150_000_000);
        assert_eq!(other.omission, ZeroOmission::Trailing);
    }

    #[test]
    fn what_cannot_be_read_has_a_code() {
        for (text, code) in [
            ("", DRILL_SYNTAX),
            (
                "M48\nMETRIC\nT1C0.5\n%\nT1\nX1.0Y1.0\nQ5\nM30\n",
                DRILL_SYNTAX,
            ),
            ("M48\nMETRIC,XY\n%\nM30\n", DRILL_SYNTAX),
            ("M48\nMETRIC,0.0.0\n%\nM30\n", DRILL_SYNTAX),
            ("M48\nMETRIC\nT1C0\n%\nM30\n", DRILL_TOOL_INVALID),
            (
                "M48\nMETRIC\nT1C0.5\n%\nT2\nX1.0Y1.0\nM30\n",
                DRILL_TOOL_INVALID,
            ),
            (
                "M48\nMETRIC\nT1C0.5\n%\nX1.0Y1.0\nM30\n",
                DRILL_TOOL_INVALID,
            ),
            ("M48\nMETRIC\n%\nG91\nM30\n", FORMAT_INVALID),
            ("M48\nICI,ON\n%\nM30\n", FORMAT_INVALID),
            ("M48\nMETRIC\nT1C0.5\n%\nT1\nXabc\nM30\n", DRILL_SYNTAX),
            ("M48\nMETRIC\nT1C0.5\n%\nT1\nG99\nM30\n", DRILL_SYNTAX),
            (
                "M48\nMETRIC\nT1C0.5\n%\nT1\nX99999999999.0\nM30\n",
                crate::codes::COORDINATE_RANGE,
            ),
        ] {
            let e = parse_drill(text, &ImportSettings::default()).unwrap_err();
            assert_eq!(e.code, code, "{text:?}: {e}");
        }
        assert_eq!(fail("M48\nMETRIC\nT1C0.5\n%\nT1\nX1.0\nQ\n").line, 7);
        // a slot with a tool is fine
        let slot = "M48\nMETRIC\nT1C0.5\n%\nT1\nX1.0Y1.0G85X2.0Y1.0\nM30\n";
        assert_eq!(read(slot).slots.len(), 1);
    }

    #[test]
    fn arcs_and_repeats_are_skipped_with_findings_and_a_missing_end_is_noted() {
        let file = read(
            "M48\nMETRIC\nT1C0.5\n%\nT1\nG00X0.0Y0.0\nM15\nG02X1.0Y1.0I0.5J0\nM16\nR3X1.0Y0.0\n",
        );
        assert!(!file.ended);
        assert_eq!((file.hits.len(), file.slots.len()), (0, 0));
        let codes: Vec<&str> = file.findings.iter().map(|f| f.code).collect();
        assert_eq!(codes, vec![FEATURE_SKIPPED, FEATURE_SKIPPED, NO_END]);
    }

    #[test]
    fn the_limits_of_the_settings_are_enforced() {
        let text = "M48\nMETRIC\nT1C0.5\nT2C0.6\n%\nT1\nX1.0Y1.0\nX2.0Y2.0\nM30\n";
        let holes = ImportSettings {
            max_objects: 1,
            ..ImportSettings::default()
        };
        assert_eq!(parse_drill(text, &holes).unwrap_err().code, LIMIT_EXCEEDED);
        let tools = ImportSettings {
            max_apertures: 1,
            ..ImportSettings::default()
        };
        assert_eq!(parse_drill(text, &tools).unwrap_err().code, LIMIT_EXCEEDED);
        let lines = ImportSettings {
            max_commands: 4,
            ..ImportSettings::default()
        };
        assert_eq!(parse_drill(text, &lines).unwrap_err().code, LIMIT_EXCEEDED);
        let bytes = ImportSettings {
            max_file_bytes: 8,
            ..ImportSettings::default()
        };
        assert_eq!(parse_drill(text, &bytes).unwrap_err().code, INPUT_TOO_LARGE);
    }
}
