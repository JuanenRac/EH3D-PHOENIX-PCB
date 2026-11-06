// SPDX-License-Identifier: GPL-3.0-or-later
//! The keyboard shortcuts of the window as a table from action to keys, without a screen. The keys
//! that the window has always answered to are the defaults; a line `key.<action> = ctrl+z, ctrl+y`
//! in the settings changes the keys of one action and `none` leaves it without a key. Two actions
//! with the same key are an error that names the line. The list of shortcuts that the window shows
//! is made from this table, not written apart.

use eh3d_phoenix_rules::Lang;

use crate::editmsg::EditMsg;
use crate::msg::Msg;
use crate::window_text::WinText;
use crate::workspace::{Key, Shortcut};

/// A key of the keyboard that a shortcut may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyName {
    /// The letter A.
    A,
    /// The letter B.
    B,
    /// The letter C.
    C,
    /// The letter D.
    D,
    /// The letter E.
    E,
    /// The letter F.
    F,
    /// The letter G.
    G,
    /// The letter H.
    H,
    /// The letter I.
    I,
    /// The letter J.
    J,
    /// The letter K.
    K,
    /// The letter L.
    L,
    /// The letter M.
    M,
    /// The letter N.
    N,
    /// The letter O.
    O,
    /// The letter P.
    P,
    /// The letter Q.
    Q,
    /// The letter R.
    R,
    /// The letter S.
    S,
    /// The letter T.
    T,
    /// The letter U.
    U,
    /// The letter V.
    V,
    /// The letter W.
    W,
    /// The letter X.
    X,
    /// The letter Y.
    Y,
    /// The letter Z.
    Z,
    /// The digit 0.
    Num0,
    /// The digit 1.
    Num1,
    /// The digit 2.
    Num2,
    /// The digit 3.
    Num3,
    /// The digit 4.
    Num4,
    /// The digit 5.
    Num5,
    /// The digit 6.
    Num6,
    /// The digit 7.
    Num7,
    /// The digit 8.
    Num8,
    /// The digit 9.
    Num9,
    /// The function key F1.
    F1,
    /// The function key F2.
    F2,
    /// The function key F3.
    F3,
    /// The function key F4.
    F4,
    /// The function key F5.
    F5,
    /// The function key F6.
    F6,
    /// The function key F7.
    F7,
    /// The function key F8.
    F8,
    /// The function key F9.
    F9,
    /// The function key F10.
    F10,
    /// The function key F11.
    F11,
    /// The function key F12.
    F12,
    /// The Delete key.
    Delete,
    /// The Escape key.
    Escape,
    /// The plus sign.
    Plus,
    /// The minus sign.
    Minus,
    /// The Enter key.
    Enter,
    /// The Tab key.
    Tab,
    /// The space bar.
    Space,
    /// The arrow that points up.
    Up,
    /// The arrow that points down.
    Down,
    /// The arrow that points left.
    Left,
    /// The arrow that points right.
    Right,
    /// The Page up key.
    PageUp,
    /// The Page down key.
    PageDown,
    /// The Home key.
    Home,
    /// The End key.
    End,
    /// The slash.
    Slash,
    /// The comma.
    Comma,
    /// The full stop.
    Period,
    /// The Backspace key.
    Backspace,
}

