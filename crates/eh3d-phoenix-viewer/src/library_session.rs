// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of the library mode of the window, without a screen: the library being edited with
//! its history, the footprint or symbol on the drawing board, the tool, the grid and the snap,
//! the selection and the drag, the measure, the checks and the plan to replace by a master. The
//! window draws [`LibrarySession::shapes`] and passes on what the person does; every edit goes
//! through the reversible operations of `eh3d-phoenix-libedit`, so it can be undone, and the
//! library is saved only as a new file that is read back and compared.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eh3d_phoenix_libedit::codes::{LibError, FILE_UNUSABLE, NOT_FOUND, VALUE_INVALID};
use eh3d_phoenix_libedit::footprints::FootprintConfig;
use eh3d_phoenix_libedit::geom::{self, pad_shape, point_segment, wire_points, Bounds, Shape};
use eh3d_phoenix_libedit::model::{
    Circle, Hole, Item, Kind, Library, Pad, Pin, Smd, Text, Wire, LAYER_NAMES, LAYER_SYMBOLS,
    LAYER_T_NAMES, LAYER_T_PLACE, LAYER_T_VALUES, LAYER_VALUES,
};
use eh3d_phoenix_libedit::ops::{History, OpeningKind, Operation, Target, TargetKind};
use eh3d_phoenix_libedit::{
    apply_step, check_library, generate, read_library, replace_plan_from_json, save_new,
    CheckReport, LibSettings, ReplacePlan,
};
use eh3d_phoenix_rules::Lang;

use crate::library_text::LibText;

/// The prefix of the references of a device set that the window makes.
pub const DEFAULT_PREFIX: &str = "U";

/// The largest plan file that is read, in bytes.
const MAX_PLAN_BYTES: u64 = 16 * 1024 * 1024;

/// The tools of the library mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibTool {
    /// Selects an item and moves it.
    Select,
    /// Places a surface-mount pad.
    Smd,
    /// Places a through-hole pad.
    Pad,
    /// Draws a line with two clicks.
    Line,
    /// Draws an arc with two clicks.
    Arc,
    /// Draws a circle with its centre and a point of it.
    Circle,
    /// Places a text.
    Text,
    /// Places a hole.
    Hole,
    /// Places a pin of a symbol.
    Pin,
    /// Measures a distance with two clicks.
    Measure,
}

impl LibTool {
    /// Every tool, in the order of the palette.
    pub const ALL: [LibTool; 10] = [
        LibTool::Select,
        LibTool::Smd,
        LibTool::Pad,
        LibTool::Hole,
        LibTool::Line,
        LibTool::Arc,
        LibTool::Circle,
        LibTool::Text,
        LibTool::Pin,
        LibTool::Measure,
    ];

    /// The name of the tool in the palette.
    pub fn text(self) -> LibText {
        match self {
            LibTool::Select => LibText::ToolSelect,
            LibTool::Smd => LibText::ToolSmd,
            LibTool::Pad => LibText::ToolPad,
            LibTool::Line => LibText::ToolLine,
            LibTool::Arc => LibText::ToolArc,
            LibTool::Circle => LibText::ToolCircle,
            LibTool::Text => LibText::ToolText,
            LibTool::Hole => LibText::ToolHole,
            LibTool::Pin => LibText::ToolPin,
            LibTool::Measure => LibText::ToolMeasure,
        }
    }

    /// True when the tool needs two clicks.
    pub fn two_clicks(self) -> bool {
        matches!(
            self,
            LibTool::Line | LibTool::Arc | LibTool::Circle | LibTool::Measure
        )
    }

    /// True when the tool makes something, as opposed to selecting or measuring.
    pub fn draws(self) -> bool {
        !matches!(self, LibTool::Select | LibTool::Measure)
    }

    /// True when the tool can draw in a footprint.
    pub fn fits_package(self) -> bool {
        !matches!(self, LibTool::Pin)
    }

    /// True when the tool can draw in a symbol.
    pub fn fits_symbol(self) -> bool {
        !matches!(self, LibTool::Smd | LibTool::Pad | LibTool::Hole)
    }
}

/// The view of the drawing: the centre in millimetres, the scale in points per millimetre and the
/// size of the canvas in points. The y axis points up, as in the files.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibView {
    /// The x of the centre, in millimetres.
    pub cx: f64,
    /// The y of the centre, in millimetres.
    pub cy: f64,
    /// Points per millimetre.
    pub scale: f64,
    /// The width of the canvas, in points.
    pub width: f64,
    /// The height of the canvas, in points.
    pub height: f64,
}

impl Default for LibView {
    fn default() -> Self {
        LibView {
            cx: 0.0,
            cy: 0.0,
            scale: 40.0,
            width: 800.0,
            height: 600.0,
        }
    }
}

/// The smallest and the largest scale, in points per millimetre.
const SCALE_RANGE: (f64, f64) = (0.5, 4000.0);

impl LibView {
    /// The place on the canvas of a point in millimetres.
    pub fn to_screen(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.width / 2.0 + (x - self.cx) * self.scale,
            self.height / 2.0 - (y - self.cy) * self.scale,
        )
    }

    /// The point in millimetres of a place on the canvas.
    pub fn to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        (
            self.cx + (sx - self.width / 2.0) / self.scale,
            self.cy - (sy - self.height / 2.0) / self.scale,
        )
    }

    /// Moves the drawing by a drag of the pointer, in points.
    pub fn pan_px(&mut self, dx: f64, dy: f64) {
        self.cx -= dx / self.scale;
        self.cy += dy / self.scale;
    }

    /// Zooms by the wheel and the pinch, keeping the point under the pointer where it is.
    pub fn zoom_at(&mut self, sx: f64, sy: f64, scroll: f64, pinch: f64) {
        let factor = ((1.0 + scroll * 0.0015) * pinch).clamp(0.5, 2.0);
        let (wx, wy) = self.to_world(sx, sy);
        self.scale = (self.scale * factor).clamp(SCALE_RANGE.0, SCALE_RANGE.1);
        let (nx, ny) = self.to_world(sx, sy);
        self.cx += wx - nx;
        self.cy += wy - ny;
    }

    /// Shows a box, or the origin when there is nothing to show.
    pub fn fit(&mut self, bounds: Option<Bounds>) {
        match bounds {
            None => {
                self.cx = 0.0;
                self.cy = 0.0;
                self.scale = 40.0;
            }
            Some(b) => {
                let w = (b.width() as f64 / 1e6).max(1.0);
                let h = (b.height() as f64 / 1e6).max(1.0);
                let scale = (self.width / (w * 1.4)).min(self.height / (h * 1.4));
                self.scale = scale.clamp(SCALE_RANGE.0, SCALE_RANGE.1);
                self.cx = (b.min_x + b.max_x) as f64 / 2e6;
                self.cy = (b.min_y + b.max_y) as f64 / 2e6;
            }
        }
    }
}

/// A line of the status bar about the last thing that happened.
#[derive(Debug, Clone, PartialEq)]
pub enum LibNote {
    /// A library was opened.
    Opened,
    /// A library was saved; true when a copy of the original was kept.
    Saved(bool),
    /// An edit was made.
    Done,
    /// A step was undone.
    Undone,
    /// A step was redone.
    Redone,
    /// There is nothing to undo.
    NothingUndo,
    /// There is nothing to redo.
    NothingRedo,
    /// The tool waits for the second click.
    SecondPoint,
    /// The tool is not for this drawing.
    WrongKind,
    /// No footprint or symbol is chosen.
    NoEntity,
    /// Nothing is selected.
    NoSelection,
    /// The selected item has no rotation.
    CannotRotate,
    /// The check was made.
    Checked,
    /// A plan was loaded.
    PlanLoaded,
    /// A footprint was generated and added.
    Generated,
    /// An error with its code.
    Error(LibError),
}

impl LibNote {
    /// True for an error.
    pub fn is_error(&self) -> bool {
        matches!(self, LibNote::Error(_))
    }

