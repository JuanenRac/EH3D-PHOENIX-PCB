// SPDX-License-Identifier: GPL-3.0-or-later
//! The expressions between braces of a netlist, `{2*r1+1k}`: numbers with suffixes, names of
//! parameters, `+ - * / ^`, parentheses, the constant `pi` and the functions `sqrt abs exp ln
//! log10 sin cos`. The nesting is bounded by the settings.

use std::collections::BTreeMap;

use crate::units::parse_number;

/// Why an expression could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprError {
    /// The text is not an expression, or the result is not finite.
    Syntax(String),
    /// A name that is not a parameter.
    Undefined(String),
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    depth: usize,
    max_depth: usize,
    params: &'a BTreeMap<String, f64>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn enter(&mut self) -> Result<(), ExprError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(ExprError::Syntax(
                "the expression is nested too deeply".to_string(),
            ));
        }
        Ok(())
    }

    fn expression(&mut self) -> Result<f64, ExprError> {
        self.enter()?;
        let mut value = self.term()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    value += self.term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        self.depth -= 1;
        Ok(value)
    }

    fn term(&mut self) -> Result<f64, ExprError> {
        let mut value = self.unary()?;
        loop {
            self.skip_spaces();
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    value *= self.unary()?;
                }
                Some('/') => {
                    self.pos += 1;
                    value /= self.unary()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn unary(&mut self) -> Result<f64, ExprError> {
        self.skip_spaces();
        match self.peek() {
            Some('-') => {
                self.pos += 1;
                self.enter()?;
                let v = self.unary()?;
                self.depth -= 1;
                Ok(-v)
            }
            Some('+') => {
                self.pos += 1;
                self.enter()?;
                let v = self.unary()?;
                self.depth -= 1;
                Ok(v)
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<f64, ExprError> {
        let base = self.atom()?;
        self.skip_spaces();
        if self.peek() == Some('^') {
            self.pos += 1;
            self.enter()?;
            let exponent = self.unary()?;
            self.depth -= 1;
            return Ok(base.powf(exponent));
        }
        Ok(base)
    }

    fn atom(&mut self) -> Result<f64, ExprError> {
        self.skip_spaces();
        let Some(c) = self.peek() else {
            return Err(ExprError::Syntax(
                "the expression ends too soon".to_string(),
            ));
        };
        if c == '(' {
            self.pos += 1;
            let v = self.expression()?;
            self.skip_spaces();
            if self.peek() != Some(')') {
                return Err(ExprError::Syntax("a parenthesis is not closed".to_string()));
            }
            self.pos += 1;
            return Ok(v);
        }
        if c.is_ascii_digit() || c == '.' {
            return self.number();
        }
        if c.is_alphabetic() || c == '_' {
            let start = self.pos;
            while self
                .peek()
                .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
            {
                self.pos += 1;
            }
            let name: String = self.chars[start..self.pos].iter().collect();
            let name = name.to_lowercase();
            self.skip_spaces();
            if self.peek() == Some('(') {
                self.pos += 1;
                let argument = self.expression()?;
                self.skip_spaces();
                if self.peek() != Some(')') {
                    return Err(ExprError::Syntax("a parenthesis is not closed".to_string()));
                }
                self.pos += 1;
                return apply(&name, argument);
            }
            if name == "pi" {
                return Ok(std::f64::consts::PI);
            }
            return match self.params.get(&name) {
                Some(v) => Ok(*v),
                None => Err(ExprError::Undefined(name)),
            };
        }
        Err(ExprError::Syntax(format!("unexpected character '{c}'")))
    }

    fn number(&mut self) -> Result<f64, ExprError> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
            self.pos += 1;
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            let mut look = self.pos + 1;
            if matches!(self.chars.get(look).copied(), Some('+') | Some('-')) {
                look += 1;
            }
            if self.chars.get(look).is_some_and(char::is_ascii_digit) {
                while self.chars.get(look).is_some_and(char::is_ascii_digit) {
                    look += 1;
                }
                self.pos = look;
            }
        }
        while self.peek().is_some_and(char::is_alphabetic) {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        parse_number(&text).ok_or_else(|| ExprError::Syntax(format!("'{text}' is not a number")))
    }
}

fn apply(name: &str, x: f64) -> Result<f64, ExprError> {
    match name {
        "sqrt" => Ok(x.sqrt()),
        "abs" => Ok(x.abs()),
        "exp" => Ok(x.exp()),
        "ln" => Ok(x.ln()),
        "log10" => Ok(x.log10()),
        "sin" => Ok(x.sin()),
        "cos" => Ok(x.cos()),
        _ => Err(ExprError::Syntax(format!("unknown function {name}"))),
    }
}

/// Evaluates an expression with the parameters defined so far (names in lower case). The result
/// must be a finite number.
pub fn eval(
    text: &str,
    params: &BTreeMap<String, f64>,
    max_depth: usize,
) -> Result<f64, ExprError> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        pos: 0,
        depth: 0,
        max_depth,
        params,
    };
    let value = parser.expression()?;
    parser.skip_spaces();
    if parser.pos != parser.chars.len() {
        return Err(ExprError::Syntax(format!(
            "unexpected text after the expression: '{}'",
            parser.chars[parser.pos..].iter().collect::<String>()
        )));
    }
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ExprError::Syntax("the result is not finite".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> BTreeMap<String, f64> {
        let mut p = BTreeMap::new();
        p.insert("r1".to_string(), 1000.0);
        p.insert("c".to_string(), 1e-6);
        p
    }

    fn value(text: &str) -> f64 {
        eval(text, &params(), 32).unwrap_or_else(|e| panic!("{text}: {e:?}"))
    }

    #[test]
    fn arithmetic_follows_the_usual_precedence() {
        assert_eq!(value("1+2*3"), 7.0);
        assert_eq!(value("(1+2)*3"), 9.0);
        assert_eq!(value("10/4"), 2.5);
        assert_eq!(value("2^3^2"), 512.0);
        assert_eq!(value("-2^2"), -4.0);
        assert_eq!(value("- -3"), 3.0);
        assert_eq!(value(" 1 + 1 "), 2.0);
    }

    #[test]
    fn numbers_keep_their_suffixes_and_names_are_looked_up() {
        assert!((value("2*r1+1k") - 3000.0).abs() < 1e-9);
        assert!(
            (value("1/(2*pi*r1*c)") - 1.0 / (2.0 * std::f64::consts::PI * 1000.0 * 1e-6)).abs()
                < 1e-9
        );
        assert!((value("R1*1meg") - 1e9).abs() < 1.0);
        assert_eq!(value("1e3+1"), 1001.0);
    }

    #[test]
    fn functions_are_applied() {
        assert_eq!(value("sqrt(16)"), 4.0);
        assert_eq!(value("abs(-2.5)"), 2.5);
        assert!((value("exp(1)") - std::f64::consts::E).abs() < 1e-12);
        assert!((value("ln(exp(2))") - 2.0).abs() < 1e-12);
        assert!((value("log10(1000)") - 3.0).abs() < 1e-12);
        assert!((value("sin(pi/2)") - 1.0).abs() < 1e-12);
        assert!((value("cos(0)") - 1.0).abs() < 1e-12);
    }

    #[test]
    fn errors_say_what_is_wrong() {
        let p = params();
        assert_eq!(
            eval("zz+1", &p, 32),
            Err(ExprError::Undefined("zz".to_string()))
        );
        for text in [
            "", "1+", "(1", "1)", "1 2", "foo(1)", "1/0", "sqrt(-1)", "#",
        ] {
            assert!(
                matches!(eval(text, &p, 32), Err(ExprError::Syntax(_))),
                "{text}"
            );
        }
    }

    #[test]
    fn nesting_is_bounded() {
        let deep = format!("{}1{}", "(".repeat(40), ")".repeat(40));
        assert!(eval(&deep, &params(), 32).is_err());
        assert_eq!(eval(&deep, &params(), 64), Ok(1.0));
    }
}