/// The words that name the keys in the settings.
const KEY_NAMES: &[(&str, KeyName)] = &[
    ("a", KeyName::A),
    ("b", KeyName::B),
    ("c", KeyName::C),
    ("d", KeyName::D),
    ("e", KeyName::E),
    ("f", KeyName::F),
    ("g", KeyName::G),
    ("h", KeyName::H),
    ("i", KeyName::I),
    ("j", KeyName::J),
    ("k", KeyName::K),
    ("l", KeyName::L),
    ("m", KeyName::M),
    ("n", KeyName::N),
    ("o", KeyName::O),
    ("p", KeyName::P),
    ("q", KeyName::Q),
    ("r", KeyName::R),
    ("s", KeyName::S),
    ("t", KeyName::T),
    ("u", KeyName::U),
    ("v", KeyName::V),
    ("w", KeyName::W),
    ("x", KeyName::X),
    ("y", KeyName::Y),
    ("z", KeyName::Z),
    ("0", KeyName::Num0),
    ("1", KeyName::Num1),
    ("2", KeyName::Num2),
    ("3", KeyName::Num3),
    ("4", KeyName::Num4),
    ("5", KeyName::Num5),
    ("6", KeyName::Num6),
    ("7", KeyName::Num7),
    ("8", KeyName::Num8),
    ("9", KeyName::Num9),
    ("f1", KeyName::F1),
    ("f2", KeyName::F2),
    ("f3", KeyName::F3),
    ("f4", KeyName::F4),
    ("f5", KeyName::F5),
    ("f6", KeyName::F6),
    ("f7", KeyName::F7),
    ("f8", KeyName::F8),
    ("f9", KeyName::F9),
    ("f10", KeyName::F10),
    ("f11", KeyName::F11),
    ("f12", KeyName::F12),
    ("delete", KeyName::Delete),
    ("escape", KeyName::Escape),
    ("plus", KeyName::Plus),
    ("minus", KeyName::Minus),
    ("enter", KeyName::Enter),
    ("tab", KeyName::Tab),
    ("space", KeyName::Space),
    ("up", KeyName::Up),
    ("down", KeyName::Down),
    ("left", KeyName::Left),
    ("right", KeyName::Right),
    ("pageup", KeyName::PageUp),
    ("pagedown", KeyName::PageDown),
    ("home", KeyName::Home),
    ("end", KeyName::End),
    ("slash", KeyName::Slash),
    ("comma", KeyName::Comma),
    ("period", KeyName::Period),
    ("backspace", KeyName::Backspace),
];

impl KeyName {
    /// The word that names the key in the settings.
    pub fn word(self) -> &'static str {
        KEY_NAMES
            .iter()
            .find(|(_, key)| *key == self)
            .map_or("", |(word, _)| *word)
    }

    /// The key a word names, in any case.
    pub fn from_word(word: &str) -> Option<KeyName> {
        let lower = word.trim().to_ascii_lowercase();
        KEY_NAMES
            .iter()
            .find(|(name, _)| *name == lower)
            .map(|(_, key)| *key)
    }

    /// The name of the key as it is shown to the person.
    pub fn shown(self) -> String {
        match self {
            KeyName::Plus => "+".to_string(),
            KeyName::Minus => "-".to_string(),
            KeyName::Slash => "/".to_string(),
            KeyName::Comma => ",".to_string(),
            KeyName::Period => ".".to_string(),
            KeyName::PageUp => "Page up".to_string(),
            KeyName::PageDown => "Page down".to_string(),
            other => {
                let mut chars = other.word().chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            }
        }
    }
}

/// A key with the modifiers that must be held with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    /// The key.
    pub key: KeyName,
    /// Control (Command on macOS) must be held.
    pub ctrl: bool,
    /// Shift must be held.
    pub shift: bool,
}

impl Binding {
    /// A key with no modifier.
    pub fn plain(key: KeyName) -> Binding {
        Binding {
            key,
            ctrl: false,
            shift: false,
        }
    }

    /// Reads `ctrl+shift+z`: the modifiers `ctrl` and `shift` in any order and then the key, all in
    /// any case. The plus sign is written `plus`.
    pub fn parse(text: &str) -> Result<Binding, String> {
        let mut ctrl = false;
        let mut shift = false;
        let words: Vec<&str> = text.split('+').map(str::trim).collect();
        let Some((last, modifiers)) = words.split_last() else {
            return Err("empty shortcut".to_string());
        };
        for word in modifiers {
            match word.to_ascii_lowercase().as_str() {
                "ctrl" if !ctrl => ctrl = true,
                "shift" if !shift => shift = true,
                "ctrl" | "shift" => return Err(format!("{word} is repeated")),
                "" => return Err("an empty part (write plus for the plus sign)".to_string()),
                _ => return Err(format!("{word} is not a modifier (ctrl or shift)")),
            }
        }
        if last.is_empty() {
            return Err("a shortcut needs a key (write plus for the plus sign)".to_string());
        }
        let key = KeyName::from_word(last).ok_or_else(|| format!("{last} is not a key"))?;
        Ok(Binding { key, ctrl, shift })
    }

