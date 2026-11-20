// SPDX-License-Identifier: GPL-3.0-or-later
//! The lists of the objects of the document that the panels of the components show, without a screen: the
//! components of a board or the parts of a schematic in the order a person reads them (`R2` before
//! `R10`), filtered by the words that were typed, and the counts of the kinds of object. The panel only
//! draws the rows and passes on the click; what a click selects is `Viewer::select_item`.

use crate::dock_text::DockMsg;
use crate::prim::P;
use crate::scene::{ItemKind, Scene};
use crate::viewer::Viewer;
use eh3d_phoenix_rules::Lang;

/// The kinds of object that are counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CountKind {
    /// A component of a board or a part of a schematic.
    Component,
    /// A pad of a board or a pin of a schematic.
    Pad,
    /// A track of a board.
    Track,
    /// A via of a board.
    Via,
    /// A copper zone of a board.
    Zone,
    /// A text or a line of the silkscreen of a board.
    Silk,
    /// A wire of a schematic.
    Wire,
}

impl CountKind {
    /// Every kind, in the order of the lists.
    pub const ALL: [CountKind; 7] = [
        CountKind::Component,
        CountKind::Pad,
        CountKind::Track,
        CountKind::Via,
        CountKind::Zone,
        CountKind::Silk,
        CountKind::Wire,
    ];

    /// The kind an item of the scene is counted as; the outline, the sheets and the labels of nets are not counted.
    pub fn of(kind: ItemKind) -> Option<CountKind> {
        match kind {
            ItemKind::Designator | ItemKind::Part => Some(CountKind::Component),
            ItemKind::Pad | ItemKind::Pin => Some(CountKind::Pad),
            ItemKind::Track => Some(CountKind::Track),
            ItemKind::Via => Some(CountKind::Via),
            ItemKind::Zone => Some(CountKind::Zone),
            ItemKind::Silk => Some(CountKind::Silk),
            ItemKind::Wire => Some(CountKind::Wire),
            ItemKind::Outline | ItemKind::NetLabel | ItemKind::Sheet => None,
        }
    }

    /// The word for the kind, in a language.
    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            CountKind::Component => DockMsg::KindComponent,
            CountKind::Pad => DockMsg::KindPad,
            CountKind::Track => DockMsg::KindTrack,
            CountKind::Via => DockMsg::KindVia,
            CountKind::Zone => DockMsg::KindZone,
            CountKind::Silk => DockMsg::KindSilk,
            CountKind::Wire => DockMsg::KindWire,
        }
        .tr(lang)
    }
}

/// How many objects of each kind the scene has; the kinds that have none are left out.
pub fn counts(scene: &Scene) -> Vec<(CountKind, usize)> {
    let mut n = [0usize; 7];
    for item in &scene.items {
        if let Some(kind) = CountKind::of(item.kind) {
            n[kind as usize] += 1;
        }
    }
    CountKind::ALL
        .into_iter()
        .zip(n)
        .filter(|(_, count)| *count > 0)
        .collect()
}

/// A row of the list of components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRow {
    /// The item of the scene that a click selects.
    pub item: u32,
    /// What the row shows: `R1 10k`.
    pub label: String,
}

/// The components that match a filter.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComponentList {
    /// The rows to draw, in the order of reading, at most the number asked for.
    pub rows: Vec<ObjectRow>,
    /// How many components the document has.
    pub total: usize,
    /// How many match the filter (the rows may be fewer).
    pub matching: usize,
}

/// The key that orders `R2` before `R10` and `C1` before `R1`: the letters of the reference, the number in it
/// and what follows.
fn natural_key(label: &str) -> (String, u64, String) {
    let reference = label.split_whitespace().next().unwrap_or("");
    let letters: String = reference
        .chars()
        .take_while(|c| !c.is_ascii_digit())
        .collect();
    let rest = &reference[letters.len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let number = digits.parse::<u64>().unwrap_or(u64::MAX);
    let tail = rest[digits.len()..].to_string();
    (letters.to_lowercase(), number, format!("{tail} {label}"))
}

/// The components of the scene whose label holds every word of `filter` (in any case; no word shows
/// them all), in the order of reading, at most `max_rows` rows.
pub fn components(scene: &Scene, filter: &str, max_rows: usize) -> ComponentList {
    let words: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
    let mut all: Vec<ObjectRow> = scene
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| CountKind::of(item.kind) == Some(CountKind::Component))
        .map(|(i, item)| ObjectRow {
            item: i as u32,
            label: item.label.clone(),
        })
        .collect();
    let total = all.len();
    all.retain(|row| {
        let label = row.label.to_lowercase();
        words.iter().all(|w| label.contains(w))
    });
    all.sort_by(|a, b| {
        natural_key(&a.label)
            .cmp(&natural_key(&b.label))
            .then(a.item.cmp(&b.item))
    });
    let matching = all.len();
    all.truncate(max_rows);
    ComponentList {
        rows: all,
        total,
        matching,
    }
}

