// SPDX-License-Identifier: GPL-3.0-or-later
//! The symbols that can be placed: those of the schematic itself and those of the libraries that
//! were loaded (any schematic file, or a JSON list of symbol definitions), with a search.

use eh3d_phoenix_schematic::{PinDef, Schematic, SymbolDef};

use crate::codes::{SchEditError, OPERATIONS_INVALID};

/// The most symbols a palette holds.
pub const MAX_ENTRIES: usize = 100_000;

/// A symbol in the palette: where it came from and its definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    /// The name of the library (or of the schematic) it came from.
    pub library: String,
    /// The symbol: its name and its pins.
    pub def: SymbolDef,
}

impl PaletteEntry {
    /// The name of the symbol.
    pub fn name(&self) -> &str {
        &self.def.name
    }
}

/// The sort key of a pin number: whole numbers first, in numeric order, then the others.
fn number_key(number: &str) -> (u8, u64, String) {
    match number.trim().parse::<u64>() {
        Ok(v) => (0, v, String::new()),
        Err(_) => (1, 0, number.trim().to_lowercase()),
    }
}

/// The symbols of a schematic that can be placed again (every pin has a position), pins in the
/// order of their numbers.
pub fn symbols_of(s: &Schematic) -> Vec<SymbolDef> {
    let mut out = Vec::new();
    for symbol in s.symbols.values() {
        let mut pins: Vec<PinDef> = Vec::new();
        for (pin_id, pin) in &symbol.pins {
            if let Some(at) = pin.at {
                pins.push(PinDef {
                    number: pin.number.clone(),
                    at,
                    kind: s.pin_kinds.get(pin_id).copied(),
                });
            }
        }
        if pins.is_empty() || pins.len() != symbol.pins.len() {
            continue;
        }
        pins.sort_by_key(|p| number_key(&p.number));
        out.push(SymbolDef {
            name: symbol.name.clone(),
            pins,
        });
    }
    out
}

/// The list of symbols that can be placed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    entries: Vec<PaletteEntry>,
}

impl Palette {
    /// An empty palette.
    pub fn new() -> Palette {
        Palette::default()
    }

    /// How many symbols.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when there are none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All the entries, in the order they were added.
    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    /// The first entry with the name.
    pub fn find(&self, name: &str) -> Option<&PaletteEntry> {
        self.entries.iter().find(|e| e.def.name == name)
    }

    /// Adds a symbol; one with the same library and name is not added twice. Returns whether it
    /// was added.
    pub fn add(&mut self, library: &str, def: SymbolDef) -> bool {
        if self.entries.len() >= MAX_ENTRIES
            || self
                .entries
                .iter()
                .any(|e| e.library == library && e.def.name == def.name)
        {
            return false;
        }
        self.entries.push(PaletteEntry {
            library: library.to_owned(),
            def,
        });
        true
    }

    /// Adds the symbols of a schematic as a library. Returns how many were added.
    pub fn add_schematic(&mut self, library: &str, s: &Schematic) -> usize {
        symbols_of(s)
            .into_iter()
            .map(|def| usize::from(self.add(library, def)))
            .sum()
    }

    /// Adds a library from JSON: a list of symbol definitions (`[{"name": "R", "pins": [...]}]`).
    /// Returns how many were added.
    pub fn add_json(&mut self, library: &str, text: &str) -> Result<usize, SchEditError> {
        let defs: Vec<SymbolDef> = serde_json::from_str(text).map_err(|e| {
            SchEditError::new(OPERATIONS_INVALID, format!("the library {library}: {e}"))
        })?;
        Ok(defs
            .into_iter()
            .map(|def| usize::from(self.add(library, def)))
            .sum())
    }

    /// The symbols that match a search, best first: every word of the query must be in the name or
    /// in the library; an exact name comes first, then a name that starts with the first word, then
    /// a name that contains it, then a library that does. At most `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&PaletteEntry> {
        let lower = query.to_lowercase();
        let terms: Vec<&str> = lower.split_whitespace().collect();
        let mut found: Vec<(u8, String, &PaletteEntry)> = Vec::new();
        for entry in &self.entries {
            let name = entry.def.name.to_lowercase();
            let library = entry.library.to_lowercase();
            if !terms
                .iter()
                .all(|t| name.contains(t) || library.contains(t))
            {
                continue;
            }
            let rank = match terms.first() {
                None => 4,
                Some(t) if name == *t => 0,
                Some(t) if name.starts_with(t) => 1,
                Some(t) if name.contains(t) => 2,
                Some(_) => 3,
            };
            found.push((rank, format!("{name}\u{0}{library}"), entry));
        }
        found.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        found.into_iter().take(limit).map(|(_, _, e)| e).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{fixture, resistor};

    #[test]
    fn the_symbols_of_a_schematic_become_entries_once() {
        let f = fixture();
        let mut palette = Palette::new();
        assert_eq!(palette.add_schematic("project", &f.schematic), 1);
        assert_eq!(palette.add_schematic("project", &f.schematic), 0);
        let entry = palette.find("R").unwrap();
        assert_eq!(entry.def.pins.len(), 2);
        assert_eq!(entry.def.pins[0].number, "1");
        assert_eq!(entry.def.pins[0].kind, resistor().pins[0].kind);
        assert_eq!(entry.library, "project");
        assert!(palette.find("Nothing").is_none());
        assert_eq!(palette.len(), 1);
        assert!(!palette.is_empty());
    }

    #[test]
    fn a_search_ranks_exact_then_prefix_then_contained() {
        let f = fixture();
        let mut palette = Palette::new();
        palette.add_schematic("project", &f.schematic);
        let library = r#"[
            {"name": "Capacitor", "pins": [{"number": "1", "at": {"x_nm": 0, "y_nm": 0}}]},
            {"name": "Resistor", "pins": [{"number": "1", "at": {"x_nm": 0, "y_nm": 0}}]}
        ]"#;
        assert_eq!(palette.add_json("Passives", library).unwrap(), 2);
        let names = |q: &str| -> Vec<String> {
            palette
                .search(q, 10)
                .iter()
                .map(|e| e.name().to_owned())
                .collect()
        };
        assert_eq!(names("r"), vec!["R", "Resistor", "Capacitor"]);
        assert_eq!(names("RES"), vec!["Resistor"]);
        assert_eq!(names("passives cap"), vec!["Capacitor"]);
        assert_eq!(names("zzz"), Vec::<String>::new());
        assert_eq!(names("").len(), 3);
        assert_eq!(palette.search("", 2).len(), 2);
    }

    #[test]
    fn a_library_that_is_not_a_list_of_symbols_is_refused() {
        let mut palette = Palette::new();
        for bad in [
            "",
            "{}",
            "[{\"name\": 1}]",
            "[{\"name\": \"A\", \"extra\": 1, \"pins\": []}]",
        ] {
            let error = palette.add_json("x", bad).unwrap_err();
            assert_eq!(error.code, OPERATIONS_INVALID, "{bad}");
        }
        assert!(palette.is_empty());
    }
}