    /// The binding as the settings write it.
    pub fn text(&self) -> String {
        let mut out = String::new();
        if self.ctrl {
            out.push_str("ctrl+");
        }
        if self.shift {
            out.push_str("shift+");
        }
        out.push_str(self.key.word());
        out
    }

    /// The binding as it is shown to the person.
    pub fn shown(&self) -> String {
        let mut out = String::new();
        if self.ctrl {
            out.push_str("Ctrl+");
        }
        if self.shift {
            out.push_str("Shift+");
        }
        out.push_str(&self.key.shown());
        out
    }
}

/// Everything that a key can ask the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Undo the last edit.
    Undo,
    /// Redo the last edit that was undone.
    Redo,
    /// Ask where to save.
    Save,
    /// Turn the selected component.
    Rotate,
    /// Turn the selected component backwards.
    RotateBack,
    /// The select tool.
    ToolSelect,
    /// The measure tool.
    ToolMeasure,
    /// The tool that moves components.
    ToolMove,
    /// The tool that turns components.
    ToolRotate,
    /// The tool that deletes tracks.
    ToolDeleteTrack,
    /// The tool that moves vertices.
    ToolMoveVertex,
    /// The tool that changes widths.
    ToolWidth,
    /// The tool that adds vias.
    ToolVia,
    /// The tool that routes tracks.
    ToolRoute,
    /// Fit the drawing to the window.
    Fit,
    /// Delete the selected track.
    Delete,
    /// Cancel what is in progress and clear the selection.
    Cancel,
    /// Make the selected track wider.
    WidthUp,
    /// Make the selected track narrower.
    WidthDown,
    /// Go to the search box.
    Search,
    /// Go to the next result of the search.
    SearchNext,
    /// Go to the previous result of the search.
    SearchPrevious,
    /// Show the help of the mode.
    Help,
    /// Show the list of shortcuts.
    ShortcutsPanel,
    /// Show the history of the document.
    HistoryPanel,
    /// Show only the active layer, and then all again.
    SoloLayer,
    /// Highlight the net of the selection.
    HighlightNet,
    /// Delete every measure.
    ClearMeasures,
    /// Change the unit of the measures.
    ToggleUnit,
    /// Apply the first layer preset.
    Preset1,
    /// Apply the second layer preset.
    Preset2,
    /// Apply the third layer preset.
    Preset3,
    /// Apply the fourth layer preset.
    Preset4,
    /// Apply the fifth layer preset.
    Preset5,
    /// Apply the sixth layer preset.
    Preset6,
    /// Apply the seventh layer preset.
    Preset7,
    /// Apply the eighth layer preset.
    Preset8,
    /// Apply the ninth layer preset.
    Preset9,
}

