// SPDX-License-Identifier: GPL-3.0-or-later
//! The text netlist of `phoenix nets`: a plain format with one line for each net,
//!
//! ```text
//! NET GND U1.1 C1.2 C2.2
//! NET SIG R1.1 R2.2
//!     R3.1 R4.1
//! ```
//!
//! that is, the word `NET`, the name of the net and the pins `PART.PAD`, separated by blanks. A line
//! that starts with a blank goes on with the pins of the net above. A `#` starts a comment that
//! runs to the end of the line, and blank lines are ignored. Nothing here reads or writes a file
//! and nothing depends on the board model: the reading and the writing of the text, the
//! settings (`config/nets.cfg`), the comparison of two tables of nets and the making of a table
//! from the pins of a board are plain functions of text and numbers.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::nets_codes as codes;

/// The most characters of a piece of input that is quoted in a message.
const SHOWN: usize = 40;

/// A problem with the code of the table of `PHOENIX-NETS-<NNN>`, the line of the input it
/// belongs to (0 when it belongs to none) and what was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetsError {
    /// The code.
    pub code: &'static str,
    /// The line of the input, from 1, or 0.
    pub line: usize,
    /// What was found.
    pub detail: String,
}

impl NetsError {
    /// An error that belongs to no line.
    pub fn new(code: &'static str, detail: impl Into<String>) -> NetsError {
        NetsError {
            code,
            line: 0,
            detail: detail.into(),
        }
    }

    /// An error of a line of the input.
    pub fn at(code: &'static str, line: usize, detail: impl Into<String>) -> NetsError {
        NetsError {
            code,
            line,
            detail: detail.into(),
        }
    }

    /// The message with the code, the title in the language at `column` of `Lang::ALL`, the
    /// line when there is one and the detail.
    pub fn message(&self, column: usize) -> String {
        let mut text = format!("{}: {}", self.code, codes::title(self.code, column));
        if self.line > 0 {
            text.push_str(&format!(" (line {})", self.line));
        }
        if !self.detail.is_empty() {
            text.push_str(&format!(": {}", self.detail));
        }
        text
    }
}

fn shown(text: &str) -> String {
    let mut out: String = text.chars().take(SHOWN).collect();
    if text.chars().count() > SHOWN {
        out.push_str("...");
    }
    out
}

// ---------------------------------------------------------------------------------------------
// the settings
// ---------------------------------------------------------------------------------------------

/// The settings of one run, read from `config/nets.cfg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetsSettings {
    /// The largest netlist file that is read, in bytes.
    pub max_input_bytes: u64,
    /// The most nets in a netlist.
    pub max_nets: usize,
    /// The most pins on one net.
    pub max_pins_per_net: usize,
    /// The most pins in a netlist.
    pub max_pins: usize,
    /// The longest name of a net, of a part or of a pad, in characters.
    pub max_name_len: usize,
    /// The most differences that a comparison lists; the total is counted all the same.
    pub max_differences: usize,
    /// Nets that a comparison and the lint leave out (any case); a name that ends in `*`
    /// matches every net that starts with the rest.
    pub ignore_nets: Vec<String>,
    /// Whether a cell of a CSV file that starts with `=`, `+`, `-`, `@`, a tab or a carriage
    /// return gets a single quote in front of it, so that a spreadsheet does not take a net
    /// called `+5V` for a formula.
    pub csv_guard_formulas: bool,
}

impl Default for NetsSettings {
    fn default() -> Self {
        NetsSettings {
            max_input_bytes: 16 * 1024 * 1024,
            max_nets: 100_000,
            max_pins_per_net: 10_000,
            max_pins: 1_000_000,
            max_name_len: 128,
            max_differences: 1000,
            ignore_nets: Vec::new(),
            csv_guard_formulas: true,
        }
    }
}

impl NetsSettings {
    /// Parses a settings text of `key = value` lines. Blank lines and lines that start with `#`
    /// are ignored; an unknown key, a value that is not a number or is out of its range is
    /// refused with its line number.
    pub fn parse(text: &str) -> Result<NetsSettings, NetsError> {
        let mut s = NetsSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let bad = |what: String| NetsError::at(codes::SETTINGS_INVALID, line, what);
            let Some((key, value)) = trimmed.split_once('=') else {
                return Err(bad("a line is `key = value`".to_owned()));
            };
            let (key, value) = (key.trim(), value.trim());
            let number = |low: u64, high: u64| -> Result<u64, NetsError> {
                let n: u64 = value
                    .parse()
                    .map_err(|_| bad(format!("{key} is a whole number")))?;
                if n < low || n > high {
                    return Err(bad(format!("{key} is from {low} to {high}")));
                }
                Ok(n)
            };
            match key {
                "max_input_bytes" => s.max_input_bytes = number(1024, 1 << 30)?,
                "max_nets" => s.max_nets = number(1, 10_000_000)? as usize,
                "max_pins_per_net" => s.max_pins_per_net = number(1, 10_000_000)? as usize,
                "max_pins" => s.max_pins = number(1, 100_000_000)? as usize,
                "max_name_len" => s.max_name_len = number(1, 1024)? as usize,
                "max_differences" => s.max_differences = number(1, 10_000_000)? as usize,
                "csv_guard_formulas" => {
                    s.csv_guard_formulas = match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(bad("csv_guard_formulas is true or false".to_owned())),
                    }
                }
                "ignore_nets" => {
                    s.ignore_nets = value
                        .split(',')
                        .map(str::trim)
                        .filter(|w| !w.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                other => return Err(bad(format!("unknown key {}", shown(other)))),
            }
        }
        Ok(s)
    }

