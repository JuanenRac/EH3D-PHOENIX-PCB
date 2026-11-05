// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of editing in the window, without a screen: the document that is open (a board or a
//! schematic with its session), the active tool, the drag in progress, the first click of a route,
//! the keyboard shortcuts, the notes that the status bar shows and the jump to the place of a
//! finding. The window only passes on what the person does (a press, a drag, a release, a click, a
//! key) and draws what this state says; everything that can be decided without a screen is decided
//! here, in code that is tested.

use std::path::{Path, PathBuf};

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_route::{Element, Mode, RouteRequest, RouteSettings};
use eh3d_phoenix_rules::{Lang, Severity};

use crate::codes::ViewError;
use crate::editmsg::EditMsg;
use crate::load::{self, Document, Loaded};
use crate::prim::P;
use crate::scene::{DocRef, ItemKind, Scene, SceneKind};
use crate::session::{mm, BoardSession, SchematicSession};
use crate::settings::ViewerSettings;
use crate::viewer::{Tool, Viewer};

use eh3d_phoenix_erc::ErcConfig;
use eh3d_phoenix_sync::SyncSettings;

/// A key that the editing answers to. The window turns the keys of the toolkit into these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Z: undo (with Ctrl), redo (with Ctrl and Shift).
    Z,
    /// Y: redo (with Ctrl).
    Y,
    /// S: save (with Ctrl).
    S,
    /// R: turn the selected component (backwards with Shift).
    R,
    /// V: the select tool.
    V,
    /// M: the measure tool.
    M,
    /// G: the tool that moves components.
    G,
    /// O: the tool that turns components.
    O,
    /// D: the tool that deletes tracks.
    D,
    /// E: the tool that moves vertices.
    E,
    /// W: the tool that changes widths.
    W,
    /// I: the tool that adds vias.
    I,
    /// T: the tool that routes tracks.
    T,
    /// F: fit the drawing to the window.
    F,
    /// Delete: delete the selected track.
    Delete,
    /// Escape: cancel what is in progress and clear the selection.
    Escape,
    /// Plus: make the selected track wider.
    Plus,
    /// Minus: make the selected track narrower.
    Minus,
}

/// A key with the state of the modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    /// The key.
    pub key: Key,
    /// Control (Command on macOS) is held.
    pub ctrl: bool,
    /// Shift is held.
    pub shift: bool,
}

impl Shortcut {
    /// A key with no modifier.
    pub fn plain(key: Key) -> Shortcut {
        Shortcut {
            key,
            ctrl: false,
            shift: false,
        }
    }

    /// A key with Control.
    pub fn ctrl(key: Key) -> Shortcut {
        Shortcut {
            key,
            ctrl: true,
            shift: false,
        }
    }
}

/// A drag in progress. The edit is made when the pointer is released.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drag {
    /// A component follows the pointer.
    Component {
        /// The component.
        id: Id,
        /// Where its origin was when the drag began.
        from: P,
        /// The distance from its origin to the pointer when the drag began.
        grab: P,
        /// Where its origin would be now, snapped to the grid.
        to: P,
    },
    /// A vertex of a track follows the pointer.
    Vertex {
        /// The track.
        track: Id,
        /// The vertex, from zero.
        index: usize,
        /// Where the vertex would be now, snapped to the grid.
        to: P,
    },
}

/// What the status bar says about the last thing that was done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// A text, with a detail that may be empty.
    Info(EditMsg, String),
    /// A file was saved; the second path is the copy kept of the file that it replaced.
    Saved(PathBuf, Option<PathBuf>),
    /// An error with its code.
    Failed(ViewError),
}

impl Note {
    /// The note in a language.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Note::Info(msg, detail) if detail.is_empty() => msg.tr(lang).to_string(),
            Note::Info(msg, detail) => format!("{}: {}", msg.tr(lang), detail),
            Note::Saved(path, None) => {
                format!("{}: {}", EditMsg::NoteSaved.tr(lang), path.display())
            }
            Note::Saved(path, Some(backup)) => format!(
                "{}: {} ({}: {})",
                EditMsg::NoteSaved.tr(lang),
                path.display(),
                EditMsg::NoteBackup.tr(lang),
                backup.display()
            ),
            Note::Failed(error) => error.message(lang),
        }
    }

    /// True for an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Note::Failed(_))
    }
}

/// The document that is open.
pub enum Doc {
    /// A board, edited.
    Board(Box<BoardSession>),
    /// A schematic, looked at, checked and synchronised with a board.
    Schematic(Box<SchematicSession>),
}

/// One line of the list of nets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetRow {
    /// The index of the net in the scene.
    pub index: u32,
    /// Its name.
    pub name: String,
    /// How many connections of the net are still to be routed.
    pub unrouted: usize,
}

/// The tool that a key chooses.
fn tool_of(key: Key) -> Option<Tool> {
    match key {
        Key::V => Some(Tool::Select),
        Key::M => Some(Tool::Measure),
        Key::G => Some(Tool::MoveComponent),
        Key::O => Some(Tool::RotateComponent),
        Key::D => Some(Tool::DeleteTrack),
        Key::E => Some(Tool::MoveVertex),
        Key::W => Some(Tool::SetWidth),
        Key::I => Some(Tool::AddVia),
        Key::T => Some(Tool::Route),
        _ => None,
    }
}

/// What to tell the person when a tool is chosen.
fn hint_of(tool: Tool) -> Option<EditMsg> {
    match tool {
        Tool::Select | Tool::Measure => None,
        Tool::MoveComponent | Tool::RotateComponent => Some(EditMsg::NotePickComponent),
        Tool::DeleteTrack | Tool::MoveVertex | Tool::SetWidth => Some(EditMsg::NotePickTrack),
        Tool::AddVia => Some(EditMsg::NotePickNet),
        Tool::Route => Some(EditMsg::NoteRouteFrom),
    }
}

fn component_of(doc: DocRef) -> Option<Id> {
    match doc {
        DocRef::Component(id) => Some(id),
        DocRef::Pad(pad) => Some(pad.component),
        _ => None,
    }
}

fn snapped(p: P, grid: f64) -> P {
    P::new((p.x / grid).round() * grid, (p.y / grid).round() * grid)
}

fn point_of(p: P) -> Option<Point> {
    Point::from_nm(
        (p.x * 1_000_000.0).round() as i64,
        (p.y * 1_000_000.0).round() as i64,
    )
    .ok()
}

fn distance(a: P, b: P) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// The vertices of a track: the start of the first piece and the end of every piece.
fn track_vertices(board: &Board, id: Id) -> Vec<Point> {
    let Some(track) = board.tracks.get(&id) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(first) = track.path.first() {
        out.push(first.start());
    }
    for piece in &track.path {
        out.push(piece.end());
    }
    out
}

fn severity_word(severity: Severity, lang: Lang) -> &'static str {
    match severity {
        Severity::Error => EditMsg::SevError.tr(lang),
        Severity::Warning => EditMsg::SevWarning.tr(lang),
        Severity::Info => EditMsg::SevInfo.tr(lang),
    }
}

/// One line of the list of findings of the rules of a board: the severity, the code with its
/// title in the language and what it is about.
pub fn finding_line(finding: &eh3d_phoenix_rules::Finding, lang: Lang) -> String {
    format!(
        "[{}] {} {}: {}",
        severity_word(finding.severity, lang),
        finding.code,
        eh3d_phoenix_rules::title(finding.code, lang).unwrap_or(""),
        finding.location
    )
}

/// One line of the list of findings of the electrical rule check of a schematic.
pub fn erc_line(finding: &eh3d_phoenix_erc::Finding, lang: Lang) -> String {
    let mut line = format!(
        "[{}] {} {}: {}",
        severity_word(finding.severity, lang),
        finding.code,
        eh3d_phoenix_erc::title(finding.code, lang).unwrap_or(""),
        finding.location
    );
    if !finding.detail.is_empty() {
        line.push_str(" | ");
        line.push_str(&finding.detail);
    }
    line
}

