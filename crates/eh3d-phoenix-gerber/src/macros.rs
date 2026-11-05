// SPDX-License-Identifier: GPL-3.0-or-later
//! Aperture macros (`%AM`): the expressions with variables and the basic primitives (circle,
//! vector line, centre line, lower-left line, outline and polygon). The thermal and the moire
//! primitives are counted and skipped.

use std::collections::BTreeMap;

use crate::model::{MacroShape, P};
use crate::settings::ImportSettings;

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
    vars: &'a BTreeMap<u32, f64>,
    depth: u32,
}

/// The deepest nesting of brackets in an expression.
const MAX_DEPTH: u32 = 32;

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut value = self.term()?;
        loop {
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    value += self.term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    value -= self.term()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut value = self.factor()?;
        loop {
            match self.peek() {
                Some('x' | 'X') => {
                    self.pos += 1;
                    value *= self.factor()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err("division by zero".to_owned());
                    }
                    value /= divisor;
                }
                _ => return Ok(value),
            }
        }
    }

    fn factor(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                Ok(-self.factor()?)
            }
            Some('+') => {
                self.pos += 1;
                self.factor()
            }
            Some('(') => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err("brackets nested too deep".to_owned());
                }
                self.pos += 1;
                let value = self.expr()?;
                if self.peek() != Some(')') {
                    return Err("a bracket is not closed".to_owned());
                }
                self.pos += 1;
                self.depth -= 1;
                Ok(value)
            }
            Some('$') => {
                self.pos += 1;
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                }
                let text: String = self.chars[start..self.pos].iter().collect();
                let number: u32 = text
                    .parse()
                    .map_err(|_| "a variable has no number".to_owned())?;
                self.vars
                    .get(&number)
                    .copied()
                    .ok_or_else(|| format!("variable ${number} has no value"))
            }
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                    self.pos += 1;
                }
                let text: String = self.chars[start..self.pos].iter().collect();
                text.parse::<f64>()
                    .map_err(|_| format!("'{text}' is not a number"))
            }
            Some(c) => Err(format!("unexpected '{c}'")),
            None => Err("the expression ends too soon".to_owned()),
        }
    }
}

/// The value of an expression with `+`, `-`, `x`, `/`, brackets, signs and variables (`$1`).
pub fn eval(text: &str, vars: &BTreeMap<u32, f64>, max_chars: usize) -> Result<f64, String> {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.is_empty() {
        return Err("empty expression".to_owned());
    }
    if chars.len() > max_chars {
        return Err("the expression is too long".to_owned());
    }
    let mut parser = Parser {
        chars: &chars,
        pos: 0,
        vars,
        depth: 0,
    };
    let value = parser.expr()?;
    if let Some(c) = parser.peek() {
        return Err(format!("unexpected '{c}'"));
    }
    if value.is_finite() {
        Ok(value)
    } else {
        Err("the value is not finite".to_owned())
    }
}

fn to_nm(value: f64, unit_nm: i64) -> Result<i64, String> {
    if !value.is_finite() || value.abs() > 1.0e9 {
        return Err("a size is out of range".to_owned());
    }
    Ok((value * unit_nm as f64).round() as i64)
}

fn to_udeg(value: f64) -> Result<i64, String> {
    if !value.is_finite() || value.abs() > 1.0e6 {
        return Err("an angle is out of range".to_owned());
    }
    Ok((value * 1.0e6).round() as i64)
}

fn to_exposure(value: f64) -> Result<bool, String> {
    if (value - 1.0).abs() < 1e-9 {
        Ok(true)
    } else if value.abs() < 1e-9 {
        Ok(false)
    } else {
        Err("the exposure is neither 0 nor 1".to_owned())
    }
}

