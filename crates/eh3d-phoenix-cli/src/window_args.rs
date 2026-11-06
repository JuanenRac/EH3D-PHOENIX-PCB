// SPDX-License-Identifier: GPL-3.0-or-later
//! The arguments of the program `phoenix-gui`, which opens the window of the viewer: an optional
//! file (also `--open FILE`), `--settings FILE` and `--lang CODE`, and the picture mode
//! (`--shot FILE.png` with `--size`, `--theme`, `--wait`, `--window` and `--replace`). They are
//! read here, and not in the program itself, so that they are tested without a window.

use std::path::PathBuf;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_viewer::appearance::Theme;
use eh3d_phoenix_viewer::shot::{
    parse_size, ShotRequest, ShotWindow, DEFAULT_SIZE, DEFAULT_WAIT, WAIT_MAX,
};

/// What `phoenix-gui` was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowArgs {
    /// A board, a schematic or a library to open at once; none shows the start screen.
    pub file: Option<PathBuf>,
    /// A settings file (`config/viewer.cfg` shows every key).
    pub settings: Option<PathBuf>,
    /// The language of the window; none takes the one of the system.
    pub lang: Option<Lang>,
    /// The picture mode: the window opens, saves a PNG file and closes by itself.
    pub shot: Option<ShotRequest>,
}

/// Reads the arguments that follow the program name. The answer for a mistake is the reason.
pub fn parse_window_args(args: &[String]) -> Result<WindowArgs, String> {
    let mut parsed = WindowArgs::default();
    let mut output: Option<PathBuf> = None;
    let mut picture = PictureOptions::default();
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
            "--shot" => {
                let path = iter.next().ok_or("--shot needs a file to write")?;
                if !path.to_ascii_lowercase().ends_with(".png") {
                    return Err("--shot writes a PNG file: the name must end in .png".to_owned());
                }
                output = Some(PathBuf::from(path));
            }
            "--size" => {
                let text = iter.next().ok_or("--size needs WIDTHxHEIGHT")?;
                picture.size = Some(parse_size(text).ok_or(
                    "--size is WIDTHxHEIGHT in pixels, each side from 320 to 8192, as 1280x720",
                )?);
            }
            "--theme" => {
                let text = iter.next().ok_or("--theme needs dark, light or contrast")?;
                picture.theme =
                    Some(Theme::parse(text).ok_or("--theme is dark, light or contrast")?);
            }
            "--wait" => {
                let text = iter.next().ok_or("--wait needs a number of frames")?;
                picture.wait = Some(
                    text.parse::<u32>()
                        .ok()
                        .filter(|n| *n <= WAIT_MAX)
                        .ok_or("--wait is a number of frames from 0 to 3600")?,
                );
            }
            "--window" => {
                let text = iter.next().ok_or("--window needs the name of a window")?;
                picture.window = Some(ShotWindow::parse(text).ok_or_else(|| {
                    let names: Vec<&str> = ShotWindow::ALL.iter().map(|w| w.name()).collect();
                    format!("--window is one of {}", names.join(", "))
                })?);
            }
            "--replace" => picture.replace = true,
            "--open" => {
                let path = iter.next().ok_or("--open needs a file")?;
                set_file(&mut parsed, path)?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => set_file(&mut parsed, other)?,
        }
    }
    parsed.shot = picture.into_request(output, parsed.file.is_some())?;
    Ok(parsed)
}

fn set_file(parsed: &mut WindowArgs, path: &str) -> Result<(), String> {
    if parsed.file.is_some() {
        return Err("only one file can be given".to_owned());
    }
    parsed.file = Some(PathBuf::from(path));
    Ok(())
}

/// The options of the picture mode as they are read, before they are checked together.
#[derive(Default)]
struct PictureOptions {
    size: Option<(u32, u32)>,
    theme: Option<Theme>,
    wait: Option<u32>,
    window: Option<ShotWindow>,
    replace: bool,
}

