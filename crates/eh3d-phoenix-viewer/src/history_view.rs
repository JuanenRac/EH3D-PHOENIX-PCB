// SPDX-License-Identifier: GPL-3.0-or-later
//! The history of the open document as a list that can be read, without a screen: the steps that
//! were done and the ones that were undone, each with a short description (`Move R1`, `Route net
//! GND`), the place where the document is in it and how many undos or redos it takes to go to a
//! step. The steps are the ones the editing already keeps: this module only reads them.

use eh3d_phoenix_board::{Board, Id, Operation, Step};
use eh3d_phoenix_rules::Lang;

use crate::session::{BoardSession, SchematicSession};
use crate::window_text::WinText;

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    /// What the step did.
    pub label: String,
    /// True for a step that is done, false for one that was undone and can be redone.
    pub done: bool,
}

/// What to do to go to a step of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Nothing: the document is already there.
    Stay,
    /// Undo this many steps.
    Undo(usize),
    /// Redo this many steps.
    Redo(usize),
}

/// The list of steps: the ones that are done, oldest first, and then the ones that were undone,
/// the next one to redo first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryView {
    /// The lines.
    pub rows: Vec<HistoryRow>,
    /// How many older steps are done but not listed, because the list is limited.
    pub hidden: usize,
    current: usize,
}

impl HistoryView {
    /// The list from the descriptions of the steps that are done (oldest first) and of the ones that
    /// were undone (the next to redo first). At most `max` of each are listed (the newest of the
    /// done ones).
    pub fn new(done: Vec<String>, undone: Vec<String>, max: usize) -> HistoryView {
        let max = max.max(1);
        let hidden = done.len().saturating_sub(max);
        let mut rows: Vec<HistoryRow> = done
            .into_iter()
            .skip(hidden)
            .map(|label| HistoryRow { label, done: true })
            .collect();
        let current = rows.len();
        rows.extend(
            undone
                .into_iter()
                .take(max)
                .map(|label| HistoryRow { label, done: false }),
        );
        HistoryView {
            rows,
            hidden,
            current,
        }
    }

    /// The list of a board that is edited.
    pub fn of_board(session: &BoardSession, lang: Lang, max: usize) -> HistoryView {
        let describe = |step: &Step| describe_step(step, &session.board, lang);
        HistoryView::new(
            session.steps_done().iter().map(describe).collect(),
            session.steps_undone().iter().rev().map(describe).collect(),
            max,
        )
    }

    /// The list of a schematic that is synchronised with a board.
    pub fn of_schematic(session: &SchematicSession, lang: Lang, max: usize) -> HistoryView {
        let describe = |to_board: &bool| {
            if *to_board {
                WinText::OpSyncToBoard.tr(lang).to_string()
            } else {
                WinText::OpSyncToSchematic.tr(lang).to_string()
            }
        };
        HistoryView::new(
            session.steps_done().iter().map(describe).collect(),
            session.steps_undone().iter().rev().map(describe).collect(),
            max,
        )
    }

    /// How many of the listed steps are done.
    pub fn done_count(&self) -> usize {
        self.current
    }

    /// The description of the step that undo would take back.
    pub fn next_undo(&self) -> Option<&str> {
        self.current
            .checked_sub(1)
            .and_then(|at| self.rows.get(at))
            .map(|row| row.label.as_str())
    }

    /// The description of the step that redo would do again.
    pub fn next_redo(&self) -> Option<&str> {
        self.rows.get(self.current).map(|row| row.label.as_str())
    }

    /// What to do to leave the document as it was just after the step of a line.
    pub fn go_to(&self, row: usize) -> Move {
        if row >= self.rows.len() {
            Move::Stay
        } else if row + 1 < self.current {
            Move::Undo(self.current - 1 - row)
        } else if row >= self.current {
            Move::Redo(row + 1 - self.current)
        } else {
            Move::Stay
        }
    }

    /// What to do to leave the document as it was before the first step; there is nothing to do
    /// when older steps are not listed.
    pub fn go_to_start(&self) -> Move {
        if self.hidden > 0 || self.current == 0 {
            Move::Stay
        } else {
            Move::Undo(self.current)
        }
    }
}

fn net_name(board: &Board, id: Id) -> String {
    board
        .nets
        .get(&id)
        .map_or_else(|| "?".to_string(), |net| net.name.clone())
}

fn fill(template: WinText, lang: Lang, name: &str) -> String {
    template.tr(lang).replace("{}", name)
}

