// SPDX-License-Identifier: GPL-3.0-or-later
//! The path of a matching network on the Smith chart, as data and as a small SVG picture.
//!
//! The path starts at the load and goes through the elements from the load side to the source
//! side. Each step is a series reactance (a move along a circle of constant resistance), a
//! shunt susceptance (a move along a circle of constant conductance) or a length of
//! lossless line (a turn around the centre). The data are the impedance, the admittance and the
//! reflection coefficient after every step, and the reflection coefficients along each step. The
//! picture is made by this module (the printing crate draws boards, not charts): it is the same
//! text for the same input.

use crate::complex::Complex;
use crate::error::RfError;
use crate::filter::Position;
use crate::matching::ElementKind;
use crate::matchnet::Part;
use crate::network::{abcd_lossless_line, input_impedance, reflection_from_impedance};
use crate::report::engineering;

/// What one step of the path does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Move {
    /// Add a reactance in series, in ohm.
    Series(f64),
    /// Add a susceptance in shunt, in siemens.
    Shunt(f64),
    /// Go through a lossless line of characteristic impedance `z0_ohm` and electrical length
    /// `degrees`, towards the source.
    Line {
        /// The characteristic impedance of the line in ohm.
        z0_ohm: f64,
        /// The electrical length in degrees.
        degrees: f64,
    },
}

/// One step of the path with a name.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// A short name for the table and the picture.
    pub label: String,
    /// What it does.
    pub movement: Move,
}

/// A point of the path.
#[derive(Clone, Debug, PartialEq)]
pub struct TracePoint {
    /// The name of the step that ends here, or `load` for the start.
    pub label: String,
    /// The impedance in ohm.
    pub z: Complex,
    /// The admittance in siemens.
    pub y: Complex,
    /// The reflection coefficient against the impedance of the chart.
    pub gamma: Complex,
}

/// The path on the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct SmithTrace {
    /// The impedance in ohm that the chart is normalised to.
    pub chart_z0_ohm: f64,
    /// The start and the end of every step, so one more than the steps.
    pub points: Vec<TracePoint>,
    /// The reflection coefficients along every step, from its start to its end, one list per step.
    pub paths: Vec<Vec<Complex>>,
}

/// The steps of a ladder, from the load side to the source side, at a frequency in Hz.
pub fn steps_from_parts(parts: &[Part], freq_hz: f64) -> Result<Vec<Step>, RfError> {
    let mut steps = Vec::with_capacity(parts.len());
    for part in parts.iter().rev() {
        let value = part.reactance_or_susceptance(freq_hz)?;
        let (word, unit) = match part.element.kind {
            ElementKind::Inductor => ("inductor", "H"),
            ElementKind::Capacitor => ("capacitor", "F"),
        };
        let place = match part.position {
            Position::Series => "series",
            Position::Shunt => "shunt",
        };
        let movement = match part.position {
            Position::Series => Move::Series(value),
            Position::Shunt => Move::Shunt(value),
        };
        steps.push(Step {
            label: format!("{place} {word} {}", engineering(part.element.value, unit)),
            movement,
        });
    }
    Ok(steps)
}

fn apply(z: Complex, movement: Move, fraction: f64) -> Result<Complex, RfError> {
    match movement {
        Move::Series(x) => Ok(z + Complex::new(0.0, x * fraction)),
        Move::Shunt(b) => (z.recip()? + Complex::new(0.0, b * fraction)).recip(),
        Move::Line { z0_ohm, degrees } => {
            let line = abcd_lossless_line(z0_ohm, (degrees * fraction).to_radians())?;
            input_impedance(&line, z)
        }
    }
}

/// The path of a load through steps on a chart normalised to `chart_z0_ohm`, with `samples`
/// intervals along each step (so `samples + 1` points).
pub fn trace(
    start: Complex,
    steps: &[Step],
    chart_z0_ohm: f64,
    samples: usize,
) -> Result<SmithTrace, RfError> {
    crate::error::positive("chart impedance", chart_z0_ohm)?;
    if samples == 0 {
        return Err(RfError::InvalidNumber {
            field: "samples".to_string(),
            reason: "must be at least 1".to_string(),
        });
    }
    let point = |label: &str, z: Complex| -> Result<TracePoint, RfError> {
        Ok(TracePoint {
            label: label.to_string(),
            z,
            y: z.recip()?,
            gamma: reflection_from_impedance(z, chart_z0_ohm)?,
        })
    };
    let mut points = vec![point("load", start)?];
    let mut paths = Vec::with_capacity(steps.len());
    let mut current = start;
    for step in steps {
        let mut along = Vec::with_capacity(samples + 1);
        for k in 0..=samples {
            let z = apply(current, step.movement, k as f64 / samples as f64)?;
            along.push(reflection_from_impedance(z, chart_z0_ohm)?);
        }
        current = apply(current, step.movement, 1.0)?;
        points.push(point(&step.label, current)?);
        paths.push(along);
    }
    Ok(SmithTrace {
        chart_z0_ohm,
        points,
        paths,
    })
}

