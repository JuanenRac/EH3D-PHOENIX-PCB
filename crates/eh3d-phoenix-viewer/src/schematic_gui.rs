// SPDX-License-Identifier: GPL-3.0-or-later
//! The schematic editor in the window (built with the `gui` feature). It draws the state of
//! [`SchEdit`] with egui and passes on the pointer and the keys; every decision is taken in
//! `eh3d-phoenix-schedit`, in code that is tested. After each change the schematic is handed to
//! the workspace (`Workspace::replace_schematic`) so that the findings, the transfer to the board
//! and the saving work on what was edited. This file has not been seen on a screen.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_schedit::{SchEdit, SchEditSettings, Tool};
use eh3d_phoenix_schematic::{Id, Item, Schematic};

use crate::editmsg::EditMsg;
use crate::schematic_text::SchMsg;
use crate::workspace::Workspace;

// the colours of the theme Forge, as in the window of the viewer
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);
const WIRE: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const PIN: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const LABEL: Color32 = Color32::from_rgb(0x6e, 0xc8, 0xff);
const POWER: Color32 = Color32::from_rgb(0xff, 0x8a, 0x3d);
const GRID: Color32 = Color32::from_rgb(0x40, 0x38, 0x30);

/// The smallest and the largest zoom, in pixels per millimetre.
const SCALE_MIN: f64 = 0.2;
const SCALE_MAX: f64 = 400.0;

/// The largest library file that is read, in bytes.
const MAX_LIBRARY_BYTES: u64 = 64 * 1024 * 1024;

/// What was said about the last library that was loaded.
enum LibraryNote {
    Added(usize),
    Failed,
}

/// The schematic editor of the window: the editor and how it is looked at.
pub struct SchematicEditUi {
    edit: SchEdit,
    center: (f64, f64),
    scale: f64,
    fitted: bool,
    synced: u64,
    search: String,
    library_path: String,
    library_note: Option<LibraryNote>,
    value_text: String,
    refdes_text: String,
    shown_part: Option<Id>,
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

    fn to_world(&self, p: Pos2) -> (f64, f64) {
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
    egui::RichText::new(text).strong().color(ACCENT)
}

fn dim(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(DIM)
}

/// The settings of the editor: `config/schedit.cfg` when it is there and good, else the built-in
/// ones.
fn load_settings() -> SchEditSettings {
    let path = "config/schedit.cfg";
    let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= 1 << 20);
    if !small {
        return SchEditSettings::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| SchEditSettings::parse(&text).ok())
        .unwrap_or_default()
}

/// Starts editing the schematic that is open; `None` when the open document is a board.
pub fn start(workspace: &Workspace) -> Option<SchematicEditUi> {
    let session = workspace.schematic_session()?;
    let edit = SchEdit::new(
        session.schematic.clone(),
        load_settings(),
        session.erc_config.clone(),
    );
    Some(SchematicEditUi {
        edit,
        center: (148.5, 105.0),
        scale: 3.0,
        fitted: false,
        synced: 0,
        search: String::new(),
        library_path: String::new(),
        library_note: None,
        value_text: String::new(),
        refdes_text: String::new(),
        shown_part: None,
    })
}

/// Hands the schematic as it is now to the workspace, when it changed since the last time.
pub fn finish(st: &mut SchematicEditUi, workspace: &mut Workspace) {
    if st.edit.revision() != st.synced {
        st.synced = st.edit.revision();
        workspace.replace_schematic(st.edit.schematic().clone());
    }
}

impl SchematicEditUi {
    fn view(&self, rect: Rect) -> View {
        View {
            origin: rect.center(),
            cx: self.center.0,
            cy: self.center.1,
            scale: self.scale,
        }
    }

    fn fit(&mut self, rect: Rect) {
        let (w, h) = self
            .edit
            .sheet()
            .and_then(|id| self.edit.schematic().sheets.get(&id))
            .map_or((297.0, 210.0), |sheet| {
                (mm(sheet.width.get()), mm(sheet.height.get()))
            });
        self.center = (w / 2.0, h / 2.0);
        let sx = f64::from(rect.width()) / (w * 1.05);
        let sy = f64::from(rect.height()) / (h * 1.05);
        self.scale = sx.min(sy).clamp(SCALE_MIN, SCALE_MAX);
    }

