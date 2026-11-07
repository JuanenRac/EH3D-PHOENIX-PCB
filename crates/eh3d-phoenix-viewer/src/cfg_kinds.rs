// SPDX-License-Identifier: GPL-3.0-or-later
//! The kinds of settings files that the editor knows, and for each the reader of the program that
//! judges a text of that kind. The editor never decides by itself whether a text is valid: it hands
//! the text to `ZoneSettings::parse`, `DfmProfile::parse`, `ViewerSettings::parse` and the other
//! readers that the commands and the window use, so what is accepted here is accepted there.

use crate::settings::ViewerSettings;

/// What a reader says about a text it refuses: its code, the line it names (when it names one)
/// and what it says, in English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// The code of the reader, `PHOENIX-<AREA>-<NNN>`; empty for a reader that has none.
    pub code: String,
    /// The line of the text that the reader names, counted from 1.
    pub line: Option<usize>,
    /// What the reader says, without the code.
    pub detail: String,
}

/// The first number that follows the word `line ` in a message.
pub(crate) fn line_in(message: &str) -> Option<usize> {
    let at = message.find("line ")?;
    let digits: String = message[at + 5..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

pub(crate) fn make_issue(code: &str, message: String) -> Issue {
    let prefix = format!("{code}: ");
    let detail = message.strip_prefix(&prefix).unwrap_or(&message).to_owned();
    Issue {
        code: code.to_owned(),
        line: line_in(&detail),
        detail,
    }
}

type Check = fn(&str) -> Option<Issue>;

/// A function that hands a text to a reader of the program and reports what it refuses.
macro_rules! checker {
    ($name:ident, $parse:path, |$e:ident| $code:expr) => {
        fn $name(text: &str) -> Option<Issue> {
            match $parse(text) {
                Ok(_) => None,
                Err($e) => Some(make_issue($code, $e.to_string())),
            }
        }
    };
}

checker!(check_viewer, ViewerSettings::parse, |e| e.code);
checker!(
    check_board,
    eh3d_phoenix_board::validate::BoardLimits::parse,
    |e| ""
);
checker!(check_arcs, eh3d_phoenix_board::ArcSettings::parse, |e| "");
checker!(
    check_eagle,
    eh3d_phoenix_eagle_board::EagleLimits::parse,
    |e| ""
);
checker!(check_zones, eh3d_phoenix_zones::ZoneSettings::parse, |e| e
    .code);
checker!(
    check_boardedit,
    eh3d_phoenix_boardedit::BoardEditSettings::parse,
    |e| e.code
);
checker!(check_route, eh3d_phoenix_route::RouteSettings::parse, |e| e
    .code);
checker!(
    check_autoroute,
    eh3d_phoenix_autoroute::AutorouteConfig::parse,
    |e| e.code
);
checker!(check_erc, eh3d_phoenix_erc::ErcConfig::parse, |e| e.code());
checker!(
    check_gerber,
    eh3d_phoenix_gerber::ImportSettings::parse,
    |e| e.code
);
checker!(check_panel, eh3d_phoenix_panel::PanelSettings::parse, |e| e
    .code);
checker!(check_print, eh3d_phoenix_print::PrintConfig::parse, |e| e
    .code);
checker!(
    check_libedit,
    eh3d_phoenix_libedit::LibSettings::parse,
    |e| e.code
);
checker!(
    check_schedit,
    eh3d_phoenix_schedit::SchEditSettings::parse,
    |e| e.code
);
checker!(check_sync, eh3d_phoenix_sync::SyncSettings::parse, |e| e
    .code());
checker!(
    check_assembly,
    eh3d_phoenix_assembly3d::AssemblySettings::parse,
    |e| e.code
);
checker!(check_dxf, eh3d_phoenix_dxf::DxfSettings::parse, |e| e.code);
checker!(check_kicad, eh3d_phoenix_kicad::KicadSettings::parse, |e| e
    .code());
checker!(check_dfm, eh3d_phoenix_rules::DfmProfile::parse, |e| e
    .code());

/// A kind of settings file that the editor knows: where it lives under the folder of the
/// settings, the text of the repository that holds its defaults, and the reader that judges it.
pub struct CfgKind {
    /// The path of the file under the folder of the settings, with `/` between folders.
    pub file: &'static str,
    /// The file of the repository, which every reader requires to be its defaults.
    pub defaults: &'static str,
    check: Check,
}

impl CfgKind {
    /// What the reader of the file refuses in a text, or `None` when it accepts it.
    pub fn validate(&self, text: &str) -> Option<Issue> {
        (self.check)(text)
    }

    /// The last part of the path: the name of the file.
    pub fn base_name(&self) -> &'static str {
        self.file.rsplit('/').next().unwrap_or(self.file)
    }
}

macro_rules! kind {
    ($file:literal, $check:ident) => {
        CfgKind {
            file: $file,
            defaults: include_str!(concat!("../../../config/", $file)),
            check: $check,
        }
    };
}

/// Every kind of file that the editor opens, with the reader of the program for each.
pub static KINDS: &[CfgKind] = &[
    kind!("viewer.cfg", check_viewer),
    kind!("board.cfg", check_board),
    kind!("outline_arcs.cfg", check_arcs),
    kind!("eagle_board.cfg", check_eagle),
    kind!("zones.cfg", check_zones),
    kind!("boardedit.cfg", check_boardedit),
    kind!("route.cfg", check_route),
    kind!("autoroute_native.cfg", check_autoroute),
    kind!("erc.cfg", check_erc),
    kind!("gerber_import.cfg", check_gerber),
    kind!("panel.cfg", check_panel),
    kind!("print.cfg", check_print),
    kind!("libedit.cfg", check_libedit),
    kind!("schedit.cfg", check_schedit),
    kind!("sync.cfg", check_sync),
    kind!("assembly3d.cfg", check_assembly),
    kind!("dxf.cfg", check_dxf),
    kind!("kicad.cfg", check_kicad),
    kind!("dfm/example.cfg", check_dfm),
    kind!("dfm_jlcpcb.cfg", check_dfm),
    kind!("dfm_pcbway.cfg", check_dfm),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(file: &str) -> &'static CfgKind {
        KINDS.iter().find(|k| k.file == file).unwrap()
    }

    #[test]
    fn every_file_of_the_repository_is_valid_for_its_own_reader() {
        for k in KINDS {
            assert_eq!(k.validate(k.defaults), None, "{}", k.file);
        }
    }

    #[test]
    fn the_names_are_distinct_and_the_base_name_is_the_last_part() {
        let mut names: Vec<&str> = KINDS.iter().map(|k| k.file).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), KINDS.len());
        assert_eq!(kind("dfm/example.cfg").base_name(), "example.cfg");
        assert_eq!(kind("zones.cfg").base_name(), "zones.cfg");
    }

    #[test]
    fn a_message_gives_its_line_and_loses_the_code_in_front() {
        let issue = make_issue(
            "PHOENIX-ZONE-001",
            "PHOENIX-ZONE-001: line 12: bad".to_owned(),
        );
        assert_eq!(issue.line, Some(12));
        assert_eq!(issue.detail, "line 12: bad");
        assert_eq!(make_issue("", "no place".to_owned()).line, None);
        assert_eq!(line_in("x (line 7, column 2)"), Some(7));
        assert_eq!(line_in("the outline is open"), None);
        assert_eq!(line_in("line x"), None);
    }

    #[test]
    fn a_line_that_is_not_key_equals_value_is_refused_with_its_number() {
        let mut text = kind("viewer.cfg").defaults.to_owned();
        text.push_str("this is not a setting\n");
        let last = text.lines().count();
        let issue = kind("viewer.cfg").validate(&text).unwrap();
        assert_eq!(issue.code, "PHOENIX-VIEW-005");
        assert_eq!(issue.line, Some(last));
        let unknown = format!("{}no_such_key = 1\n", kind("zones.cfg").defaults);
        let issue = kind("zones.cfg").validate(&unknown).unwrap();
        assert_eq!(issue.code, "PHOENIX-ZONE-001");
        assert!(issue.detail.contains("unknown key"), "{}", issue.detail);
        assert_eq!(issue.line, Some(unknown.lines().count()));
    }

    #[test]
    fn readers_that_give_no_code_still_name_the_line() {
        let issue = kind("board.cfg")
            .validate("max_polygon_points = 2\n")
            .unwrap();
        assert_eq!(issue.code, "");
        assert_eq!(issue.line, Some(1));
        let issue = kind("outline_arcs.cfg")
            .validate("tolerance_nm = x\n")
            .unwrap();
        assert_eq!(issue.line, Some(1));
        let issue = kind("eagle_board.cfg").validate("# a\nzzz = 1\n").unwrap();
        assert_eq!(issue.line, Some(2));
    }

    #[test]
    fn a_profile_of_a_maker_is_judged_by_the_reader_of_profiles() {
        let issue = kind("dfm_jlcpcb.cfg")
            .validate("name = a\nmin_track_width_nm = -5\n")
            .unwrap();
        assert_eq!(issue.code, "PHOENIX-RULEFILE-002");
        assert_eq!(issue.line, Some(2));
        assert!(kind("dfm/example.cfg")
            .validate("name = a\nname = b\n")
            .is_some());
    }

    #[test]
    fn the_readers_that_report_by_method_and_by_field_give_their_codes() {
        // erc and sync report the code with a method, the others with a field
        let erc = kind("erc.cfg").validate("zzz\n").unwrap();
        assert!(erc.code.starts_with("PHOENIX-ERC-"), "{}", erc.code);
        assert_eq!(erc.line, Some(1));
        let sync = kind("sync.cfg").validate("zzz\n").unwrap();
        assert!(sync.code.starts_with("PHOENIX-SYNC-"), "{}", sync.code);
        let kicad = kind("kicad.cfg").validate("zzz\n").unwrap();
        assert!(kicad.code.starts_with("PHOENIX-KICAD-"), "{}", kicad.code);
        let dxf = kind("dxf.cfg").validate("zzz\n").unwrap();
        assert!(dxf.code.starts_with("PHOENIX-DXF-"), "{}", dxf.code);
    }
}
