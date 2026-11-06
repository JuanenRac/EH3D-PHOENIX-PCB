// SPDX-License-Identifier: GPL-3.0-or-later
//! The scene of a panel of boards, without a screen: the copies of the boards, the frame, the tabs,
//! the V-score lines, the fiducials and the holes of a plan of `eh3d-phoenix-panel` as shapes in
//! millimetres on layers that can be shown and hidden, the measures of the panel and the part of
//! its area that the boards take. The session reads the boards, lays the panel out and lays it out
//! again when the matrix or the joint changes. The window (`panel_gui`, behind the `gui`
//! feature) only draws this and passes on what the person does.

use std::path::{Path, PathBuf};

use eh3d_phoenix_board::Board;
use eh3d_phoenix_panel::{
    items_from_settings, permille_text, plan, plan_items, BoardShape, HoleKind, Joint, Note,
    PanelError, PanelPlan, PanelSettings,
};
use eh3d_phoenix_rules::Lang;

use crate::codes::{ViewError, PANEL_UNUSABLE};
use crate::gerber_scene::load_board;
use crate::panel_text::PanelText;
use crate::prim::{Bounds, P};
use crate::settings::ViewerSettings;

/// The largest settings file of a panel that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

/// The kinds of layer of the panel scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelLayerKind {
    /// The rails of the frame.
    Frame,
    /// The copies of the boards.
    Boards,
    /// The routed outline.
    Outline,
    /// The tabs that hold the boards.
    Tabs,
    /// The V-score lines.
    VScore,
    /// The mouse bite holes.
    MouseBites,
    /// The fiducials of the frame and of the boards.
    Fiducials,
    /// The tooling holes.
    Tooling,
    /// The holes of the boards.
    Holes,
}

impl PanelLayerKind {
    /// Every kind, in the order they are drawn (the last is on top).
    pub const ALL: [PanelLayerKind; 9] = [
        PanelLayerKind::Frame,
        PanelLayerKind::Boards,
        PanelLayerKind::Holes,
        PanelLayerKind::Outline,
        PanelLayerKind::VScore,
        PanelLayerKind::Tabs,
        PanelLayerKind::MouseBites,
        PanelLayerKind::Tooling,
        PanelLayerKind::Fiducials,
    ];

    /// The text that names the layer.
    pub fn text(self) -> PanelText {
        match self {
            PanelLayerKind::Frame => PanelText::LayerFrame,
            PanelLayerKind::Boards => PanelText::Boards,
            PanelLayerKind::Outline => PanelText::LayerOutline,
            PanelLayerKind::Tabs => PanelText::Tabs,
            PanelLayerKind::VScore => PanelText::LayerVScore,
            PanelLayerKind::MouseBites => PanelText::LayerMouseBites,
            PanelLayerKind::Fiducials => PanelText::LayerFiducials,
            PanelLayerKind::Tooling => PanelText::LayerTooling,
            PanelLayerKind::Holes => PanelText::LayerHoles,
        }
    }

    /// The colour of the layer, red, green, blue and alpha.
    pub fn colour(self) -> [u8; 4] {
        match self {
            PanelLayerKind::Frame => [0x4a, 0x55, 0x68, 0xff],
            PanelLayerKind::Boards => [0x2f, 0x8f, 0x5b, 0xc0],
            PanelLayerKind::Outline => [0xe0, 0xe0, 0xe0, 0xff],
            PanelLayerKind::Tabs => [0xf2, 0xc2, 0x4b, 0xff],
            PanelLayerKind::VScore => [0xe8, 0x5d, 0x5d, 0xff],
            PanelLayerKind::MouseBites => [0xf2, 0x8c, 0x28, 0xff],
            PanelLayerKind::Fiducials => [0x5d, 0xc8, 0xe8, 0xff],
            PanelLayerKind::Tooling => [0xb0, 0x7a, 0xe8, 0xff],
            PanelLayerKind::Holes => [0x10, 0x18, 0x20, 0xff],
        }
    }
}

