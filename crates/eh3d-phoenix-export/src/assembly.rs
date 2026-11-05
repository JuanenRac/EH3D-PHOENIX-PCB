//! The assembly files: the placement (pick and place) file and the bill of materials, as CSV.
//!
//! Both are plain text with one line per part (or per group of parts), written in a fixed order, so
//! the same board always gives the same bytes. Lines end with a single line feed. A field with the
//! delimiter, a quote, a line break or a space at either end is put in quotes.
//!
//! Rounding: a coordinate in nanometres is shown in millimetres and a rotation in micro-degrees in
//! degrees, both by integer arithmetic, rounded half away from zero to the decimals of the profile.
//! The order of the parts is the natural order of the reference designators (`R2` before `R10`).

use std::collections::BTreeMap;

use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_board::{Board, Side};

use crate::codes;
use crate::profile::{BomProfile, BottomRotation, Origin, PlacementProfile};

const FULL_TURN: i64 = 360_000_000;
const HALF_TURN: i64 = 180_000_000;

/// A value in millionths of a unit (nanometres as millimetres, micro-degrees as degrees) with
/// `decimals` digits after the point (at most 6), rounded half away from zero.
pub fn format_micro(value: i64, decimals: u32) -> String {
    let decimals = decimals.min(6);
    let step = 10i128.pow(6 - decimals);
    let magnitude = i128::from(value).abs();
    let rounded = (magnitude + step / 2) / step;
    let scale = 10i128.pow(decimals);
    let sign = if value < 0 && rounded != 0 { "-" } else { "" };
    if decimals == 0 {
        format!("{sign}{rounded}")
    } else {
        format!(
            "{sign}{}.{:0width$}",
            rounded / scale,
            rounded % scale,
            width = decimals as usize
        )
    }
}

/// The key of the natural order of a designator: the letters, the number, the rest.
fn natural_key(text: &str) -> (String, u64, String) {
    let split = text
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(text.len());
    let (prefix, rest) = text.split_at(split);
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let (digits, tail) = rest.split_at(end);
    let number = if digits.is_empty() {
        0
    } else {
        digits.parse::<u64>().unwrap_or(u64::MAX)
    };
    (prefix.to_owned(), number, tail.to_owned())
}

/// A field of a CSV line. A line break inside a field would split the line, so it becomes a space.
fn csv_field(field: &str, delimiter: char) -> String {
    let field = field.replace(['\n', '\r'], " ");
    let needs_quotes = field.contains(delimiter)
        || field.contains('"')
        || field.starts_with(' ')
        || field.ends_with(' ');
    if needs_quotes {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field
    }
}