/// Each action with the word that names it in the settings and its keys when nothing is changed.
const TABLE: &[(Action, &str, &str)] = &[
    (Action::Undo, "undo", "ctrl+z"),
    (Action::Redo, "redo", "ctrl+y, ctrl+shift+z"),
    (Action::Save, "save", "ctrl+s"),
    (Action::Rotate, "rotate", "r"),
    (Action::RotateBack, "rotate_back", "shift+r"),
    (Action::ToolSelect, "tool_select", "v"),
    (Action::ToolMeasure, "tool_measure", "m"),
    (Action::ToolMove, "tool_move", "g"),
    (Action::ToolRotate, "tool_rotate", "o"),
    (Action::ToolDeleteTrack, "tool_delete_track", "d"),
    (Action::ToolMoveVertex, "tool_move_vertex", "e"),
    (Action::ToolWidth, "tool_width", "w"),
    (Action::ToolVia, "tool_via", "i"),
    (Action::ToolRoute, "tool_route", "t"),
    (Action::Fit, "fit", "f"),
    (Action::Delete, "delete", "delete"),
    (Action::Cancel, "cancel", "escape"),
    (Action::WidthUp, "width_up", "plus"),
    (Action::WidthDown, "width_down", "minus"),
    (Action::Search, "search", "ctrl+f"),
    (Action::SearchNext, "search_next", "f3"),
    (Action::SearchPrevious, "search_previous", "shift+f3"),
    (Action::Help, "help", "f1"),
    (Action::ShortcutsPanel, "shortcuts", "ctrl+slash"),
    (Action::HistoryPanel, "history", "ctrl+h"),
    (Action::SoloLayer, "solo_layer", "l"),
    (Action::HighlightNet, "highlight_net", "n"),
    (Action::ClearMeasures, "clear_measures", "shift+m"),
    (Action::ToggleUnit, "toggle_unit", "u"),
    (Action::Preset1, "preset_1", "1"),
    (Action::Preset2, "preset_2", "2"),
    (Action::Preset3, "preset_3", "3"),
    (Action::Preset4, "preset_4", "4"),
    (Action::Preset5, "preset_5", "5"),
    (Action::Preset6, "preset_6", "6"),
    (Action::Preset7, "preset_7", "7"),
    (Action::Preset8, "preset_8", "8"),
    (Action::Preset9, "preset_9", "9"),
];

impl Action {
    /// Every action, in the order of the list of shortcuts.
    pub const ALL: [Action; 38] = [
        Action::Undo,
        Action::Redo,
        Action::Save,
        Action::Rotate,
        Action::RotateBack,
        Action::ToolSelect,
        Action::ToolMeasure,
        Action::ToolMove,
        Action::ToolRotate,
        Action::ToolDeleteTrack,
        Action::ToolMoveVertex,
        Action::ToolWidth,
        Action::ToolVia,
        Action::ToolRoute,
        Action::Fit,
        Action::Delete,
        Action::Cancel,
        Action::WidthUp,
        Action::WidthDown,
        Action::Search,
        Action::SearchNext,
        Action::SearchPrevious,
        Action::Help,
        Action::ShortcutsPanel,
        Action::HistoryPanel,
        Action::SoloLayer,
        Action::HighlightNet,
        Action::ClearMeasures,
        Action::ToggleUnit,
        Action::Preset1,
        Action::Preset2,
        Action::Preset3,
        Action::Preset4,
        Action::Preset5,
        Action::Preset6,
        Action::Preset7,
        Action::Preset8,
        Action::Preset9,
    ];

    fn row(self) -> (&'static str, &'static str) {
        TABLE
            .iter()
            .find(|(action, _, _)| *action == self)
            .map_or(("", ""), |(_, id, keys)| (*id, *keys))
    }

    fn index(self) -> usize {
        Action::ALL.iter().position(|a| *a == self).unwrap_or(0)
    }