/// A shape of the scene, in millimetres; y points up.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelShape {
    /// A rectangle.
    Rect {
        /// The corner with the smallest coordinates.
        min: P,
        /// The corner with the largest coordinates.
        max: P,
        /// True to fill it, false to draw its edge only.
        filled: bool,
    },
    /// A straight line.
    Line {
        /// Start.
        from: P,
        /// End.
        to: P,
    },
    /// A circle.
    Circle {
        /// The centre.
        centre: P,
        /// The radius.
        radius: f64,
        /// True to fill it, false to draw its edge only.
        filled: bool,
    },
    /// A short text.
    Label {
        /// The middle of the text.
        at: P,
        /// The text.
        text: String,
    },
}

impl PanelShape {
    fn bounds(&self) -> Bounds {
        let mut b = Bounds::EMPTY;
        match self {
            PanelShape::Rect { min, max, .. } => {
                b.include(*min);
                b.include(*max);
            }
            PanelShape::Line { from, to } => {
                b.include(*from);
                b.include(*to);
            }
            PanelShape::Circle { centre, radius, .. } => {
                b.include(P::new(centre.x - radius, centre.y - radius));
                b.include(P::new(centre.x + radius, centre.y + radius));
            }
            PanelShape::Label { at, .. } => b.include(*at),
        }
        b
    }
}

/// A layer of the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelLayer {
    /// What the layer is.
    pub kind: PanelLayerKind,
    /// True while it is drawn.
    pub visible: bool,
    /// Its shapes.
    pub shapes: Vec<PanelShape>,
}

/// What the scene keeps of a copy of a board, for the list and for the click.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyInfo {
    /// The corner of its box with the smallest coordinates, in millimetres.
    pub min: P,
    /// The opposite corner.
    pub max: P,
    /// The number of the board in the list, from 0.
    pub board: usize,
    /// The turn in degrees.
    pub rotation_deg: u32,
    /// True when it is mirrored.
    pub mirror: bool,
}

/// The panel as a scene.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelScene {
    /// The layers, in the order they are drawn.
    pub layers: Vec<PanelLayer>,
    /// The width of the panel in millimetres.
    pub width: f64,
    /// The height of the panel in millimetres.
    pub height: f64,
    /// The copies of the boards.
    pub copies: Vec<CopyInfo>,
    /// The part of the area of the panel that the boards take, in thousandths.
    pub utilisation_permille: u32,
    /// What the layout changed or left out.
    pub notes: Vec<Note>,
    /// True when the boards are separated by V-score lines.
    pub v_score: bool,
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

fn point(x: i64, y: i64) -> P {
    P::new(mm(x), mm(y))
}