    /// Whether a net name is one that a comparison leaves out.
    pub fn ignores(&self, net: &str) -> bool {
        let net = net.to_lowercase();
        self.ignore_nets.iter().any(|pattern| {
            let pattern = pattern.to_lowercase();
            match pattern.strip_suffix('*') {
                Some(prefix) => net.starts_with(prefix),
                None => net == pattern,
            }
        })
    }
}

// ---------------------------------------------------------------------------------------------
// pins and tables
// ---------------------------------------------------------------------------------------------

/// Compares two texts so that `R2` comes before `R10`: runs of digits are compared as numbers.
/// Texts that are different never compare equal.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.as_bytes(), b.as_bytes());
    loop {
        match (x.first(), y.first()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(p), Some(q)) => {
                if p.is_ascii_digit() && q.is_ascii_digit() {
                    let dx = x.iter().take_while(|c| c.is_ascii_digit()).count();
                    let dy = y.iter().take_while(|c| c.is_ascii_digit()).count();
                    let (nx, ny) = (&x[..dx], &y[..dy]);
                    let nx = &nx[nx.iter().take_while(|c| **c == b'0').count()..];
                    let ny = &ny[ny.iter().take_while(|c| **c == b'0').count()..];
                    let order = nx.len().cmp(&ny.len()).then_with(|| nx.cmp(ny));
                    if order != Ordering::Equal {
                        return order;
                    }
                    x = &x[dx..];
                    y = &y[dy..];
                } else {
                    if p != q {
                        return p.cmp(q);
                    }
                    x = &x[1..];
                    y = &y[1..];
                }
            }
        }
    }
}

/// A pin: the reference of a part and the pad of its footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    /// The reference of the part (`R1`); it has no dot and no blank.
    pub part: String,
    /// The pad (`1`, `A3`); it has no blank.
    pub pad: String,
}

impl Pin {
    /// Reads `PART.PAD`: the part is what comes before the first dot. `None` when either side is
    /// empty or there is no dot.
    pub fn parse(token: &str) -> Option<Pin> {
        let (part, pad) = token.split_once('.')?;
        if part.is_empty() || pad.is_empty() {
            return None;
        }
        Some(Pin {
            part: part.to_owned(),
            pad: pad.to_owned(),
        })
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.part, self.pad)
    }
}

impl Ord for Pin {
    fn cmp(&self, other: &Pin) -> Ordering {
        natural_cmp(&self.part, &other.part).then_with(|| natural_cmp(&self.pad, &other.pad))
    }
}

impl PartialOrd for Pin {
    fn partial_cmp(&self, other: &Pin) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The nets of a design: the name of each net and the pins on it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NetTable {
    /// The pins of each net, by name.
    pub nets: BTreeMap<String, BTreeSet<Pin>>,
}

impl NetTable {
    /// The number of pins on all the nets.
    pub fn pin_count(&self) -> usize {
        self.nets.values().map(BTreeSet::len).sum()
    }

    /// The nets in natural order of their names, each with its pins.
    pub fn ordered(&self) -> Vec<(&String, &BTreeSet<Pin>)> {
        let mut nets: Vec<(&String, &BTreeSet<Pin>)> = self.nets.iter().collect();
        nets.sort_by(|a, b| natural_cmp(a.0, b.0));
        nets
    }

    /// The net each pin is on.
    fn owners(&self) -> BTreeMap<&Pin, &str> {
        let mut owners = BTreeMap::new();
        for (name, pins) in &self.nets {
            for pin in pins {
                owners.insert(pin, name.as_str());
            }
        }
        owners
    }

    /// The table without the nets that the settings ignore.
    fn without_ignored(&self, settings: &NetsSettings) -> NetTable {
        NetTable {
            nets: self
                .nets
                .iter()
                .filter(|(name, _)| !settings.ignores(name))
                .map(|(name, pins)| (name.clone(), pins.clone()))
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// reading and writing the text
// ---------------------------------------------------------------------------------------------

/// Reads the text of a netlist. The first problem stops the reading, with its line.
pub fn parse(text: &str, settings: &NetsSettings) -> Result<NetTable, NetsError> {
    if text.len() as u64 > settings.max_input_bytes {
        return Err(NetsError::new(
            codes::LIMIT_EXCEEDED,
            format!(
                "the text is larger than max_input_bytes {}",
                settings.max_input_bytes
            ),
        ));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut table = NetTable::default();
    let mut owners: BTreeMap<Pin, String> = BTreeMap::new();
    let mut total_pins = 0usize;
    let mut current: Option<String> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let tokens: Vec<&str> = raw
            .split_whitespace()
            .take_while(|t| !t.starts_with('#'))
            .collect();
        if tokens.is_empty() {
            continue;
        }
        let mut pin_tokens = &tokens[..];
        if raw.starts_with(char::is_whitespace) {
            if current.is_none() {
                return Err(NetsError::at(
                    codes::LINE_INVALID,
                    line,
                    "pins with no net above them",
                ));
            }
        } else {
            if !tokens[0].eq_ignore_ascii_case("NET") {
                return Err(NetsError::at(
                    codes::LINE_INVALID,
                    line,
                    format!("a line starts with NET, not {}", shown(tokens[0])),
                ));
            }
            let Some(name) = tokens.get(1) else {
                return Err(NetsError::at(
                    codes::LINE_INVALID,
                    line,
                    "NET needs the name of the net",
                ));
            };
            if name.chars().count() > settings.max_name_len {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    line,
                    format!("the name {} is longer than max_name_len", shown(name)),
                ));
            }
            if table.nets.contains_key(*name) {
                return Err(NetsError::at(codes::NET_TWICE, line, shown(name)));
            }
            if table.nets.len() >= settings.max_nets {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    line,
                    format!("more than max_nets {} nets", settings.max_nets),
                ));
            }
            table.nets.insert((*name).to_owned(), BTreeSet::new());
            current = Some((*name).to_owned());
            pin_tokens = &tokens[2..];
        }
        let net = current.clone().unwrap_or_default();
        for token in pin_tokens {
            let Some(pin) = Pin::parse(token) else {
                return Err(NetsError::at(codes::PIN_INVALID, line, shown(token)));
            };
            if pin.part.chars().count() > settings.max_name_len
                || pin.pad.chars().count() > settings.max_name_len
            {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    line,
                    format!("the pin {} is longer than max_name_len", shown(token)),
                ));
            }
            match owners.get(&pin) {
                Some(owner) if *owner != net => {
                    return Err(NetsError::at(
                        codes::PIN_ON_TWO_NETS,
                        line,
                        format!("{} is on {} and on {}", pin, shown(owner), shown(&net)),
                    ));
                }
                Some(_) => continue,
                None => {}
            }
            let pins = table.nets.entry(net.clone()).or_default();
            if pins.len() >= settings.max_pins_per_net {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    line,
                    format!(
                        "more than max_pins_per_net {} pins on {}",
                        settings.max_pins_per_net,
                        shown(&net)
                    ),
                ));
            }
            total_pins += 1;
            if total_pins > settings.max_pins {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    line,
                    format!("more than max_pins {} pins", settings.max_pins),
                ));
            }
            pins.insert(pin.clone());
            owners.insert(pin, net.clone());
        }
    }
    Ok(table)
}

