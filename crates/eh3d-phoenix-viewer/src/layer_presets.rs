// SPDX-License-Identifier: GPL-3.0-or-later
//! The layers of the window as the person arranges them, without a screen: presets that are saved
//! with a name (which layers are shown, in which order they are listed and how opaque each is),
//! the keys 1 to 9 that apply the first nine, the view of only the active layer and the
//! highlight of the net of the selection. A preset is one line of the file of preferences.

use crate::scene::Scene;
use crate::settings::ViewerSettings;
use crate::viewer::Viewer;
use crate::window_text::WinText;

/// What a preset says about one layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetLayer {
    /// The name of the layer.
    pub name: String,
    /// Whether it is shown.
    pub visible: bool,
    /// How opaque it is, in percent of the opacity its colour has in the settings.
    pub opacity: u8,
}

/// A named arrangement of layers: the layers in the order they are listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPreset {
    /// The name that the person gave it.
    pub name: String,
    /// The layers, in the order of the list.
    pub layers: Vec<PresetLayer>,
}

/// The letters that the line of a preset uses to separate its parts are written as `%` and two
/// digits of hexadecimal inside a name.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '%' => out.push_str("%25"),
            ',' => out.push_str("%2C"),
            ':' => out.push_str("%3A"),
            '|' => out.push_str("%7C"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            other => out.push(other),
        }
    }
    out
}

fn unescape(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let code = rest
            .get(at + 1..at + 3)
            .ok_or_else(|| format!("{text}: a % without two digits"))?;
        let c = match code {
            "25" => '%',
            "2C" => ',',
            "3A" => ':',
            "7C" => '|',
            "0A" => '\n',
            "0D" => '\r',
            _ => return Err(format!("{text}: %{code} is not a known code")),
        };
        out.push(c);
        rest = &rest[at + 3..];
    }
    out.push_str(rest);
    Ok(out)
}

impl LayerPreset {
    /// The preset as one line: `name | layer:1:100, layer:0:50` (shown 1 or 0, opacity in percent).
    pub fn to_line(&self) -> String {
        let layers: Vec<String> = self
            .layers
            .iter()
            .map(|l| format!("{}:{}:{}", escape(&l.name), u8::from(l.visible), l.opacity))
            .collect();
        format!("{} | {}", escape(&self.name), layers.join(", "))
    }

    /// Reads a line written by [`LayerPreset::to_line`].
    pub fn from_line(line: &str) -> Result<LayerPreset, String> {
        let (name, rest) = line
            .split_once('|')
            .ok_or_else(|| "a preset is name | layers".to_string())?;
        let name = unescape(name.trim())?;
        if !valid_name(&name) {
            return Err("the name of a preset is not valid".to_string());
        }
        let mut layers = Vec::new();
        for part in rest.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let fields: Vec<&str> = part.split(':').collect();
            let [layer, visible, opacity] = fields.as_slice() else {
                return Err(format!("{part}: expected layer:shown:opacity"));
            };
            let visible = match *visible {
                "1" => true,
                "0" => false,
                other => return Err(format!("{part}: {other} is not 0 or 1")),
            };
            let opacity: u8 = opacity
                .parse()
                .ok()
                .filter(|o| *o <= 100)
                .ok_or_else(|| format!("{part}: the opacity is a number from 0 to 100"))?;
            layers.push(PresetLayer {
                name: unescape(layer)?,
                visible,
                opacity,
            });
        }
        Ok(LayerPreset { name, layers })
    }
}

/// True for a name that is not empty and has no line break.
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.contains(['\n', '\r'])
}

/// The presets that were saved, in the order they were made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Presets {
    list: Vec<LayerPreset>,
    max: usize,
}

impl Presets {
    /// An empty list that holds at most `max` presets (at least nine, one for each key).
    pub fn new(max: usize) -> Presets {
        Presets {
            list: Vec::new(),
            max: max.max(9),
        }
    }

    /// The presets.
    pub fn items(&self) -> &[LayerPreset] {
        &self.list
    }

    /// The preset at a place from zero.
    pub fn get(&self, at: usize) -> Option<&LayerPreset> {
        self.list.get(at)
    }

    /// Saves a preset: one with the same name is replaced, otherwise it goes last. The answer is the
    /// text to show when it cannot be saved.
    pub fn save(&mut self, preset: LayerPreset) -> Result<(), WinText> {
        if !valid_name(&preset.name) {
            return Err(WinText::PresetBadName);
        }
        let preset = LayerPreset {
            name: preset.name.trim().to_string(),
            ..preset
        };
        match self.list.iter().position(|p| p.name == preset.name) {
            Some(at) => self.list[at] = preset,
            None if self.list.len() >= self.max.max(9) => return Err(WinText::PresetFull),
            None => self.list.push(preset),
        }
        Ok(())
    }