    /// The text in a language.
    pub fn text(&self, lang: Lang) -> String {
        let plain = |t: LibText| t.tr(lang).to_owned();
        match self {
            LibNote::Opened => plain(LibText::NoteOpened),
            LibNote::Saved(backup) => {
                if *backup {
                    format!(
                        "{} · {}",
                        LibText::NoteSaved.tr(lang),
                        LibText::NoteBackup.tr(lang)
                    )
                } else {
                    plain(LibText::NoteSaved)
                }
            }
            LibNote::Done => plain(LibText::NoteDone),
            LibNote::Undone => plain(LibText::NoteUndone),
            LibNote::Redone => plain(LibText::NoteRedone),
            LibNote::NothingUndo => plain(LibText::NoteNothingUndo),
            LibNote::NothingRedo => plain(LibText::NoteNothingRedo),
            LibNote::SecondPoint => plain(LibText::NoteSecondPoint),
            LibNote::WrongKind => plain(LibText::NoteWrongKind),
            LibNote::NoEntity => plain(LibText::NoteNoEntity),
            LibNote::NoSelection => plain(LibText::NoteNoSelection),
            LibNote::CannotRotate => plain(LibText::NoteCannotRotate),
            LibNote::Checked => plain(LibText::NoteChecked),
            LibNote::PlanLoaded => plain(LibText::NotePlanLoaded),
            LibNote::Generated => plain(LibText::NoteGenerated),
            LibNote::Error(e) => e.message(lang),
        }
    }
}

/// A drag of an item in progress, in nanometres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibDrag {
    /// The place of the item in the list.
    pub index: usize,
    /// Where the pointer was pressed.
    pub start: (i64, i64),
    /// Where the pointer is now.
    pub current: (i64, i64),
}

/// A shape to draw, in millimetres. `index` is the place of the item in the list of the
/// footprint or symbol.
#[derive(Debug, Clone, PartialEq)]
pub enum LibShape {
    /// A straight line.
    Line {
        /// One end.
        a: (f64, f64),
        /// The other end.
        b: (f64, f64),
        /// The width of the line.
        width: f64,
        /// The layer.
        layer: u32,
        /// The item.
        index: usize,
    },
    /// A polyline or a polygon (an arc, a turned rectangle).
    Poly {
        /// The points.
        points: Vec<(f64, f64)>,
        /// True when the last point joins the first.
        closed: bool,
        /// True when the inside is filled.
        filled: bool,
        /// The width of the line.
        width: f64,
        /// The layer.
        layer: u32,
        /// The item.
        index: usize,
    },
    /// A circle or a disc.
    Disc {
        /// The centre.
        c: (f64, f64),
        /// The radius.
        r: f64,
        /// True for a disc.
        filled: bool,
        /// The width of the line of a circle.
        width: f64,
        /// The layer.
        layer: u32,
        /// The item.
        index: usize,
    },
    /// A pad.
    Pad {
        /// The copper, in millimetres.
        shape: Shape,
        /// The radius of the hole, zero for a surface-mount pad.
        hole_r: f64,
        /// The name.
        name: String,
        /// The layer of the copper.
        layer: u32,
        /// The item.
        index: usize,
    },
    /// A text.
    Label {
        /// The anchor.
        at: (f64, f64),
        /// The text.
        text: String,
        /// The height.
        size: f64,
        /// The layer.
        layer: u32,
        /// The item.
        index: usize,
    },
    /// A pin of a symbol.
    Pin {
        /// The connection point.
        from: (f64, f64),
        /// The other end.
        to: (f64, f64),
        /// The name.
        name: String,
        /// The item.
        index: usize,
    },
    /// A hole that is not a pad.
    Hole {
        /// The centre.
        c: (f64, f64),
        /// The radius.
        r: f64,
        /// The item.
        index: usize,
    },
}

impl LibShape {
    /// The place of the item the shape is of.
    pub fn index(&self) -> usize {
        match self {
            LibShape::Line { index, .. }
            | LibShape::Poly { index, .. }
            | LibShape::Disc { index, .. }
            | LibShape::Pad { index, .. }
            | LibShape::Label { index, .. }
            | LibShape::Pin { index, .. }
            | LibShape::Hole { index, .. } => *index,
        }
    }
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1e6
}

fn to_nm(mm: f64) -> i64 {
    (mm * 1e6).round() as i64
}

/// The length of a pin as the file says it, in nanometres.
pub fn pin_length_nm(keyword: &str) -> i64 {
    match keyword {
        "point" => 0,
        "short" => 2_540_000,
        "middle" => 5_080_000,
        _ => 7_620_000,
    }
}

/// The other end of a pin, from its connection point and its rotation.
pub fn pin_end(p: &Pin) -> (f64, f64) {
    let (_, angle) = geom::rotation(&p.rot);
    let (sin, cos) = angle.to_radians().sin_cos();
    let len = pin_length_nm(&p.length) as f64;
    (p.x as f64 + len * cos, p.y as f64 + len * sin)
}

fn shape_in_mm(shape: Shape) -> Shape {
    match shape {
        Shape::Circle { cx, cy, r } => Shape::Circle {
            cx: cx / 1e6,
            cy: cy / 1e6,
            r: r / 1e6,
        },
        Shape::Rect { cx, cy, hw, hh } => Shape::Rect {
            cx: cx / 1e6,
            cy: cy / 1e6,
            hw: hw / 1e6,
            hh: hh / 1e6,
        },
    }
}

/// The distance in nanometres from a point to the copper of a pad.
fn point_shape(px: f64, py: f64, shape: &Shape) -> f64 {
    match *shape {
        Shape::Circle { cx, cy, r } => ((px - cx).hypot(py - cy) - r).max(0.0),
        Shape::Rect { cx, cy, hw, hh } => {
            let dx = ((px - cx).abs() - hw).max(0.0);
            let dy = ((py - cy).abs() - hh).max(0.0);
            dx.hypot(dy)
        }
    }
}

/// The distance in nanometres from a point to an item, `None` for an item without a place.
pub fn item_distance(item: &Item, px: f64, py: f64) -> Option<f64> {
    match item {
        Item::Pad(_) | Item::Smd(_) => pad_shape(item).map(|s| point_shape(px, py, &s)),
        Item::Wire(w) => {
            let d = wire_points(w)
                .windows(2)
                .map(|s| point_segment((px, py), s[0], s[1]))
                .fold(f64::MAX, f64::min);
            Some((d - w.width as f64 / 2.0).max(0.0))
        }
        Item::Rect(r) => {
            let (cx, cy) = ((r.x1 + r.x2) as f64 / 2.0, (r.y1 + r.y2) as f64 / 2.0);
            let (hw, hh) = (
                (r.x2 - r.x1).abs() as f64 / 2.0,
                (r.y2 - r.y1).abs() as f64 / 2.0,
            );
            let (_, angle) = geom::rotation(&r.rot);
            let (sin, cos) = (-angle).to_radians().sin_cos();
            let (lx, ly) = (px - cx, py - cy);
            let (rx, ry) = (lx * cos - ly * sin, lx * sin + ly * cos);
            let dx = (rx.abs() - hw).max(0.0);
            let dy = (ry.abs() - hh).max(0.0);
            Some(dx.hypot(dy))
        }
        Item::Circle(c) => {
            let d = (px - c.x as f64).hypot(py - c.y as f64);
            let r = c.radius as f64;
            if c.width == 0 {
                Some((d - r).max(0.0))
            } else {
                Some(((d - r).abs() - c.width as f64 / 2.0).max(0.0))
            }
        }
        Item::Text(t) => Some((px - t.x as f64).hypot(py - t.y as f64)),
        Item::Hole(h) => {
            Some(((px - h.x as f64).hypot(py - h.y as f64) - h.drill as f64 / 2.0).max(0.0))
        }
        Item::Pin(p) => {
            let end = pin_end(p);
            Some(point_segment((px, py), (p.x as f64, p.y as f64), end))
        }
        Item::Other(_) => None,
    }
}

/// The rotation a quarter of a turn after `rot`, written as the file does.
pub fn turned(rot: &str) -> String {
    let (mirror, angle) = geom::rotation(rot);
    let next = (angle + 90.0).rem_euclid(360.0);
    format!("{}R{}", if mirror { "M" } else { "" }, next)
}

/// The state of the library mode.
pub struct LibrarySession {
    /// The file the library came from, or the last file it was saved to.
    pub path: Option<PathBuf>,
    history: History,
    /// The settings of the editor.
    pub settings: LibSettings,
    /// What the list shows: footprints, symbols or device sets.
    pub kind: Kind,
    /// The footprint, symbol or device set on the drawing board.
    pub name: Option<String>,
    /// The item selected, as its place in the list of the footprint or symbol.
    pub selected: Option<usize>,
    /// The tool.
    pub tool: LibTool,
    /// True when the points snap to the grid.
    pub snap: bool,
    /// The grid in nanometres.
    pub grid_nm: i64,
    /// The first click of a tool that needs two.
    pub first_click: Option<(i64, i64)>,
    /// The drag in progress.
    pub drag: Option<LibDrag>,
    /// The two points of the measure.
    pub measure: [Option<(i64, i64)>; 2],
    /// Where the pointer is, in nanometres.
    pub cursor: Option<(i64, i64)>,
    /// The view of the drawing.
    pub view: LibView,
    /// True when the view has to be fitted to the drawing at the next frame.
    pub fit_pending: bool,
    /// The last thing that happened.
    pub note: Option<LibNote>,
    /// The result of the last check.
    pub report: CheckReport,
    /// The steps of a plan of the sanitizer for this library.
    pub plan: Option<ReplacePlan>,
}

