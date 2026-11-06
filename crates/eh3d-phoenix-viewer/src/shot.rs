// SPDX-License-Identifier: GPL-3.0-or-later
//! The picture mode of the window (`phoenix-gui --shot FILE.png`): what is asked for, read and
//! checked without a screen. The window opens at the size asked, shows the document and the
//! window asked for, waits a number of frames so that the drawing is complete, saves what it
//! drew as a PNG file and closes by itself. In this mode the window reads no file of
//! preferences, writes none and keeps no list of recent files, so a run leaves nothing of the
//! person's settings changed.

use std::path::PathBuf;

use crate::appearance::Theme;

/// The smallest side of the window of a picture, in pixels.
pub const SIZE_MIN: u32 = 320;
/// The largest side of the window of a picture, in pixels.
pub const SIZE_MAX: u32 = 8192;
/// The size of the window of a picture when `--size` is not given.
pub const DEFAULT_SIZE: (u32, u32) = (1480, 800);
/// The frames drawn before the picture is taken when `--wait` is not given.
pub const DEFAULT_WAIT: u32 = 6;
/// The most frames `--wait` may ask for.
pub const WAIT_MAX: u32 = 3600;

/// The window that is shown in the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotWindow {
    /// The start screen, with nothing open.
    Start,
    /// The document given with `--open`: a board, a schematic or a library.
    Document,
    /// The 3D window of the board given with `--open`.
    View3d,
    /// The window of the panel of boards, laid out from the file given with `--open`.
    Panel,
    /// The window of the imported Gerber and Excellon files, from the folder given with `--open`.
    Gerber,
    /// The search box.
    Search,
    /// The list of measures.
    Measures,
    /// The layers.
    Layers,
    /// The history of the steps.
    History,
    /// The appearance (theme, scale, fonts).
    Appearance,
    /// The shortcuts.
    Shortcuts,
    /// The help.
    Help,
}

impl ShotWindow {
    /// Every window that can be asked for.
    pub const ALL: [ShotWindow; 12] = [
        ShotWindow::Start,
        ShotWindow::Document,
        ShotWindow::View3d,
        ShotWindow::Panel,
        ShotWindow::Gerber,
        ShotWindow::Search,
        ShotWindow::Measures,
        ShotWindow::Layers,
        ShotWindow::History,
        ShotWindow::Appearance,
        ShotWindow::Shortcuts,
        ShotWindow::Help,
    ];

    /// The word that names the window on the command line.
    pub fn name(self) -> &'static str {
        match self {
            ShotWindow::Start => "start",
            ShotWindow::Document => "document",
            ShotWindow::View3d => "3d",
            ShotWindow::Panel => "panel",
            ShotWindow::Gerber => "gerber",
            ShotWindow::Search => "search",
            ShotWindow::Measures => "measures",
            ShotWindow::Layers => "layers",
            ShotWindow::History => "history",
            ShotWindow::Appearance => "appearance",
            ShotWindow::Shortcuts => "shortcuts",
            ShotWindow::Help => "help",
        }
    }

    /// The window a word names, in any case. `board`, `schematic` and `library` name the
    /// document, which shows whichever of them the file is.
    pub fn parse(text: &str) -> Option<ShotWindow> {
        let word = text.trim().to_ascii_lowercase();
        if matches!(word.as_str(), "board" | "schematic" | "library") {
            return Some(ShotWindow::Document);
        }
        ShotWindow::ALL.into_iter().find(|w| w.name() == word)
    }

    /// True when the window cannot be shown without a file to open (`--open`).
    pub fn needs_file(self) -> bool {
        matches!(
            self,
            ShotWindow::Document | ShotWindow::View3d | ShotWindow::Panel | ShotWindow::Gerber
        )
    }
}

/// What the picture mode was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotRequest {
    /// The PNG file to write.
    pub output: PathBuf,
    /// True when a file that exists may be replaced; otherwise it is an error.
    pub replace: bool,
    /// The size of the window in pixels, width and height.
    pub size: (u32, u32),
    /// The theme of the picture; none keeps the one of the settings.
    pub theme: Option<Theme>,
    /// The frames drawn before the picture is taken.
    pub wait: u32,
    /// The window shown.
    pub window: ShotWindow,
}

/// Reads a size written `WIDTHxHEIGHT` (`1280x720`, also with a capital X), each side between
/// [`SIZE_MIN`] and [`SIZE_MAX`].
pub fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (width, height) = text.trim().split_once(['x', 'X'])?;
    let width: u32 = width.trim().parse().ok()?;
    let height: u32 = height.trim().parse().ok()?;
    let ok = |side: u32| (SIZE_MIN..=SIZE_MAX).contains(&side);
    (ok(width) && ok(height)).then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_is_two_sides_inside_the_limits() {
        assert_eq!(parse_size("1280x720"), Some((1280, 720)));
        assert_eq!(parse_size(" 800 X 600 "), Some((800, 600)));
        assert_eq!(parse_size("320x8192"), Some((320, 8192)));
        for bad in [
            "", "1280", "x720", "1280x", "319x600", "800x8193", "-1x600", "1e3x600", "10x20x30",
            "800*600",
        ] {
            assert_eq!(parse_size(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn every_window_has_a_name_that_reads_back() {
        let mut names = std::collections::BTreeSet::new();
        for window in ShotWindow::ALL {
            assert_eq!(ShotWindow::parse(window.name()), Some(window));
            assert!(
                names.insert(window.name()),
                "{} is named twice",
                window.name()
            );
        }
        assert_eq!(names.len(), 12);
        assert_eq!(ShotWindow::parse(" 3D "), Some(ShotWindow::View3d));
        assert_eq!(ShotWindow::parse("Board"), Some(ShotWindow::Document));
        assert_eq!(ShotWindow::parse("library"), Some(ShotWindow::Document));
        assert_eq!(ShotWindow::parse("settings"), None);
    }

    #[test]
    fn the_windows_that_show_a_document_need_a_file() {
        let needing: Vec<&str> = ShotWindow::ALL
            .into_iter()
            .filter(|w| w.needs_file())
            .map(ShotWindow::name)
            .collect();
        assert_eq!(needing, ["document", "3d", "panel", "gerber"]);
    }
}
