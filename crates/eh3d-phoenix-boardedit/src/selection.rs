// SPDX-License-Identifier: GPL-3.0-or-later
//! The selection of several objects of a board, without a screen: an ordered set of references
//! (components, tracks, vias, zones and silkscreen items) with pure operations on it: add, take
//! away, toggle, every object of a net, a rectangle (what lies wholly inside or what it touches),
//! all, invert and clear. Nothing here changes the board; the operations on a selection (move,
//! delete, align) are in [`crate::group`].
//!
//! The extent of an object is the box of what it covers, in nanometres: the pads of a component
//! with half of their largest side around each centre, the pieces of a track and of a silkscreen
//! item with half of their width, a via with half of its diameter, the corners of a zone. An arc
//! is taken as its chord.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment, SilkItem};
use eh3d_phoenix_model::{segments_intersect, Point};
use serde::{Deserialize, Serialize};

use crate::codes::{BoardEditError, ITEM_MISSING};

/// One thing of the board that can be selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Item {
    /// A component.
    Component(Id),
    /// A track.
    Track(Id),
    /// A via.
    Via(Id),
    /// A copper zone.
    Zone(Id),
    /// A text or a stroke of a silkscreen layer.
    Silk(Id),
}

impl Item {
    /// True when the board still has the thing.
    pub fn exists(&self, board: &Board) -> bool {
        match self {
            Item::Component(id) => board.components.contains_key(id),
            Item::Track(id) => board.tracks.contains_key(id),
            Item::Via(id) => board.vias.contains_key(id),
            Item::Zone(id) => board.zones.contains_key(id),
            Item::Silk(id) => board.silkscreen.contains_key(id),
        }
    }

    /// True when the thing belongs to the net: a track, a via or a zone of the net, or a
    /// component with a pad on it.
    pub fn on_net(&self, board: &Board, net: Id) -> bool {
        match self {
            Item::Component(id) => board
                .pad_nets
                .iter()
                .any(|p| p.component == *id && p.net == net),
            Item::Track(id) => board.tracks.get(id).is_some_and(|t| t.net == net),
            Item::Via(id) => board.vias.get(id).is_some_and(|v| v.net == net),
            Item::Zone(id) => board.zones.get(id).is_some_and(|z| z.net == net),
            Item::Silk(_) => false,
        }
    }

    /// The id of the thing, whatever it is.
    pub fn id(&self) -> Id {
        match self {
            Item::Component(id)
            | Item::Track(id)
            | Item::Via(id)
            | Item::Zone(id)
            | Item::Silk(id) => *id,
        }
    }
}

/// A box in nanometres, borders included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// The smallest x.
    pub min_x: i64,
    /// The smallest y.
    pub min_y: i64,
    /// The largest x.
    pub max_x: i64,
    /// The largest y.
    pub max_y: i64,
}

impl Bounds {
    /// The box between two corners, whatever their order.
    pub fn from_corners(a: Point, b: Point) -> Bounds {
        let (ax, ay) = (a.x_nm.get(), a.y_nm.get());
        let (bx, by) = (b.x_nm.get(), b.y_nm.get());
        Bounds {
            min_x: ax.min(bx),
            min_y: ay.min(by),
            max_x: ax.max(bx),
            max_y: ay.max(by),
        }
    }

    fn around(p: Point, reach: i64) -> Bounds {
        let (x, y) = (p.x_nm.get(), p.y_nm.get());
        Bounds {
            min_x: x.saturating_sub(reach),
            min_y: y.saturating_sub(reach),
            max_x: x.saturating_add(reach),
            max_y: y.saturating_add(reach),
        }
    }