    /// The word that names the action in the settings, after `key.`.
    pub fn id(self) -> &'static str {
        self.row().0
    }

    /// The action a word names.
    pub fn from_id(word: &str) -> Option<Action> {
        TABLE
            .iter()
            .find(|(_, id, _)| *id == word)
            .map(|(action, _, _)| *action)
    }

    /// The keys of the action when the settings do not change them, as the settings write them.
    pub fn default_keys(self) -> &'static str {
        self.row().1
    }

    /// For the actions that choose a layer preset, the place of the preset in the list from zero.
    pub fn preset_index(self) -> Option<usize> {
        match self {
            Action::Preset1 => Some(0),
            Action::Preset2 => Some(1),
            Action::Preset3 => Some(2),
            Action::Preset4 => Some(3),
            Action::Preset5 => Some(4),
            Action::Preset6 => Some(5),
            Action::Preset7 => Some(6),
            Action::Preset8 => Some(7),
            Action::Preset9 => Some(8),
            _ => None,
        }
    }

    /// What the editing of the workspace calls the same action, for the actions that it carries
    /// out itself; the others are carried out by the window.
    pub fn workspace_shortcut(self) -> Option<Shortcut> {
        let plain = |key| Some(Shortcut::plain(key));
        match self {
            Action::Undo => Some(Shortcut::ctrl(Key::Z)),
            Action::Redo => Some(Shortcut::ctrl(Key::Y)),
            Action::Save => Some(Shortcut::ctrl(Key::S)),
            Action::Rotate => plain(Key::R),
            Action::RotateBack => Some(Shortcut {
                key: Key::R,
                ctrl: false,
                shift: true,
            }),
            Action::ToolSelect => plain(Key::V),
            Action::ToolMeasure => plain(Key::M),
            Action::ToolMove => plain(Key::G),
            Action::ToolRotate => plain(Key::O),
            Action::ToolDeleteTrack => plain(Key::D),
            Action::ToolMoveVertex => plain(Key::E),
            Action::ToolWidth => plain(Key::W),
            Action::ToolVia => plain(Key::I),
            Action::ToolRoute => plain(Key::T),
            Action::Fit => plain(Key::F),
            Action::Delete => plain(Key::Delete),
            Action::Cancel => plain(Key::Escape),
            Action::WidthUp => plain(Key::Plus),
            Action::WidthDown => plain(Key::Minus),
            _ => None,
        }
    }

    /// The name of the action in a language, with the number for a layer preset.
    pub fn label(self, lang: Lang) -> String {
        let text = match self {
            Action::Undo => EditMsg::Undo.tr(lang),
            Action::Redo => EditMsg::Redo.tr(lang),
            Action::Save => EditMsg::SaveAs.tr(lang),
            Action::Rotate => WinText::ActRotate.tr(lang),
            Action::RotateBack => WinText::ActRotateBack.tr(lang),
            Action::ToolSelect => Msg::ToolSelect.tr(lang),
            Action::ToolMeasure => Msg::ToolMeasure.tr(lang),
            Action::ToolMove => EditMsg::ToolMove.tr(lang),
            Action::ToolRotate => EditMsg::ToolRotate.tr(lang),
            Action::ToolDeleteTrack => EditMsg::ToolDeleteTrack.tr(lang),
            Action::ToolMoveVertex => EditMsg::ToolMoveVertex.tr(lang),
            Action::ToolWidth => EditMsg::ToolWidth.tr(lang),
            Action::ToolVia => EditMsg::ToolVia.tr(lang),
            Action::ToolRoute => EditMsg::ToolRoute.tr(lang),
            Action::Fit => Msg::Fit.tr(lang),
            Action::Delete => WinText::ActDelete.tr(lang),
            Action::Cancel => Msg::ClearSelection.tr(lang),
            Action::WidthUp => WinText::ActWidthUp.tr(lang),
            Action::WidthDown => WinText::ActWidthDown.tr(lang),
            Action::Search => WinText::SearchTitle.tr(lang),
            Action::SearchNext => WinText::ActNext.tr(lang),
            Action::SearchPrevious => WinText::ActPrevious.tr(lang),
            Action::Help => WinText::HelpTitle.tr(lang),
            Action::ShortcutsPanel => WinText::ShortcutsTitle.tr(lang),
            Action::HistoryPanel => WinText::HistoryTitle.tr(lang),
            Action::SoloLayer => WinText::Solo.tr(lang),
            Action::HighlightNet => WinText::HighlightNet.tr(lang),
            Action::ClearMeasures => WinText::MeasureClearAll.tr(lang),
            Action::ToggleUnit => WinText::MeasureUnit.tr(lang),
            _ => WinText::PresetsTitle.tr(lang),
        };
        match self.preset_index() {
            Some(n) => format!("{text} {}", n + 1),
            None => text.to_string(),
        }
    }
}

/// Reads the keys of one action: bindings separated by commas, or `none` for no key.
fn parse_keys(value: &str) -> Result<Vec<Binding>, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    value.split(',').map(Binding::parse).collect()
}

/// The keys of every action.
#[derive(Debug, Clone, PartialEq)]
pub struct Keymap {
    lists: Vec<Vec<Binding>>,
}

