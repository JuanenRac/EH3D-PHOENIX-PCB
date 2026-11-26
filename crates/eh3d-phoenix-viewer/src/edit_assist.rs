// SPDX-License-Identifier: GPL-3.0-or-later
//! The help of the board editor of the window, without a screen: the preview of a route that stops
//! at the first obstacle or pushes other tracks as it follows the pointer, the rule check of a
//! drag before it is let go, the measure with its snap and the history of the editor as the person
//! sees it. Each answer is kept until what it was made from changes, so that a window that draws
//! many times a second computes only when the pointer reaches another place. The window draws this
//! state in `board_gui`, behind the `gui` feature.

use eh3d_phoenix_boardedit::{
    BoardMeasure, EditOp, EditState, MeasurePoint, RouteAssist, RouteMode, SnapTo,
};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;

use crate::drc_live::{summarise, LiveDrc};
use crate::history_view::{HistoryView, Move};
use crate::measure_set::{format_length, Unit};

/// What the proposal of a route was made from.
type RouteKey = (u64, Option<Point>, Point, RouteMode, bool, bool);

/// The preview of the route that follows the pointer.
#[derive(Debug, Default)]
pub struct RoutePreview {
    /// What a click does.
    pub mode: RouteMode,
    key: Option<RouteKey>,
    found: Option<RouteAssist>,
    computed: u64,
}

impl RoutePreview {
    /// What a click at `p` would add, made again only when the board, the route, the place under
    /// the pointer or the mode changed.
    pub fn at(&mut self, state: &EditState, p: Point) -> Option<&RouteAssist> {
        let (target, _) = state.route_target(p);
        let key = (
            state.edit.revision(),
            state.route_points.last().copied(),
            target,
            self.mode,
            state.diagonal,
            state.horizontal_first,
        );
        if self.key != Some(key) || state.route_points.is_empty() {
            self.key = Some(key);
            self.found = state.route_assist(p, self.mode);
            self.computed += 1;
        }
        self.found.as_ref()
    }

    /// How many times a proposal was computed.
    pub fn computed(&self) -> u64 {
        self.computed
    }

    /// Chooses the next mode.
    pub fn next_mode(&mut self) {
        self.mode = self.mode.next();
        self.key = None;
        self.found = None;
    }
}

/// What the rules say about the place a drag is at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragVerdict {
    /// The rules were checked.
    Checked(LiveDrc),
    /// The operation could not be done there.
    Refused,
    /// The board has more objects than the settings check while dragging.
    TooLarge,
}

/// The check of a drag in progress.
#[derive(Debug, Default)]
pub struct DragCheck {
    key: Option<(u64, EditOp)>,
    verdict: Option<DragVerdict>,
    computed: u64,
}

/// The components, tracks, vias and zones of the board of the editor.
fn objects(state: &EditState) -> usize {
    let board = state.board();
    board.components.len() + board.tracks.len() + board.vias.len() + board.zones.len()
}

impl DragCheck {
    /// What the rules say about letting go now, when `op` is what letting go would do; `None`
    /// without an operation or when the check while dragging is off. The check is made again only
    /// when the board or the operation changed.
    pub fn check(&mut self, state: &EditState, op: Option<EditOp>) -> Option<&DragVerdict> {
        let settings = &state.edit.settings;
        let Some(op) = op.filter(|_| settings.drag_drc) else {
            self.key = None;
            self.verdict = None;
            return None;
        };
        if objects(state) > settings.drag_drc_max_objects {
            self.verdict = Some(DragVerdict::TooLarge);
            return self.verdict.as_ref();
        }
        let key = (state.edit.revision(), op);
        if self.key.as_ref() != Some(&key) {
            self.verdict = Some(match state.edit.trial(&key.1) {
                Ok(report) => DragVerdict::Checked(summarise(&report)),
                Err(_) => DragVerdict::Refused,
            });
            self.key = Some(key);
            self.computed += 1;
        }
        self.verdict.as_ref()
    }

    /// How many times the rules were checked.
    pub fn computed(&self) -> u64 {
        self.computed
    }
}

/// The measure of the board editor.
#[derive(Debug, Default)]
pub struct MeasureTool {
    /// Whether a click is a point of the measure.
    pub on: bool,
    /// The measure.
    pub measure: BoardMeasure,
}

impl MeasureTool {
    /// A click at `p`; `radius_nm` is the radius of the snap that the view asks for.
    pub fn click(&mut self, state: &EditState, p: Point, radius_nm: i64) {
        self.measure.click(BoardMeasure::snap(state, p, radius_nm));
    }

