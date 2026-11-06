// SPDX-License-Identifier: GPL-3.0-or-later
//! The appearance of the window, without a screen: the theme (dark, light or high contrast), the
//! scale of the interface and the size of the letters, with the limits that keep them usable, the
//! colours of the controls that go with each theme and the contrast between two colours, worked
//! out as the Web Content Accessibility Guidelines do. The colours of the drawing of each theme are
//! in the settings (`colour.light.<name>` and `colour.contrast.<name>`); the window only applies
//! what is decided here.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::scene::Scene;
use crate::settings::ViewerSettings;
use crate::window_text::WinText;

/// The smallest scale of the interface, in percent.
pub const SCALE_MIN: u32 = 50;
/// The largest scale of the interface, in percent.
pub const SCALE_MAX: u32 = 300;
/// The smallest size of the letters, in points.
pub const FONT_MIN: f64 = 6.0;
/// The largest size of the letters, in points.
pub const FONT_MAX: f64 = 48.0;
/// The size of the letters that the toolkit uses for the body text, in points; the other sizes
/// keep their proportion to it.
pub const BASE_FONT_PT: f64 = 14.0;

/// The look of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Light letters on a dark ground, the one the programs of the suite start with.
    Dark,
    /// Dark letters on a light ground.
    Light,
    /// Pure black and white and saturated colours.
    Contrast,
}

impl Theme {
    /// The three themes, in the order of the box that chooses one.
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Contrast];

    /// The word that names the theme in the settings.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Contrast => "contrast",
        }
    }

    /// The theme a word names, in any case.
    pub fn parse(text: &str) -> Option<Theme> {
        let word = text.trim().to_ascii_lowercase();
        Theme::ALL.into_iter().find(|t| t.name() == word)
    }

    /// The text that names the theme in the window.
    pub fn label(self) -> WinText {
        match self {
            Theme::Dark => WinText::ThemeDark,
            Theme::Light => WinText::ThemeLight,
            Theme::Contrast => WinText::ThemeContrast,
        }
    }
}

/// A scale of the interface brought into its limits.
pub fn clamp_scale(percent: u32) -> u32 {
    percent.clamp(SCALE_MIN, SCALE_MAX)
}

/// A size of the letters brought into its limits; a value that is not a number gives the base size.
pub fn clamp_font(points: f64) -> f64 {
    if points.is_finite() {
        points.clamp(FONT_MIN, FONT_MAX)
    } else {
        BASE_FONT_PT
    }
}

/// The factor by which the toolkit scales everything for a percentage.
pub fn zoom_factor(percent: u32) -> f32 {
    clamp_scale(percent) as f32 / 100.0
}

/// The factor by which every size of the letters is multiplied for a size of the body text.
pub fn font_factor(points: f64) -> f64 {
    clamp_font(points) / BASE_FONT_PT
}

/// The colours of the controls of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiPalette {
    /// The ground of the panels.
    pub panel: [u8; 4],
    /// The ground of the windows and of the raised controls.
    pub raised: [u8; 4],
    /// The letters.
    pub text: [u8; 4],
    /// The letters of the notes.
    pub dim: [u8; 4],
    /// The colour of the headings and of what is chosen.
    pub accent: [u8; 4],
    /// The colour of what is right.
    pub ok: [u8; 4],
    /// The colour of what is wrong.
    pub danger: [u8; 4],
}

/// The colours of the controls for the theme in use.
pub fn ui_palette(settings: &ViewerSettings) -> UiPalette {
    UiPalette {
        panel: settings.colour("ui_panel"),
        raised: settings.colour("ui_raised"),
        text: settings.colour("ui_text"),
        dim: settings.colour("ui_dim"),
        accent: settings.colour("ui_accent"),
        ok: settings.colour("ui_ok"),
        danger: settings.colour("ui_danger"),
    }
}

/// The colours of the controls in use, which the window sets when it starts and when the theme
/// changes and every panel reads when it draws (the dark theme until then).
static CURRENT: [AtomicU32; 7] = [
    AtomicU32::new(0x1713_10ff),
    AtomicU32::new(0x221c_17ff),
    AtomicU32::new(0xf4ec_e0ff),
    AtomicU32::new(0x9c8d_7aff),
    AtomicU32::new(0xffb0_00ff),
    AtomicU32::new(0x9be3_4aff),
    AtomicU32::new(0xff54_70ff),
];

fn pack(rgba: [u8; 4]) -> u32 {
    u32::from_be_bytes(rgba)
}

fn unpack(bits: u32) -> [u8; 4] {
    bits.to_be_bytes()
}

/// Makes a palette the one in use.
pub fn set_current(palette: UiPalette) {
    let colours = [
        palette.panel,
        palette.raised,
        palette.text,
        palette.dim,
        palette.accent,
        palette.ok,
        palette.danger,
    ];
    for (slot, colour) in CURRENT.iter().zip(colours) {
        slot.store(pack(colour), Ordering::Relaxed);
    }
}

/// The palette in use.
pub fn current() -> UiPalette {
    let at = |i: usize| unpack(CURRENT[i].load(Ordering::Relaxed));
    UiPalette {
        panel: at(0),
        raised: at(1),
        text: at(2),
        dim: at(3),
        accent: at(4),
        ok: at(5),
        danger: at(6),
    }
}

