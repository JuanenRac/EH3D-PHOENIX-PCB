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
    /// Parts that a comparison and the lint leave out, with their pins (any case, `*` for any run
    /// of characters, as in [`pattern_matches`]): the test points and fiducials that a board has
    /// and a schematic has not. A net that has no pin left because of them is left out too.
    pub ignore_parts: Vec<String>,
    /// Whether a cell of a CSV file that starts with `=`, `+`, `-`, `@`, a tab or a carriage
    /// return gets a single quote in front of it, so that a spreadsheet does not take a net
    /// called `+5V` for a formula.
    pub csv_guard_formulas: bool,
    /// The most pins a net may have before the lint reports it (`PHOENIX-NETS-020`); 0 does not
    /// judge it.
    pub lint_max_pins: usize,
    /// Whether the lint reports the parts with two pins or more that are all on one net
    /// (`PHOENIX-NETS-021`).
    pub lint_part_on_one_net: bool,
    /// Whether the lint reports the nets whose name a program made (`PHOENIX-NETS-022`).
    pub lint_generated_names: bool,
    /// Whether the lint reports the nets of two pins or more that all belong to one part
    /// (`PHOENIX-NETS-023`).
    pub lint_one_part_nets: bool,
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
            ignore_parts: Vec::new(),
            csv_guard_formulas: true,
            lint_max_pins: 0,
            lint_part_on_one_net: false,
            lint_generated_names: false,
            lint_one_part_nets: false,
        }
    }
}

