// SPDX-License-Identifier: GPL-3.0-or-later
//! The panels, the workspaces and the menus of the window as data, with no window in them: which
//! panels there are and what each is called in the eight languages, the factory arrangement of each
//! workspace (by task: Edit, Review and Compact), the menu bar and which of its entries can be chosen
//! at a given moment, and the rule that decides when the arrangement is written to the file of the
//! preferences. The window (`gui`) draws these with the frame of the suite (`eh3d-dock`) and only does
//! what they say.
//!
//! There are no levels of experience: one arrangement for each kind of work, and every panel can be
//! taken away, put back, moved, tabbed, floated or folded by the person.

use eh3d_dock::about::{AboutBrand, Phoenix};
use eh3d_dock::core::PanelDef;
use eh3d_dock::geom::Side;
use eh3d_dock::menu::{Entry, Item, Label, Menu, MenuBar, StdAction};
use eh3d_dock::model::{Layout, Node};
use eh3d_dock::sectors::{titles, Sector};
use eh3d_dock::shortcuts::Shortcut;
use eh3d_dock::stdmenus::{
    edit_menu, file_menu, help_menu, tools_menu, view_menu, window_menu, HelpOptions,
};
use eh3d_dock::texts::Text8;
use eh3d_dock::workspaces::Workspace;

/// The look of the window «About»: which program of the family this is and its own picture, drawn in a ring of its colour.
pub fn about_brand() -> AboutBrand {
    AboutBrand {
        program: Phoenix::Pcb,
        logo_png: include_bytes!("../../../images/brand/icon-256.png"),
    }
}
use eh3d_phoenix_boardedit::actions::EditAction;
use eh3d_phoenix_rules::Lang;
use eh3d_view::texts::Lang as ViewLang;

use crate::board_actions_text::ActionMsg;
use crate::cfg_text::CfgMsg;
use crate::editmsg::EditMsg;
use crate::gerber_text::GerberText;
use crate::library_text::LibText;
use crate::msg::Msg;
use crate::panel_text::PanelText;
use crate::view3d_text::View3dText;
use crate::window_text::WinText;

/// The names of the panels (what the file of the preferences stores, so they never change).
pub mod id {
    /// The board or the schematic, drawn; the start screen while nothing is open.
    pub const BOARD_VIEW: &str = "board_view";
    /// The layers of the document: shown or hidden, with their colours.
    pub const LAYERS: &str = "layers";
    /// The tools and the settings of the tool in use.
    pub const PROPERTIES: &str = "properties";
    /// The components of the board, or the parts of the schematic, and the counts of the objects.
    pub const COMPONENTS: &str = "components";
    /// The nets, with the connections that are still to be routed.
    pub const NETS: &str = "nets";
    /// The problems of the rules of the board, or the electrical rule check of the schematic.
    pub const DRC: &str = "drc";
    /// The libraries of footprints, symbols and devices.
    pub const LIBRARY: &str = "library";
    /// What the window has to say: the notes, the errors and the answers of the checks.
    pub const CONSOLE: &str = "console";
    /// The report of the document: its counts and the state of the rules.
    pub const REPORT: &str = "report";
    /// The file of the document and the tools of the window.
    pub const TASKS: &str = "tasks";
    /// The editor of a library of footprints, symbols and devices.
    pub const LIBRARY_EDITOR: &str = "library_editor";
    /// The imported Gerber and Excellon files of a folder.
    pub const GERBER: &str = "gerber";
    /// The 3D view of the assembled board.
    pub const VIEW3D: &str = "view3d";
    /// The panel of boards laid out for manufacturing.
    pub const BOARDS: &str = "boards";

    /// Every panel, in the order of the View menu.
    pub const ALL: [&str; 14] = [
        BOARD_VIEW,
        LAYERS,
        PROPERTIES,
        COMPONENTS,
        NETS,
        DRC,
        LIBRARY,
        CONSOLE,
        REPORT,
        TASKS,
        LIBRARY_EDITOR,
        GERBER,
        VIEW3D,
        BOARDS,
    ];

    /// The panels that hold a mode of their own: they are hidden until the person opens what they
    /// show (a library, a folder of Gerber files, the 3D view or a panel of boards), and they come
    /// back to the place where they were when they are opened again.
    pub const MODES: [&str; 4] = [LIBRARY_EDITOR, GERBER, VIEW3D, BOARDS];
}

/// The workspace of the design: the components and the nets on the left, the board in the middle, the
/// tools and the layers on the right and the rules below.
pub const WS_EDIT: &str = "Edit";
/// The workspace of the review before manufacturing: the board with the rules, the report and the layers.
pub const WS_REVIEW: &str = "Review";
/// The workspace of the board alone, with the layers and the tools folded into the edges.
pub const WS_COMPACT: &str = "Compact";