/// The text of a table, the same every time: the nets in natural order of their names, the pins
/// in natural order, and a net with many pins goes on over lines that start with four blanks so
/// that no line is longer than 100 characters (a single long word excepted).
pub fn render(table: &NetTable) -> String {
    const WIDTH: usize = 100;
    let mut text = String::new();
    for (name, pins) in table.ordered() {
        let mut line = format!("NET {name}");
        let mut on_line = 0usize;
        for pin in pins {
            let word = pin.to_string();
            if on_line > 0 && line.len() + 1 + word.len() > WIDTH {
                text.push_str(&line);
                text.push('\n');
                line = String::from("   ");
                on_line = 0;
            }
            line.push(' ');
            line.push_str(&word);
            on_line += 1;
        }
        text.push_str(&line);
        text.push('\n');
    }
    text
}

// ---------------------------------------------------------------------------------------------
// the table of a board
// ---------------------------------------------------------------------------------------------

/// Something that was left out or changed when the pins of a board were written as a netlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The code.
    pub code: &'static str,
    /// What was done.
    pub detail: String,
}

fn writable_word(word: &str) -> bool {
    !word.is_empty() && !word.starts_with('#') && !word.chars().any(char::is_whitespace)
}

/// Makes the table of nets from the pins of a board, given as `(net, part, pad)`. A pin whose
/// part or pad cannot be written (an empty word, a blank in it, a dot in the part, a part that
/// starts with `#`) is left out; the name of a net with a blank in it, or one that starts with `#`,
/// is written with underscores; a pin that two nets claim stays on the first. Every one of these
/// is a note, which is how the writing says that the netlist is not exactly the board.
pub fn table_of_pins<I>(pins: I) -> (NetTable, Vec<Note>)
where
    I: IntoIterator<Item = (String, String, String)>,
{
    let mut table = NetTable::default();
    let mut notes: Vec<Note> = Vec::new();
    let mut renamed: BTreeSet<String> = BTreeSet::new();
    let mut origin: BTreeMap<String, String> = BTreeMap::new();
    let mut owners: BTreeMap<Pin, String> = BTreeMap::new();
    for (net, part, pad) in pins {
        if !writable_word(&part) || part.contains('.') || !writable_word(&pad) {
            notes.push(Note {
                code: codes::NAME_UNWRITABLE,
                detail: format!("the pin {} of {} is left out", shown(&pad), shown(&part)),
            });
            continue;
        }
        let mut written: String = net
            .chars()
            .map(|c| if c.is_whitespace() { '_' } else { c })
            .collect();
        if written.starts_with('#') {
            written.replace_range(0..1, "_");
        }
        if written.is_empty() {
            notes.push(Note {
                code: codes::NAME_UNWRITABLE,
                detail: format!(
                    "the pin {}.{} has a net without a name",
                    shown(&part),
                    shown(&pad)
                ),
            });
            continue;
        }
        if written != net && renamed.insert(net.clone()) {
            notes.push(Note {
                code: codes::NAME_UNWRITABLE,
                detail: format!("the net {} is written as {}", shown(&net), shown(&written)),
            });
        }
        match origin.get(&written) {
            Some(first) if *first != net => {
                if renamed.insert(format!("{first}\u{0}{net}")) {
                    notes.push(Note {
                        code: codes::NAME_UNWRITABLE,
                        detail: format!(
                            "the nets {} and {} are both written as {}",
                            shown(first),
                            shown(&net),
                            shown(&written)
                        ),
                    });
                }
            }
            Some(_) => {}
            None => {
                origin.insert(written.clone(), net.clone());
            }
        }
        let pin = Pin { part, pad };
        match owners.get(&pin) {
            Some(first) if *first != written => {
                notes.push(Note {
                    code: codes::PIN_ON_TWO_NETS,
                    detail: format!(
                        "{pin} is on {} and on {}; it stays on the first",
                        shown(first),
                        shown(&written)
                    ),
                });
                continue;
            }
            Some(_) => continue,
            None => {}
        }
        owners.insert(pin.clone(), written.clone());
        table.nets.entry(written).or_default().insert(pin);
    }
    (table, notes)
}

// ---------------------------------------------------------------------------------------------
// the comparison
// ---------------------------------------------------------------------------------------------

