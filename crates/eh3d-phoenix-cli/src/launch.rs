// SPDX-License-Identifier: GPL-3.0-or-later
//! What the single program `phoenix` does for the arguments it was started with: open the window,
//! carry out a command on the console or print the help. The decision reads the arguments only (and
//! asks the caller whether a bare word names a file), so it is tested without a screen.

use crate::commands;

/// What the program does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Print the help: no arguments in a build without the window.
    Help,
    /// Carry out a command of the command line on the console.
    Cli,
    /// Open the window; the arguments after the first `skip` words are the options of the window
    /// (`skip` is 1 for `phoenix view ...`, which names the window with a command).
    Window {
        /// How many leading arguments are not options of the window.
        skip: usize,
    },
}

/// The options that only the window knows. A first argument that is one of them starts the window.
pub const WINDOW_OPTIONS: [&str; 9] = [
    "--settings",
    "--lang",
    "--shot",
    "--size",
    "--theme",
    "--wait",
    "--window",
    "--replace",
    "--open",
];

/// The options of `phoenix view` that need the console: with one of them the command runs as a
/// command and not in the window.
const VIEW_CONSOLE_OPTIONS: [&str; 5] = [
    "--summary",
    "--snapshot",
    "--bench",
    "--panel",
    "--panel-config",
];

/// The picture options of the window, which `phoenix view` hands over to the window.
const PICTURE_OPTIONS: [&str; 6] = [
    "--shot",
    "--size",
    "--theme",
    "--wait",
    "--window",
    "--replace",
];

/// The words that `run` itself handles and that are not in the table of the commands.
const OWN_WORDS: [&str; 5] = ["help", "--help", "-h", "--config", "eagle-rate"];

fn is_command_word(word: &str) -> bool {
    commands::command(word).is_some() || word == "inventory" || OWN_WORDS.contains(&word)
}

/// Decides the mode. `window_built` is true in a build with the `gui` feature; `is_file` tells
/// whether a bare word names an existing file (a board or a schematic to open).
///
/// * No arguments: the window, or the help in a build without it.
/// * A command of the command line (`check`, `bom`, `help`, `--config`, ...) runs on the console.
///   `view` opens the window, and when it is given picture options (`--shot`, `--size`, `--theme`,
///   `--wait`, `--window`, `--replace`, `--open`) and none of the options that need the console
///   (`--summary`, `--snapshot`, `--bench`, `--panel`) they are read by the window.
/// * A first argument that is an option of the window, or the name of an existing file that is not
///   a command, starts the window.
/// * Anything else is left to the command line, which answers with its usage and the code 2 as it
///   always did.
pub fn choose_mode(args: &[String], window_built: bool, is_file: &dyn Fn(&str) -> bool) -> Mode {
    let Some(first) = args.first().map(String::as_str) else {
        return if window_built {
            Mode::Window { skip: 0 }
        } else {
            Mode::Help
        };
    };
    if !window_built {
        return Mode::Cli;
    }
    if first == "view" {
        let rest = &args[1..];
        let named = |list: &[&str]| rest.iter().any(|a| list.contains(&a.as_str()));
        let wants_picture = named(&PICTURE_OPTIONS) || named(&["--open"]);
        return if wants_picture && !named(&VIEW_CONSOLE_OPTIONS) {
            Mode::Window { skip: 1 }
        } else {
            Mode::Cli
        };
    }
    if is_command_word(first) {
        return Mode::Cli;
    }
    if first.starts_with('-') {
        return if WINDOW_OPTIONS.contains(&first) {
            Mode::Window { skip: 0 }
        } else {
            Mode::Cli
        };
    }
    if is_file(first) {
        Mode::Window { skip: 0 }
    } else {
        Mode::Cli
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn mode(list: &[&str], window_built: bool) -> Mode {
        choose_mode(&args(list), window_built, &|p| p.ends_with(".brd"))
    }

    #[test]
    fn no_arguments_open_the_window_or_print_the_help() {
        assert_eq!(mode(&[], true), Mode::Window { skip: 0 });
        assert_eq!(mode(&[], false), Mode::Help);
    }

    #[test]
    fn a_command_runs_on_the_console() {
        for list in [
            vec!["check", "board.brd"],
            vec!["bom", "--picture"],
            vec!["help"],
            vec!["--help"],
            vec!["-h"],
            vec!["--config", "inventory.cfg", "inventory", "."],
            vec!["inventory", "."],
            vec!["eagle-rate", "a", "b"],
        ] {
            assert_eq!(mode(&list, true), Mode::Cli, "{list:?}");
            assert_eq!(mode(&list, false), Mode::Cli, "{list:?}");
        }
    }

    #[test]
    fn a_build_without_the_window_never_opens_it() {
        for list in [
            vec!["board.brd"],
            vec!["--shot", "a.png"],
            vec!["view", "--shot", "a.png"],
        ] {
            assert_eq!(mode(&list, false), Mode::Cli, "{list:?}");
        }
    }

    #[test]
    fn the_options_of_the_window_start_the_window() {
        for option in WINDOW_OPTIONS {
            assert_eq!(
                mode(&[option, "x"], true),
                Mode::Window { skip: 0 },
                "{option}"
            );
        }
        assert_eq!(
            mode(
                &["--shot", "a.png", "--window", "document", "--open", "b.brd"],
                true
            ),
            Mode::Window { skip: 0 }
        );
    }

    #[test]
    fn an_existing_file_opens_in_the_window_and_another_word_is_a_mistake() {
        assert_eq!(mode(&["board.brd"], true), Mode::Window { skip: 0 });
        assert_eq!(
            mode(&["board.brd", "--lang", "pt"], true),
            Mode::Window { skip: 0 }
        );
        // not a file and not a command: the usage and the code 2 of the command line
        assert_eq!(mode(&["chek"], true), Mode::Cli);
        assert_eq!(mode(&["--nonsense"], true), Mode::Cli);
    }

    #[test]
    fn view_hands_the_picture_options_to_the_window() {
        assert_eq!(mode(&["view", "board.brd"], true), Mode::Cli);
        assert_eq!(mode(&["view"], true), Mode::Cli);
        assert_eq!(
            mode(&["view", "--shot", "a.png", "board.brd"], true),
            Mode::Window { skip: 1 }
        );
        assert_eq!(
            mode(&["view", "--open", "board.brd", "--lang", "de"], true),
            Mode::Window { skip: 1 }
        );
        for console in ["--summary", "--bench", "--panel"] {
            assert_eq!(
                mode(&["view", console, "--shot", "a.png"], true),
                Mode::Cli,
                "{console}"
            );
        }
        assert_eq!(
            mode(&["view", "--snapshot", "o.svg", "--open", "b.brd"], true),
            Mode::Cli
        );
    }

    #[test]
    fn the_options_after_view_are_read_by_the_window() {
        let list = args(&["view", "--shot", "a.png", "--window", "document", "b.brd"]);
        let Mode::Window { skip } = choose_mode(&list, true, &|_| false) else {
            panic!("the window was expected");
        };
        let parsed = crate::window_args::parse_window_args(&list[skip..]).unwrap();
        assert_eq!(parsed.file, Some(std::path::PathBuf::from("b.brd")));
        assert_eq!(parsed.shot.unwrap().window.name(), "document");
        // the same options with the word `view` left in would be taken for a file
        assert!(crate::window_args::parse_window_args(&list).is_err());
    }

    #[test]
    fn every_command_of_the_table_is_a_command_word() {
        for command in commands::COMMANDS {
            assert!(is_command_word(command.name), "{}", command.name);
        }
    }
}
