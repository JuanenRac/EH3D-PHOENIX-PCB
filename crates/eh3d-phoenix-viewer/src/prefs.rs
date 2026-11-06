// SPDX-License-Identifier: GPL-3.0-or-later
//! The preferences that the window remembers between runs, without a screen: the theme, the scale
//! of the interface, the size of the letters, the unit of the measures, the size of
//! the window, the zoom and the layers of each mode, and the layer presets. They live in a text file
//! in the settings folder of the user (named by the setting `prefs_file`), one `key = value` line
//! each, read strictly (an unknown key or a value out of its range is an error with its line) and
//! written to a side file that is renamed over the old one, so that a crash never leaves half a file.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::appearance::{Theme, FONT_MAX, FONT_MIN, SCALE_MAX, SCALE_MIN};
use crate::codes::{ViewError, SAVE_FAILED, SETTINGS_INVALID};
use crate::launcher_recent::config_dir;
use crate::layer_presets::{LayerPreset, PresetLayer, Presets};
use crate::measure_set::Unit;
use crate::settings::ViewerSettings;

/// The modes of the window that keep their own zoom and layers: the ones that draw a board or a
/// schematic in the workspace.
pub const MODES: [&str; 2] = ["board", "schematic"];

/// The smallest and the largest side of the window that is remembered, in pixels.
const WINDOW_MIN: u32 = 200;
const WINDOW_MAX: u32 = 16_000;

/// What the window remembers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prefs {
    /// The theme, when the person chose one.
    pub theme: Option<Theme>,
    /// The scale of the interface in percent, when the person chose one.
    pub ui_scale_percent: Option<u32>,
    /// The size of the letters in points, when the person chose one.
    pub font_size_pt: Option<f64>,
    /// The unit of the measures, when the person chose one.
    pub unit: Option<Unit>,
    /// The size of the window in pixels, width and height.
    pub window: Option<(u32, u32)>,
    /// The zoom of each mode, in pixels per millimetre.
    pub zoom: BTreeMap<String, f64>,
    /// The layers of each mode as they were left.
    pub layers: BTreeMap<String, Vec<PresetLayer>>,
    /// The layer presets that were saved.
    pub presets: Presets,
}

fn bad(line: usize, why: impl AsRef<str>) -> ViewError {
    ViewError::new(SETTINGS_INVALID, format!("line {line}: {}", why.as_ref()))
}

fn once<T>(slot: &mut Option<T>, value: T, line: usize, key: &str) -> Result<(), ViewError> {
    if slot.is_some() {
        return Err(bad(line, format!("{key} is given twice")));
    }
    *slot = Some(value);
    Ok(())
}

/// Reads `1600x900`: both sides inside the limits.
fn parse_size(value: &str) -> Option<(u32, u32)> {
    let (w, h) = value.split_once('x')?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    let range = WINDOW_MIN..=WINDOW_MAX;
    (range.contains(&w) && range.contains(&h)).then_some((w, h))
}

fn mode_of<'a>(key: &'a str, prefix: &str, line: usize) -> Result<&'a str, ViewError> {
    let mode = key.strip_prefix(prefix).unwrap_or("");
    if MODES.contains(&mode) {
        Ok(mode)
    } else {
        Err(bad(
            line,
            format!("{key}: the mode is not one of {}", MODES.join(", ")),
        ))
    }
}