impl PanelScene {
    /// Draws a plan as a scene.
    pub fn from_plan(plan: &PanelPlan, settings: &PanelSettings) -> PanelScene {
        let mut layers: Vec<PanelLayer> = PanelLayerKind::ALL
            .iter()
            .map(|kind| PanelLayer {
                kind: *kind,
                visible: true,
                shapes: Vec::new(),
            })
            .collect();
        let mut push = |kind: PanelLayerKind, shape: PanelShape| {
            if let Some(layer) = layers.iter_mut().find(|l| l.kind == kind) {
                layer.shapes.push(shape);
            }
        };
        let (w, h) = (plan.width, plan.height);
        let rail_x = if settings.frame.has_left_right() {
            settings.frame_width_nm
        } else {
            0
        };
        let rail_y = if settings.frame.has_top_bottom() {
            settings.frame_width_nm
        } else {
            0
        };
        let rails = [
            (rail_x > 0, 0, 0, rail_x, h),
            (rail_x > 0, w - rail_x, 0, w, h),
            (rail_y > 0, 0, 0, w, rail_y),
            (rail_y > 0, 0, h - rail_y, w, h),
        ];
        for (present, x0, y0, x1, y1) in rails {
            if present {
                push(
                    PanelLayerKind::Frame,
                    PanelShape::Rect {
                        min: point(x0, y0),
                        max: point(x1, y1),
                        filled: true,
                    },
                );
            }
        }
        let mut copies = Vec::new();
        for (index, copy) in plan.copies.iter().enumerate() {
            let min = point(copy.origin.x, copy.origin.y);
            let max = point(copy.origin.x + copy.width, copy.origin.y + copy.height);
            push(
                PanelLayerKind::Boards,
                PanelShape::Rect {
                    min,
                    max,
                    filled: true,
                },
            );
            push(
                PanelLayerKind::Boards,
                PanelShape::Label {
                    at: P::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0),
                    text: (index + 1).to_string(),
                },
            );
            copies.push(CopyInfo {
                min,
                max,
                board: copy.board,
                rotation_deg: copy.rotation_deg,
                mirror: copy.mirror,
            });
        }
        for line in plan.profile.iter() {
            push(
                PanelLayerKind::Outline,
                PanelShape::Line {
                    from: point(line.from.x, line.from.y),
                    to: point(line.to.x, line.to.y),
                },
            );
        }
        for line in plan.score.iter() {
            push(
                PanelLayerKind::VScore,
                PanelShape::Line {
                    from: point(line.from.x, line.from.y),
                    to: point(line.to.x, line.to.y),
                },
            );
        }
        for tab in plan.tabs.iter() {
            push(
                PanelLayerKind::Tabs,
                PanelShape::Circle {
                    centre: point(tab.x, tab.y),
                    radius: mm(settings.tab_width_nm) / 2.0,
                    filled: false,
                },
            );
        }
        for hole in plan.holes.iter() {
            let kind = match hole.kind {
                HoleKind::Plated | HoleKind::NonPlated => PanelLayerKind::Holes,
                HoleKind::MouseBite => PanelLayerKind::MouseBites,
                HoleKind::Tooling => PanelLayerKind::Tooling,
            };
            push(
                kind,
                PanelShape::Circle {
                    centre: point(hole.at.x, hole.at.y),
                    radius: mm(hole.diameter) / 2.0,
                    filled: true,
                },
            );
        }
        for mark in plan.fiducials.iter().chain(plan.local_fiducials.iter()) {
            let centre = point(mark.at.x, mark.at.y);
            push(
                PanelLayerKind::Fiducials,
                PanelShape::Circle {
                    centre,
                    radius: mm(mark.mask_diameter) / 2.0,
                    filled: false,
                },
            );
            push(
                PanelLayerKind::Fiducials,
                PanelShape::Circle {
                    centre,
                    radius: mm(mark.copper_diameter) / 2.0,
                    filled: true,
                },
            );
        }
        PanelScene {
            layers,
            width: mm(plan.width),
            height: mm(plan.height),
            copies,
            utilisation_permille: plan.utilisation_permille(),
            notes: plan.notes.clone(),
            v_score: settings.joint == Joint::VScore,
        }
    }

    /// True while a kind of layer is drawn.
    pub fn visible(&self, kind: PanelLayerKind) -> bool {
        self.layers.iter().any(|l| l.kind == kind && l.visible)
    }

    /// Shows or hides a kind of layer.
    pub fn set_visible(&mut self, kind: PanelLayerKind, visible: bool) {
        for layer in self.layers.iter_mut().filter(|l| l.kind == kind) {
            layer.visible = visible;
        }
    }

    /// Shows or hides every layer.
    pub fn set_all_visible(&mut self, visible: bool) {
        for layer in &mut self.layers {
            layer.visible = visible;
        }
    }

    /// The number of shapes of a kind of layer.
    pub fn count(&self, kind: PanelLayerKind) -> usize {
        self.layers
            .iter()
            .filter(|l| l.kind == kind)
            .map(|l| l.shapes.len())
            .sum()
    }

    /// The box that holds the whole panel.
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::EMPTY;
        b.include(P::new(0.0, 0.0));
        b.include(P::new(self.width, self.height));
        for layer in &self.layers {
            for shape in &layer.shapes {
                b = b.union(shape.bounds());
            }
        }
        b
    }

    /// The copy under a point, as its position in the list of copies.
    pub fn copy_at(&self, at: P) -> Option<usize> {
        self.copies.iter().position(|c| {
            (c.min.x..=c.max.x).contains(&at.x) && (c.min.y..=c.max.y).contains(&at.y)
        })
    }

    /// The measures of the panel as lines of text in a language: the size, the number of boards,
    /// the part of the area that they take and the notes of the layout.
    pub fn summary(&self, lang: Lang) -> Vec<String> {
        let mut lines = vec![
            format!(
                "{}: {:.3} x {:.3} mm",
                PanelText::Size.tr(lang),
                self.width,
                self.height
            ),
            format!("{}: {}", PanelText::Boards.tr(lang), self.copies.len()),
            format!(
                "{}: {} %",
                PanelText::Use.tr(lang),
                permille_text(self.utilisation_permille)
            ),
        ];
        for (index, copy) in self.copies.iter().enumerate() {
            let mut line = format!(
                "{} {}: {:.3} x {:.3} mm",
                PanelText::Board.tr(lang),
                index + 1,
                copy.max.x - copy.min.x,
                copy.max.y - copy.min.y
            );
            if copy.rotation_deg != 0 {
                line.push_str(&format!(
                    ", {} {}°",
                    PanelText::Turned.tr(lang),
                    copy.rotation_deg
                ));
            }
            if copy.mirror {
                line.push_str(&format!(", {}", PanelText::Mirrored.tr(lang)));
            }
            lines.push(line);
        }
        for note in &self.notes {
            lines.push(note.message(lang));
        }
        lines
    }
}