impl PictureOptions {
    /// Joins the options with the file of the picture. Without `--shot` none of them may be
    /// given; the window not named is the document when a file is open and the start screen
    /// otherwise, and a window that needs a file or cannot have one is refused.
    fn into_request(
        self,
        output: Option<PathBuf>,
        has_file: bool,
    ) -> Result<Option<ShotRequest>, String> {
        let Some(output) = output else {
            let given = self.size.is_some()
                || self.theme.is_some()
                || self.wait.is_some()
                || self.window.is_some()
                || self.replace;
            return if given {
                Err("--size, --theme, --wait, --window and --replace need --shot".to_owned())
            } else {
                Ok(None)
            };
        };
        let window = self.window.unwrap_or(if has_file {
            ShotWindow::Document
        } else {
            ShotWindow::Start
        });
        if window.needs_file() && !has_file {
            return Err(format!(
                "--window {} needs a file to open (--open FILE)",
                window.name()
            ));
        }
        if window == ShotWindow::Start && has_file {
            return Err("--window start shows nothing open: leave the file out".to_owned());
        }
        Ok(Some(ShotRequest {
            output,
            replace: self.replace,
            size: self.size.unwrap_or(DEFAULT_SIZE),
            theme: self.theme,
            wait: self.wait.unwrap_or(DEFAULT_WAIT),
            window,
        }))
    }
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
    fn open_names_the_file_like_a_bare_argument() {
        let parsed = parse_window_args(&args(&["--open", "board.brd"])).unwrap();
        assert_eq!(parsed.file, Some(PathBuf::from("board.brd")));
        assert!(parse_window_args(&args(&["--open", "a.brd", "b.brd"])).is_err());
    }

    #[test]
    fn the_picture_mode_has_its_defaults() {
        let parsed = parse_window_args(&args(&["--shot", "start.png"])).unwrap();
        assert_eq!(
            parsed.shot,
            Some(ShotRequest {
                output: PathBuf::from("start.png"),
                replace: false,
                size: DEFAULT_SIZE,
                theme: None,
                wait: DEFAULT_WAIT,
                window: ShotWindow::Start,
            })
        );
        let with_file = parse_window_args(&args(&["--shot", "b.PNG", "board.brd"])).unwrap();
        assert_eq!(with_file.shot.unwrap().window, ShotWindow::Document);
    }

    #[test]
    fn every_option_of_the_picture_is_read() {
        let parsed = parse_window_args(&args(&[
            "--shot",
            "3d.png",
            "--size",
            "1280x720",
            "--theme",
            "Light",
            "--wait",
            "12",
            "--window",
            "3D",
            "--replace",
            "--open",
            "board.brd",
            "--lang",
            "de",
        ]))
        .unwrap();
        assert_eq!(parsed.lang, Some(Lang::De));
        assert_eq!(parsed.file, Some(PathBuf::from("board.brd")));
        assert_eq!(
            parsed.shot,
            Some(ShotRequest {
                output: PathBuf::from("3d.png"),
                replace: true,
                size: (1280, 720),
                theme: Some(Theme::Light),
                wait: 12,
                window: ShotWindow::View3d,
            })
        );
    }

    #[test]
    fn a_mistake_in_the_picture_options_gives_its_reason() {
        for (list, reason) in [
            (vec!["--shot"], "needs a file to write"),
            (vec!["--shot", "a.jpg"], "must end in .png"),
            (vec!["--shot", "a.png", "--size"], "needs WIDTHxHEIGHT"),
            (
                vec!["--shot", "a.png", "--size", "10x10"],
                "from 320 to 8192",
            ),
            (
                vec!["--shot", "a.png", "--theme", "blue"],
                "dark, light or contrast",
            ),
            (vec!["--shot", "a.png", "--wait", "-1"], "from 0 to 3600"),
            (vec!["--shot", "a.png", "--wait", "3601"], "from 0 to 3600"),
            (
                vec!["--shot", "a.png", "--window", "oven"],
                "is one of start, document, 3d",
            ),
            (vec!["--size", "800x600"], "need --shot"),
            (vec!["--replace"], "need --shot"),
            (
                vec!["--shot", "a.png", "--window", "3d"],
                "needs a file to open",
            ),
            (
                vec!["--shot", "a.png", "--window", "panel"],
                "needs a file to open",
            ),
            (
                vec!["--shot", "a.png", "--window", "start", "b.brd"],
                "shows nothing open",
            ),
        ] {
            let error = parse_window_args(&args(&list)).unwrap_err();
            assert!(error.contains(reason), "{list:?}: {error}");
        }
    }

    #[test]
    fn a_window_of_the_toolbox_can_be_shown_with_or_without_a_file() {
        for window in ["search", "layers", "help", "shortcuts", "appearance"] {
            let alone = parse_window_args(&args(&["--shot", "a.png", "--window", window]));
            assert_eq!(alone.unwrap().shot.unwrap().window.name(), window);
            let with = parse_window_args(&args(&["--shot", "a.png", "--window", window, "b.brd"]));
            assert!(with.is_ok(), "{window}");
        }
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