const CENTRE: f64 = 300.0;
const RADIUS: f64 = 250.0;
const GRID: [f64; 5] = [0.2, 0.5, 1.0, 2.0, 5.0];

fn xy(gamma: Complex) -> (f64, f64) {
    (CENTRE + RADIUS * gamma.re, CENTRE - RADIUS * gamma.im)
}

fn text_escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

fn polyline(points: &[(f64, f64)], stroke: &str, width: f64) -> String {
    let list: Vec<String> = points
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect();
    format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\"/>\n",
        list.join(" ")
    )
}

/// The points of the arc of the circle of constant reactance `x` (normalised) that is inside the
/// chart, from the point of open circuit to the point where it meets the edge.
fn reactance_arc(x: f64) -> Vec<(f64, f64)> {
    let centre = (1.0, 1.0 / x);
    let radius = 1.0 / x.abs();
    let edge = Complex::new((x * x - 1.0) / (x * x + 1.0), 2.0 * x / (x * x + 1.0));
    let start_angle = (-centre.1).atan2(1.0 - centre.0);
    let end_angle = (edge.im - centre.1).atan2(edge.re - centre.0);
    let mut delta = end_angle - start_angle;
    while delta > std::f64::consts::PI {
        delta -= 2.0 * std::f64::consts::PI;
    }
    while delta <= -std::f64::consts::PI {
        delta += 2.0 * std::f64::consts::PI;
    }
    (0..=32)
        .map(|k| {
            let angle = start_angle + delta * k as f64 / 32.0;
            xy(Complex::new(
                centre.0 + radius * angle.cos(),
                centre.1 + radius * angle.sin(),
            ))
        })
        .collect()
}