impl Default for Keymap {
    fn default() -> Self {
        Keymap {
            lists: Action::ALL
                .iter()
                .map(|action| parse_keys(action.default_keys()).unwrap_or_default())
                .collect(),
        }
    }
}

impl Keymap {
    /// The map with the keys that the settings change. Each change is the line of the settings, the
    /// word after `key.` and the value. An action that is not known, a key that is not valid, an
    /// action given twice and two actions with the same key are errors; the answer carries the line
    /// of the change that is at fault.
    pub fn with_changes(changes: &[(usize, String, String)]) -> Result<Keymap, (usize, String)> {
        let mut keymap = Keymap::default();
        let mut lines = [0usize; Action::ALL.len()];
        for (line, id, value) in changes {
            let action =
                Action::from_id(id).ok_or_else(|| (*line, format!("key.{id}: unknown action")))?;
            let keys = parse_keys(value).map_err(|why| (*line, format!("key.{id}: {why}")))?;
            let at = action.index();
            if lines[at] != 0 {
                return Err((*line, format!("key.{id} is given twice")));
            }
            keymap.lists[at] = keys;
            lines[at] = *line;
        }
        let mut seen: Vec<(Binding, usize)> = Vec::new();
        for (at, list) in keymap.lists.iter().enumerate() {
            for binding in list {
                if let Some((_, other)) = seen.iter().find(|(known, _)| known == binding) {
                    let line = lines[*other].max(lines[at]);
                    return Err((
                        line,
                        format!(
                            "{} is the key of {} and of {}",
                            binding.text(),
                            Action::ALL[*other].id(),
                            Action::ALL[at].id()
                        ),
                    ));
                }
                seen.push((*binding, at));
            }
        }
        Ok(keymap)
    }

    /// The keys of an action.
    pub fn keys(&self, action: Action) -> &[Binding] {
        &self.lists[action.index()]
    }

    /// The action a key asks for, when there is one.
    pub fn action_for(&self, binding: Binding) -> Option<Action> {
        self.lists
            .iter()
            .position(|list| list.contains(&binding))
            .map(|at| Action::ALL[at])
    }

