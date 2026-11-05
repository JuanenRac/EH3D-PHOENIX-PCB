// SPDX-License-Identifier: GPL-3.0-or-later
//! The layers of KiCad and how they map onto the layers of the board model.
//!
//! A layer is created the first time a file names it. The layer table of a board gives the copper
//! layers their order; a footprint read alone names the layers its pads use and gets only those.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Id, Layer, LayerKind, Side};

use crate::conv::Ctx;

/// What a KiCad layer name means: the kind and the side.
pub(crate) fn meaning(name: &str) -> (LayerKind, Option<Side>) {
    if name.ends_with(".Cu") {
        return (LayerKind::Copper, None);
    }
    if name == "Edge.Cuts" {
        return (LayerKind::Outline, None);
    }
    let (side, rest) = if let Some(rest) = name.strip_prefix("F.") {
        (Some(Side::Top), rest)
    } else if let Some(rest) = name.strip_prefix("B.") {
        (Some(Side::Bottom), rest)
    } else {
        (None, name)
    };
    let kind = match rest {
        "SilkS" | "Silkscreen" => LayerKind::Silkscreen,
        "Mask" => LayerKind::SolderMask,
        "Paste" => LayerKind::Paste,
        _ => LayerKind::Documentation,
    };
    (kind, side)
}

/// The place of a copper layer in the stack: the top first, the inner layers by number, the bottom
/// last.
fn copper_rank(name: &str) -> u32 {
    if name == "F.Cu" {
        return 0;
    }
    if name == "B.Cu" {
        return u32::MAX;
    }
    name.strip_prefix("In")
        .and_then(|rest| rest.strip_suffix(".Cu"))
        .and_then(|number| number.parse::<u32>().ok())
        .unwrap_or(u32::MAX - 1)
}

/// The layers named so far.
#[derive(Default)]
pub(crate) struct LayerMap {
    by_name: BTreeMap<String, Id>,
    /// The layers by id, ready for a board.
    pub layers: BTreeMap<Id, Layer>,
}

impl LayerMap {
    /// The id of a layer, created when it is new.
    pub fn intern(&mut self, name: &str, ctx: &mut Ctx) -> Id {
        if let Some(id) = self.by_name.get(name) {
            return *id;
        }
        let id = ctx.id();
        let (kind, side) = meaning(name);
        self.by_name.insert(name.to_owned(), id);
        self.layers.insert(
            id,
            Layer {
                name: name.to_owned(),
                kind,
                side,
            },
        );
        id
    }

    /// The id of a layer that was already named.
    #[cfg(test)]
    pub fn get(&self, name: &str) -> Option<Id> {
        self.by_name.get(name).copied()
    }

    /// The copper layers from the top to the bottom.
    pub fn copper_stack(&self) -> Vec<Id> {
        let mut ranked: Vec<(u32, &str, Id)> = self
            .by_name
            .iter()
            .filter(|(name, _)| name.ends_with(".Cu"))
            .map(|(name, id)| (copper_rank(name), name.as_str(), *id))
            .collect();
        ranked.sort();
        ranked.into_iter().map(|(_, _, id)| id).collect()
    }

    /// The layers a name stands for: a plain name is one layer, `*.Cu` is every copper layer
    /// (the two outer ones always, even when they are not named), `F&B.Cu` the two outer ones and `*.Mask`
    /// the same layer on both sides.
    pub fn expand(&mut self, name: &str, ctx: &mut Ctx) -> Vec<Id> {
        if name == "*.Cu" {
            // a pad through the board always reaches the two outer layers, named or not
            self.intern("F.Cu", ctx);
            self.intern("B.Cu", ctx);
            return self.copper_stack();
        }
        if name == "F&B.Cu" {
            return vec![self.intern("F.Cu", ctx), self.intern("B.Cu", ctx)];
        }
        if let Some(rest) = name.strip_prefix("*.") {
            return vec![
                self.intern(&format!("F.{rest}"), ctx),
                self.intern(&format!("B.{rest}"), ctx),
            ];
        }
        vec![self.intern(name, ctx)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::KicadSettings;

    #[test]
    fn names_have_a_kind_and_a_side() {
        assert_eq!(meaning("F.Cu"), (LayerKind::Copper, None));
        assert_eq!(meaning("In2.Cu"), (LayerKind::Copper, None));
        assert_eq!(meaning("Edge.Cuts"), (LayerKind::Outline, None));
        assert_eq!(meaning("F.SilkS"), (LayerKind::Silkscreen, Some(Side::Top)));
        assert_eq!(
            meaning("B.Silkscreen"),
            (LayerKind::Silkscreen, Some(Side::Bottom))
        );
        assert_eq!(
            meaning("B.Mask"),
            (LayerKind::SolderMask, Some(Side::Bottom))
        );
        assert_eq!(meaning("F.Paste"), (LayerKind::Paste, Some(Side::Top)));
        assert_eq!(
            meaning("F.Fab"),
            (LayerKind::Documentation, Some(Side::Top))
        );
        assert_eq!(meaning("Dwgs.User"), (LayerKind::Documentation, None));
    }

    #[test]
    fn the_copper_stack_is_ordered_whatever_the_order_of_naming() {
        let settings = KicadSettings::default();
        let mut ids = crate::sequential_ids().unwrap();
        let mut ctx = Ctx::new(&settings, &mut ids);
        let mut map = LayerMap::default();
        let bottom = map.intern("B.Cu", &mut ctx);
        let in10 = map.intern("In10.Cu", &mut ctx);
        let in2 = map.intern("In2.Cu", &mut ctx);
        let top = map.intern("F.Cu", &mut ctx);
        map.intern("F.SilkS", &mut ctx);
        assert_eq!(map.copper_stack(), vec![top, in2, in10, bottom]);
        assert_eq!(map.intern("F.Cu", &mut ctx), top);
        assert_eq!(map.get("B.Cu"), Some(bottom));
        assert_eq!(map.get("nope"), None);
        assert_eq!(map.layers.len(), 5);
    }

    #[test]
    fn wildcards_expand_to_the_layers_they_stand_for() {
        let settings = KicadSettings::default();
        let mut ids = crate::sequential_ids().unwrap();
        let mut ctx = Ctx::new(&settings, &mut ids);
        let mut map = LayerMap::default();
        // nothing named yet: the two outer layers
        assert_eq!(map.expand("*.Cu", &mut ctx).len(), 2);
        map.intern("In1.Cu", &mut ctx);
        assert_eq!(map.expand("*.Cu", &mut ctx).len(), 3);
        assert_eq!(map.expand("F&B.Cu", &mut ctx).len(), 2);
        let mask = map.expand("*.Mask", &mut ctx);
        assert_eq!(mask.len(), 2);
        assert_eq!(map.expand("F.Paste", &mut ctx).len(), 1);
        assert_eq!(map.layers[&mask[0]].name, "F.Mask");
        assert_eq!(map.layers[&mask[1]].name, "B.Mask");
    }
}
