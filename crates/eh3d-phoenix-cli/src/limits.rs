// SPDX-License-Identifier: GPL-3.0-or-later
//! The option `--limits FILE`, which every command accepts: the limits of the readers of EAGLE
//! files (`config/eagle_board.cfg`), of the checks of a board (`config/board.cfg`) and the cut of
//! the arcs of the outline into chords (`config/outline_arcs.cfg`).
//!
//! The file holds the keys of the three files in any order, one `key = value` on each line; a missing
//! key keeps its default and an unknown key is refused with its line number. Without the option
//! the defaults are used, so no output changes. The option names a file that is read, never one
//! that is written.

use std::cell::Cell;

use eh3d_phoenix_board::validate::BoardLimits;
use eh3d_phoenix_board::{ArcSettings, ArcSettingsGuard};
use eh3d_phoenix_eagle_board::EagleLimits;

/// The name of the option.
const OPTION: &str = "--limits";

/// The key that belongs to the limits of a board.
const BOARD_KEY: &str = "max_polygon_points";

/// The keys of the cut of the arcs of the outline; every key that is neither these nor the board
/// key belongs to the EAGLE readers.
const ARC_KEYS: [&str; 2] = ["tolerance_nm", "max_pieces"];

/// The limits that a command reads with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Limits {
    pub(crate) board: BoardLimits,
    pub(crate) eagle: EagleLimits,
    pub(crate) arcs: ArcSettings,
}

impl Limits {
    /// Reads the limits from the text of a limits file.
    pub(crate) fn parse(text: &str) -> Result<Limits, String> {
        // each parser gets the whole text with the lines of the other kind blanked, so that a
        // message keeps the number of the line in the file
        let for_board = blank_foreign(text, Kind::Board);
        let for_eagle = blank_foreign(text, Kind::Eagle);
        let for_arcs = blank_foreign(text, Kind::Arcs);
        Ok(Limits {
            board: BoardLimits::parse(&for_board)?,
            eagle: EagleLimits::parse(&for_eagle)?,
            arcs: ArcSettings::parse(&for_arcs)?,
        })
    }

    /// Reads the limits from a file.
    pub(crate) fn load(path: &str) -> Result<Limits, String> {
        let text = crate::bounded::read_config_text(path)
            .map_err(|e| format!("cannot read the limits {path}: {e}"))?;
        Limits::parse(&text).map_err(|e| format!("{path}: {e}"))
    }
}

/// The kinds of limits that a file may hold.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Board,
    Arcs,
    Eagle,
}

