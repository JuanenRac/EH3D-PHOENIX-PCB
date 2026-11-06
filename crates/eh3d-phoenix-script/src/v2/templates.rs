// SPDX-License-Identifier: GPL-3.0-or-later
//! The example scripts of `examples/scripts/`, built into the program so that `phoenix script list`
//! can show them and `phoenix script list --show NAME` can print one. Each comes with the output it
//! gives on the board, schematic or library the tests of this crate build for it, so an example
//! cannot drift from the language.

use crate::v2::syntax::Target;

/// An example script.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    /// The name, which is also the name of the file without its extension.
    pub name: &'static str,
    /// The kind of document it works on.
    pub target: Target,
    /// What it does, in one line.
    pub summary: &'static str,
    /// The text of the script.
    pub source: &'static str,
    /// The output it gives on the document of the tests.
    pub expected: &'static str,
}

/// The examples, in alphabetical order of their names.
pub static TEMPLATES: &[Template] = &[
    Template {
        name: "bom_resistors",
        target: Target::Board,
        summary: "A bill of materials of the resistors as CSV text, written to the data folder when applied",
        source: include_str!("../../../../examples/scripts/bom_resistors.phs"),
        expected: include_str!("../../../../examples/scripts/bom_resistors.expected"),
    },
    Template {
        name: "circle_placement",
        target: Target::Board,
        summary: "Places components on a circle of a radius around a centre, in integer arithmetic",
        source: include_str!("../../../../examples/scripts/circle_placement.phs"),
        expected: include_str!("../../../../examples/scripts/circle_placement.expected"),
    },
    Template {
        name: "decoupling_check",
        target: Target::Board,
        summary: "Reports the capacitors that are too far from the nearest IC (changes nothing)",
        source: include_str!("../../../../examples/scripts/decoupling_check.phs"),
        expected: include_str!("../../../../examples/scripts/decoupling_check.expected"),
    },
    Template {
        name: "header_footprint",
        target: Target::Library,
        summary: "Makes the footprint of a single-row pin header of N pins in a library",
        source: include_str!("../../../../examples/scripts/header_footprint.phs"),
        expected: include_str!("../../../../examples/scripts/header_footprint.expected"),
    },
    Template {
        name: "led_matrix",
        target: Target::Board,
        summary: "Places an N x M matrix of LEDs on a pitch, with consecutive designators and a common net",
        source: include_str!("../../../../examples/scripts/led_matrix.phs"),
        expected: include_str!("../../../../examples/scripts/led_matrix.expected"),
    },
    Template {
        name: "panelize_matrix",
        target: Target::Board,
        summary: "Panelizes the board as an N x M matrix of copies",
        source: include_str!("../../../../examples/scripts/panelize_matrix.phs"),
        expected: include_str!("../../../../examples/scripts/panelize_matrix.expected"),
    },
    Template {
        name: "renumber_by_position",
        target: Target::Board,
        summary: "Renumbers the designators of a family from left to right",
        source: include_str!("../../../../examples/scripts/renumber_by_position.phs"),
        expected: include_str!("../../../../examples/scripts/renumber_by_position.expected"),
    },
];

/// The example with a name.
pub fn find(name: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.name == name)
}

/// The text of the expected output of an example with the line endings of the program.
pub fn expected_output(t: &Template) -> String {
    t.expected.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_of_the_examples_holds_each_script_and_its_output() {
        let doc = include_str!("../../../../docs/SCRIPT_EXAMPLES.md").replace("\r\n", "\n");
        for t in TEMPLATES {
            assert!(doc.contains(&t.source.replace("\r\n", "\n")), "{}", t.name);
            assert!(doc.contains(&expected_output(t)), "{}", t.name);
        }
    }

    #[test]
    fn an_example_is_found_by_its_name() {
        assert_eq!(find("led_matrix").map(|t| t.target), Some(Target::Board));
        assert_eq!(
            find("header_footprint").map(|t| t.target),
            Some(Target::Library)
        );
        assert!(find("nothing").is_none());
        for t in TEMPLATES {
            assert!(t.source.starts_with("phoenix-script/2 "), "{}", t.name);
            assert!(t.expected.ends_with('\n'), "{}", t.name);
        }
    }
}
