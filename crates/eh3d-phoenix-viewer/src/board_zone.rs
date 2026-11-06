// SPDX-License-Identifier: GPL-3.0-or-later
//! The zone tool of the board editor, without a screen: the polygon that is being drawn by clicks
//! (on the grid when snapping is on, closed by a click on its first point), its vertices that can
//! be picked and moved, the preview of the fill that the editor of `eh3d-phoenix-boardedit` makes
//! on a copy of the board, the list of the zones in the order of their priority and the operations
//! that raise, lower, fill and remove them. Every change goes through the editor like any
//! operation, so the design rules are checked before and after. The window draws this state in
//! `board_zone_gui`, behind the `gui` feature.

use eh3d_phoenix_board::Id;
use eh3d_phoenix_boardedit::codes::{BoardEditError, OPERATION_REFUSED};
use eh3d_phoenix_boardedit::{EditOp, EditState, ZoneChanges, ZoneProps};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::board_shove::ShoveTool;

/// What the polygon that is drawn becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoneMode {
    /// The outline of a zone: a new one, or the new outline of the one that is being edited.
    #[default]
    Outline,
    /// A hole in the zone that is chosen.
    Cutout,
}

/// A line of the list of zones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneRow {
    /// The zone.
    pub id: Id,
    /// The name of its net.
    pub net: String,
    /// The name of its layer.
    pub layer: String,
    /// Its priority.
    pub priority: i32,
    /// Whether the editor keeps a fill for it.
    pub filled: bool,
}

/// What the preview of the fill shows: the polygons of copper of each zone, as the fill would be
/// made now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FillPreview {
    /// The polygons of each zone that was filled.
    pub zones: Vec<(Id, Vec<Vec<Point>>)>,
    /// How many islands of copper the fill dropped.
    pub islands_dropped: usize,
    /// How many proven breaks of the rules the fill has (it would be refused).
    pub errors: usize,
}

impl FillPreview {
    /// How many polygons of copper there are in all.
    pub fn polygons(&self) -> usize {
        self.zones.iter().map(|(_, p)| p.len()).sum()
    }
}

/// The state of the zone tool.
#[derive(Debug, Default)]
pub struct ZoneTool {
    /// Whether a click puts a point of the polygon.
    pub on: bool,
    /// What the polygon becomes.
    pub mode: ZoneMode,
    /// The place of the chosen net in the list of the nets, sorted by name.
    pub net_index: usize,
    /// The priority of a new zone.
    pub priority: i32,
    /// The points of the polygon, in the order they were clicked.
    pub points: Vec<Point>,
    /// Whether the polygon is closed (the last point joins the first); no more points are added.
    pub closed: bool,
    /// The zone whose outline is being edited, when it is not a new zone.
    pub editing: Option<Id>,
    /// The zone chosen in the list (for a cutout, a priority, a removal).
    pub selected: Option<Id>,
    /// The vertex that is being dragged.
    pub dragging: Option<usize>,
    /// The preview of the fill, when it was asked for.
    pub preview: Option<FillPreview>,
}

impl ZoneTool {
    /// The chosen net; the last one when the choice is past the end of the list.
    pub fn selected_net(&self, state: &EditState) -> Option<Id> {
        let nets = ShoveTool::nets(state);
        let last = nets.len().checked_sub(1)?;
        nets.get(self.net_index.min(last)).map(|(id, _)| *id)
    }

    /// The zones of the board in the order of the fill: the higher priority first, then by id.
    pub fn rows(state: &EditState) -> Vec<ZoneRow> {
        let board = state.board();
        let data = state.edit.zone_data();
        data.fill_order(board)
            .into_iter()
            .filter_map(|id| {
                let zone = board.zones.get(&id)?;
                Some(ZoneRow {
                    id,
                    net: board
                        .nets
                        .get(&zone.net)
                        .map(|n| n.name.clone())
                        .unwrap_or_default(),
                    layer: board
                        .layers
                        .get(&zone.layer)
                        .map(|l| l.name.clone())
                        .unwrap_or_default(),
                    priority: data.props_of(id).priority,
                    filled: data.fill_of(id).is_some(),
                })
            })
            .collect()
    }