/// The name proposed for saving a document that was read from a file: the same folder, the name
/// of the file with `_edited` added, and the JSON of the model.
pub fn suggested_save_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    input.with_file_name(format!("{stem}_edited.json"))
}

/// The item of a scene that a location of a report names: `R1.2` is a pin, `net GND` the name of
/// a net, `R1` a part.
pub fn find_item(scene: &Scene, location: &str) -> Option<u32> {
    let wanted = location.trim();
    let net = wanted.strip_prefix("net ");
    let prefix = format!("{wanted} ");
    let index = scene.items.iter().position(|item| match (net, item.kind) {
        (Some(name), ItemKind::NetLabel) => item.label == name,
        (Some(_), _) => false,
        (None, ItemKind::Pin) => item.label == wanted,
        (None, ItemKind::Part) => item.label == wanted || item.label.starts_with(&prefix),
        (None, _) => false,
    })?;
    Some(index as u32)
}

/// The open document, its drawing and the state of editing.
pub struct Workspace {
    /// The drawing, the view, the layers and the selection.
    pub viewer: Viewer,
    /// The document and its session.
    pub doc: Doc,
    /// The file the document was read from; it is never written over.
    pub input: Option<PathBuf>,
    /// How much of an EAGLE file the reader could not use.
    pub unread: usize,
    /// The width given to a track by the width tool, in millimetres.
    pub active_width_mm: f64,
    /// What the router does at an obstacle.
    pub route_mode: Mode,
    /// Whether the ratsnest is drawn.
    pub show_ratsnest: bool,
    /// The first click of a route: the element and where it was clicked.
    pub route_from: Option<(Element, P)>,
    /// The drag in progress.
    pub drag: Option<Drag>,
    /// The ring that marks the place a finding jumped to.
    pub marker: Option<P>,
    /// What the status bar says about the last thing that was done.
    pub note: Option<Note>,
    /// True after Ctrl+S: the window should ask where to save. The window clears it.
    pub want_save: bool,
}

impl Workspace {
    /// A workspace on a document that was read. A board is edited under the default rules (a rule
    /// file can be loaded); a schematic is checked with the default electrical rules.
    pub fn new(
        loaded: Loaded,
        input: Option<PathBuf>,
        settings: ViewerSettings,
        route: RouteSettings,
    ) -> Result<Workspace, ViewError> {
        let scene = load::scene_of(&loaded.document, &settings)?;
        let doc = match loaded.document {
            Document::Board(board) => Doc::Board(Box::new(BoardSession::new(
                *board,
                Default::default(),
                route.clone(),
            )?)),
            Document::Schematic(schematic) => Doc::Schematic(Box::new(SchematicSession::new(
                *schematic,
                ErcConfig::default(),
                SyncSettings::default(),
            )?)),
        };
        let active_width_mm = route.default_width_nm as f64 / 1_000_000.0;
        Ok(Workspace {
            viewer: Viewer::new(scene, settings),
            doc,
            input,
            unread: loaded.unread,
            active_width_mm,
            route_mode: Mode::Around,
            show_ratsnest: true,
            route_from: None,
            drag: None,
            marker: None,
            note: None,
            want_save: false,
        })
    }

    /// Reads a file and opens it.
    pub fn open(
        path: &Path,
        settings: &ViewerSettings,
        route: &RouteSettings,
    ) -> Result<Workspace, ViewError> {
        let loaded = load::read_document(path, settings)?;
        Workspace::new(
            loaded,
            Some(path.to_path_buf()),
            settings.clone(),
            route.clone(),
        )
    }

    /// True when the document is a board.
    pub fn is_board(&self) -> bool {
        matches!(self.doc, Doc::Board(_))
    }

    /// The session of the board, when the document is one.
    pub fn board_session(&self) -> Option<&BoardSession> {
        match &self.doc {
            Doc::Board(s) => Some(s.as_ref()),
            Doc::Schematic(_) => None,
        }
    }

    /// The session of the board, to change it.
    pub fn board_session_mut(&mut self) -> Option<&mut BoardSession> {
        match &mut self.doc {
            Doc::Board(s) => Some(s.as_mut()),
            Doc::Schematic(_) => None,
        }
    }

    /// The session of the schematic, when the document is one.
    pub fn schematic_session(&self) -> Option<&SchematicSession> {
        match &self.doc {
            Doc::Schematic(s) => Some(s.as_ref()),
            Doc::Board(_) => None,
        }
    }

    /// The session of the schematic, to change it.
    pub fn schematic_session_mut(&mut self) -> Option<&mut SchematicSession> {
        match &mut self.doc {
            Doc::Schematic(s) => Some(s.as_mut()),
            Doc::Board(_) => None,
        }
    }

    /// True when the document has changes that are not saved.
    pub fn dirty(&self) -> bool {
        match &self.doc {
            Doc::Board(s) => s.dirty(),
            Doc::Schematic(s) => s.dirty(),
        }
    }

    /// True when there is something to undo.
    pub fn can_undo(&self) -> bool {
        match &self.doc {
            Doc::Board(s) => s.can_undo(),
            Doc::Schematic(s) => s.can_undo(),
        }
    }

    /// True when there is something to redo.
    pub fn can_redo(&self) -> bool {
        match &self.doc {
            Doc::Board(s) => s.can_redo(),
            Doc::Schematic(s) => s.can_redo(),
        }
    }

    fn info(&mut self, msg: EditMsg) {
        self.note = Some(Note::Info(msg, String::new()));
    }

    /// Draws the document again after it changed.
    fn rebuild(&mut self) {
        let scene = match &self.doc {
            Doc::Board(s) => Scene::from_board(&s.board, &self.viewer.settings),
            Doc::Schematic(s) => Scene::from_schematic(&s.schematic, &self.viewer.settings),
        };
        match scene {
            Ok(scene) => self.viewer.replace_scene(scene),
            Err(error) => self.note = Some(Note::Failed(error)),
        }
    }

    /// Records the result of an edit as a note and draws the document again when it worked.
    fn finish<T>(&mut self, done: EditMsg, result: Result<T, ViewError>) -> bool {
        match result {
            Ok(_) => {
                self.info(done);
                self.rebuild();
                true
            }
            Err(error) => {
                self.note = Some(Note::Failed(error));
                false
            }
        }
    }

    fn require_board(&mut self) -> bool {
        if self.is_board() {
            true
        } else {
            self.info(EditMsg::NoteBoardOnly);
            false
        }
    }

    /// Runs an edit on the board and records the result.
    fn edit<T>(
        &mut self,
        done: EditMsg,
        run: impl FnOnce(&mut BoardSession) -> Result<T, ViewError>,
    ) -> bool {
        if !self.require_board() {
            return false;
        }
        let result = match &mut self.doc {
            Doc::Board(s) => run(s.as_mut()),
            Doc::Schematic(_) => return false,
        };
        self.finish(done, result)
    }

    /// The item and the object of the document under a pixel.
    fn doc_at(&self, sx: f64, sy: f64) -> Option<(u32, DocRef)> {
        let item = self.viewer.item_at(sx, sy)?;
        let found = self.viewer.scene.items.get(item as usize)?;
        Some((item, found.doc))
    }

    /// Chooses a tool. A tool that edits needs a board; the drag and the first click of a route
    /// are dropped.
    pub fn set_tool(&mut self, tool: Tool) {
        if tool.edits() && !self.require_board() {
            return;
        }
        self.drag = None;
        self.route_from = None;
        self.viewer.set_tool(tool);
        self.note = hint_of(tool).map(|msg| Note::Info(msg, String::new()));
    }

    /// The pointer is over a pixel of the drawing area.
    pub fn hover(&mut self, sx: f64, sy: f64) {
        self.viewer.hover(sx, sy);
    }

    /// The pointer left the drawing area.
    pub fn leave(&mut self) {
        self.viewer.leave();
    }

    // ---- drags -------------------------------------------------------------------------------