/// One difference between a netlist file and a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// The code (`PHOENIX-NETS-011` to `PHOENIX-NETS-016`).
    pub code: &'static str,
    /// The net the difference is about: of the file, or of the board for the codes 012 and 015.
    pub net: String,
    /// The pin, for the differences of a pin.
    pub pin: Option<Pin>,
    /// The other net: the one the pin is on on the board (013), or the name on the board (016).
    pub other: Option<String>,
}

/// The result of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// The differences, at most `max_differences` of them.
    pub differences: Vec<Difference>,
    /// How many there are in all.
    pub total: usize,
}

/// Compares the nets of a netlist file with those of a board. The nets that the settings ignore
/// are left out on both sides. A net of the file that is not on the board and has the same pins
/// as a net of the board that is not in the file is reported once as a rename. Every pin is
/// compared on its own: a pin of the file on another net or on none of the board, and a pin of the
/// board that is on no net of the file.
pub fn compare(file: &NetTable, board: &NetTable, settings: &NetsSettings) -> Comparison {
    let file = file.without_ignored(settings);
    let board = board.without_ignored(settings);
    let in_file = file.owners();
    let on_board = board.owners();
    let mut found: Vec<Difference> = Vec::new();
    let mut renamed_file: BTreeSet<&str> = BTreeSet::new();
    let mut renamed_board: BTreeSet<&str> = BTreeSet::new();

    // the nets that only one side has, and the ones that are the same with another name
    let mut board_only: BTreeMap<&BTreeSet<Pin>, &str> = BTreeMap::new();
    for (name, pins) in &board.nets {
        if !file.nets.contains_key(name) && !pins.is_empty() {
            board_only.entry(pins).or_insert(name.as_str());
        }
    }
    for (name, pins) in &file.nets {
        if board.nets.contains_key(name) {
            continue;
        }
        match board_only.get(pins).copied() {
            Some(other) if !pins.is_empty() && !renamed_board.contains(other) => {
                renamed_file.insert(name.as_str());
                renamed_board.insert(other);
                found.push(Difference {
                    code: codes::NET_RENAMED,
                    net: name.clone(),
                    pin: None,
                    other: Some(other.to_owned()),
                });
            }
            _ => found.push(Difference {
                code: codes::NET_ONLY_IN_FILE,
                net: name.clone(),
                pin: None,
                other: None,
            }),
        }
    }
    for name in board.nets.keys() {
        if !file.nets.contains_key(name) && !renamed_board.contains(name.as_str()) {
            found.push(Difference {
                code: codes::NET_ONLY_ON_BOARD,
                net: name.clone(),
                pin: None,
                other: None,
            });
        }
    }

    // the pins
    for (name, pins) in &file.nets {
        if renamed_file.contains(name.as_str()) {
            continue;
        }
        for pin in pins {
            match on_board.get(pin) {
                Some(net) if *net == name.as_str() => {}
                Some(net) => found.push(Difference {
                    code: codes::PIN_ON_OTHER_NET,
                    net: name.clone(),
                    pin: Some(pin.clone()),
                    other: Some((*net).to_owned()),
                }),
                None => found.push(Difference {
                    code: codes::PIN_NOT_ON_BOARD,
                    net: name.clone(),
                    pin: Some(pin.clone()),
                    other: None,
                }),
            }
        }
    }
    for (name, pins) in &board.nets {
        if renamed_board.contains(name.as_str()) {
            continue;
        }
        for pin in pins {
            if !in_file.contains_key(pin) {
                found.push(Difference {
                    code: codes::PIN_NOT_IN_FILE,
                    net: name.clone(),
                    pin: Some(pin.clone()),
                    other: None,
                });
            }
        }
    }
    let total = found.len();
    found.truncate(settings.max_differences);
    Comparison {
        differences: found,
        total,
    }
}

// ---------------------------------------------------------------------------------------------
// the lint
// ---------------------------------------------------------------------------------------------

/// One finding of the lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFinding {
    /// The code (`PHOENIX-NETS-017` to `PHOENIX-NETS-019`).
    pub code: &'static str,
    /// The net: the first of the nets for the names that are alike.
    pub net: String,
    /// The other nets with a name alike, joined by a comma and a blank.
    pub other: Option<String>,
}

/// The result of the lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintResult {
    /// The findings, at most `max_differences` of them.
    pub findings: Vec<LintFinding>,
    /// How many there are in all.
    pub total: usize,
}