impl Prefs {
    /// Reads the text of the file. `max_presets` is the most presets that are accepted.
    pub fn parse(text: &str, max_presets: usize) -> Result<Prefs, ViewError> {
        let mut prefs = Prefs {
            presets: Presets::new(max_presets),
            ..Prefs::default()
        };
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| bad(line, "expected key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "theme" => {
                    let theme = Theme::parse(value)
                        .ok_or_else(|| bad(line, "theme: dark, light or contrast"))?;
                    once(&mut prefs.theme, theme, line, key)?;
                }
                "ui_scale_percent" => {
                    let percent: u32 = value
                        .parse()
                        .ok()
                        .filter(|p| (SCALE_MIN..=SCALE_MAX).contains(p))
                        .ok_or_else(|| {
                            bad(
                                line,
                                format!("{key}: a whole number from {SCALE_MIN} to {SCALE_MAX}"),
                            )
                        })?;
                    once(&mut prefs.ui_scale_percent, percent, line, key)?;
                }
                "font_size_pt" => {
                    let points: f64 = value
                        .parse()
                        .ok()
                        .filter(|p: &f64| p.is_finite() && (FONT_MIN..=FONT_MAX).contains(p))
                        .ok_or_else(|| {
                            bad(
                                line,
                                format!("{key}: a number from {FONT_MIN} to {FONT_MAX}"),
                            )
                        })?;
                    once(&mut prefs.font_size_pt, points, line, key)?;
                }
                "unit" => {
                    let unit = Unit::parse(value).ok_or_else(|| bad(line, "unit: mm or mil"))?;
                    once(&mut prefs.unit, unit, line, key)?;
                }
                "window" => {
                    let size = parse_size(value).ok_or_else(|| {
                        bad(
                            line,
                            format!("{key}: WIDTHxHEIGHT in pixels, {WINDOW_MIN} to {WINDOW_MAX}"),
                        )
                    })?;
                    once(&mut prefs.window, size, line, key)?;
                }
                "preset" => {
                    let preset = LayerPreset::from_line(value).map_err(|why| bad(line, why))?;
                    prefs.presets.save(preset).map_err(|_| {
                        bad(line, "preset: the name is not valid or there are too many")
                    })?;
                }
                _ if key.starts_with("zoom.") => {
                    let mode = mode_of(key, "zoom.", line)?;
                    let zoom: f64 = value
                        .parse()
                        .ok()
                        .filter(|z: &f64| z.is_finite() && *z > 0.0)
                        .ok_or_else(|| bad(line, format!("{key}: a number above zero")))?;
                    if prefs.zoom.insert(mode.to_string(), zoom).is_some() {
                        return Err(bad(line, format!("{key} is given twice")));
                    }
                }
                _ if key.starts_with("layers.") => {
                    let mode = mode_of(key, "layers.", line)?;
                    let preset = LayerPreset::from_line(value).map_err(|why| bad(line, why))?;
                    if prefs
                        .layers
                        .insert(mode.to_string(), preset.layers)
                        .is_some()
                    {
                        return Err(bad(line, format!("{key} is given twice")));
                    }
                }
                _ => return Err(bad(line, format!("unknown key {key}"))),
            }
        }
        Ok(prefs)
    }

    /// The text of the file: the same preferences always give the same text.
    pub fn to_text(&self) -> String {
        let mut out = String::from("# preferences of the window; the window writes this file\n");
        if let Some(theme) = self.theme {
            out.push_str(&format!("theme = {}\n", theme.name()));
        }
        if let Some(percent) = self.ui_scale_percent {
            out.push_str(&format!("ui_scale_percent = {percent}\n"));
        }
        if let Some(points) = self.font_size_pt {
            out.push_str(&format!("font_size_pt = {points}\n"));
        }
        if let Some(unit) = self.unit {
            out.push_str(&format!("unit = {}\n", unit.name()));
        }
        if let Some((w, h)) = self.window {
            out.push_str(&format!("window = {w}x{h}\n"));
        }
        for (mode, zoom) in &self.zoom {
            out.push_str(&format!("zoom.{mode} = {zoom}\n"));
        }
        for (mode, layers) in &self.layers {
            let preset = LayerPreset {
                name: mode.clone(),
                layers: layers.clone(),
            };
            out.push_str(&format!("layers.{mode} = {}\n", preset.to_line()));
        }
        for preset in self.presets.items() {
            out.push_str(&format!("preset = {}\n", preset.to_line()));
        }
        out
    }

    /// Puts what was chosen into the settings. A value that the person never chose stays as the
    /// settings file has it.
    pub fn apply_to(&self, settings: &mut ViewerSettings) {
        if let Some(theme) = self.theme {
            settings.set_theme(theme);
        }
        if let Some(percent) = self.ui_scale_percent {
            settings.ui_scale_percent = percent.clamp(SCALE_MIN, SCALE_MAX);
        }
        if let Some(points) = self.font_size_pt {
            settings.font_size_pt = points.clamp(FONT_MIN, FONT_MAX);
        }
        if let Some(unit) = self.unit {
            settings.measure_unit = unit;
        }
    }

    /// Remembers the zoom of a mode; the answer is true when it changed by more than `step` of
    /// itself (so that a wheel that turns does not ask for a write at every frame).
    pub fn note_zoom(&mut self, mode: &str, scale: f64, step: f64) -> bool {
        if !(scale.is_finite() && scale > 0.0) || !MODES.contains(&mode) {
            return false;
        }
        match self.zoom.get(mode) {
            Some(old) if ((scale - old) / old).abs() <= step => false,
            _ => {
                self.zoom.insert(mode.to_string(), scale);
                true
            }
        }
    }

    /// Remembers the layers of a mode; the answer is true when they changed.
    pub fn note_layers(&mut self, mode: &str, layers: Vec<PresetLayer>) -> bool {
        if !MODES.contains(&mode) || self.layers.get(mode) == Some(&layers) {
            return false;
        }
        self.layers.insert(mode.to_string(), layers);
        true
    }
}

