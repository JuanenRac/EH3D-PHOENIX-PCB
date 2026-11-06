// SPDX-License-Identifier: GPL-3.0-or-later
//! The selection of several objects in the window of the board editor, without a screen. The
//! editor of the window keeps its own set of what a click picks ([`Picked`]: components, tracks
//! and vias); the selection of `eh3d-phoenix-boardedit` ([`Selection`]) also holds zones and
//! silkscreen items. This module joins them without replacing the first: the picked set stays as
//! it was and the objects that a click cannot pick (zones and silkscreen items taken by a
//! rectangle or by a net) are kept beside it in [`Group`]. When an operation on the group is run,
//! both are turned into the items of one `Selection` and the operation is applied with
//! `BoardEdit::apply`, so it is one step of the history.
//!
//! It also turns keys into those operations ([`command_of`], [`op_of`]) and tells which gesture
//! a rectangle is ([`rect_mode`]), all of it testable without a window.

use std::collections::BTreeSet;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_boardedit::{AlignMode, EditOp, Item, Picked, RectMode, Selection};

/// The step of an arrow key held with shift, in steps of the grid.
pub const FAST_STEPS: i64 = 10;

/// The item that a picked thing is.
pub fn item_of(picked: Picked) -> Item {
    match picked {
        Picked::Component(id) => Item::Component(id),
        Picked::Track(id) => Item::Track(id),
        Picked::Via(id) => Item::Via(id),
    }
}

/// The picked thing that an item is; `None` for the zones and the silkscreen items, which a click
/// of the window does not pick.
pub fn picked_of(item: Item) -> Option<Picked> {
    match item {
        Item::Component(id) => Some(Picked::Component(id)),
        Item::Track(id) => Some(Picked::Track(id)),
        Item::Via(id) => Some(Picked::Via(id)),
        Item::Zone(_) | Item::Silk(_) => None,
    }
}

/// The objects that are selected besides the picked ones: the zones and the silkscreen items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Group {
    extra: Vec<Item>,
}

impl Group {
    /// An empty group.
    pub fn new() -> Group {
        Group::default()
    }

    /// The zones and silkscreen items that are selected.
    pub fn extra(&self) -> &[Item] {
        &self.extra
    }

    /// How many objects are selected in all.
    pub fn len(&self, picked: &BTreeSet<Picked>) -> usize {
        picked.len() + self.extra.len()
    }

    /// True when nothing is selected.
    pub fn is_empty(&self, picked: &BTreeSet<Picked>) -> bool {
        self.len(picked) == 0
    }

    /// Drops the objects that are not picked ones.
    pub fn drop_extra(&mut self) {
        self.extra.clear();
    }

    /// Drops the whole selection.
    pub fn clear(&mut self, picked: &mut BTreeSet<Picked>) {
        picked.clear();
        self.extra.clear();
    }

    /// Selects the items (the ones already selected stay).
    pub fn add_items(&mut self, picked: &mut BTreeSet<Picked>, items: &[Item]) {
        for item in items {
            match picked_of(*item) {
                Some(p) => {
                    picked.insert(p);
                }
                None => {
                    if !self.extra.contains(item) {
                        self.extra.push(*item);
                    }
                }
            }
        }
    }

    /// Takes away the items that are selected and selects those that are not.
    pub fn toggle_items(&mut self, picked: &mut BTreeSet<Picked>, items: &[Item]) {
        for item in items {
            match picked_of(*item) {
                Some(p) => {
                    if !picked.remove(&p) {
                        picked.insert(p);
                    }
                }
                None => match self.extra.iter().position(|e| e == item) {
                    Some(index) => {
                        self.extra.remove(index);
                    }
                    None => self.extra.push(*item),
                },
            }
        }
    }

    /// Drops the objects that the board no longer has.
    pub fn prune(&mut self, board: &Board) {
        self.extra.retain(|item| item.exists(board));
    }

    /// The items of the whole selection: the picked ones in their order, then the others.
    pub fn items(&self, picked: &BTreeSet<Picked>) -> Vec<Item> {
        let mut items: Vec<Item> = picked.iter().map(|p| item_of(*p)).collect();
        items.extend(self.extra.iter().copied());
        items
    }

    /// The selection of the editor that holds the same objects.
    pub fn selection(&self, picked: &BTreeSet<Picked>) -> Selection {
        let mut selection = Selection::new();
        for item in self.items(picked) {
            selection.add(item);
        }
        selection
    }