/// The name of a net without its case and without the characters that people use as
/// separators, to find the names that are alike.
fn alike_key(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '_' | '-' | '.' | '/' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Looks for nets that are probably a mistake: a net with one pin (an unconnected input, or a
/// misspelt name that split a net in two), a net with no pin and nets whose names differ only in
/// case or in the separators (`GND` and `Gnd`, `VCC_3V3` and `VCC-3V3`). The nets that the settings
/// ignore are left out.
pub fn lint(table: &NetTable, settings: &NetsSettings) -> LintResult {
    let mut found: Vec<LintFinding> = Vec::new();
    let mut groups: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for (name, pins) in table.ordered() {
        if settings.ignores(name) {
            continue;
        }
        let code = match pins.len() {
            0 => Some(codes::NET_EMPTY),
            1 => Some(codes::NET_SINGLE_PIN),
            _ => None,
        };
        if let Some(code) = code {
            found.push(LintFinding {
                code,
                net: name.clone(),
                other: None,
            });
        }
        groups.entry(alike_key(name)).or_default().push(name);
    }
    for names in groups.values() {
        if let Some((first, rest)) = names.split_first() {
            if !rest.is_empty() {
                let rest: Vec<&str> = rest.iter().map(|n| n.as_str()).collect();
                found.push(LintFinding {
                    code: codes::NET_NAMES_ALIKE,
                    net: (*first).clone(),
                    other: Some(rest.join(", ")),
                });
            }
        }
    }
    let total = found.len();
    found.truncate(settings.max_differences);
    LintResult {
        findings: found,
        total,
    }
}

// ---------------------------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------------------------

/// One cell of a CSV file: a cell with a comma, a double quote or a line end is quoted, and with
/// `guard` a cell that a spreadsheet would read as a formula (it starts with `=`, `+`, `-`, `@`, a
/// tab or a carriage return) gets a single quote in front of it.
pub fn csv_field(value: &str, guard: bool) -> String {
    let mut text = value.to_owned();
    if guard && text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        text.insert(0, '\'');
    }
    if text.contains([',', '"', '\n', '\r']) {
        text = format!("\"{}\"", text.replace('"', "\"\""));
    }
    text
}

/// The pins of a table as CSV, one row for each pin under the header `net,part,pad`, in the order
/// of the netlist. A net with no pin has no row.
pub fn pins_csv(table: &NetTable, guard: bool) -> String {
    let mut text = String::from("net,part,pad\n");
    for (name, pins) in table.ordered() {
        for pin in pins {
            text.push_str(&format!(
                "{},{},{}\n",
                csv_field(name, guard),
                csv_field(&pin.part, guard),
                csv_field(&pin.pad, guard)
            ));
        }
    }
    text
}

/// The differences of a comparison as CSV under the header `code,net,pin,other`.
pub fn differences_csv(differences: &[Difference], guard: bool) -> String {
    let mut text = String::from("code,net,pin,other\n");
    for d in differences {
        let pin = d.pin.as_ref().map(ToString::to_string).unwrap_or_default();
        let other = d.other.clone().unwrap_or_default();
        text.push_str(&format!(
            "{},{},{},{}\n",
            d.code,
            csv_field(&d.net, guard),
            csv_field(&pin, guard),
            csv_field(&other, guard)
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> NetsSettings {
        NetsSettings::default()
    }

    fn table(text: &str) -> NetTable {
        parse(text, &settings()).unwrap()
    }

    fn error_of(text: &str) -> NetsError {
        parse(text, &settings()).unwrap_err()
    }

    #[test]
    fn a_netlist_is_read_with_comments_blank_lines_and_continuations() {
        let t = table(
            "# a netlist\n\nNET GND U1.1 C1.2   # the ground\n  C2.2 C3.2\nnet SIG R1.1\nNET EMPTY\n",
        );
        assert_eq!(t.nets.len(), 3);
        assert_eq!(t.nets["GND"].len(), 4);
        assert!(t.nets["SIG"].contains(&Pin::parse("R1.1").unwrap()));
        assert!(t.nets["EMPTY"].is_empty());
        assert_eq!(t.pin_count(), 5);
    }

    #[test]
    fn a_byte_order_mark_and_windows_line_ends_are_accepted() {
        let t = table("\u{feff}NET A R1.1\r\nNET B R1.2\r\n");
        assert_eq!(t.nets.len(), 2);
    }

    #[test]
    fn a_pad_may_have_a_dot_and_the_part_ends_at_the_first_dot() {
        let pin = Pin::parse("J1.1.2").unwrap();
        assert_eq!((pin.part.as_str(), pin.pad.as_str()), ("J1", "1.2"));
        assert_eq!(Pin::parse("J1."), None);
        assert_eq!(Pin::parse(".1"), None);
        assert_eq!(Pin::parse("J1"), None);
    }

    #[test]
    fn the_problems_of_a_file_are_refused_with_their_line() {
        let e = error_of("NET A R1.1\nWIRE B R2.1\n");
        assert_eq!((e.code, e.line), (codes::LINE_INVALID, 2));
        let e = error_of("NET\n");
        assert_eq!((e.code, e.line), (codes::LINE_INVALID, 1));
        let e = error_of("  R1.1\n");
        assert_eq!((e.code, e.line), (codes::LINE_INVALID, 1));
        let e = error_of("NET A R1.1\nNET A R2.1\n");
        assert_eq!((e.code, e.line), (codes::NET_TWICE, 2));
        let e = error_of("NET A R1\n");
        assert_eq!((e.code, e.line), (codes::PIN_INVALID, 1));
        let e = error_of("NET A R1.1\nNET B R1.1\n");
        assert_eq!((e.code, e.line), (codes::PIN_ON_TWO_NETS, 2));
    }

    #[test]
    fn a_pin_written_twice_on_the_same_net_is_one_pin() {
        let t = table("NET A R1.1 R1.1\n  R1.1\n");
        assert_eq!(t.pin_count(), 1);
    }

    #[test]
    fn the_limits_of_the_settings_stop_the_reading() {
        let s = NetsSettings {
            max_nets: 2,
            max_pins_per_net: 2,
            max_pins: 3,
            max_name_len: 5,
            max_input_bytes: 1 << 20,
            ..NetsSettings::default()
        };
        let limit = |text: &str| parse(text, &s).unwrap_err();
        assert_eq!(limit("NET A\nNET B\nNET C\n").code, codes::LIMIT_EXCEEDED);
        assert_eq!(limit("NET A R1.1 R2.1 R3.1\n").code, codes::LIMIT_EXCEEDED);
        assert_eq!(
            limit("NET A R1.1 R2.1\nNET B R3.1 R4.1\n").code,
            codes::LIMIT_EXCEEDED
        );
        assert_eq!(limit("NET LONGNAME R1.1\n").code, codes::LIMIT_EXCEEDED);
        assert_eq!(limit("NET A LONGPART.1\n").code, codes::LIMIT_EXCEEDED);
        assert!(parse("NET A R1.1 R2.1\nNET B R3.1\n", &s).is_ok());
        let small = NetsSettings {
            max_input_bytes: 10,
            ..NetsSettings::default()
        };
        assert_eq!(
            parse("NET A R1.1 R2.1 R3.1\n", &small).unwrap_err().code,
            codes::LIMIT_EXCEEDED
        );
    }

    #[test]
    fn the_text_of_a_table_is_in_natural_order_and_is_read_back_the_same() {
        let t = table("NET Z R10.1 R2.1 R2.10 R2.9\nNET A U1.1\nNET N10 C1.1\nNET N2 C2.1\n");
        let text = render(&t);
        assert_eq!(
            text,
            "NET A U1.1\nNET N2 C2.1\nNET N10 C1.1\nNET Z R2.1 R2.9 R2.10 R10.1\n"
        );
        assert_eq!(table(&text), t);
    }

    #[test]
    fn a_net_with_many_pins_goes_over_lines_of_at_most_100_characters() {
        let mut t = NetTable::default();
        let pins = (1..=60)
            .map(|i| Pin::parse(&format!("R{i}.1")).unwrap())
            .collect();
        t.nets.insert("BIG".to_owned(), pins);
        let text = render(&t);
        assert!(text.lines().count() > 1);
        for (i, line) in text.lines().enumerate() {
            assert!(line.len() <= 100, "{line}");
            assert_eq!(line.starts_with("NET "), i == 0, "{line}");
        }
        assert_eq!(table(&text), t);
    }

    #[test]
    fn a_long_word_is_not_split_and_an_empty_net_is_written() {
        let mut t = NetTable::default();
        let name = "N".repeat(110);
        t.nets.insert(name.clone(), BTreeSet::new());
        t.nets.insert(
            "X".to_owned(),
            [Pin::parse(&format!("{}.1", "P".repeat(120))).unwrap()]
                .into_iter()
                .collect(),
        );
        let text = render(&t);
        assert!(text.contains(&format!("NET {name}\n")));
        assert_eq!(table(&text), t);
    }

    #[test]
    fn a_table_made_from_a_fixed_generator_is_read_back_the_same() {
        // a linear congruential generator with a fixed seed: the same netlists on every run
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move |n: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % n
        };
        for _ in 0..40 {
            let mut t = NetTable::default();
            let mut used = BTreeSet::new();
            for n in 0..(next(8) + 1) {
                let mut pins = BTreeSet::new();
                for _ in 0..next(30) {
                    let pin = Pin {
                        part: format!("{}{}", ["R", "C", "U", "J"][next(4) as usize], next(40)),
                        pad: format!("{}", next(12)),
                    };
                    if used.insert(pin.clone()) {
                        pins.insert(pin);
                    }
                }
                t.nets.insert(format!("NET_{n}"), pins);
            }
            let text = render(&t);
            assert_eq!(table(&text), t, "{text}");
            assert_eq!(render(&table(&text)), text);
        }
    }

    #[test]
    fn natural_order_compares_numbers_as_numbers_and_never_calls_different_texts_equal() {
        assert_eq!(natural_cmp("R2", "R10"), Ordering::Less);
        assert_eq!(natural_cmp("R10", "R2"), Ordering::Greater);
        assert_eq!(natural_cmp("R2", "R2"), Ordering::Equal);
        assert_ne!(natural_cmp("R01", "R1"), Ordering::Equal);
        assert_eq!(natural_cmp("R", "R1"), Ordering::Less);
        assert_eq!(natural_cmp("A9", "B1"), Ordering::Less);
        assert_eq!(natural_cmp("", "A"), Ordering::Less);
    }

    #[test]
    fn the_settings_are_read_with_their_ranges_and_refuse_what_is_wrong() {
        let s = NetsSettings::parse(
            "# comment\nmax_nets = 5\nmax_pins_per_net = 6\nmax_pins = 7\nmax_name_len = 8\nmax_differences = 9\nmax_input_bytes = 2048\nignore_nets = NC, GND*\n",
        )
        .unwrap();
        assert_eq!(
            (s.max_nets, s.max_pins_per_net, s.max_pins, s.max_name_len),
            (5, 6, 7, 8)
        );
        assert_eq!((s.max_differences, s.max_input_bytes), (9, 2048));
        assert_eq!(s.ignore_nets, vec!["NC", "GND*"]);
        assert_eq!(NetsSettings::parse("").unwrap(), NetsSettings::default());
        for bad in [
            "max_nets = 0",
            "max_nets = x",
            "max_name_len = 5000",
            "max_input_bytes = 10",
            "nothing = 1",
            "max_nets",
        ] {
            let e = NetsSettings::parse(&format!("\n{bad}\n")).unwrap_err();
            assert_eq!((e.code, e.line), (codes::SETTINGS_INVALID, 2), "{bad}");
        }
    }

    #[test]
    fn the_settings_file_of_the_repository_is_the_built_in_default() {
        let shipped = include_str!("../../../config/nets.cfg");
        assert_eq!(
            NetsSettings::parse(shipped).unwrap(),
            NetsSettings::default()
        );
    }

    #[test]
    fn a_cell_of_a_csv_file_is_quoted_and_guarded_against_formulas() {
        assert_eq!(csv_field("GND", true), "GND");
        assert_eq!(csv_field("a,b", true), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\"", true), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("two\nlines", false), "\"two\nlines\"");
        // a spreadsheet takes these for formulas, so each gets a quote in front
        for formula in ["=1+1", "+5V", "-5V", "@SUM", "\tx", "\rx"] {
            let guarded = csv_field(formula, true);
            assert!(guarded.trim_matches('"').starts_with('\''), "{formula:?}");
            assert!(!csv_field(formula, false).contains('\''), "{formula:?}");
        }
        // a quote in the middle is not a formula
        assert_eq!(csv_field("V+", true), "V+");
        assert_eq!(csv_field("", true), "");
    }

    #[test]
    fn the_pins_of_a_table_are_written_as_csv_in_the_order_of_the_netlist() {
        let t = table("NET +5V U1.1 C10.1 C2.1\nNET GND U1.2\nNET EMPTY\n");
        assert_eq!(
            pins_csv(&t, true),
            "net,part,pad\n'+5V,C2,1\n'+5V,C10,1\n'+5V,U1,1\nGND,U1,2\n"
        );
        assert_eq!(
            pins_csv(&t, false),
            "net,part,pad\n+5V,C2,1\n+5V,C10,1\n+5V,U1,1\nGND,U1,2\n"
        );
        assert_eq!(pins_csv(&NetTable::default(), true), "net,part,pad\n");
    }

    #[test]
    fn the_differences_are_written_as_csv() {
        let file = table("NET A R1.1 R2.1\nNET -B R3.1\n");
        let board = table("NET A R1.1\nNET C R2.1 R3.1\n");
        let c = compare(&file, &board, &settings());
        let text = differences_csv(&c.differences, true);
        assert!(text.starts_with("code,net,pin,other\n"), "{text}");
        assert!(text.contains("PHOENIX-NETS-011,'-B,,\n"), "{text}");
        assert!(text.contains("PHOENIX-NETS-013,A,R2.1,C\n"), "{text}");
        assert_eq!(text.lines().count(), 1 + c.total);
        assert_eq!(differences_csv(&[], true), "code,net,pin,other\n");
    }

    #[test]
    fn the_lint_finds_single_pin_nets_empty_nets_and_names_that_are_alike() {
        let t = table(
            "NET GND U1.1 U1.2\nNET Gnd R1.1 R1.2\nNET VCC_3V3 U2.1 U2.2\nNET VCC-3V3 R2.1 R2.2\nNET ALONE R9.1\nNET VOID\nNET GOOD C1.1 C1.2\n",
        );
        let r = lint(&t, &settings());
        let list: Vec<(&str, &str, Option<&str>)> = r
            .findings
            .iter()
            .map(|f| (f.code, f.net.as_str(), f.other.as_deref()))
            .collect();
        assert_eq!(
            list,
            vec![
                (codes::NET_SINGLE_PIN, "ALONE", None),
                (codes::NET_EMPTY, "VOID", None),
                (codes::NET_NAMES_ALIKE, "GND", Some("Gnd")),
                (codes::NET_NAMES_ALIKE, "VCC-3V3", Some("VCC_3V3")),
            ]
        );
        assert_eq!(r.total, 4);
    }

    #[test]
    fn three_names_that_are_alike_are_one_finding_and_ignored_nets_are_left_out() {
        let t = table("NET a.b U1.1 U1.2\nNET A_B U2.1 U2.2\nNET a-b U3.1 U3.2\nNET NC1 R1.1\n");
        let r = lint(&t, &settings());
        assert_eq!(r.total, 2);
        let alike = r
            .findings
            .iter()
            .find(|f| f.code == codes::NET_NAMES_ALIKE)
            .unwrap();
        assert_eq!(alike.net, "A_B");
        assert_eq!(alike.other.as_deref(), Some("a-b, a.b"));
        let s = NetsSettings::parse("ignore_nets = nc*\nmax_differences = 1").unwrap();
        let r = lint(&t, &s);
        assert_eq!((r.total, r.findings.len()), (1, 1));
        assert_eq!(lint(&table("NET A R1.1 R2.1\n"), &settings()).total, 0);
    }

    #[test]
    fn the_guard_against_formulas_is_a_setting_that_is_on_by_default() {
        assert!(NetsSettings::default().csv_guard_formulas);
        let off = NetsSettings::parse("csv_guard_formulas = false").unwrap();
        assert!(!off.csv_guard_formulas);
        assert!(NetsSettings::parse("csv_guard_formulas = 1").is_err());
    }

    #[test]
    fn the_ignored_nets_match_by_name_in_any_case_and_by_prefix() {
        let s = NetsSettings::parse("ignore_nets = NC, unconnected*").unwrap();
        assert!(s.ignores("nc"));
        assert!(s.ignores("UNCONNECTED-(R1-Pad2)"));
        assert!(!s.ignores("NCX"));
        assert!(!NetsSettings::default().ignores("NC"));
    }

    fn codes_of(c: &Comparison) -> Vec<&'static str> {
        c.differences.iter().map(|d| d.code).collect()
    }

    #[test]
    fn two_equal_tables_have_no_difference() {
        let t = table("NET A R1.1 R2.1\nNET B R1.2\n");
        let c = compare(&t, &t.clone(), &settings());
        assert_eq!((c.total, c.differences.len()), (0, 0));
    }

    #[test]
    fn a_net_that_only_one_side_has_is_reported_with_its_pins() {
        let file = table("NET A R1.1 R2.1\nNET B R3.1\n");
        let board = table("NET A R1.1 R2.1\nNET C R4.1\n");
        let c = compare(&file, &board, &settings());
        assert_eq!(
            codes_of(&c),
            vec![
                codes::NET_ONLY_IN_FILE,
                codes::NET_ONLY_ON_BOARD,
                codes::PIN_NOT_ON_BOARD,
                codes::PIN_NOT_IN_FILE
            ]
        );
        assert_eq!(c.differences[0].net, "B");
        assert_eq!(c.differences[1].net, "C");
        assert_eq!(c.differences[2].pin.as_ref().unwrap().to_string(), "R3.1");
        assert_eq!(c.differences[3].pin.as_ref().unwrap().to_string(), "R4.1");
    }

    #[test]
    fn a_pin_on_another_net_is_reported_once_from_the_side_of_the_file() {
        let file = table("NET A R1.1 R2.1\nNET B R3.1\n");
        let board = table("NET A R1.1\nNET B R2.1 R3.1\n");
        let c = compare(&file, &board, &settings());
        assert_eq!(codes_of(&c), vec![codes::PIN_ON_OTHER_NET]);
        let d = &c.differences[0];
        assert_eq!(d.net, "A");
        assert_eq!(d.other.as_deref(), Some("B"));
        assert_eq!(d.pin.as_ref().unwrap().to_string(), "R2.1");
    }

    #[test]
    fn a_net_with_another_name_and_the_same_pins_is_a_rename_and_not_a_pile_of_pins() {
        let file = table("NET VCC U1.1 C1.1\nNET GND U1.2\n");
        let board = table("NET +5V U1.1 C1.1\nNET GND U1.2\n");
        let c = compare(&file, &board, &settings());
        assert_eq!(codes_of(&c), vec![codes::NET_RENAMED]);
        assert_eq!(c.differences[0].net, "VCC");
        assert_eq!(c.differences[0].other.as_deref(), Some("+5V"));
    }

    #[test]
    fn a_rename_needs_the_same_pins_and_an_empty_net_is_never_one() {
        let file = table("NET A R1.1 R2.1\nNET E\n");
        let board = table("NET B R1.1\nNET F\n");
        let c = compare(&file, &board, &settings());
        assert!(!codes_of(&c).contains(&codes::NET_RENAMED));
        assert!(codes_of(&c).contains(&codes::NET_ONLY_IN_FILE));
        assert!(codes_of(&c).contains(&codes::NET_ONLY_ON_BOARD));
    }

    #[test]
    fn the_ignored_nets_are_left_out_on_both_sides_and_the_list_is_cut_at_the_limit() {
        let s = NetsSettings::parse("ignore_nets = NC*\nmax_differences = 2").unwrap();
        let file = table("NET NC1 R1.1\nNET A R2.1 R3.1 R4.1 R5.1\n");
        let board = table("NET NC2 R9.1\nNET A R2.1\n");
        let c = compare(&file, &board, &s);
        assert_eq!(c.total, 3);
        assert_eq!(c.differences.len(), 2);
        assert!(c.differences.iter().all(|d| d.net == "A"));
    }

    #[test]
    fn the_comparison_is_the_same_every_time() {
        let file = table("NET A R1.1 R2.1\nNET B R3.1\nNET Z U9.1\n");
        let board = table("NET A R1.1\nNET B R2.1 R3.1\nNET Y U8.1\n");
        let first = compare(&file, &board, &settings());
        for _ in 0..5 {
            assert_eq!(compare(&file, &board, &settings()), first);
        }
    }

    fn pins(list: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
        list.iter()
            .map(|(n, p, a)| ((*n).to_owned(), (*p).to_owned(), (*a).to_owned()))
            .collect()
    }

    #[test]
    fn the_pins_of_a_board_make_the_table() {
        let (t, notes) = table_of_pins(pins(&[
            ("GND", "R1", "1"),
            ("GND", "C1", "2"),
            ("SIG", "R1", "2"),
        ]));
        assert!(notes.is_empty());
        assert_eq!(render(&t), "NET GND C1.2 R1.1\nNET SIG R1.2\n");
    }

    #[test]
    fn what_cannot_be_written_is_left_out_or_renamed_and_noted() {
        let (t, notes) = table_of_pins(pins(&[
            ("MY NET", "R1", "1"),
            ("MY NET", "R2", "1"),
            ("#X", "R3", "1"),
            ("A", "R 4", "1"),
            ("A", "R.5", "1"),
            ("A", "#R6", "1"),
            ("A", "R7", ""),
            ("", "R8", "1"),
            ("OK", "R9", "1"),
        ]));
        assert_eq!(
            render(&t),
            "NET MY_NET R1.1 R2.1\nNET OK R9.1\nNET _X R3.1\n"
        );
        let all: Vec<&str> = notes.iter().map(|n| n.code).collect();
        assert!(all.iter().all(|c| *c == codes::NAME_UNWRITABLE));
        // MY NET once, #X, and five pins left out (four words, one empty net)
        assert_eq!(notes.len(), 2 + 5);
        let text = render(&t);
        assert_eq!(table(&text), t);
    }

    #[test]
    fn two_nets_that_get_the_same_written_name_and_a_pin_on_two_nets_are_noted() {
        let (t, notes) = table_of_pins(pins(&[
            ("A B", "R1", "1"),
            ("A_B", "R2", "1"),
            ("X", "R3", "1"),
            ("Y", "R3", "1"),
        ]));
        assert_eq!(t.nets.len(), 2);
        assert_eq!(t.nets["A_B"].len(), 2);
        assert_eq!(t.nets["X"].len(), 1);
        let on_two = notes
            .iter()
            .filter(|n| n.code == codes::PIN_ON_TWO_NETS)
            .count();
        assert_eq!(on_two, 1);
        assert!(notes.iter().any(|n| n.detail.contains("both written as")));
    }

    #[test]
    fn a_message_has_the_code_the_title_the_line_and_what_was_found() {
        let e = NetsError::at(codes::PIN_INVALID, 3, "R1");
        assert_eq!(
            e.message(0),
            "PHOENIX-NETS-004: Pin is not written as part and pad (line 3): R1"
        );
        let e = NetsError::new(codes::OUTPUT_FAILED, "");
        assert_eq!(
            e.message(1),
            "PHOENIX-NETS-009: No se pudo escribir el fichero de salida de la lista de redes"
        );
        assert_eq!(shown(&"x".repeat(50)).chars().count(), SHOWN + 3);
    }
}
