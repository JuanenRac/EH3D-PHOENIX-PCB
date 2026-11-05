// SPDX-License-Identifier: GPL-3.0-or-later
//! What the readers share: the context that collects diagnostics and the counts of the quality
//! report, and the conversion of the numbers of KiCad (millimetres and degrees written as
//! decimals) into integer nanometres and micro-degrees.
//!
//! KiCad's y axis points down and the design model's points up, so every position read from a
//! file goes through [`to_model`], which turns y; angles keep their sign (a positive angle is a
//! counter-clockwise turn on the screen in both).

use std::collections::BTreeMap;

use eh3d_diagnostic::Diagnostic;
use eh3d_phoenix_board::Id;
use eh3d_phoenix_hub::QualityReport;
use eh3d_phoenix_model::{Nm, Point, ScaledDecimal, COORD_LIMIT_NM};

use crate::error::KicadError;
use crate::settings::KicadSettings;
use crate::sexpr::Node;

/// A position in nanometres, as written in the file (y down).
pub(crate) type P = (i64, i64);

/// The state of one reading.
pub(crate) struct Ctx<'a> {
    new_id: &'a mut dyn FnMut() -> Id,
    pub settings: &'a KicadSettings,
    pub diagnostics: Vec<Diagnostic>,
    suppressed: usize,
    pub kept: BTreeMap<String, usize>,
    pub transformed: BTreeMap<String, usize>,
    pub dropped: BTreeMap<String, usize>,
    ignored: BTreeMap<String, usize>,
    simplified: BTreeMap<String, (usize, String)>,
}

impl<'a> Ctx<'a> {
    pub fn new(settings: &'a KicadSettings, new_id: &'a mut dyn FnMut() -> Id) -> Ctx<'a> {
        Ctx {
            new_id,
            settings,
            diagnostics: Vec::new(),
            suppressed: 0,
            kept: BTreeMap::new(),
            transformed: BTreeMap::new(),
            dropped: BTreeMap::new(),
            ignored: BTreeMap::new(),
            simplified: BTreeMap::new(),
        }
    }

    /// A new object id.
    pub fn id(&mut self) -> Id {
        (self.new_id)()
    }

    /// Records a finding, up to the limit of the settings.
    pub fn push(&mut self, error: KicadError) {
        if self.diagnostics.len() < self.settings.max_diagnostics {
            self.diagnostics.push(error.diagnostic());
        } else {
            self.suppressed += 1;
        }
    }

    /// A value that cannot be used, at the place of `node`.
    pub fn invalid(&mut self, node: &Node, what: &str, reason: &str) {
        self.push(KicadError::InvalidValue {
            line: node.line as usize,
            column: node.column as usize,
            what: what.to_owned(),
            reason: reason.to_owned(),
        });
    }

    /// A reference to something the file does not have.
    pub fn missing(&mut self, node: &Node, what: &str) {
        self.push(KicadError::MissingReference {
            what: what.to_owned(),
            line: node.line as usize,
        });
    }

    /// Counts data that came through as it was.
    pub fn keep(&mut self, category: &str, n: usize) {
        *self.kept.entry(category.to_owned()).or_insert(0) += n;
    }

    /// Counts data that came through changed.
    pub fn transform(&mut self, category: &str, n: usize) {
        *self.transformed.entry(category.to_owned()).or_insert(0) += n;
    }

    /// Counts data that did not come through.
    pub fn lose(&mut self, category: &str, n: usize) {
        *self.dropped.entry(category.to_owned()).or_insert(0) += n;
    }

    /// Counts an element that is not interpreted, by name; one note per name is made at the end.
    pub fn ignore(&mut self, name: &str) {
        *self.ignored.entry(name.to_owned()).or_insert(0) += 1;
    }

    /// Counts an item read in a simpler form; one note per kind is made at the end.
    pub fn simplify(&mut self, what: &str, detail: &str) {
        let entry = self
            .simplified
            .entry(what.to_owned())
            .or_insert((0, detail.to_owned()));
        entry.0 += 1;
    }

    /// The diagnostics, with the notes of the counted elements and simplified items, and the
    /// quality report.
    pub fn finish(mut self) -> (Vec<Diagnostic>, QualityReport) {
        let ignored = std::mem::take(&mut self.ignored);
        for (name, count) in &ignored {
            self.push(KicadError::NotRead {
                name: name.clone(),
                count: *count,
            });
            *self
                .dropped
                .entry(format!("`{name}` elements"))
                .or_insert(0) += *count;
        }
        let simplified = std::mem::take(&mut self.simplified);
        for (what, (count, detail)) in &simplified {
            self.push(KicadError::Simplified {
                what: format!("{count} x {what}"),
                detail: detail.clone(),
            });
        }
        if self.suppressed > 0 {
            let extra = self.suppressed;
            self.diagnostics.push(
                KicadError::Simplified {
                    what: format!("{extra} more finding(s)"),
                    detail: "not listed, because of max_diagnostics".to_owned(),
                }
                .diagnostic(),
            );
        }
        let mut quality = QualityReport::default();
        for (name, n) in &self.kept {
            QualityReport::add(&mut quality.kept, name, *n);
        }
        for (name, n) in &self.transformed {
            QualityReport::add(&mut quality.transformed, name, *n);
        }
        for (name, n) in &self.dropped {
            QualityReport::add(&mut quality.dropped, name, *n);
        }
        (self.diagnostics, quality)
    }
}

