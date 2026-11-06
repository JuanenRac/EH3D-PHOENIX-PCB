// SPDX-License-Identifier: GPL-3.0-or-later
//! The board editor in the window (built with the `gui` feature). It draws the state of
//! [`EditState`] with egui and passes on the pointer and the keys; every decision is taken in
//! `eh3d-phoenix-boardedit`, in code that is tested. After each change the board is handed to the
//! workspace (`Workspace::replace_board`) so that the findings, the ratsnest and the saving work
//! on what was edited. This file has not been seen on a screen and has not been used with a mouse.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, StackFunction};
use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, EditState, Picked, Tool};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_route::copper_layers;
use eh3d_phoenix_rules::{Lang, Severity};

use crate::board_autoroute::AutorouteTool;
use crate::board_pair::PairTool;
use crate::board_shove::ShoveTool;
use crate::board_text::BoardMsg;
use crate::board_zone::ZoneTool;
use crate::editmsg::EditMsg;
use crate::gui::{ui_accent, ui_danger, ui_dim, ui_text};
use crate::workspace::Workspace;

// the colours of the theme Forge, as in the window of the viewer
const TOP_COPPER: Color32 = Color32::from_rgb(0xd8, 0x4a, 0x4a);
const BOTTOM_COPPER: Color32 = Color32::from_rgb(0x4a, 0x7a, 0xd8);
const INNER_COPPER: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const PAD: Color32 = Color32::from_rgb(0xe0, 0xc0, 0x60);
const ZONE: Color32 = Color32::from_rgb(0x60, 0x50, 0x40);
const RAT: Color32 = Color32::from_rgb(0xb8, 0xb0, 0xa4);
const GRID: Color32 = Color32::from_rgb(0x40, 0x38, 0x30);

/// The smallest and the largest zoom, in pixels per millimetre.
const SCALE_MIN: f64 = 0.2;
const SCALE_MAX: f64 = 400.0;

/// The board editor of the window: the editor with its tools, and how it is looked at.
pub struct BoardEditUi {
    state: EditState,
    center: (f64, f64),
    scale: f64,
    fitted: bool,
    synced: u64,
    rat_revision: u64,
    rat: Vec<(Point, Point)>,
    length_key: Option<(u64, Id)>,
    length_mm: Option<f64>,
    target_mm: f64,
    pair: PairTool,
    shove: ShoveTool,
    zone: ZoneTool,
    autoroute: AutorouteTool,
}

/// Where a point of the drawing is on the screen, and back.
#[derive(Clone, Copy)]
struct View {
    origin: Pos2,
    cx: f64,
    cy: f64,
    scale: f64,
}

impl View {
    fn xy(&self, x_mm: f64, y_mm: f64) -> Pos2 {
        Pos2::new(
            self.origin.x + ((x_mm - self.cx) * self.scale) as f32,
            self.origin.y - ((y_mm - self.cy) * self.scale) as f32,
        )
    }

    fn at(&self, p: Point) -> Pos2 {
        self.xy(mm(p.x_nm.get()), mm(p.y_nm.get()))
    }

    fn world_at(&self, p: Pos2) -> (f64, f64) {
        (
            self.cx + f64::from(p.x - self.origin.x) / self.scale,
            self.cy - f64::from(p.y - self.origin.y) / self.scale,
        )
    }
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

fn point_at(x_mm: f64, y_mm: f64) -> Option<Point> {
    if !x_mm.is_finite() || !y_mm.is_finite() {
        return None;
    }
    Point::from_nm(
        (x_mm * 1_000_000.0).round() as i64,
        (y_mm * 1_000_000.0).round() as i64,
    )
    .ok()
}

fn heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).strong().color(ui_accent())
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(ui_dim())
}

/// The settings of the editor: `config/boardedit.cfg` when it is there and good, else the built-in
/// ones.
fn load_settings() -> BoardEditSettings {
    let path = "config/boardedit.cfg";
    let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= 1 << 20);
    if !small {
        return BoardEditSettings::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| BoardEditSettings::parse(&text).ok())
        .unwrap_or_default()
}

