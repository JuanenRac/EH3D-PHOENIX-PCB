// SPDX-License-Identifier: GPL-3.0-or-later
//! The search of the window, without a screen: components by reference, value or footprint, nets by
//! name and pads or pins by `U1.5`, over the scene that is open; the list of results that the keys
//! walk through; the selection and the zoom that a result asks for; and the same search over the
//! components of the 3D window. Nothing here knows egui.

use eh3d_phoenix_board::Board;

use crate::prim::{Bounds, P};
use crate::scene::{ItemKind, Scene};
use crate::viewer::Viewer;

/// What a result is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HitKind {
    /// A component of a board or a part of a schematic.
    Component,
    /// A net.
    Net,
    /// A pad of a board or a pin of a schematic.
    Pad,
}

/// A result of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// What it is.
    pub kind: HitKind,
    /// The item of the scene that it selects, when it is one.
    pub item: Option<u32>,
    /// The net that it lights.
    pub net: Option<u32>,
    /// What the list shows: `R1 100n`, `GND` or `U1.5`.
    pub label: String,
    /// How well it matches; the best come first.
    pub score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentEntry {
    item: u32,
    refdes: String,
    value: String,
    footprint: String,
    net: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PadEntry {
    item: u32,
    refdes: String,
    number: String,
    net: Option<u32>,
}

/// The names of a scene prepared for searching.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchIndex {
    components: Vec<ComponentEntry>,
    pads: Vec<PadEntry>,
    nets: Vec<(u32, String)>,
}

/// How well a name matches a text, both in lower case: the whole name 100, the start of it 80, any
/// part of it 50, nothing 0; `weight` scales the answer (out of 100).
fn match_score(name: &str, wanted: &str, weight: u32) -> u32 {
    let base = if name == wanted {
        100
    } else if name.starts_with(wanted) {
        80
    } else if name.contains(wanted) {
        50
    } else {
        0
    };
    base * weight / 100
}

impl SearchIndex {
    /// Prepares a scene. `footprints` pairs the reference of a component with the name of its
    /// footprint (the scene does not know it); a reference that is not in the list has none.
    pub fn build(scene: &Scene, footprints: &[(String, String)]) -> SearchIndex {
        let mut index = SearchIndex::default();
        for (at, item) in scene.items.iter().enumerate() {
            let at = at as u32;
            match item.kind {
                ItemKind::Designator | ItemKind::Part => {
                    let (refdes, value) = item
                        .label
                        .split_once(' ')
                        .unwrap_or((item.label.as_str(), ""));
                    let footprint = footprints
                        .iter()
                        .find(|(r, _)| r == refdes)
                        .map(|(_, f)| f.to_lowercase())
                        .unwrap_or_default();
                    index.components.push(ComponentEntry {
                        item: at,
                        refdes: refdes.to_string(),
                        value: value.trim().to_string(),
                        footprint,
                        net: item.net,
                    });
                }
                ItemKind::Pad => {
                    if let Some((refdes, number)) = item.label.split_once(" pad ") {
                        index.pads.push(PadEntry {
                            item: at,
                            refdes: refdes.to_string(),
                            number: number.to_string(),
                            net: item.net,
                        });
                    }
                }
                ItemKind::Pin => {
                    if let Some((refdes, number)) = item.label.rsplit_once('.') {
                        index.pads.push(PadEntry {
                            item: at,
                            refdes: refdes.to_string(),
                            number: number.to_string(),
                            net: item.net,
                        });
                    }
                }
                _ => {}
            }
        }
        index.nets = scene
            .nets
            .iter()
            .enumerate()
            .map(|(n, name)| (n as u32, name.clone()))
            .collect();
        index
    }

    /// How many names the index holds.
    pub fn len(&self) -> usize {
        self.components.len() + self.pads.len() + self.nets.len()
    }