    fn begin_drag(&self, sx: f64, sy: f64) -> Option<Drag> {
        let Doc::Board(session) = &self.doc else {
            return None;
        };
        let at = self.viewer.view.to_world(sx, sy);
        match self.viewer.tool {
            Tool::MoveComponent => {
                let (_, doc) = self.doc_at(sx, sy)?;
                let id = component_of(doc)?;
                let origin = mm(session.board.components.get(&id)?.at);
                Some(Drag::Component {
                    id,
                    from: origin,
                    grab: P::new(at.x - origin.x, at.y - origin.y),
                    to: origin,
                })
            }
            Tool::MoveVertex => {
                let reach = self.viewer.settings.vertex_pick_px / self.viewer.view.scale;
                let mut best: Option<(Id, usize, f64)> = None;
                for track in session.board.tracks.keys() {
                    for (index, vertex) in track_vertices(&session.board, *track).iter().enumerate()
                    {
                        let d = distance(mm(*vertex), at);
                        if d <= reach && best.is_none_or(|(_, _, b)| d < b) {
                            best = Some((*track, index, d));
                        }
                    }
                }
                let (track, index, _) = best?;
                let vertices = track_vertices(&session.board, track);
                Some(Drag::Vertex {
                    track,
                    index,
                    to: mm(*vertices.get(index)?),
                })
            }
            _ => None,
        }
    }

    /// The pointer was pressed at a pixel and begins to drag. True when the active tool took the
    /// gesture (a component or a vertex is under the pointer); false lets the window pan.
    pub fn press(&mut self, sx: f64, sy: f64) -> bool {
        let drag = self.begin_drag(sx, sy);
        if matches!(drag, Some(Drag::Component { .. })) {
            if let Some((item, _)) = self.doc_at(sx, sy) {
                self.viewer.select_item(item);
            }
        }
        self.drag = drag;
        self.drag.is_some()
    }

    /// The pointer moved while it is pressed.
    pub fn drag_to(&mut self, sx: f64, sy: f64) {
        let at = self.viewer.view.to_world(sx, sy);
        let grid = self.viewer.settings.snap_mm;
        match &mut self.drag {
            Some(Drag::Component { grab, to, .. }) => {
                *to = snapped(P::new(at.x - grab.x, at.y - grab.y), grid);
            }
            Some(Drag::Vertex { to, .. }) => *to = snapped(at, grid),
            None => {}
        }
    }

    /// The pointer was released: the edit is made. True when there was a drag.
    pub fn release(&mut self, sx: f64, sy: f64) -> bool {
        self.drag_to(sx, sy);
        let Some(drag) = self.drag.take() else {
            return false;
        };
        match drag {
            Drag::Component { id, to, .. } => {
                let Some(at) = point_of(to) else {
                    return true;
                };
                let unchanged = self
                    .board_session()
                    .and_then(|s| s.board.components.get(&id))
                    .is_some_and(|c| c.at == at);
                if !unchanged {
                    self.edit(EditMsg::NoteMoved, |s| s.move_component(id, at));
                }
            }
            Drag::Vertex { track, index, to } => {
                let Some(at) = point_of(to) else {
                    return true;
                };
                let unchanged = self
                    .board_session()
                    .map(|s| track_vertices(&s.board, track))
                    .and_then(|v| v.get(index).copied())
                    .is_some_and(|v| v == at);
                if !unchanged {
                    self.edit(EditMsg::NoteVertexMoved, |s| {
                        s.move_vertex(track, index, at)
                    });
                }
            }
        }
        true
    }

    /// Drops the drag and the first click of a route.
    pub fn cancel(&mut self) {
        self.drag = None;
        self.route_from = None;
        self.viewer.clear_selection();
    }

    /// The lines to draw over the board for what is in progress: the drag of a component, the drag
    /// of a vertex and the rubber band of a route.
    pub fn preview_lines(&self) -> Vec<(P, P)> {
        let mut out = Vec::new();
        match (&self.drag, &self.doc) {
            (Some(Drag::Component { from, to, .. }), _) => {
                let arm = 1.0;
                out.push((*from, *to));
                out.push((P::new(to.x - arm, to.y), P::new(to.x + arm, to.y)));
                out.push((P::new(to.x, to.y - arm), P::new(to.x, to.y + arm)));
            }
            (Some(Drag::Vertex { track, index, to }), Doc::Board(s)) => {
                let vertices = track_vertices(&s.board, *track);
                if let Some(before) = index.checked_sub(1).and_then(|i| vertices.get(i)) {
                    out.push((mm(*before), *to));
                }
                if let Some(after) = vertices.get(*index + 1) {
                    out.push((*to, mm(*after)));
                }
            }
            _ => {}
        }
        if let (Some((_, from)), Some(cursor)) = (&self.route_from, self.viewer.cursor) {
            out.push((*from, cursor));
        }
        out
    }

    // ---- clicks ------------------------------------------------------------------------------

    /// A click at a pixel with the active tool.
    pub fn click(&mut self, sx: f64, sy: f64) {
        let tool = self.viewer.tool;
        if !tool.edits() {
            self.viewer.click(sx, sy);
            return;
        }
        if !self.require_board() {
            return;
        }
        match tool {
            Tool::RotateComponent => match self.doc_at(sx, sy) {
                Some((item, doc)) => match component_of(doc) {
                    Some(id) => {
                        self.viewer.select_item(item);
                        self.rotate_component(id, 1);
                    }
                    None => self.info(EditMsg::NotePickComponent),
                },
                None => self.info(EditMsg::NotePickComponent),
            },
            Tool::DeleteTrack => match self.doc_at(sx, sy) {
                Some((_, DocRef::Track(id))) => {
                    self.viewer.clear_selection();
                    self.delete_track(id);
                }
                _ => self.info(EditMsg::NotePickTrack),
            },
            Tool::SetWidth => match self.doc_at(sx, sy) {
                Some((item, DocRef::Track(id))) => {
                    self.viewer.select_item(item);
                    let width = self.active_width_nm();
                    self.edit(EditMsg::NoteWidthSet, |s| s.set_track_width(id, width));
                }
                _ => self.info(EditMsg::NotePickTrack),
            },
            Tool::AddVia => self.click_via(sx, sy),
            Tool::Route => self.click_route(sx, sy),
            _ => self.viewer.select_at(sx, sy),
        }
    }

    fn active_width_nm(&self) -> i64 {
        (self.active_width_mm * 1_000_000.0).round() as i64
    }

    fn click_via(&mut self, sx: f64, sy: f64) {
        let Some((item, _)) = self.doc_at(sx, sy) else {
            self.info(EditMsg::NotePickNet);
            return;
        };
        let net = self
            .viewer
            .scene
            .items
            .get(item as usize)
            .and_then(|i| i.net);
        let id = net.and_then(|index| {
            self.board_session()
                .and_then(|s| s.board.nets.keys().nth(index as usize).copied())
        });
        let at = point_of(snapped(
            self.viewer.view.to_world(sx, sy),
            self.viewer.settings.snap_mm,
        ));
        match (id, at) {
            (Some(net), Some(at)) => {
                self.edit(EditMsg::NoteViaAdded, |s| s.add_via(net, at));
            }
            _ => self.info(EditMsg::NotePickNet),
        }
    }

    /// The element of a route that an object of the board stands for, given where it was clicked
    /// (a track is joined at the end nearest to the click).
    fn element_of(&self, doc: DocRef, at: P) -> Option<Element> {
        match doc {
            DocRef::Pad(pad) => Some(Element::Pad(pad)),
            DocRef::Via(id) => Some(Element::Via(id)),
            DocRef::Track(id) => {
                let vertices = track_vertices(&self.board_session()?.board, id);
                let first = mm(*vertices.first()?);
                let last = mm(*vertices.last()?);
                Some(Element::Track {
                    id,
                    end: distance(last, at) <= distance(first, at),
                })
            }
            _ => None,
        }
    }

