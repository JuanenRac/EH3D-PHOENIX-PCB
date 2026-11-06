// SPDX-License-Identifier: GPL-3.0-or-later
//! A mask (a template) for the response of a two-port, and the check of a response against it
//! with a report of what passes and what fails.
//!
//! A mask is a text with one segment per line:
//!
//! ```text
//! # parameter  min|max  start  stop  limit_db  [limit_db_at_stop]
//! s21 min 0 100M -3
//! s21 max 300M 1G -40 -60
//! s11 max 0 100M -10
//! ```
//!
//! The parameter is `s11`, `s12`, `s21` or `s22`; the limit is on the magnitude in dB; `min`
//! means the magnitude has to be at least the limit and `max` at most. With two limits the limit
//! goes in a straight line over the frequency from the first to the second. Frequencies take the
//! prefixes `k`, `M` (or `meg`) and `G`. A segment with no data point inside it fails: a
//! response that does not cover the mask has not passed.

use crate::error::RfError;
use crate::netdesc::parse_si_text;
use crate::network::Mat2;
use crate::settings::RfSettings;

/// The S parameter a segment limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskParam {
    /// Input reflection.
    S11,
    /// Reverse transmission.
    S12,
    /// Forward transmission.
    S21,
    /// Output reflection.
    S22,
}

impl MaskParam {
    /// The name in lower case.
    pub fn name(self) -> &'static str {
        match self {
            MaskParam::S11 => "s11",
            MaskParam::S12 => "s12",
            MaskParam::S21 => "s21",
            MaskParam::S22 => "s22",
        }
    }

    fn value_db(self, s: &Mat2) -> f64 {
        match self {
            MaskParam::S11 => s.m11.db20(),
            MaskParam::S12 => s.m12.db20(),
            MaskParam::S21 => s.m21.db20(),
            MaskParam::S22 => s.m22.db20(),
        }
    }
}

/// Whether a segment is a floor or a ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskLimit {
    /// The magnitude must be at least the limit.
    Min,
    /// The magnitude must be at most the limit.
    Max,
}

impl MaskLimit {
    /// The word used in the text.
    pub fn name(self) -> &'static str {
        match self {
            MaskLimit::Min => "min",
            MaskLimit::Max => "max",
        }
    }
}

/// One segment of a mask.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    /// The line of the text it came from, counted from 1.
    pub line: usize,
    /// The parameter it limits.
    pub param: MaskParam,
    /// Floor or ceiling.
    pub limit: MaskLimit,
    /// The first frequency in Hz.
    pub start_hz: f64,
    /// The last frequency in Hz.
    pub stop_hz: f64,
    /// The limit in dB at the first frequency.
    pub start_db: f64,
    /// The limit in dB at the last frequency.
    pub stop_db: f64,
}

impl Segment {
    /// The limit in dB at a frequency inside the segment.
    pub fn limit_at(&self, freq_hz: f64) -> f64 {
        if self.stop_hz <= self.start_hz {
            return self.start_db;
        }
        let t = (freq_hz - self.start_hz) / (self.stop_hz - self.start_hz);
        self.start_db + (self.stop_db - self.start_db) * t
    }
}

/// A mask: its segments in the order of the text.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    /// The segments.
    pub segments: Vec<Segment>,
}

fn syntax(line: usize, reason: impl Into<String>) -> RfError {
    RfError::MaskSyntax {
        line,
        reason: reason.into(),
    }
}

impl Mask {
    /// Read a mask. Every error names the line.
    pub fn parse(text: &str, settings: &RfSettings) -> Result<Mask, RfError> {
        if text.len() > settings.max_net_bytes {
            return Err(RfError::OutsideLimit {
                field: "size of the mask".to_string(),
                value: text.len() as f64,
                limit: settings.max_net_bytes as f64,
            });
        }
        let mut segments: Vec<Segment> = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let content = raw.split(['#', '!']).next().unwrap_or("").trim();
            if content.is_empty() {
                continue;
            }
            let words: Vec<&str> = content.split_whitespace().collect();
            if words.len() != 5 && words.len() != 6 {
                return Err(syntax(
                    line,
                    "expected: parameter min|max start stop limit [limit at stop]",
                ));
            }
            let param = match words[0].to_ascii_lowercase().as_str() {
                "s11" => MaskParam::S11,
                "s12" => MaskParam::S12,
                "s21" => MaskParam::S21,
                "s22" => MaskParam::S22,
                _ => return Err(syntax(line, "the parameter is s11, s12, s21 or s22")),
            };
            let limit = match words[1].to_ascii_lowercase().as_str() {
                "min" => MaskLimit::Min,
                "max" => MaskLimit::Max,
                _ => return Err(syntax(line, "the second word is min or max")),
            };
            let number = |text: &str| parse_si_text(text).map_err(|reason| syntax(line, reason));
            let start_hz = number(words[2])?;
            let stop_hz = number(words[3])?;
            let start_db = number(words[4])?;
            let stop_db = match words.get(5) {
                Some(word) => number(*word)?,
                None => start_db,
            };
            if start_hz < 0.0 || stop_hz < start_hz {
                return Err(syntax(
                    line,
                    "the start frequency must be zero or more and not above the stop frequency",
                ));
            }
            if segments.len() >= settings.max_mask_segments {
                return Err(RfError::OutsideLimit {
                    field: "segments of the mask".to_string(),
                    value: (segments.len() + 1) as f64,
                    limit: settings.max_mask_segments as f64,
                });
            }
            segments.push(Segment {
                line,
                param,
                limit,
                start_hz,
                stop_hz,
                start_db,
                stop_db,
            });
        }
        if segments.is_empty() {
            return Err(syntax(0, "the mask has no segments"));
        }
        Ok(Mask { segments })
    }
}