    /// True when it holds none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The results for a text, the best first and at most `max`. A text such as `U1.5` asks for a
    /// pad or a pin; if there is none the text is looked for as a name. The case does not matter.
    pub fn search(&self, query: &str, max: usize) -> Vec<Hit> {
        let wanted = query.trim().to_lowercase();
        if wanted.is_empty() || max == 0 {
            return Vec::new();
        }
        if let Some((left, right)) = wanted.split_once('.') {
            if !left.is_empty() && !right.is_empty() {
                let pads = self.pad_hits(left, right);
                if !pads.is_empty() {
                    return pads.into_iter().take(max).collect();
                }
            }
        }
        let mut hits = Vec::new();
        for c in &self.components {
            let score = match_score(&c.refdes.to_lowercase(), &wanted, 100)
                .max(match_score(&c.value.to_lowercase(), &wanted, 70))
                .max(match_score(&c.footprint, &wanted, 45));
            if score > 0 {
                hits.push(Hit {
                    kind: HitKind::Component,
                    item: Some(c.item),
                    net: c.net,
                    label: format!("{} {}", c.refdes, c.value).trim().to_string(),
                    score,
                });
            }
        }
        for (net, name) in &self.nets {
            let score = match_score(&name.to_lowercase(), &wanted, 90);
            if score > 0 {
                hits.push(Hit {
                    kind: HitKind::Net,
                    item: None,
                    net: Some(*net),
                    label: name.clone(),
                    score,
                });
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.kind.cmp(&b.kind))
                .then_with(|| a.label.cmp(&b.label))
        });
        hits.truncate(max);
        hits
    }

    fn pad_hits(&self, refdes: &str, number: &str) -> Vec<Hit> {
        let mut hits: Vec<Hit> = self
            .pads
            .iter()
            .filter(|p| p.refdes.to_lowercase() == refdes && p.number.to_lowercase() == number)
            .map(|p| Hit {
                kind: HitKind::Pad,
                item: Some(p.item),
                net: p.net,
                label: format!("{}.{}", p.refdes, p.number),
                score: 100,
            })
            .collect();
        hits.sort_by(|a, b| a.label.cmp(&b.label));
        hits
    }
}

/// The reference and the footprint of every component of a board, for [`SearchIndex::build`].
pub fn footprint_pairs(board: &Board) -> Vec<(String, String)> {
    board
        .components
        .values()
        .map(|c| {
            let name = board
                .footprints
                .get(&c.footprint)
                .map(|f| f.name.clone())
                .unwrap_or_default();
            (c.refdes.clone(), name)
        })
        .collect()
}

/// The results of a search and the one that the keys are on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchState {
    /// The text typed in the box.
    pub query: String,
    hits: Vec<Hit>,
    cursor: usize,
}

impl SearchState {
    /// Looks for the text again and goes to the first result.
    pub fn run(&mut self, index: &SearchIndex, max: usize) {
        self.hits = index.search(&self.query, max);
        self.cursor = 0;
    }

    /// Puts results found by another means (the components of the 3D window) in the list, and goes
    /// to the first.
    pub fn set_hits(&mut self, hits: Vec<Hit>) {
        self.hits = hits;
        self.cursor = 0;
    }

    /// Forgets the text and the results.
    pub fn clear(&mut self) {
        self.query.clear();
        self.hits.clear();
        self.cursor = 0;
    }

    /// The results.
    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    /// The place of the result the keys are on.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The result the keys are on.
    pub fn current(&self) -> Option<&Hit> {
        self.hits.get(self.cursor)
    }

    /// Goes to a result; a place that does not exist changes nothing.
    pub fn set_cursor(&mut self, at: usize) -> bool {
        if at < self.hits.len() {
            self.cursor = at;
            true
        } else {
            false
        }
    }

    /// Goes to the next result, and from the last to the first.
    pub fn next(&mut self) -> Option<&Hit> {
        if self.hits.is_empty() {
            return None;
        }
        self.cursor = (self.cursor + 1) % self.hits.len();
        self.current()
    }

    /// Goes to the previous result, and from the first to the last.
    pub fn previous(&mut self) -> Option<&Hit> {
        if self.hits.is_empty() {
            return None;
        }
        self.cursor = (self.cursor + self.hits.len() - 1) % self.hits.len();
        self.current()
    }
}