/// Starts editing the board that is open; `None` when the open document is a schematic.
pub fn start(workspace: &Workspace) -> Option<BoardEditUi> {
    let session = workspace.board_session()?;
    let mut edit =
        BoardEdit::new(session.board.clone(), session.ctx.clone(), load_settings()).ok()?;
    edit.set_pair_config(crate::board_pair_gui::load_config());
    Some(BoardEditUi {
        state: EditState::new(edit),
        center: (30.0, 20.0),
        scale: 3.0,
        fitted: false,
        synced: 0,
        rat_revision: u64::MAX,
        rat: Vec::new(),
        length_key: None,
        length_mm: None,
        target_mm: 0.0,
        pair: PairTool::default(),
        shove: ShoveTool::default(),
        zone: ZoneTool::default(),
        autoroute: AutorouteTool::with_config(crate::board_autoroute::load_config()),
    })
}

/// Hands the board as it is now to the workspace, when it changed since the last time.
pub fn finish(st: &mut BoardEditUi, workspace: &mut Workspace) {
    let revision = st.state.edit.revision();
    if revision != st.synced {
        st.synced = revision;
        workspace.replace_board(st.state.board().clone());
    }
}

/// The box that holds the outline of the board, or a default box: `(min x, min y, max x, max y)`
/// in millimetres.
fn bounds(board: &Board) -> (f64, f64, f64, f64) {
    let mut found: Option<(f64, f64, f64, f64)> = None;
    for p in &board.outline {
        let (x, y) = (mm(p.x_nm.get()), mm(p.y_nm.get()));
        found = Some(match found {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }
    found.unwrap_or((0.0, 0.0, 60.0, 40.0))
}

impl BoardEditUi {
    fn view(&self, rect: Rect) -> View {
        View {
            origin: rect.center(),
            cx: self.center.0,
            cy: self.center.1,
            scale: self.scale,
        }
    }

    fn fit(&mut self, rect: Rect) {
        let (x0, y0, x1, y1) = bounds(self.state.board());
        let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        self.center = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let sx = f64::from(rect.width()) / (w * 1.1);
        let sy = f64::from(rect.height()) / (h * 1.1);
        self.scale = sx.min(sy).clamp(SCALE_MIN, SCALE_MAX);
    }

    fn zoom_at(&mut self, rect: Rect, cursor: Pos2, scroll: f32, pinch: f32) {
        let (wx, wy) = self.view(rect).world_at(cursor);
        let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
        self.scale = (self.scale * factor).clamp(SCALE_MIN, SCALE_MAX);
        self.center.0 = wx - f64::from(cursor.x - rect.center().x) / self.scale;
        self.center.1 = wy + f64::from(cursor.y - rect.center().y) / self.scale;
    }

    /// The ratsnest and the length of the lit net, made again only when the board changed.
    fn refresh(&mut self) {
        let revision = self.state.edit.revision();
        if self.rat_revision != revision {
            self.rat_revision = revision;
            self.rat = self
                .state
                .ratsnest()
                .iter()
                .map(|line| (line.from_at, line.to_at))
                .collect();
        }
        let key = self.state.highlight_net.map(|net| (revision, net));
        if key != self.length_key {
            self.length_key = key;
            self.length_mm = key
                .and_then(|(_, net)| self.state.edit.net_length_nm(net))
                .map(mm);
        }
    }
}

/// The keys the editor answers to, read once per frame.
#[derive(Default)]
struct Keys {
    a: bool,
    v: bool,
    p: bool,
    t: bool,
    i: bool,
    d: bool,
    l: bool,
    r: bool,
    x: bool,
    k: bool,
    z: bool,
    y: bool,
    delete: bool,
    escape: bool,
    space: bool,
    enter: bool,
    command: bool,
    shift: bool,
}

fn read_keys(ctx: &egui::Context) -> Keys {
    if ctx.egui_wants_keyboard_input() {
        return Keys::default();
    }
    ctx.input(|i| Keys {
        a: i.key_pressed(egui::Key::A),
        v: i.key_pressed(egui::Key::V),
        p: i.key_pressed(egui::Key::P),
        t: i.key_pressed(egui::Key::T),
        i: i.key_pressed(egui::Key::I),
        d: i.key_pressed(egui::Key::D),
        l: i.key_pressed(egui::Key::L),
        r: i.key_pressed(egui::Key::R),
        x: i.key_pressed(egui::Key::X),
        k: i.key_pressed(egui::Key::K),
        z: i.key_pressed(egui::Key::Z),
        y: i.key_pressed(egui::Key::Y),
        delete: i.key_pressed(egui::Key::Delete),
        escape: i.key_pressed(egui::Key::Escape),
        space: i.key_pressed(egui::Key::Space),
        enter: i.key_pressed(egui::Key::Enter),
        command: i.modifiers.command,
        shift: i.modifiers.shift,
    })
}

fn apply_keys(st: &mut BoardEditUi, keys: &Keys) {
    if keys.command {
        if keys.z {
            if keys.shift {
                st.state.redo();
            } else {
                st.state.undo();
            }
        } else if keys.y {
            st.state.redo();
        }
        return;
    }
    if keys.escape {
        st.state.cancel();
        st.autoroute.clear();
    }
    if keys.a {
        st.autoroute.key(&mut st.state, keys.shift);
    }
    if keys.space {
        st.state.horizontal_first = !st.state.horizontal_first;
    }
    if keys.enter {
        st.state.finish_route();
    }
    for (pressed, tool) in [
        (keys.v, Tool::Select),
        (keys.p, Tool::Place),
        (keys.t, Tool::Route),
        (keys.i, Tool::Via),
        (keys.d, Tool::Delete),
        (keys.l, Tool::Tune),
    ] {
        if pressed {
            st.state.set_tool(tool);
        }
    }
    if keys.r {
        st.state.rotate_selection(if keys.shift { -1 } else { 1 });
    }
    if keys.x {
        st.state.mirror_selection();
    }
    if keys.k {
        st.state.toggle_lock_selection();
    }
    if keys.delete {
        st.state.delete_selection();
    }
}

/// What the pointer did in this frame.
struct Frame {
    pressed: bool,
    released: bool,
    latest: Option<Pos2>,
    shift: bool,
    scroll: f32,
    pinch: f32,
}

/// The drawing area: pointer, keys, and the drawing of the board with what is in progress.
pub fn canvas(
    ui: &mut egui::Ui,
    st: &mut BoardEditUi,
    workspace: &mut Workspace,
    lang: Lang,
    background: Color32,
    fit_pending: &mut bool,
) {
    let size = ui.available_size();
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0.0, background);
    let painter = painter.with_clip_rect(rect);
    if (!st.fitted || *fit_pending) && rect.width() > 40.0 && rect.height() > 40.0 {
        st.fit(rect);
        st.fitted = true;
        *fit_pending = false;
    }
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let d = response.drag_delta();
        st.center.0 -= f64::from(d.x) / st.scale;
        st.center.1 += f64::from(d.y) / st.scale;
    }
    let frame = ui.input(|i| Frame {
        pressed: i.pointer.primary_pressed(),
        released: i.pointer.primary_released(),
        latest: i.pointer.latest_pos(),
        shift: i.modifiers.shift,
        scroll: i.smooth_scroll_delta.y,
        pinch: i.zoom_delta(),
    });
    let inside = frame.latest.filter(|p| rect.contains(*p));
    if let Some(cursor) = inside {
        if frame.scroll.abs() > f32::EPSILON || (frame.pinch - 1.0).abs() > 1e-6 {
            st.zoom_at(rect, cursor, frame.scroll, frame.pinch);
        }
    }
    let view = st.view(rect);
    let pointer = frame.latest.and_then(|p| {
        let (x, y) = view.world_at(p);
        point_at(x, y)
    });
    if let Some(at) = pointer.filter(|_| st.zone.on) {
        if frame.pressed && inside.is_some() {
            st.zone.press(&st.state, at);
        } else if st.zone.dragging.is_some() {
            st.zone.drag_to(&st.state, at);
        }
        if frame.released {
            st.zone.release();
        }
    }
    if let Some(at) = pointer.filter(|_| st.shove.on && !st.zone.on) {
        if frame.pressed && inside.is_some() {
            st.shove.click(&mut st.state, at);
        }
    }
    if let Some(at) = pointer.filter(|_| st.pair.on && !st.shove.on && !st.zone.on) {
        if frame.pressed && inside.is_some() {
            st.pair.click(&st.state, at);
        }
    }
    if let Some(at) = pointer.filter(|_| !st.pair.on && !st.shove.on && !st.zone.on) {
        if frame.pressed && inside.is_some() {
            st.state.pointer_pressed(at, frame.shift);
        } else {
            st.state.pointer_moved(at);
        }
        if frame.released {
            st.state.pointer_released(at);
        }
    }
    let keys = read_keys(ui.ctx());
    apply_keys(st, &keys);
    finish(st, workspace);
    st.refresh();

    // the drawing: the board as the drag in progress would leave it, or as it is
    let preview = st.state.drag_preview();
    let shown: &Board = preview.as_ref().unwrap_or_else(|| st.state.board());
    draw_board(
        &painter,
        &view,
        rect,
        shown,
        &st.state,
        &st.rat,
        st.state.edit.settings.grid_nm,
    );
    if let Some(at) = pointer.filter(|_| inside.is_some()) {
        let path = st.state.route_preview(at);
        if path.len() >= 2 {
            let points: Vec<Pos2> = path.iter().map(|p| view.at(*p)).collect();
            painter.add(egui::Shape::line(points, Stroke::new(1.5, ui_accent())));
        }
        let target = st.state.snap(at);
        painter.circle_stroke(view.at(target), 5.0, Stroke::new(1.0, ui_accent()));
    }
    crate::board_pair_gui::draw(&painter, &st.pair, &|p: Point| view.at(p));
    crate::board_shove_gui::draw(&painter, &st.shove, &|p: Point| view.at(p));
    crate::board_autoroute_gui::draw(&painter, &st.autoroute, &st.state, &|p: Point| view.at(p));
    crate::board_zone_gui::draw(&painter, &st.zone, &|p: Point| view.at(p));
    if let Some((a, b)) = st.state.marquee {
        let corners = vec![
            view.xy(mm(a.x_nm.get()), mm(a.y_nm.get())),
            view.xy(mm(b.x_nm.get()), mm(a.y_nm.get())),
            view.xy(mm(b.x_nm.get()), mm(b.y_nm.get())),
            view.xy(mm(a.x_nm.get()), mm(b.y_nm.get())),
        ];
        painter.add(egui::Shape::closed_line(
            corners,
            Stroke::new(1.0, ui_accent()),
        ));
    }
    let hint = match st.state.tool {
        Tool::Select => BoardMsg::HintSelect,
        Tool::Place => BoardMsg::HintPlace,
        Tool::Route => BoardMsg::HintRoute,
        Tool::Via => BoardMsg::HintVia,
        Tool::Delete => BoardMsg::HintDelete,
        Tool::Tune => BoardMsg::HintTune,
    };
    painter.text(
        rect.left_bottom() + egui::vec2(8.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        hint.tr(lang),
        egui::FontId::proportional(13.0),
        ui_dim(),
    );
}

fn copper_colour(board: &Board, layer: Id) -> Color32 {
    match board.stack_position(layer) {
        Some((_, StackFunction::Top)) => TOP_COPPER,
        Some((_, StackFunction::Bottom)) => BOTTOM_COPPER,
        _ => INNER_COPPER,
    }
}

/// The size of a pad in millimetres, before the turn of its component.
fn pad_size_mm(shape: PadShape) -> (f64, f64) {
    match shape {
        PadShape::Circle { diameter } => (mm(diameter.get()), mm(diameter.get())),
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            (mm(width.get()), mm(height.get()))
        }
    }
}

