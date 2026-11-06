// SPDX-License-Identifier: GPL-3.0-or-later
//! The sheets of a hierarchical schematic in the window of the editor, as data that is tested
//! without a screen: the tabs (one per sheet, in the order of the tree, with the number of
//! instances of a sheet that several symbols show), the way into the sheet a symbol shows and back
//! out of it, the sheet symbol under a point, and the marks that the window draws for sheet
//! symbols, their pins, the ports of a sheet and the buses. The window itself is in
//! `schematic_gui`, behind the `gui` feature, and has not been seen on a screen.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_model::Point;
use eh3d_phoenix_schematic::{HierConfig, Id, Schematic};

/// A tab of the row of sheets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetTab {
    /// The sheet.
    pub id: Id,
    /// Its name.
    pub name: String,
    /// How deep it is in the tree: 0 for a top sheet.
    pub depth: usize,
    /// How many instances it has: 2 for a sheet that two symbols show, 0 for a sheet that no tree
    /// reaches (it is in a loop).
    pub instances: usize,
}

/// The tabs: the sheets in the order of the tree (a sheet before the sheets its symbols show), each
/// sheet once, then the sheets that the tree does not reach.
pub fn tabs(s: &Schematic) -> Vec<SheetTab> {
    let hierarchy = s.hierarchy(&HierConfig::default());
    let mut counts: BTreeMap<Id, usize> = BTreeMap::new();
    for instance in &hierarchy.instances {
        *counts.entry(instance.sheet).or_default() += 1;
    }
    let mut seen: BTreeSet<Id> = BTreeSet::new();
    let mut out = Vec::new();
    for instance in &hierarchy.instances {
        if !seen.insert(instance.sheet) {
            continue;
        }
        if let Some(sheet) = s.sheets.get(&instance.sheet) {
            out.push(SheetTab {
                id: instance.sheet,
                name: sheet.name.clone(),
                depth: instance.path.len(),
                instances: counts.get(&instance.sheet).copied().unwrap_or(0),
            });
        }
    }
    for (id, sheet) in &s.sheets {
        if seen.insert(*id) {
            out.push(SheetTab {
                id: *id,
                name: sheet.name.clone(),
                depth: 0,
                instances: 0,
            });
        }
    }
    out
}

/// The way through the sheets: entering a sheet symbol remembers the sheet it was on, leaving goes
/// back to it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Navigation {
    trail: Vec<Id>,
}

impl Navigation {
    /// A way with nothing behind it.
    pub fn new() -> Navigation {
        Navigation::default()
    }

    /// Enters the sheet that the symbol `block` shows, when the symbol is drawn on `from`. Returns
    /// the sheet to show.
    pub fn enter(&mut self, s: &Schematic, from: Id, block: Id) -> Option<Id> {
        let symbol = s.sheet_blocks.get(&block)?;
        if symbol.sheet != from || !s.sheets.contains_key(&symbol.child) {
            return None;
        }
        self.trail.push(from);
        Some(symbol.child)
    }

    /// Goes back to the sheet that was left last and still exists.
    pub fn leave(&mut self, s: &Schematic) -> Option<Id> {
        while let Some(id) = self.trail.pop() {
            if s.sheets.contains_key(&id) {
                return Some(id);
            }
        }
        None
    }

    /// How many sheets are behind.
    pub fn depth(&self) -> usize {
        self.trail.len()
    }

    /// Forgets the way (after a jump to a sheet by its tab).
    pub fn clear(&mut self) {
        self.trail.clear();
    }
}

/// The sheet symbol drawn on `sheet` that contains the point, the smallest one when they overlap.
pub fn block_at(s: &Schematic, sheet: Id, at: Point) -> Option<Id> {
    let (px, py) = (at.x_nm.get(), at.y_nm.get());
    s.sheet_blocks
        .iter()
        .filter(|(_, b)| b.sheet == sheet)
        .filter(|(_, b)| {
            let (x, y) = (b.at.x_nm.get(), b.at.y_nm.get());
            px >= x
                && py >= y
                && px <= x.saturating_add(b.width.get())
                && py <= y.saturating_add(b.height.get())
        })
        .min_by_key(|(id, b)| (i128::from(b.width.get()) * i128::from(b.height.get()), **id))
        .map(|(id, _)| *id)
}

/// What a mark stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    /// The outline of a sheet symbol; `w_mm` and `h_mm` are its size.
    Block,
    /// A pin of a sheet symbol.
    Pin,
    /// A port of the sheet.
    Port,
    /// A bus.
    Bus,
    /// A bus entry.
    Entry,
}

