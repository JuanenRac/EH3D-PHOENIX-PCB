// SPDX-License-Identifier: GPL-3.0-or-later
//! The requests that the server mode makes for the plugin `pcb` of EH3D-SERVER: `rules_check`,
//! `drill_report`, `bom_report` and `inspect`. A board is first reduced to plain facts (the sizes
//! of the tracks, the gaps between copper, the holes and the parts), and the facts are written as
//! the JSON text of the request, the same every time. The server checks the request against the
//! rules of its station and makes the report; the numbers are in millimetres, written from the
//! integer nanometres of the board without rounding.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, PadRef, Segment};
use eh3d_serverclient::json::quote;

use crate::codes::{
    Note, ServerError, Severity, INPUT_UNREADABLE, LIMIT_EXCEEDED, NOTHING_TO_SEND, PARTIAL_REQUEST,
};
use crate::settings::ServerSettings;

/// The kind of report asked of the station.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The rules of the maker against the tracks, the gaps and the holes.
    Rules,
    /// The tools, the hits and the travel of the drill.
    Drill,
    /// The parts grouped by value and footprint.
    Bom,
    /// Measurements against their tolerances.
    Inspect,
}

impl Kind {
    /// The word of the command line.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Rules => "rules",
            Kind::Drill => "drill",
            Kind::Bom => "bom",
            Kind::Inspect => "inspect",
        }
    }

    /// The `kind` of the request, the name the plugin knows.
    pub fn request_kind(self) -> &'static str {
        match self {
            Kind::Rules => "rules_check",
            Kind::Drill => "drill_report",
            Kind::Bom => "bom_report",
            Kind::Inspect => "inspect",
        }
    }

    /// Reads the word of the command line.
    pub fn from_word(word: &str) -> Option<Kind> {
        [Kind::Rules, Kind::Drill, Kind::Bom, Kind::Inspect]
            .into_iter()
            .find(|k| k.word() == word)
    }
}

/// A track: its name and its width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackFact {
    /// The net and the layer.
    pub name: String,
    /// The width in nanometres.
    pub width_nm: i64,
}

/// A gap between the copper of two nets on a layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearanceFact {
    /// The two nets and the layer.
    pub name: String,
    /// The gap in nanometres (0 when the copper touches).
    pub gap_nm: i64,
}

/// A hole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillFact {
    /// The via or the pad.
    pub name: String,
    /// The diameter in nanometres.
    pub dia_nm: i64,
    /// The place across, from the left of the outline, in nanometres.
    pub x_nm: i64,
    /// The place up, from the bottom of the outline, in nanometres.
    pub y_nm: i64,
    /// Whether the wall of the hole is plated.
    pub plated: bool,
}

/// A part of the bill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartFact {
    /// The reference.
    pub refdes: String,
    /// The value.
    pub value: String,
    /// The name of the footprint.
    pub footprint: String,
}

/// A measurement of the inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// What was measured.
    pub name: String,
    /// The value it should have.
    pub nominal: f64,
    /// How far from it the value may be.
    pub tolerance: f64,
    /// The value that was measured.
    pub value: f64,
    /// The unit, up to 8 characters; may be empty.
    pub unit: String,
}

/// What a request needs to know of a board.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    /// The width and the height of the box of the outline, in nanometres; none when the outline
    /// has fewer than three corners or no area.
    pub outline_nm: Option<(i64, i64)>,
    /// The tracks.
    pub tracks: Vec<TrackFact>,
    /// The gaps between copper of different nets, the smallest first.
    pub clearances: Vec<ClearanceFact>,
    /// The holes of vias and of drilled pads.
    pub drills: Vec<DrillFact>,
    /// The parts.
    pub parts: Vec<PartFact>,
    /// What was left out or approximated.
    pub notes: Vec<Note>,
}

/// A number of nanometres as millimetres in text, exact: no exponent, no trailing zeros, at least
/// one digit before the point.
pub fn mm_text(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let abs = nm.unsigned_abs();
    let (whole, frac) = (abs / 1_000_000, abs % 1_000_000);
    if frac == 0 {
        return format!("{sign}{whole}");
    }
    let digits = format!("{frac:06}");
    format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
}

/// A name that goes into a request: control characters become blanks, up to 64 characters.
pub fn clean_name(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(64)
        .collect();
    cleaned.trim().to_owned()
}