    /// Turns the tool on or off; the measure is dropped when it goes off.
    pub fn toggle(&mut self) {
        self.on = !self.on;
        if !self.on {
            self.measure.clear();
        }
    }

    /// The line that reads the measure: the length, the differences, the angle and what the end
    /// went to; `None` until there is a first point and a second place.
    pub fn line(
        &self,
        pointer: Option<MeasurePoint>,
        unit: Unit,
        decimals: usize,
        lang: Lang,
    ) -> Option<String> {
        let reading = self.measure.reading(pointer)?;
        let end = self.measure.b.or(pointer)?;
        let mm = |nm: i64| nm as f64 / 1_000_000.0;
        Some(format!(
            "{} (dx {}, dy {}, {:.2}°) · {}",
            format_length(mm(reading.length_nm), unit, decimals),
            format_length(mm(reading.dx_nm), unit, decimals),
            format_length(mm(reading.dy_nm), unit, decimals),
            reading.angle_deg,
            snap_name(end.to, lang),
        ))
    }
}

/// The history of the editor as the window shows it: the steps with their descriptions.
pub fn history(state: &EditState, lang: Lang, max: usize) -> HistoryView {
    HistoryView::of_editor(&state.edit, lang, max)
}

/// Does what going to a line of the history asks for; the number of steps undone or redone.
pub fn go(state: &mut EditState, movement: Move) -> usize {
    let (count, undo) = match movement {
        Move::Stay => return 0,
        Move::Undo(n) => (n, true),
        Move::Redo(n) => (n, false),
    };
    let mut done = 0;
    for _ in 0..count {
        let moved = if undo { state.undo() } else { state.redo() };
        if !moved {
            break;
        }
        done += 1;
    }
    done
}

/// A text of this help. The order of the variants is the order of the rows of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistMsg {
    /// The label of the choice of what a click of the route does.
    RouteMode,
    /// The free mode.
    Free,
    /// The mode that stops at the first obstacle.
    Stop,
    /// The mode that pushes other tracks.
    Push,
    /// The hint of the key that changes the mode.
    ModeKey,
    /// What follows the number of tracks that a push moves.
    Moves,
    /// The box of the measure tool, with its key.
    Measure,
    /// What the line of a drag says when the board is too large to check.
    TooLarge,
    /// What the line of a drag says when the operation cannot be done there.
    Refused,
    /// The title of the next step that undo takes back.
    NextUndo,
    /// The title of the next step that redo does again.
    NextRedo,
    /// The title of the list of steps.
    History,
}

const TEXTS: [(AssistMsg, [&str; 8]); 12] = [
    (
        AssistMsg::RouteMode,
        [
            "Route click",
            "Clic de pista",
            "Klick beim Verlegen",
            "Clic de routage",
            "Clic di tracciamento",
            "配線クリック",
            "布线点击",
            "Clique de traçado",
        ],
    ),
    (
        AssistMsg::Free,
        [
            "Free", "Libre", "Frei", "Libre", "Libero", "自由", "自由", "Livre",
        ],
    ),
    (
        AssistMsg::Stop,
        [
            "Stop at obstacle",
            "Parar en obstáculo",
            "Am Hindernis halten",
            "Arrêter à l'obstacle",
            "Ferma all'ostacolo",
            "障害物で停止",
            "遇障碍停止",
            "Parar no obstáculo",
        ],
    ),
    (
        AssistMsg::Push,
        [
            "Push tracks",
            "Empujar pistas",
            "Leiterbahnen schieben",
            "Pousser les pistes",
            "Spingi le piste",
            "配線を押しのける",
            "推挤走线",
            "Empurrar pistas",
        ],
    ),
    (
        AssistMsg::ModeKey,
        [
            "S changes the mode",
            "S cambia el modo",
            "S wechselt den Modus",
            "S change le mode",
            "S cambia la modalità",
            "S でモード切替",
            "S 切换模式",
            "S muda o modo",
        ],
    ),
    (
        AssistMsg::Moves,
        [
            "tracks move",
            "pistas se mueven",
            "Leiterbahnen werden verschoben",
            "pistes déplacées",
            "piste si spostano",
            "本の配線が移動",
            "条走线移动",
            "pistas movem-se",
        ],
    ),
    (
        AssistMsg::Measure,
        [
            "Measure (U)",
            "Medir (U)",
            "Messen (U)",
            "Mesurer (U)",
            "Misura (U)",
            "測定 (U)",
            "测量 (U)",
            "Medir (U)",
        ],
    ),
    (
        AssistMsg::TooLarge,
        [
            "rules checked when released",
            "las reglas se comprueban al soltar",
            "Regeln werden beim Loslassen geprüft",
            "règles vérifiées au relâchement",
            "regole controllate al rilascio",
            "離すとルールを確認します",
            "松开后检查规则",
            "regras verificadas ao largar",
        ],
    ),
    (
        AssistMsg::Refused,
        [
            "cannot be placed here",
            "no se puede colocar aquí",
            "hier nicht möglich",
            "impossible à cet endroit",
            "non si può mettere qui",
            "ここには配置できません",
            "无法放在此处",
            "não pode ficar aqui",
        ],
    ),
    (
        AssistMsg::NextUndo,
        [
            "Undo",
            "Deshacer",
            "Rückgängig",
            "Annuler",
            "Annulla",
            "元に戻す",
            "撤销",
            "Anular",
        ],
    ),
    (
        AssistMsg::NextRedo,
        [
            "Redo",
            "Rehacer",
            "Wiederholen",
            "Rétablir",
            "Ripristina",
            "やり直し",
            "重做",
            "Refazer",
        ],
    ),
    (
        AssistMsg::History,
        [
            "Steps", "Pasos", "Schritte", "Étapes", "Passaggi", "手順", "步骤", "Passos",
        ],
    ),
];