/// A text of the window as the text of the frame: the eight languages in the order that the frame
/// wants (English, Spanish, Italian, German, French, Portuguese of Portugal, Chinese, Japanese).
pub fn t8(text: impl Fn(Lang) -> &'static str) -> Text8 {
    Text8([
        text(Lang::En),
        text(Lang::Es),
        text(Lang::It),
        text(Lang::De),
        text(Lang::Fr),
        text(Lang::Pt),
        text(Lang::Zh),
        text(Lang::Ja),
    ])
}

/// The language of the frame for a language of the window.
pub fn view_lang(lang: Lang) -> ViewLang {
    match lang {
        Lang::En => ViewLang::En,
        Lang::Es => ViewLang::Es,
        Lang::De => ViewLang::De,
        Lang::Fr => ViewLang::Fr,
        Lang::It => ViewLang::It,
        Lang::Ja => ViewLang::Ja,
        Lang::Zh => ViewLang::Zh,
        Lang::Pt => ViewLang::Pt,
    }
}

/// The panels: the ones the suite proposes for a circuit board program, and the two that this program
/// adds (the report and the tasks).
pub fn defs() -> Vec<PanelDef> {
    let mut v = Sector::Pcb.panels();
    v.push(
        PanelDef::new(id::REPORT, titles::REPORT)
            .icon("R")
            .min(200.0, 100.0),
    );
    v.push(
        PanelDef::new(id::TASKS, titles::TASKS)
            .icon("K")
            .min(220.0, 140.0),
    );
    v.push(
        PanelDef::new(id::LIBRARY_EDITOR, t8(|l| LibText::Title.tr(l)))
            .icon("E")
            .min(420.0, 240.0),
    );
    v.push(
        PanelDef::new(id::GERBER, t8(|l| GerberText::Title.tr(l)))
            .icon("G")
            .min(420.0, 240.0),
    );
    v.push(
        PanelDef::new(id::VIEW3D, t8(|l| View3dText::Title.tr(l)))
            .icon("3")
            .min(420.0, 240.0),
    );
    v.push(
        PanelDef::new(id::BOARDS, t8(|l| PanelText::Title.tr(l)))
            .icon("P")
            .min(420.0, 240.0),
    );
    v
}

fn ws(name: &str, root: Node) -> Workspace {
    Workspace::new(name, Layout::with_root(root))
}

/// The factory workspaces, by task and not by experience. The four panels of the modes of their own
/// (`id::MODES`) are in none of them: they are hidden until the person opens what they show, and then
/// they join the group of the first panel, so the board stays on the screen beside them.
///
/// * `Edit`: components, nets, library and tasks on the left; the board in the middle with the
///   console, the problems and the report below; the tools above the layers on the right.
/// * `Review`: the board with the problems, the report and the console below; the layers, the
///   properties and the nets on the right. Nothing is there to edit with.
/// * `Compact`: the board alone; the layers and the tools fold into the left and the right edge and the
///   problems into the bottom one.
pub fn workspaces() -> Vec<Workspace> {
    let t = |ids: &[&str]| Node::tabs(ids);
    vec![
        ws(
            WS_EDIT,
            Node::row(
                0.2,
                t(&[id::COMPONENTS, id::NETS, id::LIBRARY, id::TASKS]),
                Node::row(
                    0.76,
                    Node::column(
                        0.78,
                        t(&[id::BOARD_VIEW]),
                        t(&[id::CONSOLE, id::DRC, id::REPORT]),
                    ),
                    Node::column(0.55, t(&[id::PROPERTIES]), t(&[id::LAYERS])),
                ),
            ),
        ),
        ws(
            WS_REVIEW,
            Node::row(
                0.72,
                Node::column(
                    0.6,
                    t(&[id::BOARD_VIEW]),
                    t(&[id::DRC, id::REPORT, id::CONSOLE]),
                ),
                Node::column(0.45, t(&[id::LAYERS]), t(&[id::PROPERTIES, id::NETS])),
            ),
        ),
        Workspace::new(
            WS_COMPACT,
            Layout::with_root(t(&[id::BOARD_VIEW]))
                .with_auto_hidden(id::LAYERS, Side::Left)
                .with_auto_hidden(id::PROPERTIES, Side::Right)
                .with_auto_hidden(id::DRC, Side::Bottom),
        ),
    ]
}

/// The name of a workspace a word of a command line stands for (`edit`, `review`, `compact`).
pub fn workspace_named(word: &str) -> Option<&'static str> {
    [WS_EDIT, WS_REVIEW, WS_COMPACT]
        .into_iter()
        .find(|name| name.eq_ignore_ascii_case(word.trim()))
}

/// What the program does when an entry of its menus is chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    /// An action every program has (New, Open, Save, Quit...).
    Std(StdAction),
    /// Look at the whole drawing.
    Fit,
    /// Let go of the selection and cancel what is in progress.
    ClearSelection,
    /// Run the electrical rule check of the schematic.
    RunErc,
    /// Open the 3D window of the board.
    View3d,
    /// Open the editor of the settings files.
    SettingsFiles,
    /// The search box.
    Search,
    /// The list of measures.
    Measures,
    /// The layer presets.
    LayerPresets,
    /// The history of the steps.
    History,
    /// The list of shortcuts of the window.
    Shortcuts,
    /// An action of the board editor on what is selected (tidy, join, snap, widths, swap).
    BoardAction(EditAction),
}