fn too_many(what: &str, count: usize, limit: usize) -> ServerError {
    ServerError::new(
        LIMIT_EXCEEDED,
        format!("{count} {what} are more than max_items = {limit}"),
    )
}

// ---------------------------------------------------------------------------------------------
// the gaps between copper

type P = (i64, i64);

fn cross(a: P, b: P, c: P) -> i128 {
    (i128::from(b.0) - i128::from(a.0)) * (i128::from(c.1) - i128::from(a.1))
        - (i128::from(b.1) - i128::from(a.1)) * (i128::from(c.0) - i128::from(a.0))
}

fn on_segment(a: P, b: P, p: P) -> bool {
    (a.0.min(b.0)..=a.0.max(b.0)).contains(&p.0) && (a.1.min(b.1)..=a.1.max(b.1)).contains(&p.1)
}

/// Whether two straight pieces share a point, by exact integer arithmetic.
fn pieces_meet(a: P, b: P, c: P, d: P) -> bool {
    let (d1, d2) = (cross(a, b, c).signum(), cross(a, b, d).signum());
    let (d3, d4) = (cross(c, d, a).signum(), cross(c, d, b).signum());
    if d1 * d2 < 0 && d3 * d4 < 0 {
        return true;
    }
    (d1 == 0 && on_segment(a, b, c))
        || (d2 == 0 && on_segment(a, b, d))
        || (d3 == 0 && on_segment(c, d, a))
        || (d4 == 0 && on_segment(c, d, b))
}