/// The picture of a path as an SVG text: the chart with circles of constant resistance and
/// reactance, the path, a marker at every point and a legend of the steps. It is the same text
/// for the same input. The labels are the English names of the steps.
pub fn smith_svg(trace: &SmithTrace, title: &str) -> String {
    let rows = trace.points.len();
    let height = 610 + 16 * rows;
    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 600 {height}\" width=\"600\" height=\"{height}\" font-family=\"sans-serif\">\n"
    ));
    svg.push_str(&format!("<title>{}</title>\n", text_escape(title)));
    svg.push_str(&format!(
        "<rect width=\"600\" height=\"{height}\" fill=\"#ffffff\"/>\n"
    ));
    svg.push_str(&format!(
        "<text x=\"300\" y=\"24\" font-size=\"16\" text-anchor=\"middle\" fill=\"#222222\">{}</text>\n",
        text_escape(title)
    ));
    for r in GRID {
        let centre = r / (1.0 + r);
        let radius = 1.0 / (1.0 + r);
        svg.push_str(&format!(
            "<circle cx=\"{:.2}\" cy=\"{CENTRE:.2}\" r=\"{:.2}\" fill=\"none\" stroke=\"#bbbbbb\" stroke-width=\"1\"/>\n",
            CENTRE + RADIUS * centre,
            RADIUS * radius
        ));
    }
    for x in GRID {
        for sign in [1.0, -1.0] {
            svg.push_str(&polyline(&reactance_arc(sign * x), "#bbbbbb", 1.0));
        }
    }
    svg.push_str(&format!(
        "<line x1=\"{:.2}\" y1=\"{CENTRE:.2}\" x2=\"{:.2}\" y2=\"{CENTRE:.2}\" stroke=\"#999999\" stroke-width=\"1\"/>\n",
        CENTRE - RADIUS,
        CENTRE + RADIUS
    ));
    svg.push_str(&format!(
        "<circle cx=\"{CENTRE:.2}\" cy=\"{CENTRE:.2}\" r=\"{RADIUS:.2}\" fill=\"none\" stroke=\"#444444\" stroke-width=\"1.5\"/>\n"
    ));
    for path in &trace.paths {
        let points: Vec<(f64, f64)> = path.iter().map(|g| xy(*g)).collect();
        svg.push_str(&polyline(&points, "#c0392b", 2.0));
    }
    for (index, point) in trace.points.iter().enumerate() {
        let (x, y) = xy(point.gamma);
        svg.push_str(&format!(
            "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"4\" fill=\"#1f4e79\"/>\n<text x=\"{:.2}\" y=\"{:.2}\" font-size=\"12\" fill=\"#1f4e79\">{index}</text>\n",
            x + 6.0,
            y - 6.0
        ));
    }
    for (index, point) in trace.points.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-size=\"12\" fill=\"#222222\">{index}: {} (z = {:.3} {}j{:.3} ohm)</text>\n",
            610 + 16 * index,
            text_escape(&point.label),
            point.z.re,
            if point.z.im < 0.0 { "-" } else { "+" },
            point.z.im.abs()
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::l_match;
    use crate::matchnet::{l_parts, pi_match, PassKind};
    use crate::settings::RfSettings;

    #[test]
    fn a_series_step_moves_along_a_circle_of_constant_resistance() {
        let steps = vec![Step {
            label: "x".to_string(),
            movement: Move::Series(30.0),
        }];
        let t = trace(Complex::new(50.0, 0.0), &steps, 50.0, 8).unwrap();
        assert_eq!(t.points.len(), 2);
        assert_eq!(t.paths.len(), 1);
        assert_eq!(t.paths[0].len(), 9);
        for g in &t.paths[0] {
            // the resistance stays at 50 ohm along the whole step
            let z = crate::network::impedance_from_reflection(*g, 50.0).unwrap();
            assert!((z.re - 50.0).abs() < 1e-9, "{z:?}");
        }
        assert!((t.points[1].z.im - 30.0).abs() < 1e-12);
    }

    #[test]
    fn a_shunt_step_moves_along_a_circle_of_constant_conductance() {
        let steps = vec![Step {
            label: "b".to_string(),
            movement: Move::Shunt(0.01),
        }];
        let t = trace(Complex::new(100.0, 50.0), &steps, 50.0, 6).unwrap();
        let g0 = Complex::new(100.0, 50.0).recip().unwrap().re;
        for gamma in &t.paths[0] {
            let z = crate::network::impedance_from_reflection(*gamma, 50.0).unwrap();
            assert!((z.recip().unwrap().re - g0).abs() < 1e-12);
        }
        assert!(
            (t.points[1].y.im - (Complex::new(100.0, 50.0).recip().unwrap().im + 0.01)).abs()
                < 1e-12
        );
    }

    #[test]
    fn a_line_step_turns_around_the_centre_by_twice_its_length() {
        let steps = vec![Step {
            label: "line".to_string(),
            movement: Move::Line {
                z0_ohm: 50.0,
                degrees: 45.0,
            },
        }];
        let t = trace(Complex::new(100.0, 0.0), &steps, 50.0, 4).unwrap();
        let first = t.paths[0][0];
        let last = t.paths[0][4];
        assert!((first.abs() - last.abs()).abs() < 1e-12);
        // 45 degrees of line turn the coefficient by 90 degrees
        let turn = (last * first.conj()).arg_deg();
        assert!((turn + 90.0).abs() < 1e-9, "{turn}");
    }

    #[test]
    fn the_path_of_a_matching_network_ends_at_the_conjugate_of_the_source() {
        let s = RfSettings::default();
        let source = Complex::real(50.0);
        let load = Complex::real(100.0);
        let found = l_match(source, load, 1e8, &s).unwrap();
        for m in found {
            let steps = steps_from_parts(&l_parts(&m), 1e8).unwrap();
            assert_eq!(steps.len(), 2);
            let t = trace(load, &steps, 50.0, 12).unwrap();
            let end = t.points[2].z;
            assert!((end - source).abs() < 1e-9, "{end:?}");
            assert!(t.points[2].gamma.abs() < 1e-9);
        }
        let net = pi_match(source, load, 1e8, 5.0, PassKind::LowPass, &s).unwrap();
        let steps = steps_from_parts(&net.parts, 1e8).unwrap();
        assert_eq!(steps.len(), 3);
        let t = trace(load, &steps, 50.0, 12).unwrap();
        assert!((t.points[3].z - source).abs() < 1e-9);
        // the first step goes from the load side
        assert!(steps[0].label.starts_with("shunt"));
    }

    #[test]
    fn the_picture_is_the_same_text_every_time_and_has_the_parts() {
        let steps = vec![
            Step {
                label: "series inductor 79.6 nH".to_string(),
                movement: Move::Series(50.0),
            },
            Step {
                label: "shunt <capacitor> & more".to_string(),
                movement: Move::Shunt(0.005),
            },
        ];
        let t = trace(Complex::new(100.0, 0.0), &steps, 50.0, 10).unwrap();
        let a = smith_svg(&t, "A \"title\"");
        let b = smith_svg(&t, "A \"title\"");
        assert_eq!(a, b);
        assert!(a.starts_with("<svg "));
        assert!(a.trim_end().ends_with("</svg>"));
        assert_eq!(a.matches("<polyline").count(), 10 + 2);
        assert!(a.contains("&lt;capacitor&gt; &amp; more"));
        assert!(a.contains("A &quot;title&quot;"));
        assert!(!a.contains("NaN") && !a.contains("inf"));
        // every opening tag of the pieces we write is closed
        assert_eq!(a.matches("<svg").count(), 1);
        assert_eq!(a.matches("</svg>").count(), 1);
        assert_eq!(a.matches("<text").count(), a.matches("</text>").count());
    }

    #[test]
    fn bad_inputs_are_refused() {
        let steps: Vec<Step> = Vec::new();
        assert!(trace(Complex::real(50.0), &steps, 0.0, 4).is_err());
        assert!(trace(Complex::real(50.0), &steps, 50.0, 0).is_err());
        // a short circuit has no admittance
        assert!(trace(Complex::ZERO, &steps, 50.0, 4).is_err());
        let shunt_on_short = vec![Step {
            label: "b".to_string(),
            movement: Move::Shunt(0.1),
        }];
        assert!(trace(Complex::real(50.0), &shunt_on_short, 50.0, 4).is_ok());
    }
}