/// The file of the preferences in the settings folder of the user, named by the setting
/// `prefs_file`.
pub fn prefs_path(get: &dyn Fn(&str) -> Option<String>, file_name: &str) -> Option<PathBuf> {
    config_dir(get).map(|dir| dir.join(file_name))
}

/// Reads the preferences from a file. A file that is not there gives the empty preferences; one
/// that is larger than `max_bytes` or not valid is an error that says why.
pub fn load(path: &Path, max_bytes: u64, max_presets: usize) -> Result<Prefs, ViewError> {
    let failed =
        |e: std::io::Error| ViewError::new(SETTINGS_INVALID, format!("{}: {e}", path.display()));
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Prefs {
                presets: Presets::new(max_presets),
                ..Prefs::default()
            })
        }
        Err(e) => return Err(failed(e)),
    };
    if meta.len() > max_bytes {
        return Err(ViewError::new(
            SETTINGS_INVALID,
            format!("{}: larger than {max_bytes} bytes", path.display()),
        ));
    }
    let text = fs::read_to_string(path).map_err(failed)?;
    Prefs::parse(&text, max_presets).map_err(|e| {
        ViewError::new(
            SETTINGS_INVALID,
            format!("{}: {}", path.display(), e.detail),
        )
    })
}

/// Writes the preferences: the text goes to a side file next to the target, is synced, and the side
/// file is renamed over the target, trying again `retries` times, `wait_ms` apart, when the rename
/// fails (a program that scans the file may hold it for a moment). The folder is made if needed.
pub fn save(path: &Path, prefs: &Prefs, retries: u32, wait_ms: u64) -> Result<(), ViewError> {
    let failed =
        |e: std::io::Error| ViewError::new(SAVE_FAILED, format!("{}: {e}", path.display()));
    if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
        fs::create_dir_all(folder).map_err(failed)?;
    }
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".part");
    let side = path.with_file_name(name);
    {
        // a side file left by an interrupted save is ours and is removed; a link is not followed
        // (the exclusive creation below fails on it)
        if fs::symlink_metadata(&side).is_ok_and(|m| m.is_file()) {
            let _ = fs::remove_file(&side);
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&side)
            .map_err(failed)?;
        file.write_all(prefs.to_text().as_bytes()).map_err(failed)?;
        file.sync_all().map_err(failed)?;
    }
    let mut tries = 0u32;
    loop {
        match fs::rename(&side, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tries += 1;
                if tries > retries {
                    let _ = fs::remove_file(&side);
                    return Err(failed(e));
                }
                thread::sleep(Duration::from_millis(wait_ms));
            }
        }
    }
}

/// Decides when the window may write the preferences again, so that a wheel that turns does not
/// write a file at every frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Saver {
    pending: bool,
    last: f64,
}

impl Saver {
    /// Something changed and must be written.
    pub fn mark(&mut self) {
        self.pending = true;
    }

    /// True when something is waiting to be written.
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// True when a write should be made now: something is waiting and at least `interval_s` have
    /// passed since the last write (`now` is in seconds from any fixed start). The write counts as
    /// made when the answer is true.
    pub fn due(&mut self, now: f64, interval_s: f64) -> bool {
        if self.pending && (now - self.last >= interval_s || self.last == 0.0) {
            self.pending = false;
            self.last = now.max(f64::MIN_POSITIVE);
            true
        } else {
            false
        }
    }

