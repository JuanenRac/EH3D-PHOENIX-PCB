// SPDX-License-Identifier: GPL-3.0-or-later
//! The state of the viewer, without a screen: the scene, the view, the visibility and the colour
//! of each layer, the selection by position with a tolerance, the highlight of a net and the
//! measure. The window only passes on what the person does (a pixel under the pointer, a click, a
//! turn of the wheel) and draws what this state says.

use eh3d_phoenix_rules::Lang;

use crate::grid::Grid;
use crate::measure::Measure;
use crate::msg::Msg;
use crate::prim::{Bounds, Shape, P};
use crate::scene::{DocRef, Item, Scene, SceneKind};
use crate::settings::ViewerSettings;
use crate::view::{View, ZoomLimits};

/// What a click does. The tools after `Measure` edit a board and are put to work by the
/// [`crate::workspace::Workspace`]; the viewer itself only selects and measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Select the item under the pointer and highlight its net.
    Select,
    /// Measure the distance between two points.
    Measure,
    /// Drag a component to another place.
    MoveComponent,
    /// Turn the component under the click by one step.
    RotateComponent,
    /// Delete the track under the click.
    DeleteTrack,
    /// Drag a vertex of a track to another place.
    MoveVertex,
    /// Give the track under the click the active width.
    SetWidth,
    /// Add a via on the copper under the click.
    AddVia,
    /// Route a track between two elements: click the origin, then the destination.
    Route,
}

impl Tool {
    /// True for the tools that change a board.
    pub fn edits(self) -> bool {
        !matches!(self, Tool::Select | Tool::Measure)
    }
}

/// The viewer of one scene.
#[derive(Debug, Clone)]
pub struct Viewer {
    /// What is drawn.
    pub scene: Scene,
    /// Where and how large.
    pub view: View,
    /// The settings in use.
    pub settings: ViewerSettings,
    /// What a click does.
    pub tool: Tool,
    /// The net that is highlighted.
    pub highlight: Option<u32>,
    /// The item that is selected.
    pub selected: Option<u32>,
    /// The measure.
    pub measure: Measure,
    /// The point of the drawing under the pointer, when the pointer is over the drawing.
    pub cursor: Option<P>,
    grid: Grid,
}

/// The order of preference among the primitives found at a point.
#[derive(Clone, Copy, PartialEq)]
struct Rank {
    /// A text counts only when nothing else is there.
    solid: bool,
    layer: u32,
    distance: f64,
    area: f64,
    index: u32,
}

impl Rank {
    /// True when `self` is a better choice than `other`: not a text, then the layer drawn last,
    /// then the nearest, then the smallest, then the last one.
    fn beats(&self, other: &Rank) -> bool {
        if self.solid != other.solid {
            return self.solid;
        }
        if self.layer != other.layer {
            return self.layer > other.layer;
        }
        if self.distance != other.distance {
            return self.distance < other.distance;
        }
        if self.area != other.area {
            return self.area < other.area;
        }
        self.index > other.index
    }
}

impl Viewer {
    /// A viewer of a scene, with the drawing fitted to the window.
    pub fn new(scene: Scene, settings: ViewerSettings) -> Viewer {
        let grid = Grid::build(&scene.prims, scene.bounds, &settings);
        let mut viewer = Viewer {
            scene,
            view: View::default(),
            settings,
            tool: Tool::Select,
            highlight: None,
            selected: None,
            measure: Measure::default(),
            cursor: None,
            grid,
        };
        viewer.fit();
        viewer
    }

    /// The spatial index, for the reports of the benchmark.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// The limits of the zoom from the settings.
    pub fn limits(&self) -> ZoomLimits {
        ZoomLimits {
            min: self.settings.zoom_min_px_per_mm,
            max: self.settings.zoom_max_px_per_mm,
        }
    }

    /// Fits the whole drawing to the window.
    pub fn fit(&mut self) {
        let limits = self.limits();
        self.view
            .fit(self.scene.bounds, self.settings.fit_margin_fraction, limits);
    }