fn point_to_piece(p: P, a: P, b: P) -> f64 {
    let (px, py) = (p.0 as f64, p.1 as f64);
    let (ax, ay) = (a.0 as f64, a.1 as f64);
    let (dx, dy) = (b.0 as f64 - ax, b.1 as f64 - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

/// The distance between two straight pieces, in nanometres; 0 when they meet.
pub fn piece_distance(a: P, b: P, c: P, d: P) -> f64 {
    if pieces_meet(a, b, c, d) {
        return 0.0;
    }
    point_to_piece(a, c, d)
        .min(point_to_piece(b, c, d))
        .min(point_to_piece(c, a, b))
        .min(point_to_piece(d, a, b))
}

/// A track as the gaps see it: its net and layer, its pieces and the box they fill.
struct Run {
    net: String,
    layer: String,
    width: i64,
    pieces: Vec<(P, P)>,
    low: P,
    high: P,
}

fn runs_of(board: &Board, arcs: &mut bool) -> Vec<Run> {
    let mut runs = Vec::new();
    for track in board.tracks.values() {
        let mut pieces = Vec::new();
        for segment in &track.path {
            let (s, e) = (segment.start(), segment.end());
            if matches!(segment, Segment::Arc { .. }) {
                *arcs = true;
            }
            pieces.push(((s.x_nm.get(), s.y_nm.get()), (e.x_nm.get(), e.y_nm.get())));
        }
        if pieces.is_empty() {
            continue;
        }
        let mut low = pieces[0].0;
        let mut high = pieces[0].0;
        for (a, b) in &pieces {
            for p in [a, b] {
                low = (low.0.min(p.0), low.1.min(p.1));
                high = (high.0.max(p.0), high.1.max(p.1));
            }
        }
        runs.push(Run {
            net: board
                .nets
                .get(&track.net)
                .map_or_else(|| "?".to_owned(), |n| n.name.clone()),
            layer: board
                .layers
                .get(&track.layer)
                .map_or_else(|| "?".to_owned(), |l| l.name.clone()),
            width: track.width.get(),
            pieces,
            low,
            high,
        });
    }
    runs
}

/// The gaps between the copper of tracks of different nets on the same layer that are smaller than
/// `below_nm`, the smallest first and at most `max` of them. The second value is false when the
/// pieces to compare went over the budget, so the list may miss gaps.
pub fn gaps_between_tracks(
    board: &Board,
    below_nm: i64,
    max: usize,
    budget: u64,
) -> (Vec<ClearanceFact>, bool, bool) {
    let mut arcs = false;
    let runs = runs_of(board, &mut arcs);
    let mut found: Vec<(i64, String)> = Vec::new();
    let mut spent = 0u64;
    let mut complete = true;
    'outer: for (i, a) in runs.iter().enumerate() {
        for b in &runs[i + 1..] {
            if a.layer != b.layer || a.net == b.net {
                continue;
            }
            let reach = below_nm.saturating_add((a.width + b.width) / 2);
            if a.high.0.saturating_add(reach) < b.low.0
                || b.high.0.saturating_add(reach) < a.low.0
                || a.high.1.saturating_add(reach) < b.low.1
                || b.high.1.saturating_add(reach) < a.low.1
            {
                continue;
            }
            let mut best = f64::MAX;
            for (p, q) in &a.pieces {
                for (r, s) in &b.pieces {
                    spent += 1;
                    if spent > budget {
                        complete = false;
                        break 'outer;
                    }
                    best = best.min(piece_distance(*p, *q, *r, *s));
                }
            }
            let gap = (best - (a.width + b.width) as f64 / 2.0).max(0.0).floor() as i64;
            if gap < below_nm {
                let (first, second) = if a.net <= b.net {
                    (&a.net, &b.net)
                } else {
                    (&b.net, &a.net)
                };
                found.push((gap, clean_name(&format!("{first}/{second} {}", a.layer))));
            }
        }
    }
    found.sort();
    found.truncate(max);
    let facts = found
        .into_iter()
        .map(|(gap_nm, name)| ClearanceFact { name, gap_nm })
        .collect();
    (facts, complete, arcs)
}

// ---------------------------------------------------------------------------------------------
// the facts of a board

fn outline_box(outline: &[P]) -> Option<(P, P)> {
    if outline.len() < 3 {
        return None;
    }
    let mut low = outline[0];
    let mut high = outline[0];
    for p in outline {
        low = (low.0.min(p.0), low.1.min(p.1));
        high = (high.0.max(p.0), high.1.max(p.1));
    }
    (high.0 > low.0 && high.1 > low.1).then_some((low, high))
}

/// Reduces a board to the facts a request needs.
pub fn facts_from_board(board: &Board, settings: &ServerSettings) -> Result<Facts, ServerError> {
    let mut facts = Facts::default();
    let corners: Vec<P> = board
        .outline
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect();
    let origin = match outline_box(&corners) {
        Some((low, high)) => {
            facts.outline_nm = Some((high.0 - low.0, high.1 - low.1));
            low
        }
        None => (0, 0),
    };
    let net_name = |id| {
        board
            .nets
            .get(id)
            .map_or_else(|| "?".to_owned(), |n| n.name.clone())
    };
    let layer_name = |id| {
        board
            .layers
            .get(id)
            .map_or_else(|| "?".to_owned(), |l| l.name.clone())
    };
    for track in board.tracks.values() {
        facts.tracks.push(TrackFact {
            name: clean_name(&format!(
                "{} {}",
                net_name(&track.net),
                layer_name(&track.layer)
            )),
            width_nm: track.width.get(),
        });
    }
    for via in board.vias.values() {
        facts.drills.push(DrillFact {
            name: clean_name(&format!("via {}", net_name(&via.net))),
            dia_nm: via.drill.get(),
            x_nm: via.at.x_nm.get() - origin.0,
            y_nm: via.at.y_nm.get() - origin.1,
            plated: true,
        });
    }
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        facts.parts.push(PartFact {
            refdes: component.refdes.clone(),
            value: component.value.clone(),
            footprint: footprint.name.clone(),
        });
        for (pad_id, pad) in &footprint.pads {
            let Some(drill) = pad.drill else { continue };
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *component_id,
                    pad: *pad_id,
                },
            ) else {
                continue;
            };
            facts.drills.push(DrillFact {
                name: clean_name(&format!("{}.{}", component.refdes, pad.number)),
                dia_nm: drill.get(),
                x_nm: at.x_nm.get() - origin.0,
                y_nm: at.y_nm.get() - origin.1,
                plated: pad.plated,
            });
        }
    }
    if settings.clearance_max > 0
        && !facts.tracks.is_empty()
        && facts.tracks.len() <= settings.clearance_max_tracks
    {
        let (gaps, complete, arcs) = gaps_between_tracks(
            board,
            settings.clearance_below_nm,
            settings.clearance_max,
            settings.clearance_pair_budget,
        );
        facts.clearances = gaps;
        if !complete {
            facts.notes.push(Note::new(
                PARTIAL_REQUEST,
                Severity::Warning,
                "the gaps between tracks were cut at clearance_pair_budget, so some may be missing",
            ));
        }
        if arcs {
            facts.notes.push(Note::new(
                PARTIAL_REQUEST,
                Severity::Info,
                "the arcs of the tracks were taken as their chords for the gaps",
            ));
        }
    } else if !facts.tracks.is_empty() && settings.clearance_max > 0 {
        facts.notes.push(Note::new(
            PARTIAL_REQUEST,
            Severity::Info,
            format!(
                "{} tracks are more than clearance_max_tracks = {}: no gaps were listed",
                facts.tracks.len(),
                settings.clearance_max_tracks
            ),
        ));
    }
    if facts.outline_nm.is_none() {
        facts.notes.push(Note::new(
            PARTIAL_REQUEST,
            Severity::Info,
            "the outline has no area, so the holes are not judged against the edge",
        ));
    }
    Ok(facts)
}