/// The board files typed in a box: names separated by commas.
pub fn parse_files(text: &str) -> Vec<PathBuf> {
    text.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn refused(error: PanelError) -> ViewError {
    ViewError::new(PANEL_UNUSABLE, error.message(Lang::En))
}

/// The settings of a panel from a file; the defaults when no file is given.
pub fn read_settings(file: Option<&Path>) -> Result<PanelSettings, ViewError> {
    let Some(path) = file else {
        return Ok(PanelSettings::default());
    };
    let unusable =
        |e: std::io::Error| ViewError::new(PANEL_UNUSABLE, format!("{}: {e}", path.display()));
    let size = std::fs::metadata(path).map_err(unusable)?.len();
    if size > MAX_SETTINGS_BYTES {
        return Err(ViewError::new(
            PANEL_UNUSABLE,
            format!(
                "{} is {size} bytes, the limit is {MAX_SETTINGS_BYTES}",
                path.display()
            ),
        ));
    }
    let text = std::fs::read_to_string(path).map_err(unusable)?;
    PanelSettings::parse(&text).map_err(refused)
}

/// A panel that is laid out: the boards, the settings, the plan and the scene.
#[derive(Debug, Clone)]
pub struct PanelSession {
    /// The boards, in the order of the list.
    pub boards: Vec<Board>,
    /// The settings of the layout.
    pub settings: PanelSettings,
    /// The plan.
    pub plan: PanelPlan,
    /// The scene.
    pub scene: PanelScene,
}

/// Lays the boards out: a matrix of the first board when there is one board and no `place` line,
/// else the list.
pub fn layout(boards: &[Board], settings: &PanelSettings) -> Result<PanelPlan, ViewError> {
    if boards.is_empty() {
        return Err(ViewError::new(PANEL_UNUSABLE, "there is no board"));
    }
    let mut shapes = Vec::new();
    let mut notes: Vec<Note> = Vec::new();
    for board in boards {
        let (shape, mut found) = BoardShape::from_board(board).map_err(refused)?;
        shapes.push(shape);
        notes.append(&mut found);
    }
    let mut laid = if boards.len() == 1 && settings.placements.is_empty() {
        plan(&shapes[0], settings).map_err(refused)?
    } else {
        let items = items_from_settings(boards.len(), settings);
        plan_items(&shapes, &items, settings).map_err(refused)?
    };
    notes.append(&mut laid.notes);
    laid.notes = notes;
    Ok(laid)
}

impl PanelSession {
    /// Reads the boards (EAGLE or the JSON of the model) and the settings file and lays the panel
    /// out.
    pub fn open(
        files: &[PathBuf],
        settings_file: Option<&Path>,
        viewer: &ViewerSettings,
    ) -> Result<PanelSession, ViewError> {
        let settings = read_settings(settings_file)?;
        let mut boards = Vec::new();
        for file in files {
            boards.push(load_board(file, viewer)?);
        }
        PanelSession::from_boards(boards, settings)
    }

    /// Lays out boards that are already read.
    pub fn from_boards(
        boards: Vec<Board>,
        settings: PanelSettings,
    ) -> Result<PanelSession, ViewError> {
        let laid = layout(&boards, &settings)?;
        let scene = PanelScene::from_plan(&laid, &settings);
        Ok(PanelSession {
            boards,
            settings,
            plan: laid,
            scene,
        })
    }

    /// Lays the panel out again with other settings. The layers that were hidden stay hidden. When
    /// the new settings cannot be used the panel stays as it was and the error is returned.
    pub fn relayout(&mut self, settings: PanelSettings) -> Result<(), ViewError> {
        let laid = layout(&self.boards, &settings)?;
        let mut scene = PanelScene::from_plan(&laid, &settings);
        for old in &self.scene.layers {
            scene.set_visible(old.kind, old.visible);
        }
        self.settings = settings;
        self.plan = laid;
        self.scene = scene;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use eh3d_phoenix_board::{Layer, LayerKind};
    use eh3d_phoenix_model::{ObjectId, Point};

    fn board(width_mm: i64, height_mm: i64) -> Board {
        let pt = |x: i64, y: i64| Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap();
        let mut layers = BTreeMap::new();
        layers.insert(
            ObjectId::from_parts(1_700_000_000_000, 1, 1).unwrap(),
            Layer {
                name: "Top".to_owned(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        Board {
            name: "b".to_owned(),
            outline: vec![
                pt(0, 0),
                pt(width_mm, 0),
                pt(width_mm, height_mm),
                pt(0, height_mm),
            ],
            layers,
            ..Board::default()
        }
    }

    fn session(text: &str, boards: Vec<Board>) -> PanelSession {
        PanelSession::from_boards(boards, PanelSettings::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn the_scene_holds_the_copies_the_frame_the_tabs_and_the_marks() {
        let s = session("columns = 2\nrows = 1\n", vec![board(30, 20)]);
        let scene = &s.scene;
        assert_eq!((scene.width, scene.height), (82.0, 40.0));
        assert_eq!(scene.copies.len(), 2);
        // a rail on each side, the boards with their numbers (a rectangle and a label each)
        assert_eq!(scene.count(PanelLayerKind::Frame), 4);
        assert_eq!(scene.count(PanelLayerKind::Boards), 4);
        assert_eq!(scene.count(PanelLayerKind::Tabs), s.plan.tabs.len());
        assert!(scene.count(PanelLayerKind::Tabs) > 0);
        assert_eq!(scene.count(PanelLayerKind::VScore), 0);
        // 4 tooling holes, 3 fiducials drawn with their opening and their copper
        assert_eq!(scene.count(PanelLayerKind::Tooling), 4);
        assert_eq!(scene.count(PanelLayerKind::Fiducials), 6);
        assert!(scene.count(PanelLayerKind::MouseBites) > 0);
        assert!(!scene.v_score);
        // 2 boards of 30 x 20 in 82 x 40
        assert_eq!(scene.utilisation_permille, 1200 * 1000 / 3280);
    }

    #[test]
    fn v_score_lines_replace_the_tabs() {
        let s = session(
            "columns = 2\nrows = 2\njoint = v_score\ngap_nm = 0\n",
            vec![board(30, 20)],
        );
        assert!(s.scene.v_score);
        assert_eq!(s.scene.count(PanelLayerKind::Tabs), 0);
        assert_eq!(s.scene.count(PanelLayerKind::MouseBites), 0);
        // 3 lines across and 3 lines up, as the frame has rails
        assert_eq!(s.scene.count(PanelLayerKind::VScore), 6);
    }

    #[test]
    fn layers_are_shown_and_hidden_and_the_bounds_hold_the_panel() {
        let mut s = session("columns = 2\nrows = 1\n", vec![board(30, 20)]);
        assert!(s.scene.visible(PanelLayerKind::Tabs));
        s.scene.set_visible(PanelLayerKind::Tabs, false);
        assert!(!s.scene.visible(PanelLayerKind::Tabs));
        assert!(s.scene.visible(PanelLayerKind::Boards));
        s.scene.set_all_visible(false);
        assert!(!s.scene.visible(PanelLayerKind::Boards));
        let b = s.scene.bounds();
        assert!(b.min.x <= 0.0 && b.min.y <= 0.0);
        assert!(b.max.x >= 82.0 && b.max.y >= 40.0);
        // a click inside the first board finds it, a click in the rail finds nothing
        assert_eq!(s.scene.copy_at(P::new(20.0, 20.0)), Some(0));
        assert_eq!(s.scene.copy_at(P::new(60.0, 20.0)), Some(1));
        assert_eq!(s.scene.copy_at(P::new(2.0, 2.0)), None);
    }

    #[test]
    fn a_list_of_different_boards_is_packed_and_summarised_in_each_language() {
        let s = session("frame = none\n", vec![board(30, 20), board(20, 10)]);
        assert_eq!(s.scene.copies.len(), 2);
        assert_eq!(s.scene.copies[1].min, P::new(32.0, 0.0));
        let lines = s.scene.summary(Lang::En);
        assert_eq!(lines[0], "Size: 52.000 x 20.000 mm");
        assert_eq!(lines[1], "Boards: 2");
        assert!(
            lines[2].starts_with("Area used by the boards: "),
            "{lines:?}"
        );
        assert!(lines[2].ends_with(" %"), "{lines:?}");
        let pt = s.scene.summary(Lang::Pt);
        assert_eq!(pt[0], "Tamanho: 52.000 x 20.000 mm");
        assert_eq!(pt[1], "Placas: 2");
        let ja = s.scene.summary(Lang::Ja);
        assert!(ja[0].starts_with("サイズ: "), "{ja:?}");
    }

    #[test]
    fn turned_and_mirrored_copies_are_named_in_the_list() {
        let s = session(
            "columns = 1\nrows = 1\ncopy_rotation_deg = 90\ncopy_mirror = true\n",
            vec![board(30, 20)],
        );
        let lines = s.scene.summary(Lang::De);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("gedreht 90°") && l.contains("gespiegelt")),
            "{lines:?}"
        );
        assert_eq!(
            (
                s.scene.copies[0].max.x - s.scene.copies[0].min.x,
                s.scene.copies[0].rotation_deg
            ),
            (20.0, 90)
        );
    }

    #[test]
    fn a_new_layout_keeps_the_hidden_layers_hidden_and_a_bad_one_changes_nothing() {
        let mut s = session("columns = 2\nrows = 1\n", vec![board(30, 20)]);
        s.scene.set_visible(PanelLayerKind::Frame, false);
        let mut settings = s.settings.clone();
        settings.columns = 3;
        s.relayout(settings).unwrap();
        assert_eq!(s.scene.copies.len(), 3);
        assert!(!s.scene.visible(PanelLayerKind::Frame));
        let before = s.scene.clone();
        let mut bad = s.settings.clone();
        bad.max_boards = 2;
        let e = s.relayout(bad).unwrap_err();
        assert_eq!(e.code, PANEL_UNUSABLE);
        assert!(e.detail.contains("PHOENIX-PANEL-003"), "{}", e.detail);
        assert_eq!(s.scene, before);
        assert_eq!(s.settings.max_boards, 400);
        let none = PanelSession::from_boards(Vec::new(), PanelSettings::default());
        assert_eq!(none.unwrap_err().code, PANEL_UNUSABLE);
    }

    #[test]
    fn the_typed_files_are_split_at_the_commas() {
        assert_eq!(
            parse_files(" a.brd , b.json,, "),
            [PathBuf::from("a.brd"), PathBuf::from("b.json")]
        );
        assert!(parse_files("  ").is_empty());
    }

    #[test]
    fn the_settings_file_is_read_or_refused_with_the_code_of_the_viewer() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("panel.cfg");
        std::fs::write(&good, "columns = 3\n").unwrap();
        assert_eq!(read_settings(Some(&good)).unwrap().columns, 3);
        assert_eq!(read_settings(None).unwrap(), PanelSettings::default());
        let bad = dir.path().join("bad.cfg");
        std::fs::write(&bad, "columns = many\n").unwrap();
        assert_eq!(read_settings(Some(&bad)).unwrap_err().code, PANEL_UNUSABLE);
        let missing = dir.path().join("none.cfg");
        assert_eq!(
            read_settings(Some(&missing)).unwrap_err().code,
            PANEL_UNUSABLE
        );
    }
}