    fn zoom_at(&mut self, rect: Rect, cursor: Pos2, scroll: f32, pinch: f32) {
        let (wx, wy) = self.view(rect).to_world(cursor);
        let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
        self.scale = (self.scale * factor).clamp(SCALE_MIN, SCALE_MAX);
        self.center.0 = wx - f64::from(cursor.x - rect.center().x) / self.scale;
        self.center.1 = wy + f64::from(cursor.y - rect.center().y) / self.scale;
    }

    fn load_library(&mut self) {
        let path = std::path::PathBuf::from(self.library_path.trim());
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("library")
            .to_owned();
        let small = std::fs::metadata(&path).is_ok_and(|m| m.len() <= MAX_LIBRARY_BYTES);
        let text = if small {
            std::fs::read_to_string(&path).ok()
        } else {
            None
        };
        let Some(text) = text else {
            self.library_note = Some(LibraryNote::Failed);
            return;
        };
        let added = match self.edit.palette.add_json(&name, &text) {
            Ok(n) => Some(n),
            Err(_) => Schematic::from_bytes(text.as_bytes())
                .ok()
                .map(|s| self.edit.palette.add_schematic(&name, &s)),
        };
        self.library_note = Some(match added {
            Some(n) => LibraryNote::Added(n),
            None => LibraryNote::Failed,
        });
    }
}

/// The keys the editor answers to, read once per frame.
#[derive(Default)]
struct Keys {
    v: bool,
    w: bool,
    l: bool,
    p: bool,
    b: bool,
    r: bool,
    x: bool,
    z: bool,
    y: bool,
    delete: bool,
    escape: bool,
    space: bool,
    copy: bool,
    cut: bool,
    paste: bool,
    command: bool,
    shift: bool,
}

fn read_keys(ctx: &egui::Context) -> Keys {
    if ctx.egui_wants_keyboard_input() {
        return Keys::default();
    }
    ctx.input(|i| Keys {
        v: i.key_pressed(egui::Key::V),
        w: i.key_pressed(egui::Key::W),
        l: i.key_pressed(egui::Key::L),
        p: i.key_pressed(egui::Key::P),
        b: i.key_pressed(egui::Key::B),
        r: i.key_pressed(egui::Key::R),
        x: i.key_pressed(egui::Key::X),
        z: i.key_pressed(egui::Key::Z),
        y: i.key_pressed(egui::Key::Y),
        delete: i.key_pressed(egui::Key::Delete),
        escape: i.key_pressed(egui::Key::Escape),
        space: i.key_pressed(egui::Key::Space),
        // the toolkit turns Ctrl+C, Ctrl+X and Ctrl+V into events of their own
        copy: i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
        cut: i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
        paste: i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
        command: i.modifiers.command,
        shift: i.modifiers.shift,
    })
}