/// Shows a box in the window: the view is centred on it and zoomed to hold it, with at least
/// `context_mm` of the drawing around it so that a single pad is not blown up.
pub fn frame(viewer: &mut Viewer, area: Bounds, context_mm: f64) {
    if area.is_empty() {
        return;
    }
    let centre = area.centre();
    let half = area.width().max(area.height()).max(context_mm.max(1e-6)) / 2.0;
    let wanted = Bounds {
        min: P::new(centre.x - half, centre.y - half),
        max: P::new(centre.x + half, centre.y + half),
    };
    let limits = viewer.limits();
    let margin = viewer.settings.fit_margin_fraction;
    viewer.view.fit(wanted, margin, limits);
}

/// Does what a result asks for: the item is selected or the net is lit, and the view goes to it.
/// The answer is false when the result no longer exists in the scene.
pub fn go_to(viewer: &mut Viewer, hit: &Hit, context_mm: f64) -> bool {
    let area = match (hit.item, hit.net) {
        (Some(item), _) => {
            if item as usize >= viewer.scene.items.len() {
                return false;
            }
            viewer.select_item(item);
            viewer.item_bounds(item)
        }
        (None, Some(net)) => {
            if net as usize >= viewer.scene.nets.len() {
                return false;
            }
            viewer.select_net(net);
            let mut area = Bounds::EMPTY;
            for prim in &viewer.scene.prims {
                if prim.net == Some(net) {
                    area = area.union(prim.bounds());
                }
            }
            area
        }
        (None, None) => return false,
    };
    frame(viewer, area, context_mm);
    true
}

