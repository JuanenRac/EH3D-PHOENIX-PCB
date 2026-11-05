// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on boards: a board with one or two tracks, or one zone, is built from the inputs of
//! the case and given to the rule check, to the length measure and to the zone fill. The shapes
//! are chosen so that the right answer follows from plane geometry.

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, Net, Segment, Track, Zone};
use eh3d_phoenix_lengths::measure::measure_nets;
use eh3d_phoenix_lengths::LengthSettings;
use eh3d_phoenix_model::{Nm, ObjectId, Point};
use eh3d_phoenix_rules::{check_board, NetClasses, RuleValues};
use eh3d_phoenix_zones::{fill_board, ZoneRules, ZoneSettings};

use crate::codes::{OracleError, CALC_FAILED};
use crate::compute::{calc, Inputs};

/// Nanometres in a millimetre.
const MM: f64 = 1_000_000.0;

fn model(error: impl std::fmt::Debug) -> OracleError {
    OracleError::new(CALC_FAILED, format!("the board cannot be built: {error:?}"))
}

fn id(n: u32) -> Result<Id, OracleError> {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).map_err(model)
}

fn nm(millimetres: f64) -> i64 {
    (millimetres * MM).round() as i64
}

fn point(x_nm: i64, y_nm: i64) -> Result<Point, OracleError> {
    Point::from_nm(x_nm, y_nm).map_err(model)
}

/// A square board with one copper layer (id 1) and the nets A (id 2) and B (id 3).
fn square_board(name: &str, side_mm: f64) -> Result<Board, OracleError> {
    let side = nm(side_mm);
    let mut board = Board {
        name: name.to_string(),
        outline: vec![
            point(0, 0)?,
            point(side, 0)?,
            point(side, side)?,
            point(0, side)?,
        ],
        ..Board::default()
    };
    board.layers.insert(
        id(1)?,
        Layer {
            name: "Top".to_string(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    board.nets.insert(
        id(2)?,
        Net {
            name: "A".to_string(),
        },
    );
    board.nets.insert(
        id(3)?,
        Net {
            name: "B".to_string(),
        },
    );
    Ok(board)
}

/// A straight track of a net on the layer 1, from `from` to `to` (nanometres).
fn add_track(
    board: &mut Board,
    track: u32,
    net: u32,
    width_nm: i64,
    from: (i64, i64),
    to: (i64, i64),
) -> Result<(), OracleError> {
    board.tracks.insert(
        id(track)?,
        Track {
            net: id(net)?,
            layer: id(1)?,
            width: Nm::new(width_nm).map_err(model)?,
            path: vec![Segment::Line {
                start: point(from.0, from.1)?,
                end: point(to.0, to.1)?,
            }],
        },
    );
    Ok(())
}

/// Two parallel tracks of different nets against a clearance rule.
fn clearance(i: &Inputs) -> Result<f64, OracleError> {
    let width = nm(i.num("width_mm")?);
    let pitch = nm(i.num("pitch_mm")?);
    let limit = nm(i.num("limit_mm")?);
    let (x0, x1, y) = (nm(2.0), nm(12.0), nm(5.0));
    let mut board = square_board("oracle-rules", 20.0)?;
    add_track(&mut board, 10, 2, width, (x0, y), (x1, y))?;
    add_track(&mut board, 11, 3, width, (x0, y + pitch), (x1, y + pitch))?;
    let rules = RuleValues {
        min_clearance: Nm::new(limit).map_err(model)?,
        ..RuleValues::default()
    };
    let report = check_board(&board, &rules);
    let mut hits = report
        .confirmed
        .iter()
        .filter(|f| f.rule == "copper_clearance");
    match i.text("quantity")? {
        "violations" => Ok(hits.count() as f64),
        "measured_mm" => hits
            .next()
            .and_then(|f| f.measured_nm)
            .map(|n| n as f64 / MM)
            .ok_or_else(|| OracleError::new(CALC_FAILED, "the check found no clearance finding")),
        other => Err(i.unknown("quantity", other)),
    }
}

/// One straight track, its copper length and its delay at a rate set in the settings.
fn track_length(i: &Inputs) -> Result<f64, OracleError> {
    let dx = nm(i.num("dx_mm")?);
    let dy = nm(i.num("dy_mm")?);
    let mut board = square_board("oracle-lengths", 100.0)?;
    add_track(&mut board, 10, 2, nm(0.2), (0, 0), (dx, dy))?;
    let settings = LengthSettings {
        default_delay_ps_per_mm: i.num("rate_ps_per_mm")?,
        ..LengthSettings::default()
    };
    let (nets, _findings) = measure_nets(&board, &settings, None).map_err(calc)?;
    let net = nets
        .first()
        .ok_or_else(|| OracleError::new(CALC_FAILED, "the measure found no net"))?;
    match i.text("quantity")? {
        "length_mm" => Ok(net.total_nm as f64 / MM),
        "delay_ps" => Ok(net.delay_fs as f64 / 1000.0),
        other => Err(i.unknown("quantity", other)),
    }
}

/// The copper area of a square zone, with or without a track of another net across it.
fn zone_area(i: &Inputs) -> Result<f64, OracleError> {
    let clearance = nm(i.num("clearance_mm")?);
    let track_width = i.num("track_width_mm")?;
    let (low, high) = (nm(5.0), nm(15.0));
    let mut board = square_board("oracle-zones", 20.0)?;
    board.zones.insert(
        id(20)?,
        Zone {
            net: id(2)?,
            layer: id(1)?,
            outline: vec![
                point(low, low)?,
                point(high, low)?,
                point(high, high)?,
                point(low, high)?,
            ],
            clearance: Nm::new(clearance).map_err(model)?,
        },
    );
    if track_width > 0.0 {
        let middle = nm(10.0);
        add_track(
            &mut board,
            10,
            3,
            nm(track_width),
            (nm(2.0), middle),
            (nm(18.0), middle),
        )?;
    }
    let rules = ZoneRules::from_rules(&board, &RuleValues::default(), &NetClasses::default());
    let settings = ZoneSettings {
        keep_unconnected_islands: true,
        ..ZoneSettings::default()
    };
    let report = fill_board(&board, &rules, &settings, None).map_err(calc)?;
    let area_nm2: f64 = report.stats.iter().map(|s| s.area_nm2 as f64).sum();
    Ok(area_nm2 / (MM * MM))
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "rules_clearance" => clearance(i)?,
        "lengths_track" => track_length(i)?,
        "zones_fill_area_mm2" => zone_area(i)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
