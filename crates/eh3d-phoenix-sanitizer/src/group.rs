// SPDX-License-Identifier: GPL-3.0-or-later
//! Groups of duplicates: footprints and symbols with the same canonical hash, or close enough to
//! be a proposal, and device sets that use equivalent footprints and symbols the same way.
//!
//! A group never merges anything: it names the item to keep and the items that look like it, with
//! the score of each and the facts the owner needs to confirm (whether the pads are named the same
//! way, whether a replacement would turn the part).

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::codes::{COMPARE_LIMIT, NAMES_DIFFER, WEAK_LINK};
use crate::fuzzy::{self, Features, Mode};
use crate::hash::{canonical, CanonHash};
use crate::model::{DeviceSetData, Item, Prim};
use crate::scan::{LoadedLibrary, Note};
use crate::settings::{MasterPick, SanitizerSettings};

/// The kinds of group, in the order of the report.
pub const KINDS: [&str; 3] = ["package", "symbol", "deviceset"];

/// One member of a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Member {
    /// The library file, relative to the folder.
    pub library: String,
    /// The name in that library.
    pub name: String,
    /// The geometric hash (the signature for a device set).
    pub hash: String,
    /// The score against the item to keep, 0 to 100.
    pub score: u32,
    /// True when the geometry is the same as the item to keep.
    pub exact: bool,
    /// True when a replacement would have to turn, mirror or shift the part.
    pub turned: bool,
    /// How many device sets of its library use it.
    pub uses: usize,
    /// True for the item to keep.
    pub keep: bool,
}

/// A group of items that look like one another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    /// A short id such as `P001`, `S002` or `D003`.
    pub id: String,
    /// `package`, `symbol` or `deviceset`.
    pub kind: String,
    /// The lowest score of the members against the item to keep.
    pub score: u32,
    /// True when every member has the geometry of the item to keep.
    pub exact: bool,
    /// True when the pads or pins are named the same way in every member.
    pub names_agree: bool,
    /// True when a member is below the threshold and is in the group only through other members.
    pub weak: bool,
    /// The members, the item to keep marked.
    pub members: Vec<Member>,
}

impl Group {
    /// The member to keep.
    pub fn keep(&self) -> Option<&Member> {
        self.members.iter().find(|m| m.keep)
    }
}

/// What the grouping found and what it could not do.
#[derive(Debug, Default)]
pub struct Grouping {
    /// The groups, with their ids.
    pub groups: Vec<Group>,
    /// The notes: weak links, names that differ, limits reached.
    pub notes: Vec<Note>,
}

struct Entry {
    lib: usize,
    name: String,
    hash: CanonHash,
    features: Features,
    names_sig: String,
    uses: usize,
    has_description: bool,
    size: usize,
}

struct Union {
    parent: Vec<usize>,
}

impl Union {
    fn new(n: usize) -> Union {
        Union {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut a: usize) -> usize {
        while self.parent[a] != a {
            self.parent[a] = self.parent[self.parent[a]];
            a = self.parent[a];
        }
        a
    }

    fn join(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            // the smaller index stays the root, so the result does not depend on the order
            let (low, high) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[high] = low;
        }
    }
}