fn channel(value: u8) -> f64 {
    let s = f64::from(value) / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// The relative luminance of a colour, from 0 for black to 1 for white.
pub fn luminance(rgba: [u8; 4]) -> f64 {
    0.2126 * channel(rgba[0]) + 0.7152 * channel(rgba[1]) + 0.0722 * channel(rgba[2])
}

/// The contrast between two colours, from 1 for the same colour to 21 for black on white.
pub fn contrast_ratio(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

/// Gives every layer of a scene the colour its role has in the theme in use, and keeps whether it
/// is shown. A colour that was chosen by hand for a layer is lost.
pub fn recolour(scene: &mut Scene, settings: &ViewerSettings) {
    for layer in &mut scene.layers {
        layer.colour = settings.colour(layer.role.colour_key());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;

    #[test]
    fn a_theme_is_read_from_its_word_in_any_case() {
        for theme in Theme::ALL {
            assert_eq!(Theme::parse(theme.name()), Some(theme));
        }
        assert_eq!(Theme::parse(" Light "), Some(Theme::Light));
        assert_eq!(Theme::parse("CONTRAST"), Some(Theme::Contrast));
        assert_eq!(Theme::parse("blue"), None);
        assert_eq!(Theme::parse(""), None);
    }

    #[test]
    fn the_scale_and_the_letters_stay_within_their_limits() {
        assert_eq!(clamp_scale(0), SCALE_MIN);
        assert_eq!(clamp_scale(49), 50);
        assert_eq!(clamp_scale(100), 100);
        assert_eq!(clamp_scale(301), SCALE_MAX);
        assert_eq!(clamp_scale(u32::MAX), SCALE_MAX);
        assert_eq!(clamp_font(1.0), FONT_MIN);
        assert_eq!(clamp_font(100.0), FONT_MAX);
        assert_eq!(clamp_font(f64::NAN), BASE_FONT_PT);
        assert_eq!(clamp_font(f64::INFINITY), BASE_FONT_PT);
        assert!((zoom_factor(150) - 1.5).abs() < 1e-6);
        assert!((zoom_factor(10) - 0.5).abs() < 1e-6);
        assert!((font_factor(BASE_FONT_PT) - 1.0).abs() < 1e-12);
        assert!((font_factor(28.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_contrast_is_one_for_equal_colours_and_twenty_one_for_black_on_white() {
        let white = [255, 255, 255, 255];
        let black = [0, 0, 0, 255];
        assert!((contrast_ratio(white, white) - 1.0).abs() < 1e-9);
        assert!((contrast_ratio(white, black) - 21.0).abs() < 1e-9);
        assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn the_letters_of_every_theme_can_be_read_on_their_ground() {
        for (theme, least) in [
            (Theme::Dark, 7.0),
            (Theme::Light, 7.0),
            (Theme::Contrast, 15.0),
        ] {
            let mut settings = ViewerSettings::default();
            settings.set_theme(theme);
            let ui = ui_palette(&settings);
            assert!(
                contrast_ratio(ui.text, ui.panel) >= least,
                "{theme:?} text on panel"
            );
            assert!(
                contrast_ratio(ui.text, ui.raised) >= least,
                "{theme:?} text on raised"
            );
            assert!(
                contrast_ratio(ui.dim, ui.panel) >= 4.5,
                "{theme:?} notes on panel"
            );
            assert!(
                contrast_ratio(ui.accent, ui.panel) >= 4.5,
                "{theme:?} accent on panel"
            );
            assert!(
                contrast_ratio(ui.ok, ui.panel) >= 4.5,
                "{theme:?} ok on panel"
            );
            assert!(
                contrast_ratio(ui.danger, ui.panel) >= 4.5,
                "{theme:?} danger on panel"
            );
        }
    }

    #[test]
    fn the_drawing_of_every_theme_is_seen_against_its_ground() {
        for theme in Theme::ALL {
            let mut settings = ViewerSettings::default();
            settings.set_theme(theme);
            let ground = settings.colour("background");
            for name in [
                "top_copper",
                "bottom_copper",
                "outline",
                "silk_top",
                "wire",
                "symbol",
                "highlight",
                "measure",
            ] {
                assert!(
                    contrast_ratio(settings.colour(name), ground) >= 3.0,
                    "{theme:?} {name}"
                );
            }
        }
    }

    #[test]
    fn the_theme_changes_the_colours_in_use_and_back() {
        let mut settings = ViewerSettings::default();
        let dark = settings.colour("background");
        settings.set_theme(Theme::Light);
        assert_eq!(settings.theme, Theme::Light);
        assert_ne!(settings.colour("background"), dark);
        settings.set_theme(Theme::Contrast);
        assert_eq!(settings.colour("background"), [0, 0, 0, 255]);
        settings.set_theme(Theme::Dark);
        assert_eq!(settings.colour("background"), dark);
    }

    #[test]
    fn a_scene_takes_the_colours_of_the_theme() {
        let mut settings = ViewerSettings::default();
        let mut scene = sample::scene(&[], &[]);
        recolour(&mut scene, &settings);
        let dark_top = scene.layers[0].colour;
        assert_eq!(dark_top, settings.colour("top_copper"));
        settings.set_theme(Theme::Contrast);
        recolour(&mut scene, &settings);
        assert_eq!(scene.layers[0].colour, settings.colour("top_copper"));
        assert_ne!(scene.layers[0].colour, dark_top);
        assert!(scene.layers.iter().all(|l| l.visible));
    }

    #[test]
    fn the_palette_in_use_is_the_one_that_was_set() {
        let mut settings = ViewerSettings::default();
        settings.set_theme(Theme::Light);
        let light = ui_palette(&settings);
        set_current(light);
        assert_eq!(current(), light);
        settings.set_theme(Theme::Dark);
        let dark = ui_palette(&settings);
        set_current(dark);
        assert_eq!(current(), dark);
        assert_eq!(unpack(pack([1, 2, 3, 4])), [1, 2, 3, 4]);
        assert_eq!(pack([0x12, 0x34, 0x56, 0x78]), 0x1234_5678);
    }
}