/// Selects an item of the scene and puts the middle of its box in the middle of the window, keeping the zoom.
/// The answer is false when the item does not exist (nothing changes) and true otherwise; an item
/// with nothing drawn is selected and the view stays where it is.
pub fn focus(viewer: &mut Viewer, item: u32) -> bool {
    if (item as usize) >= viewer.scene.items.len() {
        return false;
    }
    viewer.select_item(item);
    let area = viewer.item_bounds(item);
    if !area.is_empty() {
        viewer.centre_on(P::new(
            (area.min.x + area.max.x) / 2.0,
            (area.min.y + area.max.y) / 2.0,
        ));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim::{Shape, P};
    use crate::sample;
    use crate::settings::ViewerSettings;

    fn dot() -> Shape {
        Shape::Circle {
            centre: P::new(0.0, 0.0),
            radius: 0.3,
        }
    }

    fn scene_of(labels: &[(ItemKind, &str)]) -> Scene {
        let items: Vec<(ItemKind, &str, Option<u32>, Shape)> =
            labels.iter().map(|(k, l)| (*k, *l, None, dot())).collect();
        sample::scene(&["GND"], &items)
    }

    /// A generator of numbers with a fixed seed, so the test tries the same cases every time.
    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed >> 33
    }

    #[test]
    fn the_components_come_in_the_order_a_person_reads_them() {
        let scene = scene_of(&[
            (ItemKind::Designator, "R10 1k"),
            (ItemKind::Designator, "R2 2k"),
            (ItemKind::Designator, "U1 LM358"),
            (ItemKind::Designator, "C3 100n"),
            (ItemKind::Designator, "R1 10k"),
            (ItemKind::Pad, "R1 pad 1"),
        ]);
        let list = components(&scene, "", 100);
        let labels: Vec<&str> = list.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["C3 100n", "R1 10k", "R2 2k", "R10 1k", "U1 LM358"]);
        assert_eq!(list.total, 5);
        assert_eq!(list.matching, 5);
    }

    #[test]
    fn a_row_selects_the_item_it_stands_for() {
        let scene = scene_of(&[(ItemKind::Track, "track"), (ItemKind::Designator, "R1 10k")]);
        let list = components(&scene, "r1", 10);
        assert_eq!(list.rows.len(), 1);
        assert_eq!(list.rows[0].item, 1);
        assert_eq!(scene.items[list.rows[0].item as usize].label, "R1 10k");
    }

    #[test]
    fn the_filter_needs_every_word_in_any_case() {
        let scene = scene_of(&[
            (ItemKind::Designator, "R1 10k"),
            (ItemKind::Designator, "R12 4k7"),
            (ItemKind::Designator, "U1 LM358"),
        ]);
        assert_eq!(components(&scene, "R1", 10).matching, 2);
        assert_eq!(components(&scene, "r1 10K", 10).matching, 1);
        assert_eq!(components(&scene, "lm", 10).matching, 1);
        assert_eq!(components(&scene, "zzz", 10).matching, 0);
        assert_eq!(components(&scene, "  ", 10).matching, 3);
        // the total does not depend on the filter
        assert_eq!(components(&scene, "zzz", 10).total, 3);
    }

    #[test]
    fn the_rows_are_cut_at_the_most_asked_for_and_the_count_says_how_many_matched() {
        let scene = scene_of(&[
            (ItemKind::Designator, "R1"),
            (ItemKind::Designator, "R2"),
            (ItemKind::Designator, "R3"),
        ]);
        let list = components(&scene, "", 2);
        assert_eq!(list.rows.len(), 2);
        assert_eq!(list.matching, 3);
        assert_eq!(components(&scene, "", 0).rows.len(), 0);
    }

    #[test]
    fn the_parts_of_a_schematic_are_components_too() {
        let scene = scene_of(&[(ItemKind::Part, "R1 10k"), (ItemKind::Wire, "wire")]);
        assert_eq!(components(&scene, "", 10).total, 1);
        assert_eq!(
            counts(&scene),
            vec![(CountKind::Component, 1), (CountKind::Wire, 1)]
        );
    }

    #[test]
    fn the_counts_of_a_board_leave_out_what_it_has_none_of() {
        let scene = scene_of(&[
            (ItemKind::Designator, "R1"),
            (ItemKind::Pad, "R1 pad 1"),
            (ItemKind::Pad, "R1 pad 2"),
            (ItemKind::Track, "t"),
            (ItemKind::Via, "v"),
            (ItemKind::Outline, "outline"),
            (ItemKind::Sheet, "sheet"),
        ]);
        assert_eq!(
            counts(&scene),
            vec![
                (CountKind::Component, 1),
                (CountKind::Pad, 2),
                (CountKind::Track, 1),
                (CountKind::Via, 1),
            ]
        );
        assert!(counts(&scene_of(&[])).is_empty());
    }

    #[test]
    fn every_kind_has_a_word_in_the_eight_languages() {
        for kind in CountKind::ALL {
            for lang in Lang::ALL {
                assert!(!kind.name(lang).is_empty(), "{kind:?} {lang:?}");
            }
        }
    }

    #[test]
    fn a_label_with_no_number_or_a_huge_one_still_has_a_place() {
        let scene = scene_of(&[
            (ItemKind::Designator, "99999999999999999999999 x"),
            (ItemKind::Designator, "J"),
            (ItemKind::Designator, ""),
            (ItemKind::Designator, "J2"),
        ]);
        let list = components(&scene, "", 10);
        assert_eq!(list.rows.len(), 4);
        // the same list again gives the same order
        assert_eq!(list, components(&scene, "", 10));
    }

    #[test]
    fn a_shuffled_list_comes_out_in_one_order_and_every_row_is_kept() {
        // labels from a generator with a fixed seed, in several orders
        let mut seed = 0x0b_1e_c7_u64;
        let mut labels: Vec<String> = (0..60)
            .map(|_| {
                let prefix = ["R", "C", "U", "D", "Q"][(lcg(&mut seed) % 5) as usize];
                format!("{prefix}{} v{}", lcg(&mut seed) % 40, lcg(&mut seed) % 9)
            })
            .collect();
        let build = |labels: &[String]| {
            let items: Vec<(ItemKind, &str)> = labels
                .iter()
                .map(|l| (ItemKind::Designator, l.as_str()))
                .collect();
            components(&scene_of(&items), "", 1000)
        };
        let first = build(&labels);
        assert_eq!(first.rows.len(), 60);
        let sorted: Vec<String> = first.rows.iter().map(|r| r.label.clone()).collect();
        for _ in 0..5 {
            for i in (1..labels.len()).rev() {
                let j = (lcg(&mut seed) as usize) % (i + 1);
                labels.swap(i, j);
            }
            let again: Vec<String> = build(&labels)
                .rows
                .iter()
                .map(|r| r.label.clone())
                .collect();
            // the labels in the same order whatever the order of the scene (equal labels are the same text)
            assert_eq!(again, sorted);
        }
        // letters first, then the number: every C comes before every D
        let first_d = sorted.iter().position(|l| l.starts_with('D'));
        let last_c = sorted.iter().rposition(|l| l.starts_with('C'));
        if let (Some(d), Some(c)) = (first_d, last_c) {
            assert!(c < d);
        }
    }

    #[test]
    fn a_click_on_a_row_selects_the_item_and_centres_the_view_on_it() {
        let at = |x: f64| Shape::Circle {
            centre: P::new(x, 4.0),
            radius: 1.0,
        };
        let scene = sample::scene(
            &["GND"],
            &[
                (ItemKind::Designator, "R1 10k", None, at(2.0)),
                (ItemKind::Designator, "R2 10k", None, at(40.0)),
            ],
        );
        let mut viewer = Viewer::new(scene, ViewerSettings::default());
        assert!(focus(&mut viewer, 1));
        assert_eq!(viewer.selected, Some(1));
        assert!(
            (viewer.view.centre.x - 40.0).abs() < 1e-6,
            "{:?}",
            viewer.view.centre
        );
        assert!((viewer.view.centre.y - 4.0).abs() < 1e-6);
        assert!(focus(&mut viewer, 0));
        assert_eq!(viewer.selected, Some(0));
        assert!((viewer.view.centre.x - 2.0).abs() < 1e-6);
    }

    #[test]
    fn an_item_that_does_not_exist_changes_nothing() {
        let scene = scene_of(&[(ItemKind::Designator, "R1")]);
        let mut viewer = Viewer::new(scene, ViewerSettings::default());
        let before = viewer.view;
        assert!(!focus(&mut viewer, 1));
        assert!(!focus(&mut viewer, u32::MAX));
        assert_eq!(viewer.selected, None);
        assert_eq!(viewer.view, before);
    }
}