    /// Takes one out; the answer is false when there is none at that place.
    pub fn remove(&mut self, at: usize) -> bool {
        if at < self.list.len() {
            self.list.remove(at);
            true
        } else {
            false
        }
    }
}

/// The layers as they are now, in the order of the list: `order` holds the places of the layers of
/// the scene in the order they are listed (the layers that it leaves out follow in the order of the
/// scene).
pub fn capture(scene: &Scene, order: &[usize], settings: &ViewerSettings) -> Vec<PresetLayer> {
    complete_order(order, scene.layers.len())
        .into_iter()
        .map(|at| {
            let layer = &scene.layers[at];
            let base = settings.colour(layer.role.colour_key())[3];
            let opacity = if base == 0 {
                100
            } else {
                (u32::from(layer.colour[3]) * 100 / u32::from(base)).min(100) as u8
            };
            PresetLayer {
                name: layer.name.clone(),
                visible: layer.visible,
                opacity,
            }
        })
        .collect()
}

/// A list of places of layers made whole: the ones that exist and are not repeated, then the rest in
/// order.
pub fn complete_order(order: &[usize], count: usize) -> Vec<usize> {
    let mut whole: Vec<usize> = Vec::with_capacity(count);
    for &at in order {
        if at < count && !whole.contains(&at) {
            whole.push(at);
        }
    }
    for at in 0..count {
        if !whole.contains(&at) {
            whole.push(at);
        }
    }
    whole
}

/// Sets how opaque a layer is, in percent of the opacity its colour has in the settings; a place
/// that does not exist changes nothing.
pub fn set_opacity(scene: &mut Scene, layer: usize, percent: u8, settings: &ViewerSettings) {
    if let Some(l) = scene.layers.get_mut(layer) {
        let base = u32::from(settings.colour(l.role.colour_key())[3]);
        l.colour[3] = (base * u32::from(percent.min(100)) / 100) as u8;
    }
}

/// What applying a preset did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The places of the layers of the scene in the order they are listed now.
    pub order: Vec<usize>,
    /// How many layers of the scene the preset named.
    pub matched: usize,
}

/// Applies a preset to the layers of a scene: a layer it names is shown or hidden and gets the
/// opacity, and the list follows its order; a layer it does not name stays as it is and goes after
/// the others in the list. A name of the preset that the scene does not have is ignored.
pub fn apply(scene: &mut Scene, preset: &LayerPreset, settings: &ViewerSettings) -> Applied {
    let mut order = Vec::new();
    for wanted in &preset.layers {
        let found = scene
            .layers
            .iter()
            .position(|l| l.name == wanted.name)
            .filter(|at| !order.contains(at));
        if let Some(at) = found {
            scene.layers[at].visible = wanted.visible;
            set_opacity(scene, at, wanted.opacity, settings);
            order.push(at);
        }
    }
    let matched = order.len();
    Applied {
        order: complete_order(&order, scene.layers.len()),
        matched,
    }
}

/// The layer that the person works on and the memory of what was shown before the view of only
/// that layer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerControl {
    /// The active layer, a place in the layers of the scene.
    pub active: Option<usize>,
    solo_saved: Option<Vec<bool>>,
}

impl LayerControl {
    /// Chooses the active layer; a place that does not exist changes nothing.
    pub fn set_active(&mut self, scene: &Scene, layer: usize) -> bool {
        if layer < scene.layers.len() {
            self.active = Some(layer);
            true
        } else {
            false
        }
    }

    /// True while only the active layer is shown.
    pub fn is_solo(&self) -> bool {
        self.solo_saved.is_some()
    }

    /// Shows only the active layer, or brings back what was shown before. The answer is true when
    /// only the active layer is shown now. With no active layer nothing changes.
    pub fn toggle_solo(&mut self, scene: &mut Scene) -> bool {
        if let Some(saved) = self.solo_saved.take() {
            for (at, layer) in scene.layers.iter_mut().enumerate() {
                layer.visible = saved.get(at).copied().unwrap_or(true);
            }
            return false;
        }
        let Some(active) = self.active.filter(|a| *a < scene.layers.len()) else {
            return false;
        };
        self.solo_saved = Some(scene.layers.iter().map(|l| l.visible).collect());
        for (at, layer) in scene.layers.iter_mut().enumerate() {
            layer.visible = at == active;
        }
        true
    }

    /// Forgets the memory of the view of only one layer, for a scene that was replaced.
    pub fn forget(&mut self) {
        self.solo_saved = None;
    }
}

/// The layer of the first primitive of the selected item, when something is selected.
pub fn layer_of_selection(viewer: &Viewer) -> Option<usize> {
    let item = viewer.selected?;
    viewer
        .scene
        .prims
        .iter()
        .find(|p| p.item == item)
        .map(|p| p.layer as usize)
}

