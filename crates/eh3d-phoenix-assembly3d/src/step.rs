// SPDX-License-Identifier: GPL-3.0-or-later
//! A reader of the least a STEP file (ISO 10303-21) must give for a component on a board: the
//! box that encloses its solid, in nanometres.
//!
//! It reads the text of the file (the entities of the `DATA` section, the length unit, the points
//! of the vertices, the lines, the planes and the circles) and nothing else. A solid that is a
//! plain box, with 8 vertices on 2 values of each axis and plane faces only, is a [`SolidKind::Box`]
//! and its box is the solid. Any other solid is replaced by its enclosing box and says so in its
//! kind: [`SolidKind::Polyhedron`] (straight edges and plane faces: the box of the vertices is the
//! box of the solid), [`SolidKind::Curved`] (circles and cylinders as well: the box takes the
//! circles by their centre plus and minus the radius on each axis, so it may be larger than the
//! solid) and [`SolidKind::Approximate`] (a surface or a curve that this reader does not know,
//! such as a spline, a sphere or a cone: the box of every point of the file, which may not enclose
//! the solid). Nothing is invented: a file with no vertex has no geometry.
//!
//! Numbers are turned into nanometres exactly, from the decimal text and the unit, with no
//! floating-point number.

use std::collections::{BTreeMap, BTreeSet};

/// What the solid of the file is, which is also how much the box can be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidKind {
    /// A plain box: the box is the solid.
    Box,
    /// Straight edges and plane faces: the box is the exact box around the solid.
    Polyhedron,
    /// Circles and cylinders too: the box encloses the solid and may be larger.
    Curved,
    /// Something this reader does not know: the box of the points of the file.
    Approximate,
}

/// The box that encloses the solid of a STEP file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSolid {
    /// The least corner, in nanometres.
    pub min: [i64; 3],
    /// The greatest corner, in nanometres.
    pub max: [i64; 3],
    /// What the solid is.
    pub kind: SolidKind,
    /// The entity names this reader does not know, when the kind is approximate.
    pub unknown: Vec<String>,
}

/// Why a STEP file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepError {
    /// The text is not a STEP file.
    NotStep(String),
    /// An entity or a number is malformed.
    Syntax(String),
    /// The length unit is missing or is not one this reader knows.
    Unit(String),
    /// The file has no vertex and no circle.
    NoGeometry,
    /// The file has more entities than the limit.
    TooMany(usize),
}