impl AssistMsg {
    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        TEXTS
            .iter()
            .find(|(msg, _)| *msg == self)
            .map_or("", |(_, row)| row[crate::codes::column(lang)])
    }

    /// The text of a mode of the route.
    pub fn of_mode(mode: RouteMode) -> AssistMsg {
        match mode {
            RouteMode::Free => AssistMsg::Free,
            RouteMode::Stop => AssistMsg::Stop,
            RouteMode::Push => AssistMsg::Push,
        }
    }
}

const SNAPS: [(SnapTo, [&str; 8]); 6] = [
    (
        SnapTo::Pad,
        [
            "pad",
            "pad",
            "Pad",
            "pastille",
            "piazzola",
            "パッド",
            "焊盘",
            "ilha",
        ],
    ),
    (
        SnapTo::Via,
        ["via", "vía", "Via", "via", "via", "ビア", "过孔", "via"],
    ),
    (
        SnapTo::TrackEnd,
        [
            "end of track",
            "extremo de pista",
            "Leiterbahnende",
            "bout de piste",
            "estremo di pista",
            "配線の端",
            "走线端点",
            "ponta de pista",
        ],
    ),
    (
        SnapTo::Corner,
        [
            "corner", "esquina", "Ecke", "coin", "angolo", "角", "拐角", "canto",
        ],
    ),
    (
        SnapTo::Grid,
        [
            "grid",
            "rejilla",
            "Raster",
            "grille",
            "griglia",
            "グリッド",
            "栅格",
            "grelha",
        ],
    ),
    (
        SnapTo::Free,
        [
            "free point",
            "punto libre",
            "freier Punkt",
            "point libre",
            "punto libero",
            "任意の点",
            "任意点",
            "ponto livre",
        ],
    ),
];

/// The name of what a point of a measure went to.
pub fn snap_name(to: SnapTo, lang: Lang) -> &'static str {
    SNAPS
        .iter()
        .find(|(kind, _)| *kind == to)
        .map_or("", |(_, row)| row[crate::codes::column(lang)])
}

#[cfg(test)]
mod tests {
    use eh3d_phoenix_boardedit::{BoardEdit, BoardEditSettings, Tool};
    use eh3d_phoenix_route::RuleContext;

    use super::*;
    use crate::sample;

    fn state(settings: BoardEditSettings) -> EditState {
        let edit = BoardEdit::new(sample::board(), RuleContext::default(), settings).unwrap();
        EditState::new(edit)
    }

    fn component(state: &EditState) -> EditOp {
        let at = state.board().components[&sample::id(30)].at;
        EditOp::Move {
            id: sample::id(30),
            at: Point::from_nm(at.x_nm.get() + 1_000_000, at.y_nm.get()).unwrap(),
            rotation_udeg: None,
            follow: None,
        }
    }