/// Lights the net of the selected item and lets go of the item. The answer is false when nothing
/// is selected or the item has no net.
pub fn highlight_selected_net(viewer: &mut Viewer) -> bool {
    match viewer.selected_item().and_then(|item| item.net) {
        Some(net) => {
            viewer.select_net(net);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim::{Shape, P};
    use crate::sample;
    use crate::scene::ItemKind;

    fn scene() -> Scene {
        sample::scene(
            &["GND"],
            &[(
                ItemKind::Pad,
                "R1 pad 1",
                Some(0),
                Shape::Circle {
                    centre: P::new(0.0, 0.0),
                    radius: 1.0,
                },
            )],
        )
    }

    fn preset(name: &str, layers: &[(&str, bool, u8)]) -> LayerPreset {
        LayerPreset {
            name: name.to_string(),
            layers: layers
                .iter()
                .map(|(n, v, o)| PresetLayer {
                    name: (*n).to_string(),
                    visible: *v,
                    opacity: *o,
                })
                .collect(),
        }
    }

    #[test]
    fn a_preset_is_written_as_a_line_and_read_back() {
        let p = preset("Top only", &[("Top", true, 100), ("Bottom", false, 40)]);
        assert_eq!(p.to_line(), "Top only | Top:1:100, Bottom:0:40");
        assert_eq!(LayerPreset::from_line(&p.to_line()).unwrap(), p);
        // the letters of the format inside a name are written with a code
        let odd = preset("a|b%c", &[("1,2:3", true, 5), ("x\ny", false, 0)]);
        let line = odd.to_line();
        assert_eq!(line.lines().count(), 1);
        assert_eq!(LayerPreset::from_line(&line).unwrap(), odd);
        // a preset without layers is allowed
        let empty = preset("none", &[]);
        assert_eq!(LayerPreset::from_line(&empty.to_line()).unwrap(), empty);
    }

    #[test]
    fn a_bad_line_is_an_error_and_never_a_panic() {
        for bad in [
            "no bar",
            " | Top:1:100",
            "n | Top:1",
            "n | Top:2:100",
            "n | Top:1:101",
            "n | Top:1:-1",
            "n | Top:1:abc",
            "n | Top:1:100:7",
            "n | %",
            "n | %Z9:1:1",
            "n%7 | Top:1:1",
        ] {
            assert!(LayerPreset::from_line(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_name_must_be_on_one_line_and_not_empty() {
        assert!(valid_name("Top"));
        assert!(valid_name(" a b "));
        assert!(!valid_name(""));
        assert!(!valid_name("   "));
        assert!(!valid_name("a\nb"));
        assert!(!valid_name("a\rb"));
    }

    #[test]
    fn presets_are_saved_replaced_and_taken_out() {
        let mut list = Presets::new(0);
        assert!(list.items().is_empty());
        assert_eq!(list.save(preset("", &[])), Err(WinText::PresetBadName));
        assert_eq!(list.save(preset("a\nb", &[])), Err(WinText::PresetBadName));
        assert!(list.save(preset(" One ", &[("Top", true, 100)])).is_ok());
        assert!(list.save(preset("Two", &[])).is_ok());
        assert_eq!(list.items()[0].name, "One");
        // the same name replaces
        assert!(list.save(preset("One", &[("Bottom", true, 50)])).is_ok());
        assert_eq!(list.items().len(), 2);
        assert_eq!(list.get(0).unwrap().layers[0].name, "Bottom");
        assert!(list.get(2).is_none());
        assert!(!list.remove(2));
        assert!(list.remove(0));
        assert_eq!(list.items()[0].name, "Two");
    }

    #[test]
    fn the_list_never_holds_less_than_nine_and_stops_at_its_limit() {
        let mut list = Presets::new(3);
        for n in 0..9 {
            assert!(list.save(preset(&format!("p{n}"), &[])).is_ok(), "{n}");
        }
        assert_eq!(list.save(preset("tenth", &[])), Err(WinText::PresetFull));
        // a name that exists is still replaced when the list is full
        assert!(list.save(preset("p0", &[("Top", false, 0)])).is_ok());
        assert_eq!(list.items().len(), 9);
    }

    #[test]
    fn the_layers_are_captured_in_the_order_of_the_list() {
        let mut s = scene();
        s.layers[1].visible = false;
        s.layers[2].colour[3] = 128;
        let settings = ViewerSettings::default();
        let layers = capture(&s, &[2, 0], &settings);
        let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["tPlace", "Top", "Bottom"]);
        assert!(layers[1].visible && !layers[2].visible);
        assert_eq!(layers[0].opacity, 50);
        assert_eq!(layers[1].opacity, 100);
    }

    #[test]
    fn a_list_of_places_is_made_whole() {
        assert_eq!(complete_order(&[], 3), [0, 1, 2]);
        assert_eq!(complete_order(&[2, 2, 9, 0], 3), [2, 0, 1]);
        assert!(complete_order(&[1], 0).is_empty());
    }

    #[test]
    fn applying_a_preset_shows_hides_and_sets_the_opacity_and_the_order() {
        let mut s = scene();
        let settings = ViewerSettings::default();
        let p = preset(
            "mine",
            &[
                ("tPlace", true, 100),
                ("Missing", true, 10),
                ("Top", false, 25),
            ],
        );
        let done = apply(&mut s, &p, &settings);
        assert_eq!(done.matched, 2);
        assert_eq!(done.order, [2, 0, 1]);
        assert!(s.layers[2].visible && !s.layers[0].visible);
        // the layer that the preset does not name stays as it was
        assert!(s.layers[1].visible);
        let base = settings.colour(s.layers[0].role.colour_key())[3];
        assert_eq!(u32::from(s.layers[0].colour[3]), u32::from(base) * 25 / 100);
        // applying again gives the same
        let again = apply(&mut s, &p, &settings);
        assert_eq!(again, done);
        assert_eq!(u32::from(s.layers[0].colour[3]), u32::from(base) * 25 / 100);
    }

    #[test]
    fn the_opacity_of_a_layer_is_a_part_of_the_opacity_of_its_colour() {
        let mut s = scene();
        let settings = ViewerSettings::default();
        set_opacity(&mut s, 0, 50, &settings);
        assert_eq!(s.layers[0].colour[3], 127);
        set_opacity(&mut s, 0, 100, &settings);
        assert_eq!(s.layers[0].colour[3], 255);
        set_opacity(&mut s, 0, 250, &settings);
        assert_eq!(s.layers[0].colour[3], 255);
        set_opacity(&mut s, 1, 0, &settings);
        assert_eq!(s.layers[1].colour[3], 0);
        let before = s.clone();
        set_opacity(&mut s, 9, 10, &settings);
        assert_eq!(s, before);
    }

    #[test]
    fn a_captured_preset_applied_to_a_changed_scene_brings_the_state_back() {
        let settings = ViewerSettings::default();
        let mut s = scene();
        s.layers[0].visible = false;
        s.layers[1].colour[3] = 200;
        let saved = LayerPreset {
            name: "x".to_string(),
            layers: capture(&s, &[1, 2, 0], &settings),
        };
        let mut t = scene();
        let done = apply(&mut t, &saved, &settings);
        assert_eq!(done.order, [1, 2, 0]);
        assert_eq!(done.matched, 3);
        assert!(!t.layers[0].visible && t.layers[1].visible);
        assert!((i32::from(t.layers[1].colour[3]) - 200).abs() <= 3);
    }

    #[test]
    fn only_the_active_layer_is_shown_and_then_everything_comes_back() {
        let mut s = scene();
        s.layers[2].visible = false;
        let mut control = LayerControl::default();
        // with no active layer nothing happens
        assert!(!control.toggle_solo(&mut s));
        assert!(!control.is_solo() && s.layers[0].visible && s.layers[1].visible);
        assert!(!control.set_active(&s, 7));
        assert!(control.set_active(&s, 1));
        assert!(control.toggle_solo(&mut s));
        assert!(control.is_solo());
        let shown: Vec<bool> = s.layers.iter().map(|l| l.visible).collect();
        assert_eq!(shown, [false, true, false]);
        assert!(!control.toggle_solo(&mut s));
        let back: Vec<bool> = s.layers.iter().map(|l| l.visible).collect();
        assert_eq!(back, [true, true, false]);
        assert!(!control.is_solo());
    }

    #[test]
    fn the_memory_of_the_solo_view_survives_a_scene_with_fewer_layers() {
        let mut s = scene();
        let mut control = LayerControl::default();
        control.set_active(&s, 0);
        control.toggle_solo(&mut s);
        s.layers.truncate(2);
        assert!(!control.toggle_solo(&mut s));
        assert!(s.layers.iter().all(|l| l.visible));
        control.set_active(&s, 0);
        control.toggle_solo(&mut s);
        control.forget();
        assert!(!control.is_solo());
    }

    #[test]
    fn the_net_of_the_selection_is_lit_and_the_item_is_let_go() {
        let mut v = Viewer::new(scene(), ViewerSettings::default());
        assert!(!highlight_selected_net(&mut v));
        assert_eq!(layer_of_selection(&v), None);
        v.select_item(0);
        assert_eq!(layer_of_selection(&v), Some(0));
        assert!(highlight_selected_net(&mut v));
        assert_eq!(v.highlight, Some(0));
        assert!(v.selected.is_none());
        assert!(!highlight_selected_net(&mut v));
    }
}