/// A decimal in millimetres as integer nanometres. `None` when it is not a number or does not fit
/// the checked range of the model.
pub(crate) fn nm_text(text: &str) -> Option<i64> {
    if text.bytes().any(|b| b == b'e' || b == b'E') {
        let value: f64 = text.parse().ok()?;
        if !value.is_finite() {
            return None;
        }
        let scaled = (value * 1e6).round();
        if scaled.abs() >= COORD_LIMIT_NM as f64 {
            return None;
        }
        return Some(scaled as i64);
    }
    Nm::parse_mm(text).ok().map(|(n, _)| n.get())
}

/// A decimal in degrees as integer micro-degrees, taken to a value inside one turn either way.
pub(crate) fn udeg_text(text: &str) -> Option<i64> {
    let d = ScaledDecimal::parse(text).ok()?;
    i64::try_from(d.scaled % 360_000_000).ok()
}

/// The first two arguments of a node as a position, as in `(xy 1 2)` or `(start 1 2)`.
pub(crate) fn xy_args(node: &Node) -> Option<P> {
    Some((nm_text(node.arg_text(0)?)?, nm_text(node.arg_text(1)?)?))
}

/// A position with an angle, as in `(at 1 2)` or `(at 1 2 90)`. The angle is in micro-degrees.
pub(crate) fn at_args(node: &Node) -> Option<(P, i64)> {
    let p = xy_args(node)?;
    let angle = node.arg_text(2).and_then(udeg_text).unwrap_or(0);
    Some((p, angle))
}

/// A length in nanometres from the first argument of a node, as in `(width 0.25)`.
pub(crate) fn length_arg(node: Option<&Node>) -> Option<i64> {
    nm_text(node?.arg_text(0)?)
}

/// The width of a line: `(width w)` of KiCad 6 or `(stroke (width w) ...)` of KiCad 7 and later.
pub(crate) fn stroke_width(node: &Node) -> Option<i64> {
    length_arg(node.child("width")).or_else(|| length_arg(node.child("stroke")?.child("width")))
}

/// A position of the file (y down) as a point of the model (y up).
pub(crate) fn to_model(p: P) -> Option<Point> {
    Point::from_nm(p.0, p.1.checked_neg()?).ok()
}

/// A position that is already in the model's orientation (inside a symbol).
pub(crate) fn to_model_up(p: P) -> Option<Point> {
    Point::from_nm(p.0, p.1).ok()
}