    /// The smallest box that holds both.
    pub fn union(&self, other: &Bounds) -> Bounds {
        Bounds {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// True when the boxes share at least a point.
    pub fn intersects(&self, other: &Bounds) -> bool {
        self.min_x <= other.max_x
            && other.min_x <= self.max_x
            && self.min_y <= other.max_y
            && other.min_y <= self.max_y
    }

    /// True when this box lies wholly inside the other.
    pub fn inside(&self, outer: &Bounds) -> bool {
        self.min_x >= outer.min_x
            && self.max_x <= outer.max_x
            && self.min_y >= outer.min_y
            && self.max_y <= outer.max_y
    }

    fn holds(&self, p: Point) -> bool {
        (self.min_x..=self.max_x).contains(&p.x_nm.get())
            && (self.min_y..=self.max_y).contains(&p.y_nm.get())
    }

    fn corners(&self) -> Option<[Point; 4]> {
        Some([
            Point::from_nm(self.min_x, self.min_y).ok()?,
            Point::from_nm(self.max_x, self.min_y).ok()?,
            Point::from_nm(self.max_x, self.max_y).ok()?,
            Point::from_nm(self.min_x, self.max_y).ok()?,
        ])
    }
}

fn grow(bounds: &mut Option<Bounds>, add: Bounds) {
    *bounds = Some((*bounds).map_or(add, |b| b.union(&add)));
}

fn pieces_bounds(path: &[Segment], width: i64) -> Option<Bounds> {
    let half = width / 2;
    let mut found: Option<Bounds> = None;
    for piece in path {
        grow(&mut found, Bounds::around(piece.start(), half));
        grow(&mut found, Bounds::around(piece.end(), half));
    }
    found
}

/// The extent of an object of the board; none when the board does not have it or it covers
/// nothing.
pub fn item_bounds(board: &Board, item: Item) -> Option<Bounds> {
    match item {
        Item::Component(id) => {
            let component = board.components.get(&id)?;
            let mut found: Option<Bounds> = None;
            if let Some(footprint) = board.footprints.get(&component.footprint) {
                for (pad_id, pad) in &footprint.pads {
                    let Some(at) = pad_position(
                        board,
                        PadRef {
                            component: id,
                            pad: *pad_id,
                        },
                    ) else {
                        continue;
                    };
                    let half = match pad.shape {
                        PadShape::Circle { diameter } => diameter.get() / 2,
                        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
                            width.get().max(height.get()) / 2
                        }
                    };
                    grow(&mut found, Bounds::around(at, half.max(0)));
                }
            }
            found.or_else(|| Some(Bounds::around(component.at, 0)))
        }
        Item::Track(id) => {
            let track = board.tracks.get(&id)?;
            pieces_bounds(&track.path, track.width.get())
        }
        Item::Via(id) => {
            let via = board.vias.get(&id)?;
            Some(Bounds::around(via.at, via.diameter.get() / 2))
        }
        Item::Zone(id) => {
            let zone = board.zones.get(&id)?;
            let mut found: Option<Bounds> = None;
            for p in &zone.outline {
                grow(&mut found, Bounds::around(*p, 0));
            }
            found
        }
        Item::Silk(id) => {
            let silk = board.silkscreen.get(&id)?;
            match silk {
                SilkItem::Text { at, .. } => match silk.drawn() {
                    Ok(drawn) => pieces_bounds(&drawn.pieces, drawn.width.get()),
                    Err(_) => Some(Bounds::around(*at, 0)),
                },
                SilkItem::Stroke { width, path, .. } => pieces_bounds(path, width.get()),
            }
        }
    }
}

/// True when some piece of the centre line of the track touches the rectangle.
fn track_touches(board: &Board, id: Id, rect: &Bounds) -> bool {
    let Some(track) = board.tracks.get(&id) else {
        return false;
    };
    let Some(corners) = rect.corners() else {
        return false;
    };
    track.path.iter().any(|piece| {
        let (a, b) = (piece.start(), piece.end());
        rect.holds(a)
            || rect.holds(b)
            || (0..4).any(|i| segments_intersect(a, b, corners[i], corners[(i + 1) % 4]))
    })
}

/// What the rectangle of a selection takes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RectMode {
    /// Only what lies wholly inside the rectangle.
    #[default]
    Contained,
    /// What lies inside and what the rectangle touches (a track by its centre line, the rest by
    /// its extent).
    Crossed,
}

/// What a step of selection does with the objects it finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectAction {
    /// Adds them to the selection.
    #[default]
    Add,
    /// Takes them away from the selection.
    Remove,
    /// Takes away those that are selected and adds those that are not.
    Toggle,
}