/// What the operation that undoes a removal carries: the first component, net, footprint or item
/// it adds, looking inside a batch.
fn first_added(op: &Operation) -> Option<&Operation> {
    match op {
        Operation::AddComponent { .. }
        | Operation::AddNet { .. }
        | Operation::AddTrack { .. }
        | Operation::AddVia { .. }
        | Operation::AddZone { .. }
        | Operation::AddFootprint { .. } => Some(op),
        Operation::Batch { ops } => ops.iter().find_map(first_added),
        _ => None,
    }
}

/// The reference of a component that a step names: the board has it, or the step that undoes it
/// carries it.
fn refdes_of(board: &Board, id: Id, inverse: &Operation) -> String {
    if let Some(component) = board.components.get(&id) {
        return component.refdes.clone();
    }
    match first_added(inverse) {
        Some(Operation::AddComponent { component, .. }) => component.refdes.clone(),
        _ => "?".to_string(),
    }
}

/// The name of the net of a track, a via or a zone that a step removed: the step that undoes the
/// removal carries the item.
fn item_net(board: &Board, inverse: &Operation) -> String {
    let net = match first_added(inverse) {
        Some(Operation::AddTrack { track, .. }) => Some(track.net),
        Some(Operation::AddVia { via, .. }) => Some(via.net),
        Some(Operation::AddZone { zone, .. }) => Some(zone.net),
        _ => None,
    };
    net.map_or_else(|| "?".to_string(), |id| net_name(board, id))
}

/// A short description of one operation in a language. `board` is the board as it is now and
/// `inverse` the operation that undoes this one, which carries the names of what was removed.
pub fn describe_operation(
    op: &Operation,
    inverse: &Operation,
    board: &Board,
    lang: Lang,
) -> String {
    match op {
        Operation::AddNet { name, .. } => fill(WinText::OpAddNet, lang, name),
        Operation::RemoveNet { .. } => {
            let name = match first_added(inverse) {
                Some(Operation::AddNet { name, .. }) => name.clone(),
                _ => "?".to_string(),
            };
            fill(WinText::OpRemoveNet, lang, &name)
        }
        Operation::RenameNet { name, .. } => fill(WinText::OpRenameNet, lang, name),
        Operation::AddComponent { component, .. } => {
            fill(WinText::OpAddComponent, lang, &component.refdes)
        }
        Operation::RemoveComponent { id } => fill(
            WinText::OpRemoveComponent,
            lang,
            &refdes_of(board, *id, inverse),
        ),
        Operation::MoveComponent { id, .. } => fill(
            WinText::OpMoveComponent,
            lang,
            &refdes_of(board, *id, inverse),
        ),
        Operation::SetComponentFields { refdes, .. } => {
            fill(WinText::OpEditComponent, lang, refdes)
        }
        Operation::SetPadNet { pad, .. } => {
            let refdes = refdes_of(board, pad.component, inverse);
            let number = board
                .components
                .get(&pad.component)
                .and_then(|c| board.footprints.get(&c.footprint))
                .and_then(|f| f.pads.get(&pad.pad))
                .map(|p| p.number.clone());
            let name = match number {
                Some(number) => format!("{refdes}.{number}"),
                None => refdes,
            };
            fill(WinText::OpConnectPad, lang, &name)
        }
        Operation::AddTrack { track, .. } => {
            fill(WinText::OpAddTrack, lang, &net_name(board, track.net))
        }
        Operation::RemoveTrack { .. } => {
            fill(WinText::OpRemoveTrack, lang, &item_net(board, inverse))
        }
        Operation::AddVia { via, .. } => fill(WinText::OpAddVia, lang, &net_name(board, via.net)),
        Operation::RemoveVia { .. } => fill(WinText::OpRemoveVia, lang, &item_net(board, inverse)),
        Operation::AddZone { zone, .. } => {
            fill(WinText::OpAddZone, lang, &net_name(board, zone.net))
        }
        Operation::RemoveZone { .. } => {
            fill(WinText::OpRemoveZone, lang, &item_net(board, inverse))
        }
        Operation::AddFootprint { footprint, .. } => {
            fill(WinText::OpAddFootprint, lang, &footprint.name)
        }
        Operation::RemoveFootprint { .. } => {
            let name = match first_added(inverse) {
                Some(Operation::AddFootprint { footprint, .. }) => footprint.name.clone(),
                _ => "?".to_string(),
            };
            fill(WinText::OpRemoveFootprint, lang, &name)
        }
        Operation::SetCopperStack { .. } => WinText::OpCopperStack.tr(lang).to_string(),
        Operation::AddSilk { .. } => WinText::OpAddSilk.tr(lang).to_string(),
        Operation::RemoveSilk { .. } => WinText::OpRemoveSilk.tr(lang).to_string(),
        Operation::Batch { ops } => {
            let inner_inverse = match inverse {
                Operation::Batch { ops: inverses } => inverses.last(),
                _ => None,
            };
            match (ops.first(), inner_inverse) {
                (Some(first), Some(first_inverse)) => fill(
                    WinText::OpBatch,
                    lang,
                    &describe_operation(first, first_inverse, board, lang),
                ),
                (Some(first), None) => fill(
                    WinText::OpBatch,
                    lang,
                    &describe_operation(first, &Operation::Batch { ops: Vec::new() }, board, lang),
                ),
                (None, _) => WinText::OpUnknown.tr(lang).to_string(),
            }
        }
        _ => WinText::OpUnknown.tr(lang).to_string(),
    }
}