    /// The list of shortcuts for the window: the name of each action in a language and its keys.
    pub fn rows(&self, lang: Lang) -> Vec<(String, String)> {
        Action::ALL
            .iter()
            .map(|action| {
                let keys = self.keys(*action);
                let shown = if keys.is_empty() {
                    WinText::NoKey.tr(lang).to_string()
                } else {
                    keys.iter()
                        .map(Binding::shown)
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                (action.label(lang), shown)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind(text: &str) -> Binding {
        Binding::parse(text).unwrap()
    }

    fn change(line: usize, id: &str, value: &str) -> (usize, String, String) {
        (line, id.to_string(), value.to_string())
    }

    #[test]
    fn the_table_has_one_row_for_each_action_in_order_with_unique_words() {
        assert_eq!(TABLE.len(), Action::ALL.len());
        for (row, action) in TABLE.iter().zip(Action::ALL) {
            assert_eq!(row.0, action);
        }
        for (i, a) in TABLE.iter().enumerate() {
            for b in &TABLE[i + 1..] {
                assert_ne!(a.1, b.1);
            }
        }
        for action in Action::ALL {
            assert_eq!(Action::from_id(action.id()), Some(action));
            assert_eq!(Action::ALL[action.index()], action);
        }
        assert_eq!(Action::from_id("nothing"), None);
    }

    #[test]
    fn every_key_name_round_trips_and_is_listed_once() {
        for (i, (word, key)) in KEY_NAMES.iter().enumerate() {
            assert_eq!(KeyName::from_word(word), Some(*key));
            assert_eq!(key.word(), *word);
            assert!(!key.shown().is_empty());
            for (other, _) in &KEY_NAMES[i + 1..] {
                assert_ne!(word, other);
            }
        }
        assert_eq!(KeyName::from_word(" F3 "), Some(KeyName::F3));
        assert_eq!(KeyName::from_word("nothing"), None);
        assert_eq!(KeyName::Plus.shown(), "+");
        assert_eq!(KeyName::Delete.shown(), "Delete");
        assert_eq!(KeyName::Z.shown(), "Z");
        assert_eq!(KeyName::PageUp.shown(), "Page up");
    }

    #[test]
    fn a_binding_is_read_in_any_order_and_any_case_and_shown_back() {
        assert_eq!(
            bind("Ctrl+Shift+Z"),
            Binding {
                key: KeyName::Z,
                ctrl: true,
                shift: true
            }
        );
        assert_eq!(bind("shift+ctrl+z"), bind("ctrl+shift+z"));
        assert_eq!(bind(" f1 "), Binding::plain(KeyName::F1));
        assert_eq!(bind("ctrl+shift+z").text(), "ctrl+shift+z");
        assert_eq!(bind("ctrl+shift+z").shown(), "Ctrl+Shift+Z");
        assert_eq!(bind("ctrl+slash").shown(), "Ctrl+/");
        for bad in [
            "",
            "+",
            "ctrl+",
            "ctrl++z",
            "ctrl+ctrl+z",
            "alt+z",
            "z+ctrl",
            "ctrl+nothing",
            "shift",
        ] {
            assert!(Binding::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_defaults_have_no_two_actions_on_one_key() {
        let keymap = Keymap::default();
        let mut seen: Vec<Binding> = Vec::new();
        for action in Action::ALL {
            for binding in keymap.keys(action) {
                assert!(!seen.contains(binding), "{binding:?}");
                seen.push(*binding);
            }
        }
        assert_eq!(Keymap::with_changes(&[]).unwrap(), keymap);
        // every action but none has a key
        assert!(Action::ALL.iter().all(|a| !keymap.keys(*a).is_empty()));
    }

    #[test]
    fn the_keys_of_the_window_before_the_table_are_the_defaults() {
        let keymap = Keymap::default();
        let want = [
            ("ctrl+z", Action::Undo),
            ("ctrl+y", Action::Redo),
            ("ctrl+shift+z", Action::Redo),
            ("ctrl+s", Action::Save),
            ("r", Action::Rotate),
            ("shift+r", Action::RotateBack),
            ("v", Action::ToolSelect),
            ("m", Action::ToolMeasure),
            ("g", Action::ToolMove),
            ("o", Action::ToolRotate),
            ("d", Action::ToolDeleteTrack),
            ("e", Action::ToolMoveVertex),
            ("w", Action::ToolWidth),
            ("i", Action::ToolVia),
            ("t", Action::ToolRoute),
            ("f", Action::Fit),
            ("delete", Action::Delete),
            ("escape", Action::Cancel),
            ("plus", Action::WidthUp),
            ("minus", Action::WidthDown),
        ];
        for (keys, action) in want {
            assert_eq!(keymap.action_for(bind(keys)), Some(action), "{keys}");
        }
        assert_eq!(keymap.action_for(bind("ctrl+g")), None);
        assert_eq!(keymap.action_for(bind("shift+g")), None);
    }

    #[test]
    fn the_actions_of_the_workspace_map_to_its_own_shortcuts_and_the_others_to_none() {
        let carried = Action::ALL
            .iter()
            .filter(|a| a.workspace_shortcut().is_some())
            .count();
        assert_eq!(carried, 19);
        assert_eq!(
            Action::Undo.workspace_shortcut(),
            Some(Shortcut::ctrl(Key::Z))
        );
        assert_eq!(
            Action::RotateBack.workspace_shortcut(),
            Some(Shortcut {
                key: Key::R,
                ctrl: false,
                shift: true
            })
        );
        assert_eq!(
            Action::Delete.workspace_shortcut(),
            Some(Shortcut::plain(Key::Delete))
        );
        for action in [
            Action::Search,
            Action::Help,
            Action::HistoryPanel,
            Action::SoloLayer,
            Action::Preset3,
        ] {
            assert_eq!(action.workspace_shortcut(), None);
        }
    }

    #[test]
    fn a_change_replaces_the_keys_of_one_action_and_leaves_the_rest() {
        let keymap = Keymap::with_changes(&[
            change(3, "undo", "ctrl+u"),
            change(4, "search", "ctrl+k, f4"),
            change(5, "help", "none"),
        ])
        .unwrap();
        assert_eq!(keymap.keys(Action::Undo), [bind("ctrl+u")]);
        assert_eq!(keymap.keys(Action::Search), [bind("ctrl+k"), bind("f4")]);
        assert!(keymap.keys(Action::Help).is_empty());
        assert_eq!(keymap.action_for(bind("ctrl+z")), None);
        assert_eq!(keymap.action_for(bind("ctrl+u")), Some(Action::Undo));
        assert_eq!(keymap.action_for(bind("f1")), None);
        assert_eq!(
            keymap.keys(Action::Fit),
            Keymap::default().keys(Action::Fit)
        );
    }

    #[test]
    fn a_key_given_to_two_actions_is_an_error_that_names_the_line() {
        // a default key taken by a change
        let (line, why) = Keymap::with_changes(&[change(7, "fit", "v")]).unwrap_err();
        assert_eq!(line, 7);
        assert!(why.contains("fit") && why.contains("tool_select"), "{why}");
        // two changes that give the same key: the later line is at fault
        let (line, why) =
            Keymap::with_changes(&[change(2, "undo", "ctrl+u"), change(9, "redo", "ctrl+u")])
                .unwrap_err();
        assert_eq!(line, 9);
        assert!(why.contains("ctrl+u"), "{why}");
        // a key twice in one action
        let (line, _) = Keymap::with_changes(&[change(4, "undo", "ctrl+u, ctrl+u")]).unwrap_err();
        assert_eq!(line, 4);
        // freeing a key first makes it free for another action
        let both = Keymap::with_changes(&[change(1, "tool_select", "none"), change(2, "fit", "v")]);
        assert!(both.is_ok());
    }

    #[test]
    fn a_bad_change_is_an_error_with_its_line_and_never_a_panic() {
        for (id, value) in [
            ("nothing", "z"),
            ("undo", "ctrl+nothing"),
            ("undo", ""),
            ("undo", "ctrl+z,"),
            ("undo", "alt+z"),
        ] {
            let (line, why) = Keymap::with_changes(&[change(6, id, value)]).unwrap_err();
            assert_eq!(line, 6, "{id} = {value}");
            assert!(why.starts_with(&format!("key.{id}")), "{why}");
        }
        let (line, _) =
            Keymap::with_changes(&[change(1, "undo", "ctrl+u"), change(2, "undo", "ctrl+i")])
                .unwrap_err();
        assert_eq!(line, 2);
    }

    #[test]
    fn the_list_of_shortcuts_is_made_from_the_table_in_every_language() {
        let keymap = Keymap::with_changes(&[change(1, "help", "none")]).unwrap();
        for lang in Lang::ALL {
            let rows = keymap.rows(lang);
            assert_eq!(rows.len(), Action::ALL.len());
            for (label, keys) in &rows {
                assert!(!label.is_empty() && !keys.is_empty(), "{lang:?}");
            }
        }
        let rows = keymap.rows(Lang::En);
        assert_eq!(rows[1].1, "Ctrl+Y, Ctrl+Shift+Z");
        assert_eq!(rows[22].1, WinText::NoKey.tr(Lang::En));
        assert!(rows[37].0.ends_with(" 9"), "{}", rows[37].0);
        assert!(rows[29].0.ends_with(" 1"), "{}", rows[29].0);
    }

    #[test]
    fn the_guide_lists_every_action_with_its_default_keys() {
        let guide = include_str!("../../../docs/SHORTCUTS.md");
        for action in Action::ALL {
            let start = format!("| `{}` |", action.id());
            let row = guide
                .lines()
                .find(|l| l.starts_with(&start))
                .unwrap_or_else(|| panic!("{} is not in the guide", action.id()));
            let keys: Vec<String> = parse_keys(action.default_keys())
                .unwrap()
                .iter()
                .map(Binding::shown)
                .collect();
            assert!(row.ends_with(&format!("| {} |", keys.join(", "))), "{row}");
        }
    }
}