/// The primitives of the macro `name` for the parameters of one `%AD`, and how many primitives
/// were skipped because the import does not read them.
pub fn instantiate(
    statements: &[String],
    params: &[f64],
    unit_nm: i64,
    settings: &ImportSettings,
) -> Result<(Vec<MacroShape>, usize), String> {
    let mut vars: BTreeMap<u32, f64> = BTreeMap::new();
    for (i, v) in params.iter().enumerate() {
        vars.insert(i as u32 + 1, *v);
    }
    let mut shapes: Vec<MacroShape> = Vec::new();
    let mut skipped = 0usize;
    for statement in statements {
        let statement = statement.trim();
        if statement.is_empty() || statement.starts_with('0') {
            continue;
        }
        if let Some(rest) = statement.strip_prefix('$') {
            let (number, expression) = rest
                .split_once('=')
                .ok_or_else(|| "a variable without '='".to_owned())?;
            let number: u32 = number
                .trim()
                .parse()
                .map_err(|_| "a variable has no number".to_owned())?;
            let value = eval(expression, &vars, settings.max_expression_chars)?;
            vars.insert(number, value);
            continue;
        }
        let mut parts = statement.split(',');
        let code: u32 = parts
            .next()
            .unwrap_or("")
            .trim()
            .parse()
            .map_err(|_| format!("'{statement}' does not start with a primitive code"))?;
        let mut args: Vec<f64> = Vec::new();
        for expression in parts {
            args.push(eval(expression, &vars, settings.max_expression_chars)?);
        }
        if shapes.len() + skipped >= settings.max_macro_primitives {
            return Err("the macro has more primitives than the limit".to_owned());
        }
        let need = |low: usize, high: usize| -> Result<(), String> {
            if (low..=high).contains(&args.len()) {
                Ok(())
            } else {
                Err(format!(
                    "primitive {code} takes {low} to {high} parameters, not {}",
                    args.len()
                ))
            }
        };
        let point = |x: f64, y: f64| -> Result<P, String> {
            Ok(P {
                x: to_nm(x, unit_nm)?,
                y: to_nm(y, unit_nm)?,
            })
        };
        let rotation = |index: usize| -> Result<i64, String> {
            args.get(index).map_or(Ok(0), |v| to_udeg(*v))
        };
        match code {
            1 => {
                need(4, 5)?;
                shapes.push(MacroShape::Circle {
                    exposure: to_exposure(args[0])?,
                    diameter: to_nm(args[1], unit_nm)?,
                    centre: point(args[2], args[3])?,
                    rotation: rotation(4)?,
                });
            }
            20 => {
                need(6, 7)?;
                shapes.push(MacroShape::VectorLine {
                    exposure: to_exposure(args[0])?,
                    width: to_nm(args[1], unit_nm)?,
                    start: point(args[2], args[3])?,
                    end: point(args[4], args[5])?,
                    rotation: rotation(6)?,
                });
            }
            21 => {
                need(5, 6)?;
                shapes.push(MacroShape::CenterLine {
                    exposure: to_exposure(args[0])?,
                    width: to_nm(args[1], unit_nm)?,
                    height: to_nm(args[2], unit_nm)?,
                    centre: point(args[3], args[4])?,
                    rotation: rotation(5)?,
                });
            }
            22 => {
                need(5, 6)?;
                shapes.push(MacroShape::CenterLine {
                    exposure: to_exposure(args[0])?,
                    width: to_nm(args[1], unit_nm)?,
                    height: to_nm(args[2], unit_nm)?,
                    centre: point(args[3] + args[1] / 2.0, args[4] + args[2] / 2.0)?,
                    rotation: rotation(5)?,
                });
            }
            4 => {
                need(2, 8200)?;
                let count = args[1];
                if !(1.0..=4096.0).contains(&count) || count.fract() != 0.0 {
                    return Err("an outline has 1 to 4096 corners after the start".to_owned());
                }
                let n = count as usize;
                let with_rotation = 2 + 2 * (n + 1) + 1;
                if args.len() != with_rotation && args.len() != with_rotation - 1 {
                    return Err(format!(
                        "an outline of {n} corners takes {} parameters",
                        with_rotation - 1
                    ));
                }
                let mut points = Vec::with_capacity(n + 1);
                for i in 0..=n {
                    points.push(point(args[2 + 2 * i], args[3 + 2 * i])?);
                }
                shapes.push(MacroShape::Outline {
                    exposure: to_exposure(args[0])?,
                    points,
                    rotation: rotation(with_rotation - 1)?,
                });
            }
            5 => {
                need(5, 6)?;
                let count = args[1];
                if !(3.0..=100.0).contains(&count) || count.fract() != 0.0 {
                    return Err("a polygon has 3 to 100 corners".to_owned());
                }
                shapes.push(MacroShape::Polygon {
                    exposure: to_exposure(args[0])?,
                    vertices: count as u32,
                    centre: point(args[2], args[3])?,
                    diameter: to_nm(args[4], unit_nm)?,
                    rotation: rotation(5)?,
                });
            }
            6 | 7 => skipped += 1,
            other => return Err(format!("primitive {other} is not known")),
        }
    }
    Ok((shapes, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(values: &[f64]) -> BTreeMap<u32, f64> {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| (i as u32 + 1, *v))
            .collect()
    }

    #[test]
    fn expressions_follow_the_usual_order() {
        let v = vars(&[2.0, 3.0]);
        assert_eq!(eval("1+2x3", &v, 64), Ok(7.0));
        assert_eq!(eval("(1+2)x3", &v, 64), Ok(9.0));
        assert_eq!(eval("$1x$2-1", &v, 64), Ok(5.0));
        assert_eq!(eval("-$1/4", &v, 64), Ok(-0.5));
        assert_eq!(eval(" 1.5 X 2 ", &v, 64), Ok(3.0));
        assert_eq!(eval("+-2", &v, 64), Ok(-2.0));
    }

    #[test]
    fn bad_expressions_are_refused() {
        let v = vars(&[1.0]);
        for (text, needle) in [
            ("", "empty"),
            ("1/0", "division"),
            ("$9", "no value"),
            ("1+", "ends too soon"),
            ("(1", "bracket"),
            ("1)", "unexpected"),
            ("2 q 3", "unexpected"),
            ("1.2.3", "not a number"),
        ] {
            let e = eval(text, &v, 64).unwrap_err();
            assert!(e.contains(needle), "{text}: {e}");
        }
        assert!(eval(&"1+".repeat(40), &v, 16)
            .unwrap_err()
            .contains("too long"));
        assert!(
            eval(&format!("{}1{}", "(".repeat(40), ")".repeat(40)), &v, 256)
                .unwrap_err()
                .contains("deep")
        );
    }

    fn statements(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_basic_primitives_become_shapes_in_nanometres() {
        let settings = ImportSettings::default();
        let body = statements(&[
            "0 a comment",
            "$3=$1x2",
            "1,1,$1,0,0",
            "20,1,0.1,0,0,$3,0,45",
            "21,0,$1,$2,0,0",
            "22,1,1,2,0.5,0.25,0",
            "4,1,3,0,0,1,0,1,1,0,0,10",
            "5,1,6,0,0,2,15",
            "7,0,0,1,0.8,0.1,45",
        ]);
        let (shapes, skipped) = instantiate(&body, &[1.0, 2.0], 1_000_000, &settings).unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(shapes.len(), 6);
        assert_eq!(
            shapes[0],
            MacroShape::Circle {
                exposure: true,
                diameter: 1_000_000,
                centre: P { x: 0, y: 0 },
                rotation: 0
            }
        );
        assert_eq!(
            shapes[1],
            MacroShape::VectorLine {
                exposure: true,
                width: 100_000,
                start: P { x: 0, y: 0 },
                end: P { x: 2_000_000, y: 0 },
                rotation: 45_000_000
            }
        );
        assert_eq!(
            shapes[3],
            MacroShape::CenterLine {
                exposure: true,
                width: 1_000_000,
                height: 2_000_000,
                centre: P {
                    x: 1_000_000,
                    y: 1_250_000
                },
                rotation: 0
            }
        );
        match &shapes[4] {
            MacroShape::Outline {
                points, rotation, ..
            } => assert_eq!((points.len(), *rotation), (4, 10_000_000)),
            other => panic!("{other:?}"),
        }
        match &shapes[5] {
            MacroShape::Polygon {
                vertices, diameter, ..
            } => assert_eq!((*vertices, *diameter), (6, 2_000_000)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_bad_primitive_is_refused() {
        let settings = ImportSettings::default();
        for body in [
            vec!["1,1,1"],
            vec!["99,1,1,1,1,1"],
            vec!["1,2,1,0,0"],
            vec!["4,1,2,0,0,1,1,0"],
            vec!["5,1,2,0,0,1"],
            vec!["$x=1"],
            vec!["$1 1"],
            vec!["z,1"],
        ] {
            let list = statements(&body);
            assert!(
                instantiate(&list, &[], 1_000_000, &settings).is_err(),
                "{body:?}"
            );
        }
        let small = ImportSettings {
            max_macro_primitives: 2,
            ..ImportSettings::default()
        };
        let list = statements(&["1,1,1,0,0", "1,1,1,1,1", "1,1,1,2,2"]);
        let e = instantiate(&list, &[], 1_000_000, &small).unwrap_err();
        assert!(e.contains("limit"), "{e}");
    }
}