// ---------------------------------------------------------------------------------------------
// the text of the requests

fn rules_object(settings: &ServerSettings) -> String {
    let overrides = settings.rule_overrides();
    if overrides.is_empty() {
        return String::new();
    }
    let members: Vec<String> = overrides
        .iter()
        .map(|(key, nm)| format!("{}:{}", quote(key), mm_text(*nm)))
        .collect();
    format!(",\"rules\":{{{}}}", members.join(","))
}

/// The request `rules_check`: the box of the outline, the widths of the tracks, the gaps and the
/// holes. The number of items is checked against `max_items`.
pub fn rules_request(facts: &Facts, settings: &ServerSettings) -> Result<String, ServerError> {
    let items = facts.tracks.len() + facts.clearances.len() + facts.drills.len();
    if items == 0 {
        return Err(ServerError::new(
            NOTHING_TO_SEND,
            "the board has no tracks, gaps or holes to check",
        ));
    }
    if items > settings.max_items {
        return Err(too_many("items", items, settings.max_items));
    }
    let outline = match facts.outline_nm {
        Some((w, h)) => format!(
            "\"outline\":{{\"width_mm\":{},\"height_mm\":{}}},",
            mm_text(w),
            mm_text(h)
        ),
        None => String::new(),
    };
    let tracks: Vec<String> = facts
        .tracks
        .iter()
        .map(|t| {
            format!(
                "{{\"name\":{},\"width_mm\":{}}}",
                quote(&t.name),
                mm_text(t.width_nm)
            )
        })
        .collect();
    let gaps: Vec<String> = facts
        .clearances
        .iter()
        .map(|c| {
            format!(
                "{{\"name\":{},\"gap_mm\":{}}}",
                quote(&c.name),
                mm_text(c.gap_nm)
            )
        })
        .collect();
    let drills: Vec<String> = facts.drills.iter().map(drill_json).collect();
    Ok(format!(
        "{{\"kind\":\"rules_check\",\"board\":{{{outline}\"tracks\":[{}],\"clearances\":[{}],\"drills\":[{}]}}{}}}",
        tracks.join(","),
        gaps.join(","),
        drills.join(","),
        rules_object(settings)
    ))
}

fn drill_json(d: &DrillFact) -> String {
    format!(
        "{{\"name\":{},\"dia_mm\":{},\"x\":{},\"y\":{},\"plated\":{}}}",
        quote(&d.name),
        mm_text(d.dia_nm),
        mm_text(d.x_nm),
        mm_text(d.y_nm),
        d.plated
    )
}

/// The request `drill_report`: the holes with their place and whether they are plated.
pub fn drill_request(facts: &Facts, settings: &ServerSettings) -> Result<String, ServerError> {
    if facts.drills.is_empty() {
        return Err(ServerError::new(
            NOTHING_TO_SEND,
            "the board has no vias or drilled pads",
        ));
    }
    if facts.drills.len() > settings.max_items {
        return Err(too_many("holes", facts.drills.len(), settings.max_items));
    }
    let drills: Vec<String> = facts.drills.iter().map(drill_json).collect();
    Ok(format!(
        "{{\"kind\":\"drill_report\",\"drills\":[{}]}}",
        drills.join(",")
    ))
}

/// The request `bom_report`: the parts with their value and footprint.
pub fn bom_request(facts: &Facts, settings: &ServerSettings) -> Result<String, ServerError> {
    if facts.parts.is_empty() {
        return Err(ServerError::new(NOTHING_TO_SEND, "the board has no parts"));
    }
    if facts.parts.len() > settings.max_items {
        return Err(too_many("parts", facts.parts.len(), settings.max_items));
    }
    let parts: Vec<String> = facts
        .parts
        .iter()
        .map(|p| {
            format!(
                "{{\"ref\":{},\"value\":{},\"footprint\":{}}}",
                quote(&p.refdes),
                quote(&p.value),
                quote(&p.footprint)
            )
        })
        .collect();
    Ok(format!(
        "{{\"kind\":\"bom_report\",\"components\":[{}]}}",
        parts.join(",")
    ))
}