    fn refuse(state: &mut EditState, what: &str) -> BoardEditError {
        let error = BoardEditError::new(OPERATION_REFUSED, what);
        state.error = Some(error.clone());
        error
    }

    /// Puts a point of the polygon (on the grid when snapping is on). A click on the first point
    /// of a polygon of three points or more closes it; the same point twice in a row counts once;
    /// nothing is added to a closed polygon. The preview is dropped.
    pub fn click(&mut self, state: &EditState, at: Point) {
        if self.closed {
            return;
        }
        let at = state.snap(at);
        self.preview = None;
        if self.points.len() >= 3 && self.points.first() == Some(&at) {
            self.closed = true;
            return;
        }
        if self.points.last() != Some(&at) {
            self.points.push(at);
        }
    }

    /// Closes the polygon; false when it has fewer than three points.
    pub fn close(&mut self) -> bool {
        if self.points.len() >= 3 {
            self.closed = true;
        }
        self.closed
    }

    /// The vertex nearest to a point, when it is within the pick tolerance of the editor.
    pub fn pick_vertex(&self, state: &EditState, at: Point) -> Option<usize> {
        let tolerance = i128::from(state.edit.settings.pick_tolerance_nm);
        let limit = tolerance * tolerance;
        self.points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let dx = i128::from(p.x_nm.get() - at.x_nm.get());
                let dy = i128::from(p.y_nm.get() - at.y_nm.get());
                (dx * dx + dy * dy, i)
            })
            .filter(|(d, _)| *d <= limit)
            .min()
            .map(|(_, i)| i)
    }

    /// A press of the mouse: it takes a vertex when one is near, else it puts a point.
    pub fn press(&mut self, state: &EditState, at: Point) {
        match self.pick_vertex(state, at) {
            Some(i) if self.closed || i > 0 || self.points.len() < 3 => self.dragging = Some(i),
            _ => self.click(state, at),
        }
    }

    /// Moves the vertex that is being dragged to a point (on the grid when snapping is on).
    pub fn drag_to(&mut self, state: &EditState, at: Point) {
        if let Some(i) = self.dragging {
            if let Some(p) = self.points.get_mut(i) {
                *p = state.snap(at);
                self.preview = None;
            }
        }
    }

    /// Lets go of the vertex.
    pub fn release(&mut self) {
        self.dragging = None;
    }

    /// Drops the last point; a closed polygon is opened first.
    pub fn remove_last(&mut self) {
        if self.closed {
            self.closed = false;
        } else {
            self.points.pop();
        }
        self.preview = None;
    }

    /// Drops the polygon, the edit and the preview.
    pub fn clear(&mut self) {
        self.points.clear();
        self.closed = false;
        self.editing = None;
        self.dragging = None;
        self.preview = None;
    }

    /// Loads the outline of a zone to move its vertices; false when the board has no such zone.
    pub fn begin_edit(&mut self, state: &EditState, zone: Id) -> bool {
        let Some(z) = state.board().zones.get(&zone) else {
            return false;
        };
        self.clear();
        self.points = z.outline.clone();
        self.closed = true;
        self.editing = Some(zone);
        self.selected = Some(zone);
        self.mode = ZoneMode::Outline;
        true
    }

    /// The operation that the polygon is: a new zone of the chosen net on the active layer, the
    /// new outline of the zone that is being edited, or a cutout of the chosen zone.
    pub fn operation(&self, state: &mut EditState) -> Result<EditOp, BoardEditError> {
        if self.points.len() < 3 {
            return Err(Self::refuse(state, "click at least three points"));
        }
        match self.mode {
            ZoneMode::Cutout => {
                let Some(zone) = self.selected else {
                    return Err(Self::refuse(state, "choose the zone to cut"));
                };
                Ok(EditOp::ZoneCutout {
                    zone,
                    polygon: self.points.clone(),
                })
            }
            ZoneMode::Outline => {
                if let Some(zone) = self.editing {
                    return Ok(EditOp::ZoneEdit {
                        zone,
                        net: None,
                        layer: None,
                        outline: Some(self.points.clone()),
                        clearance_nm: None,
                        props: ZoneChanges::default(),
                    });
                }
                let Some(net) = self.selected_net(state) else {
                    return Err(Self::refuse(state, "the board has no net"));
                };
                let Some(layer) = state.active_layer else {
                    return Err(Self::refuse(state, "the board has no copper layer"));
                };
                Ok(EditOp::ZoneAdd {
                    id: None,
                    net,
                    layer,
                    outline: self.points.clone(),
                    clearance_nm: None,
                    props: ZoneProps {
                        priority: self.priority,
                        ..ZoneProps::default()
                    },
                })
            }
        }
    }

    /// Accepts the polygon: it goes through the editor as one step. The polygon is dropped when
    /// it was done; the zone it made is chosen.
    pub fn accept(&mut self, state: &mut EditState) -> bool {
        let Ok(op) = self.operation(state) else {
            return false;
        };
        if !state.run(&op) {
            return false;
        }
        if let Some(created) = state.last.as_ref().and_then(|o| o.created.first().copied()) {
            self.selected = Some(created);
        }
        self.clear();
        true
    }

    /// Makes the preview of the fill again, on a copy of the editor: with the polygon applied
    /// when it can be, else of the board as it is. The error, if any, is kept in the state. The
    /// board is not changed.
    pub fn refresh_preview(&mut self, state: &mut EditState) {
        self.preview = None;
        let mut trial = state.edit.clone();
        if self.points.len() >= 3 {
            let Ok(op) = self.operation(state) else {
                return;
            };
            if let Err(e) = trial.apply(&op) {
                state.error = Some(e);
                return;
            }
        }
        match trial.preview_fill() {
            Ok(report) => {
                state.error = None;
                let zones = report
                    .fill
                    .zones
                    .iter()
                    .map(|fz| {
                        let polygons: Vec<Vec<Point>> = fz
                            .polygons
                            .iter()
                            .map(|poly| {
                                poly.iter()
                                    .filter_map(|p| Point::from_nm(p[0], p[1]).ok())
                                    .collect()
                            })
                            .collect();
                        (fz.zone, polygons)
                    })
                    .collect();
                self.preview = Some(FillPreview {
                    zones,
                    islands_dropped: report.stats.iter().map(|s| s.islands_removed).sum(),
                    errors: report
                        .findings
                        .iter()
                        .filter(|f| f.severity.as_str() == "error")
                        .count(),
                });
            }
            Err(e) => state.error = Some(e),
        }
    }

    /// Fills the zones and keeps the fills, as one step of the editor.
    pub fn fill(&mut self, state: &mut EditState) -> bool {
        self.preview = None;
        state.run(&EditOp::ZoneFill { zone: None })
    }

    /// Gives a zone one more (`up`) or one less priority.
    pub fn shift_priority(&mut self, state: &mut EditState, zone: Id, up: bool) -> bool {
        let Some(row) = Self::rows(state).into_iter().find(|r| r.id == zone) else {
            return false;
        };
        self.preview = None;
        let priority = if up {
            row.priority.saturating_add(1)
        } else {
            row.priority.saturating_sub(1)
        };
        state.run(&EditOp::ZonePriority { zone, priority })
    }

    /// Removes a zone.
    pub fn remove(&mut self, state: &mut EditState, zone: Id) -> bool {
        self.preview = None;
        let done = state.run(&EditOp::ZoneRemove { zone });
        if done {
            if self.selected == Some(zone) {
                self.selected = None;
            }
            if self.editing == Some(zone) {
                self.clear();
            }
        }
        done
    }

    /// What the zones say about the board now, one line each, in a language: zones with no fill,
    /// proven breaks by a fill that is kept, islands that join nothing.
    pub fn issue_lines(state: &EditState, lang: Lang) -> Vec<String> {
        state
            .edit
            .zone_issues()
            .iter()
            .map(|issue| issue.line(lang))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings};
    use eh3d_phoenix_route::RuleContext;

    use super::*;
    use crate::sample;

    /// The editor of the sample board, with no snapping so that the points land where they are
    /// given, and without the live check.
    fn state() -> EditState {
        let settings = BoardEditSettings {
            live_drc: false,
            snap: false,
            ..BoardEditSettings::default()
        };
        let edit = BoardEdit::new(sample::board(), RuleContext::default(), settings).unwrap();
        EditState::new(edit)
    }

    fn draw_square(tool: &mut ZoneTool, s: &EditState) {
        for (x, y) in [(12.0, 12.0), (18.0, 12.0), (18.0, 18.0), (12.0, 18.0)] {
            tool.click(s, sample::point(x, y));
        }
    }

    #[test]
    fn a_click_on_the_first_point_closes_the_polygon_and_no_more_points_are_added() {
        let s = state();
        let mut tool = ZoneTool::default();
        draw_square(&mut tool, &s);
        assert_eq!(tool.points.len(), 4);
        assert!(!tool.closed);
        tool.click(&s, sample::point(12.0, 18.0));
        assert_eq!(tool.points.len(), 4, "the same point twice counts once");
        tool.click(&s, sample::point(12.0, 12.0));
        assert!(tool.closed);
        tool.click(&s, sample::point(1.0, 1.0));
        assert_eq!(tool.points.len(), 4);
        // two points cannot be closed
        let mut small = ZoneTool::default();
        small.click(&s, sample::point(1.0, 1.0));
        small.click(&s, sample::point(2.0, 1.0));
        assert!(!small.close());
        tool.remove_last();
        assert!(!tool.closed);
        tool.remove_last();
        assert_eq!(tool.points.len(), 3);
        tool.clear();
        assert!(tool.points.is_empty() && !tool.closed && tool.editing.is_none());
    }

    #[test]
    fn a_click_snaps_to_the_grid_when_snapping_is_on() {
        let edit = BoardEdit::new(
            sample::board(),
            RuleContext::default(),
            BoardEditSettings::default(),
        )
        .unwrap();
        let s = EditState::new(edit);
        let mut tool = ZoneTool::default();
        tool.click(&s, sample::point(5.04, 15.1));
        assert_eq!(tool.points, vec![sample::point(5.0, 15.0)]);
    }

    #[test]
    fn a_vertex_is_picked_and_moved_and_the_move_is_kept_on_the_grid() {
        let s = state();
        let mut tool = ZoneTool::default();
        draw_square(&mut tool, &s);
        assert!(tool.close());
        assert_eq!(tool.pick_vertex(&s, sample::point(18.05, 12.0)), Some(1));
        assert_eq!(tool.pick_vertex(&s, sample::point(25.0, 25.0)), None);
        tool.press(&s, sample::point(18.05, 12.0));
        assert_eq!(tool.dragging, Some(1));
        tool.drag_to(&s, sample::point(20.0, 11.0));
        tool.release();
        assert_eq!(tool.points[1], sample::point(20.0, 11.0));
        assert_eq!(tool.dragging, None);
        // a press away from every vertex of a closed polygon changes nothing
        tool.press(&s, sample::point(25.0, 25.0));
        assert_eq!(tool.points.len(), 4);
    }

    #[test]
    fn the_polygon_becomes_a_zone_in_one_step_that_can_be_undone() {
        let mut s = state();
        let mut tool = ZoneTool {
            net_index: 1,
            priority: 3,
            ..ZoneTool::default()
        };
        let zones = s.board().zones.len();
        let before = s.board().to_bytes().unwrap();
        // fewer than three points
        assert!(!tool.accept(&mut s));
        assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
        draw_square(&mut tool, &s);
        assert!(tool.accept(&mut s), "{:?}", s.error);
        assert_eq!(s.board().zones.len(), zones + 1);
        let made = tool.selected.unwrap();
        // the nets are GND and VCC sorted by name: the choice 1 is VCC
        assert_eq!(s.board().zones[&made].net, sample::id(11));
        assert_eq!(s.edit.zone_data().props_of(made).priority, 3);
        assert!(tool.points.is_empty());
        assert!(s.undo());
        assert_eq!(s.board().to_bytes().unwrap(), before);
    }

    #[test]
    fn the_outline_of_a_zone_is_edited_by_moving_its_vertices() {
        let mut s = state();
        let mut tool = ZoneTool::default();
        assert!(!tool.begin_edit(&s, sample::id(999)));
        assert!(tool.begin_edit(&s, sample::id(43)));
        assert_eq!(tool.points.len(), 4);
        assert!(tool.closed);
        tool.press(&s, sample::point(8.0, 12.0));
        tool.drag_to(&s, sample::point(9.0, 12.0));
        tool.release();
        assert!(tool.accept(&mut s), "{:?}", s.error);
        assert_eq!(
            s.board().zones[&sample::id(43)].outline[1],
            sample::point(9.0, 12.0)
        );
        assert!(tool.editing.is_none() && tool.points.is_empty());
    }

    #[test]
    fn a_cutout_needs_a_chosen_zone_and_goes_inside_it() {
        let mut s = state();
        let mut tool = ZoneTool {
            mode: ZoneMode::Cutout,
            ..ZoneTool::default()
        };
        for (x, y) in [(3.0, 13.0), (5.0, 13.0), (5.0, 15.0), (3.0, 15.0)] {
            tool.click(&s, sample::point(x, y));
        }
        assert!(!tool.accept(&mut s));
        assert!(s.error.is_some());
        tool.selected = Some(sample::id(43));
        assert!(tool.accept(&mut s), "{:?}", s.error);
        assert_eq!(s.edit.zone_data().props_of(sample::id(43)).cutouts.len(), 1);
        // a cutout outside the zone is refused by the editor
        for (x, y) in [(20.0, 1.0), (22.0, 1.0), (22.0, 3.0)] {
            tool.click(&s, sample::point(x, y));
        }
        assert!(!tool.accept(&mut s));
        assert_eq!(
            s.error.as_ref().unwrap().code,
            eh3d_phoenix_boardedit::zone_codes::ZONE_CUTOUT
        );
    }

    #[test]
    fn the_list_is_in_the_order_of_the_priority_and_a_priority_is_raised_and_lowered() {
        let mut s = state();
        let mut tool = ZoneTool::default();
        draw_square(&mut tool, &s);
        assert!(tool.accept(&mut s), "{:?}", s.error);
        let new = tool.selected.unwrap();
        let rows = ZoneTool::rows(&s);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| !r.filled));
        assert_eq!(rows[0].priority, 0);
        assert!(tool.shift_priority(&mut s, new, true));
        let rows = ZoneTool::rows(&s);
        assert_eq!((rows[0].id, rows[0].priority), (new, 1));
        assert!(tool.shift_priority(&mut s, new, false));
        assert!(tool.shift_priority(&mut s, new, false));
        let rows = ZoneTool::rows(&s);
        assert_eq!((rows[1].id, rows[1].priority), (new, -1));
        assert!(!tool.shift_priority(&mut s, sample::id(999), true));
        assert!(tool.remove(&mut s, new));
        assert_eq!(ZoneTool::rows(&s).len(), 1);
        assert_eq!(tool.selected, None);
    }

    #[test]
    fn the_preview_shows_the_fill_without_changing_the_board_and_the_fill_is_kept_on_request() {
        let mut s = state();
        let mut tool = ZoneTool::default();
        draw_square(&mut tool, &s);
        let before = s.board().to_bytes().unwrap();
        tool.refresh_preview(&mut s);
        let preview = tool.preview.clone().expect("a preview");
        // the zone that is drawn is in the preview, and so is the one of the board
        assert_eq!(preview.zones.len(), 2, "{:?} {:?}", preview, s.error);
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert!(s.edit.zone_data().fills.is_empty());
        // a click drops the preview
        tool.click(&s, sample::point(1.0, 1.0));
        assert!(tool.preview.is_none());
        tool.remove_last();
        assert!(tool.fill(&mut s), "{:?}", s.error);
        assert_eq!(s.edit.zone_data().fills.len(), 1);
        assert!(ZoneTool::rows(&s).iter().all(|r| r.filled));
        let lines = ZoneTool::issue_lines(&s, Lang::En);
        assert!(lines.iter().all(|l| !l.is_empty()));
    }
}