fn apply_keys(st: &mut SchematicEditUi, keys: &Keys, pointer: Option<Point>) {
    if keys.command {
        if keys.z {
            if keys.shift {
                st.edit.redo();
            } else {
                st.edit.undo();
            }
        } else if keys.y {
            st.edit.redo();
        }
    }
    if keys.copy {
        st.edit.copy_selection();
    }
    if keys.cut {
        let _ = st.edit.cut_selection();
    }
    if keys.paste {
        let _ = st.edit.paste(pointer);
    }
    if keys.command {
        return;
    }
    if keys.escape {
        st.edit.cancel();
    }
    if keys.space {
        st.edit.flip_corner();
    }
    for (pressed, tool) in [
        (keys.v, Tool::Select),
        (keys.w, Tool::Wire),
        (keys.l, Tool::Label),
        (keys.p, Tool::PowerPort),
        (keys.b, Tool::Bus),
    ] {
        if pressed {
            st.edit.set_tool(tool);
        }
    }
    if keys.r {
        let _ = st.edit.rotate_selection(if keys.shift { -1 } else { 1 });
    }
    if keys.x {
        let _ = st.edit.mirror_selection();
    }
    if keys.delete {
        let _ = st.edit.delete_selection();
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

/// The drawing area: pointer, keys, and the drawing of the schematic with what is in progress.
pub fn canvas(
    ui: &mut egui::Ui,
    st: &mut SchematicEditUi,
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
        let (x, y) = view.to_world(p);
        point_at(x, y)
    });
    if let Some(at) = pointer {
        if frame.pressed && inside.is_some() {
            st.edit.pointer_pressed(at, frame.shift);
        } else {
            st.edit.pointer_moved(at);
        }
        if frame.released {
            st.edit.pointer_released(at);
        }
    }
    let keys = read_keys(ui.ctx());
    apply_keys(st, &keys, pointer);
    finish(st, workspace);

    // the drawing: the schematic as the drag in progress would leave it, or as it is
    let preview = st.edit.drag_preview();
    let shown: &Schematic = preview.as_ref().unwrap_or_else(|| st.edit.schematic());
    if let Some(sheet) = st.edit.sheet() {
        draw_schematic(
            &painter,
            &view,
            rect,
            shown,
            sheet,
            &st.edit.selection,
            st.edit.settings.grid_nm,
        );
    }
    if let Some(at) = pointer.filter(|_| inside.is_some()) {
        let path = st.edit.wire_preview(at);
        if path.len() >= 2 {
            let points: Vec<Pos2> = path.iter().map(|p| view.at(*p)).collect();
            painter.add(egui::Shape::line(points, Stroke::new(1.5, ACCENT)));
        }
        let target = st.edit.snap(at);
        painter.circle_stroke(view.at(target), 5.0, Stroke::new(1.0, ACCENT));
        if st.edit.tool == Tool::Place {
            draw_ghost(&painter, &view, st, target);
        }
    }
    if let Some((a, b)) = st.edit.marquee {
        let corners = vec![
            view.xy(mm(a.x_nm.get()), mm(a.y_nm.get())),
            view.xy(mm(b.x_nm.get()), mm(a.y_nm.get())),
            view.xy(mm(b.x_nm.get()), mm(b.y_nm.get())),
            view.xy(mm(a.x_nm.get()), mm(b.y_nm.get())),
        ];
        painter.add(egui::Shape::closed_line(corners, Stroke::new(1.0, ACCENT)));
    }
    let hint = match st.edit.tool {
        Tool::Select => SchMsg::HintSelect,
        Tool::Wire => SchMsg::HintWire,
        Tool::Label | Tool::PowerPort | Tool::Bus => SchMsg::HintLabel,
        Tool::Place => SchMsg::HintPlace,
    };
    painter.text(
        rect.left_bottom() + egui::vec2(8.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        hint.tr(lang),
        egui::FontId::proportional(13.0),
        DIM,
    );
}

/// The pins of the symbol that the place tool would place, at the point where it would go.
fn draw_ghost(painter: &egui::Painter, view: &View, st: &SchematicEditUi, at: Point) {
    let Some(name) = st.edit.place_name.as_deref() else {
        return;
    };
    let Some(entry) = st.edit.palette.find(name) else {
        return;
    };
    for pin in &entry.def.pins {
        let x = at.x_nm.get().saturating_add(pin.at.x_nm.get());
        let y = at.y_nm.get().saturating_add(pin.at.y_nm.get());
        painter.circle_stroke(view.xy(mm(x), mm(y)), 3.0, Stroke::new(1.0, DIM));
    }
}

fn draw_schematic(
    painter: &egui::Painter,
    view: &View,
    rect: Rect,
    s: &Schematic,
    sheet: Id,
    selection: &BTreeSet<Item>,
    grid_nm: i64,
) {
    if let Some(sh) = s.sheets.get(&sheet) {
        let (w, h) = (mm(sh.width.get()), mm(sh.height.get()));
        let corners = vec![
            view.xy(0.0, 0.0),
            view.xy(w, 0.0),
            view.xy(w, h),
            view.xy(0.0, h),
        ];
        painter.add(egui::Shape::closed_line(corners, Stroke::new(1.0, DIM)));
    }
    let step = mm(grid_nm);
    if step > 0.0 && step * view.scale >= 6.0 {
        let (x0, y1) = view.to_world(rect.left_top());
        let (x1, y0) = view.to_world(rect.right_bottom());
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
    let font = (view.scale * 1.8).clamp(0.0, 16.0) as f32;
    let text_shown = font >= 7.0;
    for (id, wire) in s.wires.iter().filter(|(_, w)| w.sheet == sheet) {
        let lit = selection.contains(&Item::Wire(*id));
        let points: Vec<Pos2> = wire.path.iter().map(|p| view.at(*p)).collect();
        let stroke = Stroke::new(if lit { 2.5 } else { 1.5 }, if lit { ACCENT } else { WIRE });
        painter.add(egui::Shape::line(points, stroke));
    }
    for junction in s.junctions.values().filter(|j| j.sheet == sheet) {
        painter.circle_filled(view.at(junction.at), 3.0, WIRE);
    }
    // a part is drawn as the box around its origin and its pins
    let mut boxes: BTreeMap<Id, (f64, f64, f64, f64)> = BTreeMap::new();
    for (id, part) in &s.parts {
        if let Some(place) = part.placement.filter(|p| p.sheet == sheet) {
            let (x, y) = (mm(place.at.x_nm.get()), mm(place.at.y_nm.get()));
            boxes.insert(*id, (x, y, x, y));
        }
    }
    let pins = s.pin_positions();
    for (pin, pin_sheet, p) in &pins {
        if *pin_sheet != sheet {
            continue;
        }
        if let Some(b) = boxes.get_mut(&pin.part) {
            let (x, y) = (mm(p.x_nm.get()), mm(p.y_nm.get()));
            *b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    let pad = 1.27;
    for (id, (x0, y0, x1, y1)) in &boxes {
        let lit = selection.contains(&Item::Part(*id));
        let colour = if lit { ACCENT } else { TEXT };
        let corners = vec![
            view.xy(x0 - pad, y0 - pad),
            view.xy(x1 + pad, y0 - pad),
            view.xy(x1 + pad, y1 + pad),
            view.xy(x0 - pad, y1 + pad),
        ];
        painter.add(egui::Shape::closed_line(
            corners,
            Stroke::new(if lit { 2.5 } else { 1.2 }, colour),
        ));
        if let (true, Some(part)) = (text_shown, s.parts.get(id)) {
            let text = if part.value.is_empty() {
                part.refdes.clone()
            } else {
                format!("{}  {}", part.refdes, part.value)
            };
            painter.text(
                view.xy(x0 - pad, y1 + pad),
                egui::Align2::LEFT_BOTTOM,
                text,
                egui::FontId::proportional(font),
                colour,
            );
        }
    }
    for (pin, pin_sheet, p) in &pins {
        if *pin_sheet != sheet {
            continue;
        }
        let at = view.at(*p);
        painter.circle_filled(at, 2.5, PIN);
        if view.scale >= 8.0 {
            if let Some(number) = s.pin_number(*pin) {
                painter.text(
                    at + egui::vec2(3.0, -3.0),
                    egui::Align2::LEFT_BOTTOM,
                    number,
                    egui::FontId::proportional(10.0),
                    DIM,
                );
            }
        }
    }
    for (id, label) in s.labels.iter().filter(|(_, l)| l.sheet == sheet) {
        let lit = selection.contains(&Item::Label(*id));
        let colour = if lit { ACCENT } else { LABEL };
        let at = view.at(label.at);
        painter.circle_filled(at, 2.5, colour);
        if text_shown {
            painter.text(
                at + egui::vec2(4.0, -2.0),
                egui::Align2::LEFT_BOTTOM,
                &label.name,
                egui::FontId::proportional(font),
                colour,
            );
        }
    }
    for (id, port) in s.power_ports.iter().filter(|(_, p)| p.sheet == sheet) {
        let lit = selection.contains(&Item::PowerPort(*id));
        let colour = if lit { ACCENT } else { POWER };
        let at = view.at(port.at);
        painter.line_segment(
            [at + egui::vec2(-6.0, -8.0), at + egui::vec2(6.0, -8.0)],
            Stroke::new(2.0, colour),
        );
        painter.line_segment([at, at + egui::vec2(0.0, -8.0)], Stroke::new(1.5, colour));
        if text_shown {
            painter.text(
                at + egui::vec2(0.0, -10.0),
                egui::Align2::CENTER_BOTTOM,
                &port.name,
                egui::FontId::proportional(font),
                colour,
            );
        }
    }
}

/// The panel of the tools. Returns true when the person asked to stop editing.
pub fn tools_panel(ui: &mut egui::Ui, st: &mut SchematicEditUi, lang: Lang) -> bool {
    let mut leave = false;
    ui.label(heading(SchMsg::EditingTitle.tr(lang)));
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                st.edit.can_undo(),
                egui::Button::new(EditMsg::Undo.tr(lang)),
            )
            .clicked()
        {
            st.edit.undo();
        }
        if ui
            .add_enabled(
                st.edit.can_redo(),
                egui::Button::new(EditMsg::Redo.tr(lang)),
            )
            .clicked()
        {
            st.edit.redo();
        }
        if ui.button(SchMsg::StopEditing.tr(lang)).clicked() {
            leave = true;
        }
    });
    let mut tool = st.edit.tool;
    ui.horizontal_wrapped(|ui| {
        for (value, text) in [
            (Tool::Select, SchMsg::ToolSelect),
            (Tool::Wire, SchMsg::ToolWire),
            (Tool::Label, SchMsg::ToolLabel),
            (Tool::PowerPort, SchMsg::ToolPower),
            (Tool::Bus, SchMsg::ToolBus),
            (Tool::Place, SchMsg::ToolPlace),
        ] {
            ui.selectable_value(&mut tool, value, text.tr(lang));
        }
    });
    if tool != st.edit.tool {
        st.edit.set_tool(tool);
    }
    if matches!(tool, Tool::Label | Tool::PowerPort | Tool::Bus) {
        ui.horizontal(|ui| {
            ui.label(dim(SchMsg::NameLabel.tr(lang)));
            ui.add(egui::TextEdit::singleline(&mut st.edit.pending_name).desired_width(140.0));
        });
        ui.label(dim(SchMsg::NameHint.tr(lang)));
    }
    if let Some(error) = &st.edit.error {
        ui.label(
            egui::RichText::new(format!("{}: {error}", SchMsg::Refused.tr(lang))).color(DANGER),
        );
    }
    ui.label(dim(SchMsg::KeysHint.tr(lang)));

    ui.separator();
    ui.label(dim(format!(
        "{} {}",
        st.edit.selection.len(),
        SchMsg::SelectedCount.tr(lang)
    )));
    ui.horizontal_wrapped(|ui| {
        if ui.button(SchMsg::Rotate.tr(lang)).clicked() {
            let _ = st.edit.rotate_selection(1);
        }
        if ui.button(SchMsg::Mirror.tr(lang)).clicked() {
            let _ = st.edit.mirror_selection();
        }
        if ui.button(SchMsg::Delete.tr(lang)).clicked() {
            let _ = st.edit.delete_selection();
        }
        if ui.button(SchMsg::Copy.tr(lang)).clicked() {
            st.edit.copy_selection();
        }
        if ui.button(SchMsg::Cut.tr(lang)).clicked() {
            let _ = st.edit.cut_selection();
        }
        if ui.button(SchMsg::Paste.tr(lang)).clicked() {
            let _ = st.edit.paste(None);
        }
    });
    let parts = st.edit.selected_parts();
    if let [part] = parts.as_slice() {
        if st.shown_part != Some(*part) {
            st.shown_part = Some(*part);
            if let Some(p) = st.edit.schematic().parts.get(part) {
                st.value_text = p.value.clone();
                st.refdes_text = p.refdes.clone();
            }
        }
        ui.horizontal(|ui| {
            ui.label(dim(SchMsg::ValueLabel.tr(lang)));
            ui.add(egui::TextEdit::singleline(&mut st.value_text).desired_width(110.0));
            if ui.button(SchMsg::Apply.tr(lang)).clicked() {
                let value = st.value_text.clone();
                let _ = st.edit.set_value(&value);
            }
        });
        ui.horizontal(|ui| {
            ui.label(dim(SchMsg::RefLabel.tr(lang)));
            ui.add(egui::TextEdit::singleline(&mut st.refdes_text).desired_width(110.0));
            if ui.button(SchMsg::Apply.tr(lang)).clicked() {
                let refdes = st.refdes_text.clone();
                let _ = st.edit.set_refdes(&refdes);
            }
        });
    } else {
        st.shown_part = None;
    }

    ui.separator();
    ui.label(heading(SchMsg::PaletteTitle.tr(lang)));
    ui.horizontal(|ui| {
        ui.label(dim(SchMsg::SearchLabel.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut st.search).desired_width(150.0));
    });
    let results: Vec<(String, String)> = st
        .edit
        .palette
        .search(&st.search, st.edit.settings.palette_results)
        .iter()
        .map(|entry| (entry.name().to_owned(), entry.library.clone()))
        .collect();
    if results.is_empty() {
        ui.label(dim(SchMsg::NoSymbols.tr(lang)));
    }
    let mut picked: Option<String> = None;
    egui::ScrollArea::vertical()
        .max_height(180.0)
        .show(ui, |ui| {
            for (name, library) in &results {
                let chosen = st.edit.tool == Tool::Place
                    && st.edit.place_name.as_deref() == Some(name.as_str());
                if ui
                    .selectable_label(chosen, format!("{name}  ({library})"))
                    .clicked()
                {
                    picked = Some(name.clone());
                }
            }
        });
    if let Some(name) = picked {
        st.edit.choose_symbol(&name);
    }
    let mut load = false;
    ui.horizontal(|ui| {
        ui.label(dim(SchMsg::LibraryPath.tr(lang)));
        ui.add(egui::TextEdit::singleline(&mut st.library_path).desired_width(120.0));
        if ui.button(SchMsg::LoadLibrary.tr(lang)).clicked() {
            load = true;
        }
    });
    if load {
        st.load_library();
    }
    match &st.library_note {
        Some(LibraryNote::Added(n)) => {
            ui.label(dim(format!("{n} {}", SchMsg::LibraryAdded.tr(lang))));
        }
        Some(LibraryNote::Failed) => {
            ui.label(egui::RichText::new(SchMsg::LibraryFailed.tr(lang)).color(DANGER));
        }
        None => {}
    }

    ui.separator();
    ui.horizontal(|ui| {
        ui.label(dim(SchMsg::GridLabel.tr(lang)));
        let mut grid_mm = mm(st.edit.settings.grid_nm);
        if ui
            .add(
                egui::DragValue::new(&mut grid_mm)
                    .speed(0.01)
                    .range(0.01..=25.4)
                    .fixed_decimals(3),
            )
            .changed()
        {
            st.edit.settings.grid_nm = (grid_mm * 1_000_000.0).round() as i64;
        }
    });
    ui.checkbox(&mut st.edit.settings.snap, SchMsg::SnapToGrid.tr(lang));
    ui.checkbox(&mut st.edit.settings.live_erc, SchMsg::LiveErc.tr(lang));
    let sheets: Vec<(Id, String)> = st
        .edit
        .schematic()
        .sheets
        .iter()
        .map(|(id, sheet)| (*id, sheet.name.clone()))
        .collect();
    if sheets.len() > 1 {
        let current = st.edit.sheet();
        let mut chosen = current;
        let selected = sheets
            .iter()
            .find(|(id, _)| Some(*id) == current)
            .map(|(_, name)| name.clone())
            .unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(dim(SchMsg::SheetLabel.tr(lang)));
            egui::ComboBox::from_id_salt("schematic-sheet")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for (id, name) in &sheets {
                        ui.selectable_value(&mut chosen, Some(*id), name.as_str());
                    }
                });
        });
        if chosen != current {
            if let Some(id) = chosen {
                st.edit.set_sheet(id);
            }
        }
    }
    leave
}