/// The request `inspect`: the measurements against their tolerances.
pub fn inspect_request(
    rows: &[Measurement],
    settings: &ServerSettings,
) -> Result<String, ServerError> {
    if rows.is_empty() {
        return Err(ServerError::new(
            NOTHING_TO_SEND,
            "the file has no measurements",
        ));
    }
    if rows.len() > settings.max_items {
        return Err(too_many("measurements", rows.len(), settings.max_items));
    }
    let list: Vec<String> = rows
        .iter()
        .map(|m| {
            format!(
                "{{\"name\":{},\"nominal\":{},\"tolerance\":{},\"value\":{},\"unit\":{}}}",
                quote(&m.name),
                m.nominal,
                m.tolerance,
                m.value,
                quote(&m.unit)
            )
        })
        .collect();
    Ok(format!(
        "{{\"kind\":\"inspect\",\"measurements\":[{}]}}",
        list.join(",")
    ))
}

/// Reads measurements from lines `name, nominal, tolerance, value[, unit]`; blank lines and lines
/// starting with `#` are ignored. A line that cannot be read is refused with its number.
pub fn parse_measurements(text: &str) -> Result<Vec<Measurement>, ServerError> {
    let mut rows = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split(',').map(str::trim).collect();
        if fields.len() != 4 && fields.len() != 5 {
            return Err(ServerError::new(
                INPUT_UNREADABLE,
                format!("line {line}: expected name, nominal, tolerance, value[, unit]"),
            ));
        }
        let number = |text: &str, what: &str| -> Result<f64, ServerError> {
            text.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| {
                    ServerError::new(
                        INPUT_UNREADABLE,
                        format!("line {line}: {what} is not a number"),
                    )
                })
        };
        let tolerance = number(fields[2], "tolerance")?;
        if tolerance < 0.0 {
            return Err(ServerError::new(
                INPUT_UNREADABLE,
                format!("line {line}: tolerance is from 0"),
            ));
        }
        let unit = fields.get(4).copied().unwrap_or("");
        if unit.chars().count() > 8 {
            return Err(ServerError::new(
                INPUT_UNREADABLE,
                format!("line {line}: the unit has up to 8 characters"),
            ));
        }
        rows.push(Measurement {
            name: clean_name(fields[0]),
            nominal: number(fields[1], "nominal")?,
            tolerance,
            value: number(fields[3], "value")?,
            unit: clean_name(unit),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_serverclient::Json;

    fn settings() -> ServerSettings {
        ServerSettings::default()
    }

    fn facts() -> Facts {
        Facts {
            outline_nm: Some((50_000_000, 30_000_000)),
            tracks: vec![
                TrackFact {
                    name: "GND top".into(),
                    width_nm: 150_000,
                },
                TrackFact {
                    name: "+5V top".into(),
                    width_nm: 400_000,
                },
            ],
            clearances: vec![ClearanceFact {
                name: "GND/+5V top".into(),
                gap_nm: 120_000,
            }],
            drills: vec![
                DrillFact {
                    name: "via GND".into(),
                    dia_nm: 300_000,
                    x_nm: 10_000_000,
                    y_nm: 5_500_000,
                    plated: true,
                },
                DrillFact {
                    name: "J1.1".into(),
                    dia_nm: 1_000_000,
                    x_nm: 1_000_000,
                    y_nm: 1_000_000,
                    plated: false,
                },
            ],
            parts: vec![
                PartFact {
                    refdes: "R1".into(),
                    value: "10k".into(),
                    footprint: "R0603".into(),
                },
                PartFact {
                    refdes: "R2".into(),
                    value: "10k".into(),
                    footprint: "R0603".into(),
                },
            ],
            notes: Vec::new(),
        }
    }

    #[test]
    fn millimetres_are_written_exactly_without_rounding_or_exponents() {
        for (nm, text) in [
            (0, "0"),
            (1, "0.000001"),
            (150_000, "0.15"),
            (1_000_000, "1"),
            (1_500_000, "1.5"),
            (-2_250_000, "-2.25"),
            (-1, "-0.000001"),
            (1_073_741_823, "1073.741823"),
            (-1_073_741_823, "-1073.741823"),
            (100_000_000, "100"),
        ] {
            assert_eq!(mm_text(nm), text, "{nm}");
        }
    }

    #[test]
    fn the_text_of_a_millimetre_value_reads_back_to_the_same_nanometres() {
        // a fixed-seed generator of values over the whole range of a coordinate
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        for _ in 0..500 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let nm = (seed % 2_147_483_647) as i64 - 1_073_741_823;
            let text = mm_text(nm);
            let back: f64 = text.parse().unwrap();
            assert_eq!((back * 1_000_000.0).round() as i64, nm, "{text}");
            assert!(!text.contains('e') && !text.ends_with('.'), "{text}");
        }
    }

    #[test]
    fn names_lose_their_control_characters_and_are_cut_at_64() {
        assert_eq!(clean_name("a\nb\tc"), "a b c");
        assert_eq!(clean_name("  x  "), "x");
        assert_eq!(clean_name(&"é".repeat(100)).chars().count(), 64);
        assert_eq!(clean_name("\u{7}"), "");
    }

    #[test]
    fn the_rules_request_is_json_the_server_reads_with_its_fields() {
        let text = rules_request(&facts(), &settings()).unwrap();
        let j = Json::parse(&text).unwrap();
        assert_eq!(j.get("kind").and_then(Json::as_str), Some("rules_check"));
        let board = j.get("board").unwrap();
        assert_eq!(
            board.get("outline").unwrap().get("width_mm"),
            Some(&Json::Num(50.0))
        );
        assert_eq!(board.get("tracks").unwrap().as_array().unwrap().len(), 2);
        assert_eq!(
            board.get("clearances").unwrap().as_array().unwrap()[0].get("gap_mm"),
            Some(&Json::Num(0.12))
        );
        let drills = board.get("drills").unwrap().as_array().unwrap();
        assert_eq!(drills[1].get("plated").and_then(Json::as_bool), Some(false));
        assert_eq!(drills[0].get("y"), Some(&Json::Num(5.5)));
        assert!(j.get("rules").is_none());
        // the request is the same every time
        assert_eq!(text, rules_request(&facts(), &settings()).unwrap());
    }

    #[test]
    fn a_rule_of_the_settings_goes_over_the_rule_of_the_station() {
        let mut s = settings();
        s.rule_min_track_nm = 200_000;
        s.rule_min_edge_nm = 500_000;
        let j = Json::parse(&rules_request(&facts(), &s).unwrap()).unwrap();
        let rules = j.get("rules").unwrap();
        assert_eq!(rules.get("min_track_mm"), Some(&Json::Num(0.2)));
        assert_eq!(rules.get("min_edge_mm"), Some(&Json::Num(0.5)));
        assert!(rules.get("min_gap_mm").is_none());
    }

    #[test]
    fn a_board_without_outline_sends_no_outline() {
        let mut f = facts();
        f.outline_nm = None;
        let j = Json::parse(&rules_request(&f, &settings()).unwrap()).unwrap();
        assert!(j.get("board").unwrap().get("outline").is_none());
    }

    #[test]
    fn the_other_requests_have_the_lists_the_plugin_asks_for() {
        let d = Json::parse(&drill_request(&facts(), &settings()).unwrap()).unwrap();
        assert_eq!(d.get("kind").and_then(Json::as_str), Some("drill_report"));
        assert_eq!(d.get("drills").unwrap().as_array().unwrap().len(), 2);
        let b = Json::parse(&bom_request(&facts(), &settings()).unwrap()).unwrap();
        assert_eq!(b.get("kind").and_then(Json::as_str), Some("bom_report"));
        let parts = b.get("components").unwrap().as_array().unwrap();
        assert_eq!(parts[0].get("ref").and_then(Json::as_str), Some("R1"));
        assert_eq!(
            parts[1].get("footprint").and_then(Json::as_str),
            Some("R0603")
        );
    }

    #[test]
    fn a_request_with_nothing_to_send_or_too_much_is_refused_with_its_code() {
        let empty = Facts::default();
        for result in [
            rules_request(&empty, &settings()),
            drill_request(&empty, &settings()),
            bom_request(&empty, &settings()),
            inspect_request(&[], &settings()),
        ] {
            assert_eq!(result.unwrap_err().code, NOTHING_TO_SEND);
        }
        let mut s = settings();
        s.max_items = 3;
        assert_eq!(
            rules_request(&facts(), &s).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        s.max_items = 1;
        assert_eq!(
            drill_request(&facts(), &s).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        assert_eq!(bom_request(&facts(), &s).unwrap_err().code, LIMIT_EXCEEDED);
        let rows = parse_measurements("a,1,0.1,1\nb,1,0.1,1\n").unwrap();
        assert_eq!(inspect_request(&rows, &s).unwrap_err().code, LIMIT_EXCEEDED);
    }

    #[test]
    fn a_name_with_quotes_and_new_lines_stays_inside_its_string() {
        let mut f = facts();
        f.tracks[0].name = clean_name("a\"b\\c\nd");
        let j = Json::parse(&rules_request(&f, &settings()).unwrap()).unwrap();
        let name = j
            .get("board")
            .unwrap()
            .get("tracks")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .get("name")
            .and_then(Json::as_str)
            .unwrap()
            .to_owned();
        assert_eq!(name, "a\"b\\c d");
    }

    #[test]
    fn measurements_are_read_from_lines_and_a_bad_line_is_refused_with_its_number() {
        let rows = parse_measurements(
            "# name, nominal, tolerance, value, unit\nwidth, 1.6, 0.1, 1.62, mm\n\nbow,0,0.5,0.7\n",
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].unit, "mm");
        assert_eq!(rows[1].unit, "");
        let text = inspect_request(&rows, &settings()).unwrap();
        let j = Json::parse(&text).unwrap();
        let list = j.get("measurements").unwrap().as_array().unwrap();
        assert_eq!(list[0].get("value"), Some(&Json::Num(1.62)));
        assert_eq!(list[1].get("name").and_then(Json::as_str), Some("bow"));
        for (text, line) in [
            ("a,1,2\n", "line 1"),
            ("ok,1,1,1\nb,x,1,1\n", "line 2"),
            ("a,1,-1,1\n", "line 1"),
            ("a,1,1,inf\n", "line 1"),
            ("a,1,1,1,123456789\n", "line 1"),
            ("a,1,1,1,mm,extra\n", "line 1"),
        ] {
            let e = parse_measurements(text).unwrap_err();
            assert_eq!(e.code, INPUT_UNREADABLE, "{text}");
            assert!(e.detail.contains(line), "{}", e.detail);
        }
        assert!(parse_measurements("# only a comment\n").unwrap().is_empty());
    }

    #[test]
    fn the_distance_between_pieces_is_exact_for_crossing_touching_and_parallel_pieces() {
        // crossing and touching pieces are at distance 0
        assert_eq!(piece_distance((0, 0), (10, 10), (0, 10), (10, 0)), 0.0);
        assert_eq!(piece_distance((0, 0), (10, 0), (10, 0), (10, 10)), 0.0);
        assert_eq!(piece_distance((0, 0), (10, 0), (5, 0), (15, 0)), 0.0);
        // parallel pieces, and a piece that ends next to the side of another
        assert_eq!(piece_distance((0, 0), (10, 0), (0, 4), (10, 4)), 4.0);
        assert_eq!(piece_distance((0, 0), (10, 0), (12, 0), (20, 0)), 2.0);
        assert_eq!(piece_distance((0, 0), (10, 0), (5, 3), (5, 9)), 3.0);
        // pieces of no length
        assert_eq!(piece_distance((0, 0), (0, 0), (3, 4), (3, 4)), 5.0);
        // the far corners of the range of a coordinate do not overflow
        let far = 1_073_741_823;
        assert_eq!(
            piece_distance((-far, -far), (-far, far), (far, -far), (far, far)),
            (2 * far) as f64
        );
    }

    #[test]
    fn the_distance_is_the_same_in_either_order_and_never_below_zero_for_generated_pieces() {
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 2_000_001) as i64 - 1_000_000
        };
        for _ in 0..300 {
            let (a, b, c, d) = (
                (next(), next()),
                (next(), next()),
                (next(), next()),
                (next(), next()),
            );
            let forward = piece_distance(a, b, c, d);
            assert!(forward >= 0.0);
            assert_eq!(forward, piece_distance(c, d, a, b));
            // the same pieces written the other way round differ at most by the last bits
            assert!((forward - piece_distance(b, a, d, c)).abs() < 1e-6);
        }
    }
}