fn draw_board(
    painter: &egui::Painter,
    view: &View,
    rect: Rect,
    board: &Board,
    st: &EditState,
    rat: &[(Point, Point)],
    grid_nm: i64,
) {
    if !board.outline.is_empty() {
        let corners: Vec<Pos2> = board.outline.iter().map(|p| view.at(*p)).collect();
        painter.add(egui::Shape::closed_line(
            corners,
            Stroke::new(1.0, ui_dim()),
        ));
    }
    let step = mm(grid_nm);
    if step > 0.0 && step * view.scale >= 6.0 {
        let (x0, y1) = view.world_at(rect.left_top());
        let (x1, y0) = view.world_at(rect.right_bottom());
        let columns = ((x1 - x0) / step).ceil() as i64;
        let rows = ((y1 - y0) / step).ceil() as i64;
        if columns > 0 && rows > 0 && columns.saturating_mul(rows) <= 40_000 {
            let first_x = (x0 / step).floor() as i64;
            let first_y = (y0 / step).floor() as i64;
            for i in 0..=columns {
                for j in 0..=rows {
                    let x = (first_x + i) as f64 * step;
                    let y = (first_y + j) as f64 * step;
                    painter.circle_filled(view.xy(x, y), 0.7, GRID);
                }
            }
        }
    }
    for zone in board.zones.values() {
        let corners: Vec<Pos2> = zone.outline.iter().map(|p| view.at(*p)).collect();
        if corners.len() >= 3 {
            painter.add(egui::Shape::closed_line(corners, Stroke::new(1.0, ZONE)));
        }
    }
    for (a, b) in rat {
        painter.line_segment([view.at(*a), view.at(*b)], Stroke::new(1.0, RAT));
    }
    for (id, track) in &board.tracks {
        let selected = st.selection.contains(&Picked::Track(*id));
        let lit = st.highlight_net == Some(track.net);
        let colour = if selected || lit {
            ui_accent()
        } else {
            copper_colour(board, track.layer)
        };
        let width = ((mm(track.width.get()) * view.scale) as f32).max(1.0);
        let width = if selected { width + 1.5 } else { width };
        for segment in &track.path {
            painter.line_segment(
                [view.at(segment.start()), view.at(segment.end())],
                Stroke::new(width, colour),
            );
        }
    }
    for (id, via) in &board.vias {
        let selected = st.selection.contains(&Picked::Via(*id));
        let lit = st.highlight_net == Some(via.net);
        let colour = if selected || lit { ui_accent() } else { PAD };
        let radius = ((mm(via.diameter.get()) * view.scale / 2.0) as f32).max(2.0);
        painter.circle_filled(view.at(via.at), radius, colour);
        let hole = ((mm(via.drill.get()) * view.scale / 2.0) as f32).max(1.0);
        painter.circle_stroke(view.at(via.at), hole, Stroke::new(1.0, ui_dim()));
    }
    let text_shown = view.scale >= 6.0;
    for (cid, component) in &board.components {
        let selected = st.selection.contains(&Picked::Component(*cid));
        let locked = st.edit.is_locked(*cid);
        let colour = if selected {
            ui_accent()
        } else if locked {
            ui_danger()
        } else {
            PAD
        };
        let turned = (component.rotation.normalised().get() / 90_000_000) % 2 == 1;
        if let Some(footprint) = board.footprints.get(&component.footprint) {
            for (pid, pad) in &footprint.pads {
                let pad_ref = PadRef {
                    component: *cid,
                    pad: *pid,
                };
                let Some(at) = pad_position(board, pad_ref) else {
                    continue;
                };
                let centre = view.at(at);
                let lit = board
                    .net_of(pad_ref)
                    .is_some_and(|net| st.highlight_net == Some(net));
                let fill = if lit { ui_accent() } else { colour };
                let (w, h) = pad_size_mm(pad.shape);
                let (w, h) = if turned { (h, w) } else { (w, h) };
                match pad.shape {
                    PadShape::Circle { .. } => {
                        painter.circle_filled(
                            centre,
                            ((w * view.scale / 2.0) as f32).max(1.5),
                            fill,
                        );
                    }
                    _ => {
                        let size = egui::vec2(
                            ((w * view.scale) as f32).max(2.0),
                            ((h * view.scale) as f32).max(2.0),
                        );
                        painter.rect_filled(Rect::from_center_size(centre, size), 0.0, fill);
                    }
                }
            }
        }
        if text_shown {
            painter.text(
                view.at(component.at) + egui::vec2(0.0, -10.0),
                egui::Align2::CENTER_BOTTOM,
                &component.refdes,
                egui::FontId::proportional((view.scale * 1.6).clamp(7.0, 16.0) as f32),
                if selected { ui_accent() } else { ui_text() },
            );
        }
    }
}