/// Something the window draws on a sheet for the hierarchy and the buses: a position in
/// millimetres, a size (for a block) and a text.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    /// What it is.
    pub kind: MarkKind,
    /// The x of the point (the corner for a block), in millimetres.
    pub x_mm: f64,
    /// The y of the point, in millimetres.
    pub y_mm: f64,
    /// The width in millimetres; 0 except for a block.
    pub w_mm: f64,
    /// The height in millimetres; 0 except for a block.
    pub h_mm: f64,
    /// The text to write beside it.
    pub text: String,
    /// The id of the object (the symbol, pin, port, bus or entry).
    pub id: Id,
}

fn mm(nm: i64) -> f64 {
    nm as f64 / 1_000_000.0
}

/// The marks of a sheet, in a fixed order: symbols, pins, ports, buses, entries.
pub fn marks(s: &Schematic, sheet: Id) -> Vec<Mark> {
    let mut out = Vec::new();
    for (id, b) in s.sheet_blocks.iter().filter(|(_, b)| b.sheet == sheet) {
        let shown = s.sheets.get(&b.child).map_or("", |c| c.name.as_str());
        out.push(Mark {
            kind: MarkKind::Block,
            x_mm: mm(b.at.x_nm.get()),
            y_mm: mm(b.at.y_nm.get()),
            w_mm: mm(b.width.get()),
            h_mm: mm(b.height.get()),
            text: format!("{} : {shown}", b.name),
            id: *id,
        });
    }
    for (id, p) in &s.hier_pins {
        let on_sheet = s
            .sheet_blocks
            .get(&p.block)
            .is_some_and(|b| b.sheet == sheet);
        if on_sheet {
            out.push(Mark {
                kind: MarkKind::Pin,
                x_mm: mm(p.at.x_nm.get()),
                y_mm: mm(p.at.y_nm.get()),
                w_mm: 0.0,
                h_mm: 0.0,
                text: p.name.clone(),
                id: *id,
            });
        }
    }
    for (id, p) in s.sheet_ports.iter().filter(|(_, p)| p.sheet == sheet) {
        out.push(Mark {
            kind: MarkKind::Port,
            x_mm: mm(p.at.x_nm.get()),
            y_mm: mm(p.at.y_nm.get()),
            w_mm: 0.0,
            h_mm: 0.0,
            text: p.name.clone(),
            id: *id,
        });
    }
    for (id, b) in s.buses.iter().filter(|(_, b)| b.sheet == sheet) {
        out.push(Mark {
            kind: MarkKind::Bus,
            x_mm: mm(b.at.x_nm.get()),
            y_mm: mm(b.at.y_nm.get()),
            w_mm: 0.0,
            h_mm: 0.0,
            text: b.spec.clone(),
            id: *id,
        });
    }
    for (id, e) in &s.bus_entries {
        let on_sheet = s.buses.get(&e.bus).is_some_and(|b| b.sheet == sheet);
        if on_sheet {
            out.push(Mark {
                kind: MarkKind::Entry,
                x_mm: mm(e.at.x_nm.get()),
                y_mm: mm(e.at.y_nm.get()),
                w_mm: 0.0,
                h_mm: 0.0,
                text: e.member.clone(),
                id: *id,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::Nm;
    use eh3d_phoenix_schematic::{EditOp, Editor, PortDir};

    fn pt(x_mm: i64, y_mm: i64) -> Point {
        Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
    }

    fn nm(v: i64) -> Nm {
        Nm::new(v * 1_000_000).unwrap()
    }

    /// A top sheet that two symbols show the sheet "Stage" with, and a third sheet nobody shows.
    fn design() -> (Schematic, [Id; 6]) {
        let mut ed = Editor::new(Schematic::default());
        let mut sheet = |name: &str| {
            ed.apply(&EditOp::AddSheet {
                name: name.into(),
                width: nm(297),
                height: nm(210),
            })
            .unwrap()
            .created[0]
        };
        let top = sheet("Top");
        let stage = sheet("Stage");
        let lone = sheet("Lone");
        let mut block = |name: &str, x: i64| {
            ed.apply(&EditOp::AddSheetBlock {
                sheet: top,
                child: stage,
                name: name.into(),
                at: pt(x, 50),
                width: nm(20),
                height: nm(10),
            })
            .unwrap()
            .created[0]
        };
        let a = block("A", 20);
        let b = block("B", 60);
        let pin = ed
            .apply(&EditOp::AddHierPin {
                block: a,
                name: "IN".into(),
                dir: PortDir::Input,
                at: pt(20, 55),
            })
            .unwrap()
            .created[0];
        ed.apply(&EditOp::AddSheetPort {
            sheet: stage,
            name: "IN".into(),
            dir: PortDir::Input,
            at: pt(10, 10),
        })
        .unwrap();
        (ed.into_schematic(), [top, stage, lone, a, b, pin])
    }

    #[test]
    fn the_tabs_follow_the_tree_and_count_the_instances() {
        let (s, [top, stage, lone, ..]) = design();
        let tabs = tabs(&s);
        let shown: Vec<(&str, usize, usize)> = tabs
            .iter()
            .map(|t| (t.name.as_str(), t.depth, t.instances))
            .collect();
        // the lone sheet is on top too: nothing shows it
        let ids: Vec<Id> = tabs.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[1], stage);
        assert!(ids.contains(&top) && ids.contains(&lone));
        assert!(shown.contains(&("Top", 0, 1)), "{shown:?}");
        assert!(shown.contains(&("Stage", 1, 2)), "{shown:?}");
        assert!(shown.contains(&("Lone", 0, 1)), "{shown:?}");
    }

    #[test]
    fn entering_a_symbol_and_leaving_it_go_back_where_we_were() {
        let (s, [top, stage, _, a, b, _]) = design();
        let mut way = Navigation::new();
        assert_eq!(way.depth(), 0);
        // a symbol that is not on the sheet we are on is not entered
        assert_eq!(way.enter(&s, stage, a), None);
        assert_eq!(way.depth(), 0);
        assert_eq!(way.enter(&s, top, a), Some(stage));
        assert_eq!(way.depth(), 1);
        assert_eq!(way.leave(&s), Some(top));
        assert_eq!(way.leave(&s), None);
        assert_eq!(way.enter(&s, top, b), Some(stage));
        way.clear();
        assert_eq!(way.depth(), 0);
        let nothing = Id::from_parts(1_800_000_000_000, 1, 77).unwrap();
        assert_eq!(way.enter(&s, top, nothing), None);
    }

    #[test]
    fn the_symbol_under_a_point_is_found_and_the_smallest_wins() {
        let (mut s, [top, _, _, a, b, _]) = design();
        assert_eq!(block_at(&s, top, pt(25, 55)), Some(a));
        assert_eq!(block_at(&s, top, pt(65, 52)), Some(b));
        assert_eq!(block_at(&s, top, pt(45, 55)), None);
        assert_eq!(block_at(&s, top, pt(20, 50)), Some(a));
        // a bigger symbol under the first one: the smaller one is picked
        let mut big = s.sheet_blocks[&b].clone();
        big.name = "BIG".into();
        big.at = pt(0, 0);
        big.width = nm(100);
        big.height = nm(100);
        let big_id = Id::from_parts(1_800_000_000_000, 2, 78).unwrap();
        s.sheet_blocks.insert(big_id, big);
        assert_eq!(block_at(&s, top, pt(25, 55)), Some(a));
        assert_eq!(block_at(&s, top, pt(45, 55)), Some(big_id));
    }

    #[test]
    fn the_marks_of_a_sheet_are_its_symbols_pins_ports_and_buses() {
        let (mut s, [top, stage, _, a, _, pin]) = design();
        let on_top = marks(&s, top);
        let kinds: Vec<MarkKind> = on_top.iter().map(|m| m.kind).collect();
        assert_eq!(kinds, vec![MarkKind::Block, MarkKind::Block, MarkKind::Pin]);
        assert_eq!(on_top[0].text, "A : Stage");
        assert_eq!((on_top[0].w_mm, on_top[0].h_mm), (20.0, 10.0));
        assert_eq!(on_top[2].id, pin);
        assert_eq!(on_top[0].id, a);
        let on_stage = marks(&s, stage);
        assert_eq!(on_stage.len(), 1);
        assert_eq!(on_stage[0].kind, MarkKind::Port);
        assert_eq!((on_stage[0].x_mm, on_stage[0].y_mm), (10.0, 10.0));
        // a bus and its entry are marks of the sheet of the bus
        let mut ed = Editor::new(std::mem::take(&mut s));
        let bus = ed
            .apply(&EditOp::PlaceBus {
                sheet: top,
                at: pt(5, 5),
                spec: "D[0..1]".into(),
            })
            .unwrap()
            .created[0];
        ed.apply(&EditOp::AddBusEntry {
            bus,
            member: "D0".into(),
            at: pt(6, 6),
        })
        .unwrap();
        let after = marks(ed.schematic(), top);
        assert!(after
            .iter()
            .any(|m| m.kind == MarkKind::Bus && m.text == "D[0..1]"));
        assert!(after
            .iter()
            .any(|m| m.kind == MarkKind::Entry && m.text == "D0"));
        assert!(marks(ed.schematic(), stage)
            .iter()
            .all(|m| m.kind == MarkKind::Port));
    }
}