/// Turns a decimal number of a file, such as `-1.5E-1` or `12.`, into nanometres when the unit is
/// worth `nm_per_unit` nanometres, rounded half away from zero. `None` for text that is not a
/// number, with more than 30 digits, or out of range.
pub fn decimal_to_nm(text: &str, nm_per_unit: i128) -> Option<i64> {
    let t = text.trim();
    let (negative, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let (mantissa, exponent) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i32>().ok()?),
        None => (t, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if whole.len() + fraction.len() > 30 {
        return None;
    }
    let mut digits: i128 = 0;
    for b in whole.bytes().chain(fraction.bytes()) {
        if !b.is_ascii_digit() {
            return None;
        }
        digits = digits * 10 + i128::from(b - b'0');
    }
    let power = exponent.checked_sub(i32::try_from(fraction.len()).ok()?)?;
    if !(-40..=40).contains(&power) {
        return None;
    }
    let scaled = digits.checked_mul(nm_per_unit)?;
    let magnitude = if power >= 0 {
        scaled.checked_mul(10i128.checked_pow(power.unsigned_abs())?)?
    } else {
        let divisor = 10i128.checked_pow(power.unsigned_abs())?;
        (scaled + divisor / 2) / divisor
    };
    i64::try_from(if negative { -magnitude } else { magnitude }).ok()
}

/// One entity of the `DATA` section: its name (empty for a complex entity) and the text inside
/// its outer parentheses (the whole text for a complex entity).
struct Entity {
    name: String,
    args: String,
}

/// Removes the `/* ... */` comments, leaving the text of quoted strings alone.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\'' {
                in_string = false;
            }
            continue;
        }
        if c == '\'' {
            in_string = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut previous = ' ';
            for d in chars.by_ref() {
                if previous == '*' && d == '/' {
                    break;
                }
                previous = d;
            }
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// The statements of a text, split at the semicolons that are not inside a quoted string.
fn statements(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    for c in text.chars() {
        if c == '\'' {
            in_string = !in_string;
        }
        if c == ';' && !in_string {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

fn parse_entities(text: &str, max_entities: usize) -> Result<BTreeMap<u64, Entity>, StepError> {
    let clean = strip_comments(text);
    let upper_start = clean
        .find("DATA;")
        .ok_or_else(|| StepError::NotStep("no DATA section".to_owned()))?;
    if !clean.trim_start().starts_with("ISO-10303-21") {
        return Err(StepError::NotStep(
            "it does not start with ISO-10303-21".to_owned(),
        ));
    }
    let body = &clean[upper_start + "DATA;".len()..];
    let mut entities = BTreeMap::new();
    for statement in statements(body) {
        let s = statement.trim();
        if s.starts_with("ENDSEC") {
            break;
        }
        if s.is_empty() {
            continue;
        }
        let Some(rest) = s.strip_prefix('#') else {
            return Err(StepError::Syntax(format!("not an entity: {}", shorten(s))));
        };
        let (id_text, definition) = rest
            .split_once('=')
            .ok_or_else(|| StepError::Syntax(format!("no = in {}", shorten(s))))?;
        let id: u64 = id_text
            .trim()
            .parse()
            .map_err(|_| StepError::Syntax(format!("entity number {}", shorten(id_text))))?;
        let definition = definition.trim();
        let entity = if definition.starts_with('(') {
            Entity {
                name: String::new(),
                args: definition.to_owned(),
            }
        } else {
            let open = definition
                .find('(')
                .ok_or_else(|| StepError::Syntax(format!("no ( in {}", shorten(s))))?;
            if !definition.ends_with(')') {
                return Err(StepError::Syntax(format!(
                    "no ) at the end of {}",
                    shorten(s)
                )));
            }
            Entity {
                name: definition[..open].trim().to_ascii_uppercase(),
                args: definition[open + 1..definition.len() - 1].to_owned(),
            }
        };
        if entities.len() >= max_entities {
            return Err(StepError::TooMany(max_entities));
        }
        if entities.insert(id, entity).is_some() {
            return Err(StepError::Syntax(format!("entity #{id} is defined twice")));
        }
    }
    Ok(entities)
}

fn shorten(text: &str) -> String {
    text.chars().take(40).collect()
}

/// The references (`#12`) in a text, in order.
fn references(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                if let Ok(n) = text[start..end].parse() {
                    out.push(n);
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// The numbers in the last parenthesised group of an argument text, as text:
/// `'',(1.,2.,3.)` gives `1.`, `2.`, `3.`.
fn tuple_numbers(args: &str) -> Vec<String> {
    let Some(open) = args.rfind('(') else {
        return Vec::new();
    };
    let inner = &args[open + 1..];
    let inner = inner.split(')').next().unwrap_or("");
    inner.split(',').map(|s| s.trim().to_owned()).collect()
}

/// The length unit in nanometres per unit, from the entities of the file.
fn length_unit(entities: &BTreeMap<u64, Entity>) -> Result<i128, StepError> {
    for entity in entities.values() {
        let upper = entity.args.to_ascii_uppercase();
        let is_length = entity.name == "LENGTH_UNIT" || upper.contains("LENGTH_UNIT");
        if !is_length {
            continue;
        }
        if upper.contains("CONVERSION_BASED_UNIT") {
            if upper.contains("INCH") {
                return Ok(25_400_000);
            }
            return Err(StepError::Unit(
                "a unit based on a conversion that is not the inch".to_owned(),
            ));
        }
        if let Some(at) = upper.find("SI_UNIT(") {
            let prefix = upper[at + "SI_UNIT(".len()..]
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_owned();
            return match prefix.as_str() {
                ".MILLI." => Ok(1_000_000),
                ".CENTI." => Ok(10_000_000),
                ".DECI." => Ok(100_000_000),
                ".MICRO." => Ok(1_000),
                "$" => Ok(1_000_000_000),
                other => Err(StepError::Unit(format!("the prefix {other}"))),
            };
        }
    }
    Err(StepError::Unit("the file has no length unit".to_owned()))
}

/// Entity names that are the structure of a solid or the parts of the geometry this reader reads.
const KNOWN: &[&str] = &[
    "CARTESIAN_POINT",
    "DIRECTION",
    "VECTOR",
    "AXIS2_PLACEMENT_3D",
    "LINE",
    "PLANE",
    "CIRCLE",
    "CYLINDRICAL_SURFACE",
    "SURFACE_CURVE",
    "SEAM_CURVE",
    "PCURVE",
    "DEFINITIONAL_REPRESENTATION",
    "VERTEX_POINT",
    "EDGE_CURVE",
    "ORIENTED_EDGE",
    "EDGE_LOOP",
    "FACE_BOUND",
    "FACE_OUTER_BOUND",
    "ADVANCED_FACE",
    "FACE_SURFACE",
    "CLOSED_SHELL",
    "OPEN_SHELL",
    "MANIFOLD_SOLID_BREP",
    "BREP_WITH_VOIDS",
    "ORIENTED_CLOSED_SHELL",
    "POLY_LOOP",
    "FACETED_BREP",
    "FACETED_BREP_SHAPE_REPRESENTATION",
    "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "SHAPE_REPRESENTATION",
    "SHAPE_DEFINITION_REPRESENTATION",
    "PRODUCT_DEFINITION_SHAPE",
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION",
    "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_RELATED_PRODUCT_CATEGORY",
    "PRODUCT_CONTEXT",
    "PRODUCT",
    "APPLICATION_CONTEXT",
    "APPLICATION_PROTOCOL_DEFINITION",
    "UNCERTAINTY_MEASURE_WITH_UNIT",
    "MECHANICAL_CONTEXT",
    "DESIGN_CONTEXT",
];

/// Reads the box that encloses the solid of a STEP file, with no more than `max_entities`
/// entities.
pub fn read_step(text: &str, max_entities: usize) -> Result<StepSolid, StepError> {
    let entities = parse_entities(text, max_entities)?;
    let unit = length_unit(&entities)?;

    let point_of = |id: u64| -> Result<[i64; 3], StepError> {
        let entity = entities
            .get(&id)
            .filter(|e| e.name == "CARTESIAN_POINT")
            .ok_or_else(|| StepError::Syntax(format!("#{id} is not a point")))?;
        let numbers = tuple_numbers(&entity.args);
        if numbers.len() != 3 {
            return Err(StepError::Syntax(format!(
                "point #{id} has {} numbers",
                numbers.len()
            )));
        }
        let mut out = [0i64; 3];
        for (slot, number) in out.iter_mut().zip(&numbers) {
            *slot = decimal_to_nm(number, unit).ok_or_else(|| {
                StepError::Syntax(format!("point #{id}: {number} is not a number"))
            })?;
        }
        Ok(out)
    };

    // the points of the vertices (or of the loops of a faceted solid)
    let mut vertex_points: Vec<[i64; 3]> = Vec::new();
    for entity in entities.values() {
        if entity.name == "VERTEX_POINT" {
            if let Some(id) = references(&entity.args).first() {
                vertex_points.push(point_of(*id)?);
            }
        }
    }
    if vertex_points.is_empty() {
        for entity in entities.values() {
            if entity.name == "POLY_LOOP" {
                for id in references(&entity.args) {
                    vertex_points.push(point_of(id)?);
                }
            }
        }
    }

    // the circles: their centre plus and minus the radius on each axis
    let mut circle_points: Vec<[i64; 3]> = Vec::new();
    for entity in entities.values() {
        if entity.name != "CIRCLE" {
            continue;
        }
        let axis = references(&entity.args)
            .first()
            .and_then(|id| entities.get(id))
            .filter(|e| e.name == "AXIS2_PLACEMENT_3D")
            .ok_or_else(|| StepError::Syntax("a circle without a placement".to_owned()))?;
        let centre_id = references(&axis.args)
            .first()
            .copied()
            .ok_or_else(|| StepError::Syntax("a placement without a point".to_owned()))?;
        let centre = point_of(centre_id)?;
        let radius_text = entity
            .args
            .rsplit(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        let radius = decimal_to_nm(&radius_text, unit)
            .filter(|r| *r >= 0)
            .ok_or_else(|| StepError::Syntax(format!("circle radius {radius_text}")))?;
        for sign in [-1i64, 1] {
            let mut p = centre;
            for v in &mut p {
                *v = v
                    .checked_add(sign * radius)
                    .ok_or_else(|| StepError::Syntax("a circle out of range".to_owned()))?;
            }
            circle_points.push(p);
        }
    }

    // what this reader does not know
    let mut unknown: BTreeSet<String> = BTreeSet::new();
    for entity in entities.values() {
        let upper = entity.args.to_ascii_uppercase();
        if entity.name.is_empty() {
            if upper.contains("B_SPLINE") || upper.contains("SURFACE") || upper.contains("CURVE") {
                unknown.insert("complex entity with a curve or a surface".to_owned());
            }
            continue;
        }
        let geometric = entity.name.ends_with("_SURFACE")
            || entity.name.ends_with("_CURVE")
            || entity.name.starts_with("SURFACE_OF_")
            || entity.name.contains("B_SPLINE")
            || entity.name == "ELLIPSE"
            || entity.name == "HYPERBOLA"
            || entity.name == "PARABOLA";
        if geometric && !KNOWN.contains(&entity.name.as_str()) {
            unknown.insert(entity.name.clone());
        }
    }

    if vertex_points.is_empty() && circle_points.is_empty() {
        return Err(StepError::NoGeometry);
    }
    let (kind, points): (SolidKind, Vec<[i64; 3]>) = if !unknown.is_empty() {
        let mut every = Vec::new();
        for (id, entity) in &entities {
            if entity.name == "CARTESIAN_POINT" {
                every.push(point_of(*id)?);
            }
        }
        (SolidKind::Approximate, every)
    } else if !circle_points.is_empty() {
        let mut all = vertex_points.clone();
        all.extend(circle_points);
        (SolidKind::Curved, all)
    } else if is_box(&vertex_points, &entities) {
        (SolidKind::Box, vertex_points.clone())
    } else {
        (SolidKind::Polyhedron, vertex_points.clone())
    };
    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for p in &points {
        for ((lo, hi), v) in min.iter_mut().zip(max.iter_mut()).zip(p) {
            *lo = (*lo).min(*v);
            *hi = (*hi).max(*v);
        }
    }
    Ok(StepSolid {
        min,
        max,
        kind,
        unknown: unknown.into_iter().collect(),
    })
}

/// Whether the vertices are the 8 corners of a box and the faces are 6 planes.
fn is_box(vertices: &[[i64; 3]], entities: &BTreeMap<u64, Entity>) -> bool {
    let distinct: BTreeSet<[i64; 3]> = vertices.iter().copied().collect();
    if distinct.len() != 8 {
        return false;
    }
    let mut values: [BTreeSet<i64>; 3] = [BTreeSet::new(), BTreeSet::new(), BTreeSet::new()];
    for p in &distinct {
        for (axis, set) in values.iter_mut().enumerate() {
            set.insert(p[axis]);
        }
    }
    if values.iter().any(|s| s.len() != 2) {
        return false;
    }
    let faces = entities
        .values()
        .filter(|e| e.name == "ADVANCED_FACE" || e.name == "FACE_SURFACE")
        .count();
    let planes = entities.values().filter(|e| e.name == "PLANE").count();
    faces == 6 && planes >= 6
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box 4 by 2 by 1 millimetres with its least corner at (1, 1, 0), written like the files
    /// of the program: 8 vertices and 6 planes.
    fn box_step(unit: &str) -> String {
        let mut s = String::from("ISO-10303-21;\nHEADER;\nFILE_NAME('a.step');\nENDSEC;\nDATA;\n");
        s.push_str(&format!(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT({unit},.METRE.));\n"
        ));
        let mut n = 2;
        let mut point_ids = Vec::new();
        for z in [0.0, 1.0] {
            for y in [1.0, 3.0] {
                for x in [1.0, 5.0] {
                    s.push_str(&format!(
                        "#{n}=CARTESIAN_POINT('',({x:.1},{y:.1},{z:.1}));\n"
                    ));
                    point_ids.push(n);
                    n += 1;
                }
            }
        }
        for id in &point_ids {
            s.push_str(&format!("#{n}=VERTEX_POINT('',#{id});\n"));
            n += 1;
        }
        for _ in 0..6 {
            s.push_str(&format!("#{n}=PLANE('',#2);\n"));
            n += 1;
            s.push_str(&format!("#{n}=ADVANCED_FACE('',(),#{},.T.);\n", n - 1));
            n += 1;
        }
        s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        s
    }

    #[test]
    fn decimals_become_nanometres_exactly() {
        assert_eq!(decimal_to_nm("1.5", 1_000_000), Some(1_500_000));
        assert_eq!(decimal_to_nm("-0.25", 1_000_000), Some(-250_000));
        assert_eq!(decimal_to_nm("12.", 1_000_000), Some(12_000_000));
        assert_eq!(decimal_to_nm(".5", 1_000_000), Some(500_000));
        assert_eq!(decimal_to_nm("1.E-3", 1_000_000), Some(1_000));
        assert_eq!(decimal_to_nm("2.54E1", 1_000_000), Some(25_400_000));
        assert_eq!(decimal_to_nm("0.0000005", 1_000_000), Some(1));
        assert_eq!(decimal_to_nm("-0.0000005", 1_000_000), Some(-1));
        assert_eq!(decimal_to_nm("0.0000004", 1_000_000), Some(0));
        assert_eq!(decimal_to_nm("1", 1_000_000_000), Some(1_000_000_000));
        for bad in ["", ".", "abc", "1..2", "1e", "1e99", "--1"] {
            assert_eq!(decimal_to_nm(bad, 1_000_000), None, "{bad}");
        }
        assert_eq!(decimal_to_nm("9999999999999999999999", 1_000_000), None);
    }

    #[test]
    fn a_box_is_a_box_with_its_corners_in_nanometres() {
        let solid = read_step(&box_step(".MILLI."), 1000).unwrap();
        assert_eq!(solid.kind, SolidKind::Box);
        assert_eq!(solid.min, [1_000_000, 1_000_000, 0]);
        assert_eq!(solid.max, [5_000_000, 3_000_000, 1_000_000]);
        assert!(solid.unknown.is_empty());
    }

    #[test]
    fn the_unit_of_the_file_decides_the_size() {
        let cm = read_step(&box_step(".CENTI."), 1000).unwrap();
        assert_eq!(cm.max, [50_000_000, 30_000_000, 10_000_000]);
        let m = read_step(&box_step("$"), 1000).unwrap();
        assert_eq!(m.max[0], 5_000_000_000);
        let inch = box_step(".MILLI.").replace(
            "SI_UNIT(.MILLI.,.METRE.)",
            "CONVERSION_BASED_UNIT('INCH',#9)",
        );
        let solid = read_step(&inch, 1000).unwrap();
        assert_eq!(solid.max[0], 127_000_000);
        let no_unit = box_step(".MILLI.").replace("LENGTH_UNIT()", "AREA_UNIT()");
        assert!(matches!(read_step(&no_unit, 1000), Err(StepError::Unit(_))));
        let kilo = box_step(".KILO.");
        assert!(matches!(read_step(&kilo, 1000), Err(StepError::Unit(_))));
    }

    #[test]
    fn eight_vertices_that_are_not_a_box_are_a_polyhedron() {
        let skew = box_step(".MILLI.").replace("(5.0,1.0,0.0)", "(5.5,1.0,0.0)");
        let solid = read_step(&skew, 1000).unwrap();
        assert_eq!(solid.kind, SolidKind::Polyhedron);
        assert_eq!(solid.max[0], 5_500_000);
        let planes_gone = box_step(".MILLI.").replace("PLANE(", "SOMETHING(");
        assert_eq!(
            read_step(&planes_gone, 1000).unwrap().kind,
            SolidKind::Polyhedron
        );
    }

    #[test]
    fn a_circle_is_covered_by_its_centre_and_radius() {
        let mut s = box_step(".MILLI.");
        s = s.replace(
            "ENDSEC;\nEND-ISO",
            "#900=CARTESIAN_POINT('',(10.0,10.0,0.0));\n#901=AXIS2_PLACEMENT_3D('',#900,#2,#2);\n#902=CIRCLE('',#901,2.0);\nENDSEC;\nEND-ISO",
        );
        let solid = read_step(&s, 1000).unwrap();
        assert_eq!(solid.kind, SolidKind::Curved);
        assert_eq!(solid.min, [1_000_000, 1_000_000, -2_000_000]);
        assert_eq!(solid.max, [12_000_000, 12_000_000, 2_000_000]);
    }

    #[test]
    fn a_surface_that_is_not_known_gives_the_approximate_box_of_every_point() {
        let s = box_step(".MILLI.").replace(
            "ENDSEC;\nEND-ISO",
            "#900=CARTESIAN_POINT('',(20.0,0.0,0.0));\n#901=SPHERICAL_SURFACE('',#2,1.0);\nENDSEC;\nEND-ISO",
        );
        let solid = read_step(&s, 1000).unwrap();
        assert_eq!(solid.kind, SolidKind::Approximate);
        assert_eq!(solid.unknown, vec!["SPHERICAL_SURFACE".to_owned()]);
        assert_eq!(solid.max[0], 20_000_000);
    }

    #[test]
    fn what_is_not_a_step_file_or_has_no_geometry_is_refused() {
        assert!(matches!(read_step("hello", 10), Err(StepError::NotStep(_))));
        assert!(matches!(
            read_step("ISO-10303-21;\nHEADER;\nENDSEC;\n", 10),
            Err(StepError::NotStep(_))
        ));
        let empty = "ISO-10303-21;\nDATA;\n#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\nENDSEC;\n";
        assert_eq!(read_step(empty, 10), Err(StepError::NoGeometry));
        let broken = "ISO-10303-21;\nDATA;\n#1 CARTESIAN_POINT;\nENDSEC;\n";
        assert!(matches!(read_step(broken, 10), Err(StepError::Syntax(_))));
        let twice = "ISO-10303-21;\nDATA;\n#1=LINE();\n#1=LINE();\nENDSEC;\n";
        assert!(matches!(read_step(twice, 10), Err(StepError::Syntax(_))));
        assert_eq!(
            read_step(&box_step(".MILLI."), 5),
            Err(StepError::TooMany(5))
        );
    }

    #[test]
    fn comments_and_semicolons_inside_strings_are_not_statements() {
        let s = box_step(".MILLI.").replace(
            "FILE_NAME('a.step');",
            "FILE_NAME('a;b.step'); /* a comment; with a semicolon */",
        );
        let solid = read_step(&s, 1000).unwrap();
        assert_eq!(solid.kind, SolidKind::Box);
    }

    #[test]
    fn a_circle_written_like_the_files_of_the_cad_program_is_read() {
        // a circle on the plane z = 0 in the style of the files that the CAD program of the suite writes
        let text = "ISO-10303-21;\nDATA;\n#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=DIRECTION('',(0.,0.,1.));\n#3=DIRECTION('',(1.,0.,0.));\n#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n#5=CIRCLE('',#4,0.0015);\n#6=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\nENDSEC;\n";
        let solid = read_step(text, 100).unwrap();
        assert_eq!(solid.kind, SolidKind::Curved);
        assert_eq!(solid.max, [1_500, 1_500, 1_500]);
    }
}