/// Keeps the lines that the parser of one kind of limits reads and blanks the others: the parser
/// of the board gets the blank lines, the comments and the board key, the one of the arcs the
/// blank lines, the comments and the keys of the arcs, the one of the EAGLE readers gets every
/// line that is neither (so it also refuses a line that is not a limit).
fn blank_foreign(text: &str, kind: Kind) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let t = line.trim();
        let key = t.split_once('=').map(|(k, _)| k.trim());
        let is_board_key = key == Some(BOARD_KEY);
        let is_arc_key = key.is_some_and(|k| ARC_KEYS.contains(&k));
        let plain = t.is_empty() || t.starts_with('#');
        let keep = match kind {
            Kind::Board => plain || is_board_key,
            Kind::Arcs => plain || is_arc_key,
            Kind::Eagle => !is_board_key && !is_arc_key,
        };
        if keep {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

thread_local! {
    static ACTIVE: Cell<Limits> = Cell::new(Limits::default());
}

/// The limits of the command that runs now (the defaults when `--limits` was not given).
pub(crate) fn active() -> Limits {
    ACTIVE.with(Cell::get)
}

/// Puts the limits in force until the guard is dropped, then puts back the ones that were.
pub(crate) struct Guard {
    previous: Limits,
    // dropped after `drop` has run, which puts the cut of the arcs back
    _arcs: ArcSettingsGuard,
}

impl Guard {
    pub(crate) fn set(limits: Limits) -> Guard {
        Guard {
            previous: ACTIVE.with(|cell| cell.replace(limits)),
            _arcs: limits.arcs.set_current(),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|cell| cell.set(self.previous));
    }
}

/// Takes `--limits FILE` out of the arguments, wherever it stands, and returns the file. The last
/// one wins. `--limits` as the last word, with no file, is an error.
pub(crate) fn take_option(args: &mut Vec<String>) -> Result<Option<String>, String> {
    let mut file = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == OPTION {
            if i + 1 >= args.len() {
                return Err(format!("{OPTION} needs a file"));
            }
            file = Some(args.remove(i + 1));
            args.remove(i);
        } else {
            i += 1;
        }
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn both_shipped_files_together_are_the_defaults() {
        let board = include_str!("../../../config/board.cfg");
        let eagle = include_str!("../../../config/eagle_board.cfg");
        let arcs = include_str!("../../../config/outline_arcs.cfg");
        let both = format!("{board}\n{eagle}");
        assert_eq!(Limits::parse(&both), Ok(Limits::default()));
        let all = format!("{board}\n{eagle}\n{arcs}");
        assert_eq!(Limits::parse(&all), Ok(Limits::default()));
        assert_eq!(Limits::parse(board), Ok(Limits::default()));
        assert_eq!(Limits::parse(eagle), Ok(Limits::default()));
        assert_eq!(Limits::parse(arcs), Ok(Limits::default()));
        assert_eq!(Limits::parse(""), Ok(Limits::default()));
    }

    #[test]
    fn the_keys_of_both_kinds_are_read_in_any_order() {
        let l = Limits::parse(
            "max_bus_checks = 7\n# note\nmax_polygon_points = 12\nmax_outline_rings=3\n",
        )
        .unwrap();
        assert_eq!(l.board.max_polygon_points, 12);
        assert_eq!(l.eagle.max_bus_checks, 7);
        assert_eq!(l.eagle.max_outline_rings, 3);
        assert_eq!(
            l.eagle.max_restrict_checks,
            EagleLimits::default().max_restrict_checks
        );
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number_in_the_file() {
        let e = Limits::parse("max_bus_checks = 7\nmax_polygon_points = 2\n").unwrap_err();
        assert!(e.starts_with("line 2:"), "{e}");
        let e = Limits::parse("\n\nmax_bus_checks = 0\n").unwrap_err();
        assert!(e.starts_with("line 3:"), "{e}");
        let e = Limits::parse("max_polygon_points = 5\nno_such_key = 1\n").unwrap_err();
        assert!(e.starts_with("line 2:") && e.contains("no_such_key"), "{e}");
        let e = Limits::parse("just words\n").unwrap_err();
        assert!(e.starts_with("line 1:"), "{e}");
    }

    #[test]
    fn the_option_is_taken_from_any_place_and_the_last_one_wins() {
        let mut args = words(&["check", "--limits", "a.cfg", "x.brd", "--limits", "b.cfg"]);
        assert_eq!(take_option(&mut args), Ok(Some("b.cfg".to_owned())));
        assert_eq!(args, words(&["check", "x.brd"]));
        let mut none = words(&["check", "x.brd"]);
        assert_eq!(take_option(&mut none), Ok(None));
        assert_eq!(none, words(&["check", "x.brd"]));
        let mut open = words(&["check", "--limits"]);
        assert!(take_option(&mut open).is_err());
    }

    #[test]
    fn the_guard_puts_the_limits_in_force_and_back() {
        assert_eq!(active(), Limits::default());
        let set = Limits {
            board: BoardLimits {
                max_polygon_points: 9,
            },
            eagle: EagleLimits::default(),
            arcs: ArcSettings {
                tolerance_nm: 40,
                max_pieces: 12,
            },
        };
        {
            let _guard = Guard::set(set);
            assert_eq!(active().board.max_polygon_points, 9);
            // the cut of the arcs is in force for the code that cuts them
            assert_eq!(ArcSettings::current().max_pieces, 12);
        }
        assert_eq!(active(), Limits::default());
        assert_eq!(ArcSettings::current(), ArcSettings::default());
    }

    #[test]
    fn the_keys_of_the_arcs_are_read_with_the_others_and_their_range_is_checked() {
        let l = Limits::parse(
            "tolerance_nm = 250\nmax_bus_checks = 7\nmax_pieces = 64\nmax_polygon_points = 12\n",
        )
        .unwrap();
        assert_eq!(l.arcs.tolerance_nm, 250);
        assert_eq!(l.arcs.max_pieces, 64);
        assert_eq!(l.eagle.max_bus_checks, 7);
        assert_eq!(l.board.max_polygon_points, 12);
        for (text, line) in [
            ("tolerance_nm = 0\n", "line 1:"),
            ("max_bus_checks = 7\ntolerance_nm = 1000001\n", "line 2:"),
            ("\nmax_pieces = 0\n", "line 2:"),
            ("max_pieces = 65537\n", "line 1:"),
            ("max_pieces = many\n", "line 1:"),
        ] {
            let e = Limits::parse(text).unwrap_err();
            assert!(e.starts_with(line), "{text}: {e}");
        }
    }
}