/// A short description of one step of the history of a board.
pub fn describe_step(step: &Step, board: &Board, lang: Lang) -> String {
    describe_operation(&step.op, &step.inverse, board, lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::{self, id, point};
    use eh3d_phoenix_board::{History, PadRef, Segment, Track};
    use eh3d_phoenix_model::{Nm, Udeg};
    use eh3d_phoenix_route::RouteSettings;

    fn mv(x: f64) -> Operation {
        Operation::MoveComponent {
            id: id(30),
            at: point(x, 10.0),
            rotation: Udeg::ZERO,
        }
    }

    fn rows(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|l| (*l).to_string()).collect()
    }

    #[test]
    fn the_steps_are_described_with_the_names_of_what_they_touch() {
        let mut board = sample::board();
        let mut history = History::new();
        let new_track = Track {
            net: id(11),
            layer: id(1),
            width: Nm::new(200_000).unwrap(),
            path: vec![Segment::Line {
                start: point(1.0, 1.0),
                end: point(2.0, 1.0),
            }],
        };
        let ops = [
            mv(11.0),
            Operation::RemoveTrack { id: id(40) },
            Operation::AddTrack {
                id: id(50),
                track: new_track,
            },
            Operation::RenameNet {
                id: id(10),
                name: "GROUND".to_string(),
            },
            Operation::RemoveVia { id: id(42) },
            Operation::RemoveZone { id: id(43) },
            Operation::RemoveSilk { id: id(44) },
        ];
        for op in ops {
            history.execute(&mut board, op).unwrap();
        }
        let texts: Vec<String> = history
            .done()
            .iter()
            .map(|s| describe_step(s, &board, Lang::En))
            .collect();
        assert_eq!(
            texts,
            [
                "Move R1",
                "Delete track of GROUND",
                "Route net VCC",
                "Rename net GROUND",
                "Remove via of GROUND",
                "Remove zone of GROUND",
                "Remove silkscreen",
            ]
        );
    }

    #[test]
    fn a_connected_pad_is_named_by_its_reference_and_number() {
        let mut board = sample::board();
        let mut history = History::new();
        for pad in [id(21), id(22)] {
            let pad = PadRef {
                component: id(30),
                pad,
            };
            history
                .execute(&mut board, Operation::SetPadNet { pad, net: None })
                .unwrap();
        }
        let texts: Vec<String> = history
            .done()
            .iter()
            .map(|s| describe_step(s, &board, Lang::En))
            .collect();
        assert_eq!(texts, ["Connect pad R1.1", "Connect pad R1.2"]);
    }

    #[test]
    fn a_removed_component_is_named_by_the_step_that_undoes_it() {
        let mut board = sample::board();
        let mut history = History::new();
        history
            .execute(&mut board, Operation::RemoveComponent { id: id(30) })
            .unwrap();
        assert!(board.components.is_empty());
        let text = describe_step(&history.done()[0], &board, Lang::En);
        assert_eq!(text, "Remove R1");
        // a step that names what is gone shows a question mark and not a made-up name
        let lost = describe_operation(&mv(11.0), &mv(10.0), &board, Lang::En);
        assert_eq!(lost, "Move ?");
    }

    #[test]
    fn a_batch_is_named_after_its_first_step() {
        let mut board = sample::board();
        let mut history = History::new();
        history
            .execute(
                &mut board,
                Operation::Batch {
                    ops: vec![mv(11.0), mv(12.0)],
                },
            )
            .unwrap();
        let text = describe_step(&history.done()[0], &board, Lang::En);
        assert_eq!(text, "Move R1 (several changes)");
        let empty = describe_operation(
            &Operation::Batch { ops: Vec::new() },
            &Operation::Batch { ops: Vec::new() },
            &board,
            Lang::En,
        );
        assert_eq!(empty, "Edit");
    }

    #[test]
    fn every_language_names_the_component_in_a_step() {
        let board = sample::board();
        for lang in Lang::ALL {
            let text = describe_operation(&mv(11.0), &mv(10.0), &board, lang);
            assert!(text.contains("R1"), "{lang:?}: {text}");
            assert!(!text.contains("{}"), "{lang:?}: {text}");
        }
        let in_german = describe_operation(&mv(11.0), &mv(10.0), &board, Lang::De);
        assert_eq!(in_german, "R1 verschieben");
    }

    #[test]
    fn the_list_has_the_done_steps_then_the_undone_ones_and_a_place_in_it() {
        let view = HistoryView::new(rows(&["A", "B", "C"]), rows(&["D", "E"]), 10);
        let labels: Vec<&str> = view.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["A", "B", "C", "D", "E"]);
        assert_eq!(view.done_count(), 3);
        assert_eq!(view.hidden, 0);
        assert!(view.rows[2].done && !view.rows[3].done);
        assert_eq!(view.next_undo(), Some("C"));
        assert_eq!(view.next_redo(), Some("D"));
        let empty = HistoryView::new(Vec::new(), Vec::new(), 10);
        assert_eq!(empty.next_undo(), None);
        assert_eq!(empty.next_redo(), None);
        assert!(empty.rows.is_empty());
    }

    #[test]
    fn a_long_history_lists_the_newest_steps_and_says_how_many_it_left_out() {
        let done: Vec<String> = (0..7).map(|n| format!("s{n}")).collect();
        let undone: Vec<String> = (0..4).map(|n| format!("u{n}")).collect();
        let view = HistoryView::new(done, undone, 3);
        assert_eq!(view.hidden, 4);
        assert_eq!(view.done_count(), 3);
        assert_eq!(view.rows.len(), 6);
        assert_eq!(view.rows[0].label, "s4");
        assert_eq!(view.rows[3].label, "u0");
        assert_eq!(view.go_to_start(), Move::Stay);
        // a limit of zero still lists one
        assert_eq!(
            HistoryView::new(rows(&["a", "b"]), Vec::new(), 0)
                .rows
                .len(),
            1
        );
    }

    #[test]
    fn going_to_a_step_is_a_number_of_undos_or_redos() {
        let view = HistoryView::new(rows(&["A", "B", "C"]), rows(&["D", "E"]), 10);
        assert_eq!(view.go_to(0), Move::Undo(2));
        assert_eq!(view.go_to(1), Move::Undo(1));
        assert_eq!(view.go_to(2), Move::Stay);
        assert_eq!(view.go_to(3), Move::Redo(1));
        assert_eq!(view.go_to(4), Move::Redo(2));
        assert_eq!(view.go_to(5), Move::Stay);
        assert_eq!(view.go_to_start(), Move::Undo(3));
        assert_eq!(HistoryView::default().go_to_start(), Move::Stay);
    }

    #[test]
    fn the_list_of_a_board_follows_the_edits_the_undo_and_the_redo() {
        let relaxed = RouteSettings {
            strict_edits: false,
            ..RouteSettings::default()
        };
        let mut session = BoardSession::new(sample::board(), Default::default(), relaxed).unwrap();
        assert!(HistoryView::of_board(&session, Lang::En, 10)
            .rows
            .is_empty());
        session.apply(mv(10.5)).unwrap();
        session
            .apply(Operation::RenameNet {
                id: id(10),
                name: "G".to_string(),
            })
            .unwrap();
        let view = HistoryView::of_board(&session, Lang::En, 10);
        let labels: Vec<&str> = view.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Move R1", "Rename net G"]);
        assert!(session.undo());
        let view = HistoryView::of_board(&session, Lang::Es, 10);
        assert_eq!(view.done_count(), 1);
        assert_eq!(view.rows[0].label, "Mover R1");
        assert!(!view.rows[1].done);
        assert_eq!(view.rows[1].label, "Renombrar la red G");
        assert!(session.redo());
        assert_eq!(
            HistoryView::of_board(&session, Lang::En, 10).done_count(),
            2
        );
    }
}