/// What the check found for one segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentResult {
    /// The segment.
    pub segment: Segment,
    /// How many points of the response lie inside the segment.
    pub points: usize,
    /// The smallest margin in dB: the distance to the limit, positive when inside it. Infinite
    /// when no point is inside the segment or the response is zero.
    pub worst_margin_db: f64,
    /// The frequency in Hz of that point.
    pub worst_freq_hz: f64,
    /// Whether every point is inside the limit (with the tolerance of the settings) and there is
    /// at least one point.
    pub passed: bool,
}

/// The report of a check.
#[derive(Clone, Debug, PartialEq)]
pub struct MaskReport {
    /// One result per segment, in the order of the mask.
    pub results: Vec<SegmentResult>,
    /// Whether every segment passed.
    pub passed: bool,
}

/// Check a response against a mask. `data` is the S matrix at each frequency in Hz.
pub fn check_mask(mask: &Mask, data: &[(f64, Mat2)], settings: &RfSettings) -> MaskReport {
    let tolerance = settings.mask_margin_tolerance_db;
    let results: Vec<SegmentResult> = mask
        .segments
        .iter()
        .map(|segment| {
            let mut points = 0usize;
            let mut worst = f64::INFINITY;
            let mut worst_freq = segment.start_hz;
            for (freq, s) in data {
                if !(segment.start_hz..=segment.stop_hz).contains(freq) {
                    continue;
                }
                points += 1;
                let value = segment.param.value_db(s);
                let limit = segment.limit_at(*freq);
                let margin = match segment.limit {
                    MaskLimit::Max => limit - value,
                    MaskLimit::Min => value - limit,
                };
                // an unordered margin (a NaN) counts as the worst there is
                if margin.is_nan() || margin < worst {
                    worst = if margin.is_nan() {
                        f64::NEG_INFINITY
                    } else {
                        margin
                    };
                    worst_freq = *freq;
                }
            }
            SegmentResult {
                segment: *segment,
                points,
                worst_margin_db: worst,
                worst_freq_hz: worst_freq,
                passed: points > 0 && worst >= -tolerance,
            }
        })
        .collect();
    let passed = results.iter().all(|r| r.passed);
    MaskReport { results, passed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complex::Complex;

    fn settings() -> RfSettings {
        RfSettings::default()
    }

    fn s(s11_db: f64, s21_db: f64) -> Mat2 {
        Mat2::new(
            Complex::from_db_deg(s11_db, 0.0),
            Complex::ZERO,
            Complex::from_db_deg(s21_db, 0.0),
            Complex::ZERO,
        )
    }

    #[test]
    fn a_mask_is_read_with_its_segments() {
        let text = "# low-pass template\ns21 min 0 100M -3\n\
                    S21 MAX 300M 1G -40 -60 ! stop band\ns11 max 0 100meg -10\n";
        let mask = Mask::parse(text, &settings()).unwrap();
        assert_eq!(mask.segments.len(), 3);
        let a = mask.segments[0];
        assert_eq!(
            (a.param, a.limit, a.line),
            (MaskParam::S21, MaskLimit::Min, 2)
        );
        assert_eq!(
            (a.start_hz, a.stop_hz, a.start_db, a.stop_db),
            (0.0, 1e8, -3.0, -3.0)
        );
        let b = mask.segments[1];
        assert_eq!(
            (b.start_hz, b.stop_hz, b.start_db, b.stop_db),
            (3e8, 1e9, -40.0, -60.0)
        );
        assert_eq!(mask.segments[2].stop_hz, 1e8);
        // the limit goes in a straight line over the segment
        assert!((b.limit_at(3e8) + 40.0).abs() < 1e-12);
        assert!((b.limit_at(1e9) + 60.0).abs() < 1e-12);
        assert!((b.limit_at(6.5e8) + 50.0).abs() < 1e-9);
    }

    #[test]
    fn errors_name_the_line() {
        let cases = [
            ("s21 min 0 100M", 1),
            ("s31 min 0 100M -3", 1),
            ("s21 low 0 100M -3", 1),
            ("\ns21 min zero 100M -3", 2),
            ("s21 min 0 100M x", 1),
            ("s21 min 200M 100M -3", 1),
            ("s21 min -1 100M -3", 1),
            ("s21 min 0 100M -3 -4 -5", 1),
            ("s21 min 0 100M -3\ns11 max 5", 2),
        ];
        for (text, line) in cases {
            match Mask::parse(text, &settings()) {
                Err(RfError::MaskSyntax { line: got, .. }) => assert_eq!(got, line, "{text:?}"),
                other => panic!("{text:?}: {other:?}"),
            }
        }
        assert!(matches!(
            Mask::parse("# only a comment", &settings()),
            Err(RfError::MaskSyntax { line: 0, .. })
        ));
    }

    #[test]
    fn the_limits_of_a_mask_come_from_the_settings() {
        let few = RfSettings {
            max_mask_segments: 1,
            ..settings()
        };
        assert!(matches!(
            Mask::parse("s21 min 0 1 -3\ns21 min 2 3 -3", &few),
            Err(RfError::OutsideLimit { .. })
        ));
        let small = RfSettings {
            max_net_bytes: 8,
            ..settings()
        };
        assert!(matches!(
            Mask::parse("s21 min 0 1 -3", &small),
            Err(RfError::OutsideLimit { .. })
        ));
    }

    #[test]
    fn a_response_inside_the_mask_passes_and_reports_the_worst_margin() {
        let mask = Mask::parse("s21 min 0 100M -3\ns21 max 200M 1G -20\n", &settings()).unwrap();
        let data = vec![
            (50e6, s(-30.0, -0.5)),
            (100e6, s(-20.0, -2.5)),
            (300e6, s(-1.0, -35.0)),
            (900e6, s(-1.0, -50.0)),
        ];
        let report = check_mask(&mask, &data, &settings());
        assert!(report.passed);
        assert_eq!(report.results[0].points, 2);
        assert!((report.results[0].worst_margin_db - 0.5).abs() < 1e-9);
        assert_eq!(report.results[0].worst_freq_hz, 100e6);
        assert_eq!(report.results[1].points, 2);
        assert!((report.results[1].worst_margin_db - 15.0).abs() < 1e-9);
        assert_eq!(report.results[1].worst_freq_hz, 300e6);
    }

    #[test]
    fn a_point_outside_the_limit_fails_and_names_where() {
        let mask = Mask::parse("s21 max 200M 1G -20\ns11 max 0 1G -10", &settings()).unwrap();
        let data = vec![
            (300e6, s(-15.0, -35.0)),
            (600e6, s(-5.0, -15.0)),
            (900e6, s(-12.0, -50.0)),
        ];
        let report = check_mask(&mask, &data, &settings());
        assert!(!report.passed);
        assert!(!report.results[0].passed);
        assert!((report.results[0].worst_margin_db + 5.0).abs() < 1e-9);
        assert_eq!(report.results[0].worst_freq_hz, 600e6);
        assert!(!report.results[1].passed);
        assert_eq!(report.results[1].worst_freq_hz, 600e6);
    }

    #[test]
    fn a_response_on_the_limit_passes() {
        let mask = Mask::parse("s21 min 0 1G -3", &settings()).unwrap();
        let data = vec![(1e8, s(-20.0, -3.0))];
        assert!(check_mask(&mask, &data, &settings()).passed);
    }

    #[test]
    fn a_segment_with_no_data_fails() {
        let mask = Mask::parse("s21 min 0 100M -3\ns21 max 2G 3G -40", &settings()).unwrap();
        let data = vec![(50e6, s(-30.0, -1.0))];
        let report = check_mask(&mask, &data, &settings());
        assert!(!report.passed);
        assert!(report.results[0].passed);
        assert_eq!(report.results[1].points, 0);
        assert!(!report.results[1].passed);
        assert!(report.results[1].worst_margin_db.is_infinite());
        // no data at all
        assert!(!check_mask(&mask, &[], &settings()).passed);
    }

    #[test]
    fn a_zero_response_has_an_infinite_margin_for_a_ceiling() {
        let mask = Mask::parse("s12 max 0 1G -40", &settings()).unwrap();
        let data = vec![(1e8, s(-10.0, -1.0))];
        let report = check_mask(&mask, &data, &settings());
        assert!(report.passed);
        assert!(report.results[0].worst_margin_db.is_infinite());
        let floor = Mask::parse("s12 min 0 1G -40", &settings()).unwrap();
        assert!(!check_mask(&floor, &data, &settings()).passed);
    }
}