/// Reads CSV text written by this module back into rows of fields.
pub fn parse_csv(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' {
            quoted = true;
        } else if c == delimiter {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else {
            field.push(c);
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn csv_line(fields: &[String], delimiter: char) -> String {
    let mut line = fields
        .iter()
        .map(|f| csv_field(f, delimiter))
        .collect::<Vec<_>>()
        .join(&delimiter.to_string());
    line.push('\n');
    line
}

/// The rotation written for a part, in micro-degrees from 0 up to a full turn (not included).
fn written_rotation(rotation: i64, side: Side, rule: BottomRotation) -> i64 {
    let r = rotation.rem_euclid(FULL_TURN);
    if side == Side::Top {
        return r;
    }
    match rule {
        BottomRotation::Keep => r,
        BottomRotation::Mirror => (FULL_TURN - r).rem_euclid(FULL_TURN),
        BottomRotation::HalfTurn => (r + HALF_TURN).rem_euclid(FULL_TURN),
    }
}

struct Row {
    refdes: String,
    value: String,
    package: String,
    x: i64,
    y: i64,
    rotation: i64,
    side: Side,
}

fn rows(board: &Board, profile: &PlacementProfile) -> Vec<Row> {
    let (origin_x, origin_y) = match profile.origin {
        Origin::Absolute => (0, 0),
        Origin::OutlineMin => (
            board
                .outline
                .iter()
                .map(|p| p.x_nm.get())
                .min()
                .unwrap_or(0),
            board
                .outline
                .iter()
                .map(|p| p.y_nm.get())
                .min()
                .unwrap_or(0),
        ),
    };
    let mut parts: Vec<_> = board.components.iter().collect();
    parts.sort_by_cached_key(|(id, c)| (natural_key(&c.refdes), **id));
    parts
        .into_iter()
        .map(|(_, c)| Row {
            refdes: c.refdes.clone(),
            value: c.value.clone(),
            package: board
                .footprints
                .get(&c.footprint)
                .map_or_else(String::new, |f| f.name.clone()),
            x: c.at.x_nm.get() - origin_x,
            y: c.at.y_nm.get() - origin_y,
            rotation: written_rotation(c.rotation.get(), c.side, profile.bottom_rotation),
            side: c.side,
        })
        .collect()
}

/// The placement file: one line per component, with the columns of the profile.
pub fn placement_csv(board: &Board, profile: &PlacementProfile) -> (String, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut text = csv_line(
        &profile
            .columns
            .iter()
            .map(|c| c.header.clone())
            .collect::<Vec<_>>(),
        profile.delimiter,
    );
    let rows = rows(board, profile);
    if rows.is_empty() {
        diagnostics.push(codes::diagnostic(codes::NO_COMPONENTS, Severity::Info, ""));
    }
    for row in &rows {
        let fields: Vec<String> = profile
            .columns
            .iter()
            .map(|c| match c.key.as_str() {
                "ref" => row.refdes.clone(),
                "value" => row.value.clone(),
                "package" => row.package.clone(),
                "x" => format_micro(row.x, profile.coordinate_decimals),
                "y" => format_micro(row.y, profile.coordinate_decimals),
                "rotation" => format_micro(row.rotation, profile.rotation_decimals),
                "side" => match row.side {
                    Side::Top => profile.side_top.clone(),
                    Side::Bottom => profile.side_bottom.clone(),
                },
                _ => String::new(),
            })
            .collect();
        text.push_str(&csv_line(&fields, profile.delimiter));
    }
    (text, diagnostics)
}

/// The bill of materials: the components grouped by value and package, with their designators.
pub fn bom_csv(board: &Board, profile: &BomProfile) -> (String, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut text = csv_line(
        &profile
            .columns
            .iter()
            .map(|c| c.header.clone())
            .collect::<Vec<_>>(),
        profile.delimiter,
    );
    // the package name is not needed to place a part, so the rows of the placement are reused
    let all = rows(board, &PlacementProfile::default());
    if all.is_empty() {
        diagnostics.push(codes::diagnostic(codes::NO_COMPONENTS, Severity::Info, ""));
    }
    let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for row in all {
        groups
            .entry((row.value, row.package))
            .or_default()
            .push(row.refdes);
    }
    // the rows came in the natural order of the designators, so each list is already in order
    let mut lines: Vec<((String, String), Vec<String>)> = groups.into_iter().collect();
    lines.sort_by_cached_key(|(key, designators)| {
        (designators.first().map(|d| natural_key(d)), key.clone())
    });
    for ((value, package), designators) in &lines {
        let fields: Vec<String> = profile
            .columns
            .iter()
            .map(|c| match c.key.as_str() {
                "designators" => designators.join(&profile.designator_separator),
                "quantity" => designators.len().to_string(),
                "value" => value.clone(),
                "package" => package.clone(),
                _ => String::new(),
            })
            .collect();
        text.push_str(&csv_line(&fields, profile.delimiter));
    }
    (text, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micro_units_are_rounded_half_away_from_zero() {
        assert_eq!(format_micro(1_234_567, 4), "1.2346");
        assert_eq!(format_micro(-1_234_567, 4), "-1.2346");
        assert_eq!(format_micro(1_234_500, 3), "1.235");
        assert_eq!(format_micro(-1_234_500, 3), "-1.235");
        assert_eq!(format_micro(0, 2), "0.00");
        assert_eq!(format_micro(-400, 3), "0.000");
        assert_eq!(format_micro(45_000_000, 0), "45");
        assert_eq!(format_micro(999_999, 0), "1");
        assert_eq!(format_micro(1, 9), "0.000001");
    }

    #[test]
    fn designators_sort_by_their_number_not_their_text() {
        let mut list = vec!["R10", "C2", "R2", "C10", "R1A", "U1"];
        list.sort_by_cached_key(|s| natural_key(s));
        assert_eq!(list, vec!["C2", "C10", "R1A", "R2", "R10", "U1"]);
    }

    #[test]
    fn fields_are_quoted_only_when_they_must_be() {
        assert_eq!(csv_field("10k", ','), "10k");
        assert_eq!(csv_field("a,b", ','), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\"", ','), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("a,b", ';'), "a,b");
        assert_eq!(csv_field("two\nlines", ','), "two lines");
        let text = csv_line(
            &["a,b".to_owned(), "say \"hi\"".to_owned(), String::new()],
            ',',
        ) + "x,y,z\n";
        assert_eq!(
            parse_csv(&text, ','),
            vec![
                vec!["a,b".to_owned(), "say \"hi\"".to_owned(), String::new()],
                vec!["x".to_owned(), "y".to_owned(), "z".to_owned()]
            ]
        );
        assert_eq!(csv_field(" x", ','), "\" x\"");
        assert_eq!(
            csv_line(&["a".to_owned(), "b c".to_owned()], ';'),
            "a;b c\n"
        );
    }

    #[test]
    fn the_bottom_rotation_follows_the_rule() {
        let r = 30_000_000;
        assert_eq!(written_rotation(r, Side::Top, BottomRotation::Mirror), r);
        assert_eq!(written_rotation(r, Side::Bottom, BottomRotation::Keep), r);
        assert_eq!(
            written_rotation(r, Side::Bottom, BottomRotation::Mirror),
            330_000_000
        );
        assert_eq!(
            written_rotation(r, Side::Bottom, BottomRotation::HalfTurn),
            210_000_000
        );
        assert_eq!(written_rotation(0, Side::Bottom, BottomRotation::Mirror), 0);
        assert_eq!(
            written_rotation(270_000_000, Side::Bottom, BottomRotation::HalfTurn),
            90_000_000
        );
        assert_eq!(
            written_rotation(-90_000_000, Side::Top, BottomRotation::Keep),
            270_000_000
        );
    }
}