impl From<StdAction> for Cmd {
    fn from(a: StdAction) -> Cmd {
        Cmd::Std(a)
    }
}

/// The entries whose key is the key of an action of the program (the keys of the program are the
/// master: they are handled by the window, so the entry shows no key of its own and cannot fire twice).
const KEYS_OF_THE_PROGRAM: [&str; 4] = ["edit.undo", "edit.redo", "file.save", "help.online"];

fn own(id: &str, text: impl Fn(Lang) -> &'static str, cmd: Cmd) -> Entry<Cmd> {
    Entry::app(id, Label::Own(t8(text)), cmd)
}

/// The menu bar: File, Edit, View, Tools, Window and Help.
pub fn menu_bar() -> MenuBar<Cmd> {
    let file: Menu<Cmd> = file_menu(true);
    let mut edit: Menu<Cmd> = edit_menu();
    let mut view: Menu<Cmd> = view_menu();
    let mut tools: Menu<Cmd> = tools_menu();
    let window: Menu<Cmd> = window_menu();
    let mut help: Menu<Cmd> = help_menu(HelpOptions {
        online_help: true,
        check_updates: false,
    });
    edit = edit.sep().entry(own(
        "edit.clear_selection",
        |l| Msg::ClearSelection.tr(l),
        Cmd::ClearSelection,
    ));
    edit = edit.sep();
    for action in EditAction::ALL {
        edit = edit.entry(own(
            &format!("edit.{}", action.key()),
            move |l| ActionMsg::of(action).tr(l),
            Cmd::BoardAction(action),
        ));
    }
    view = view
        .sep()
        .entry(own("view.fit", |l| Msg::Fit.tr(l), Cmd::Fit))
        .entry(own(
            "view.measures",
            |l| WinText::MeasureTitle.tr(l),
            Cmd::Measures,
        ))
        .entry(own(
            "view.presets",
            |l| WinText::PresetsTitle.tr(l),
            Cmd::LayerPresets,
        ))
        .entry(own(
            "view.history",
            |l| WinText::HistoryTitle.tr(l),
            Cmd::History,
        ));
    tools = tools
        .entry(own(
            "tools.search",
            |l| WinText::SearchTitle.tr(l),
            Cmd::Search,
        ))
        .entry(own("tools.erc", |l| EditMsg::RunErc.tr(l), Cmd::RunErc))
        .entry(own(
            "tools.view3d",
            |l| View3dText::Show3d.tr(l),
            Cmd::View3d,
        ))
        .sep()
        .entry(own(
            "tools.settings_files",
            |l| CfgMsg::Title.tr(l),
            Cmd::SettingsFiles,
        ));
    // Help: the list of shortcuts is the one of the window, which knows the keys of the program
    for item in &mut help.items {
        if let Item::Entry(e) = item {
            if e.id == "help.shortcuts" {
                *e = own(
                    "help.shortcuts",
                    |l| WinText::ShortcutsTitle.tr(l),
                    Cmd::Shortcuts,
                );
            }
        }
    }
    let mut menus = vec![file, edit, view, tools, window, help];
    for menu in &mut menus {
        for item in &mut menu.items {
            if let Item::Entry(e) = item {
                if KEYS_OF_THE_PROGRAM.contains(&e.id.as_str()) {
                    e.shortcut = None;
                }
            }
        }
    }
    // the keys of the program that have an entry in a menu show it beside the text of the entry
    let search_key = Shortcut::ctrl("F");
    for menu in &mut menus {
        for item in &mut menu.items {
            if let Item::Entry(e) = item {
                if e.id == "tools.search" {
                    e.shortcut = Some(search_key.clone());
                }
            }
        }
    }
    MenuBar::new(menus)
}

/// What the window knows when it decides which entries of the menus can be chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuFacts {
    /// A board or a schematic is open.
    pub has_document: bool,
    /// The document is a board.
    pub is_board: bool,
    /// An editor of the board or the schematic has the document.
    pub editing: bool,
    /// The editor that has the document keeps its own steps and the entries Undo and Redo of the
    /// window work on them.
    pub editor_history: bool,
    /// There is a step to undo.
    pub can_undo: bool,
    /// There is a step to redo.
    pub can_redo: bool,
    /// The actions of the board editor that can be chosen now: the bit of the place of the action
    /// in `EditAction::ALL` is set.
    pub actions: u16,
}

