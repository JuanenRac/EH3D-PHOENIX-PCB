// SPDX-License-Identifier: GPL-3.0-or-later
//! Prisms (a polygon of the board plane taken from one height to another) written as OBJ and as
//! STEP (ISO 10303-21, a boundary representation with plane faces and straight edges).
//!
//! The text is a pure function of the prisms: no date, no host, no program version. Lengths are
//! millimetres with up to six decimals, exact for the points; the lengths of slanted edges and
//! the directions are floating-point numbers written with a fixed number of decimals. Entities
//! are numbered in the order they are made. The STEP file has only been checked for its
//! structure (header, entity references, counts); no CAD program has opened it.

use std::fmt::Write as _;

use crate::geom::{counter_clockwise, P};

/// A polygon of the board plane taken from one height to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prism {
    /// The name, made of letters, digits, `_`, `.` and `-` (any other character becomes `_`).
    pub name: String,
    /// The corners of the base, in nanometres.
    pub base: Vec<P>,
    /// The lower height, in nanometres.
    pub z_low: i64,
    /// The upper height, in nanometres.
    pub z_high: i64,
}

/// A name that is safe in OBJ and STEP.
pub fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "_".to_owned()
    } else {
        cleaned
    }
}

/// Nanometres as millimetres with six decimals.
fn mm6(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
}

/// Nanometres as a STEP real in millimetres: the digits they have, at least the point.
fn step_real(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    let digits = format!("{:06}", a % 1_000_000);
    format!("{sign}{}.{}", a / 1_000_000, digits.trim_end_matches('0'))
}

/// A length in nanometres given as a number, as a STEP real in millimetres.
fn step_length(nm: f64) -> String {
    format!("{:.6}", nm / 1_000_000.0)
}

/// A unit direction as STEP reals; a component too small to print becomes 0.
fn direction(dx: f64, dy: f64, dz: f64) -> String {
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    let part = |v: f64| {
        let r = v / length;
        if r.abs() < 5e-10 {
            "0.".to_owned()
        } else {
            format!("{r:.9}")
        }
    };
    format!("{},{},{}", part(dx), part(dy), part(dz))
}

/// The prisms as an OBJ file: one object each, the vertices in millimetres, the faces as
/// polygons seen from outside.
pub fn prisms_obj(prisms: &[Prism]) -> String {
    let mut out =
        String::from("# EH3D-PHOENIX-PCB assembly: boxes and a board prism, in millimetres\n");
    let mut base_index = 0usize;
    for prism in prisms {
        let base = counter_clockwise(&prism.base);
        let n = base.len();
        if n < 3 {
            continue;
        }
        let _ = writeln!(out, "o {}", safe_name(&prism.name));
        for z in [prism.z_low, prism.z_high] {
            for p in &base {
                let _ = writeln!(out, "v {} {} {}", mm6(p.0), mm6(p.1), mm6(z));
            }
        }
        let at = |i: usize| base_index + i + 1;
        let bottom: Vec<String> = (0..n).rev().map(|i| at(i).to_string()).collect();
        let _ = writeln!(out, "f {}", bottom.join(" "));
        let top: Vec<String> = (0..n).map(|i| at(n + i).to_string()).collect();
        let _ = writeln!(out, "f {}", top.join(" "));
        for i in 0..n {
            let j = (i + 1) % n;
            let _ = writeln!(out, "f {} {} {} {}", at(i), at(j), at(n + j), at(n + i));
        }
        base_index += 2 * n;
    }
    out
}

#[derive(Default)]
struct Builder {
    lines: Vec<String>,
}

impl Builder {
    fn add(&mut self, body: String) -> usize {
        self.lines.push(body);
        self.lines.len()
    }

    fn point(&mut self, x: i64, y: i64, z: i64) -> usize {
        self.add(format!(
            "CARTESIAN_POINT('',({},{},{}))",
            step_real(x),
            step_real(y),
            step_real(z)
        ))
    }

