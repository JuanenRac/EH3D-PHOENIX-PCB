// SPDX-License-Identifier: GPL-3.0-or-later
//! The values of a run: whole numbers (64 bits), real numbers, texts, arrays and handles on the
//! objects of the design. A character is a whole number, the code of the letter.

use crate::world::Obj;

/// A value of the program.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A whole number; also a character.
    Int(i64),
    /// A real number. It is never infinite and never not-a-number: an operation that would make
    /// one is an error.
    Real(f64),
    /// A text.
    Str(String),
    /// An array; every element has the type of the array.
    Array(Vec<Value>),
    /// A handle on an object of the design.
    Obj(Obj),
    /// No value: a function without a result, an object variable that has not been set.
    Void,
}

impl Value {
    /// The bytes the value counts for in the memory budget.
    pub fn size(&self) -> usize {
        match self {
            Value::Str(s) => 24 + s.len(),
            Value::Array(items) => 24 + items.iter().map(Value::size).sum::<usize>(),
            _ => 16,
        }
    }

    /// The name of the type, for messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Real(_) => "real",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Obj(_) => "object",
            Value::Void => "void",
        }
    }
}

/// The declared type of a variable, a parameter or the result of a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// `int` and `char`.
    Int,
    /// `real`.
    Real,
    /// `string`.
    Str,
    /// A type that starts with `UL_`.
    Obj,
    /// `void`.
    Void,
}

impl Ty {
    /// The type a name of the language stands for.
    pub fn of(name: &str) -> Ty {
        match name {
            "int" | "char" => Ty::Int,
            "real" => Ty::Real,
            "string" => Ty::Str,
            "void" => Ty::Void,
            _ => Ty::Obj,
        }
    }

    /// The value a variable of this type starts with.
    pub fn default_value(self) -> Value {
        match self {
            Ty::Int => Value::Int(0),
            Ty::Real => Value::Real(0.0),
            Ty::Str => Value::Str(String::new()),
            Ty::Obj | Ty::Void => Value::Void,
        }
    }
}