fn names_signature(prims: &[Prim]) -> String {
    let mut names: Vec<&str> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Pad { name, .. } | Prim::Pin { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    names.sort_unstable();
    names.join(",")
}

fn entries_of(libs: &[LoadedLibrary], kind: &str, mode: Mode, s: &SanitizerSettings) -> Vec<Entry> {
    let mut out = Vec::new();
    for (lib_index, lib) in libs.iter().enumerate() {
        let Some(data) = &lib.data else { continue };
        let items: &[Item] = if kind == "package" {
            &data.packages
        } else {
            &data.symbols
        };
        for item in items {
            let uses = data
                .devicesets
                .iter()
                .filter(|set| {
                    if kind == "package" {
                        set.devices
                            .iter()
                            .any(|d| d.package.as_deref() == Some(item.name.as_str()))
                    } else {
                        set.gates.iter().any(|g| g.1 == item.name)
                    }
                })
                .count();
            out.push(Entry {
                lib: lib_index,
                name: item.name.clone(),
                hash: canonical(&item.prims, kind, s),
                features: fuzzy::features(&item.prims, mode, s),
                names_sig: names_signature(&item.prims),
                uses,
                has_description: item.has_description,
                size: item.prims.len(),
            });
        }
    }
    out.sort_by(|a, b| (&libs[a.lib].path, &a.name).cmp(&(&libs[b.lib].path, &b.name)));
    out
}

/// The index of the member to keep, among `members` (indexes into `entries`).
fn pick_master(
    members: &[usize],
    entries: &[Entry],
    libs: &[LoadedLibrary],
    s: &SanitizerSettings,
) -> usize {
    let key = |i: usize| {
        let e = &entries[i];
        match s.master_pick {
            MasterPick::Richest => (
                Reverse(e.has_description),
                Reverse(e.uses),
                Reverse(e.size),
                &libs[e.lib].path,
                &e.name,
            ),
            MasterPick::First => (
                Reverse(false),
                Reverse(0),
                Reverse(0),
                &libs[e.lib].path,
                &e.name,
            ),
        }
    };
    members
        .iter()
        .copied()
        .min_by(|a, b| key(*a).cmp(&key(*b)))
        .unwrap_or(members[0])
}

/// Groups the footprints (`package`) or the symbols (`symbol`) of all the libraries.
pub fn group_items(
    libs: &[LoadedLibrary],
    kind: &str,
    s: &SanitizerSettings,
    pair_budget: &mut u64,
) -> Grouping {
    let mode = if kind == "package" {
        Mode::Footprint
    } else {
        Mode::Symbol
    };
    let entries = entries_of(libs, kind, mode, s);
    let mut result = Grouping::default();

    // the nodes: items with the same geometric hash are one node
    let mut by_hash: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        by_hash.entry(e.hash.geometry.as_str()).or_default().push(i);
    }
    let nodes: Vec<Vec<usize>> = by_hash.into_values().collect();

    // the buckets: only items with the same kinds of anchors can be close to one another
    let mut buckets: BTreeMap<[usize; 5], Vec<usize>> = BTreeMap::new();
    let mut too_big = 0usize;
    for (n, node) in nodes.iter().enumerate() {
        let f = &entries[node[0]].features;
        if f.anchors.len() > s.max_anchors {
            too_big += 1;
            continue;
        }
        let mut key = [f.anchors.len(), 0, 0, 0, 0];
        for a in &f.anchors {
            key[1 + usize::from(a.kind.min(3))] += 1;
        }
        buckets.entry(key).or_default().push(n);
    }
    if too_big > 0 {
        result.notes.push(Note::new(
            COMPARE_LIMIT,
            "",
            format!("{too_big} {kind} items have more than {} pads or pins and were only compared by hash", s.max_anchors),
        ));
    }

    let mut union = Union::new(nodes.len());
    let mut limit_hit = false;
    'buckets: for list in buckets.values() {
        for (a_pos, &a) in list.iter().enumerate() {
            for &b in &list[a_pos + 1..] {
                if *pair_budget == 0 {
                    limit_hit = true;
                    break 'buckets;
                }
                *pair_budget -= 1;
                let fa = &entries[nodes[a][0]].features;
                let fb = &entries[nodes[b][0]].features;
                let (score, _) = fuzzy::best_score(fa, fb, mode, s, s.threshold);
                if score >= s.threshold {
                    union.join(a, b);
                }
            }
        }
    }
    if limit_hit {
        result.notes.push(Note::new(
            COMPARE_LIMIT,
            "",
            format!("{kind}: the limit of {} pairs was reached", s.max_pairs),
        ));
    }

    // the components
    let mut components: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (n, node) in nodes.iter().enumerate() {
        let root = union.find(n);
        components
            .entry(root)
            .or_default()
            .extend(node.iter().copied());
    }
    for mut members in components.into_values() {
        if members.len() < 2 {
            continue;
        }
        members.sort_unstable();
        let master = pick_master(&members, &entries, libs, s);
        let m = &entries[master];
        let mut out_members = Vec::with_capacity(members.len());
        for &i in &members {
            let e = &entries[i];
            let same = e.hash.geometry == m.hash.geometry;
            let (score, turned) = if same {
                (100, e.hash.transform != m.hash.transform)
            } else {
                let (sc, t) = fuzzy::best_score(&m.features, &e.features, mode, s, 0);
                (sc, !t.is_identity())
            };
            out_members.push(Member {
                library: libs[e.lib].path.clone(),
                name: e.name.clone(),
                hash: e.hash.geometry.clone(),
                score,
                exact: same,
                turned,
                uses: e.uses,
                keep: i == master,
            });
        }
        let exact = out_members.iter().all(|x| x.exact);
        // the same geometry is compared in the same frame, so the named hashes tell whether the
        // pads are numbered the same way; a close one is compared by the set of its names
        let names_agree = members.iter().all(|&i| {
            let e = &entries[i];
            if e.hash.geometry == m.hash.geometry {
                e.hash.named == m.hash.named
            } else {
                e.names_sig == m.names_sig
            }
        });
        let score = out_members.iter().map(|x| x.score).min().unwrap_or(100);
        let weak = out_members.iter().any(|x| x.score < s.threshold);
        result.groups.push(Group {
            id: String::new(),
            kind: kind.to_owned(),
            score,
            exact,
            names_agree,
            weak,
            members: out_members,
        });
    }
    finish(&mut result, kind);
    result
}