impl LibrarySession {
    /// A session on a library.
    pub fn new(library: Library, path: Option<PathBuf>, settings: LibSettings) -> LibrarySession {
        let report = check_library(&library, &settings);
        LibrarySession {
            path,
            history: History::new(library, settings.history_limit),
            kind: Kind::Package,
            name: None,
            selected: None,
            tool: LibTool::Select,
            snap: settings.snap,
            grid_nm: settings.grid_nm,
            first_click: None,
            drag: None,
            measure: [None, None],
            cursor: None,
            view: LibView::default(),
            fit_pending: true,
            note: None,
            report,
            plan: None,
            settings,
        }
    }

    /// Reads a library file and starts a session on it.
    pub fn open(path: &Path, settings: LibSettings) -> Result<LibrarySession, LibError> {
        let unusable = |e: String| LibError::new(FILE_UNUSABLE, format!("{}: {e}", path.display()));
        let length = std::fs::metadata(path)
            .map_err(|e| unusable(e.to_string()))?
            .len();
        if length > settings.max_file_bytes {
            return Err(unusable(format!(
                "larger than {} bytes",
                settings.max_file_bytes
            )));
        }
        let bytes = std::fs::read(path).map_err(|e| unusable(e.to_string()))?;
        let library = read_library(&bytes, &settings.limits())
            .map_err(|e| LibError::new(e.code, format!("{}: {}", path.display(), e.detail)))?;
        let mut session = LibrarySession::new(library, Some(path.to_path_buf()), settings);
        session.note = Some(LibNote::Opened);
        Ok(session)
    }

    /// A session on a new library with nothing in it.
    pub fn empty(settings: LibSettings) -> LibrarySession {
        LibrarySession::new(Library::empty(), None, settings)
    }

    /// The library as it is now.
    pub fn library(&self) -> &Library {
        self.history.library()
    }

    /// True when the library has changed since it was read or saved.
    pub fn dirty(&self) -> bool {
        self.history.dirty()
    }

    /// True when there is a step to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// True when there is a step to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// The names of the footprints, symbols or device sets.
    pub fn names(&self, kind: Kind) -> Vec<String> {
        self.library().names(kind)
    }

    /// Puts a footprint, symbol or device set on the drawing board.
    pub fn select_entity(&mut self, kind: Kind, name: Option<String>) {
        if kind != self.kind {
            self.grid_nm = if kind == Kind::Symbol {
                self.settings.symbol_grid_nm
            } else {
                self.settings.grid_nm
            };
        }
        self.kind = kind;
        self.name = name;
        self.selected = None;
        self.first_click = None;
        self.drag = None;
        self.fit_pending = true;
    }

    /// The footprint or symbol on the drawing board as a target of the operations.
    pub fn target(&self) -> Option<Target> {
        let name = self.name.clone()?;
        match self.kind {
            Kind::Package => Some(Target {
                kind: TargetKind::Package,
                name,
            }),
            Kind::Symbol => Some(Target {
                kind: TargetKind::Symbol,
                name,
            }),
            Kind::DeviceSet => None,
        }
    }

    /// The items of the footprint or symbol on the drawing board.
    pub fn items(&self) -> &[Item] {
        let Some(name) = &self.name else {
            return &[];
        };
        match self.kind {
            Kind::Package => self
                .library()
                .package(name)
                .map_or(&[][..], |p| p.items.as_slice()),
            Kind::Symbol => self
                .library()
                .symbol(name)
                .map_or(&[][..], |s| s.items.as_slice()),
            Kind::DeviceSet => &[],
        }
    }

    /// The item selected.
    pub fn selected_item(&self) -> Option<&Item> {
        self.selected.and_then(|i| self.items().get(i))
    }

    /// A point rounded to the grid when the snap is on.
    pub fn snapped(&self, x: i64, y: i64) -> (i64, i64) {
        if !self.snap || self.grid_nm <= 0 {
            return (x, y);
        }
        let g = self.grid_nm as f64;
        (
            ((x as f64 / g).round() as i64) * self.grid_nm,
            ((y as f64 / g).round() as i64) * self.grid_nm,
        )
    }

    /// The point of the pointer on the drawing, in nanometres, from a place on the canvas.
    pub fn world(&self, sx: f64, sy: f64) -> (i64, i64) {
        let (x, y) = self.view.to_world(sx, sy);
        (to_nm(x), to_nm(y))
    }

    /// The distance in nanometres that a click picks an item from, at the scale of the view.
    pub fn pick_tolerance_nm(&self) -> i64 {
        let px = f64::from(self.settings.pick_px);
        to_nm(px / self.view.scale)
    }

