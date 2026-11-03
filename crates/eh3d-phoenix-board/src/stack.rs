//! The checks of the ordered stack of copper layers (`Board::copper_stack`).
//!
//! The stack says which copper layer is the top one, which are inner and which is the bottom one.
//! It may be empty (the order is not given); when it is not, it must list every copper layer of
//! the board exactly once and nothing else.

use std::collections::BTreeSet;

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

use crate::model::{Board, LayerKind};

/// The stack names a layer that does not exist, is not copper or is listed twice.
pub const STACK_ENTRY_INVALID: &str = "PHOENIX-BOARD-012";
/// The stack is given but leaves out a copper layer of the board.
pub const STACK_INCOMPLETE: &str = "PHOENIX-BOARD-013";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (STACK_ENTRY_INVALID, "Copper stack entry is invalid"),
    (STACK_INCOMPLETE, "Copper stack leaves out a copper layer"),
];

fn make(code: &'static str, detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, Severity::Error, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Geometric)
        .with_recoverability(Recoverability::Unrecoverable)
}

impl Board {
    /// The problems of the copper stack, in a fixed order. Nothing when the stack is empty.
    pub fn stack_problems(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        if self.copper_stack.is_empty() {
            return out;
        }
        let mut seen = BTreeSet::new();
        for layer in &self.copper_stack {
            match self.layers.get(layer) {
                None => out.push(make(
                    STACK_ENTRY_INVALID,
                    format!("layer {layer} does not exist"),
                )),
                Some(l) if l.kind != LayerKind::Copper => out.push(make(
                    STACK_ENTRY_INVALID,
                    format!("layer {layer} is not a copper layer"),
                )),
                Some(_) => {}
            }
            if !seen.insert(*layer) {
                out.push(make(
                    STACK_ENTRY_INVALID,
                    format!("layer {layer} is listed twice"),
                ));
            }
        }
        for (id, layer) in &self.layers {
            if layer.kind == LayerKind::Copper && !seen.contains(id) {
                out.push(make(
                    STACK_INCOMPLETE,
                    format!("copper layer {id} is not in the stack"),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Id, Layer};
    use crate::ops::Operation;
    use eh3d_phoenix_model::ObjectId;

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    fn board() -> (Board, [Id; 4]) {
        let ids = [id(1), id(2), id(3), id(4)];
        let mut b = Board::default();
        for (i, kind) in [
            (0, LayerKind::Copper),
            (1, LayerKind::Copper),
            (2, LayerKind::Copper),
            (3, LayerKind::Silkscreen),
        ] {
            b.layers.insert(
                ids[i],
                Layer {
                    name: format!("L{i}"),
                    kind,
                    side: None,
                },
            );
        }
        (b, ids)
    }

    #[test]
    fn an_empty_stack_is_allowed_and_a_complete_one_is_clean() {
        let (mut b, ids) = board();
        assert!(b.stack_problems().is_empty());
        assert!(b
            .validate()
            .iter()
            .all(|d| d.code.as_deref() != Some(STACK_INCOMPLETE)));
        b.copper_stack = vec![ids[0], ids[1], ids[2]];
        assert!(b.stack_problems().is_empty());
        assert!(b.is_consistent());
    }

    #[test]
    fn the_place_in_the_stack_gives_the_function() {
        use crate::model::StackFunction::{Bottom, Inner, Top};
        let (mut b, ids) = board();
        assert_eq!(b.stack_position(ids[0]), None);
        b.copper_stack = vec![ids[2]];
        assert_eq!(b.stack_position(ids[2]), Some((1, Top)));
        b.copper_stack = vec![ids[0], ids[1], ids[2]];
        assert_eq!(b.stack_position(ids[0]), Some((1, Top)));
        assert_eq!(b.stack_position(ids[1]), Some((2, Inner)));
        assert_eq!(b.stack_position(ids[2]), Some((3, Bottom)));
        assert_eq!(b.stack_position(ids[3]), None);
    }

    #[test]
    fn each_kind_of_problem_has_its_code() {
        let (mut b, ids) = board();
        let codes = |b: &Board| -> Vec<String> {
            b.stack_problems()
                .iter()
                .filter_map(|d| d.code.as_deref().map(str::to_owned))
                .collect()
        };
        b.copper_stack = vec![ids[0], ids[1]];
        assert_eq!(codes(&b), vec![STACK_INCOMPLETE]);
        b.copper_stack = vec![ids[0], ids[1], ids[2], ids[3]];
        assert_eq!(codes(&b), vec![STACK_ENTRY_INVALID]);
        b.copper_stack = vec![ids[0], ids[1], ids[2], id(99)];
        assert_eq!(codes(&b), vec![STACK_ENTRY_INVALID]);
        b.copper_stack = vec![ids[0], ids[1], ids[2], ids[0]];
        assert_eq!(codes(&b), vec![STACK_ENTRY_INVALID]);
        assert!(!b.is_consistent());
    }

    #[test]
    fn setting_the_stack_is_an_operation_with_an_exact_inverse() {
        let (mut b, ids) = board();
        let before = b.clone();
        let undo = b
            .apply(&Operation::SetCopperStack {
                stack: vec![ids[0], ids[1], ids[2]],
            })
            .unwrap();
        assert_eq!(b.copper_stack.len(), 3);
        b.apply(&undo).unwrap();
        assert_eq!(b, before);
        // refused: a missing layer, a layer that is not copper, a repeated layer; the board stays
        for bad in [vec![id(99)], vec![ids[3]], vec![ids[0], ids[0]]] {
            assert!(b.apply(&Operation::SetCopperStack { stack: bad }).is_err());
            assert_eq!(b, before);
        }
    }

    #[test]
    fn the_stack_is_additive_in_the_file() {
        let (mut b, ids) = board();
        let without = String::from_utf8(b.to_bytes().unwrap()).unwrap();
        assert!(
            !without.contains("copper_stack"),
            "an empty stack is not written"
        );
        assert_eq!(Board::from_bytes(without.as_bytes()).unwrap(), b);
        b.copper_stack = vec![ids[0], ids[1], ids[2]];
        let with = b.to_bytes().unwrap();
        assert!(String::from_utf8(with.clone())
            .unwrap()
            .contains("copper_stack"));
        assert_eq!(Board::from_bytes(&with).unwrap(), b);
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