/// Sorts the groups and gives them their ids; adds the notes that the groups call for.
fn finish(result: &mut Grouping, kind: &str) {
    result.groups.sort_by(|a, b| {
        let key = |g: &Group| {
            g.keep()
                .map(|m| (m.library.clone(), m.name.clone()))
                .unwrap_or_default()
        };
        key(a).cmp(&key(b))
    });
    let letter = match kind {
        "package" => 'P',
        "symbol" => 'S',
        _ => 'D',
    };
    for (i, g) in result.groups.iter_mut().enumerate() {
        g.id = format!("{letter}{:03}", i + 1);
        let library = g.keep().map(|m| m.library.clone()).unwrap_or_default();
        if !g.names_agree {
            result
                .notes
                .push(Note::new(NAMES_DIFFER, &library, g.id.clone()));
        }
        if g.weak {
            result
                .notes
                .push(Note::new(WEAK_LINK, &library, g.id.clone()));
        }
    }
}

/// The class of an item for the signature of a device set: the group when it is in one, and its
/// own hash otherwise. The raw one is always the hash.
type ClassMap = BTreeMap<(usize, String), (String, String)>;

fn class_map(
    libs: &[LoadedLibrary],
    kind: &str,
    groups: &[Group],
    s: &SanitizerSettings,
) -> ClassMap {
    let mut in_group: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for g in groups {
        for m in &g.members {
            in_group.insert((m.library.as_str(), m.name.as_str()), g.id.as_str());
        }
    }
    let mut map = ClassMap::new();
    for (i, lib) in libs.iter().enumerate() {
        let Some(data) = &lib.data else { continue };
        let items = if kind == "package" {
            &data.packages
        } else {
            &data.symbols
        };
        for item in items {
            let raw = format!("h:{}", canonical(&item.prims, kind, s).geometry);
            let logical = match in_group.get(&(lib.path.as_str(), item.name.as_str())) {
                Some(id) => format!("g:{id}"),
                None => raw.clone(),
            };
            map.insert((i, item.name.clone()), (raw, logical));
        }
    }
    map
}