    /// Tells the viewer how large the window is now.
    pub fn resize(&mut self, width_px: f64, height_px: f64) {
        self.view.resize(width_px, height_px);
    }

    /// The drawing follows a drag of the pointer.
    pub fn pan_px(&mut self, dx: f64, dy: f64) {
        self.view.pan_px(dx, dy);
    }

    /// Zooms about a pixel: `scroll` is the scroll of the wheel in points and `pinch` the factor
    /// of a pinch gesture (1 for none).
    pub fn zoom_at_px(&mut self, sx: f64, sy: f64, scroll: f64, pinch: f64) {
        let factor = (scroll * self.settings.zoom_wheel_rate).exp() * pinch;
        let limits = self.limits();
        self.view.zoom_at(sx, sy, factor, limits);
    }

    /// The distance in millimetres that a click may miss an item by.
    pub fn tolerance_mm(&self) -> f64 {
        self.settings.pick_tolerance_px / self.view.scale
    }

    /// Shows or hides a layer.
    pub fn set_layer_visible(&mut self, layer: usize, visible: bool) {
        if let Some(l) = self.scene.layers.get_mut(layer) {
            l.visible = visible;
        }
    }

    /// Shows or hides every layer.
    pub fn set_all_visible(&mut self, visible: bool) {
        for l in &mut self.scene.layers {
            l.visible = visible;
        }
    }

    /// Changes the colour of a layer.
    pub fn set_layer_colour(&mut self, layer: usize, rgba: [u8; 4]) {
        if let Some(l) = self.scene.layers.get_mut(layer) {
            l.colour = rgba;
        }
    }

    /// The primitives of the visible layers that touch an area, in the order of drawing: by
    /// layer and then as they come in the document.
    pub fn visible(&self, area: Bounds) -> Vec<u32> {
        let mut found = Vec::new();
        self.grid.query(area, &mut found);
        found.retain(|&i| self.layer_visible(i));
        found.sort_unstable_by_key(|&i| (self.scene.prims[i as usize].layer, i));
        found
    }

    fn layer_visible(&self, prim: u32) -> bool {
        let layer = self.scene.prims[prim as usize].layer as usize;
        self.scene.layers.get(layer).is_some_and(|l| l.visible)
    }

    fn rank(&self, index: u32, p: P) -> Rank {
        let prim = &self.scene.prims[index as usize];
        Rank {
            solid: !matches!(prim.shape, Shape::Text { .. }),
            layer: prim.layer,
            distance: prim.distance(p),
            area: self.grid.bounds_of(index).area(),
            index,
        }
    }

    fn best_of(
        &self,
        candidates: impl IntoIterator<Item = u32>,
        p: P,
        tolerance: f64,
    ) -> Option<u32> {
        let mut best: Option<Rank> = None;
        for i in candidates {
            if !self.layer_visible(i) {
                continue;
            }
            let rank = self.rank(i, p);
            if rank.distance > tolerance {
                continue;
            }
            if best.as_ref().is_none_or(|b| rank.beats(b)) {
                best = Some(rank);
            }
        }
        best.map(|r| r.index)
    }

    /// The primitive under a point of the drawing, within `tolerance` millimetres, found with the
    /// spatial index.
    pub fn pick(&self, p: P, tolerance: f64) -> Option<u32> {
        let area = Bounds {
            min: P::new(p.x - tolerance, p.y - tolerance),
            max: P::new(p.x + tolerance, p.y + tolerance),
        };
        let mut found = Vec::new();
        self.grid.query(area, &mut found);
        self.best_of(found.iter().copied(), p, tolerance)
    }

    /// The same as [`Viewer::pick`] by looking at every primitive: the answer that the index has
    /// to agree with.
    pub fn pick_linear(&self, p: P, tolerance: f64) -> Option<u32> {
        self.best_of(0..self.scene.prims.len() as u32, p, tolerance)
    }

    /// The pointer is at a pixel of the drawing area.
    pub fn hover(&mut self, sx: f64, sy: f64) {
        self.cursor = Some(self.view.to_world(sx, sy));
    }