    /// Takes the objects of a selection as they are: the picked ones go to the set, the rest stay
    /// here.
    pub fn set_from(&mut self, picked: &mut BTreeSet<Picked>, selection: &Selection) {
        picked.clear();
        self.extra.clear();
        self.add_items(picked, selection.items());
    }
}

/// The rectangle of a drag: what lies wholly inside, or with alt also what it touches.
pub fn rect_mode(alt: bool) -> RectMode {
    if alt {
        RectMode::Crossed
    } else {
        RectMode::Contained
    }
}

/// True when the press starts a rectangle of the group instead of the rectangle of a plain drag:
/// a modifier is held (shift, ctrl or alt) and nothing is under the pointer.
pub fn starts_rectangle(shift: bool, command: bool, alt: bool, over_object: bool) -> bool {
    (shift || command || alt) && !over_object
}

/// True when a press with the modifiers toggles the thing under the pointer: shift or ctrl.
pub fn toggles(shift: bool, command: bool) -> bool {
    shift || command
}

/// The keys that matter for the group, read once per frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupKeys {
    /// The left arrow.
    pub left: bool,
    /// The right arrow.
    pub right: bool,
    /// The up arrow.
    pub up: bool,
    /// The down arrow.
    pub down: bool,
    /// The key A.
    pub a: bool,
    /// The key C.
    pub c: bool,
    /// The key M.
    pub m: bool,
    /// The key N.
    pub n: bool,
    /// The escape key.
    pub escape: bool,
    /// Shift is held.
    pub shift: bool,
    /// Ctrl (or the command key) is held.
    pub command: bool,
    /// Alt is held.
    pub alt: bool,
}

/// What a key does to the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupCommand {
    /// Moves the selection by steps of the grid (`dx`, `dy` are -1, 0 or 1; up is positive).
    Move {
        /// Steps along x.
        dx: i64,
        /// Steps along y.
        dy: i64,
        /// Ten steps instead of one.
        fast: bool,
    },
    /// Lines the selection up.
    Align(AlignMode),
    /// Selects every object of the net under the pointer.
    SelectNet,
    /// Selects every object of the board.
    SelectAll,
    /// Drops the selection.
    Clear,
}

/// The command that the keys of a frame ask for, when there is one. Arrows move the selection and,
/// with alt, line it up (left, right, top, bottom); alt+C and alt+M centre it along x and along
/// y; N selects the net under the pointer; ctrl+A selects all; escape drops the selection. The
/// moves and the alignments need a selection.
pub fn command_of(keys: &GroupKeys, has_selection: bool) -> Option<GroupCommand> {
    if keys.command {
        return keys.a.then_some(GroupCommand::SelectAll);
    }
    if keys.escape {
        return Some(GroupCommand::Clear);
    }
    if keys.n && !keys.alt {
        return Some(GroupCommand::SelectNet);
    }
    if !has_selection {
        return None;
    }
    if keys.alt {
        let mode = if keys.left {
            Some(AlignMode::Left)
        } else if keys.right {
            Some(AlignMode::Right)
        } else if keys.up {
            Some(AlignMode::Top)
        } else if keys.down {
            Some(AlignMode::Bottom)
        } else if keys.c {
            Some(AlignMode::CenterX)
        } else if keys.m {
            Some(AlignMode::CenterY)
        } else {
            None
        };
        return mode.map(GroupCommand::Align);
    }
    let dx = i64::from(keys.right) - i64::from(keys.left);
    let dy = i64::from(keys.up) - i64::from(keys.down);
    if dx == 0 && dy == 0 {
        return None;
    }
    Some(GroupCommand::Move {
        dx,
        dy,
        fast: keys.shift,
    })
}

/// The operation of the editor that a command is, when it is one: the step of the grid is
/// `grid_nm`. The selection commands are not operations of the board.
pub fn op_of(command: GroupCommand, grid_nm: i64) -> Option<EditOp> {
    match command {
        GroupCommand::Move { dx, dy, fast } => {
            let step = grid_nm
                .max(1)
                .saturating_mul(if fast { FAST_STEPS } else { 1 });
            Some(EditOp::MoveSelection {
                dx_nm: dx.saturating_mul(step),
                dy_nm: dy.saturating_mul(step),
                snap: Some(false),
            })
        }
        GroupCommand::Align(mode) => Some(EditOp::AlignSelection { mode }),
        GroupCommand::SelectNet | GroupCommand::SelectAll | GroupCommand::Clear => None,
    }
}