fn signature(
    set: &DeviceSetData,
    lib: usize,
    packages: &ClassMap,
    symbols: &ClassMap,
    logical: bool,
) -> String {
    let pick = |pair: Option<&(String, String)>, name: &str| match pair {
        Some((raw, log)) => (if logical { log } else { raw }).clone(),
        None => format!("?{name}"),
    };
    let symbol_of_gate = |gate: &str| {
        set.gates
            .iter()
            .find(|g| g.0 == gate)
            .map(|g| g.1.clone())
            .unwrap_or_default()
    };
    let mut lines = vec![format!("X|{}", set.prefix)];
    let mut gates: Vec<String> = set
        .gates
        .iter()
        .map(|g| format!("G|{}", pick(symbols.get(&(lib, g.1.clone())), &g.1)))
        .collect();
    gates.sort();
    lines.extend(gates);
    let mut devices: Vec<String> = Vec::new();
    for d in &set.devices {
        let package = match &d.package {
            Some(p) => pick(packages.get(&(lib, p.clone())), p),
            None => "none".to_owned(),
        };
        let mut connects: Vec<String> = d
            .connects
            .iter()
            .map(|(gate, pin, pad)| {
                let symbol = symbol_of_gate(gate);
                let class = pick(symbols.get(&(lib, symbol.clone())), &symbol);
                format!("{class}.{pin}>{pad}")
            })
            .collect();
        connects.sort();
        devices.push(format!("D|{package}|{}", connects.join(";")));
    }
    devices.sort();
    lines.extend(devices);
    let mut hasher = Sha256::new();
    for line in &lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Groups the device sets: those that use footprints and symbols of the same group (or the same
/// ones) with the same prefix and the same connections of pins to pads.
pub fn group_device_sets(
    libs: &[LoadedLibrary],
    packages: &[Group],
    symbols: &[Group],
    s: &SanitizerSettings,
) -> Grouping {
    let package_classes = class_map(libs, "package", packages, s);
    let symbol_classes = class_map(libs, "symbol", symbols, s);
    let scores = |groups: &[Group]| {
        let mut map: BTreeMap<(String, String), u32> = BTreeMap::new();
        for g in groups {
            for m in &g.members {
                map.insert((m.library.clone(), m.name.clone()), m.score);
            }
        }
        map
    };
    let package_scores = scores(packages);
    let symbol_scores = scores(symbols);
    // a device set with its two signatures and the lowest score of the footprints and symbols it uses
    struct SetEntry<'a> {
        lib: usize,
        set: &'a DeviceSetData,
        raw: String,
        logical: String,
        floor: u32,
    }
    let mut entries: Vec<SetEntry<'_>> = Vec::new();
    for (i, lib) in libs.iter().enumerate() {
        let Some(data) = &lib.data else { continue };
        for set in &data.devicesets {
            let mut floor = 100;
            for g in &set.gates {
                if let Some(v) = symbol_scores.get(&(lib.path.clone(), g.1.clone())) {
                    floor = floor.min(*v);
                }
            }
            for d in &set.devices {
                if let Some(p) = &d.package {
                    if let Some(v) = package_scores.get(&(lib.path.clone(), p.clone())) {
                        floor = floor.min(*v);
                    }
                }
            }
            entries.push(SetEntry {
                lib: i,
                set,
                raw: signature(set, i, &package_classes, &symbol_classes, false),
                logical: signature(set, i, &package_classes, &symbol_classes, true),
                floor,
            });
        }
    }
    entries.sort_by(|a, b| (&libs[a.lib].path, &a.set.name).cmp(&(&libs[b.lib].path, &b.set.name)));
    let mut by_logical: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        by_logical.entry(e.logical.as_str()).or_default().push(i);
    }
    let mut result = Grouping::default();
    for members in by_logical.into_values() {
        if members.len() < 2 {
            continue;
        }
        let pick_key = |i: usize| {
            let e = &entries[i];
            match s.master_pick {
                MasterPick::Richest => (
                    Reverse(e.set.has_description),
                    Reverse(e.set.devices.len()),
                    &libs[e.lib].path,
                    &e.set.name,
                ),
                MasterPick::First => (Reverse(false), Reverse(0), &libs[e.lib].path, &e.set.name),
            }
        };
        let master = members
            .iter()
            .copied()
            .min_by(|a, b| pick_key(*a).cmp(&pick_key(*b)))
            .unwrap_or(members[0]);
        let m = &entries[master];
        let out_members: Vec<Member> = members
            .iter()
            .map(|&i| {
                let e = &entries[i];
                let same = e.raw == m.raw;
                Member {
                    library: libs[e.lib].path.clone(),
                    name: e.set.name.clone(),
                    hash: e.raw.clone(),
                    score: if same { 100 } else { e.floor.min(m.floor) },
                    exact: same,
                    turned: false,
                    uses: 0,
                    keep: i == master,
                }
            })
            .collect();
        let exact = out_members.iter().all(|x| x.exact);
        let score = out_members.iter().map(|x| x.score).min().unwrap_or(100);
        let weak = out_members.iter().any(|x| x.score < s.threshold);
        result.groups.push(Group {
            id: String::new(),
            kind: "deviceset".to_owned(),
            score,
            exact,
            names_agree: true,
            weak,
            members: out_members,
        });
    }
    finish(&mut result, "deviceset");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::load_bytes;
    use crate::testdata::{deviceset_xml, library_xml, package_xml, symbol_xml};

    fn lib(path: &str, text: &str, s: &SanitizerSettings) -> LoadedLibrary {
        let mut l = LoadedLibrary {
            path: path.into(),
            bytes: text.len() as u64,
            sha256: String::new(),
            version: String::new(),
            data: None,
            notes: Vec::new(),
        };
        load_bytes(&mut l, text.as_bytes(), s);
        l
    }

    const TWO: [(f64, f64); 2] = [(-0.9, 0.0), (0.9, 0.0)];

    #[test]
    fn exact_duplicates_across_libraries_form_a_group_and_the_richest_is_kept() {
        let s = SanitizerSettings::default();
        let a = lib(
            "old/a.lbr",
            &library_xml(&[package_xml("R0805", &TWO, 1.2, 1.4)], &[], &[]),
            &s,
        );
        // the same geometry under another name and in another order of pads
        let b_text = library_xml(&[package_xml("RES-0805", &TWO, 1.2, 1.4)], &[], &[])
            .replace("<description>test footprint</description>", "");
        let b = lib("old/b.lbr", &b_text, &s);
        let c = lib(
            "new/c.lbr",
            &library_xml(
                &[package_xml(
                    "C0603",
                    &[(-0.75, 0.0), (0.75, 0.0)],
                    0.8,
                    0.95,
                )],
                &[],
                &[],
            ),
            &s,
        );
        let mut budget = s.max_pairs;
        let g = group_items(&[a, b, c], "package", &s, &mut budget);
        assert_eq!(g.groups.len(), 1, "{:?}", g.groups);
        let group = &g.groups[0];
        assert_eq!(group.id, "P001");
        assert!(group.exact && group.names_agree && !group.weak);
        assert_eq!(group.score, 100);
        let names: Vec<&str> = group.members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["R0805", "RES-0805"]);
        // the one with a description is kept
        assert_eq!(group.keep().unwrap().name, "R0805");
        assert!(g.notes.is_empty());
    }

    #[test]
    fn close_but_not_equal_items_group_by_the_threshold_with_a_score() {
        let s = SanitizerSettings::default();
        let a = lib(
            "a.lbr",
            &library_xml(&[package_xml("A", &TWO, 1.2, 1.4)], &[], &[]),
            &s,
        );
        let b = lib(
            "b.lbr",
            &library_xml(
                &[package_xml("B", &[(-0.92, 0.0), (0.9, 0.0)], 1.2, 1.4)],
                &[],
                &[],
            ),
            &s,
        );
        let far = lib(
            "c.lbr",
            &library_xml(
                &[package_xml("C", &[(-1.5, 0.0), (1.5, 0.0)], 1.2, 1.4)],
                &[],
                &[],
            ),
            &s,
        );
        let mut budget = s.max_pairs;
        let g = group_items(
            &[a.clone(), b.clone(), far.clone()],
            "package",
            &s,
            &mut budget,
        );
        assert_eq!(g.groups.len(), 1);
        let group = &g.groups[0];
        assert!(!group.exact);
        assert!(group.score >= 90 && group.score < 100, "{}", group.score);
        assert_eq!(group.members.len(), 2);
        // the top threshold does not group them, the bottom one groups all that share a bucket
        let strict = SanitizerSettings {
            threshold: 100,
            ..SanitizerSettings::default()
        };
        let mut budget = strict.max_pairs;
        assert!(group_items(
            &[a.clone(), b.clone(), far.clone()],
            "package",
            &strict,
            &mut budget
        )
        .groups
        .is_empty());
        let loose = SanitizerSettings {
            threshold: 0,
            ..SanitizerSettings::default()
        };
        let mut budget = loose.max_pairs;
        let g = group_items(&[a, b, far], "package", &loose, &mut budget);
        assert_eq!(g.groups.len(), 1);
        assert_eq!(g.groups[0].members.len(), 3);
    }

    #[test]
    fn a_chain_of_matches_is_marked_weak_and_noted() {
        let s = SanitizerSettings {
            threshold: 85,
            ..SanitizerSettings::default()
        };
        let make = |name: &str, dx: f64| {
            lib(
                &format!("{name}.lbr"),
                &library_xml(
                    &[package_xml(name, &[(-0.9 + dx, 0.0), (0.9, 0.0)], 1.2, 1.4)],
                    &[],
                    &[],
                ),
                &s,
            )
        };
        // 0.9 mm and 0.9 mm - 0.04 and 0.9 - 0.08: each step is close, the ends are further apart
        let libs = [make("A", 0.0), make("B", -0.03), make("C", -0.06)];
        let mut budget = s.max_pairs;
        let g = group_items(&libs, "package", &s, &mut budget);
        assert_eq!(g.groups.len(), 1);
        assert_eq!(g.groups[0].members.len(), 3);
        let min = g.groups[0].score;
        assert!(min < 100);
        assert_eq!(g.groups[0].weak, min < s.threshold);
    }

    #[test]
    fn duplicates_that_number_their_pads_differently_are_flagged() {
        let s = SanitizerSettings::default();
        let a_text = library_xml(&[package_xml("A", &TWO, 1.2, 1.4)], &[], &[]);
        let b_text = a_text
            .replace("name=\"A\"", "name=\"B\"")
            .replace("smd name=\"1\"", "smd name=\"X\"")
            .replace("smd name=\"2\"", "smd name=\"1\"")
            .replace("smd name=\"X\"", "smd name=\"2\"");
        let libs = [lib("a.lbr", &a_text, &s), lib("b.lbr", &b_text, &s)];
        let mut budget = s.max_pairs;
        let g = group_items(&libs, "package", &s, &mut budget);
        assert_eq!(g.groups.len(), 1);
        assert!(g.groups[0].exact);
        assert!(!g.groups[0].names_agree);
        assert!(g
            .notes
            .iter()
            .any(|n| n.code == NAMES_DIFFER && n.detail == "P001"));
    }

    #[test]
    fn the_pair_budget_stops_the_comparison_and_says_so() {
        let s = SanitizerSettings::default();
        let libs: Vec<LoadedLibrary> = (0..4)
            .map(|i| {
                lib(
                    &format!("{i}.lbr"),
                    &library_xml(
                        &[package_xml(
                            "P",
                            &[(-0.9 - f64::from(i), 0.0), (0.9, 0.0)],
                            1.2,
                            1.4,
                        )],
                        &[],
                        &[],
                    ),
                    &s,
                )
            })
            .collect();
        let mut budget = 2;
        let g = group_items(&libs, "package", &s, &mut budget);
        assert_eq!(budget, 0);
        assert!(g.notes.iter().any(|n| n.code == COMPARE_LIMIT));
    }

    #[test]
    fn symbols_group_by_their_pins_and_body() {
        let s = SanitizerSettings::default();
        let pins = [("1", 0.0, 0.0), ("2", 5.08, 0.0)];
        let a = lib(
            "a.lbr",
            &library_xml(&[], &[symbol_xml("R", &pins, 1.0)], &[]),
            &s,
        );
        let other_names = [("A", 0.0, 0.0), ("B", 5.08, 0.0)];
        let b = lib(
            "b.lbr",
            &library_xml(&[], &[symbol_xml("RESISTOR", &other_names, 1.0)], &[]),
            &s,
        );
        let mut budget = s.max_pairs;
        let g = group_items(&[a, b], "symbol", &s, &mut budget);
        assert_eq!(g.groups.len(), 1);
        assert_eq!(g.groups[0].id, "S001");
        assert!(g.groups[0].exact);
        // the pin names differ: the geometry is the same, the names are not
        assert!(!g.groups[0].names_agree);
    }

    #[test]
    fn device_sets_that_use_equivalent_parts_the_same_way_are_grouped() {
        let s = SanitizerSettings::default();
        let pins = [("1", 0.0, 0.0), ("2", 5.08, 0.0)];
        let conn = [("1", "1"), ("2", "2")];
        let a = lib(
            "a.lbr",
            &library_xml(
                &[package_xml("R0805", &TWO, 1.2, 1.4)],
                &[symbol_xml("R", &pins, 1.0)],
                &[deviceset_xml("R", "R", "R", "R0805", &conn)],
            ),
            &s,
        );
        let b = lib(
            "b.lbr",
            &library_xml(
                &[package_xml("RES", &TWO, 1.2, 1.4)],
                &[symbol_xml("RES-SYM", &pins, 1.0)],
                &[deviceset_xml("RESISTOR", "R", "RES-SYM", "RES", &conn)],
            ),
            &s,
        );
        let c = lib(
            "c.lbr",
            &library_xml(
                &[package_xml("RES", &TWO, 1.2, 1.4)],
                &[symbol_xml("RES-SYM", &pins, 1.0)],
                &[deviceset_xml("CAP", "C", "RES-SYM", "RES", &conn)],
            ),
            &s,
        );
        let libs = [a, b, c];
        let mut budget = s.max_pairs;
        let packages = group_items(&libs, "package", &s, &mut budget);
        let symbols = group_items(&libs, "symbol", &s, &mut budget);
        let sets = group_device_sets(&libs, &packages.groups, &symbols.groups, &s);
        assert_eq!(sets.groups.len(), 1, "{:?}", sets.groups);
        let g = &sets.groups[0];
        assert_eq!(g.id, "D001");
        assert_eq!(g.members.len(), 2);
        assert!(g.exact && g.names_agree);
        // the prefix tells the capacitor apart
        assert!(g.members.iter().all(|m| m.name != "CAP"));
    }

    #[test]
    fn the_first_pick_keeps_the_first_by_path_and_name() {
        let s = SanitizerSettings {
            master_pick: MasterPick::First,
            ..SanitizerSettings::default()
        };
        let plain = library_xml(&[package_xml("Z", &TWO, 1.2, 1.4)], &[], &[])
            .replace("<description>test footprint</description>", "");
        let rich = library_xml(&[package_xml("A", &TWO, 1.2, 1.4)], &[], &[]);
        let libs = [lib("a.lbr", &plain, &s), lib("b.lbr", &rich, &s)];
        let mut budget = s.max_pairs;
        let g = group_items(&libs, "package", &s, &mut budget);
        assert_eq!(g.groups[0].keep().unwrap().library, "a.lbr");
        let richest = SanitizerSettings::default();
        let mut budget = richest.max_pairs;
        let g = group_items(&libs, "package", &richest, &mut budget);
        assert_eq!(g.groups[0].keep().unwrap().library, "b.lbr");
    }
}