    fn axis(
        &mut self,
        origin: (i64, i64, i64),
        normal: (f64, f64, f64),
        reference: (f64, f64, f64),
    ) -> usize {
        let p = self.point(origin.0, origin.1, origin.2);
        let n = self.add(format!(
            "DIRECTION('',({}))",
            direction(normal.0, normal.1, normal.2)
        ));
        let r = self.add(format!(
            "DIRECTION('',({}))",
            direction(reference.0, reference.1, reference.2)
        ));
        self.add(format!("AXIS2_PLACEMENT_3D('',#{p},#{n},#{r})"))
    }

    /// An edge from one vertex to another along a straight line.
    fn edge(&mut self, from: (usize, (i64, i64, i64)), to: (usize, (i64, i64, i64))) -> usize {
        let (dx, dy, dz) = (
            (to.1 .0 - from.1 .0) as f64,
            (to.1 .1 - from.1 .1) as f64,
            (to.1 .2 - from.1 .2) as f64,
        );
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let start = self.point(from.1 .0, from.1 .1, from.1 .2);
        let d = self.add(format!("DIRECTION('',({}))", direction(dx, dy, dz)));
        let vector = self.add(format!("VECTOR('',#{d},{})", step_length(length)));
        let line = self.add(format!("LINE('',#{start},#{vector})"));
        self.add(format!("EDGE_CURVE('',#{},#{},#{line},.T.)", from.0, to.0))
    }

    /// A closed edge that goes once round a circle about the axis through `centre`, from the
    /// vertex `vertex` that lies on the circle at its angle 0.
    fn circle_edge(&mut self, vertex: usize, centre: (i64, i64, i64), radius: i64) -> usize {
        let placement = self.axis(centre, (0.0, 0.0, 1.0), (1.0, 0.0, 0.0));
        let circle = self.add(format!("CIRCLE('',#{placement},{})", step_real(radius)));
        self.add(format!("EDGE_CURVE('',#{vertex},#{vertex},#{circle},.T.)"))
    }

    /// An edge loop of oriented edges.
    fn edge_loop(&mut self, sides: &[(usize, bool)]) -> usize {
        let oriented: Vec<String> = sides
            .iter()
            .map(|(edge, forward)| {
                let id = self.add(format!(
                    "ORIENTED_EDGE('',*,*,#{edge},{})",
                    if *forward { ".T." } else { ".F." }
                ));
                format!("#{id}")
            })
            .collect();
        self.add(format!("EDGE_LOOP('',({}))", oriented.join(",")))
    }

    /// A face on a surface, with an outer loop and the loops of its holes.
    fn face_on(
        &mut self,
        outer: &[(usize, bool)],
        holes: &[Vec<(usize, bool)>],
        surface: usize,
        same_sense: bool,
    ) -> usize {
        let outer_loop = self.edge_loop(outer);
        let mut bounds = vec![self.add(format!("FACE_OUTER_BOUND('',#{outer_loop},.T.)"))];
        for hole in holes {
            let hole_loop = self.edge_loop(hole);
            bounds.push(self.add(format!("FACE_BOUND('',#{hole_loop},.T.)")));
        }
        let list: Vec<String> = bounds.iter().map(|b| format!("#{b}")).collect();
        self.add(format!(
            "ADVANCED_FACE('',({}),#{surface},{})",
            list.join(","),
            if same_sense { ".T." } else { ".F." }
        ))
    }

    fn face(&mut self, sides: &[(usize, bool)], placement: usize) -> usize {
        let oriented: Vec<String> = sides
            .iter()
            .map(|(edge, forward)| {
                let id = self.add(format!(
                    "ORIENTED_EDGE('',*,*,#{edge},{})",
                    if *forward { ".T." } else { ".F." }
                ));
                format!("#{id}")
            })
            .collect();
        let edge_loop = self.add(format!("EDGE_LOOP('',({}))", oriented.join(",")));
        let bound = self.add(format!("FACE_OUTER_BOUND('',#{edge_loop},.T.)"));
        let plane = self.add(format!("PLANE('',#{placement})"));
        self.add(format!("ADVANCED_FACE('',(#{bound}),#{plane},.T.)"))
    }
}

