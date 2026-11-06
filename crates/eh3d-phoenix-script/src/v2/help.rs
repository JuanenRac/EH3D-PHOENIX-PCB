// SPDX-License-Identifier: GPL-3.0-or-later
//! The reference of the language of the scripts of the second version, written from the one table
//! of functions (`phoenix script help`). The part about the language is the same text in
//! `docs/SCRIPT_API.md`; the part about the functions is not a list of its own: it is the table the
//! interpreter and the static check use.

use crate::v2::funcs::{groups, targets_text};

/// The language, in a few lines.
pub const LANGUAGE: &str = "\
phoenix-script/2 - the language of the scripts of the second version

The first significant line is the header: phoenix-script/2, optionally followed by board,
schematic or library (the kind of document the script is written for). A statement ends at a line
break or at a semicolon; a # starts a comment; a block is written between braces, and else
follows the closing brace on the same line.

Values: integers (64 bits), true and false, nil, texts in single or double quotes (\\n and \\t are
understood), lists written [a, b] and objects of the document (components, tracks, vias, zones,
nets, parts). A length is written with a unit and is the integer count of nanometres: 0.2mm,
8mil, 1in, 25um. An angle is written with deg and is the integer count of micro-degrees: 90deg,
22.5deg. A decimal number without a unit does not exist.

Operators, from the loosest: or, and, not, == != < <= > >=, + - (also + joins two texts), * / %
(integer division towards zero), unary minus, then a[i] (a list or a text, from 0) and
object.field. and and or evaluate the right side only when needed.

Statements:
  let name = expression        defines a variable in the block (a name once per block)
  name = expression            changes a variable that exists
  if c { ... } else if d { ... } else { ... }
  for i in first..end { ... }  the numbers first, first+1, ... end-1
  for item in list { ... }
  while c { ... }              break and continue work in both loops
  any expression               usually a call: name(arguments)

Limits (config/script.cfg): max_steps, max_loop_iterations, max_block_depth, max_expr_depth,
max_value_cells, max_list_len, max_text_bytes, max_output_bytes, max_operations, max_data_files and
max_data_bytes. A script that passes one is stopped with an error and nothing of it is kept.

The document is changed through the reversible operations of the editors. The run works on a copy
and is all or nothing: the first error ends it and the document is not touched; a run that ends
well is one step of the history. Nothing is written unless the run is applied with --out, and then
only to a new file.
";

/// The functions of one group as lines of text.
fn group_text(heading: &str, table: &[crate::v2::funcs::FuncDef]) -> String {
    let mut out = format!("\n{heading}\n");
    for f in table {
        out.push_str(&format!("  {}\n", f.sig));
        let mut note = String::new();
        if f.targets != 0 {
            note.push_str(&format!(" [applies to: {}]", targets_text(f.targets)));
        }
        if f.op {
            note.push_str(" [changes the document]");
        }
        out.push_str(&format!("      {}{}\n", f.doc, note));
    }
    out
}

/// The whole reference: the language and the functions.
pub fn reference() -> String {
    let mut out = LANGUAGE.to_owned();
    for (heading, table) in groups() {
        out.push_str(&group_text(heading, table));
    }
    out
}

/// The signature, the documents and the description of every function, in the order of the table.
pub fn functions() -> Vec<(String, String, String)> {
    groups()
        .into_iter()
        .flat_map(|(_, table)| table.iter())
        .map(|f| (f.sig.to_owned(), targets_text(f.targets), f.doc.to_owned()))
        .collect()
}

/// The names of the functions, in the order of the table.
pub fn function_names() -> Vec<&'static str> {
    groups()
        .into_iter()
        .flat_map(|(_, table)| table.iter())
        .map(|f| f.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_lists_every_function_of_the_table() {
        let text = reference();
        for (sig, _, doc) in functions() {
            assert!(text.contains(&sig), "{sig}");
            assert!(text.contains(&doc), "{sig}");
        }
        assert!(text.starts_with("phoenix-script/2"));
        assert!(text.contains("[changes the document]"));
        assert!(text.contains("[applies to: board]"));
        assert_eq!(function_names().len(), functions().len());
    }

    #[test]
    fn the_guide_documents_every_function_and_the_limits() {
        let doc = include_str!("../../../../docs/SCRIPT_API.md");
        for (sig, _, _) in functions() {
            assert!(
                doc.contains(&format!("`{sig}`")),
                "SCRIPT_API.md lacks {sig}"
            );
        }
        for key in [
            "max_steps",
            "max_loop_iterations",
            "max_value_cells",
            "max_output_bytes",
            "data_dir",
        ] {
            assert!(doc.contains(key), "SCRIPT_API.md lacks {key}");
        }
    }
}
