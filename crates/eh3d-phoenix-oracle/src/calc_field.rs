// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks on the field solver and on the heating of tracks: a capacitor of parallel plates and
//! a coaxial line drawn on the mesh, a stripline against the exact formula of zero thickness, and
//! the closed forms of the heating of a track.

use eh3d_phoenix_field2d::grid::{Grid, Rect, Wall, Walls};
use eh3d_phoenix_field2d::mesh::uniform_lines;
use eh3d_phoenix_field2d::{analyze_shape, Block, Electrode, Field2dSettings, Shape};
use eh3d_phoenix_stackup::heating::{convection_rise_k, trace_heating, HeatSettings, Placement};

use crate::codes::OracleError;
use crate::compute::{calc, Inputs};

/// Millimetres to metres.
fn mm(value: f64) -> f64 {
    value * 1.0e-3
}

/// The settings of the solver with a tolerance tight enough for a closed form.
fn tight() -> Field2dSettings {
    Field2dSettings {
        tolerance: 1.0e-12,
        ..Field2dSettings::default()
    }
}

/// The capacitance per metre of a capacitor of two plates with a dielectric between them, in
/// picofarad per metre: the bottom plate is the wall at zero, the top plate is a conductor, and the
/// sides have no flux through them.
fn parallel_plates(i: &Inputs) -> Result<f64, OracleError> {
    let er = i.num("er")?;
    let width = mm(i.num("width_mm")?);
    let gap = mm(i.num("gap_mm")?);
    let cells = i.num("cells")?.round().max(1.0) as usize;
    let walls = Walls {
        left: Wall::Symmetry,
        right: Wall::Symmetry,
        bottom: Wall::Ground,
        top: Wall::Symmetry,
    };
    let mut ys = uniform_lines(0.0, gap, cells);
    let top = gap * 1.1;
    ys.push(top);
    let mut grid =
        Grid::new(uniform_lines(0.0, width, cells), ys, walls, 1_000_000).map_err(calc)?;
    grid.paint_dielectric(Rect::new(0.0, width, 0.0, gap), er, 0.0);
    grid.paint_conductor(Rect::new(0.0, width, gap, top), 1);
    let solution = grid.solve(1, &tight()).map_err(calc)?;
    Ok(solution.c[0][0] * 1.0e12)
}

/// The capacitance per metre of a coaxial line of circular conductors, in picofarad per metre,
/// from a quarter of it drawn as steps of a square mesh with `steps` cells in the inner radius.
fn coax(i: &Inputs) -> Result<f64, OracleError> {
    let a = mm(i.num("a_mm")?);
    let b = mm(i.num("b_mm")?);
    let steps = i.num("steps")?.round().max(2.0);
    let step = a / steps;
    let cells = (b / step).ceil() as usize + 1;
    let lines = uniform_lines(0.0, step * cells as f64, cells);
    let walls = Walls {
        left: Wall::Symmetry,
        bottom: Wall::Symmetry,
        right: Wall::Ground,
        top: Wall::Ground,
    };
    let mut grid = Grid::new(lines.clone(), lines, walls, 1_000_000).map_err(calc)?;
    grid.paint_conductor_where(1, |x, y| x.hypot(y) < a);
    grid.paint_conductor_where(0, |x, y| x.hypot(y) > b);
    let solution = grid.solve(1, &tight()).map_err(calc)?;
    Ok(4.0 * solution.c[0][0] * 1.0e12)
}

/// The impedance of a centred stripline of the inputs, by the field solver, in ohm.
fn stripline(i: &Inputs) -> Result<f64, OracleError> {
    let w = mm(i.num("w_mm")?);
    let t = mm(i.num("t_mm")?);
    let b = mm(i.num("b_mm")?);
    let er = i.num("er")?;
    let half = w / 2.0 + 4.0 * b;
    let y0 = (b - t) / 2.0;
    let shape = Shape {
        domain: Rect::new(-half, half, 0.0, b),
        walls: Walls::GROUNDED,
        blocks: vec![Block {
            rect: Rect::new(-half, half, 0.0, b),
            eps_r: er,
            tan_d: 0.0,
        }],
        electrodes: vec![Electrode {
            rect: Rect::new(-w / 2.0, w / 2.0, y0, y0 + t),
            id: 1,
        }],
        grounds: Vec::new(),
        conductors: 1,
    };
    Ok(analyze_shape(&shape, &Field2dSettings::default(), 1.0)
        .map_err(calc)?
        .z0_ohm)
}

/// The rise of a track by the convection model, in kelvin.
fn convection(i: &Inputs) -> Result<f64, OracleError> {
    let settings = HeatSettings {
        ambient_c: i.num("ambient_c")?,
        convection_w_m2k: i.num("h_w_m2k")?,
        resistivity_ohm_m: i.num("resistivity_ohm_m")?,
        ..HeatSettings::default()
    };
    let width_nm = (i.num("width_mm")? * 1.0e6).round() as i64;
    let thickness_nm = (i.num("copper_um")? * 1.0e3).round() as i64;
    convection_rise_k(width_nm, thickness_nm, i.num("current_a")?, &settings).map_err(calc)
}

/// The resistance of a track at 20 degrees Celsius, in milliohm per millimetre.
fn resistance(i: &Inputs) -> Result<f64, OracleError> {
    let settings = HeatSettings {
        resistivity_ohm_m: i.num("resistivity_ohm_m")?,
        ..HeatSettings::default()
    };
    let width_nm = (i.num("width_mm")? * 1.0e6).round() as i64;
    let thickness_nm = (i.num("copper_um")? * 1.0e3).round() as i64;
    trace_heating(width_nm, thickness_nm, 1.0, Placement::External, &settings)
        .map(|h| h.resistance_mohm_per_mm)
        .map_err(calc)
}

/// Run a check of this family; `None` when the name is not one of them.
pub(crate) fn run(check: &str, i: &Inputs) -> Result<Option<f64>, OracleError> {
    let value = match check {
        "field2d_coax_pf_per_m" => coax(i)?,
        "field2d_parallel_plates_pf_per_m" => parallel_plates(i)?,
        "field2d_stripline_z0_ohm" => stripline(i)?,
        "heat_convection_rise_k" => convection(i)?,
        "heat_resistance_mohm_per_mm" => resistance(i)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