    #[test]
    fn a_drag_is_checked_once_for_each_place_and_changes_nothing() {
        let s = state(BoardEditSettings::default());
        let before = s.board().to_bytes().unwrap();
        let mut check = DragCheck::default();
        assert_eq!(check.check(&s, None), None);
        let op = component(&s);
        for _ in 0..5 {
            let verdict = check.check(&s, Some(op.clone())).cloned();
            assert!(
                matches!(verdict, Some(DragVerdict::Checked(_))),
                "{verdict:?}"
            );
        }
        assert_eq!(check.computed(), 1);
        assert_eq!(s.board().to_bytes().unwrap(), before);
        assert_eq!(s.edit.steps(), 0);
        assert_eq!(check.check(&s, None), None);
        check.check(&s, Some(op));
        assert_eq!(check.computed(), 2);
    }

    #[test]
    fn a_drag_is_not_checked_when_the_settings_say_so_or_the_board_is_too_large() {
        let mut s = state(BoardEditSettings::default());
        let op = component(&s);
        let mut check = DragCheck::default();
        s.edit.settings.drag_drc_max_objects = 1;
        assert_eq!(
            check.check(&s, Some(op.clone())),
            Some(&DragVerdict::TooLarge)
        );
        s.edit.settings.drag_drc = false;
        assert_eq!(check.check(&s, Some(op)), None);
        assert_eq!(check.computed(), 0);
    }

    #[test]
    fn the_preview_of_a_route_is_computed_again_only_when_something_changed() {
        let mut s = state(BoardEditSettings::default());
        let mut preview = RoutePreview::default();
        preview.next_mode();
        assert_eq!(preview.mode, RouteMode::Stop);
        s.set_tool(Tool::Route);
        assert!(preview.at(&s, sample::point(20.0, 20.0)).is_none());
        s.route_net = Some(sample::id(10));
        s.route_points.push(sample::point(22.0, 17.0));
        let first = preview.at(&s, sample::point(26.0, 17.0)).cloned().unwrap();
        let count = preview.computed();
        // the same place of the grid: nothing is computed again
        preview.at(&s, sample::point(26.01, 17.0));
        assert_eq!(preview.computed(), count);
        preview.at(&s, sample::point(27.0, 17.0));
        assert_eq!(preview.computed(), count + 1);
        assert_eq!(first.points.first(), Some(&sample::point(22.0, 17.0)));
    }

    #[test]
    fn the_measure_reads_with_the_snap_and_goes_off_clean() {
        let s = state(BoardEditSettings::default());
        let mut tool = MeasureTool::default();
        tool.toggle();
        assert!(tool.on);
        assert_eq!(tool.line(None, Unit::Mm, 3, Lang::En), None);
        tool.click(&s, sample::point(1.0, 1.0), 0);
        let pointer = MeasurePoint {
            at: sample::point(4.0, 5.0),
            to: SnapTo::Free,
        };
        let line = tool.line(Some(pointer), Unit::Mm, 1, Lang::En);
        assert!(line.is_some_and(|l| l.contains("free point") && l.contains('°')));
        tool.toggle();
        assert_eq!(tool.measure, BoardMeasure::default());
    }

    #[test]
    fn the_history_names_the_steps_and_goes_to_any_of_them() {
        let mut s = state(BoardEditSettings::default());
        let start = s.board().to_bytes().unwrap();
        assert_eq!(history(&s, Lang::En, 10).next_undo(), None);
        let op = component(&s);
        assert!(s.run(&op), "{:?}", s.error);
        let view = history(&s, Lang::En, 10);
        assert!(view.next_undo().is_some_and(|l| !l.is_empty()));
        assert_eq!(view.next_redo(), None);
        assert_eq!(go(&mut s, view.go_to_start()), 1);
        assert_eq!(s.board().to_bytes().unwrap(), start);
        let view = history(&s, Lang::En, 10);
        assert!(view.next_redo().is_some());
        assert_eq!(go(&mut s, view.go_to(0)), 1);
        assert_eq!(go(&mut s, Move::Stay), 0);
        assert_eq!(go(&mut s, Move::Redo(5)), 0);
    }

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for (msg, row) in TEXTS {
            assert!(row.iter().all(|t| !t.is_empty()), "{msg:?}");
        }
        for (kind, row) in SNAPS {
            assert!(row.iter().all(|t| !t.is_empty()), "{kind:?}");
        }
        for lang in Lang::ALL {
            for mode in RouteMode::ALL {
                assert!(!AssistMsg::of_mode(mode).tr(lang).is_empty());
            }
        }
        assert!(!AssistMsg::Push.tr(Lang::Ja).is_ascii());
    }
}
