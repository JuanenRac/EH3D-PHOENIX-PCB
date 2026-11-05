// SPDX-License-Identifier: GPL-3.0-or-later
//! The arguments of the program `phoenix-gui`, which opens the window of the viewer: an optional
//! file, `--settings FILE` and `--lang CODE`. They are read here, and not in the program itself,
//! so that they are tested without a window.

use std::path::PathBuf;

use eh3d_phoenix_rules::Lang;

/// What `phoenix-gui` was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowArgs {
    /// A board, a schematic or a library to open at once; none shows the start screen.
    pub file: Option<PathBuf>,
    /// A settings file (`config/viewer.cfg` shows every key).
    pub settings: Option<PathBuf>,
    /// The language of the window; none takes the one of the system.
    pub lang: Option<Lang>,
}

/// Reads the arguments that follow the program name. The answer for a mistake is the reason.
pub fn parse_window_args(args: &[String]) -> Result<WindowArgs, String> {
    let mut parsed = WindowArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--settings" => {
                let path = iter.next().ok_or("--settings needs a file")?;
                parsed.settings = Some(PathBuf::from(path));
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                parsed.lang = Some(Lang::from_tag(tag).ok_or("unknown language code")?);
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => {
                if parsed.file.is_some() {
                    return Err("only one file can be given".to_owned());
                }
                parsed.file = Some(PathBuf::from(other));
            }
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn no_arguments_show_the_start_screen() {
        assert_eq!(parse_window_args(&[]), Ok(WindowArgs::default()));
    }

    #[test]
    fn a_file_a_settings_file_and_a_language_are_read_in_any_order() {
        let parsed =
            parse_window_args(&args(&["--lang", "pt", "board.brd", "--settings", "v.cfg"]))
                .unwrap();
        assert_eq!(parsed.file, Some(PathBuf::from("board.brd")));
        assert_eq!(parsed.settings, Some(PathBuf::from("v.cfg")));
        assert_eq!(parsed.lang, Some(Lang::Pt));
    }

    #[test]
    fn a_mistake_gives_its_reason() {
        for (list, reason) in [
            (vec!["--settings"], "needs a file"),
            (vec!["--lang"], "needs a language code"),
            (vec!["--lang", "xx"], "unknown language code"),
            (vec!["--nonsense"], "unknown option --nonsense"),
            (vec!["a.brd", "b.brd"], "only one file"),
        ] {
            let error = parse_window_args(&args(&list)).unwrap_err();
            assert!(error.contains(reason), "{list:?}: {error}");
        }
    }
}