/// The alignments of the menu, in the order the buttons show them, with the key of each.
pub const ALIGN_ENTRIES: [(AlignMode, &str); 8] = [
    (AlignMode::Left, "Alt+Left"),
    (AlignMode::Right, "Alt+Right"),
    (AlignMode::Top, "Alt+Up"),
    (AlignMode::Bottom, "Alt+Down"),
    (AlignMode::CenterX, "Alt+C"),
    (AlignMode::CenterY, "Alt+M"),
    (AlignMode::DistributeX, ""),
    (AlignMode::DistributeY, ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::ObjectId;

    type Id = ObjectId;

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    #[test]
    fn picked_and_item_go_back_and_forth() {
        for picked in [
            Picked::Component(id(1)),
            Picked::Track(id(2)),
            Picked::Via(id(3)),
        ] {
            assert_eq!(picked_of(item_of(picked)), Some(picked));
        }
        assert_eq!(picked_of(Item::Zone(id(4))), None);
        assert_eq!(picked_of(Item::Silk(id(5))), None);
    }

    #[test]
    fn adding_keeps_zones_apart_and_never_twice() {
        let mut picked = BTreeSet::new();
        let mut group = Group::new();
        let items = [
            Item::Component(id(1)),
            Item::Zone(id(2)),
            Item::Silk(id(3)),
            Item::Zone(id(2)),
        ];
        group.add_items(&mut picked, &items);
        assert_eq!(picked.len(), 1);
        assert_eq!(group.extra(), &[Item::Zone(id(2)), Item::Silk(id(3))]);
        assert_eq!(group.len(&picked), 3);
        group.add_items(&mut picked, &items);
        assert_eq!(group.len(&picked), 3);
    }

    #[test]
    fn toggling_twice_gives_the_same_selection() {
        let mut picked = BTreeSet::new();
        let mut group = Group::new();
        let items = [Item::Track(id(1)), Item::Zone(id(2))];
        group.toggle_items(&mut picked, &items);
        assert_eq!(group.len(&picked), 2);
        group.toggle_items(&mut picked, &items);
        assert!(group.is_empty(&picked));
    }

    #[test]
    fn the_items_are_the_picked_then_the_others() {
        let mut picked = BTreeSet::new();
        let mut group = Group::new();
        group.add_items(
            &mut picked,
            &[Item::Silk(id(9)), Item::Via(id(2)), Item::Component(id(1))],
        );
        let items = group.items(&picked);
        assert_eq!(items.len(), 3);
        assert_eq!(items[2], Item::Silk(id(9)));
        assert!(items[..2].contains(&Item::Via(id(2))));
        let selection = group.selection(&picked);
        assert_eq!(selection.len(), 3);
        assert!(selection.contains(Item::Silk(id(9))));
    }

    #[test]
    fn a_selection_of_the_editor_is_split_between_the_picked_and_the_rest() {
        let mut selection = Selection::new();
        selection.add(Item::Zone(id(1)));
        selection.add(Item::Component(id(2)));
        let mut picked = BTreeSet::new();
        picked.insert(Picked::Via(id(7)));
        let mut group = Group::new();
        group.set_from(&mut picked, &selection);
        assert_eq!(picked.len(), 1);
        assert!(picked.contains(&Picked::Component(id(2))));
        assert_eq!(group.extra(), &[Item::Zone(id(1))]);
    }

    #[test]
    fn clearing_drops_everything() {
        let mut picked = BTreeSet::new();
        let mut group = Group::new();
        group.add_items(&mut picked, &[Item::Track(id(1)), Item::Zone(id(2))]);
        group.clear(&mut picked);
        assert!(group.is_empty(&picked));
    }

    #[test]
    fn pruning_drops_what_the_board_does_not_have() {
        let board = Board::default();
        let mut picked = BTreeSet::new();
        let mut group = Group::new();
        group.add_items(&mut picked, &[Item::Zone(id(1))]);
        group.prune(&board);
        assert!(group.extra().is_empty());
    }

    #[test]
    fn the_rectangle_is_wholly_inside_unless_alt_is_held() {
        assert_eq!(rect_mode(false), RectMode::Contained);
        assert_eq!(rect_mode(true), RectMode::Crossed);
    }

    #[test]
    fn a_rectangle_of_the_group_needs_a_modifier_and_an_empty_place() {
        assert!(starts_rectangle(true, false, false, false));
        assert!(starts_rectangle(false, true, false, false));
        assert!(starts_rectangle(false, false, true, false));
        assert!(!starts_rectangle(false, false, false, false));
        assert!(!starts_rectangle(true, false, false, true));
    }

    #[test]
    fn shift_and_ctrl_toggle_a_click() {
        assert!(toggles(true, false));
        assert!(toggles(false, true));
        assert!(!toggles(false, false));
    }

    #[test]
    fn an_arrow_moves_one_step_and_shift_ten() {
        let keys = GroupKeys {
            right: true,
            ..GroupKeys::default()
        };
        let command = command_of(&keys, true).unwrap();
        assert_eq!(
            command,
            GroupCommand::Move {
                dx: 1,
                dy: 0,
                fast: false
            }
        );
        assert_eq!(
            op_of(command, 250_000),
            Some(EditOp::MoveSelection {
                dx_nm: 250_000,
                dy_nm: 0,
                snap: Some(false)
            })
        );
        let fast = GroupKeys {
            down: true,
            shift: true,
            ..GroupKeys::default()
        };
        assert_eq!(
            op_of(command_of(&fast, true).unwrap(), 250_000),
            Some(EditOp::MoveSelection {
                dx_nm: 0,
                dy_nm: -2_500_000,
                snap: Some(false)
            })
        );
    }

    #[test]
    fn two_arrows_move_along_the_diagonal_and_opposites_cancel() {
        let diagonal = GroupKeys {
            left: true,
            up: true,
            ..GroupKeys::default()
        };
        assert_eq!(
            command_of(&diagonal, true),
            Some(GroupCommand::Move {
                dx: -1,
                dy: 1,
                fast: false
            })
        );
        let cancelled = GroupKeys {
            left: true,
            right: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&cancelled, true), None);
    }

    #[test]
    fn nothing_moves_or_aligns_without_a_selection() {
        let arrow = GroupKeys {
            left: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&arrow, false), None);
        let align = GroupKeys {
            left: true,
            alt: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&align, false), None);
    }

    #[test]
    fn alt_and_a_key_line_the_selection_up() {
        let cases = [
            (
                GroupKeys {
                    left: true,
                    alt: true,
                    ..GroupKeys::default()
                },
                AlignMode::Left,
            ),
            (
                GroupKeys {
                    right: true,
                    alt: true,
                    ..GroupKeys::default()
                },
                AlignMode::Right,
            ),
            (
                GroupKeys {
                    up: true,
                    alt: true,
                    ..GroupKeys::default()
                },
                AlignMode::Top,
            ),
            (
                GroupKeys {
                    down: true,
                    alt: true,
                    ..GroupKeys::default()
                },
                AlignMode::Bottom,
            ),
            (
                GroupKeys {
                    c: true,
                    alt: true,
                    ..GroupKeys::default()
                },
                AlignMode::CenterX,
            ),
            (
                GroupKeys {
                    m: true,
                    alt: true,
                    ..GroupKeys::default()
                },
                AlignMode::CenterY,
            ),
        ];
        for (keys, mode) in cases {
            let command = command_of(&keys, true).unwrap();
            assert_eq!(command, GroupCommand::Align(mode));
            assert_eq!(
                op_of(command, 1_000_000),
                Some(EditOp::AlignSelection { mode })
            );
        }
    }

    #[test]
    fn the_selection_keys_are_not_operations() {
        let all = GroupKeys {
            a: true,
            command: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&all, false), Some(GroupCommand::SelectAll));
        let net = GroupKeys {
            n: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&net, false), Some(GroupCommand::SelectNet));
        let escape = GroupKeys {
            escape: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&escape, true), Some(GroupCommand::Clear));
        for command in [
            GroupCommand::SelectAll,
            GroupCommand::SelectNet,
            GroupCommand::Clear,
        ] {
            assert_eq!(op_of(command, 1_000_000), None);
        }
    }

    #[test]
    fn ctrl_with_an_arrow_does_nothing_here() {
        let keys = GroupKeys {
            left: true,
            command: true,
            ..GroupKeys::default()
        };
        assert_eq!(command_of(&keys, true), None);
    }

    #[test]
    fn a_step_never_collapses_to_zero() {
        let command = GroupCommand::Move {
            dx: 1,
            dy: 0,
            fast: false,
        };
        assert_eq!(
            op_of(command, 0),
            Some(EditOp::MoveSelection {
                dx_nm: 1,
                dy_nm: 0,
                snap: Some(false)
            })
        );
    }

    #[test]
    fn the_menu_lists_the_eight_alignments_once() {
        let mut seen = BTreeSet::new();
        for (mode, _) in ALIGN_ENTRIES {
            assert!(seen.insert(format!("{mode:?}")));
        }
        assert_eq!(seen.len(), 8);
    }
}
