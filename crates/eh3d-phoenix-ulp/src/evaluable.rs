// SPDX-License-Identifier: GPL-3.0-or-later
//! What the evaluator runs, by name. The compatibility report asks here for each name a program
//! uses, so that it can say whether `phoenix ulp run` is able to run the program, apart from what
//! the list of allowed names (`config/ulp_support.cfg`) says. A name can be allowed by the list and
//! not be written in the evaluator yet (the clock, the sort, the dialogs); the report then says
//! "allowed" and "not run" and does not hide either.

use crate::builtins::{PURE, WITH_VARIABLES};
use crate::eval::{is_dialog_name, CONSTANTS, IO_FUNCTIONS, RUN_FUNCTIONS};
use crate::support::UseKind;
use crate::world::{MEMBERS, WALKS};

/// Whether the evaluator runs a name of the given kind.
pub fn evaluates(kind: UseKind, name: &str) -> bool {
    match kind {
        UseKind::Directive => matches!(name, "usage" | "require" | "include"),
        UseKind::Statement => matches!(
            name,
            "block"
                | "declaration"
                | "expression"
                | "function"
                | "if"
                | "for"
                | "while"
                | "do"
                | "switch"
                | "case"
                | "default"
                | "break"
                | "continue"
                | "return"
                | "loop"
        ),
        UseKind::Type => {
            matches!(name, "char" | "int" | "real" | "string") || name.starts_with("UL_")
        }
        UseKind::Function => {
            PURE.contains(&name)
                || WITH_VARIABLES.contains(&name)
                || RUN_FUNCTIONS.contains(&name)
                || IO_FUNCTIONS.contains(&name)
                || is_dialog_name(name)
                || matches!(name, "board" | "schematic" | "sheet")
        }
        UseKind::Member => WALKS.contains(&name) || MEMBERS.contains(&name),
        UseKind::Constant => CONSTANTS.contains(&name) || crate::constants::lookup(name).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{Status, Support};

    #[test]
    fn what_the_evaluator_runs_is_known_by_name() {
        assert!(evaluates(UseKind::Function, "sqrt"));
        assert!(evaluates(UseKind::Function, "printf"));
        assert!(evaluates(UseKind::Function, "board"));
        assert!(evaluates(UseKind::Member, "elements"));
        assert!(evaluates(UseKind::Member, "x"));
        assert!(evaluates(UseKind::Constant, "PI"));
        assert!(evaluates(UseKind::Type, "UL_ELEMENT"));
        assert!(evaluates(UseKind::Directive, "usage"));
        assert!(evaluates(UseKind::Directive, "include"));
        assert!(!evaluates(UseKind::Directive, "pragma"));
        assert!(evaluates(UseKind::Function, "strxstr"));
        assert!(evaluates(UseKind::Function, "country"));
        assert!(!evaluates(UseKind::Function, "system"));
        assert!(evaluates(UseKind::Function, "output"));
        assert!(evaluates(UseKind::Function, "time"));
        assert!(evaluates(UseKind::Function, "dlgLabel"));
        assert!(!evaluates(UseKind::Function, "dlgNoSuchControl"));
        assert!(!evaluates(UseKind::Constant, "UL_WIRE"));
    }

    #[test]
    fn nothing_the_list_denies_is_run() {
        let list = Support::shipped();
        for kind in UseKind::ALL {
            for name in ["system", "sleep"] {
                if list.lookup(kind, name) == Some(Status::Denied) {
                    assert!(!evaluates(kind, name), "{name}");
                }
            }
        }
    }
}