/// Reads `true` or `false` for a setting.
fn flag(key: &str, value: &str, line: usize) -> Result<bool, NetsError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(NetsError::at(
            codes::SETTINGS_INVALID,
            line,
            format!("{key} is true or false"),
        )),
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
                "csv_guard_formulas" => s.csv_guard_formulas = flag(key, value, line)?,
                "lint_max_pins" => s.lint_max_pins = number(0, 10_000_000)? as usize,
                "lint_part_on_one_net" => s.lint_part_on_one_net = flag(key, value, line)?,
                "lint_generated_names" => s.lint_generated_names = flag(key, value, line)?,
                "lint_one_part_nets" => s.lint_one_part_nets = flag(key, value, line)?,
                "ignore_parts" => {
                    s.ignore_parts = value
                        .split(',')
                        .map(str::trim)
                        .filter(|w| !w.is_empty())
                        .map(str::to_owned)
                        .collect();
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

    /// Whether a part is one that a comparison and the lint leave out (see [`pattern_matches`]).
    pub fn ignores_part(&self, part: &str) -> bool {
        self.ignore_parts.iter().any(|p| pattern_matches(p, part))
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

    /// The table without the nets that the settings ignore and without the pins of the parts that
    /// they ignore; a net that had pins and has none left is left out, and a net that had none
    /// stays.
    fn without_ignored(&self, settings: &NetsSettings) -> NetTable {
        let mut nets = BTreeMap::new();
        for (name, pins) in &self.nets {
            if settings.ignores(name) {
                continue;
            }
            let kept: BTreeSet<Pin> = pins
                .iter()
                .filter(|pin| !settings.ignores_part(&pin.part))
                .cloned()
                .collect();
            if !kept.is_empty() || pins.is_empty() {
                nets.insert(name.clone(), kept);
            }
        }
        NetTable { nets }
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
    /// The code (`PHOENIX-NETS-017` to `PHOENIX-NETS-023`).
    pub code: &'static str,
    /// The net: the first of the nets for the names that are alike.
    pub net: String,
    /// The other nets with a name alike, joined by a comma and a blank; for `PHOENIX-NETS-020`
    /// the number of pins, for `PHOENIX-NETS-021` the parts of the net and for `PHOENIX-NETS-023`
    /// the part.
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

/// Whether a net name is one that a program makes when the designer gives none: `N$12` (EAGLE),
/// `Net-(R1-Pad1)` and `unconnected-(U1-Pad3)` (KiCad, with or without the `/` of the root sheet)
/// and `$1N1234` (OrCAD), in any case.
pub fn generated_name(name: &str) -> bool {
    let lower = name.strip_prefix('/').unwrap_or(name).to_lowercase();
    if lower.starts_with("net-(") || lower.starts_with("unconnected-(") {
        return true;
    }
    let digits_only = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    if let Some(rest) = lower.strip_prefix("n$") {
        return digits_only(rest);
    }
    if let Some(rest) = lower.strip_prefix('$') {
        let count = rest.chars().take_while(char::is_ascii_digit).count();
        return count > 0 && rest[count..].strip_prefix('n').is_some_and(digits_only);
    }
    false
}

/// The nets that hold parts with two pins or more that are all on that one net, one finding for
/// each net in natural order, with the parts in natural order in `other`. A net that the settings
/// ignore is left out.
fn parts_on_one_net(table: &NetTable, settings: &NetsSettings) -> Vec<LintFinding> {
    let mut pin_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut nets_of: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (name, pins) in &table.nets {
        for pin in pins {
            *pin_counts.entry(pin.part.as_str()).or_default() += 1;
            nets_of
                .entry(pin.part.as_str())
                .or_default()
                .insert(name.as_str());
        }
    }
    let mut by_net: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (part, count) in &pin_counts {
        let only = nets_of.get(part).filter(|nets| nets.len() == 1);
        if let (true, Some(net)) = (*count >= 2, only.and_then(|nets| nets.iter().next())) {
            if !settings.ignores(net) {
                by_net.entry(net).or_default().push(part);
            }
        }
    }
    let mut ordered: Vec<(&str, Vec<&str>)> = by_net.into_iter().collect();
    ordered.sort_by(|a, b| natural_cmp(a.0, b.0));
    ordered
        .into_iter()
        .map(|(net, mut parts)| {
            parts.sort_by(|a, b| natural_cmp(a, b));
            LintFinding {
                code: codes::PART_ON_ONE_NET,
                net: net.to_owned(),
                other: Some(parts.join(", ")),
            }
        })
        .collect()
}

/// Looks for nets that are probably a mistake: a net with one pin (an unconnected input, or a
/// misspelt name that split a net in two), a net with no pin and nets whose names differ only in
/// case or in the separators (`GND` and `Gnd`, `VCC_3V3` and `VCC-3V3`). The nets that the settings
/// ignore are left out. Three more findings are asked for by the settings: a net with more pins
/// than `lint_max_pins` (the finding holds the number of pins in `other`), the parts whose pins are
/// all on one net (`lint_part_on_one_net`, one finding for each net, naming the parts in `other`),
/// the nets with a name that a program made (`lint_generated_names`) and the nets of two pins or
/// more that all belong to one part (`lint_one_part_nets`, naming the part in `other`). The pins
/// of the parts that `ignore_parts` names are not looked at.
pub fn lint(table: &NetTable, settings: &NetsSettings) -> LintResult {
    let table = &table.without_ignored(settings);
    let mut found: Vec<LintFinding> = Vec::new();
    let mut groups: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for (name, pins) in table.ordered() {
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
        if settings.lint_max_pins > 0 && pins.len() > settings.lint_max_pins {
            found.push(LintFinding {
                code: codes::NET_MANY_PINS,
                net: name.clone(),
                other: Some(pins.len().to_string()),
            });
        }
        if settings.lint_generated_names && generated_name(name) {
            found.push(LintFinding {
                code: codes::NET_NAME_GENERATED,
                net: name.clone(),
                other: None,
            });
        }
        if settings.lint_one_part_nets && pins.len() >= 2 {
            if let Some(first) = pins.iter().next() {
                if pins.iter().all(|pin| pin.part == first.part) {
                    found.push(LintFinding {
                        code: codes::NET_ONE_PART,
                        net: name.clone(),
                        other: Some(first.part.clone()),
                    });
                }
            }
        }
        groups.entry(alike_key(name)).or_default().push(name);
    }
    if settings.lint_part_on_one_net {
        found.extend(parts_on_one_net(table, settings));
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
// the summary, the filter and the pins of a part
// ---------------------------------------------------------------------------------------------

/// The size of a netlist: how many nets, parts and pins, how many nets have none, one or two pins,
/// and the nets with the most pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// The number of nets.
    pub nets: usize,
    /// The number of pins.
    pub pins: usize,
    /// The number of different parts.
    pub parts: usize,
    /// The nets with no pin.
    pub empty: usize,
    /// The nets with one pin.
    pub single_pin: usize,
    /// The nets with two pins.
    pub two_pin: usize,
    /// The nets with the most pins, as `(name, pins)`: the most pins first, and the natural order
    /// of the names among nets with as many.
    pub largest: Vec<(String, usize)>,
}

/// Counts a table and lists its `top` largest nets.
pub fn summary(table: &NetTable, top: usize) -> Summary {
    let mut parts: BTreeSet<&str> = BTreeSet::new();
    let mut sizes: Vec<(&String, usize)> = Vec::with_capacity(table.nets.len());
    for (name, pins) in &table.nets {
        parts.extend(pins.iter().map(|pin| pin.part.as_str()));
        sizes.push((name, pins.len()));
    }
    let with = |count: usize| sizes.iter().filter(|(_, n)| *n == count).count();
    let (empty, single_pin, two_pin) = (with(0), with(1), with(2));
    sizes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| natural_cmp(a.0, b.0)));
    Summary {
        nets: table.nets.len(),
        pins: table.pin_count(),
        parts: parts.len(),
        empty,
        single_pin,
        two_pin,
        largest: sizes
            .into_iter()
            .take(top)
            .map(|(name, n)| (name.clone(), n))
            .collect(),
    }
}

/// Whether a name matches a pattern: any case, and `*` stands for any run of characters (also an
/// empty one) anywhere in the pattern.
pub fn pattern_matches(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark): (Option<usize>, usize) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// The table with only the nets whose name matches one of the patterns, separated by commas (see
/// [`pattern_matches`]); an empty list of patterns keeps no net.
pub fn filter_nets(table: &NetTable, patterns: &str) -> NetTable {
    let patterns: Vec<&str> = patterns
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    NetTable {
        nets: table
            .nets
            .iter()
            .filter(|(name, _)| patterns.iter().any(|p| pattern_matches(p, name)))
            .map(|(name, pins)| (name.clone(), pins.clone()))
            .collect(),
    }
}

/// The pins of one part (its reference exactly as written, case included) with the net each is
/// on and how many pins that net has, in natural order of the pins.
pub fn pins_of_part(table: &NetTable, part: &str) -> Vec<(Pin, String, usize)> {
    let mut found: Vec<(Pin, String, usize)> = Vec::new();
    for (name, pins) in &table.nets {
        for pin in pins.iter().filter(|pin| pin.part == part) {
            found.push((pin.clone(), name.clone(), pins.len()));
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
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

    fn findings(
        text: &str,
        settings: &NetsSettings,
    ) -> Vec<(&'static str, String, Option<String>)> {
        lint(&table(text), settings)
            .findings
            .into_iter()
            .map(|f| (f.code, f.net, f.other))
            .collect()
    }

    #[test]
    fn a_net_with_more_pins_than_the_limit_is_found_with_its_pin_count() {
        let text = "NET BIG R1.1 R2.1 R3.1 R4.1\nNET SMALL R1.2 R2.2\n";
        assert!(findings(text, &settings()).is_empty());
        let s = NetsSettings::parse("lint_max_pins = 3").unwrap();
        assert_eq!(
            findings(text, &s),
            vec![(codes::NET_MANY_PINS, "BIG".to_owned(), Some("4".to_owned()))]
        );
        // a net at the limit is allowed, and an ignored net is not judged
        let s = NetsSettings::parse("lint_max_pins = 4").unwrap();
        assert!(findings(text, &s).is_empty());
        let s = NetsSettings::parse("lint_max_pins = 1\nignore_nets = big").unwrap();
        assert_eq!(findings(text, &s).len(), 1);
    }

    #[test]
    fn the_parts_with_all_their_pins_on_one_net_are_found_for_each_net() {
        let text =
            "NET GND R1.2 R2.2 R3.1 R3.2 U1.1\nNET SIG R1.1 R2.1 U1.2 U2.1 U2.2\nNET LONE J1.1\n";
        assert!(findings(text, &settings())
            .iter()
            .all(|f| f.0 != codes::PART_ON_ONE_NET));
        let s = NetsSettings::parse("lint_part_on_one_net = true").unwrap();
        let found: Vec<_> = findings(text, &s)
            .into_iter()
            .filter(|f| f.0 == codes::PART_ON_ONE_NET)
            .collect();
        assert_eq!(
            found,
            vec![
                (
                    codes::PART_ON_ONE_NET,
                    "GND".to_owned(),
                    Some("R3".to_owned())
                ),
                (
                    codes::PART_ON_ONE_NET,
                    "SIG".to_owned(),
                    Some("U2".to_owned())
                ),
            ]
        );
        // a part with one pin (J1) is not a part on one net, and an ignored net is left out
        let s = NetsSettings::parse("lint_part_on_one_net = true\nignore_nets = gnd").unwrap();
        let found = findings(text, &s);
        assert!(found.iter().all(|f| f.1 != "GND"));
        assert!(found.iter().any(|f| f.1 == "SIG"));
    }

    #[test]
    fn two_parts_on_the_same_net_are_one_finding_in_natural_order() {
        let text = "NET GND C10.1 C10.2 C2.1 C2.2 R1.1\nNET A R1.2 C3.1\nNET B C3.2\n";
        let s = NetsSettings::parse("lint_part_on_one_net = true").unwrap();
        let found: Vec<_> = findings(text, &s)
            .into_iter()
            .filter(|f| f.0 == codes::PART_ON_ONE_NET)
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2.as_deref(), Some("C2, C10"));
    }

    #[test]
    fn the_names_that_a_program_makes_are_known_and_a_name_of_the_designer_is_not() {
        for name in [
            "N$1",
            "n$123",
            "Net-(R1-Pad1)",
            "/Net-(U1-Pad2)",
            "unconnected-(U3-Pad4)",
            "$1N1234",
            "$25N7",
        ] {
            assert!(generated_name(name), "{name}");
        }
        for name in [
            "", "GND", "N$", "N$1A", "N1", "Netted", "$N12", "$1N", "$1X5", "SIG_N$1", "VCC",
        ] {
            assert!(!generated_name(name), "{name}");
        }
        let text = "NET N$7 R1.1 R2.1\nNET GND R1.2 R2.2\n";
        assert!(findings(text, &settings()).is_empty());
        let s = NetsSettings::parse("lint_generated_names = true").unwrap();
        assert_eq!(
            findings(text, &s),
            vec![(codes::NET_NAME_GENERATED, "N$7".to_owned(), None)]
        );
    }

    #[test]
    fn the_settings_of_the_lint_are_read_and_refused_out_of_their_range() {
        let s = NetsSettings::default();
        assert_eq!(s.lint_max_pins, 0);
        assert!(!s.lint_part_on_one_net && !s.lint_generated_names);
        let s = NetsSettings::parse(
            "lint_max_pins = 250\nlint_part_on_one_net = true\nlint_generated_names = true",
        )
        .unwrap();
        assert_eq!(s.lint_max_pins, 250);
        assert!(s.lint_part_on_one_net && s.lint_generated_names);
        for bad in [
            "lint_max_pins = -1",
            "lint_max_pins = 10000001",
            "lint_max_pins = many",
            "lint_part_on_one_net = 1",
            "lint_generated_names = yes",
        ] {
            let e = NetsSettings::parse(bad).unwrap_err();
            assert_eq!((e.code, e.line), (codes::SETTINGS_INVALID, 1), "{bad}");
        }
    }

    #[test]
    fn the_summary_counts_the_nets_by_size_and_lists_the_largest_first() {
        let t = table("NET A R1.1 R2.1 R3.1\nNET B R1.2 R2.2\nNET C R3.2 U1.1\nNET D U1.2\nNET E\nNET F R4.1 R4.2 R4.3\n");
        let s = summary(&t, 3);
        assert_eq!((s.nets, s.pins, s.parts), (6, 11, 5));
        assert_eq!((s.empty, s.single_pin, s.two_pin), (1, 1, 2));
        assert_eq!(
            s.largest,
            vec![
                ("A".to_owned(), 3),
                ("F".to_owned(), 3),
                ("B".to_owned(), 2)
            ]
        );
        assert_eq!(summary(&t, 0).largest, Vec::new());
        assert_eq!(summary(&t, 100).largest.len(), 6);
        let none = summary(&NetTable::default(), 5);
        assert_eq!((none.nets, none.pins, none.parts), (0, 0, 0));
        assert!(none.largest.is_empty());
    }

    #[test]
    fn a_pattern_matches_in_any_case_with_stars_anywhere() {
        for (pattern, name, expected) in [
            ("GND", "gnd", true),
            ("GND", "GND1", false),
            ("gnd*", "GND_ANALOG", true),
            ("*3v3", "VCC_3V3", true),
            ("*3v3", "VCC_3V3X", false),
            ("v*_3v3", "VCC_3V3", true),
            ("v*_3v3", "V_3V3", true),
            ("*", "", true),
            ("*", "anything", true),
            ("", "", true),
            ("", "a", false),
            ("a*b*c", "aXbYc", true),
            ("a*b*c", "aXbY", false),
            ("**x", "x", true),
            ("a*a*a", "aaa", true),
            ("a*a*a", "aa", false),
        ] {
            assert_eq!(pattern_matches(pattern, name), expected, "{pattern} {name}");
        }
    }

    #[test]
    fn the_filter_keeps_the_nets_that_match_one_of_the_patterns() {
        let t = table("NET GND R1.1\nNET GND_A R2.1\nNET VCC_3V3 R3.1\nNET SIG R4.1\n");
        let names = |patterns: &str| -> Vec<String> {
            filter_nets(&t, patterns).nets.keys().cloned().collect()
        };
        assert_eq!(names("gnd*"), vec!["GND", "GND_A"]);
        assert_eq!(names("GND, *3v3"), vec!["GND", "VCC_3V3"]);
        assert_eq!(names("sig"), vec!["SIG"]);
        assert!(names("").is_empty());
        assert!(names(" , ").is_empty());
        assert!(names("nothing*").is_empty());
        assert_eq!(names("*").len(), 4);
        // the pins of a kept net are all there
        assert_eq!(filter_nets(&t, "gnd").pin_count(), 1);
    }

    #[test]
    fn the_pins_of_a_part_come_with_their_net_in_natural_order() {
        let t = table("NET GND U1.1 U1.10 U1.2 R1.2 R2.2\nNET SIG U1.3 R1.1 R2.1\nNET U11 U11.1\n");
        let pins = pins_of_part(&t, "U1");
        let shown: Vec<String> = pins
            .iter()
            .map(|(p, n, c)| format!("{p} {n} {c}"))
            .collect();
        assert_eq!(
            shown,
            vec!["U1.1 GND 5", "U1.2 GND 5", "U1.3 SIG 3", "U1.10 GND 5"]
        );
        assert!(pins_of_part(&t, "u1").is_empty());
        assert!(pins_of_part(&t, "X9").is_empty());
        assert_eq!(pins_of_part(&t, "U11").len(), 1);
    }

    #[test]
    fn a_net_that_joins_pins_of_one_part_only_is_found_when_asked() {
        let text = "NET LOOP U1.1 U1.2\nNET OK U1.3 R1.1\nNET ONE U2.1\nNET VOID\nNET BIG U3.1 U3.2 U3.3\n";
        let found = |s: &NetsSettings| -> Vec<(String, Option<String>)> {
            findings(text, s)
                .into_iter()
                .filter(|f| f.0 == codes::NET_ONE_PART)
                .map(|f| (f.1, f.2))
                .collect()
        };
        assert!(found(&settings()).is_empty());
        let s = NetsSettings::parse("lint_one_part_nets = true").unwrap();
        assert_eq!(
            found(&s),
            vec![
                ("BIG".to_owned(), Some("U3".to_owned())),
                ("LOOP".to_owned(), Some("U1".to_owned())),
            ]
        );
        // a net of one pin is not a loop of a part, and an ignored net is not judged
        let s = NetsSettings::parse("lint_one_part_nets = true\nignore_nets = big").unwrap();
        assert_eq!(found(&s).len(), 1);
        assert!(NetsSettings::parse("lint_one_part_nets = maybe").is_err());
    }

    #[test]
    fn the_parts_that_are_ignored_leave_the_lint_with_their_pins() {
        let text = "NET SIG R1.1 TP1.1\nNET GND R1.2 R2.1\nNET TPONLY TP2.1 TP3.1\nNET VOID\n";
        let s = settings();
        let none = findings(text, &s);
        assert_eq!(none.len(), 1, "{none:?}");
        assert_eq!(none[0].0, codes::NET_EMPTY);
        let s = NetsSettings::parse("ignore_parts = tp*").unwrap();
        let found = findings(text, &s);
        // SIG has one pin left, TPONLY has none left and is not judged, VOID had none to begin with
        assert_eq!(
            found,
            vec![
                (codes::NET_SINGLE_PIN, "SIG".to_owned(), None),
                (codes::NET_EMPTY, "VOID".to_owned(), None),
            ]
        );
    }

    #[test]
    fn a_comparison_leaves_out_the_parts_that_are_ignored_on_both_sides() {
        let file = table("NET SIG R1.1\nNET GND R1.2\n");
        let board = table("NET SIG R1.1 TP1.1\nNET GND R1.2\nNET TPNET TP2.1 FID1.1\n");
        let plain = compare(&file, &board, &settings());
        assert!(plain.total >= 2, "{:?}", plain.differences);
        let s = NetsSettings::parse("ignore_parts = TP*, fid*").unwrap();
        let same = compare(&file, &board, &s);
        assert_eq!((same.total, same.differences.len()), (0, 0));
        // the other way round
        assert_eq!(compare(&board, &file, &s).total, 0);
        // a part that is not ignored still shows
        let s = NetsSettings::parse("ignore_parts = fid*").unwrap();
        let partly = compare(&file, &board, &s);
        assert!(
            partly.total >= 1 && partly.total <= plain.total,
            "{:?}",
            partly.differences
        );
    }

    #[test]
    fn the_names_of_the_parts_that_are_ignored_are_read_and_matched_in_any_case() {
        assert!(NetsSettings::default().ignore_parts.is_empty());
        assert!(!NetsSettings::default().ignores_part("TP1"));
        let s = NetsSettings::parse("ignore_parts = TP*, fid?, *mount , ").unwrap();
        assert_eq!(s.ignore_parts, vec!["TP*", "fid?", "*mount"]);
        assert!(s.ignores_part("tp12"));
        assert!(s.ignores_part("TP"));
        assert!(s.ignores_part("hole_mount"));
        assert!(s.ignores_part("FID?"));
        assert!(!s.ignores_part("FID1"));
        assert!(!s.ignores_part("R1"));
        assert!(NetsSettings::parse("ignore_parts =")
            .unwrap()
            .ignore_parts
            .is_empty());
    }

    #[test]
    fn every_setting_is_in_the_shipped_file_and_in_the_document() {
        let shipped = include_str!("../../../config/nets.cfg");
        let doc = include_str!("../../../docs/NETS.md");
        for key in [
            "max_input_bytes",
            "max_nets",
            "max_pins_per_net",
            "max_pins",
            "max_name_len",
            "max_differences",
            "ignore_nets",
            "ignore_parts",
            "csv_guard_formulas",
            "lint_max_pins",
            "lint_part_on_one_net",
            "lint_generated_names",
            "lint_one_part_nets",
        ] {
            assert!(
                shipped.lines().any(|l| l.starts_with(&format!("{key} ="))),
                "{key} is not in config/nets.cfg"
            );
            assert!(
                doc.contains(&format!("`{key}`")),
                "{key} is not in docs/NETS.md"
            );
            // the key is one that the reader knows: it is never refused as unknown
            let refused = NetsSettings::parse(&format!("{key} = x")).err();
            assert!(
                refused.is_none_or(|e| !e.detail.contains("unknown key")),
                "{key}"
            );
        }
    }

    /// A linear congruential generator with a fixed seed: the same cases on every run.
    fn generator(seed: u64) -> impl FnMut(u64) -> u64 {
        let mut state = seed;
        move |n: u64| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) % n
        }
    }

    fn random_table(next: &mut impl FnMut(u64) -> u64) -> NetTable {
        let mut t = NetTable::default();
        let mut used = BTreeSet::new();
        for n in 0..=next(10) {
            let mut pins = BTreeSet::new();
            for _ in 0..next(12) {
                let pin = Pin {
                    part: format!(
                        "{}{}",
                        ["R", "C", "U", "J", "TP"][next(5) as usize],
                        next(30)
                    ),
                    pad: format!("{}", next(9)),
                };
                if used.insert(pin.clone()) {
                    pins.insert(pin);
                }
            }
            t.nets.insert(
                format!("{}{n}", ["NET", "GND", "N$", "VCC_"][next(4) as usize]),
                pins,
            );
        }
        t
    }

    #[test]
    fn a_table_compared_with_itself_has_no_difference_whatever_the_table() {
        let mut next = generator(0x9E37_79B9_7F4A_7C15);
        for _ in 0..60 {
            let t = random_table(&mut next);
            let same = compare(&t, &t, &settings());
            assert_eq!(
                (same.total, same.differences.len()),
                (0, 0),
                "{}",
                render(&t)
            );
        }
    }

    #[test]
    fn the_lint_and_the_summary_give_the_same_answer_on_the_same_table_every_time() {
        let mut next = generator(0xD1B5_4A32_D192_ED03);
        let s = NetsSettings::parse(
            "lint_max_pins = 3\nlint_part_on_one_net = true\nlint_generated_names = true",
        )
        .unwrap();
        for _ in 0..60 {
            let t = random_table(&mut next);
            let first = lint(&t, &s);
            assert_eq!(first, lint(&table(&render(&t)), &s));
            assert!(first.findings.len() <= first.total);
            assert_eq!(summary(&t, 4), summary(&table(&render(&t)), 4));
            let sum = summary(&t, 4);
            assert!(sum.largest.windows(2).all(|w| w[0].1 >= w[1].1));
            assert_eq!(
                sum.nets,
                sum.empty
                    + sum.single_pin
                    + sum.two_pin
                    + t.nets.values().filter(|p| p.len() > 2).count()
            );
        }
    }

    #[test]
    fn the_table_of_the_pins_of_a_table_is_that_table_without_notes() {
        let mut next = generator(0x2545_F491_4F6C_DD1D);
        for _ in 0..60 {
            let mut t = random_table(&mut next);
            t.nets.retain(|_, pins| !pins.is_empty());
            let pins: Vec<(String, String, String)> = t
                .nets
                .iter()
                .flat_map(|(n, ps)| {
                    ps.iter()
                        .map(move |p| (n.clone(), p.part.clone(), p.pad.clone()))
                })
                .collect();
            let (again, notes) = table_of_pins(pins);
            assert!(notes.is_empty(), "{notes:?}");
            assert_eq!(again, t);
        }
    }

    #[test]
    fn a_cell_of_a_csv_file_never_starts_like_a_formula_when_the_guard_is_on() {
        let mut next = generator(0x0123_4567_89AB_CDEF);
        let alphabet = [
            '=', '+', '-', '@', '\t', '\r', ',', '"', '\n', 'a', 'Z', '5', ' ', '_',
        ];
        for _ in 0..300 {
            let value: String = (0..next(6))
                .map(|_| alphabet[next(alphabet.len() as u64) as usize])
                .collect();
            let guarded = csv_field(&value, true);
            assert!(
                !guarded.starts_with(['=', '+', '-', '@', '\t', '\r']),
                "{value:?} -> {guarded:?}"
            );
            // without the guard the cell is only quoted when it has to be
            let plain = csv_field(&value, false);
            assert_eq!(
                plain.starts_with('"'),
                value.contains([',', '"', '\n', '\r']),
                "{value:?}"
            );
        }
    }

    #[test]
    fn the_natural_order_is_a_total_order_on_texts_made_by_a_fixed_generator() {
        let mut next = generator(0x3C6E_F372_FE94_F82B);
        let alphabet = ['R', 'r', '0', '1', '9', '_', '.', 'a'];
        let texts: Vec<String> = (0..40)
            .map(|_| (0..next(6)).map(|_| alphabet[next(8) as usize]).collect())
            .collect();
        for a in &texts {
            assert_eq!(natural_cmp(a, a), Ordering::Equal);
            for b in &texts {
                assert_eq!(
                    natural_cmp(a, b),
                    natural_cmp(b, a).reverse(),
                    "{a:?} {b:?}"
                );
                assert_eq!(natural_cmp(a, b) == Ordering::Equal, a == b, "{a:?} {b:?}");
                for c in &texts {
                    if natural_cmp(a, b) != Ordering::Greater
                        && natural_cmp(b, c) != Ordering::Greater
                    {
                        assert_ne!(natural_cmp(a, c), Ordering::Greater, "{a:?} {b:?} {c:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_pattern_made_of_a_prefix_a_star_and_a_suffix_matches_what_it_should() {
        let mut next = generator(0xA409_3822_299F_31D0);
        let alphabet = ['A', 'b', 'C', '3', '_', 'v'];
        let word = |next: &mut dyn FnMut(u64) -> u64, n: u64| -> String {
            (0..n).map(|_| alphabet[next(6) as usize]).collect()
        };
        for _ in 0..200 {
            let (head, middle, tail) = (word(&mut next, 3), word(&mut next, 4), word(&mut next, 3));
            let name = format!("{head}{middle}{tail}");
            assert!(pattern_matches(&format!("{head}*{tail}"), &name), "{name}");
            assert!(
                pattern_matches(&format!("{}*", head.to_uppercase()), &name),
                "{name}"
            );
            assert!(pattern_matches("*", &name));
            assert!(pattern_matches(&name.to_lowercase(), &name));
            assert!(!pattern_matches(&format!("{name}x"), &name));
        }
    }
}