    fn click_route(&mut self, sx: f64, sy: f64) {
        let wanted = if self.route_from.is_some() {
            EditMsg::NoteRouteTo
        } else {
            EditMsg::NoteRouteFrom
        };
        let at = self.viewer.view.to_world(sx, sy);
        let found = self
            .doc_at(sx, sy)
            .and_then(|(item, doc)| self.element_of(doc, at).map(|element| (item, element)));
        let Some((item, element)) = found else {
            self.info(wanted);
            return;
        };
        let Some((from, _)) = self.route_from.take() else {
            self.viewer.select_item(item);
            self.route_from = Some((element, at));
            self.info(EditMsg::NoteRouteTo);
            return;
        };
        let request = RouteRequest {
            from,
            to: element,
            layer: None,
            mode: self.route_mode,
        };
        let result = match self.board_session_mut() {
            Some(s) => s.route(&request),
            None => return,
        };
        match result {
            Ok(outcome) => {
                let msg = if outcome.complete {
                    EditMsg::NoteRouted
                } else {
                    EditMsg::NoteRouteStopped
                };
                self.info(msg);
                self.rebuild();
            }
            Err(error) => self.note = Some(Note::Failed(error)),
        }
    }

    // ---- edits by name -----------------------------------------------------------------------

    /// Turns a component by one step of the settings, forwards (1) or backwards (-1).
    pub fn rotate_component(&mut self, id: Id, direction: i64) -> bool {
        let step = (self.viewer.settings.rotate_step_deg * 1_000_000.0).round() as i64;
        self.edit(EditMsg::NoteRotated, |s| {
            s.rotate_component(id, step * direction)
        })
    }

    /// Deletes a track.
    pub fn delete_track(&mut self, id: Id) -> bool {
        self.edit(EditMsg::NoteTrackDeleted, |s| s.delete_track(id))
    }

    fn selected_doc(&self) -> Option<DocRef> {
        self.viewer.selected_item().map(|item| item.doc)
    }

    /// Turns the selected component.
    pub fn rotate_selected(&mut self, direction: i64) -> bool {
        match self.selected_doc().and_then(component_of) {
            Some(id) => self.rotate_component(id, direction),
            None => {
                self.info(EditMsg::NotePickComponent);
                false
            }
        }
    }

    /// Deletes the selected track.
    pub fn delete_selected(&mut self) -> bool {
        match self.selected_doc() {
            Some(DocRef::Track(id)) => {
                self.viewer.clear_selection();
                self.delete_track(id)
            }
            _ => {
                self.info(EditMsg::NotePickTrack);
                false
            }
        }
    }

    /// Makes the selected track wider (1) or narrower (-1) by the step of the settings, and the
    /// active width with it.
    pub fn change_width(&mut self, direction: i64) -> bool {
        let step = (self.viewer.settings.width_step_mm * 1_000_000.0).round() as i64;
        let Some(DocRef::Track(id)) = self.selected_doc() else {
            self.info(EditMsg::NotePickTrack);
            return false;
        };
        let current = self
            .board_session()
            .and_then(|s| s.board.tracks.get(&id))
            .map_or(0, |t| t.width.get());
        let width = (current + step * direction).max(step.max(1));
        let done = self.edit(EditMsg::NoteWidthSet, |s| s.set_track_width(id, width));
        if done {
            self.active_width_mm = width as f64 / 1_000_000.0;
        }
        done
    }