/// The prisms as a STEP file with one solid for each.
pub fn prisms_step(prisms: &[Prism]) -> String {
    let mut b = Builder::default();
    let mut solids: Vec<usize> = Vec::new();
    for prism in prisms {
        let base = counter_clockwise(&prism.base);
        let n = base.len();
        if n < 3 {
            continue;
        }
        let low = |i: usize| (base[i].0, base[i].1, prism.z_low);
        let high = |i: usize| (base[i].0, base[i].1, prism.z_high);
        let bottom_vertices: Vec<usize> = (0..n)
            .map(|i| {
                let p = b.point(low(i).0, low(i).1, low(i).2);
                b.add(format!("VERTEX_POINT('',#{p})"))
            })
            .collect();
        let top_vertices: Vec<usize> = (0..n)
            .map(|i| {
                let p = b.point(high(i).0, high(i).1, high(i).2);
                b.add(format!("VERTEX_POINT('',#{p})"))
            })
            .collect();
        let mut bottom_edges = Vec::new();
        let mut top_edges = Vec::new();
        let mut upright_edges = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            bottom_edges.push(b.edge((bottom_vertices[i], low(i)), (bottom_vertices[j], low(j))));
            top_edges.push(b.edge((top_vertices[i], high(i)), (top_vertices[j], high(j))));
            upright_edges.push(b.edge((bottom_vertices[i], low(i)), (top_vertices[i], high(i))));
        }
        let mut faces = Vec::new();
        let down = b.axis(low(0), (0.0, 0.0, -1.0), (1.0, 0.0, 0.0));
        let reversed: Vec<(usize, bool)> = (0..n).rev().map(|i| (bottom_edges[i], false)).collect();
        faces.push(b.face(&reversed, down));
        let up = b.axis(high(0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0));
        let forward: Vec<(usize, bool)> = (0..n).map(|i| (top_edges[i], true)).collect();
        faces.push(b.face(&forward, up));
        for i in 0..n {
            let j = (i + 1) % n;
            let (dx, dy) = (
                (base[j].0 - base[i].0) as f64,
                (base[j].1 - base[i].1) as f64,
            );
            let placement = b.axis(low(i), (dy, -dx, 0.0), (dx, dy, 0.0));
            let sides = [
                (bottom_edges[i], true),
                (upright_edges[j], true),
                (top_edges[i], false),
                (upright_edges[i], false),
            ];
            faces.push(b.face(&sides, placement));
        }
        let list: Vec<String> = faces.iter().map(|f| format!("#{f}")).collect();
        let shell = b.add(format!("CLOSED_SHELL('',({}))", list.join(",")));
        solids.push(b.add(format!(
            "MANIFOLD_SOLID_BREP('{}',#{shell})",
            safe_name(&prism.name)
        )));
    }

    finish(b, &solids)
}