/// The ids of the entries that cannot be chosen now. The ones for things the program does not have (cut,
/// copy, paste) are always off. An entry that is off fires no key and no click.
pub fn disabled_entries(f: &MenuFacts) -> Vec<String> {
    let mut off: Vec<&str> = vec!["edit.cut", "edit.copy", "edit.paste"];
    let mut when = |cond: bool, ids: &[&'static str]| {
        if cond {
            off.extend_from_slice(ids);
        }
    };
    let document = f.has_document;
    when(
        !document,
        &["file.export", "view.fit", "edit.clear_selection"],
    );
    when(!document || f.editing, &["file.save", "file.save_as"]);
    let window_history = f.editing && !f.editor_history;
    when(!document || window_history || !f.can_undo, &["edit.undo"]);
    when(!document || window_history || !f.can_redo, &["edit.redo"]);
    when(
        !document,
        &["view.measures", "view.presets", "view.history"],
    );
    when(!document || f.is_board || f.editing, &["tools.erc"]);
    when(!document || !f.is_board, &["tools.view3d"]);
    let mut v: Vec<String> = off.into_iter().map(str::to_string).collect();
    for (i, action) in EditAction::ALL.iter().enumerate() {
        if f.actions & (1 << i) == 0 {
            v.push(format!("edit.{}", action.key()));
        }
    }
    v.sort();
    v.dedup();
    v
}

/// Decides when the arrangement of the panels is written to the file of the preferences: when the lines
/// of the frame are not the ones last written (or read). It writes nothing when nothing changed, so
/// dragging a panel writes once and a quiet window writes never.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayoutWatch {
    saved: String,
}

impl LayoutWatch {
    /// A watch that knows the lines of the frame as they were read.
    pub fn new(pairs: &[(String, String)]) -> LayoutWatch {
        LayoutWatch {
            saved: LayoutWatch::flat(pairs),
        }
    }

    fn flat(pairs: &[(String, String)]) -> String {
        pairs.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
    }

    /// Whether the lines changed since the last time this answered yes (or since the start); a yes
    /// counts them as written.
    pub fn changed(&mut self, pairs: &[(String, String)]) -> bool {
        let now = LayoutWatch::flat(pairs);
        if now == self.saved {
            return false;
        }
        self.saved = now;
        true
    }
}

