// SPDX-License-Identifier: GPL-3.0-or-later
//! The editing actions that work on what is selected, for the menus and the buttons of the
//! window: which of them can be chosen now, and what each one does. The decisions are taken here,
//! in code that is tested without a screen.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Id, PadRef};
use eh3d_phoenix_model::Point;

use crate::hit::{pad_near, Picked};
use crate::ops::EditOp;
use crate::tool::EditState;

/// An action of the Edit menu of the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditAction {
    /// Tidies the tracks of the nets that the selection touches (every net when nothing is
    /// selected).
    TidyTracks,
    /// Joins the two selected tracks.
    JoinTracks,
    /// Puts the selected components on the grid.
    SnapToGrid,
    /// Makes the selected tracks wider by the step of the settings.
    WidenTracks,
    /// Makes the selected tracks narrower by the step of the settings.
    NarrowTracks,
    /// Takes the swap tool.
    SwapPads,
}

impl EditAction {
    /// Every action, in the order of the menu.
    pub const ALL: [EditAction; 6] = [
        EditAction::TidyTracks,
        EditAction::JoinTracks,
        EditAction::SnapToGrid,
        EditAction::WidenTracks,
        EditAction::NarrowTracks,
        EditAction::SwapPads,
    ];

    /// The name of the action in the files and the menus.
    pub fn key(self) -> &'static str {
        match self {
            EditAction::TidyTracks => "tidy_tracks",
            EditAction::JoinTracks => "join_tracks",
            EditAction::SnapToGrid => "snap_to_grid",
            EditAction::WidenTracks => "widen_tracks",
            EditAction::NarrowTracks => "narrow_tracks",
            EditAction::SwapPads => "swap_pads",
        }
    }
}

/// What a click of the swap tool did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapClick {
    /// No pad is under the pointer.
    NoPad,
    /// The first pad was taken; the second click chooses the other one.
    First(PadRef),
    /// The two pads were swapped.
    Swapped,
    /// The swap was refused; the reason is in `EditState::error`, and the first pad is dropped.
    Refused,
}

impl EditState {
    fn selected_tracks(&self) -> Vec<Id> {
        self.selection
            .iter()
            .filter_map(|p| match p {
                Picked::Track(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// The actions that can be chosen now.
    pub fn available_actions(&self) -> Vec<EditAction> {
        let tracks = self.selected_tracks();
        let board = self.board();
        EditAction::ALL
            .into_iter()
            .filter(|a| match a {
                EditAction::TidyTracks => !board.tracks.is_empty(),
                EditAction::JoinTracks => tracks.len() == 2,
                EditAction::SnapToGrid => !self.selected_components().is_empty(),
                EditAction::WidenTracks | EditAction::NarrowTracks => !tracks.is_empty(),
                EditAction::SwapPads => board.pad_nets.len() >= 2,
            })
            .collect()
    }

    /// Does an action; false when it cannot be chosen or the board refused it (the reason is in
    /// `error`).
    pub fn run_action(&mut self, action: EditAction) -> bool {
        if !self.available_actions().contains(&action) {
            return false;
        }
        match action {
            EditAction::TidyTracks => {
                let nets: BTreeSet<Id> = self
                    .selected_tracks()
                    .iter()
                    .filter_map(|t| self.board().tracks.get(t).map(|t| t.net))
                    .collect();
                if nets.is_empty() {
                    return self.run(&EditOp::TidyTracks { net: None });
                }
                let mut any = false;
                for net in nets {
                    any |= self.run(&EditOp::TidyTracks { net: Some(net) });
                }
                any
            }
            EditAction::JoinTracks => {
                let t = self.selected_tracks();
                self.run(&EditOp::JoinTracks { a: t[0], b: t[1] })
            }
            EditAction::SnapToGrid => {
                let grid_nm = self.edit.settings.grid_nm;
                let components = self.selected_components();
                self.run(&EditOp::SnapToGrid {
                    components,
                    grid_nm,
                })
            }
            EditAction::WidenTracks => self.change_widths(1),
            EditAction::NarrowTracks => self.change_widths(-1),
            EditAction::SwapPads => {
                self.set_tool(crate::tool::Tool::SwapPads);
                true
            }
        }
    }

    fn change_widths(&mut self, direction: i64) -> bool {
        let step = self.edit.settings.width_step_nm;
        let mut any = false;
        for id in self.selected_tracks() {
            let Some(width) = self.board().tracks.get(&id).map(|t| t.width.get()) else {
                continue;
            };
            let width_nm = width + direction * step;
            if width_nm > 0 {
                any |= self.run(&EditOp::SetTrackWidth {
                    track: id,
                    width_nm,
                });
            }
        }
        any
    }

    /// A click of the swap tool: the first click takes a pad, the second swaps it with the pad
    /// that was clicked.
    pub fn swap_click(&mut self, at: Point, tolerance_nm: i64) -> SwapClick {
        let Some(pad) = pad_near(self.board(), at, tolerance_nm) else {
            return SwapClick::NoPad;
        };
        match self.swap_first.take() {
            None => {
                self.swap_first = Some(pad);
                SwapClick::First(pad)
            }
            Some(first) => {
                if self.run(&EditOp::SwapPads { a: first, b: pad }) {
                    SwapClick::Swapped
                } else {
                    SwapClick::Refused
                }
            }
        }
    }

    /// Cuts the track under a point, at that point (on the grid when snapping is on); false
    /// when no track is there or the point is not on a straight piece of it.
    pub fn split_track_near(&mut self, at: Point) -> bool {
        let at = self.snap(at);
        match self.pick_at(at) {
            Some(Picked::Track(track)) => self.run(&EditOp::SplitTrackAt { track, at }),
            _ => false,
        }
    }
}