    /// Undoes the last edit.
    pub fn undo(&mut self) -> bool {
        let done = match &mut self.doc {
            Doc::Board(s) => s.undo(),
            Doc::Schematic(s) => s.undo(),
        };
        if done {
            self.info(EditMsg::NoteUndone);
            self.rebuild();
        } else {
            self.info(EditMsg::NoteNothingToUndo);
        }
        done
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> bool {
        let done = match &mut self.doc {
            Doc::Board(s) => s.redo(),
            Doc::Schematic(s) => s.redo(),
        };
        if done {
            self.info(EditMsg::NoteRedone);
            self.rebuild();
        } else {
            self.info(EditMsg::NoteNothingToRedo);
        }
        done
    }

    /// A key with its modifiers. True when the editing took it.
    pub fn shortcut(&mut self, shortcut: Shortcut) -> bool {
        match (shortcut.key, shortcut.ctrl) {
            (Key::Z, true) => {
                if shortcut.shift {
                    self.redo();
                } else {
                    self.undo();
                }
                true
            }
            (Key::Y, true) => {
                self.redo();
                true
            }
            (Key::S, true) => {
                self.want_save = true;
                true
            }
            (_, true) => false,
            (Key::Escape, false) => {
                self.cancel();
                true
            }
            (Key::R, false) => {
                self.rotate_selected(if shortcut.shift { -1 } else { 1 });
                true
            }
            (Key::Delete, false) => {
                self.delete_selected();
                true
            }
            (Key::Plus, false) => {
                self.change_width(1);
                true
            }
            (Key::Minus, false) => {
                self.change_width(-1);
                true
            }
            (Key::F, false) => {
                self.viewer.fit();
                true
            }
            (key, false) => match tool_of(key) {
                Some(tool) => {
                    self.set_tool(tool);
                    true
                }
                None => false,
            },
        }
    }

    // ---- panels ------------------------------------------------------------------------------

    /// Centres the view on a point and marks it with a ring.
    pub fn jump_to_point(&mut self, p: P) {
        self.viewer.centre_on(p);
        self.marker = Some(p);
    }

    /// Goes to the place of a finding of the rules of a board, given in nanometres; a finding with
    /// no place says so.
    pub fn jump_to_place(&mut self, at_nm: Option<[i64; 2]>) -> bool {
        match at_nm {
            Some([x, y]) => {
                self.jump_to_point(P::new(x as f64 / 1_000_000.0, y as f64 / 1_000_000.0));
                true
            }
            None => {
                self.info(EditMsg::NoteNoPlace);
                false
            }
        }
    }

    /// Goes to the pin, the net or the part that a location of a report names: the item is
    /// selected, its net is highlighted and the view is centred on it.
    pub fn jump_to_location(&mut self, location: &str) -> bool {
        let Some(item) = find_item(&self.viewer.scene, location) else {
            self.info(EditMsg::NoteNoPlace);
            return false;
        };
        self.viewer.select_item(item);
        let area = self.viewer.item_bounds(item);
        if !area.is_empty() {
            self.jump_to_point(area.centre());
        }
        true
    }

    /// The nets of the drawing with the connections of each that are still to be routed.
    pub fn net_rows(&self) -> Vec<NetRow> {
        let mut unrouted = vec![0usize; self.viewer.scene.nets.len()];
        if let Some(session) = self.board_session() {
            for segment in &session.rat_segments {
                if let Some(count) = unrouted.get_mut(segment.net as usize) {
                    *count += 1;
                }
            }
        }
        self.viewer
            .scene
            .nets
            .iter()
            .enumerate()
            .map(|(index, name)| NetRow {
                index: index as u32,
                name: name.clone(),
                unrouted: unrouted[index],
            })
            .collect()
    }

    /// Reads a rule file and judges the board by it.
    pub fn load_rules(&mut self, path: &Path) -> bool {
        if !self.require_board() {
            return false;
        }
        let result = match self.board_session_mut() {
            Some(s) => s.load_rules(path),
            None => return false,
        };
        match result {
            Ok(()) => {
                self.info(EditMsg::NoteRulesLoaded);
                true
            }
            Err(error) => {
                self.note = Some(Note::Failed(error));
                false
            }
        }
    }

    /// Runs the electrical rule check of the schematic again.
    pub fn run_erc(&mut self) {
        if let Some(s) = self.schematic_session_mut() {
            s.run_erc();
        }
    }

    /// Opens the board that the schematic is compared with.
    pub fn open_companion(&mut self, path: &Path) -> bool {
        let settings = self.viewer.settings.clone();
        let result = match self.schematic_session_mut() {
            Some(s) => s.open_companion(path, &settings),
            None => return false,
        };
        match result {
            Ok(()) => {
                self.note = None;
                true
            }
            Err(error) => {
                self.note = Some(Note::Failed(error));
                false
            }
        }
    }

    /// The dry run of the transfer to the board (`forward`) or of the annotation back into the
    /// schematic. Nothing is changed until the package is applied.
    pub fn plan_sync(&mut self, forward: bool) -> bool {
        let Some(has_board) = self.schematic_session().map(|s| s.companion.is_some()) else {
            return false;
        };
        if !has_board {
            self.info(EditMsg::NoteNeedCompanion);
            return false;
        }
        let result = match self.schematic_session_mut() {
            Some(s) if forward => s.plan_forward(),
            Some(s) => s.plan_backward(),
            None => return false,
        };
        match result {
            Ok(()) => {
                self.info(EditMsg::NotePackageReady);
                true
            }
            Err(error) => {
                self.note = Some(Note::Failed(error));
                false
            }
        }
    }

    /// Applies the package that the dry run made, as one step that can be undone.
    pub fn apply_package(&mut self) -> bool {
        let result = match self.schematic_session_mut() {
            Some(s) => s.apply_preview(),
            None => return false,
        };
        self.finish(EditMsg::NotePackageApplied, result)
    }

    /// Throws the package that the dry run made away.
    pub fn discard_package(&mut self) {
        if let Some(s) = self.schematic_session_mut() {
            s.discard_preview();
        }
    }

    /// Saves the document under another name than the file it was read from. For a schematic the
    /// board it is compared with is saved by [`Workspace::save_companion_as`].
    pub fn save_as(&mut self, target: &Path) -> bool {
        let suffix = self.viewer.settings.backup_suffix.clone();
        let input = self.input.clone();
        let result = match &mut self.doc {
            Doc::Board(s) => s.save_as(target, input.as_deref(), &suffix),
            Doc::Schematic(s) => s.save_schematic_as(target, input.as_deref(), &suffix),
        };
        self.after_save(result)
    }

    /// Saves the board that the schematic is compared with under another name.
    pub fn save_companion_as(&mut self, target: &Path) -> bool {
        let suffix = self.viewer.settings.backup_suffix.clone();
        let result = match self.schematic_session_mut() {
            Some(s) => s.save_board_as(target, &suffix),
            None => return false,
        };
        self.after_save(result)
    }

    fn after_save(&mut self, result: Result<crate::session::Saved, ViewError>) -> bool {
        match result {
            Ok(saved) => {
                self.note = Some(Note::Saved(saved.path, saved.backup));
                true
            }
            Err(error) => {
                self.note = Some(Note::Failed(error));
                false
            }
        }
    }

    /// The kind of drawing, for the window.
    pub fn kind(&self) -> SceneKind {
        self.viewer.scene.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{EDIT_FAILED, SAVE_REFUSED};
    use crate::sample;

    fn relaxed() -> RouteSettings {
        RouteSettings {
            strict_edits: false,
            ..RouteSettings::default()
        }
    }

    fn board_workspace() -> Workspace {
        let loaded = Loaded {
            document: Document::Board(Box::new(sample::board())),
            unread: 0,
        };
        Workspace::new(loaded, None, ViewerSettings::default(), relaxed()).unwrap()
    }

    fn schematic_workspace() -> Workspace {
        let loaded = Loaded {
            document: Document::Schematic(Box::new(sample::schematic())),
            unread: 0,
        };
        Workspace::new(loaded, None, ViewerSettings::default(), relaxed()).unwrap()
    }

    fn pixel(w: &Workspace, x: f64, y: f64) -> (f64, f64) {
        w.viewer.view.to_screen(P::new(x, y))
    }

    fn click_at(w: &mut Workspace, x: f64, y: f64) {
        let (sx, sy) = pixel(w, x, y);
        w.click(sx, sy);
    }

    fn drag(w: &mut Workspace, from: (f64, f64), to: (f64, f64)) -> bool {
        let (sx, sy) = pixel(w, from.0, from.1);
        let taken = w.press(sx, sy);
        let (ex, ey) = pixel(w, to.0, to.1);
        w.drag_to(ex, ey);
        w.release(ex, ey);
        taken
    }

    fn component(w: &Workspace) -> (i64, i64) {
        let at = w.board_session().unwrap().board.components[&sample::id(30)].at;
        (at.x_nm.get(), at.y_nm.get())
    }

    fn rotation(w: &Workspace) -> i64 {
        w.board_session().unwrap().board.components[&sample::id(30)]
            .rotation
            .get()
    }

    #[test]
    fn a_workspace_opens_a_board_or_a_schematic_and_says_which() {
        let w = board_workspace();
        assert!(w.is_board() && w.board_session().is_some() && w.schematic_session().is_none());
        assert_eq!(w.kind(), SceneKind::Board);
        assert!(!w.dirty() && !w.can_undo() && !w.can_redo());
        assert!((w.active_width_mm - 0.25).abs() < 1e-9);
        let s = schematic_workspace();
        assert!(!s.is_board() && s.schematic_session().is_some() && s.board_session().is_none());
        assert_eq!(s.kind(), SceneKind::Schematic);
    }

    #[test]
    fn a_document_with_nothing_to_draw_is_refused() {
        let loaded = Loaded {
            document: Document::Board(Box::default()),
            unread: 0,
        };
        let error = Workspace::new(loaded, None, ViewerSettings::default(), relaxed())
            .err()
            .unwrap();
        assert_eq!(error.code, crate::codes::NOTHING_TO_DRAW);
    }

    #[test]
    fn a_file_is_opened_from_its_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.json");
        std::fs::write(&path, sample::board().to_bytes().unwrap()).unwrap();
        let w = Workspace::open(&path, &ViewerSettings::default(), &relaxed()).unwrap();
        assert_eq!(w.input.as_deref(), Some(path.as_path()));
        assert!(w.is_board());
        let missing = dir.path().join("none.json");
        let error = Workspace::open(&missing, &ViewerSettings::default(), &relaxed());
        assert_eq!(error.err().unwrap().code, crate::codes::FILE_UNUSABLE);
    }

    #[test]
    fn the_keys_choose_the_tools_and_a_schematic_refuses_the_ones_that_edit() {
        let mut w = board_workspace();
        for (key, tool) in [
            (Key::G, Tool::MoveComponent),
            (Key::O, Tool::RotateComponent),
            (Key::D, Tool::DeleteTrack),
            (Key::E, Tool::MoveVertex),
            (Key::W, Tool::SetWidth),
            (Key::I, Tool::AddVia),
            (Key::T, Tool::Route),
            (Key::M, Tool::Measure),
            (Key::V, Tool::Select),
        ] {
            assert!(w.shortcut(Shortcut::plain(key)));
            assert_eq!(w.viewer.tool, tool);
        }
        let mut s = schematic_workspace();
        assert!(s.shortcut(Shortcut::plain(Key::G)));
        assert_eq!(s.viewer.tool, Tool::Select);
        assert_eq!(
            s.note,
            Some(Note::Info(EditMsg::NoteBoardOnly, String::new()))
        );
        assert!(s.shortcut(Shortcut::plain(Key::M)));
        assert_eq!(s.viewer.tool, Tool::Measure);
        // a key that means nothing is not taken
        assert!(!w.shortcut(Shortcut::ctrl(Key::G)));
    }

    #[test]
    fn choosing_a_tool_says_what_to_click_and_drops_what_was_in_progress() {
        let mut w = board_workspace();
        w.shortcut(Shortcut::plain(Key::T));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteRouteFrom, String::new()))
        );
        click_at(&mut w, 8.5, 10.0);
        assert!(w.route_from.is_some());
        w.shortcut(Shortcut::plain(Key::G));
        assert!(w.route_from.is_none());
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePickComponent, String::new()))
        );
    }

    #[test]
    fn dragging_a_component_moves_it_by_the_distance_of_the_pointer_and_snaps_to_the_grid() {
        let mut w = board_workspace();
        w.set_tool(Tool::MoveComponent);
        // the pad 2 of R1 is at (11.5, 10) and the origin of R1 at (10, 10)
        let taken = drag(&mut w, (11.5, 10.0), (13.52, 10.01));
        assert!(taken);
        assert_eq!(component(&w), (12_000_000, 10_000_000));
        assert_eq!(w.note, Some(Note::Info(EditMsg::NoteMoved, String::new())));
        assert!(w.drag.is_none());
        assert!(w.dirty() && w.can_undo());
        // the drawing was made again: the pad is at its new place
        let (sx, sy) = pixel(&w, 13.5, 10.0);
        let item = w.viewer.item_at(sx, sy).unwrap();
        assert_eq!(w.viewer.scene.items[item as usize].label, "R1 pad 2");
    }

    #[test]
    fn a_drag_that_ends_where_it_began_edits_nothing_and_a_press_on_nothing_pans() {
        let mut w = board_workspace();
        w.set_tool(Tool::MoveComponent);
        assert!(drag(&mut w, (11.5, 10.0), (11.5, 10.0)));
        assert_eq!(w.board_session().unwrap().edits(), 0);
        assert!(!w.dirty());
        let (sx, sy) = pixel(&w, 28.0, 18.0);
        assert!(!w.press(sx, sy));
        assert!(w.drag.is_none());
        assert!(!w.release(sx, sy));
        // the select tool never takes a press
        w.set_tool(Tool::Select);
        let (sx, sy) = pixel(&w, 11.5, 10.0);
        assert!(!w.press(sx, sy));
    }

    #[test]
    fn undo_and_redo_by_the_keys_and_by_the_methods_bring_the_board_back_and_forth() {
        let mut w = board_workspace();
        w.set_tool(Tool::MoveComponent);
        drag(&mut w, (11.5, 10.0), (13.5, 10.0));
        assert_eq!(component(&w), (12_000_000, 10_000_000));
        assert!(w.shortcut(Shortcut::ctrl(Key::Z)));
        assert_eq!(component(&w), (10_000_000, 10_000_000));
        assert_eq!(w.note, Some(Note::Info(EditMsg::NoteUndone, String::new())));
        assert!(w.can_redo());
        assert!(w.shortcut(Shortcut::ctrl(Key::Y)));
        assert_eq!(component(&w), (12_000_000, 10_000_000));
        assert_eq!(w.note, Some(Note::Info(EditMsg::NoteRedone, String::new())));
        // Ctrl+Shift+Z redoes too
        w.shortcut(Shortcut::ctrl(Key::Z));
        let both = Shortcut {
            key: Key::Z,
            ctrl: true,
            shift: true,
        };
        assert!(w.shortcut(both));
        assert_eq!(component(&w), (12_000_000, 10_000_000));
        // nothing more to redo, and nothing to undo on a fresh board
        assert!(!w.redo());
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteNothingToRedo, String::new()))
        );
        let mut fresh = board_workspace();
        assert!(!fresh.undo());
        assert_eq!(
            fresh.note,
            Some(Note::Info(EditMsg::NoteNothingToUndo, String::new()))
        );
    }

    #[test]
    fn the_rotate_tool_and_the_key_turn_a_component_by_the_step_of_the_settings() {
        let mut w = board_workspace();
        w.set_tool(Tool::RotateComponent);
        click_at(&mut w, 11.5, 10.0);
        assert_eq!(rotation(&w), 90_000_000);
        // the component is selected, so the key turns it again, and Shift turns it back
        assert!(w.viewer.selected_item().is_some());
        assert!(w.shortcut(Shortcut::plain(Key::R)));
        assert_eq!(rotation(&w), 180_000_000);
        let back = Shortcut {
            key: Key::R,
            ctrl: false,
            shift: true,
        };
        assert!(w.shortcut(back));
        assert_eq!(rotation(&w), 90_000_000);
        // a click on nothing says what to click
        click_at(&mut w, 28.0, 18.0);
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePickComponent, String::new()))
        );
        assert_eq!(w.board_session().unwrap().edits(), 3);
    }

    #[test]
    fn the_delete_tool_and_the_key_delete_a_track_and_nothing_else() {
        let mut w = board_workspace();
        w.set_tool(Tool::DeleteTrack);
        click_at(&mut w, 11.5, 10.0);
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePickTrack, String::new()))
        );
        assert_eq!(w.board_session().unwrap().edits(), 0);
        click_at(&mut w, 7.0, 5.0);
        let board = &w.board_session().unwrap().board;
        assert!(!board.tracks.contains_key(&sample::id(40)));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteTrackDeleted, String::new()))
        );
        assert!(w.shortcut(Shortcut::ctrl(Key::Z)));
        assert!(w
            .board_session()
            .unwrap()
            .board
            .tracks
            .contains_key(&sample::id(40)));
        // with the select tool, a selected track is deleted by the key
        w.set_tool(Tool::Select);
        click_at(&mut w, 7.0, 5.0);
        assert!(w.shortcut(Shortcut::plain(Key::Delete)));
        assert!(!w
            .board_session()
            .unwrap()
            .board
            .tracks
            .contains_key(&sample::id(40)));
        assert!(w.viewer.selected.is_none());
        // with nothing selected the key says what to select
        assert!(w.shortcut(Shortcut::plain(Key::Delete)));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePickTrack, String::new()))
        );
    }

    #[test]
    fn the_width_tool_gives_a_track_the_active_width_and_the_keys_change_it_by_a_step() {
        let mut w = board_workspace();
        w.set_tool(Tool::SetWidth);
        w.active_width_mm = 0.4;
        click_at(&mut w, 7.0, 5.0);
        let width = |w: &Workspace| {
            w.board_session().unwrap().board.tracks[&sample::id(40)]
                .width
                .get()
        };
        assert_eq!(width(&w), 400_000);
        assert!(w.shortcut(Shortcut::plain(Key::Plus)));
        assert_eq!(width(&w), 450_000);
        assert!((w.active_width_mm - 0.45).abs() < 1e-9);
        assert!(w.shortcut(Shortcut::plain(Key::Minus)));
        assert!(w.shortcut(Shortcut::plain(Key::Minus)));
        assert_eq!(width(&w), 350_000);
        // the width never goes to zero or below
        for _ in 0..20 {
            w.shortcut(Shortcut::plain(Key::Minus));
        }
        assert_eq!(width(&w), 50_000);
        // on a pad it says what to click
        click_at(&mut w, 11.5, 10.0);
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePickTrack, String::new()))
        );
    }

    #[test]
    fn the_via_tool_adds_a_via_of_the_net_under_the_click() {
        let mut w = board_workspace();
        w.set_tool(Tool::AddVia);
        let vias = w.board_session().unwrap().board.vias.len();
        click_at(&mut w, 7.0, 5.0);
        let board = &w.board_session().unwrap().board;
        assert_eq!(board.vias.len(), vias + 1);
        let added = board
            .vias
            .values()
            .find(|v| v.at == sample::point(7.0, 5.0))
            .unwrap();
        assert_eq!(added.net, sample::id(10));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteViaAdded, String::new()))
        );
        // a click on nothing, or on the silkscreen that has no net, adds nothing
        click_at(&mut w, 28.0, 18.0);
        click_at(&mut w, 15.0, 3.0);
        assert_eq!(w.board_session().unwrap().board.vias.len(), vias + 1);
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePickNet, String::new()))
        );
    }

    #[test]
    fn dragging_a_vertex_moves_the_nearest_one_and_a_press_far_from_every_vertex_pans() {
        let mut w = board_workspace();
        w.set_tool(Tool::MoveVertex);
        let (sx, sy) = pixel(&w, 7.0, 5.0);
        assert!(!w.press(sx, sy));
        assert!(drag(&mut w, (9.0, 5.0), (9.0, 9.0)));
        let track = &w.board_session().unwrap().board.tracks[&sample::id(40)];
        assert_eq!(track.path[0].end(), sample::point(9.0, 9.0));
        assert_eq!(track.path[0].start(), sample::point(5.0, 5.0));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteVertexMoved, String::new()))
        );
        // a move that would turn the piece off its angles is refused and says why
        assert!(drag(&mut w, (9.0, 9.0), (9.0, 12.0)));
        assert!(w.note.as_ref().unwrap().is_error());
        assert_eq!(
            w.board_session().unwrap().board.tracks[&sample::id(40)].path[0].end(),
            sample::point(9.0, 9.0)
        );
        assert_eq!(w.board_session().unwrap().edits(), 1);
    }

    #[test]
    fn the_preview_lines_follow_the_drag_and_the_first_click_of_a_route() {
        let mut w = board_workspace();
        assert!(w.preview_lines().is_empty());
        w.set_tool(Tool::MoveComponent);
        let (sx, sy) = pixel(&w, 11.5, 10.0);
        assert!(w.press(sx, sy));
        let (ex, ey) = pixel(&w, 13.5, 10.0);
        w.drag_to(ex, ey);
        // the line from the origin to the new place and the two arms of a cross
        assert_eq!(w.preview_lines().len(), 3);
        w.cancel();
        assert!(w.preview_lines().is_empty());
        w.set_tool(Tool::MoveVertex);
        let (sx, sy) = pixel(&w, 9.0, 5.0);
        assert!(w.press(sx, sy));
        let (ex, ey) = pixel(&w, 9.0, 8.0);
        w.drag_to(ex, ey);
        // the vertex 1 of a track of one piece has only a neighbour before it
        assert_eq!(w.preview_lines().len(), 1);
        w.cancel();
        w.set_tool(Tool::Route);
        click_at(&mut w, 8.5, 10.0);
        let (hx, hy) = pixel(&w, 12.0, 12.0);
        w.hover(hx, hy);
        assert_eq!(w.preview_lines().len(), 1);
        w.leave();
        assert!(w.preview_lines().is_empty());
    }

    #[test]
    fn the_route_tool_takes_the_origin_and_then_the_destination_and_joins_them() {
        let mut w = board_workspace();
        w.set_tool(Tool::Route);
        // a click on nothing does not start a route
        click_at(&mut w, 28.0, 18.0);
        assert!(w.route_from.is_none());
        // the pad 1 of R1 and the end of the track of GND, both of the net GND
        let before = w.board_session().unwrap().rat.len();
        click_at(&mut w, 8.5, 10.0);
        assert_eq!(
            w.route_from.as_ref().map(|(e, _)| e.clone()),
            Some(Element::Pad(eh3d_phoenix_board::PadRef {
                component: sample::id(30),
                pad: sample::id(21),
            }))
        );
        click_at(&mut w, 9.0, 5.0);
        assert!(w.route_from.is_none());
        assert_eq!(w.note, Some(Note::Info(EditMsg::NoteRouted, String::new())));
        let session = w.board_session().unwrap();
        assert_eq!(session.edits(), 1);
        assert!(session.rat.len() < before);
        // the drawing shows the new track
        assert!(w.viewer.scene.items.len() > 9);
        assert!(w.shortcut(Shortcut::ctrl(Key::Z)));
        assert_eq!(w.board_session().unwrap().rat.len(), before);
    }

    #[test]
    fn a_route_between_two_nets_is_refused_and_escape_drops_the_first_click() {
        let mut w = board_workspace();
        w.set_tool(Tool::Route);
        click_at(&mut w, 8.5, 10.0);
        // the pad 2 is of the net VCC
        click_at(&mut w, 11.5, 10.0);
        assert!(w.route_from.is_none());
        match &w.note {
            Some(Note::Failed(error)) => assert_eq!(error.code, EDIT_FAILED),
            other => panic!("{other:?}"),
        }
        assert_eq!(w.board_session().unwrap().edits(), 0);
        click_at(&mut w, 8.5, 10.0);
        assert!(w.route_from.is_some());
        assert!(w.shortcut(Shortcut::plain(Key::Escape)));
        assert!(w.route_from.is_none());
        assert!(w.viewer.selected.is_none());
    }

    #[test]
    fn the_stop_mode_is_a_setting_of_the_workspace() {
        let mut w = board_workspace();
        assert_eq!(w.route_mode, Mode::Around);
        w.route_mode = Mode::Stop;
        w.set_tool(Tool::Route);
        click_at(&mut w, 8.5, 10.0);
        click_at(&mut w, 9.0, 5.0);
        // the straight way is free, so it reaches the end in this mode too
        assert!(matches!(
            w.note,
            Some(Note::Info(
                EditMsg::NoteRouted | EditMsg::NoteRouteStopped,
                _
            ))
        ));
        assert_eq!(w.board_session().unwrap().edits(), 1);
    }

    #[test]
    fn a_finding_jumps_to_its_place_and_one_without_a_place_says_so() {
        let mut w = board_workspace();
        assert!(w.jump_to_place(Some([15_000_000, 15_000_000])));
        assert_eq!(w.viewer.view.centre, P::new(15.0, 15.0));
        assert_eq!(w.marker, Some(P::new(15.0, 15.0)));
        w.marker = None;
        assert!(!w.jump_to_place(None));
        assert!(w.marker.is_none());
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteNoPlace, String::new()))
        );
    }

    #[test]
    fn a_location_of_the_electrical_check_is_found_on_the_schematic_and_selected() {
        let scene =
            Scene::from_schematic(&sample::schematic(), &ViewerSettings::default()).unwrap();
        let pin = find_item(&scene, "R1.2").unwrap();
        assert_eq!(scene.items[pin as usize].kind, ItemKind::Pin);
        assert_eq!(scene.items[pin as usize].label, "R1.2");
        let net = find_item(&scene, "net N1").unwrap();
        assert_eq!(scene.items[net as usize].kind, ItemKind::NetLabel);
        let part = find_item(&scene, "R2").unwrap();
        assert_eq!(scene.items[part as usize].kind, ItemKind::Part);
        assert!(find_item(&scene, "R9.1").is_none());
        assert!(find_item(&scene, "net GND").is_none());
        assert!(find_item(&scene, "").is_none());
        let mut w = schematic_workspace();
        assert!(w.jump_to_location("R1.2"));
        assert_eq!(w.viewer.selected_item().unwrap().label, "R1.2");
        assert_eq!(w.viewer.highlight_name(), Some("N1"));
        // the pin R1.2 is at (25, 40) on the first sheet
        let centre = w.viewer.view.centre;
        assert!(
            (centre.x - 25.0).abs() < 2.0 && (centre.y - 40.0).abs() < 2.0,
            "{centre:?}"
        );
        assert!(w.marker.is_some());
        assert!(!w.jump_to_location("R9.9"));
    }

    #[test]
    fn a_finding_is_written_as_one_line_in_the_language() {
        let ctx = eh3d_phoenix_route::RuleContext {
            rules: eh3d_phoenix_rules::RuleValues {
                min_track_width: eh3d_phoenix_model::Nm::new(200_000).unwrap(),
                ..Default::default()
            },
            classes: Default::default(),
        };
        let mut w = board_workspace();
        w.board_session_mut().unwrap().set_context(ctx);
        w.set_tool(Tool::SetWidth);
        w.active_width_mm = 0.1;
        click_at(&mut w, 7.0, 5.0);
        let session = w.board_session().unwrap();
        let added = session
            .last_diff
            .added
            .iter()
            .find(|f| f.severity == Severity::Error)
            .expect("a track narrower than the rule is an error");
        let line = finding_line(added, Lang::En);
        assert!(line.starts_with("[error] PHOENIX-"), "{line}");
        assert!(line.ends_with(&added.location), "{line}");
        assert!(finding_line(added, Lang::De).starts_with("[Fehler] PHOENIX-"));
        assert!(finding_line(added, Lang::Pt).starts_with("[erro] PHOENIX-"));
        // the title of the code is in the line
        let title = eh3d_phoenix_rules::title(added.code, Lang::En).unwrap();
        assert!(line.contains(title), "{line}");
    }

    #[test]
    fn a_finding_of_the_electrical_check_is_written_with_its_detail() {
        let s = schematic_workspace();
        let session = s.schematic_session().unwrap();
        // the parts of the sample have no footprint and their first pins are on no net
        let finding = session
            .erc
            .confirmed
            .iter()
            .find(|f| f.location == "R1.1")
            .expect("the first pin of R1 is on no net");
        let line = erc_line(finding, Lang::En);
        assert!(line.starts_with('['), "{line}");
        assert!(line.contains(finding.code), "{line}");
        assert!(line.contains("R1.1"), "{line}");
        let spanish = erc_line(finding, Lang::Es);
        assert!(spanish.contains(finding.code), "{spanish}");
        // a location of the check is found on the drawing: the pin
        let mut w = schematic_workspace();
        assert!(w.jump_to_location(&finding.location));
        assert_eq!(w.viewer.selected_item().unwrap().label, "R1.1");
    }

    #[test]
    fn the_name_proposed_for_saving_is_beside_the_input_and_is_not_the_input() {
        let input = Path::new("designs").join("board.brd");
        let proposed = suggested_save_path(&input);
        assert_eq!(proposed, Path::new("designs").join("board_edited.json"));
        assert_ne!(proposed, input);
        assert_eq!(
            suggested_save_path(Path::new("board.json")),
            PathBuf::from("board_edited.json")
        );
    }

    #[test]
    fn the_nets_are_listed_with_the_connections_still_to_be_routed() {
        let w = board_workspace();
        let rows = w.net_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "GND");
        assert_eq!(rows[1].name, "VCC");
        let session = w.board_session().unwrap();
        for row in &rows {
            let expected = session
                .rat_segments
                .iter()
                .filter(|s| s.net == row.index)
                .count();
            assert_eq!(row.unrouted, expected);
        }
        let total: usize = rows.iter().map(|r| r.unrouted).sum();
        assert_eq!(total, session.rat.len());
        // a schematic has none to route
        let s = schematic_workspace();
        assert!(s.net_rows().iter().all(|r| r.unrouted == 0));
    }

    #[test]
    fn a_net_chosen_in_the_list_stays_highlighted_after_an_edit() {
        let mut w = board_workspace();
        w.viewer.select_net(1);
        assert_eq!(w.viewer.highlight_name(), Some("VCC"));
        assert!(w.rotate_component(sample::id(30), 1));
        assert_eq!(w.viewer.highlight_name(), Some("VCC"));
        assert!(w.viewer.selected.is_none());
    }

    #[test]
    fn the_notes_are_written_in_the_language_and_an_error_carries_its_code() {
        let note = Note::Info(EditMsg::NoteMoved, String::new());
        assert_eq!(note.text(Lang::Es), "Componente movido");
        assert_eq!(note.text(Lang::Pt), "Componente movido");
        assert!(!note.is_error());
        let with_detail = Note::Info(EditMsg::NoteRulesLoaded, "a.json".to_string());
        assert_eq!(with_detail.text(Lang::En), "Rules loaded: a.json");
        let saved = Note::Saved(PathBuf::from("a.json"), None);
        assert_eq!(saved.text(Lang::En), "Saved: a.json");
        assert_eq!(saved.text(Lang::De), "Gespeichert: a.json");
        let kept = Note::Saved(PathBuf::from("a.json"), Some(PathBuf::from("a.json.bak")));
        assert_eq!(
            kept.text(Lang::Pt),
            "Guardado: a.json (cópia do ficheiro anterior: a.json.bak)"
        );
        assert!(!saved.is_error() && !kept.is_error());
        let failed = Note::Failed(ViewError::new(EDIT_FAILED, "no such track"));
        assert!(failed.is_error());
        assert_eq!(
            failed.text(Lang::En),
            "PHOENIX-VIEW-010: The edit could not be made: no such track"
        );
    }

    #[test]
    fn control_s_asks_for_a_place_to_save_and_the_input_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.json");
        let original = sample::board().to_bytes().unwrap();
        std::fs::write(&path, &original).unwrap();
        let mut w = Workspace::open(&path, &ViewerSettings::default(), &relaxed()).unwrap();
        assert!(w.shortcut(Shortcut::ctrl(Key::S)));
        assert!(w.want_save);
        w.set_tool(Tool::RotateComponent);
        click_at(&mut w, 11.5, 10.0);
        assert!(w.dirty());
        assert!(!w.save_as(&path));
        match &w.note {
            Some(Note::Failed(error)) => assert_eq!(error.code, SAVE_REFUSED),
            other => panic!("{other:?}"),
        }
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let target = dir.path().join("rotated.json");
        assert!(w.save_as(&target));
        assert!(!w.dirty());
        assert_eq!(w.note, Some(Note::Saved(target.clone(), None)));
        // the component turned and its pad 2 is now at (10, 11.5); the next save keeps a copy of
        // the first file and says so
        click_at(&mut w, 10.0, 11.5);
        assert_eq!(rotation(&w), 180_000_000);
        assert!(w.save_as(&target));
        let copy = dir.path().join("rotated.json.bak");
        assert_eq!(
            w.note,
            Some(Note::Saved(target.clone(), Some(copy.clone())))
        );
        assert!(copy.exists());
    }

    #[test]
    fn the_rules_of_a_file_replace_the_default_rules_of_a_board_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.json");
        let rules = eh3d_phoenix_rules::RuleValues {
            min_track_width: eh3d_phoenix_model::Nm::new(300_000).unwrap(),
            ..Default::default()
        };
        std::fs::write(&path, rules.to_json("mine")).unwrap();
        let mut w = board_workspace();
        assert!(w.load_rules(&path));
        assert_eq!(
            w.board_session().unwrap().ctx.rules.min_track_width.get(),
            300_000
        );
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteRulesLoaded, String::new()))
        );
        assert!(!w.load_rules(&dir.path().join("none.json")));
        assert!(w.note.as_ref().unwrap().is_error());
        let mut s = schematic_workspace();
        assert!(!s.load_rules(&path));
    }

    #[test]
    fn the_synchronisation_is_planned_applied_and_undone_from_the_workspace() {
        let mut w = schematic_workspace();
        // without a board there is nothing to compare with
        assert!(!w.plan_sync(true));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NoteNeedCompanion, String::new()))
        );
        let mut board = sample::board();
        let component = board.components.get_mut(&sample::id(30)).unwrap();
        component.symbol = Some(sample::sch_id(10));
        component.value = "22k".to_string();
        w.schematic_session_mut()
            .unwrap()
            .set_companion(PathBuf::from("board.json"), board);
        // the dry run changes nothing
        assert!(w.plan_sync(false));
        assert_eq!(
            w.note,
            Some(Note::Info(EditMsg::NotePackageReady, String::new()))
        );
        let session = w.schematic_session().unwrap();
        assert!(session.preview_has_changes());
        assert_eq!(session.schematic.parts[&sample::sch_id(10)].value, "10k");
        assert!(w.apply_package());
        let session = w.schematic_session().unwrap();
        assert_eq!(session.schematic.parts[&sample::sch_id(10)].value, "22k");
        assert!(w.dirty() && w.can_undo());
        // the drawing shows the new value in the label of the part
        assert!(w
            .viewer
            .scene
            .items
            .iter()
            .any(|i| i.kind == ItemKind::Part && i.label == "R1 22k"));
        assert!(w.shortcut(Shortcut::ctrl(Key::Z)));
        assert!(w
            .viewer
            .scene
            .items
            .iter()
            .any(|i| i.kind == ItemKind::Part && i.label == "R1 10k"));
        // a package that is discarded is gone
        assert!(w.plan_sync(true));
        w.discard_package();
        assert!(w.schematic_session().unwrap().preview.is_none());
        // applying with no package is an error
        assert!(!w.apply_package());
        assert!(w.note.as_ref().unwrap().is_error());
        // a board has none of this
        let mut b = board_workspace();
        assert!(!b.plan_sync(true));
        assert!(!b.apply_package());
    }

    #[test]
    fn a_companion_board_is_opened_from_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("board.json");
        std::fs::write(&path, sample::board().to_bytes().unwrap()).unwrap();
        let mut w = schematic_workspace();
        assert!(w.open_companion(&path));
        assert!(w.schematic_session().unwrap().companion.is_some());
        assert!(!w.open_companion(&dir.path().join("none.json")));
        assert!(w.note.as_ref().unwrap().is_error());
        w.run_erc();
        let mut b = board_workspace();
        assert!(!b.open_companion(&path));
    }
}