    /// The item nearest to a point, within the tolerance; the later one when two are as near.
    pub fn pick(&self, x: i64, y: i64, tolerance_nm: i64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, item) in self.items().iter().enumerate() {
            if let Some(d) = item_distance(item, x as f64, y as f64) {
                if d <= tolerance_nm as f64 && best.is_none_or(|(_, b)| d <= b) {
                    best = Some((i, d));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Applies operations all or nothing, keeps the note and checks the library again when it
    /// is small enough.
    pub fn apply(&mut self, ops: Vec<Operation>) -> bool {
        match self.history.apply(&ops) {
            Ok(()) => {
                self.note = Some(LibNote::Done);
                self.after_edit();
                true
            }
            Err((_, e)) => {
                self.note = Some(LibNote::Error(e));
                false
            }
        }
    }

    fn total_items(&self) -> usize {
        let lib = self.library();
        lib.packages.len() + lib.symbols.len() + lib.devicesets.len()
    }

    fn after_edit(&mut self) {
        self.fix_selection();
        if self.total_items() <= self.settings.auto_check_max_items {
            self.report = check_library(self.history.library(), &self.settings);
        }
    }

    /// Checks the library and keeps the report.
    pub fn check(&mut self) {
        self.report = check_library(self.history.library(), &self.settings);
        self.note = Some(LibNote::Checked);
    }

    fn fix_selection(&mut self) {
        let exists = match (&self.name, self.kind) {
            (Some(n), Kind::Package) => self.library().package(n).is_some(),
            (Some(n), Kind::Symbol) => self.library().symbol(n).is_some(),
            (Some(n), Kind::DeviceSet) => self.library().deviceset(n).is_some(),
            (None, _) => true,
        };
        if !exists {
            self.name = None;
        }
        let len = self.items().len();
        if self.selected.is_some_and(|i| i >= len) {
            self.selected = None;
        }
        self.first_click = None;
        self.drag = None;
    }

    /// Takes back the last step.
    pub fn undo(&mut self) {
        if self.history.undo() {
            self.note = Some(LibNote::Undone);
            self.after_edit();
        } else {
            self.note = Some(LibNote::NothingUndo);
        }
    }

    /// Does again the last step that was taken back.
    pub fn redo(&mut self) {
        if self.history.redo() {
            self.note = Some(LibNote::Redone);
            self.after_edit();
        } else {
            self.note = Some(LibNote::NothingRedo);
        }
    }

    /// Makes a footprint, symbol or device set and puts it on the drawing board.
    pub fn create_entity(&mut self, kind: Kind, name: &str) -> bool {
        let name = name.trim().to_owned();
        let op = match kind {
            Kind::Package => Operation::CreatePackage { name: name.clone() },
            Kind::Symbol => Operation::CreateSymbol { name: name.clone() },
            Kind::DeviceSet => Operation::CreateDeviceSet {
                name: name.clone(),
                prefix: DEFAULT_PREFIX.to_owned(),
            },
        };
        let ok = self.apply(vec![op]);
        if ok {
            self.select_entity(kind, Some(name));
        }
        ok
    }

    /// Renames the footprint, symbol or device set on the drawing board.
    pub fn rename_entity(&mut self, to: &str) -> bool {
        let Some(from) = self.name.clone() else {
            self.note = Some(LibNote::NoEntity);
            return false;
        };
        let to = to.trim().to_owned();
        let op = match self.kind {
            Kind::Package => Operation::RenamePackage {
                from,
                to: to.clone(),
            },
            Kind::Symbol => Operation::RenameSymbol {
                from,
                to: to.clone(),
            },
            Kind::DeviceSet => Operation::RenameDeviceSet {
                from,
                to: to.clone(),
            },
        };
        let ok = self.apply(vec![op]);
        if ok {
            self.name = Some(to);
        }
        ok
    }

    /// Deletes the footprint, symbol or device set on the drawing board.
    pub fn delete_entity(&mut self) -> bool {
        let Some(name) = self.name.clone() else {
            self.note = Some(LibNote::NoEntity);
            return false;
        };
        let op = match self.kind {
            Kind::Package => Operation::DeletePackage { name },
            Kind::Symbol => Operation::DeleteSymbol { name },
            Kind::DeviceSet => Operation::DeleteDeviceSet { name },
        };
        let ok = self.apply(vec![op]);
        if ok {
            self.name = None;
            self.selected = None;
        }
        ok
    }

    fn lowest_free(&self, taken: impl Fn(&str) -> bool) -> String {
        (1u32..)
            .map(|n| n.to_string())
            .find(|name| !taken(name))
            .unwrap_or_default()
    }

    fn has_text(&self, content: &str) -> bool {
        self.items().iter().any(|i| match i {
            Item::Text(t) => t.content.eq_ignore_ascii_case(content),
            _ => false,
        })
    }

    /// The layers a tool draws on in the footprint or symbol on the drawing board: the line
    /// work, the name and the value.
    fn layers(&self) -> (u32, u32, u32) {
        if self.kind == Kind::Symbol {
            (LAYER_SYMBOLS, LAYER_NAMES, LAYER_VALUES)
        } else {
            (LAYER_T_PLACE, LAYER_T_NAMES, LAYER_T_VALUES)
        }
    }

    /// The item a tool makes at a point (the second point for the tools with two clicks).
    fn new_item(&self, tool: LibTool, p: (i64, i64), q: Option<(i64, i64)>) -> Option<Item> {
        let s = &self.settings;
        let (line_layer, name_layer, value_layer) = self.layers();
        let width = if self.kind == Kind::Symbol {
            s.new_symbol_line_width_nm
        } else {
            s.new_line_width_nm
        };
        Some(match tool {
            LibTool::Smd => Item::Smd(Smd {
                name: self.lowest_free(|n| {
                    self.items()
                        .iter()
                        .any(|i| i.is_pad() && i.name() == Some(n))
                }),
                x: p.0,
                y: p.1,
                dx: s.new_smd_dx_nm,
                dy: s.new_smd_dy_nm,
                ..Smd::default()
            }),
            LibTool::Pad => Item::Pad(Pad {
                name: self.lowest_free(|n| {
                    self.items()
                        .iter()
                        .any(|i| i.is_pad() && i.name() == Some(n))
                }),
                x: p.0,
                y: p.1,
                drill: s.new_pad_drill_nm,
                diameter: s.new_pad_diameter_nm,
                ..Pad::default()
            }),
            LibTool::Hole => Item::Hole(Hole {
                x: p.0,
                y: p.1,
                drill: s.new_pad_drill_nm,
                extra: Vec::new(),
            }),
            LibTool::Text => {
                let (content, layer) = if !self.has_text(">NAME") {
                    (">NAME", name_layer)
                } else if !self.has_text(">VALUE") {
                    (">VALUE", value_layer)
                } else {
                    ("TEXT", line_layer)
                };
                Item::Text(Text {
                    layer,
                    x: p.0,
                    y: p.1,
                    size: s.new_text_size_nm,
                    content: content.to_owned(),
                    ..Text::default()
                })
            }
            LibTool::Pin => Item::Pin(Pin {
                name: self.lowest_free(|n| {
                    self.items()
                        .iter()
                        .any(|i| i.name() == Some(n) && matches!(i, Item::Pin(_)))
                }),
                x: p.0,
                y: p.1,
                length: s.new_pin_length.clone(),
                direction: "pas".to_owned(),
                ..Pin::default()
            }),
            LibTool::Line | LibTool::Arc => {
                let q = q?;
                if q == p {
                    return None;
                }
                Item::Wire(Wire {
                    layer: line_layer,
                    x1: p.0,
                    y1: p.1,
                    x2: q.0,
                    y2: q.1,
                    width,
                    curve: if tool == LibTool::Arc { 90_000_000 } else { 0 },
                    extra: Vec::new(),
                })
            }
            LibTool::Circle => {
                let q = q?;
                let radius = ((q.0 - p.0) as f64).hypot((q.1 - p.1) as f64).round() as i64;
                if radius <= 0 {
                    return None;
                }
                Item::Circle(Circle {
                    layer: line_layer,
                    x: p.0,
                    y: p.1,
                    radius,
                    width,
                    extra: Vec::new(),
                })
            }
            LibTool::Select | LibTool::Measure => return None,
        })
    }

    /// Adds an item at the end of the footprint or symbol on the drawing board and selects it.
    pub fn add_item(&mut self, item: Item) -> bool {
        let Some(target) = self.target() else {
            self.note = Some(LibNote::NoEntity);
            return false;
        };
        let ok = self.apply(vec![Operation::AddItem {
            target,
            item,
            at: None,
        }]);
        if ok {
            self.selected = self.items().len().checked_sub(1);
        }
        ok
    }

    /// A click on the drawing with the tool in hand, at a point of the drawing in nanometres.
    pub fn click(&mut self, x: i64, y: i64) {
        let tool = self.tool;
        let p = self.snapped(x, y);
        match tool {
            LibTool::Select => {
                let tolerance = self.pick_tolerance_nm();
                self.selected = self.pick(x, y, tolerance);
            }
            LibTool::Measure => {
                if self.measure[0].is_none() || self.measure[1].is_some() {
                    self.measure = [Some(p), None];
                } else {
                    self.measure[1] = Some(p);
                }
            }
            _ => {
                if self.target().is_none() {
                    self.note = Some(LibNote::NoEntity);
                    return;
                }
                let fits = if self.kind == Kind::Symbol {
                    tool.fits_symbol()
                } else {
                    tool.fits_package()
                };
                if !fits {
                    self.note = Some(LibNote::WrongKind);
                    return;
                }
                if tool.two_clicks() {
                    match self.first_click.take() {
                        None => {
                            self.first_click = Some(p);
                            self.note = Some(LibNote::SecondPoint);
                        }
                        Some(first) => {
                            if let Some(item) = self.new_item(tool, first, Some(p)) {
                                self.add_item(item);
                            }
                        }
                    }
                } else if let Some(item) = self.new_item(tool, p, None) {
                    self.add_item(item);
                }
            }
        }
    }

    /// Starts the drag of the item under the pointer; false when there is none (the drawing is
    /// panned instead).
    pub fn press(&mut self, x: i64, y: i64) -> bool {
        if self.tool != LibTool::Select {
            return false;
        }
        let tolerance = self.pick_tolerance_nm();
        match self.pick(x, y, tolerance) {
            Some(index) => {
                self.selected = Some(index);
                self.drag = Some(LibDrag {
                    index,
                    start: (x, y),
                    current: (x, y),
                });
                true
            }
            None => false,
        }
    }

    /// The pointer moves while an item is dragged.
    pub fn drag_to(&mut self, x: i64, y: i64) {
        if let Some(d) = self.drag.as_mut() {
            d.current = (x, y);
        }
    }

    /// The shift of the item being dragged, rounded to the grid.
    pub fn drag_delta(&self) -> Option<(i64, i64)> {
        let d = self.drag?;
        let dx = d.current.0 - d.start.0;
        let dy = d.current.1 - d.start.1;
        if !self.snap || self.grid_nm <= 0 {
            return Some((dx, dy));
        }
        let g = self.grid_nm as f64;
        Some((
            ((dx as f64 / g).round() as i64) * self.grid_nm,
            ((dy as f64 / g).round() as i64) * self.grid_nm,
        ))
    }

    /// The drag ends: the item moves by the shift, as one step.
    pub fn release(&mut self, x: i64, y: i64) {
        self.drag_to(x, y);
        let delta = self.drag_delta();
        let drag = self.drag.take();
        if let (Some(d), Some((dx, dy)), Some(target)) = (drag, delta, self.target()) {
            if dx != 0 || dy != 0 {
                self.apply(vec![Operation::MoveItem {
                    target,
                    index: d.index,
                    dx,
                    dy,
                }]);
            }
        }
    }

    /// Cancels what is in progress: the first click, the drag and the measure.
    pub fn cancel(&mut self) {
        self.first_click = None;
        self.drag = None;
        self.measure = [None, None];
    }

    /// Deletes the item selected.
    pub fn delete_selected(&mut self) -> bool {
        let (Some(target), Some(index)) = (self.target(), self.selected) else {
            self.note = Some(LibNote::NoSelection);
            return false;
        };
        let ok = self.apply(vec![Operation::RemoveItem { target, index }]);
        if ok {
            self.selected = None;
        }
        ok
    }

    /// Replaces the item selected by another (the way a property is edited).
    pub fn replace_selected(&mut self, item: Item) -> bool {
        let (Some(target), Some(index)) = (self.target(), self.selected) else {
            self.note = Some(LibNote::NoSelection);
            return false;
        };
        self.apply(vec![Operation::ReplaceItem {
            target,
            index,
            item,
        }])
    }

    /// Turns the item selected by a quarter of a turn.
    pub fn rotate_selected(&mut self) -> bool {
        let Some(item) = self.selected_item().cloned() else {
            self.note = Some(LibNote::NoSelection);
            return false;
        };
        let mut item = item;
        match &mut item {
            Item::Pad(p) => p.rot = turned(&p.rot),
            Item::Smd(p) => p.rot = turned(&p.rot),
            Item::Rect(p) => p.rot = turned(&p.rot),
            Item::Text(p) => p.rot = turned(&p.rot),
            Item::Pin(p) => p.rot = turned(&p.rot),
            _ => {
                self.note = Some(LibNote::CannotRotate);
                return false;
            }
        }
        self.replace_selected(item)
    }

    /// Moves the origin of the footprint or symbol to a point (every item moves the other way).
    pub fn origin_to(&mut self, x: i64, y: i64) -> bool {
        let Some(target) = self.target() else {
            self.note = Some(LibNote::NoEntity);
            return false;
        };
        let (x, y) = self.snapped(x, y);
        self.apply(vec![Operation::ShiftOrigin {
            target,
            dx: x,
            dy: y,
        }])
    }

    /// Sets (or, without an expansion, takes away) the opening of the mask or the paste of the
    /// pad selected.
    pub fn set_opening(&mut self, kind: OpeningKind, expansion: Option<i64>) -> bool {
        let (Some(name), Some(item)) = (self.name.clone(), self.selected_item()) else {
            self.note = Some(LibNote::NoSelection);
            return false;
        };
        let Some(pad) = item.name().filter(|_| item.is_pad()).map(str::to_owned) else {
            self.note = Some(LibNote::Error(LibError::new(
                NOT_FOUND,
                "the item is not a pad",
            )));
            return false;
        };
        if self.kind != Kind::Package {
            self.note = Some(LibNote::WrongKind);
            return false;
        }
        self.apply(vec![Operation::SetOpening {
            package: name,
            pad,
            kind,
            expansion,
        }])
    }

    /// Makes a footprint with a generator and adds it to the library.
    pub fn add_generated(
        &mut self,
        family: &str,
        params: &BTreeMap<String, String>,
        cfg: &FootprintConfig,
    ) -> bool {
        let made = match generate(family, params, cfg) {
            Ok(m) => m,
            Err(e) => {
                self.note = Some(LibNote::Error(e));
                return false;
            }
        };
        let name = made.package.name.clone();
        let mut ops = vec![Operation::CreatePackage { name: name.clone() }];
        for item in made.package.items {
            ops.push(Operation::AddItem {
                target: Target {
                    kind: TargetKind::Package,
                    name: name.clone(),
                },
                item,
                at: None,
            });
        }
        let ok = self.apply(ops);
        if ok {
            self.select_entity(Kind::Package, Some(name));
            self.note = Some(LibNote::Generated);
        }
        ok
    }

    /// Reads a plan of the sanitizer and keeps the steps for this library.
    pub fn load_plan(&mut self, plan_path: &Path) -> bool {
        let fail =
            |e: String| LibError::new(FILE_UNUSABLE, format!("{}: {e}", plan_path.display()));
        let Some(file) = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
        else {
            self.note = Some(LibNote::Error(LibError::new(
                VALUE_INVALID,
                "the library has no file name yet",
            )));
            return false;
        };
        let loaded = std::fs::metadata(plan_path)
            .map_err(|e| fail(e.to_string()))
            .and_then(|m| {
                if m.len() > MAX_PLAN_BYTES {
                    Err(fail("the file is too large".to_owned()))
                } else {
                    std::fs::read_to_string(plan_path).map_err(|e| fail(e.to_string()))
                }
            })
            .and_then(|text| replace_plan_from_json(&text, &file));
        match loaded {
            Ok(plan) => {
                self.plan = Some(plan);
                self.note = Some(LibNote::PlanLoaded);
                true
            }
            Err(e) => {
                self.note = Some(LibNote::Error(e));
                false
            }
        }
    }

    /// Replaces the item of a step of the plan by the master item, as one step that can be
    /// undone. Nothing is done unless `confirmed` is true.
    pub fn replace_with_master(&mut self, step: usize, folder: &Path, confirmed: bool) -> bool {
        let Some(step) = self.plan.as_ref().and_then(|p| p.steps.get(step)).cloned() else {
            self.note = Some(LibNote::Error(LibError::new(
                NOT_FOUND,
                "the step of the plan",
            )));
            return false;
        };
        let master_path = folder.join(&step.master_library);
        let master = match LibrarySession::open(&master_path, self.settings.clone()) {
            Ok(s) => s.library().clone(),
            Err(e) => {
                self.note = Some(LibNote::Error(e));
                return false;
            }
        };
        let mut library = self.library().clone();
        match apply_step(&mut library, &step, &master, confirmed) {
            Ok(()) => {
                self.history.replace(
                    library,
                    &format!("replace {} by {}", step.name, step.master_name),
                );
                self.note = Some(LibNote::Done);
                self.after_edit();
                true
            }
            Err(e) => {
                self.note = Some(LibNote::Error(e));
                false
            }
        }
    }

    /// Saves the library as a new file that is read back and compared; the file that the
    /// library came from is never written over and a copy of it is kept next to the new file.
    pub fn save_as(&mut self, target: &Path) -> bool {
        match save_new(self.history.library(), self.path.as_deref(), target) {
            Ok(saved) => {
                self.history.mark_saved();
                self.note = Some(LibNote::Saved(saved.backup.is_some()));
                self.path = Some(saved.path);
                true
            }
            Err(e) => {
                self.note = Some(LibNote::Error(e));
                false
            }
        }
    }

    /// The distance of the measure in millimetres, when both points are set.
    pub fn measure_distance(&self) -> Option<f64> {
        let (a, b) = (self.measure[0]?, self.measure[1]?);
        Some(((b.0 - a.0) as f64).hypot((b.1 - a.1) as f64) / 1e6)
    }

    /// The box of the footprint or symbol on the drawing board.
    pub fn bounds(&self) -> Option<Bounds> {
        geom::items_bounds(self.items())
    }

    /// The shapes to draw for the footprint or symbol on the drawing board; the item being
    /// dragged is drawn where it would go.
    pub fn shapes(&self) -> Vec<LibShape> {
        let delta = self.drag_delta();
        let moving = self.drag.map(|d| d.index);
        let mut out = Vec::new();
        for (index, original) in self.items().iter().enumerate() {
            let mut item = original.clone();
            if let (Some((dx, dy)), Some(m)) = (delta, moving) {
                if m == index {
                    let _ = eh3d_phoenix_libedit::ops::translate(&mut item, dx, dy);
                }
            }
            if let Some(shape) = shape_of(&item, index) {
                out.push(shape);
            }
        }
        out
    }
}

/// The shape to draw for an item; `None` for an item that the model keeps as it came.
pub fn shape_of(item: &Item, index: usize) -> Option<LibShape> {
    Some(match item {
        Item::Pad(p) => LibShape::Pad {
            shape: shape_in_mm(pad_shape(item)?),
            hole_r: mm(p.drill) / 2.0,
            name: p.name.clone(),
            layer: 1,
            index,
        },
        Item::Smd(p) => LibShape::Pad {
            shape: shape_in_mm(pad_shape(item)?),
            hole_r: 0.0,
            name: p.name.clone(),
            layer: p.layer,
            index,
        },
        Item::Wire(w) => {
            let points: Vec<(f64, f64)> = wire_points(w)
                .into_iter()
                .map(|(x, y)| (x / 1e6, y / 1e6))
                .collect();
            if points.len() == 2 {
                LibShape::Line {
                    a: points[0],
                    b: points[1],
                    width: mm(w.width),
                    layer: w.layer,
                    index,
                }
            } else {
                LibShape::Poly {
                    points,
                    closed: false,
                    filled: false,
                    width: mm(w.width),
                    layer: w.layer,
                    index,
                }
            }
        }
        Item::Rect(r) => {
            let (cx, cy) = ((r.x1 + r.x2) as f64 / 2e6, (r.y1 + r.y2) as f64 / 2e6);
            let (hw, hh) = (
                (r.x2 - r.x1).abs() as f64 / 2e6,
                (r.y2 - r.y1).abs() as f64 / 2e6,
            );
            let (_, angle) = geom::rotation(&r.rot);
            let (sin, cos) = angle.to_radians().sin_cos();
            let points = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
                .iter()
                .map(|(x, y)| (cx + x * cos - y * sin, cy + x * sin + y * cos))
                .collect();
            LibShape::Poly {
                points,
                closed: true,
                filled: true,
                width: 0.0,
                layer: r.layer,
                index,
            }
        }
        Item::Circle(c) => LibShape::Disc {
            c: (mm(c.x), mm(c.y)),
            r: mm(c.radius),
            filled: c.width == 0,
            width: mm(c.width),
            layer: c.layer,
            index,
        },
        Item::Text(t) => LibShape::Label {
            at: (mm(t.x), mm(t.y)),
            text: t.content.clone(),
            size: mm(t.size),
            layer: t.layer,
            index,
        },
        Item::Hole(h) => LibShape::Hole {
            c: (mm(h.x), mm(h.y)),
            r: mm(h.drill) / 2.0,
            index,
        },
        Item::Pin(p) => {
            let end = pin_end(p);
            LibShape::Pin {
                from: (mm(p.x), mm(p.y)),
                to: (end.0 / 1e6, end.1 / 1e6),
                name: p.name.clone(),
                index,
            }
        }
        Item::Other(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_libedit::codes::{IN_USE, NOT_CONFIRMED};
    use std::fs;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2">
<drawing>
<settings/>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="21" name="tPlace" color="7" fill="1" visible="yes" active="yes"/>
</layers>
<library>
<packages>
<package name="R0603">
<smd name="1" x="-0.8" y="0" dx="0.8" dy="0.9" layer="1"/>
<smd name="2" x="0.8" y="0" dx="0.8" dy="0.9" layer="1"/>
<text x="0" y="1.2" size="1" layer="25">&gt;NAME</text>
<text x="0" y="-1.2" size="1" layer="27">&gt;VALUE</text>
</package>
</packages>
<symbols>
<symbol name="R">
<pin name="1" x="-5.08" y="0" length="short" direction="pas"/>
<pin name="2" x="5.08" y="0" length="short" direction="pas" rot="R180"/>
</symbol>
</symbols>
<devicesets>
<deviceset name="R" prefix="R">
<gates>
<gate name="G$1" symbol="R" x="0" y="0"/>
</gates>
<devices>
<device name="0603" package="R0603">
<connects>
<connect gate="G$1" pin="1" pad="1"/>
<connect gate="G$1" pin="2" pad="2"/>
</connects>
</device>
</devices>
</deviceset>
</devicesets>
</library>
</drawing>
</eagle>
"#;

    fn session() -> LibrarySession {
        let library = read_library(SAMPLE.as_bytes(), &LibSettings::default().limits()).unwrap();
        let mut s = LibrarySession::new(library, None, LibSettings::default());
        s.select_entity(Kind::Package, Some("R0603".to_owned()));
        s
    }

    fn pad_xy(s: &LibrarySession, index: usize) -> (i64, i64) {
        match &s.items()[index] {
            Item::Smd(p) => (p.x, p.y),
            Item::Pad(p) => (p.x, p.y),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_library_file_is_opened_and_its_names_listed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("in.lbr");
        fs::write(&path, SAMPLE).unwrap();
        let s = LibrarySession::open(&path, LibSettings::default()).unwrap();
        assert_eq!(s.names(Kind::Package), ["R0603"]);
        assert_eq!(s.names(Kind::Symbol), ["R"]);
        assert_eq!(s.names(Kind::DeviceSet), ["R"]);
        assert_eq!(s.note, Some(LibNote::Opened));
        assert!(!s.dirty());
        assert!(s.report.findings.is_empty());
        let e = LibrarySession::open(&dir.path().join("none.lbr"), LibSettings::default())
            .err()
            .unwrap();
        assert_eq!(e.code, FILE_UNUSABLE);
    }

    #[test]
    fn the_pad_tool_places_a_pad_on_the_grid_with_the_next_free_name() {
        let mut s = session();
        s.tool = LibTool::Smd;
        s.click(1_234_000, 567_000);
        assert_eq!(s.items().len(), 5);
        assert_eq!(s.selected, Some(4));
        match &s.items()[4] {
            Item::Smd(p) => {
                assert_eq!(p.name, "3");
                assert_eq!((p.x, p.y), (1_250_000, 550_000));
                assert_eq!((p.dx, p.dy), (800_000, 800_000));
            }
            other => panic!("{other:?}"),
        }
        assert!(s.dirty());
        s.snap = false;
        s.click(1_234_000, 567_000);
        assert_eq!(pad_xy(&s, 5), (1_234_000, 567_000));
        s.tool = LibTool::Pad;
        s.click(0, 0);
        match &s.items()[6] {
            Item::Pad(p) => assert_eq!(
                (p.name.as_str(), p.drill, p.diameter),
                ("5", 800_000, 1_600_000)
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_line_and_the_arc_take_two_clicks_and_the_circle_takes_its_radius() {
        let mut s = session();
        s.tool = LibTool::Line;
        s.click(0, 0);
        assert_eq!(s.note, Some(LibNote::SecondPoint));
        assert_eq!(s.items().len(), 4);
        s.click(1_000_000, 0);
        match &s.items()[4] {
            Item::Wire(w) => {
                assert_eq!(
                    (w.x1, w.x2, w.width, w.layer, w.curve),
                    (0, 1_000_000, 150_000, 21, 0)
                );
            }
            other => panic!("{other:?}"),
        }
        s.tool = LibTool::Arc;
        s.click(0, 0);
        s.click(0, 2_000_000);
        match &s.items()[5] {
            Item::Wire(w) => assert_eq!(w.curve, 90_000_000),
            other => panic!("{other:?}"),
        }
        s.tool = LibTool::Circle;
        s.click(0, 0);
        s.click(3_000_000, 4_000_000);
        match &s.items()[6] {
            Item::Circle(c) => assert_eq!((c.x, c.y, c.radius), (0, 0, 5_000_000)),
            other => panic!("{other:?}"),
        }
        // two clicks on one point make nothing
        s.tool = LibTool::Line;
        s.click(2_000_000, 2_000_000);
        s.click(2_000_000, 2_000_000);
        assert_eq!(s.items().len(), 7);
    }

    #[test]
    fn the_text_tool_asks_first_for_the_name_and_the_value() {
        let mut s = session();
        s.delete_entity_items_for_test();
        s.tool = LibTool::Text;
        s.click(0, 0);
        s.click(0, 1_000_000);
        s.click(0, 2_000_000);
        let texts: Vec<(String, u32)> = s
            .items()
            .iter()
            .filter_map(|i| match i {
                Item::Text(t) => Some((t.content.clone(), t.layer)),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            vec![
                (">NAME".to_owned(), 25),
                (">VALUE".to_owned(), 27),
                ("TEXT".to_owned(), 21)
            ]
        );
    }

    impl LibrarySession {
        /// Takes the texts away so that a test can place them with the tool.
        fn delete_entity_items_for_test(&mut self) {
            let target = self.target().unwrap();
            self.apply(vec![
                Operation::RemoveItem {
                    target: target.clone(),
                    index: 3,
                },
                Operation::RemoveItem { target, index: 2 },
            ]);
        }
    }

    #[test]
    fn a_tool_that_does_not_fit_the_drawing_says_so() {
        let mut s = session();
        s.tool = LibTool::Pin;
        s.click(0, 0);
        assert_eq!(s.note, Some(LibNote::WrongKind));
        s.select_entity(Kind::Symbol, Some("R".to_owned()));
        s.tool = LibTool::Smd;
        s.click(0, 0);
        assert_eq!(s.note, Some(LibNote::WrongKind));
        s.tool = LibTool::Pin;
        s.snap = false;
        s.click(-5_080_000, 2_540_000);
        match s.items().last().unwrap() {
            Item::Pin(p) => assert_eq!(
                (p.name.as_str(), p.direction.as_str(), p.length.as_str()),
                ("3", "pas", "short")
            ),
            other => panic!("{other:?}"),
        }
        s.select_entity(Kind::DeviceSet, Some("R".to_owned()));
        s.tool = LibTool::Line;
        s.click(0, 0);
        assert_eq!(s.note, Some(LibNote::NoEntity));
    }

    #[test]
    fn a_click_with_the_select_tool_picks_the_item_under_it() {
        let mut s = session();
        s.click(-800_000, 100_000);
        assert_eq!(s.selected, Some(0));
        s.click(800_000, 0);
        assert_eq!(s.selected, Some(1));
        s.click(0, 5_000_000);
        assert_eq!(s.selected, None);
        // the tolerance follows the scale of the view: 8 points at 40 points per millimetre
        assert_eq!(s.pick_tolerance_nm(), 200_000);
        assert_eq!(s.pick(-800_000 - 400_000 - 150_000, 0, 200_000), Some(0));
        assert_eq!(s.pick(-800_000 - 400_000 - 250_000, 0, 200_000), None);
    }

    #[test]
    fn dragging_an_item_moves_it_by_the_snapped_shift_as_one_step_that_can_be_undone() {
        let mut s = session();
        assert!(s.press(800_000, 0));
        assert_eq!(s.selected, Some(1));
        s.drag_to(1_600_000, 120_000);
        // 800000 along x is on the grid; 120000 along y rounds to 100000
        assert_eq!(s.drag_delta(), Some((800_000, 100_000)));
        let shapes = s.shapes();
        match &shapes[1] {
            LibShape::Pad {
                shape: Shape::Rect { cx, cy, .. },
                ..
            } => {
                assert!(
                    (cx - 1.6).abs() < 1e-9 && (cy - 0.1).abs() < 1e-9,
                    "{cx} {cy}"
                );
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(pad_xy(&s, 1), (800_000, 0));
        s.release(1_600_000, 120_000);
        assert_eq!(pad_xy(&s, 1), (1_600_000, 100_000));
        s.undo();
        assert_eq!(pad_xy(&s, 1), (800_000, 0));
        s.redo();
        assert_eq!(pad_xy(&s, 1), (1_600_000, 100_000));
        // a release that did not move changes nothing
        assert!(s.press(-800_000, 0));
        s.release(-800_000, 0);
        assert_eq!(pad_xy(&s, 0), (-800_000, 0));
        // other tools do not drag
        s.tool = LibTool::Smd;
        assert!(!s.press(0, 0));
    }

    #[test]
    fn items_are_deleted_turned_and_changed() {
        let mut s = session();
        s.click(-800_000, 0);
        assert!(s.rotate_selected());
        match s.selected_item().unwrap() {
            Item::Smd(p) => assert_eq!(p.rot, "R90"),
            other => panic!("{other:?}"),
        }
        assert!(s.rotate_selected());
        match s.selected_item().unwrap() {
            Item::Smd(p) => assert_eq!(p.rot, "R180"),
            other => panic!("{other:?}"),
        }
        let mut changed = s.selected_item().unwrap().clone();
        if let Item::Smd(p) = &mut changed {
            p.dx = 1_000_000;
        }
        assert!(s.replace_selected(changed));
        match s.selected_item().unwrap() {
            Item::Smd(p) => assert_eq!((p.dx, p.rot.as_str()), (1_000_000, "R180")),
            other => panic!("{other:?}"),
        }
        assert!(s.delete_selected());
        assert_eq!(s.items().len(), 3);
        assert_eq!(s.selected, None);
        assert!(!s.delete_selected());
        assert_eq!(s.note, Some(LibNote::NoSelection));
        // a line has no rotation
        s.tool = LibTool::Line;
        s.click(0, 0);
        s.click(1_000_000, 0);
        assert!(!s.rotate_selected());
        assert_eq!(s.note, Some(LibNote::CannotRotate));
    }

    #[test]
    fn a_quarter_turn_after_another_wraps_and_keeps_the_mirror() {
        assert_eq!(turned(""), "R90");
        assert_eq!(turned("R270"), "R0");
        assert_eq!(turned("MR90"), "MR180");
        assert_eq!(turned("SR45.5"), "R135.5");
    }

    #[test]
    fn the_measure_takes_two_clicks_and_starts_again_on_the_third() {
        let mut s = session();
        s.tool = LibTool::Measure;
        s.click(0, 0);
        assert_eq!(s.measure_distance(), None);
        s.click(3_000_000, 4_000_000);
        assert!((s.measure_distance().unwrap() - 5.0).abs() < 1e-12);
        s.click(1_000_000, 1_000_000);
        assert_eq!(s.measure, [Some((1_000_000, 1_000_000)), None]);
        s.cancel();
        assert_eq!(s.measure, [None, None]);
    }

    #[test]
    fn the_shapes_follow_the_items() {
        let mut s = session();
        s.tool = LibTool::Line;
        s.click(0, 0);
        s.click(1_000_000, 0);
        s.tool = LibTool::Arc;
        s.click(0, 0);
        s.click(0, 2_000_000);
        s.tool = LibTool::Circle;
        s.click(0, 0);
        s.click(1_000_000, 0);
        let shapes = s.shapes();
        assert_eq!(shapes.len(), 7);
        assert!(matches!(shapes[0], LibShape::Pad { hole_r, .. } if hole_r == 0.0));
        assert!(matches!(shapes[2], LibShape::Label { .. }));
        assert!(matches!(shapes[4], LibShape::Line { .. }));
        assert!(
            matches!(&shapes[5], LibShape::Poly { points, closed: false, .. } if points.len() > 3)
        );
        assert!(matches!(shapes[6], LibShape::Disc { filled: false, .. }));
        for (i, shape) in shapes.iter().enumerate() {
            assert_eq!(shape.index(), i);
        }
        s.select_entity(Kind::Symbol, Some("R".to_owned()));
        let pins = s.shapes();
        match (&pins[0], &pins[1]) {
            (
                LibShape::Pin { from, to, .. },
                LibShape::Pin {
                    from: f2, to: t2, ..
                },
            ) => {
                assert_eq!((*from, *to), ((-5.08, 0.0), (-2.54, 0.0)));
                assert!(
                    (f2.0 - 5.08).abs() < 1e-9 && (t2.0 - 2.54).abs() < 1e-9,
                    "{f2:?} {t2:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn entities_are_made_renamed_and_deleted_and_a_refusal_is_noted() {
        let mut s = session();
        assert!(s.create_entity(Kind::Package, "NEW"));
        assert_eq!(s.name.as_deref(), Some("NEW"));
        assert!(!s.create_entity(Kind::Package, "NEW"));
        assert!(s.note.as_ref().unwrap().is_error());
        assert!(s.rename_entity("NEW2"));
        assert_eq!(s.library().names(Kind::Package), ["R0603", "NEW2"]);
        assert!(s.delete_entity());
        assert_eq!(s.name, None);
        assert!(!s.delete_entity());
        // a footprint that a device uses stays
        s.select_entity(Kind::Package, Some("R0603".to_owned()));
        assert!(!s.delete_entity());
        match &s.note {
            Some(LibNote::Error(e)) => assert_eq!(e.code, IN_USE),
            other => panic!("{other:?}"),
        }
        assert!(s.create_entity(Kind::DeviceSet, "SET"));
        assert_eq!(s.library().deviceset("SET").unwrap().prefix, "U");
        assert!(s.create_entity(Kind::Symbol, "SYM"));
        assert_eq!(s.grid_nm, 1_270_000);
        s.select_entity(Kind::Package, Some("R0603".to_owned()));
        assert_eq!(s.grid_nm, 50_000);
    }

    #[test]
    fn undo_and_redo_say_when_there_is_nothing_and_the_selection_survives_them() {
        let mut s = session();
        s.undo();
        assert_eq!(s.note, Some(LibNote::NothingUndo));
        s.redo();
        assert_eq!(s.note, Some(LibNote::NothingRedo));
        s.tool = LibTool::Smd;
        s.click(0, 0);
        assert_eq!(s.selected, Some(4));
        s.undo();
        assert_eq!(s.note, Some(LibNote::Undone));
        assert_eq!(s.selected, None);
        assert!(s.can_redo() && !s.can_undo());
        s.redo();
        assert_eq!(s.note, Some(LibNote::Redone));
        // the footprint that is on the board is made again by the undo of its deletion
        s.select_entity(Kind::Package, Some("NEW".to_owned()));
        assert!(s.create_entity(Kind::Package, "NEW"));
        s.undo();
        assert_eq!(s.name, None);
    }

    #[test]
    fn the_check_runs_after_an_edit_and_when_it_is_asked_for() {
        let mut s = session();
        assert!(s.report.findings.is_empty());
        s.tool = LibTool::Smd;
        s.snap = false;
        s.click(-800_000, 0);
        // a second pad on the place of the first has the name 3, so no finding yet
        assert!(s.report.findings.is_empty());
        let item = Item::Smd(Smd {
            name: "1".into(),
            dx: 500_000,
            dy: 500_000,
            ..Smd::default()
        });
        assert!(s.add_item(item));
        assert_eq!(s.report.errors(), 1);
        // above the limit of the settings the check waits for the button
        let settings = LibSettings {
            auto_check_max_items: 1,
            ..LibSettings::default()
        };
        let library = s.library().clone();
        let mut big = LibrarySession::new(library, None, settings);
        big.select_entity(Kind::Package, Some("R0603".to_owned()));
        assert_eq!(big.report.errors(), 1);
        big.undo();
        assert_eq!(big.note, Some(LibNote::NothingUndo));
        assert!(big.add_item(Item::Smd(Smd {
            name: "9".into(),
            dx: 500_000,
            dy: 500_000,
            ..Smd::default()
        })));
        big.check();
        assert_eq!(big.note, Some(LibNote::Checked));
    }

    #[test]
    fn an_opening_is_set_for_the_selected_pad() {
        let mut s = session();
        s.click(-800_000, 0);
        assert!(s.set_opening(OpeningKind::Mask, Some(50_000)));
        assert!(s.items().iter().any(|i| i.layer() == Some(29)));
        assert!(s.set_opening(OpeningKind::Mask, None));
        assert!(!s.items().iter().any(|i| i.layer() == Some(29)));
        s.selected = Some(2);
        assert!(!s.set_opening(OpeningKind::Mask, Some(0)));
        assert!(s.note.as_ref().unwrap().is_error());
        s.selected = None;
        assert!(!s.set_opening(OpeningKind::Mask, Some(0)));
        assert_eq!(s.note, Some(LibNote::NoSelection));
    }

    #[test]
    fn the_origin_moves_to_the_cursor_on_the_grid() {
        let mut s = session();
        assert!(s.origin_to(800_010, 0));
        assert_eq!(pad_xy(&s, 0), (-1_600_000, 0));
        assert_eq!(pad_xy(&s, 1), (0, 0));
    }

    #[test]
    fn a_generated_footprint_is_added_and_put_on_the_board() {
        let mut s = session();
        let mut params = BTreeMap::new();
        params.insert("size".to_owned(), "0805".to_owned());
        assert!(s.add_generated("chip", &params, &FootprintConfig::builtin()));
        assert_eq!(s.name.as_deref(), Some("CHIP_0805"));
        assert_eq!(s.note, Some(LibNote::Generated));
        assert!(s.items().len() > 10);
        assert!(!s.add_generated("chip", &params, &FootprintConfig::builtin()));
        assert!(s.note.as_ref().unwrap().is_error());
        params.insert("size".to_owned(), "9999".to_owned());
        assert!(!s.add_generated("chip", &params, &FootprintConfig::builtin()));
        // all of it is one step
        s.undo();
        assert_eq!(s.library().names(Kind::Package), ["R0603"]);
    }

    #[test]
    fn saving_makes_a_new_file_and_marks_the_library_as_saved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("in.lbr");
        fs::write(&path, SAMPLE).unwrap();
        let mut s = LibrarySession::open(&path, LibSettings::default()).unwrap();
        s.select_entity(Kind::Package, Some("R0603".to_owned()));
        s.tool = LibTool::Smd;
        s.click(0, 0);
        assert!(s.dirty());
        assert!(!s.save_as(&path));
        assert!(s.note.as_ref().unwrap().is_error());
        let out = dir.path().join("out.lbr");
        assert!(s.save_as(&out));
        assert_eq!(s.note, Some(LibNote::Saved(true)));
        assert!(!s.dirty());
        assert_eq!(s.path.as_deref(), Some(out.as_path()));
        assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE);
        let back = LibrarySession::open(&out, LibSettings::default()).unwrap();
        assert_eq!(back.library().package("R0603").unwrap().items.len(), 5);
    }

    #[test]
    fn a_plan_is_loaded_and_nothing_is_replaced_without_the_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mine.lbr");
        fs::write(&path, SAMPLE).unwrap();
        let master_text = SAMPLE.replace("R0603", "R0603_M");
        fs::write(dir.path().join("master.lbr"), master_text).unwrap();
        let plan = dir.path().join("plan.json");
        fs::write(
            &plan,
            r#"{"format":"phoenix-sanitize-plan/1","groups":[
              {"id":"P001","kind":"package","keep":{"library":"master.lbr","name":"R0603_M"},
               "replace":[{"library":"mine.lbr","name":"R0603","score":100,"exact":true,"turned":false}],
               "review":[]}]}"#,
        )
        .unwrap();
        let mut s = LibrarySession::open(&path, LibSettings::default()).unwrap();
        assert!(s.load_plan(&plan));
        assert_eq!(s.plan.as_ref().unwrap().steps.len(), 1);
        assert!(!s.replace_with_master(0, dir.path(), false));
        match &s.note {
            Some(LibNote::Error(e)) => assert_eq!(e.code, NOT_CONFIRMED),
            other => panic!("{other:?}"),
        }
        assert!(!s.dirty());
        assert!(s.replace_with_master(0, dir.path(), true));
        assert!(s.dirty());
        assert!(s.can_undo());
        assert!(!s.replace_with_master(5, dir.path(), true));
        s.undo();
        assert!(!s.dirty());
        let mut bare = session();
        assert!(!bare.load_plan(&plan));
        assert!(bare.note.as_ref().unwrap().is_error());
    }

    #[test]
    fn the_view_maps_millimetres_to_points_and_back_and_zooms_around_the_pointer() {
        let mut v = LibView::default();
        let (sx, sy) = v.to_screen(1.0, 2.0);
        assert_eq!((sx, sy), (440.0, 220.0));
        let (x, y) = v.to_world(sx, sy);
        assert!((x - 1.0).abs() < 1e-12 && (y - 2.0).abs() < 1e-12);
        v.pan_px(40.0, -40.0);
        assert!((v.cx + 1.0).abs() < 1e-12 && (v.cy + 1.0).abs() < 1e-12);
        let before = v.to_world(100.0, 100.0);
        v.zoom_at(100.0, 100.0, 120.0, 1.0);
        let after = v.to_world(100.0, 100.0);
        assert!(v.scale > 40.0);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
        v.fit(Some(Bounds {
            min_x: 0,
            min_y: 0,
            max_x: 10_000_000,
            max_y: 5_000_000,
        }));
        assert_eq!((v.cx, v.cy), (5.0, 2.5));
        // 10 mm wide in 800 points with a margin of 40 per cent
        assert!((v.scale - 800.0 / 14.0).abs() < 1e-9, "{}", v.scale);
        v.fit(None);
        assert_eq!((v.cx, v.cy, v.scale), (0.0, 0.0, 40.0));
    }

    #[test]
    fn tools_say_what_they_need() {
        assert!(LibTool::Line.two_clicks() && !LibTool::Smd.two_clicks());
        assert!(LibTool::Smd.draws() && !LibTool::Select.draws() && !LibTool::Measure.draws());
        assert!(LibTool::Smd.fits_package() && !LibTool::Smd.fits_symbol());
        assert!(LibTool::Pin.fits_symbol() && !LibTool::Pin.fits_package());
        assert!(LibTool::Line.fits_package() && LibTool::Line.fits_symbol());
        assert_eq!(LibTool::ALL.len(), 10);
        assert_eq!(pin_length_nm("point"), 0);
        assert_eq!(pin_length_nm("long"), 7_620_000);
        let note = LibNote::Saved(true);
        assert!(note.text(Lang::Pt).contains("ficheiro"));
        assert!(note.text(Lang::Pt).contains("Cópia"));
        let err = LibNote::Error(LibError::new(VALUE_INVALID, "x"));
        assert!(err.text(Lang::Es).starts_with("PHOENIX-LIB-024"));
    }
}