/// One step of a selection, as the file of operations of `phoenix boardedit` writes it
/// (`{"select": "net", "net": "..."}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "select", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectStep {
    /// Adds objects.
    Add {
        /// The objects, which the board must have.
        items: Vec<Item>,
    },
    /// Takes objects away.
    Remove {
        /// The objects.
        items: Vec<Item>,
    },
    /// Toggles objects.
    Toggle {
        /// The objects, which the board must have.
        items: Vec<Item>,
    },
    /// Every object of a net.
    Net {
        /// The net, which the board must have.
        net: Id,
        /// What to do with them; add when left out.
        #[serde(default)]
        action: SelectAction,
    },
    /// What a rectangle takes in.
    Rect {
        /// One corner.
        from: Point,
        /// The other corner.
        to: Point,
        /// Wholly inside or also touched; wholly inside when left out.
        #[serde(default)]
        mode: RectMode,
        /// What to do with them; add when left out.
        #[serde(default)]
        action: SelectAction,
    },
    /// Every object of the board.
    All,
    /// Selects what is not selected and drops what is.
    Invert,
    /// Drops the whole selection.
    Clear,
}

/// Every object of the board, in a fixed order: components, tracks, vias, zones, silkscreen, each
/// by id.
pub fn all_items(board: &Board) -> Vec<Item> {
    let mut all = Vec::new();
    all.extend(board.components.keys().map(|id| Item::Component(*id)));
    all.extend(board.tracks.keys().map(|id| Item::Track(*id)));
    all.extend(board.vias.keys().map(|id| Item::Via(*id)));
    all.extend(board.zones.keys().map(|id| Item::Zone(*id)));
    all.extend(board.silkscreen.keys().map(|id| Item::Silk(*id)));
    all
}

fn missing(item: Item) -> BoardEditError {
    BoardEditError::new(ITEM_MISSING, format!("the board has no {item:?}"))
}

/// An ordered set of objects: the order is the one in which they were selected, and none is
/// there twice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    items: Vec<Item>,
}

impl Selection {
    /// An empty selection.
    pub fn new() -> Selection {
        Selection::default()
    }

    /// The objects, in the order they were selected.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// How many objects are selected.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// True when the object is selected.
    pub fn contains(&self, item: Item) -> bool {
        self.items.contains(&item)
    }

    /// Selects an object; false when it already was.
    pub fn add(&mut self, item: Item) -> bool {
        if self.contains(item) {
            return false;
        }
        self.items.push(item);
        true
    }

    /// Takes an object away; false when it was not selected.
    pub fn remove(&mut self, item: Item) -> bool {
        match self.items.iter().position(|i| *i == item) {
            Some(index) => {
                self.items.remove(index);
                true
            }
            None => false,
        }
    }

    /// Takes the object away when it is selected and selects it when it is not; true when it is
    /// selected now.
    pub fn toggle(&mut self, item: Item) -> bool {
        if self.remove(item) {
            false
        } else {
            self.items.push(item);
            true
        }
    }

    /// Drops the whole selection.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Does `action` for each object; how many objects changed state.
    pub fn act(&mut self, action: SelectAction, items: &[Item]) -> usize {
        let mut changed = 0;
        for item in items {
            let did = match action {
                SelectAction::Add => self.add(*item),
                SelectAction::Remove => self.remove(*item),
                SelectAction::Toggle => {
                    self.toggle(*item);
                    true
                }
            };
            changed += usize::from(did);
        }
        changed
    }

    /// Every object of a net: the tracks, vias and zones of the net and the components with a
    /// pad on it.
    pub fn net_items(board: &Board, net: Id) -> Vec<Item> {
        all_items(board)
            .into_iter()
            .filter(|item| item.on_net(board, net))
            .collect()
    }

    /// The objects that a rectangle takes in, in the fixed order of [`all_items`].
    pub fn rect_items(board: &Board, a: Point, b: Point, mode: RectMode) -> Vec<Item> {
        let rect = Bounds::from_corners(a, b);
        all_items(board)
            .into_iter()
            .filter(|item| {
                let Some(bounds) = item_bounds(board, *item) else {
                    return false;
                };
                match (mode, item) {
                    (RectMode::Contained, _) => bounds.inside(&rect),
                    (RectMode::Crossed, Item::Track(id)) => track_touches(board, *id, &rect),
                    (RectMode::Crossed, _) => bounds.intersects(&rect),
                }
            })
            .collect()
    }