    /// A write failed: it is waiting again.
    pub fn retry(&mut self) {
        self.pending = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Prefs {
        let mut prefs = Prefs {
            theme: Some(Theme::Light),
            ui_scale_percent: Some(125),
            font_size_pt: Some(15.5),
            unit: Some(Unit::Mil),
            window: Some((1600, 900)),
            presets: Presets::new(20),
            ..Prefs::default()
        };
        prefs.zoom.insert("board".to_string(), 12.5);
        prefs.zoom.insert("schematic".to_string(), 3.0);
        prefs.layers.insert(
            "board".to_string(),
            vec![
                PresetLayer {
                    name: "Top".to_string(),
                    visible: true,
                    opacity: 100,
                },
                PresetLayer {
                    name: "Bottom".to_string(),
                    visible: false,
                    opacity: 40,
                },
            ],
        );
        prefs
            .presets
            .save(LayerPreset {
                name: "Top only".to_string(),
                layers: vec![PresetLayer {
                    name: "Top".to_string(),
                    visible: true,
                    opacity: 100,
                }],
            })
            .unwrap();
        prefs
    }

    #[test]
    fn the_preferences_are_written_and_read_back_the_same() {
        let prefs = full();
        let text = prefs.to_text();
        assert_eq!(Prefs::parse(&text, 20).unwrap(), prefs);
        assert_eq!(prefs.to_text(), text);
        assert!(text.contains("theme = light\n"));
        assert!(text.contains("window = 1600x900\n"));
        let empty = Prefs::parse("", 20).unwrap();
        assert_eq!(
            empty.to_text(),
            "# preferences of the window; the window writes this file\n"
        );
        assert_eq!(Prefs::parse(&empty.to_text(), 20).unwrap(), empty);
    }

    #[test]
    fn a_bad_line_is_an_error_with_its_line_and_never_a_panic() {
        for text in [
            "nonsense",
            "unknown = 1",
            "theme = blue",
            "theme = dark\ntheme = light",
            "ui_scale_percent = 49",
            "ui_scale_percent = 301",
            "ui_scale_percent = abc",
            "font_size_pt = 5",
            "font_size_pt = nan",
            "unit = inch",
            "window = 100x100",
            "window = 800",
            "window = 800x900000",
            "zoom.nothing = 2",
            "zoom.board = 0",
            "zoom.board = -1",
            "zoom.board = 1\nzoom.board = 2",
            "layers.board = nope",
            "layers.board = a | Top:1:100\nlayers.board = a | Top:1:100",
            "preset = no bar",
            "preset =  | Top:1:100",
        ] {
            let error = Prefs::parse(text, 20).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
            assert!(
                error.detail.starts_with("line "),
                "{text}: {}",
                error.detail
            );
        }
        let error = Prefs::parse("\n\nbad", 20).unwrap_err();
        assert!(error.detail.starts_with("line 3"), "{}", error.detail);
    }

    #[test]
    fn too_many_presets_are_an_error_and_the_same_name_is_one_preset() {
        let mut text = String::new();
        for n in 0..10 {
            text.push_str(&format!("preset = p{n} | Top:1:100\n"));
        }
        // the list holds at least nine
        let error = Prefs::parse(&text, 0).unwrap_err();
        assert!(error.detail.starts_with("line 10"), "{}", error.detail);
        assert_eq!(Prefs::parse(&text, 10).unwrap().presets.items().len(), 10);
        let same = Prefs::parse("preset = a | Top:1:1\npreset = a | Top:0:1\n", 9).unwrap();
        assert_eq!(same.presets.items().len(), 1);
        assert!(!same.presets.items()[0].layers[0].visible);
    }

    #[test]
    fn the_choices_go_into_the_settings_and_the_rest_stays() {
        let mut settings = ViewerSettings::default();
        Prefs::default().apply_to(&mut settings);
        assert_eq!(settings, ViewerSettings::default());
        full().apply_to(&mut settings);
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.ui_scale_percent, 125);
        assert_eq!(settings.font_size_pt, 15.5);
        assert_eq!(settings.measure_unit, Unit::Mil);
        assert_eq!(
            settings.colour("background"),
            settings.palette(Theme::Light)["background"]
        );
        // the other settings are untouched
        assert_eq!(settings.snap_mm, ViewerSettings::default().snap_mm);
    }

    #[test]
    fn a_zoom_is_remembered_only_when_it_changes_enough() {
        let mut prefs = Prefs::default();
        assert!(prefs.note_zoom("board", 10.0, 0.05));
        assert!(!prefs.note_zoom("board", 10.2, 0.05));
        assert!(prefs.note_zoom("board", 12.0, 0.05));
        assert_eq!(prefs.zoom["board"], 12.0);
        assert!(!prefs.note_zoom("nothing", 3.0, 0.05));
        assert!(!prefs.note_zoom("board", f64::NAN, 0.05));
        assert!(!prefs.note_zoom("board", 0.0, 0.05));
        assert_eq!(prefs.zoom.len(), 1);
    }