/// The panel of the tools. Returns true when the person asked to stop editing.
pub fn tools_panel(ui: &mut egui::Ui, st: &mut BoardEditUi, lang: Lang) -> bool {
    let mut leave = false;
    ui.label(heading(BoardMsg::EditingTitle.tr(lang)));
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                st.state.edit.can_undo(),
                egui::Button::new(EditMsg::Undo.tr(lang)),
            )
            .clicked()
        {
            st.state.undo();
        }
        if ui
            .add_enabled(
                st.state.edit.can_redo(),
                egui::Button::new(EditMsg::Redo.tr(lang)),
            )
            .clicked()
        {
            st.state.redo();
        }
        if ui.button(BoardMsg::StopEditing.tr(lang)).clicked() {
            leave = true;
        }
    });
    let mut tool = st.state.tool;
    ui.horizontal_wrapped(|ui| {
        for (value, text) in [
            (Tool::Select, BoardMsg::ToolSelect),
            (Tool::Place, BoardMsg::ToolPlace),
            (Tool::Route, BoardMsg::ToolRoute),
            (Tool::Via, BoardMsg::ToolVia),
            (Tool::Delete, BoardMsg::ToolDelete),
            (Tool::Tune, BoardMsg::ToolTune),
        ] {
            ui.selectable_value(&mut tool, value, text.tr(lang));
        }
    });
    if tool != st.state.tool {
        st.state.set_tool(tool);
    }

    let layers: Vec<(Id, String)> = copper_layers(st.state.board())
        .into_iter()
        .map(|id| {
            let name = st
                .state
                .board()
                .layers
                .get(&id)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            (id, name)
        })
        .collect();
    let mut layer = st.state.active_layer;
    let layer_name = layers
        .iter()
        .find(|(id, _)| Some(*id) == layer)
        .map(|(_, name)| name.clone())
        .unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(dim(BoardMsg::LayerLabel.tr(lang)));
        egui::ComboBox::from_id_salt("board-layer")
            .selected_text(layer_name)
            .show_ui(ui, |ui| {
                for (id, name) in &layers {
                    ui.selectable_value(&mut layer, Some(*id), name.as_str());
                }
            });
    });
    if layer != st.state.active_layer {
        if let Some(id) = layer {
            st.state.set_active_layer(id);
        }
    }

    match st.state.tool {
        Tool::Place => {
            let footprints: Vec<(Id, String)> = st
                .state
                .board()
                .footprints
                .iter()
                .map(|(id, f)| (*id, f.name.clone()))
                .collect();
            let mut chosen = st.state.place_footprint;
            let chosen_name = footprints
                .iter()
                .find(|(id, _)| Some(*id) == chosen)
                .map(|(_, name)| name.clone())
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(dim(BoardMsg::FootprintLabel.tr(lang)));
                egui::ComboBox::from_id_salt("board-footprint")
                    .selected_text(chosen_name)
                    .show_ui(ui, |ui| {
                        for (id, name) in &footprints {
                            ui.selectable_value(&mut chosen, Some(*id), name.as_str());
                        }
                    });
            });
            st.state.place_footprint = chosen;
            ui.horizontal(|ui| {
                ui.label(dim(BoardMsg::RefLabel.tr(lang)));
                ui.add(egui::TextEdit::singleline(&mut st.state.place_refdes).desired_width(80.0));
                ui.label(dim(BoardMsg::ValueLabel.tr(lang)));
                ui.add(egui::TextEdit::singleline(&mut st.state.place_value).desired_width(80.0));
            });
        }
        Tool::Route => {
            ui.checkbox(&mut st.state.diagonal, BoardMsg::Diagonal.tr(lang));
            ui.checkbox(
                &mut st.state.horizontal_first,
                BoardMsg::CornerFirst.tr(lang),
            );
            if ui.button(BoardMsg::FinishRoute.tr(lang)).clicked() {
                st.state.finish_route();
            }
        }
        Tool::Tune => {
            ui.horizontal(|ui| {
                ui.label(dim(BoardMsg::TargetLength.tr(lang)));
                if ui
                    .add(
                        egui::DragValue::new(&mut st.target_mm)
                            .speed(0.1)
                            .range(0.0..=5000.0)
                            .fixed_decimals(3),
                    )
                    .changed()
                {
                    st.state.tune_target_nm = (st.target_mm * 1_000_000.0).round() as i64;
                }
            });
        }
        _ => {}
    }
    if let Some(error) = &st.state.error {
        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                BoardMsg::Refused.tr(lang),
                error.message(lang)
            ))
            .color(ui_danger()),
        );
    }
    ui.label(dim(BoardMsg::KeysHint.tr(lang)));

    ui.separator();
    ui.label(dim(format!(
        "{} {} · {} {}",
        st.state.selection.len(),
        BoardMsg::SelectedCount.tr(lang),
        st.state.edit.locked().len(),
        BoardMsg::LockedCount.tr(lang)
    )));
    ui.horizontal_wrapped(|ui| {
        if ui.button(BoardMsg::Rotate.tr(lang)).clicked() {
            st.state.rotate_selection(1);
        }
        if ui.button(BoardMsg::Mirror.tr(lang)).clicked() {
            st.state.mirror_selection();
        }
        if ui.button(BoardMsg::LockToggle.tr(lang)).clicked() {
            st.state.toggle_lock_selection();
        }
        if ui.button(BoardMsg::Delete.tr(lang)).clicked() {
            st.state.delete_selection();
        }
    });
    let errors = st
        .state
        .edit
        .findings()
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    ui.label(dim(format!(
        "{errors} {} · {} {}",
        BoardMsg::RuleErrors.tr(lang),
        st.rat.len(),
        BoardMsg::Unrouted.tr(lang)
    )));
    if let Some(length) = st.length_mm {
        ui.label(dim(format!(
            "{}: {length:.3}",
            BoardMsg::NetLength.tr(lang)
        )));
    }
    crate::board_pair_gui::panel(ui, &mut st.pair, &mut st.state, lang);
    crate::board_shove_gui::panel(ui, &mut st.shove, &mut st.state, lang);
    crate::board_autoroute_gui::panel(ui, &mut st.autoroute, &mut st.state, lang);
    crate::board_zone_gui::panel(ui, &mut st.zone, &mut st.state, lang);

    ui.separator();
    ui.horizontal(|ui| {
        ui.label(dim(BoardMsg::GridLabel.tr(lang)));
        let mut grid_mm = mm(st.state.edit.settings.grid_nm);
        if ui
            .add(
                egui::DragValue::new(&mut grid_mm)
                    .speed(0.01)
                    .range(0.01..=25.4)
                    .fixed_decimals(3),
            )
            .changed()
        {
            st.state.edit.settings.grid_nm = (grid_mm * 1_000_000.0).round() as i64;
        }
    });
    ui.checkbox(
        &mut st.state.edit.settings.snap,
        BoardMsg::SnapToGrid.tr(lang),
    );
    ui.checkbox(
        &mut st.state.edit.settings.live_drc,
        BoardMsg::LiveDrc.tr(lang),
    );
    leave
}