/// Turns an offset by `quarters` quarter turns, counter-clockwise with y pointing up.
pub(crate) fn turn_up(p: P, quarters: i64) -> P {
    match quarters.rem_euclid(4) {
        0 => p,
        1 => (-p.1, p.0),
        2 => (-p.0, -p.1),
        _ => (p.1, -p.0),
    }
}

/// Checks that the file is of one of the kinds asked for and reads the version and the generator
/// it declares. A version outside the tested range is a warning.
pub(crate) fn check_root(
    root: &Node,
    wanted: &[&str],
    label: &str,
    ctx: &mut Ctx,
) -> Result<(String, String), KicadError> {
    let found = root.head().unwrap_or("");
    if !wanted.contains(&found) {
        let shown: String = found.chars().take(40).collect();
        return Err(KicadError::NotKicad {
            expected: label.to_owned(),
            found: if shown.is_empty() {
                "a list without a name".to_owned()
            } else {
                format!("a `{shown}` list")
            },
        });
    }
    let version = root
        .child("version")
        .and_then(|n| n.arg_text(0))
        .unwrap_or("");
    let generator = root
        .child("generator")
        .and_then(|n| n.arg_text(0))
        .unwrap_or("");
    match version.parse::<u64>() {
        Ok(v) if v < ctx.settings.min_tested_version => ctx.push(KicadError::Version {
            found: version.to_owned(),
            detail: format!(
                "older than {}, the oldest version covered by the tests",
                ctx.settings.min_tested_version
            ),
        }),
        Ok(v) if v > ctx.settings.max_tested_version => ctx.push(KicadError::Version {
            found: version.to_owned(),
            detail: format!(
                "newer than {}, the newest version covered by the tests",
                ctx.settings.max_tested_version
            ),
        }),
        Ok(_) => {}
        Err(_) => ctx.push(KicadError::Version {
            found: if version.is_empty() {
                "none".to_owned()
            } else {
                version.chars().take(20).collect()
            },
            detail: "the file does not declare a usable version".to_owned(),
        }),
    }
    Ok((version.to_owned(), generator.to_owned()))
}

/// The text of a file and, when some bytes were not UTF-8, the note about it.
pub(crate) fn decode(
    bytes: &[u8],
    settings: &KicadSettings,
) -> Result<(String, Option<KicadError>), KicadError> {
    if bytes.len() as u64 > settings.max_file_bytes {
        return Err(KicadError::Limit {
            what: "the size of the file in bytes".to_owned(),
            value: bytes.len() as u64,
            limit: settings.max_file_bytes,
            line: 0,
            column: 0,
        });
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok((text.to_owned(), None)),
        Err(e) => Ok((
            String::from_utf8_lossy(bytes).into_owned(),
            Some(KicadError::Encoding {
                reason: format!(
                    "invalid bytes after offset {}; they were replaced",
                    e.valid_up_to()
                ),
            }),
        )),
    }
}