    /// The pointer left the drawing area.
    pub fn leave(&mut self) {
        self.cursor = None;
    }

    /// A click at a pixel of the drawing area: the tool selects an item, or sets a point of the
    /// measure. The tools that edit are handled by the workspace.
    pub fn click(&mut self, sx: f64, sy: f64) {
        match self.tool {
            Tool::Select => self.select_at(sx, sy),
            Tool::Measure => {
                let p = self.view.to_world(sx, sy);
                self.measure.click(p);
            }
            _ => {}
        }
    }

    /// Selects the item under a pixel and highlights its net; nothing under it clears both.
    pub fn select_at(&mut self, sx: f64, sy: f64) {
        match self.item_at(sx, sy) {
            Some(item) => self.select_item(item),
            None => self.clear_selection(),
        }
    }

    /// The item under a pixel of the drawing area, found with the tolerance of the settings.
    pub fn item_at(&self, sx: f64, sy: f64) -> Option<u32> {
        let p = self.view.to_world(sx, sy);
        self.pick(p, self.tolerance_mm())
            .map(|prim| self.scene.prims[prim as usize].item)
    }

    /// Selects an item by its index and highlights its net.
    pub fn select_item(&mut self, item: u32) {
        if let Some(found) = self.scene.items.get(item as usize) {
            self.highlight = found.net;
            self.selected = Some(item);
        }
    }

    /// Highlights a net by its index, with no item selected. An index that does not exist clears
    /// the highlight.
    pub fn select_net(&mut self, net: u32) {
        self.selected = None;
        self.highlight = if (net as usize) < self.scene.nets.len() {
            Some(net)
        } else {
            None
        };
    }

    /// The box of all the primitives of an item; empty for an item that has none.
    pub fn item_bounds(&self, item: u32) -> Bounds {
        let mut area = Bounds::EMPTY;
        for (index, prim) in self.scene.prims.iter().enumerate() {
            if prim.item == item {
                area = area.union(self.grid.bounds_of(index as u32));
            }
        }
        area
    }

    /// Puts a point of the drawing in the middle of the window, keeping the zoom.
    pub fn centre_on(&mut self, p: P) {
        self.view.centre = p;
    }

    /// Draws another scene of the same document (after an edit): the view stays, a layer keeps
    /// its colour and whether it is shown when the new scene has a layer of the same name and
    /// role, and the selection follows the object of the document it was on, when it still exists.
    pub fn replace_scene(&mut self, mut scene: Scene) {
        let selected_doc = self.selected_item().map(|i| i.doc);
        for layer in &mut scene.layers {
            if let Some(old) = self
                .scene
                .layers
                .iter()
                .find(|o| o.name == layer.name && o.role == layer.role)
            {
                layer.visible = old.visible;
                layer.colour = old.colour;
            }
        }
        self.grid = Grid::build(&scene.prims, scene.bounds, &self.settings);
        self.scene = scene;
        self.selected = selected_doc
            .filter(|doc| *doc != DocRef::None)
            .and_then(|doc| self.scene.items.iter().position(|i| i.doc == doc))
            .map(|i| i as u32);
        self.highlight = match self.selected_item() {
            Some(item) => item.net,
            None if selected_doc.is_some() => None,
            None => self
                .highlight
                .filter(|n| (*n as usize) < self.scene.nets.len()),
        };
    }