/// The file around the solids that a builder made: the units, the context, the product and the
/// header.
fn finish(mut b: Builder, solids: &[usize]) -> String {
    let origin = b.axis((0, 0, 0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0));
    let length_unit = b.add("(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))".to_owned());
    let angle = b.add("(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))".to_owned());
    let solid_angle = b.add("(NAMED_UNIT(*) SOLID_ANGLE_UNIT() SI_UNIT($,.STERADIAN.))".to_owned());
    let uncertainty = b.add(format!(
        "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-06),#{length_unit},'distance_accuracy_value','')"
    ));
    let context = b.add(format!(
        "(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty})) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length_unit},#{angle},#{solid_angle})) REPRESENTATION_CONTEXT('assembly','3D'))"
    ));
    let mut items = format!("#{origin}");
    for solid in solids {
        items.push_str(&format!(",#{solid}"));
    }
    let representation = b.add(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('assembly',({items}),#{context})"
    ));
    let app = b.add(
        "APPLICATION_CONTEXT('core data for automotive mechanical design processes')".to_owned(),
    );
    b.add(format!(
        "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,#{app})"
    ));
    let product_context = b.add(format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
    let product = b.add(format!(
        "PRODUCT('assembly','assembly','',(#{product_context}))"
    ));
    b.add(format!(
        "PRODUCT_RELATED_PRODUCT_CATEGORY('part','',(#{product}))"
    ));
    let formation = b.add(format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"));
    let definition_context = b.add(format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{app},'design')"
    ));
    let definition = b.add(format!(
        "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
    ));
    let shape = b.add(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
    b.add(format!(
        "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
    ));

    let mut out = String::new();
    out.push_str("ISO-10303-21;\nHEADER;\n");
    out.push_str("FILE_DESCRIPTION(('EH3D-PHOENIX-PCB assembly'),'2;1');\n");
    out.push_str("FILE_NAME('assembly.step','',(''),(''),'EH3D-PHOENIX-PCB','','');\n");
    out.push_str("FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\n");
    out.push_str("ENDSEC;\nDATA;\n");
    for (i, body) in b.lines.iter().enumerate() {
        let _ = writeln!(out, "#{}={};", i + 1, body);
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out
}

/// A hole through the board: its centre and its diameter, in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hole {
    /// The centre.
    pub at: P,
    /// The diameter.
    pub diameter_nm: i64,
}

/// The board as one solid in a STEP file: the prism of the outline from height 0 to
/// `thickness_nm`, with a cylindrical hole through it for each of `holes` (circles about the
/// axis of the hole, a cylinder face for the wall, both circles as the inner loops of the top and
/// the bottom face). The holes must lie inside the outline and clear of each other and of its
/// edges; `usable_holes` of the enclosure module picks those. The radius is half the diameter,
/// rounded down to the nanometre.
pub fn board_step(outline: &[P], thickness_nm: i64, holes: &[Hole]) -> String {
    let base = counter_clockwise(outline);
    let n = base.len();
    let mut b = Builder::default();
    if n < 3 || thickness_nm <= 0 {
        return finish(b, &[]);
    }
    let low = |i: usize| (base[i].0, base[i].1, 0);
    let high = |i: usize| (base[i].0, base[i].1, thickness_nm);
    let bottom_vertices: Vec<usize> = (0..n)
        .map(|i| {
            let p = b.point(low(i).0, low(i).1, low(i).2);
            b.add(format!("VERTEX_POINT('',#{p})"))
        })
        .collect();
    let top_vertices: Vec<usize> = (0..n)
        .map(|i| {
            let p = b.point(high(i).0, high(i).1, high(i).2);
            b.add(format!("VERTEX_POINT('',#{p})"))
        })
        .collect();
    let mut bottom_edges = Vec::new();
    let mut top_edges = Vec::new();
    let mut upright_edges = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        bottom_edges.push(b.edge((bottom_vertices[i], low(i)), (bottom_vertices[j], low(j))));
        top_edges.push(b.edge((top_vertices[i], high(i)), (top_vertices[j], high(j))));
        upright_edges.push(b.edge((bottom_vertices[i], low(i)), (top_vertices[i], high(i))));
    }

    // the holes: a circle on the bottom, a circle on the top and the line along the wall
    let mut bottom_loops: Vec<Vec<(usize, bool)>> = Vec::new();
    let mut top_loops: Vec<Vec<(usize, bool)>> = Vec::new();
    let mut walls: Vec<usize> = Vec::new();
    for hole in holes {
        let radius = hole.diameter_nm / 2;
        let (cx, cy) = hole.at;
        let at_bottom = (cx + radius, cy, 0);
        let at_top = (cx + radius, cy, thickness_nm);
        let p_bottom = b.point(at_bottom.0, at_bottom.1, at_bottom.2);
        let v_bottom = b.add(format!("VERTEX_POINT('',#{p_bottom})"));
        let p_top = b.point(at_top.0, at_top.1, at_top.2);
        let v_top = b.add(format!("VERTEX_POINT('',#{p_top})"));
        let circle_bottom = b.circle_edge(v_bottom, (cx, cy, 0), radius);
        let circle_top = b.circle_edge(v_top, (cx, cy, thickness_nm), radius);
        let seam = b.edge((v_bottom, at_bottom), (v_top, at_top));
        bottom_loops.push(vec![(circle_bottom, true)]);
        top_loops.push(vec![(circle_top, false)]);
        // the wall of the hole looks into the hole: its face is the cylinder turned inside out,
        // walked clockwise about the axis on the bottom, up the seam, anticlockwise on the top
        // and down the seam
        let axis = b.axis((cx, cy, 0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0));
        let cylinder = b.add(format!(
            "CYLINDRICAL_SURFACE('',#{axis},{})",
            step_real(radius)
        ));
        let sides = [
            (circle_bottom, false),
            (seam, true),
            (circle_top, true),
            (seam, false),
        ];
        walls.push(b.face_on(&sides, &[], cylinder, false));
    }

    let mut faces = Vec::new();
    let down = b.axis(low(0), (0.0, 0.0, -1.0), (1.0, 0.0, 0.0));
    let down_plane = b.add(format!("PLANE('',#{down})"));
    let reversed: Vec<(usize, bool)> = (0..n).rev().map(|i| (bottom_edges[i], false)).collect();
    faces.push(b.face_on(&reversed, &bottom_loops, down_plane, true));
    let up = b.axis(high(0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0));
    let up_plane = b.add(format!("PLANE('',#{up})"));
    let forward: Vec<(usize, bool)> = (0..n).map(|i| (top_edges[i], true)).collect();
    faces.push(b.face_on(&forward, &top_loops, up_plane, true));
    for i in 0..n {
        let j = (i + 1) % n;
        let (dx, dy) = (
            (base[j].0 - base[i].0) as f64,
            (base[j].1 - base[i].1) as f64,
        );
        let placement = b.axis(low(i), (dy, -dx, 0.0), (dx, dy, 0.0));
        let sides = [
            (bottom_edges[i], true),
            (upright_edges[j], true),
            (top_edges[i], false),
            (upright_edges[i], false),
        ];
        faces.push(b.face(&sides, placement));
    }
    faces.extend(walls);
    let list: Vec<String> = faces.iter().map(|f| format!("#{f}")).collect();
    let shell = b.add(format!("CLOSED_SHELL('',({}))", list.join(",")));
    let solid = b.add(format!("MANIFOLD_SOLID_BREP('board',#{shell})"));
    finish(b, &[solid])
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    fn square(name: &str, size: i64, low: i64, high: i64) -> Prism {
        Prism {
            name: name.to_owned(),
            base: vec![(0, 0), (size, 0), (size, size), (0, size)],
            z_low: low,
            z_high: high,
        }
    }

    #[test]
    fn names_are_made_safe() {
        assert_eq!(safe_name("R1"), "R1");
        assert_eq!(safe_name("a b'c\n"), "a_b_c_");
        assert_eq!(safe_name(""), "_");
        assert_eq!(safe_name("é"), "_");
    }

    #[test]
    fn reals_keep_the_digits_they_have() {
        assert_eq!(step_real(0), "0.");
        assert_eq!(step_real(MM), "1.");
        assert_eq!(step_real(1_500_000), "1.5");
        assert_eq!(step_real(-1_600_001), "-1.600001");
        assert_eq!(mm6(-500), "-0.000500");
        assert_eq!(step_length(2_500_000.0), "2.500000");
    }

    #[test]
    fn a_direction_has_unit_length_and_no_negative_zero() {
        assert_eq!(direction(0.0, 0.0, -2.0), "0.,0.,-1.000000000");
        assert_eq!(direction(3.0, -0.0, 4.0), "0.600000000,0.,0.800000000");
        assert_eq!(direction(1.0, 1e-12, 0.0), "1.000000000,0.,0.");
    }

    #[test]
    fn the_obj_has_the_vertices_and_the_faces_of_each_prism() {
        let obj = prisms_obj(&[
            square("board", 10 * MM, 0, 1_600_000),
            square("R 1", 2 * MM, 1_600_000, 2_600_000),
        ]);
        let lines: Vec<&str> = obj.lines().collect();
        assert!(lines[0].starts_with("# "));
        assert_eq!(lines[1], "o board");
        assert_eq!(lines[2], "v 0.000000 0.000000 0.000000");
        assert_eq!(lines[5], "v 0.000000 10.000000 0.000000");
        assert_eq!(lines[6], "v 0.000000 0.000000 1.600000");
        assert_eq!(lines[10], "f 4 3 2 1");
        assert_eq!(lines[11], "f 5 6 7 8");
        assert_eq!(lines[12], "f 1 2 6 5");
        assert_eq!(lines[15], "f 4 1 5 8");
        assert_eq!(lines[16], "o R_1");
        // the second prism counts its vertices after the first one's 8
        assert_eq!(lines[25], "f 12 11 10 9");
        assert_eq!(obj.matches("\nv ").count(), 16);
        assert_eq!(obj.matches("\nf ").count(), 12);
    }

    #[test]
    fn a_polygon_that_runs_clockwise_is_written_counter_clockwise() {
        let mut clockwise = square("c", MM, 0, MM);
        clockwise.base.reverse();
        assert_eq!(
            prisms_obj(&[clockwise]),
            prisms_obj(&[square("c", MM, 0, MM)])
        );
        let flat = Prism {
            name: "x".to_owned(),
            base: vec![(0, 0), (MM, 0)],
            z_low: 0,
            z_high: MM,
        };
        assert_eq!(prisms_obj(std::slice::from_ref(&flat)), prisms_obj(&[]));
        assert_eq!(prisms_step(&[flat]), prisms_step(&[]));
    }

    #[test]
    fn the_step_is_the_same_every_time_with_every_reference_defined() {
        let prisms = [
            square("board", 10 * MM, 0, 1_600_000),
            square("R1", 2 * MM, 1_600_000, 2_600_000),
        ];
        let a = prisms_step(&prisms);
        assert_eq!(a, prisms_step(&prisms));
        assert!(a.starts_with("ISO-10303-21;\nHEADER;\n"));
        assert!(a.ends_with("END-ISO-10303-21;\n"));
        assert!(a.contains("MANIFOLD_SOLID_BREP('board',"));
        assert!(a.contains("MANIFOLD_SOLID_BREP('R1',"));
        assert!(a.contains("SI_UNIT(.MILLI.,.METRE.)"));
        // every reference points to an entity that is defined before the end of the file
        let defined: Vec<usize> = a
            .lines()
            .filter_map(|l| l.strip_prefix('#'))
            .filter_map(|l| l.split('=').next())
            .filter_map(|n| n.parse().ok())
            .collect();
        assert_eq!(
            defined.len(),
            a.lines().filter(|l| l.starts_with('#')).count()
        );
        for (position, number) in defined.iter().enumerate() {
            assert_eq!(*number, position + 1);
        }
        for line in a.lines().filter(|l| l.starts_with('#')) {
            let body = line.split_once('=').unwrap().1;
            let mut rest = body;
            while let Some(at) = rest.find('#') {
                let digits: String = rest[at + 1..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                let id: usize = digits.parse().unwrap();
                assert!((1..=defined.len()).contains(&id), "{line}");
                rest = &rest[at + 1..];
            }
        }
        // 2 solids of 6 faces
        assert_eq!(a.matches("ADVANCED_FACE(").count(), 12);
        assert_eq!(a.matches("EDGE_CURVE(").count(), 24);
        assert_eq!(a.matches("CLOSED_SHELL(").count(), 2);
    }

    #[test]
    fn a_side_face_has_its_normal_to_the_outside() {
        let a = prisms_step(&[square("b", 2 * MM, 0, MM)]);
        // the face on y = 0 looks to -y, the face on x = 2 mm looks to +x
        assert!(a.contains("DIRECTION('',(0.,-1.000000000,0.))"));
        assert!(a.contains("DIRECTION('',(1.000000000,0.,0.))"));
        assert!(a.contains("CARTESIAN_POINT('',(2.,2.,1.))"));
    }

    fn board_with_two_holes() -> String {
        let outline = [(0, 0), (20 * MM, 0), (20 * MM, 10 * MM), (0, 10 * MM)];
        let holes = [
            Hole {
                at: (5 * MM, 5 * MM),
                diameter_nm: 3 * MM,
            },
            Hole {
                at: (15 * MM, 5 * MM),
                diameter_nm: 2 * MM,
            },
        ];
        board_step(&outline, 1_600_000, &holes)
    }

    #[test]
    fn a_board_with_holes_has_a_bound_and_a_wall_for_each_hole() {
        let a = board_with_two_holes();
        assert_eq!(a, board_with_two_holes());
        assert!(a.starts_with("ISO-10303-21;\nHEADER;\n"));
        assert!(a.ends_with("END-ISO-10303-21;\n"));
        // the six faces of the prism and a wall for each hole
        assert_eq!(a.matches("ADVANCED_FACE(").count(), 8);
        // the top and the bottom face have an inner bound for each hole
        assert_eq!(a.matches("FACE_BOUND(").count(), 4);
        assert_eq!(a.matches("FACE_OUTER_BOUND(").count(), 8);
        assert_eq!(a.matches("CYLINDRICAL_SURFACE(").count(), 2);
        assert_eq!(a.matches("CIRCLE(").count(), 4);
        // 12 edges of the prism, and for each hole two circles and the line along the wall
        assert_eq!(a.matches("EDGE_CURVE(").count(), 18);
        assert_eq!(a.matches("CLOSED_SHELL(").count(), 1);
        assert_eq!(a.matches("MANIFOLD_SOLID_BREP('board',").count(), 1);
        // the radius of each hole: 1.5 mm and 1 mm
        assert!(a.contains("CYLINDRICAL_SURFACE('',#") && a.contains(",1.5)"));
        assert!(a.contains(",1.)"));
        // the wall of a hole is the cylinder turned inside out: its face has the sense false
        let inside_out = a
            .lines()
            .filter(|l| l.contains("ADVANCED_FACE(") && l.ends_with(",.F.);"))
            .count();
        assert_eq!(inside_out, 2);
    }

    #[test]
    fn every_reference_of_a_board_with_holes_is_defined_before_the_end_of_the_file() {
        let a = board_with_two_holes();
        let defined = a.lines().filter(|l| l.starts_with('#')).count();
        for line in a.lines().filter(|l| l.starts_with('#')) {
            let mut rest = line.split_once('=').unwrap().1;
            while let Some(at) = rest.find('#') {
                let digits: String = rest[at + 1..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                let id: usize = digits.parse().unwrap();
                assert!((1..=defined).contains(&id), "{line}");
                rest = &rest[at + 1..];
            }
        }
    }

    #[test]
    fn the_reader_of_the_suite_reads_the_board_back_with_its_circles() {
        let solid = crate::step::read_step(&board_with_two_holes(), 100_000).unwrap();
        assert_eq!(solid.kind, crate::step::SolidKind::Curved);
        // the circles count as cubes about their centres: 1.5 mm for the biggest hole
        assert_eq!(solid.min, [0, 0, -1_500_000]);
        assert_eq!(solid.max, [20 * MM, 10 * MM, 3_100_000]);
        // without holes it is a plain box
        let plain = board_step(
            &[(0, 0), (20 * MM, 0), (20 * MM, 10 * MM), (0, 10 * MM)],
            1_600_000,
            &[],
        );
        let solid = crate::step::read_step(&plain, 100_000).unwrap();
        assert_eq!(solid.kind, crate::step::SolidKind::Box);
        assert_eq!(solid.max, [20 * MM, 10 * MM, 1_600_000]);
        assert_eq!(plain.matches("ADVANCED_FACE(").count(), 6);
    }

    #[test]
    fn an_outline_without_area_or_a_board_without_thickness_gives_a_file_with_no_solid() {
        let none = board_step(&[(0, 0), (MM, 0)], 1_600_000, &[]);
        assert_eq!(none, prisms_step(&[]));
        let thin = board_step(&[(0, 0), (MM, 0), (MM, MM)], 0, &[]);
        assert_eq!(thin, prisms_step(&[]));
    }
}