/// The lines of the frame as the text it reads (`key = value`, one for each line).
pub fn dock_text(pairs: &[(String, String)]) -> String {
    pairs.iter().map(|(k, v)| format!("{k} = {v}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{Action, Keymap};
    use eh3d_dock::core::DockCore;
    use eh3d_dock::menu::MenuContext;
    use eh3d_dock::sectors::workspace_title;

    fn core() -> DockCore {
        DockCore::new(defs(), workspaces())
    }

    fn entries(bar: &MenuBar<Cmd>) -> Vec<&Entry<Cmd>> {
        fn walk<'a>(items: &'a [Item<Cmd>], out: &mut Vec<&'a Entry<Cmd>>) {
            for i in items {
                match i {
                    Item::Entry(e) => out.push(e),
                    Item::Submenu(m) => walk(&m.items, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for m in &bar.menus {
            walk(&m.items, &mut out);
        }
        out
    }

    fn entry<'a>(bar: &'a MenuBar<Cmd>, id: &str) -> &'a Entry<Cmd> {
        entries(bar)
            .into_iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no entry {id}"))
    }

    /// A generator of numbers with a fixed seed, so a test that tries many cases tries the same ones every time.
    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed >> 33
    }

    #[test]
    fn the_panels_have_names_that_hold_and_titles_in_the_eight_languages() {
        let d = defs();
        assert_eq!(d.len(), 14);
        let names: Vec<&str> = d.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(names, id::ALL.to_vec());
        for p in &d {
            assert!(
                p.id.len() <= 48
                    && p.id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{}",
                p.id
            );
            for t in p.title.0 {
                assert!(!t.trim().is_empty(), "{}", p.id);
            }
        }
        let report = d.iter().find(|p| p.id == id::REPORT).unwrap();
        assert_eq!(report.title.0[1], "Informe");
        assert_eq!(report.title.0[5], "Relatório");
        let tasks = d.iter().find(|p| p.id == id::TASKS).unwrap();
        assert_eq!(tasks.title.0[7], "タスク");
    }

    #[test]
    fn the_panels_of_the_sector_are_all_there_and_none_is_repeated() {
        let d = defs();
        for p in Sector::Pcb.panels() {
            assert!(d.iter().any(|x| x.id == p.id), "{}", p.id);
        }
        let mut seen = std::collections::BTreeSet::new();
        for p in &d {
            assert!(seen.insert(p.id.clone()), "{} twice", p.id);
        }
    }

    #[test]
    fn the_factory_arrangement_is_sound_and_every_workspace_has_a_name_in_eight_languages() {
        let c = core();
        assert_eq!(c.workspaces().names(), vec![WS_EDIT, WS_REVIEW, WS_COMPACT]);
        assert!(c.warnings().is_empty(), "{:?}", c.warnings());
        assert!(c.layout_is_sound());
        for name in c.workspaces().names() {
            assert!(workspace_title(name).is_some(), "{name}");
        }
        // every panel of the document is somewhere in a workspace (shown or folded), so the View menu
        // can bring any of them back from a factory layout
        for p in id::ALL.iter().filter(|p| !id::MODES.contains(p)) {
            let shown = c
                .workspaces()
                .factory()
                .iter()
                .any(|w| w.layout.is_shown(p));
            assert!(shown, "{p} is in no workspace");
        }
        // the panels of the modes are hidden in all of them, and each can be shown from the View menu
        for p in id::MODES {
            for w in c.workspaces().factory() {
                assert!(!w.layout.is_shown(p), "{} shows {p}", w.name);
                assert!(w.layout.contains(p), "{} does not know {p}", w.name);
            }
        }
    }

    #[test]
    fn a_panel_of_a_mode_opens_beside_the_board_and_closes_back_to_hidden() {
        for name in [WS_EDIT, WS_REVIEW, WS_COMPACT] {
            let mut c = core();
            c.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(
                name.to_string(),
            ))
            .unwrap();
            for p in id::MODES {
                c.apply(&eh3d_dock::menu::DockAction::ShowPanel(p.to_string()))
                    .unwrap();
                assert!(c.layout().is_shown(p), "{name}: {p}");
                assert!(c.layout_is_sound(), "{name}: {p}");
                assert!(c.layout().is_shown(id::BOARD_VIEW), "{name}: {p}");
                assert_eq!(c.focus(), Some(p));
            }
            for p in id::MODES {
                c.apply(&eh3d_dock::menu::DockAction::HidePanel(p.to_string()))
                    .unwrap();
                assert!(!c.layout().is_shown(p), "{name}: {p}");
            }
            assert!(c.layout_is_sound(), "{name}");
        }
    }

    /// A core as the release before the modes were panels made it: ten panels and the three
    /// workspaces.
    fn old_core() -> DockCore {
        let old: Vec<PanelDef> = defs()
            .into_iter()
            .filter(|p| !id::MODES.contains(&p.id.as_str()))
            .collect();
        DockCore::new(old, workspaces())
    }

    #[test]
    fn an_arrangement_saved_before_the_modes_were_panels_loads_whole() {
        let mut old = old_core();
        // the person works in Review, hid the console there and floated the layers
        old.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(
            WS_REVIEW.to_string(),
        ))
        .unwrap();
        old.apply(&eh3d_dock::menu::DockAction::HidePanel(
            id::CONSOLE.to_string(),
        ))
        .unwrap();
        old.apply(&eh3d_dock::menu::DockAction::FloatPanel(
            id::LAYERS.to_string(),
        ))
        .unwrap();
        let saved = dock_text(&old.config_pairs());
        for mode in id::MODES {
            assert!(!saved.contains(mode), "the old file knows {mode}");
        }
        let mut now = core();
        now.load_config(&saved);
        // only the repair that hides the new panels is noted; nothing is read as damaged
        assert!(
            now.warnings()
                .iter()
                .all(|w| w.kind == eh3d_dock::workspaces::WarnKind::Repaired),
            "{:?}",
            now.warnings()
        );
        assert!(now.layout_is_sound());
        assert_eq!(now.workspaces().current_name(), WS_REVIEW);
        assert!(!now.layout().is_shown(id::CONSOLE));
        assert!(now.layout().is_shown(id::BOARD_VIEW));
        assert_eq!(
            now.layout().state_of(id::LAYERS),
            old.layout().state_of(id::LAYERS)
        );
        for mode in id::MODES {
            assert!(!now.layout().is_shown(mode), "{mode}");
        }
        // every other workspace came back as it was
        for name in [WS_EDIT, WS_COMPACT] {
            now.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(
                name.to_string(),
            ))
            .unwrap();
            old.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(
                name.to_string(),
            ))
            .unwrap();
            for p in id::ALL.iter().filter(|p| !id::MODES.contains(p)) {
                assert_eq!(
                    now.layout().state_of(p),
                    old.layout().state_of(p),
                    "{name}: {p}"
                );
            }
        }
        // and the first save after it writes the new panels, once
        let mut watch = LayoutWatch::new(&old.config_pairs());
        assert!(watch.changed(&now.config_pairs()));
        let mut again = core();
        again.load_config(&dock_text(&now.config_pairs()));
        assert!(again.warnings().is_empty(), "{:?}", again.warnings());
    }

    #[test]
    fn an_arrangement_with_the_modes_open_loads_in_a_release_that_does_not_know_them() {
        // the other way round: a file written with the modes shown, read by the older set of panels
        let mut now = core();
        for p in id::MODES {
            now.apply(&eh3d_dock::menu::DockAction::ShowPanel(p.to_string()))
                .unwrap();
        }
        let saved = dock_text(&now.config_pairs());
        let mut old = old_core();
        old.load_config(&saved);
        assert!(old.layout_is_sound());
        assert!(old.layout().is_shown(id::BOARD_VIEW));
        assert_eq!(old.workspaces().current_name(), WS_EDIT);
    }

    #[test]
    fn each_workspace_shows_what_its_task_needs() {
        let c = core();
        let factory = c.workspaces().factory();
        let shown = |name: &str, panel: &str| {
            factory
                .iter()
                .find(|w| w.name == name)
                .unwrap_or_else(|| panic!("no workspace {name}"))
                .layout
                .is_shown(panel)
        };
        // the design: the board, the tools, the layers, the components and the console
        for p in [
            id::BOARD_VIEW,
            id::PROPERTIES,
            id::LAYERS,
            id::COMPONENTS,
            id::NETS,
            id::LIBRARY,
            id::CONSOLE,
            id::DRC,
            id::REPORT,
            id::TASKS,
        ] {
            assert!(shown(WS_EDIT, p), "Edit lacks {p}");
        }
        // the review: the board with the rules and the report, no library and no tasks
        for p in [
            id::BOARD_VIEW,
            id::DRC,
            id::REPORT,
            id::CONSOLE,
            id::LAYERS,
            id::PROPERTIES,
            id::NETS,
        ] {
            assert!(shown(WS_REVIEW, p), "Review lacks {p}");
        }
        for p in [id::LIBRARY, id::TASKS, id::COMPONENTS] {
            assert!(!shown(WS_REVIEW, p), "Review has {p}");
        }
        // the compact one: the board alone, the rest folded
        assert!(shown(WS_COMPACT, id::BOARD_VIEW));
        for p in [id::LAYERS, id::PROPERTIES, id::DRC] {
            assert!(!shown(WS_COMPACT, p), "Compact has {p} on show");
        }
    }

    #[test]
    fn a_workspace_is_found_by_a_word_of_the_command_line() {
        assert_eq!(workspace_named("edit"), Some("Edit"));
        assert_eq!(workspace_named(" REVIEW "), Some("Review"));
        assert_eq!(workspace_named("compact"), Some("Compact"));
        assert_eq!(workspace_named("expert"), None);
        assert_eq!(workspace_named("beginner"), None);
        assert_eq!(workspace_named(""), None);
    }

    #[test]
    fn there_are_no_levels_of_experience() {
        // the workspaces are tasks; no name stands for a level of the person
        for name in core().workspaces().names() {
            let lower = name.to_ascii_lowercase();
            for level in ["beginner", "novice", "advanced", "expert", "pro"] {
                assert!(!lower.contains(level), "{name}");
            }
        }
    }

    #[test]
    fn the_menu_bar_is_well_made_in_every_language() {
        let bar = menu_bar();
        assert!(bar.validate().is_empty(), "{:?}", bar.validate());
        let titles: Vec<&str> = bar.menus.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(titles, ["file", "edit", "view", "tools", "window", "help"]);
        for l in Lang::ALL {
            let ctx = MenuContext::empty(view_lang(l));
            assert!(
                bar.conflicts(&ctx).is_empty(),
                "{:?} in {l:?}",
                bar.conflicts(&ctx)
            );
        }
        let ids: Vec<&str> = entries(&bar).iter().map(|e| e.id.as_str()).collect();
        for want in [
            "file.new",
            "file.open",
            "file.save",
            "file.save_as",
            "file.export",
            "file.quit",
            "edit.undo",
            "edit.redo",
            "edit.preferences",
            "edit.clear_selection",
            "view.fit",
            "tools.search",
            "tools.erc",
            "tools.view3d",
            "tools.settings_files",
            "help.shortcuts",
            "help.online",
        ] {
            assert!(ids.contains(&want), "no entry {want}");
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in &ids {
            assert!(seen.insert(*id), "{id} twice");
        }
    }

    #[test]
    fn every_text_of_the_menus_exists_in_the_eight_languages() {
        let bar = menu_bar();
        for e in entries(&bar) {
            if let Label::Own(t) = &e.label {
                for (i, s) in t.0.iter().enumerate() {
                    assert!(!s.trim().is_empty(), "{} language {i}", e.id);
                }
            }
        }
    }

    #[test]
    fn no_key_of_a_menu_is_also_a_key_of_the_program() {
        // the keys of the program are handled by the window; an entry that shared one would fire twice
        let bar = menu_bar();
        let keymap = Keymap::default();
        for e in entries(&bar) {
            let Some(s) = &e.shortcut else { continue };
            for action in Action::ALL {
                for b in keymap.keys(action) {
                    let same = b.ctrl == s.ctrl
                        && b.shift == s.shift
                        && !s.alt
                        && format!("{:?}", b.key).eq_ignore_ascii_case(&s.key);
                    // the search box opens from the menu with the same key on purpose: the window
                    // handles the key and the entry only shows it, so it is the one allowed pair
                    if e.id == "tools.search" && action == Action::Search {
                        continue;
                    }
                    assert!(!same, "{} has the key of {action:?}", e.id);
                }
            }
        }
        for id in KEYS_OF_THE_PROGRAM {
            assert!(entry(&bar, id).shortcut.is_none(), "{id}");
        }
    }

    #[test]
    fn the_entries_that_are_off_follow_what_is_open() {
        let nothing = disabled_entries(&MenuFacts::default());
        for id in [
            "file.save",
            "file.save_as",
            "file.export",
            "edit.undo",
            "edit.redo",
            "edit.cut",
            "view.fit",
            "tools.erc",
            "tools.view3d",
        ] {
            assert!(nothing.contains(&id.to_string()), "{id}");
        }
        let board = disabled_entries(&MenuFacts {
            has_document: true,
            is_board: true,
            can_undo: true,
            ..MenuFacts::default()
        });
        assert!(!board.contains(&"file.save".to_string()));
        assert!(!board.contains(&"edit.undo".to_string()));
        assert!(board.contains(&"edit.redo".to_string()));
        assert!(!board.contains(&"tools.view3d".to_string()));
        assert!(board.contains(&"tools.erc".to_string()));
        let schematic = disabled_entries(&MenuFacts {
            has_document: true,
            ..MenuFacts::default()
        });
        assert!(!schematic.contains(&"tools.erc".to_string()));
        assert!(schematic.contains(&"tools.view3d".to_string()));
        // the board editor keeps its own steps and the entries undo and redo of the window drive
        // them; saving stays off while it is on
        let board_editor = disabled_entries(&MenuFacts {
            has_document: true,
            is_board: true,
            editing: true,
            editor_history: true,
            can_undo: true,
            ..MenuFacts::default()
        });
        assert!(!board_editor.contains(&"edit.undo".to_string()));
        assert!(board_editor.contains(&"edit.redo".to_string()));
        assert!(board_editor.contains(&"file.save".to_string()));
        // an editor without such steps: the entries of the window are off
        let editing = disabled_entries(&MenuFacts {
            has_document: true,
            is_board: true,
            editing: true,
            editor_history: false,
            can_undo: true,
            can_redo: true,
            actions: 0,
        });
        for id in ["file.save", "file.save_as", "edit.undo", "edit.redo"] {
            assert!(editing.contains(&id.to_string()), "{id}");
        }
        // a panel of a mode (library, Gerber, 3D, boards) does not take the place of the drawing, so it
        // does not turn off what belongs to the document
        let with_modes = disabled_entries(&MenuFacts {
            has_document: true,
            is_board: true,
            can_undo: true,
            ..MenuFacts::default()
        });
        assert!(!with_modes.contains(&"file.export".to_string()));
        assert!(!with_modes.contains(&"view.fit".to_string()));
    }

    #[test]
    fn every_off_entry_exists_in_the_bar_and_the_list_is_sorted_and_without_repeats() {
        let bar = menu_bar();
        let ids: Vec<&str> = entries(&bar).iter().map(|e| e.id.as_str()).collect();
        // facts drawn from a generator with a fixed seed
        let mut seed = 0x5eed_u64;
        for _ in 0..200 {
            let n = lcg(&mut seed);
            let f = MenuFacts {
                has_document: n & 1 != 0,
                is_board: n & 2 != 0,
                editing: n & 4 != 0,
                editor_history: n & 8 != 0,
                can_undo: n & 16 != 0,
                can_redo: n & 32 != 0,
                actions: (n >> 6) as u16 & 0x3f,
            };
            let off = disabled_entries(&f);
            let mut sorted = off.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(off, sorted, "{f:?}");
            for id in &off {
                assert!(
                    ids.contains(&id.as_str()),
                    "{id} is off but is not in the bar"
                );
            }
            // what is off with a document that is open and drawn is a subset of what is off with none
            if f.has_document {
                let none = disabled_entries(&MenuFacts::default());
                for id in ["file.export", "view.fit"] {
                    assert!(none.contains(&id.to_string()));
                    assert!(!off.contains(&id.to_string()), "{id} {f:?}");
                }
            }
        }
    }

    #[test]
    fn the_arrangement_is_written_once_when_it_changes_and_never_when_it_does_not() {
        let mut c = core();
        let mut watch = LayoutWatch::new(&c.config_pairs());
        assert!(!watch.changed(&c.config_pairs()));
        assert!(!watch.changed(&c.config_pairs()));
        c.apply(&eh3d_dock::menu::DockAction::HidePanel(
            id::LAYERS.to_string(),
        ))
        .unwrap();
        assert!(watch.changed(&c.config_pairs()));
        assert!(!watch.changed(&c.config_pairs()));
        c.apply(&eh3d_dock::menu::DockAction::ShowPanel(
            id::LAYERS.to_string(),
        ))
        .unwrap();
        assert!(watch.changed(&c.config_pairs()));
    }

    #[test]
    fn the_arrangement_read_back_gives_what_was_saved() {
        let mut c = core();
        c.apply(&eh3d_dock::menu::DockAction::HidePanel(
            id::CONSOLE.to_string(),
        ))
        .unwrap();
        c.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(
            WS_REVIEW.to_string(),
        ))
        .unwrap();
        let pairs = c.config_pairs();
        let text = dock_text(&pairs);
        let mut back = core();
        back.load_config(&text);
        assert!(back.warnings().is_empty(), "{:?}", back.warnings());
        assert_eq!(back.config_pairs(), pairs);
        assert_eq!(back.workspaces().current_name(), WS_REVIEW);
    }

    #[test]
    fn a_damaged_arrangement_never_stops_the_window_and_gives_the_factory_one() {
        // texts drawn from a generator with a fixed seed: whatever they say, the core stays sound
        let mut seed = 0xd0c4_u64;
        let words = [
            "dock.version = 1",
            "dock.current = 99",
            "dock.ws.1.name = Edit",
            "dock.ws.1.tree = H200(T0(board_view),T0(nothing))",
            "dock.ws.2.tree = V(((",
            "dock.ws.1.hidden = layers,layers,x",
            "garbage",
            "dock.ws.9.name = \u{7}",
            "dock.version = 7",
        ];
        for _ in 0..100 {
            let mut text = String::new();
            for _ in 0..(lcg(&mut seed) % 6 + 1) {
                text.push_str(words[(lcg(&mut seed) as usize) % words.len()]);
                text.push('\n');
            }
            let mut c = core();
            c.load_config(&text);
            assert!(c.layout_is_sound(), "{text:?}");
            assert!(!c.workspaces().names().is_empty());
        }
    }

    #[test]
    fn the_language_of_the_frame_follows_the_language_of_the_window() {
        let mut seen = std::collections::BTreeSet::new();
        for l in Lang::ALL {
            assert_eq!(view_lang(l).code(), l.tag());
            seen.insert(view_lang(l).code().to_string());
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn the_titles_of_the_frame_are_made_from_the_texts_of_the_program_in_the_order_it_wants() {
        let t = t8(|l| Msg::Fit.tr(l));
        assert_eq!(t.0[0], Msg::Fit.tr(Lang::En));
        assert_eq!(t.0[1], Msg::Fit.tr(Lang::Es));
        assert_eq!(t.0[2], Msg::Fit.tr(Lang::It));
        assert_eq!(t.0[3], Msg::Fit.tr(Lang::De));
        assert_eq!(t.0[4], Msg::Fit.tr(Lang::Fr));
        assert_eq!(t.0[5], Msg::Fit.tr(Lang::Pt));
        assert_eq!(t.0[6], Msg::Fit.tr(Lang::Zh));
        assert_eq!(t.0[7], Msg::Fit.tr(Lang::Ja));
    }
    #[test]
    fn the_actions_of_the_board_editor_are_in_the_edit_menu_and_follow_the_facts() {
        let bar = menu_bar();
        let ids: Vec<&str> = entries(&bar).iter().map(|e| e.id.as_str()).collect();
        let none = disabled_entries(&MenuFacts::default());
        for (i, action) in EditAction::ALL.iter().enumerate() {
            let id = format!("edit.{}", action.key());
            assert!(ids.contains(&id.as_str()), "{id}");
            assert!(none.contains(&id), "{id}");
            let one = disabled_entries(&MenuFacts {
                has_document: true,
                is_board: true,
                editing: true,
                actions: 1 << i,
                ..MenuFacts::default()
            });
            assert!(!one.contains(&id), "{id}");
            for (j, other) in EditAction::ALL.iter().enumerate() {
                if j != i {
                    assert!(one.contains(&format!("edit.{}", other.key())), "{other:?}");
                }
            }
        }
    }

    #[test]
    fn a_choice_in_the_edit_menu_is_the_command_of_its_action() {
        let bar = menu_bar();
        for action in EditAction::ALL {
            let id = format!("edit.{}", action.key());
            let entry = entries(&bar).into_iter().find(|e| e.id == id).unwrap();
            let shown = format!("{:?}", entry.action);
            assert!(
                shown.contains(&format!("BoardAction({action:?})")),
                "{shown}"
            );
        }
    }
}

#[cfg(test)]
mod about_tests {
    use super::*;

    #[test]
    fn the_window_about_has_the_name_the_repository_and_the_picture_of_this_program() {
        let brand = about_brand();
        assert_eq!(brand.program, Phoenix::Pcb);
        assert_eq!(brand.program.name(), "EH3D-PHOENIX-PCB");
        assert_eq!(
            brand.program.repository(),
            "https://github.com/JuanenRac/EH3D-PHOENIX-PCB"
        );
        let png = brand.logo_png;
        assert_eq!(png.get(1..4), Some(&b"PNG"[..]));
        // the width and the height in the header of the picture
        assert_eq!(png.get(16..24), Some(&[0, 0, 1, 0, 0, 0, 1, 0][..]));
    }
}