    /// Selects every object of a net (or takes them away, or toggles them).
    pub fn select_net(&mut self, board: &Board, net: Id, action: SelectAction) -> usize {
        self.act(action, &Selection::net_items(board, net))
    }

    /// Selects what a rectangle takes in (or takes it away, or toggles it).
    pub fn select_rect(
        &mut self,
        board: &Board,
        a: Point,
        b: Point,
        mode: RectMode,
        action: SelectAction,
    ) -> usize {
        self.act(action, &Selection::rect_items(board, a, b, mode))
    }

    /// Selects every object of the board.
    pub fn select_all(&mut self, board: &Board) -> usize {
        self.act(SelectAction::Add, &all_items(board))
    }

    /// Selects what is not selected and drops what is.
    pub fn invert(&mut self, board: &Board) {
        let before = std::mem::take(&mut self.items);
        self.items = all_items(board)
            .into_iter()
            .filter(|item| !before.contains(item))
            .collect();
    }

    /// Drops the objects that the board no longer has.
    pub fn prune(&mut self, board: &Board) {
        self.items.retain(|item| item.exists(board));
    }

    /// Does one step. On an error the selection is as it was.
    pub fn apply(&mut self, board: &Board, step: &SelectStep) -> Result<(), BoardEditError> {
        match step {
            SelectStep::Add { items } | SelectStep::Toggle { items } => {
                if let Some(gone) = items.iter().find(|i| !i.exists(board)) {
                    return Err(missing(*gone));
                }
                let action = if matches!(step, SelectStep::Add { .. }) {
                    SelectAction::Add
                } else {
                    SelectAction::Toggle
                };
                self.act(action, items);
            }
            SelectStep::Remove { items } => {
                self.act(SelectAction::Remove, items);
            }
            SelectStep::Net { net, action } => {
                if !board.nets.contains_key(net) {
                    return Err(BoardEditError::new(
                        ITEM_MISSING,
                        format!("the board has no net {net}"),
                    ));
                }
                self.select_net(board, *net, *action);
            }
            SelectStep::Rect {
                from,
                to,
                mode,
                action,
            } => {
                self.select_rect(board, *from, *to, *mode, *action);
            }
            SelectStep::All => {
                self.select_all(board);
            }
            SelectStep::Invert => self.invert(board),
            SelectStep::Clear => self.clear(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{fixture, id, Fixture};
    use eh3d_phoenix_board::{Via, Zone};
    use eh3d_phoenix_model::Nm;

    /// The fixture with a via of `sig` at (20, 20) and a zone of `gnd` from (40, 20) to (50, 30).
    fn busy() -> (Fixture, Item, Item) {
        let mut f = fixture();
        let via = id(50);
        let zone = id(51);
        f.board.vias.insert(
            via,
            Via {
                net: f.sig,
                at: f.pt(20, 20),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: f.top,
                to_layer: f.bottom,
            },
        );
        f.board.zones.insert(
            zone,
            Zone {
                net: f.gnd,
                layer: f.top,
                outline: vec![f.pt(40, 20), f.pt(50, 20), f.pt(50, 30), f.pt(40, 30)],
                clearance: Nm::new(200_000).unwrap(),
                cutouts: Vec::new(),
            },
        );
        (f, Item::Via(via), Item::Zone(zone))
    }

    #[test]
    fn the_extents_are_worked_out_by_hand() {
        let (f, via, zone) = busy();
        // R1 at (10, 10): pads at 8.5 and 11.5, each 1 mm square, so half of the largest side is 0.5
        let r1 = item_bounds(&f.board, Item::Component(f.r1)).unwrap();
        assert_eq!(
            r1,
            Bounds {
                min_x: 8_000_000,
                min_y: 9_500_000,
                max_x: 12_000_000,
                max_y: 10_500_000
            }
        );
        // the track runs from 11.5 to 28.5 with a width of 0.25: 0.125 mm on each side
        let track = item_bounds(&f.board, Item::Track(f.track)).unwrap();
        assert_eq!(
            track,
            Bounds {
                min_x: 11_375_000,
                min_y: 9_875_000,
                max_x: 28_625_000,
                max_y: 10_125_000
            }
        );
        // the via has a diameter of 0.6 mm
        let v = item_bounds(&f.board, via).unwrap();
        assert_eq!(
            (v.min_x, v.min_y, v.max_x, v.max_y),
            (19_700_000, 19_700_000, 20_300_000, 20_300_000)
        );
        let z = item_bounds(&f.board, zone).unwrap();
        assert_eq!(
            (z.min_x, z.min_y, z.max_x, z.max_y),
            (40_000_000, 20_000_000, 50_000_000, 30_000_000)
        );
        assert_eq!(item_bounds(&f.board, Item::Via(id(99))), None);
    }

    #[test]
    fn add_remove_and_toggle_keep_the_order_and_no_repeats() {
        let (f, via, _) = busy();
        let mut s = Selection::new();
        assert!(s.is_empty());
        assert!(s.add(Item::Component(f.r2)));
        assert!(s.add(Item::Component(f.r1)));
        assert!(!s.add(Item::Component(f.r2)));
        assert_eq!(
            s.items(),
            &[Item::Component(f.r2), Item::Component(f.r1)][..]
        );
        assert!(s.toggle(via));
        assert!(!s.toggle(Item::Component(f.r2)));
        assert_eq!(s.items(), &[Item::Component(f.r1), via][..]);
        assert!(!s.remove(Item::Track(f.track)));
        assert!(s.remove(via));
        assert_eq!(s.len(), 1);
        assert!(s.contains(Item::Component(f.r1)));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn a_net_takes_its_components_tracks_vias_and_zones() {
        let (f, via, zone) = busy();
        let mut s = Selection::new();
        // sig: both resistors have a pad on it, the track and the via; the zone is on gnd
        assert_eq!(s.select_net(&f.board, f.sig, SelectAction::Add), 4);
        assert_eq!(
            s.items(),
            &[
                Item::Component(f.r1),
                Item::Component(f.r2),
                Item::Track(f.track),
                via
            ][..]
        );
        // gnd: both resistors (already in) and the zone
        assert_eq!(s.select_net(&f.board, f.gnd, SelectAction::Add), 1);
        assert!(s.contains(zone));
        assert_eq!(s.select_net(&f.board, f.sig, SelectAction::Remove), 4);
        assert_eq!(s.items(), &[zone][..]);
    }

    #[test]
    fn a_rectangle_takes_what_is_inside_or_also_what_it_touches() {
        let (f, via, zone) = busy();
        // from (5, 5) to (15, 15): R1 is wholly inside (8 to 12, 9.5 to 10.5); the track is not
        let mut s = Selection::new();
        s.select_rect(
            &f.board,
            f.pt(5, 5),
            f.pt(15, 15),
            RectMode::Contained,
            SelectAction::Add,
        );
        assert_eq!(s.items(), &[Item::Component(f.r1)][..]);
        // crossed: the track touches the rectangle at x = 15, y = 10; R2 (28 to 32) does not
        let mut c = Selection::new();
        c.select_rect(
            &f.board,
            f.pt(15, 15),
            f.pt(5, 5),
            RectMode::Crossed,
            SelectAction::Add,
        );
        assert_eq!(
            c.items(),
            &[Item::Component(f.r1), Item::Track(f.track)][..]
        );
        // the box of the track crosses (18, 5)-(22, 15) and so does its line; the via at y = 20 not
        let mut d = Selection::new();
        d.select_rect(
            &f.board,
            f.pt(18, 5),
            f.pt(22, 15),
            RectMode::Crossed,
            SelectAction::Add,
        );
        assert_eq!(d.items(), &[Item::Track(f.track)][..]);
        // a rectangle around the via and the zone only
        let mut e = Selection::new();
        e.select_rect(
            &f.board,
            f.pt(19, 19),
            f.pt(55, 35),
            RectMode::Contained,
            SelectAction::Add,
        );
        assert_eq!(e.items(), &[via, zone][..]);
        // toggling the same rectangle drops them again
        e.select_rect(
            &f.board,
            f.pt(19, 19),
            f.pt(55, 35),
            RectMode::Contained,
            SelectAction::Toggle,
        );
        assert!(e.is_empty());
    }

    #[test]
    fn a_long_diagonal_track_is_not_taken_by_a_rectangle_off_its_line() {
        let mut f = fixture();
        let diag = id(60);
        let mut t = f.board.tracks[&f.track].clone();
        t.path = vec![Segment::Line {
            start: f.pt(0, 0),
            end: f.pt(10, 10),
        }];
        f.board.tracks.insert(diag, t);
        // the box of the track is (0,0)-(10,10) but the line does not go through (6,1)-(9,3)
        let off = Selection::rect_items(&f.board, f.pt(6, 1), f.pt(9, 3), RectMode::Crossed);
        assert!(!off.contains(&Item::Track(diag)));
        let on = Selection::rect_items(&f.board, f.pt(4, 3), f.pt(6, 5), RectMode::Crossed);
        assert!(on.contains(&Item::Track(diag)));
    }

    #[test]
    fn invert_all_and_clear() {
        let (f, via, zone) = busy();
        let mut s = Selection::new();
        s.add(Item::Component(f.r1));
        s.add(via);
        s.invert(&f.board);
        assert_eq!(
            s.items(),
            &[Item::Component(f.r2), Item::Track(f.track), zone][..]
        );
        s.invert(&f.board);
        assert_eq!(s.len(), 2);
        assert_eq!(s.select_all(&f.board), 3);
        assert_eq!(s.len(), 5);
        s.invert(&f.board);
        assert!(s.is_empty());
    }

    #[test]
    fn prune_drops_what_the_board_lost() {
        let (mut f, via, _) = busy();
        let mut s = Selection::new();
        s.add(via);
        s.add(Item::Track(f.track));
        f.board.vias.clear();
        s.prune(&f.board);
        assert_eq!(s.items(), &[Item::Track(f.track)][..]);
    }

    #[test]
    fn steps_are_applied_in_order_and_a_missing_object_is_an_error() {
        let (f, via, _) = busy();
        let mut s = Selection::new();
        s.apply(&f.board, &SelectStep::All).unwrap();
        s.apply(
            &f.board,
            &SelectStep::Net {
                net: f.sig,
                action: SelectAction::Remove,
            },
        )
        .unwrap();
        assert_eq!(s.len(), 1);
        let before = s.clone();
        let error = s
            .apply(
                &f.board,
                &SelectStep::Add {
                    items: vec![via, Item::Track(id(99))],
                },
            )
            .unwrap_err();
        assert_eq!(error.code, ITEM_MISSING);
        assert_eq!(s, before);
        let error = s
            .apply(
                &f.board,
                &SelectStep::Net {
                    net: id(98),
                    action: SelectAction::Add,
                },
            )
            .unwrap_err();
        assert_eq!(error.code, ITEM_MISSING);
        s.apply(&f.board, &SelectStep::Clear).unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn a_step_is_written_and_read_back_in_json() {
        let text = r#"{"select":"rect","from":{"x_nm":0,"y_nm":0},"to":{"x_nm":5,"y_nm":5},"mode":"crossed","action":"toggle"}"#;
        let step: SelectStep = serde_json::from_str(text).unwrap();
        assert!(matches!(
            step,
            SelectStep::Rect {
                mode: RectMode::Crossed,
                action: SelectAction::Toggle,
                ..
            }
        ));
        let back = serde_json::to_string(&step).unwrap();
        assert_eq!(serde_json::from_str::<SelectStep>(&back).unwrap(), step);
        let net = format!(r#"{{"select":"net","net":"{}"}}"#, id(10));
        assert!(matches!(
            serde_json::from_str::<SelectStep>(&net).unwrap(),
            SelectStep::Net {
                action: SelectAction::Add,
                ..
            }
        ));
        let item = format!(
            r#"{{"select":"add","items":[{{"kind":"zone","id":"{}"}}]}}"#,
            id(51)
        );
        assert_eq!(
            serde_json::from_str::<SelectStep>(&item).unwrap(),
            SelectStep::Add {
                items: vec![Item::Zone(id(51))]
            }
        );
        assert_eq!(
            serde_json::from_str::<SelectStep>(r#"{"select":"all"}"#).unwrap(),
            SelectStep::All
        );
        assert!(serde_json::from_str::<SelectStep>(r#"{"select":"fly"}"#).is_err());
    }
}