/// The places, from zero, of the components of the 3D window that a text names: the references and
/// the footprints are looked at, the whole reference first. `parts` pairs each reference with its
/// footprint, in the order of the scene.
pub fn search_parts(parts: &[(String, String)], query: &str) -> Vec<usize> {
    let wanted = query.trim().to_lowercase();
    if wanted.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(u32, usize)> = parts
        .iter()
        .enumerate()
        .filter_map(|(at, (reference, footprint))| {
            let score = match_score(&reference.to_lowercase(), &wanted, 100).max(match_score(
                &footprint.to_lowercase(),
                &wanted,
                45,
            ));
            (score > 0).then_some((score, at))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    found.into_iter().map(|(_, at)| at).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim::Shape;
    use crate::sample;
    use crate::settings::ViewerSettings;

    fn dot(x: f64, y: f64) -> Shape {
        Shape::Circle {
            centre: P::new(x, y),
            radius: 0.3,
        }
    }

    fn scene() -> Scene {
        sample::scene(
            &["GND", "VCC", "3.3V"],
            &[
                (ItemKind::Designator, "R1 10k", None, dot(0.0, 0.0)),
                (ItemKind::Designator, "R12 4k7", None, dot(5.0, 0.0)),
                (ItemKind::Designator, "U1 LM358", None, dot(10.0, 0.0)),
                (ItemKind::Designator, "C1 100n", None, dot(15.0, 0.0)),
                (ItemKind::Pad, "U1 pad 5", Some(1), dot(10.0, 1.0)),
                (ItemKind::Pad, "U1 pad 4", Some(0), dot(10.0, -1.0)),
                (ItemKind::Pad, "R1 pad 1", Some(0), dot(0.0, 1.0)),
                (ItemKind::Track, "track", Some(0), dot(20.0, 0.0)),
            ],
        )
    }

    fn footprints() -> Vec<(String, String)> {
        vec![
            ("R1".to_string(), "R0603".to_string()),
            ("R12".to_string(), "R0805".to_string()),
            ("U1".to_string(), "SOIC8".to_string()),
            ("C1".to_string(), "C0603".to_string()),
        ]
    }

    fn index() -> SearchIndex {
        SearchIndex::build(&scene(), &footprints())
    }

    fn labels(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|h| h.label.as_str()).collect()
    }

    #[test]
    fn the_index_holds_the_components_the_pads_and_the_nets() {
        let i = index();
        assert_eq!(i.len(), 4 + 3 + 3);
        assert!(!i.is_empty());
        assert!(SearchIndex::default().is_empty());
        assert!(SearchIndex::build(&sample::scene(&[], &[]), &[]).is_empty());
    }

    #[test]
    fn a_reference_finds_its_component_and_the_whole_one_comes_first() {
        let hits = index().search("r1", 10);
        assert_eq!(labels(&hits), ["R1 10k", "R12 4k7"]);
        assert_eq!(hits[0].kind, HitKind::Component);
        assert_eq!(hits[0].item, Some(0));
        assert!(hits[0].score > hits[1].score);
        // case does not matter and blanks around are ignored
        assert_eq!(index().search("  U1 ", 10).len(), 1);
    }

    #[test]
    fn a_value_or_a_footprint_finds_the_component() {
        let by_value = index().search("lm358", 10);
        assert_eq!(labels(&by_value), ["U1 LM358"]);
        let by_footprint = index().search("soic", 10);
        assert_eq!(labels(&by_footprint), ["U1 LM358"]);
        let by_family = index().search("0603", 10);
        assert_eq!(labels(&by_family), ["C1 100n", "R1 10k"]);
        // a scene without the footprints still finds by reference and value
        let bare = SearchIndex::build(&scene(), &[]);
        assert!(bare.search("soic", 10).is_empty());
        assert_eq!(bare.search("lm358", 10).len(), 1);
    }

    #[test]
    fn a_net_is_found_by_name_and_it_lights_the_net() {
        let hits = index().search("gnd", 10);
        assert_eq!(labels(&hits), ["GND"]);
        assert_eq!(hits[0].kind, HitKind::Net);
        assert_eq!(hits[0].net, Some(0));
        assert_eq!(hits[0].item, None);
        // a name with a point is a net when no pad has it
        let point = index().search("3.3v", 10);
        assert_eq!(labels(&point), ["3.3V"]);
        assert_eq!(point[0].kind, HitKind::Net);
    }

    #[test]
    fn a_reference_dot_number_finds_the_pad_or_the_pin() {
        let hits = index().search("u1.5", 10);
        assert_eq!(labels(&hits), ["U1.5"]);
        assert_eq!(hits[0].kind, HitKind::Pad);
        assert_eq!(hits[0].item, Some(4));
        assert_eq!(hits[0].net, Some(1));
        assert!(index().search("U1.9", 10).is_empty());
        assert!(index().search("U1.", 10).is_empty());
    }

    #[test]
    fn the_pins_of_a_schematic_are_found_the_same_way() {
        let scene = sample::scene(
            &["N1"],
            &[
                (ItemKind::Part, "R1 10k", None, dot(0.0, 0.0)),
                (ItemKind::Pin, "R1.2", Some(0), dot(1.0, 0.0)),
                (ItemKind::Pin, "R1.1", None, dot(-1.0, 0.0)),
            ],
        );
        let index = SearchIndex::build(&scene, &[]);
        let hits = index.search("r1.2", 5);
        assert_eq!(labels(&hits), ["R1.2"]);
        assert_eq!(hits[0].kind, HitKind::Pad);
        let part = index.search("r1", 5);
        assert_eq!(labels(&part), ["R1 10k"]);
    }

    #[test]
    fn nothing_matches_an_empty_text_or_a_limit_of_zero_and_the_limit_cuts() {
        let i = index();
        assert!(i.search("", 10).is_empty());
        assert!(i.search("   ", 10).is_empty());
        assert!(i.search("r", 0).is_empty());
        assert!(i.search("zzz", 10).is_empty());
        assert_eq!(i.search("c", 2).len(), 2);
    }

    #[test]
    fn the_keys_walk_through_the_results_and_wrap_around() {
        let mut state = SearchState {
            query: "r1".to_string(),
            ..SearchState::default()
        };
        assert!(state.next().is_none() && state.previous().is_none());
        state.run(&index(), 10);
        assert_eq!(state.hits().len(), 2);
        assert_eq!(state.cursor(), 0);
        assert_eq!(state.current().unwrap().label, "R1 10k");
        assert_eq!(state.next().unwrap().label, "R12 4k7");
        assert_eq!(state.next().unwrap().label, "R1 10k");
        assert_eq!(state.previous().unwrap().label, "R12 4k7");
        assert!(state.set_cursor(0));
        assert!(!state.set_cursor(2));
        assert_eq!(state.cursor(), 0);
        state.clear();
        assert!(state.hits().is_empty() && state.query.is_empty() && state.current().is_none());
    }

    #[test]
    fn results_found_by_another_means_are_walked_the_same_way() {
        let mut state = SearchState::default();
        let hit = |n: u32| Hit {
            kind: HitKind::Component,
            item: Some(n),
            net: None,
            label: format!("P{n}"),
            score: 0,
        };
        state.set_hits(vec![hit(4), hit(7)]);
        assert_eq!(state.current().unwrap().item, Some(4));
        assert_eq!(state.next().unwrap().item, Some(7));
        state.set_hits(Vec::new());
        assert!(state.current().is_none() && state.cursor() == 0);
    }

    #[test]
    fn a_component_result_selects_it_and_the_view_goes_to_it() {
        let mut v = Viewer::new(scene(), ViewerSettings::default());
        let hit = index().search("u1", 1).remove(0);
        assert!(go_to(&mut v, &hit, 10.0));
        assert_eq!(v.selected, Some(2));
        assert!((v.view.centre.x - 10.0).abs() < 1e-9 && v.view.centre.y.abs() < 1e-9);
        let area = v.view.visible_area();
        assert!(area.contains(P::new(10.0, 0.0)));
        // at least ten millimetres of drawing are in view: the pad was not blown up
        assert!(area.width() >= 10.0 && area.height() >= 10.0);
    }

    #[test]
    fn a_net_result_lights_the_net_and_holds_all_of_its_items() {
        let mut v = Viewer::new(scene(), ViewerSettings::default());
        let hit = index().search("gnd", 1).remove(0);
        assert!(go_to(&mut v, &hit, 1.0));
        assert_eq!(v.highlight, Some(0));
        assert!(v.selected.is_none());
        let area = v.view.visible_area();
        for p in [P::new(10.0, -1.0), P::new(0.0, 1.0), P::new(20.0, 0.0)] {
            assert!(area.contains(p), "{p:?}");
        }
    }

    #[test]
    fn a_result_that_is_no_longer_in_the_scene_does_nothing() {
        let mut v = Viewer::new(scene(), ViewerSettings::default());
        let old_item = Hit {
            kind: HitKind::Component,
            item: Some(99),
            net: None,
            label: "X".to_string(),
            score: 1,
        };
        assert!(!go_to(&mut v, &old_item, 5.0));
        let old_net = Hit {
            item: None,
            net: Some(99),
            ..old_item.clone()
        };
        assert!(!go_to(&mut v, &old_net, 5.0));
        let none = Hit {
            item: None,
            net: None,
            ..old_item
        };
        assert!(!go_to(&mut v, &none, 5.0));
        assert!(v.selected.is_none() && v.highlight.is_none());
    }

    #[test]
    fn the_components_of_the_3d_window_are_found_by_reference_or_footprint() {
        let parts = vec![
            ("R1".to_string(), "R0603".to_string()),
            ("R10".to_string(), "R0805".to_string()),
            ("U1".to_string(), "SOIC8".to_string()),
        ];
        assert_eq!(search_parts(&parts, "r1"), [0, 1]);
        assert_eq!(search_parts(&parts, "R10"), [1]);
        assert_eq!(search_parts(&parts, "soic"), [2]);
        assert_eq!(search_parts(&parts, "0805"), [1]);
        assert!(search_parts(&parts, "zzz").is_empty());
        assert!(search_parts(&parts, " ").is_empty());
    }

    #[test]
    fn the_footprints_of_a_board_are_paired_with_the_references() {
        let board = sample::board();
        let pairs = footprint_pairs(&board);
        assert_eq!(pairs.len(), board.components.len());
        assert!(pairs.iter().any(|(r, f)| r == "R1" && !f.is_empty()));
    }
}