/// True when a sheet or file name stays below the folder it is read from: no absolute path, no
/// drive, no `..` and no empty name.
pub fn is_safe_relative_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\0') {
        return false;
    }
    let normal = name.replace('\\', "/");
    if normal.starts_with('/') || normal.chars().nth(1) == Some(':') {
        return false;
    }
    normal
        .split('/')
        .all(|part| part != ".." && !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millimetres_become_exact_nanometres() {
        assert_eq!(nm_text("1.27"), Some(1_270_000));
        assert_eq!(nm_text("-0.0254"), Some(-25_400));
        assert_eq!(nm_text(".5"), Some(500_000));
        assert_eq!(nm_text("0.0000004"), Some(0));
        assert_eq!(nm_text("1e-3"), Some(1000));
        assert_eq!(nm_text("2.5E1"), Some(25_000_000));
        for bad in ["", "abc", "1.2.3", "1e999", "nan", "5000", "--1"] {
            assert_eq!(nm_text(bad), None, "{bad}");
        }
    }

    #[test]
    fn degrees_stay_inside_one_turn() {
        assert_eq!(udeg_text("90"), Some(90_000_000));
        assert_eq!(udeg_text("-45.5"), Some(-45_500_000));
        assert_eq!(udeg_text("450"), Some(90_000_000));
        assert_eq!(udeg_text("360"), Some(0));
        assert_eq!(udeg_text("x"), None);
    }

    #[test]
    fn positions_and_angles_are_read_from_nodes() {
        let settings = KicadSettings::default();
        let root = crate::sexpr::parse("(a (at 1 -2 90) (xy 3 4) (at 1))", &settings).unwrap();
        let at = root.child("at").unwrap();
        assert_eq!(at_args(at), Some(((1_000_000, -2_000_000), 90_000_000)));
        assert_eq!(
            xy_args(root.child("xy").unwrap()),
            Some((3_000_000, 4_000_000))
        );
        let bad = root.children("at").nth(1).unwrap();
        assert_eq!(at_args(bad), None);
    }

    #[test]
    fn the_stroke_width_has_the_form_of_each_version() {
        let settings = KicadSettings::default();
        let v6 = crate::sexpr::parse("(gr_line (width 0.1))", &settings).unwrap();
        let v7 =
            crate::sexpr::parse("(gr_line (stroke (width 0.2) (type solid)))", &settings).unwrap();
        assert_eq!(stroke_width(&v6), Some(100_000));
        assert_eq!(stroke_width(&v7), Some(200_000));
        let none = crate::sexpr::parse("(gr_line)", &settings).unwrap();
        assert_eq!(stroke_width(&none), None);
    }

    #[test]
    fn the_y_axis_is_turned_and_quarter_turns_are_counter_clockwise() {
        let p = to_model((1_000_000, 2_000_000)).unwrap();
        assert_eq!((p.x_nm.get(), p.y_nm.get()), (1_000_000, -2_000_000));
        assert!(to_model((i64::MAX, 0)).is_none());
        assert_eq!(turn_up((3, 1), 0), (3, 1));
        assert_eq!(turn_up((3, 1), 1), (-1, 3));
        assert_eq!(turn_up((3, 1), 2), (-3, -1));
        assert_eq!(turn_up((3, 1), 3), (1, -3));
        assert_eq!(turn_up((3, 1), -1), (1, -3));
        assert_eq!(turn_up((3, 1), 5), (-1, 3));
    }

    #[test]
    fn names_that_leave_the_folder_are_not_safe() {
        for good in ["a.kicad_sch", "sub/a.kicad_sch", "sub\\a.kicad_sch"] {
            assert!(is_safe_relative_name(good), "{good}");
        }
        for bad in [
            "",
            "../a.kicad_sch",
            "sub/../../a",
            "/etc/passwd",
            "C:\\a",
            "c:/a",
            "a//b",
            "a\0b",
            "\\\\server\\share",
        ] {
            assert!(!is_safe_relative_name(bad), "{bad}");
        }
    }

    #[test]
    fn the_context_counts_and_limits_what_it_keeps() {
        let settings = KicadSettings {
            max_diagnostics: 2,
            ..KicadSettings::default()
        };
        let mut make = crate::sequential_ids().unwrap();
        let mut ctx = Ctx::new(&settings, &mut make);
        for _ in 0..4 {
            ctx.push(KicadError::Derived { detail: "x".into() });
        }
        ctx.keep("pads", 2);
        ctx.keep("pads", 3);
        ctx.ignore("image");
        ctx.ignore("image");
        ctx.simplify("pad shape `custom`", "read as a rectangle");
        let (diagnostics, quality) = ctx.finish();
        // two kept, then the notes of the end are added after the limit: the limit counts findings
        assert!(diagnostics.len() >= 2);
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("more finding(s)")));
        assert_eq!(quality.kept[0].count, 5);
        assert_eq!(quality.dropped[0].category, "`image` elements");
        assert_eq!(quality.dropped[0].count, 2);
    }
}