    /// Clears the selection and the highlight.
    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.highlight = None;
    }

    /// Changes the tool; the measure starts again.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.tool = tool;
            self.measure.clear();
        }
    }

    /// The selected item.
    pub fn selected_item(&self) -> Option<&Item> {
        self.selected.and_then(|i| self.scene.items.get(i as usize))
    }

    /// The name of the highlighted net.
    pub fn highlight_name(&self) -> Option<&str> {
        self.highlight
            .and_then(|n| self.scene.nets.get(n as usize))
            .map(String::as_str)
    }

    /// How many primitives each layer holds.
    pub fn layer_counts(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.scene.layers.len()];
        for prim in &self.scene.prims {
            if let Some(c) = counts.get_mut(prim.layer as usize) {
                *c += 1;
            }
        }
        counts
    }

    /// A report of the scene as text, for the command line: what it is, how many primitives,
    /// items and nets, what could not be drawn, the primitives of each layer and the size.
    pub fn summary(&self, lang: Lang, unread: usize) -> String {
        let kind = match self.scene.kind {
            SceneKind::Board => Msg::Board,
            SceneKind::Schematic => Msg::Schematic,
        };
        let mut out = format!(
            "{}: {} {}, {} {}, {} {}",
            kind.tr(lang),
            self.scene.prims.len(),
            Msg::Primitives.tr(lang),
            self.scene.items.len(),
            Msg::Items.tr(lang),
            self.scene.nets.len(),
            Msg::Nets.tr(lang)
        );
        let not_drawn = self.scene.skipped + unread;
        if not_drawn > 0 {
            out.push_str(&format!(", {} {}", not_drawn, Msg::Skipped.tr(lang)));
        }
        out.push('\n');
        for (layer, count) in self.scene.layers.iter().zip(self.layer_counts()) {
            let name = Msg::for_role(layer.role)
                .map_or_else(|| layer.name.clone(), |m| m.tr(lang).to_string());
            out.push_str(&format!("  {name}: {count}\n"));
        }
        let b = self.scene.bounds;
        out.push_str(&format!("  {:.3} x {:.3} mm\n", b.width(), b.height()));
        out
    }

    /// The line of the status bar: the position of the pointer, the selected item with its net and
    /// the measure.
    pub fn status(&self, lang: Lang) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(p) = self.cursor {
            parts.push(format!(
                "{}: {:.3}, {:.3} mm",
                Msg::Cursor.tr(lang),
                p.x,
                p.y
            ));
        }
        if let Some(item) = self.selected_item() {
            let net = self
                .highlight_name()
                .map_or_else(|| Msg::NoNet.tr(lang).to_string(), str::to_string);
            parts.push(format!(
                "{}: {} ({}: {})",
                Msg::Selected.tr(lang),
                item.label,
                Msg::Net.tr(lang),
                net
            ));
        }
        if self.tool == Tool::Measure {
            match (self.measure.distance(), self.measure.delta()) {
                (Some(d), Some((dx, dy))) => parts.push(format!(
                    "{}: {:.3} mm (dx {:.3}, dy {:.3}, {:.2}°)",
                    Msg::Distance.tr(lang),
                    d,
                    dx,
                    dy,
                    self.measure.angle_deg().unwrap_or(0.0)
                )),
                _ => parts.push(Msg::MeasureHint.tr(lang).to_string()),
            }
        }
        parts.join("   |   ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;

    fn board_viewer() -> Viewer {
        let settings = ViewerSettings::default();
        let scene = Scene::from_board(&sample::board(), &settings).unwrap();
        Viewer::new(scene, settings)
    }

    fn click_at(v: &mut Viewer, x: f64, y: f64) {
        let (sx, sy) = v.view.to_screen(P::new(x, y));
        v.click(sx, sy);
    }

    #[test]
    fn a_new_viewer_shows_the_whole_drawing() {
        let v = board_viewer();
        let area = v.view.visible_area();
        assert!(area.contains(v.scene.bounds.min) && area.contains(v.scene.bounds.max));
        assert!(v.selected.is_none() && v.highlight.is_none());
        assert_eq!(v.tool, Tool::Select);
    }

    #[test]
    fn a_click_on_a_pad_selects_it_and_highlights_its_net() {
        let mut v = board_viewer();
        // the round pad of R1 is at (11.5, 10) and is on the net VCC
        click_at(&mut v, 11.5, 10.0);
        let item = v.selected_item().unwrap();
        assert_eq!(item.label, "R1 pad 2");
        assert_eq!(v.highlight, Some(1));
        assert_eq!(v.highlight_name(), Some("VCC"));
        // a click on the track of the other net moves the highlight
        click_at(&mut v, 7.0, 5.0);
        assert_eq!(v.selected_item().unwrap().label, "track");
        assert_eq!(v.highlight_name(), Some("GND"));
        // a click on nothing clears both
        click_at(&mut v, 28.0, 18.0);
        assert!(v.selected.is_none() && v.highlight.is_none());
    }

    #[test]
    fn the_tolerance_is_in_pixels_so_it_shrinks_when_the_zoom_grows() {
        let mut v = board_viewer();
        let before = v.tolerance_mm();
        v.zoom_at_px(100.0, 100.0, 200.0, 1.0);
        assert!(v.tolerance_mm() < before);
        // 4 mm above the track is out of reach whatever the zoom
        let p = P::new(7.0, 9.0);
        assert!(v.pick(p, v.tolerance_mm()).is_none());
        // a click a little off the thin track still hits it when the tolerance is large enough
        assert!(v.pick(P::new(7.0, 5.3), 0.3).is_some());
        assert!(v.pick(P::new(7.0, 5.3), 0.1).is_none());
    }

    #[test]
    fn a_text_is_picked_only_when_nothing_else_is_under_it() {
        let mut v = board_viewer();
        // the designator R1 is drawn at the origin of the component, between the two pads
        click_at(&mut v, 10.0, 10.2);
        assert_eq!(v.selected_item().unwrap().label, "R1 10k");
        // over the pad the pad wins although the text is drawn later
        click_at(&mut v, 11.5, 10.0);
        assert_eq!(v.selected_item().unwrap().label, "R1 pad 2");
    }

    #[test]
    fn a_layer_that_is_hidden_cannot_be_picked_or_listed() {
        let mut v = board_viewer();
        let silk = v.scene.layer_of_role(crate::scene::Role::SilkTop).unwrap() as usize;
        v.set_all_visible(false);
        click_at(&mut v, 15.0, 3.0);
        assert!(v.selected.is_none());
        assert!(v.visible(v.scene.bounds).is_empty());
        v.set_layer_visible(silk, true);
        click_at(&mut v, 15.0, 3.0);
        assert_eq!(v.selected_item().unwrap().label, "silkscreen");
        assert_eq!(v.visible(v.scene.bounds).len(), 1);
        // out of range indexes do nothing
        v.set_layer_visible(99, true);
        v.set_layer_colour(99, [1, 2, 3, 4]);
        v.set_layer_colour(silk, [1, 2, 3, 4]);
        assert_eq!(v.scene.layers[silk].colour, [1, 2, 3, 4]);
    }

    #[test]
    fn the_visible_primitives_come_in_the_order_of_drawing() {
        let v = board_viewer();
        let list = v.visible(v.scene.bounds);
        assert_eq!(list.len(), v.scene.prims.len());
        let keys: Vec<(u32, u32)> = list
            .iter()
            .map(|&i| (v.scene.prims[i as usize].layer, i))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
        // a small window sees only what is in it
        let corner = Bounds {
            min: P::new(0.0, 0.0),
            max: P::new(1.0, 1.0),
        };
        assert!(v.visible(corner).len() < list.len());
    }

    #[test]
    fn the_index_and_the_scan_agree_on_a_grid_of_points() {
        let v = board_viewer();
        for ix in 0..60 {
            for iy in 0..40 {
                let p = P::new(f64::from(ix) * 0.5, f64::from(iy) * 0.5);
                assert_eq!(v.pick(p, 0.2), v.pick_linear(p, 0.2), "{p:?}");
            }
        }
    }

    #[test]
    fn the_measure_tool_measures_between_two_clicks_in_millimetres() {
        let mut v = board_viewer();
        v.set_tool(Tool::Measure);
        click_at(&mut v, 3.0, 4.0);
        assert!(v.measure.distance().is_none());
        assert!(v.status(Lang::En).contains("Click two points"));
        click_at(&mut v, 6.0, 8.0);
        assert!((v.measure.distance().unwrap() - 5.0).abs() < 1e-6);
        assert!(v.status(Lang::En).contains("Distance: 5.000 mm"));
        // the select tool is not touched by a measure
        assert!(v.selected.is_none());
        // changing the tool starts again
        v.set_tool(Tool::Select);
        assert_eq!(v.measure, Measure::default());
    }

    #[test]
    fn the_cursor_shows_in_millimetres_and_leaves_with_the_pointer() {
        let mut v = board_viewer();
        let (sx, sy) = v.view.to_screen(P::new(12.5, 7.25));
        v.hover(sx, sy);
        assert!(v.status(Lang::Es).contains("Cursor: 12.500, 7.250 mm"));
        v.leave();
        assert!(v.cursor.is_none());
        assert!(v.status(Lang::En).is_empty());
    }

    #[test]
    fn the_status_names_the_selected_item_and_its_net_in_the_language() {
        let mut v = board_viewer();
        click_at(&mut v, 11.5, 10.0);
        assert_eq!(v.status(Lang::En), "Selected: R1 pad 2 (Net: VCC)");
        assert!(v
            .status(Lang::De)
            .starts_with("Ausgewählt: R1 pad 2 (Netz: VCC)"));
        v.clear_selection();
        click_at(&mut v, 8.0, 18.0);
        // the zone has a net: its name is shown
        assert!(v.status(Lang::En).contains("(Net: GND)"));
        // the silkscreen has none
        click_at(&mut v, 15.0, 3.0);
        assert!(v.status(Lang::En).contains("no net"));
    }

    #[test]
    fn the_wheel_zooms_about_the_pointer_and_a_drag_pans() {
        let mut v = board_viewer();
        let (sx, sy) = (300.0, 200.0);
        let before = v.view.to_world(sx, sy);
        v.zoom_at_px(sx, sy, 120.0, 1.0);
        let after = v.view.to_world(sx, sy);
        assert!((before.x - after.x).abs() < 1e-9 && (before.y - after.y).abs() < 1e-9);
        let scale = v.view.scale;
        v.zoom_at_px(sx, sy, 0.0, 2.0);
        assert!((v.view.scale - 2.0 * scale).abs() < 1e-9 || v.view.scale >= 4000.0);
        let p = P::new(5.0, 5.0);
        let (x0, _) = v.view.to_screen(p);
        v.pan_px(10.0, 0.0);
        let (x1, _) = v.view.to_screen(p);
        assert!((x1 - x0 - 10.0).abs() < 1e-9);
        v.fit();
        let area = v.view.visible_area();
        assert!(area.contains(v.scene.bounds.min));
    }

    #[test]
    fn a_schematic_net_is_highlighted_from_a_pin_and_from_a_wire() {
        let settings = ViewerSettings::default();
        let scene = Scene::from_schematic(&sample::schematic(), &settings).unwrap();
        let mut v = Viewer::new(scene, settings);
        // the pin 2 of R1 is at (25, 40) on the first sheet and on the net N1
        click_at(&mut v, 25.0, 40.0);
        assert_eq!(v.highlight_name(), Some("N1"));
        v.clear_selection();
        // the middle of the wire from (40, 40) to (55, 40) reaches the same net
        click_at(&mut v, 47.0, 40.0);
        assert_eq!(v.selected_item().unwrap().label, "wire");
        assert_eq!(v.highlight_name(), Some("N1"));
        // the wire that touches nothing has no net
        click_at(&mut v, 15.0, 10.0);
        assert_eq!(v.selected_item().unwrap().label, "wire");
        assert!(v.highlight.is_none());
    }

    #[test]
    fn the_summary_lists_the_layers_with_their_counts_in_the_language() {
        let v = board_viewer();
        let text = v.summary(Lang::En, 0);
        assert!(
            text.starts_with("Board: 15 primitives, 9 Items, 2 Nets"),
            "{text}"
        );
        assert!(text.contains("  Top: 5\n"));
        assert!(text.contains("  Drills: 2\n"));
        assert!(text.contains("30.100 x 20.100 mm"), "{text}");
        assert!(!text.contains("not drawn"));
        let spanish = v.summary(Lang::Es, 3);
        assert!(spanish.starts_with("Placa: 15 primitivas"));
        assert!(spanish.contains("3 sin dibujar"));
        assert!(spanish.contains("  Taladros: 2\n"));
    }

    #[test]
    fn the_tools_that_edit_are_told_from_select_and_measure() {
        assert!(!Tool::Select.edits() && !Tool::Measure.edits());
        for tool in [
            Tool::MoveComponent,
            Tool::RotateComponent,
            Tool::DeleteTrack,
            Tool::MoveVertex,
            Tool::SetWidth,
            Tool::AddVia,
            Tool::Route,
        ] {
            assert!(tool.edits(), "{tool:?}");
        }
        // a click with a tool that edits does nothing in the viewer itself
        let mut v = board_viewer();
        v.set_tool(Tool::DeleteTrack);
        click_at(&mut v, 7.0, 5.0);
        assert!(v.selected.is_none());
        assert!(v.measure.distance().is_none());
    }

    #[test]
    fn an_item_is_found_under_a_pixel_and_a_net_is_selected_by_its_index() {
        let mut v = board_viewer();
        let (sx, sy) = v.view.to_screen(P::new(7.0, 5.0));
        let item = v.item_at(sx, sy).unwrap();
        assert_eq!(v.scene.items[item as usize].label, "track");
        let (sx, sy) = v.view.to_screen(P::new(28.0, 18.0));
        assert!(v.item_at(sx, sy).is_none());
        v.select_item(item);
        assert_eq!(v.highlight_name(), Some("GND"));
        v.select_net(1);
        assert!(v.selected.is_none());
        assert_eq!(v.highlight_name(), Some("VCC"));
        v.select_net(99);
        assert!(v.highlight.is_none());
        // an item that does not exist changes nothing
        v.select_item(999);
        assert!(v.selected.is_none());
    }

    #[test]
    fn the_box_of_an_item_holds_all_of_its_primitives_and_the_view_can_be_centred() {
        let mut v = board_viewer();
        let (sx, sy) = v.view.to_screen(P::new(11.5, 10.0));
        let pad = v.item_at(sx, sy).unwrap();
        assert_eq!(v.scene.items[pad as usize].label, "R1 pad 2");
        let area = v.item_bounds(pad);
        // the round pad has a radius of 0.8 mm and a hole in the drill layer inside it
        assert!(area.contains(P::new(11.5, 10.0)));
        assert!(area.width() >= 1.6 - 1e-9 && area.width() < 3.0);
        assert!(v.item_bounds(999).is_empty());
        v.centre_on(P::new(3.0, 4.0));
        assert_eq!(v.view.centre, P::new(3.0, 4.0));
    }

    #[test]
    fn a_new_scene_keeps_the_view_the_layers_and_the_selection() {
        let mut v = board_viewer();
        let silk = v.scene.layer_of_role(crate::scene::Role::SilkTop).unwrap() as usize;
        v.set_layer_visible(silk, false);
        v.set_layer_colour(silk, [9, 8, 7, 255]);
        click_at(&mut v, 7.0, 5.0);
        let before = v.view;
        let doc = v.selected_item().unwrap().doc;
        assert!(matches!(doc, DocRef::Track(_)));
        // a board without the other track gives other item numbers
        let mut board = sample::board();
        board.tracks.remove(&sample::id(41));
        let scene = Scene::from_board(&board, &v.settings).unwrap();
        v.replace_scene(scene);
        assert_eq!(v.view, before);
        assert!(!v.scene.layers[silk].visible);
        assert_eq!(v.scene.layers[silk].colour, [9, 8, 7, 255]);
        assert_eq!(v.selected_item().unwrap().doc, doc);
        assert_eq!(v.highlight_name(), Some("GND"));
        // when the selected object is gone the selection is gone, and the highlight with it
        board.tracks.remove(&sample::id(40));
        let scene = Scene::from_board(&board, &v.settings).unwrap();
        v.replace_scene(scene);
        assert!(v.selected.is_none());
        assert!(v.highlight.is_none());
    }

    #[test]
    fn the_layer_counts_add_up_to_the_primitives() {
        let v = board_viewer();
        assert_eq!(v.layer_counts().iter().sum::<usize>(), v.scene.prims.len());
        assert!(v.grid().cells() >= 1);
    }
}
