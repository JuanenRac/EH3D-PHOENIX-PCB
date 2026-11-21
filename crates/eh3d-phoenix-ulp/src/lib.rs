// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader and the evaluator of the user programs of the reference tool: files with the
//! extension `.ulp` that read the design and write reports, bills of materials and exports. This
//! is the first three steps of the plan of `docs/adr/0011-ulp-interpreter-for-the-owner-list.md`:
//! the compatibility report, the reader and the evaluator of the part that only computes and reads.
//! The sandbox in a process of its own, the fourth step, is **not written**.
//!
//! * [`lexer`]: a program as tokens with line and column, with limits of size, tokens and the
//!   length of names and texts;
//! * [`parser`]: the tokens as a syntax tree ([`ast`]) with limits of nesting, chains and
//!   statements; a mistake is a problem with its place and the reading goes on;
//! * [`uses`]: what a program uses (directives, kinds of statement, types, functions, members and
//!   named values), with counts and first places;
//! * [`support`]: the list of names and what is decided for each one, a data file
//!   (`config/ulp_support.cfg`) and not code;
//! * [`eval`], [`builtins`], [`world`] and [`value`]: the evaluator. It runs pure computation (numbers,
//!   texts, arrays, functions of the program, the usual statements) and read-only walks over the
//!   model (`board(B)`, `B.elements(E)`), with a budget of steps, memory, nesting and output; it
//!   opens no file, starts no program and writes nothing to the model. [`evaluable`] says which
//!   names it runs;
//! * [`report`]: the compatibility report with a verdict, as text in eight languages and as JSON;
//! * [`settings`] and [`codes`]: every limit, read from `config/ulp.cfg`, and the codes
//!   `PHOENIX-ULP-<NNN>` with their titles in the eight languages.
//!
//! The names and meanings of the language were taken from its public description and nothing is
//! copied from it; no program of a third party is stored, and the tests use small programs written
//! here. No claim of compatibility with the language as a whole is made: the verdicts say what the
//! list of names covers. **State: prototype.**
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ast;
pub mod builtins;
pub mod codes;
pub mod eval;
pub mod evaluable;
pub mod lexer;
pub mod parser;
pub mod report;
pub mod runenv;
pub mod settings;
pub mod support;
pub mod uses;
pub mod value;
pub mod world;

use std::io::Read;
use std::path::Path;

pub use codes::{title, Diag, UlpError};
pub use eval::{run, run_source, run_source_with, run_with, Control, Dialog, Outcome};
pub use evaluable::evaluates;
pub use parser::{parse, Parsed};
pub use report::{analyse, report_json, report_text, Report, Verdict, FORMAT};
pub use runenv::RunEnv;
pub use settings::UlpSettings;
pub use support::{Status, Support, UseKind};
pub use value::Value;
pub use world::{Obj, World};

/// Reads a file as text, at most `max` bytes. A file larger than that is refused and a file
/// that is not valid UTF-8 is read with the characters that cannot be read replaced, because
/// programs written years ago are often not UTF-8 and only their comments hold such letters.
pub fn read_text_limited(path: &Path, max: usize) -> Result<String, Diag> {
    let shown = path.display();
    let file = std::fs::File::open(path)
        .map_err(|e| Diag::new(codes::INPUT_UNREADABLE, format!("{shown}: {e}")))?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Diag::new(codes::INPUT_UNREADABLE, format!("{shown}: {e}")))?;
    if bytes.len() > max {
        return Err(Diag::new(
            codes::SOURCE_TOO_LARGE,
            format!("{shown}: more than {max} bytes"),
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_with_a_limit_and_a_wrong_byte_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("a.ulp");
        std::fs::write(&ok, b"// caf\xe9\nint a;\n").unwrap();
        let text = read_text_limited(&ok, 100).unwrap();
        assert!(text.contains("int a;") && text.contains('\u{fffd}'));
        assert_eq!(
            read_text_limited(&ok, 5).unwrap_err().code,
            codes::SOURCE_TOO_LARGE
        );
        assert_eq!(
            read_text_limited(&dir.path().join("none.ulp"), 100)
                .unwrap_err()
                .code,
            codes::INPUT_UNREADABLE
        );
    }
}