    #[test]
    fn the_layers_are_remembered_only_when_they_change() {
        let mut prefs = Prefs::default();
        let one = vec![PresetLayer {
            name: "Top".to_string(),
            visible: true,
            opacity: 100,
        }];
        assert!(prefs.note_layers("board", one.clone()));
        assert!(!prefs.note_layers("board", one.clone()));
        let mut hidden = one;
        hidden[0].visible = false;
        assert!(prefs.note_layers("board", hidden));
        assert!(!prefs.note_layers("nothing", Vec::new()));
    }

    #[test]
    fn the_file_is_in_the_settings_folder_under_the_name_of_the_setting() {
        let get = |name: &str| (name == "HOME").then(|| "/h".to_string());
        let path = prefs_path(&get, "prefs.cfg").unwrap();
        assert!(path.ends_with("prefs.cfg"));
        assert!(path.to_string_lossy().contains("eh3d-phoenix-pcb"));
        let none = |_: &str| None;
        assert_eq!(prefs_path(&none, "prefs.cfg"), None);
    }

    #[test]
    fn saving_writes_a_side_file_and_replaces_the_old_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep").join("prefs.cfg");
        let prefs = full();
        save(&path, &prefs, 2, 1).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), prefs.to_text());
        assert!(!dir.path().join("deep").join("prefs.cfg.part").exists());
        let mut changed = prefs.clone();
        changed.unit = Some(Unit::Mm);
        save(&path, &changed, 2, 1).unwrap();
        let back = load(&path, 1 << 20, 20).unwrap();
        assert_eq!(back, changed);
        assert_ne!(back, prefs);
    }

    #[test]
    fn a_failed_save_says_so_and_leaves_no_side_file() {
        let dir = tempfile::tempdir().unwrap();
        // the target is a folder: the rename cannot replace it
        let target = dir.path().join("prefs.cfg");
        fs::create_dir(&target).unwrap();
        let error = save(&target, &full(), 1, 1).unwrap_err();
        assert_eq!(error.code, SAVE_FAILED);
        assert!(!dir.path().join("prefs.cfg.part").exists());
        // the parent is a file: the folder cannot be made
        let file = dir.path().join("file");
        fs::write(&file, b"x").unwrap();
        let error = save(&file.join("prefs.cfg"), &full(), 0, 1).unwrap_err();
        assert_eq!(error.code, SAVE_FAILED);
    }

    #[test]
    fn a_side_file_left_by_an_interrupted_save_does_not_block_the_next_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.cfg");
        fs::write(dir.path().join("prefs.cfg.part"), b"half a file").unwrap();
        save(&path, &full(), 1, 1).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), full().to_text());
        assert!(!dir.path().join("prefs.cfg.part").exists());
    }

    #[test]
    fn loading_a_missing_file_gives_the_empty_preferences_and_a_bad_one_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("none.cfg");
        let prefs = load(&missing, 1 << 20, 20).unwrap();
        assert_eq!(prefs.theme, None);
        assert!(prefs.presets.items().is_empty());
        let broken = dir.path().join("broken.cfg");
        fs::write(&broken, "theme = blue\n").unwrap();
        let error = load(&broken, 1 << 20, 20).unwrap_err();
        assert_eq!(error.code, SETTINGS_INVALID);
        assert!(error.detail.contains("broken.cfg"), "{}", error.detail);
        assert!(error.detail.contains("line 1"), "{}", error.detail);
        let large = dir.path().join("large.cfg");
        fs::write(&large, "# x\n".repeat(100)).unwrap();
        assert!(load(&large, 50, 20).is_err());
    }

    #[test]
    fn the_saver_writes_at_most_once_in_each_interval() {
        let mut saver = Saver::default();
        assert!(!saver.due(10.0, 1.0));
        saver.mark();
        assert!(saver.pending());
        assert!(saver.due(10.0, 1.0));
        assert!(!saver.pending());
        saver.mark();
        assert!(!saver.due(10.5, 1.0));
        assert!(saver.pending());
        assert!(saver.due(11.2, 1.0));
        saver.mark();
        saver.retry();
        assert!(saver.pending());
    }
}
