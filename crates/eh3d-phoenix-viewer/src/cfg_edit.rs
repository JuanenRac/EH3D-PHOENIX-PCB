// SPDX-License-Identifier: GPL-3.0-or-later
//! The editor of the settings files (`config/*.cfg` and the capability profiles of the board
//! makers), as state without a screen. A file is a list of `key = value` lines with comments; the
//! editor lists its settings with their value, their default, the range the program allows and
//! the comment above them, changes a value in the text without touching the rest of the file, and
//! writes the result without replacing anything that was not ordered to be replaced.
//!
//! Whether a text is valid is never decided here: every kind of file is handed to the very reader
//! that the command or the window of that file uses (see [`crate::cfg_kinds`]), so the editor
//! cannot accept what the program then refuses or the other way round. The range of a number is
//! found with the same readers, by offering them values and seeing which they accept, on the file
//! of the defaults of the repository.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cfg_kinds::{make_issue, CfgKind, Issue, KINDS};
use crate::codes::{
    ViewError, CHECK_FILE_UNUSABLE, EDIT_FAILED, FILE_UNUSABLE, SAVE_FAILED, SAVE_REFUSED,
};
use crate::session::same_file;
use crate::settings::ViewerSettings;

/// One setting of a text: a `key = value` line with the comment that stands above it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The line, counted from 1 as the readers count it.
    pub line: usize,
    /// The key.
    pub key: String,
    /// The value, without the spaces around it.
    pub value: String,
    /// The comment lines right above it (up to the blank line before them), joined with spaces.
    /// Settings that follow one another share the comment above the first.
    pub help: String,
}

/// The settings of a text. Only the shape of the lines is read here (a line with `=` that does
/// not start with `#`); whether a key or a value is good is the business of the readers.
pub fn entries(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut block: Vec<String> = Vec::new();
    let mut after_key = false;
    for (index, raw) in text.lines().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            block.clear();
            after_key = false;
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix('#') {
            if after_key {
                block.clear();
                after_key = false;
            }
            block.push(comment.trim().to_owned());
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            out.push(Entry {
                line: index + 1,
                key: key.trim().to_owned(),
                value: value.trim().to_owned(),
                help: block.join(" "),
            });
            after_key = true;
        } else {
            block.clear();
            after_key = false;
        }
    }
    out
}

/// The text with the value of the setting on a line replaced and everything else left as it was
/// (the spaces before the key, the line endings, the comments). `None` for a line that is not a
/// setting or a value that would span lines.
pub fn replace_value(text: &str, line: usize, value: &str) -> Option<String> {
    if line == 0 || value.contains(['\n', '\r']) {
        return None;
    }
    let mut out = String::with_capacity(text.len() + value.len());
    let mut found = false;
    for (index, piece) in text.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            if piece.trim_start().starts_with('#') {
                return None;
            }
            let equals = piece.find('=')?;
            let ending = &piece[piece.trim_end_matches(['\r', '\n']).len()..];
            out.push_str(&piece[..=equals]);
            out.push(' ');
            out.push_str(value);
            out.push_str(ending);
            found = true;
        } else {
            out.push_str(piece);
        }
    }
    found.then_some(out)
}

/// True when `message` names `key` as a word of its own (not as a part of a longer name).
fn mentions(message: &str, key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '.';
    let mut from = 0;
    while let Some(at) = message[from..].find(key) {
        let start = from + at;
        let end = start + key.len();
        let before = message[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !word(c));
        let after = message[end..].chars().next().is_none_or(|c| !word(c));
        if before && after {
            return true;
        }
        from = end;
    }
    false
}

/// A reader that checks the ranges after it has read every line does not name the line of a value
/// it refuses, but it names the setting in its message; the line of the first setting named there
/// is taken as the place, so the table can mark it.
fn locate(text: &str, mut issue: Issue) -> Issue {
    if issue.line.is_none() {
        let found = entries(text)
            .into_iter()
            .find(|e| mentions(&issue.detail, &e.key))
            .map(|e| e.line);
        issue.line = found;
    }
    issue
}

/// What sort of value a setting holds, taken from how its default is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// `true` or `false`.
    Flag,
    /// A whole number.
    Whole,
    /// A number with a fraction.
    Number,
    /// A word, a name, a colour, a list: anything else.
    Word,
}

/// The sort of a value.
pub fn kind_of(value: &str) -> ValueKind {
    if value == "true" || value == "false" {
        return ValueKind::Flag;
    }
    let starts_like_number = value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'));
    if !starts_like_number {
        return ValueKind::Word;
    }
    if value.parse::<i64>().is_ok() {
        ValueKind::Whole
    } else if value.parse::<f64>().is_ok_and(f64::is_finite) {
        ValueKind::Number
    } else {
        ValueKind::Word
    }
}

/// What the reader of a file allows for a setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Range {
    /// `true` or `false`.
    Flag,
    /// A number between two limits; a limit that is `None` was not found within the span that was
    /// tried. `gaps` is true when a value between the limits is refused, so the limits are not
    /// the whole story (a setting that takes only 8 or 16, for instance).
    Numbers {
        /// The smallest value accepted.
        low: Option<String>,
        /// The largest value accepted.
        high: Option<String>,
        /// Whether some value between the limits is refused.
        gaps: bool,
    },
    /// Not measured: a word, a name or a list, whose choices are in the comment of the file.
    NotProbed,
}

impl Range {
    /// The range as a short text that needs no translation: `low .. high` with `…` for a limit
    /// that was not found, `true | false`, or `-` when it was not measured.
    pub fn describe(&self) -> String {
        match self {
            Range::Flag => "true | false".to_owned(),
            Range::Numbers { low, high, .. } => format!(
                "{} .. {}",
                low.as_deref().unwrap_or("…"),
                high.as_deref().unwrap_or("…")
            ),
            Range::NotProbed => "-".to_owned(),
        }
    }

    /// True when the range has gaps.
    pub fn has_gaps(&self) -> bool {
        matches!(self, Range::Numbers { gaps: true, .. })
    }
}

fn number_text(value: f64, whole: bool) -> String {
    if whole {
        format!("{}", value as i64)
    } else {
        let tidy = format!("{value:.6e}").parse::<f64>().unwrap_or(value);
        format!("{tidy}")
    }
}

/// The value nearest to `far` that `accepts` takes, found by halving between `inside` (accepted)
/// and `far`; `None` when `far` itself is accepted (no limit within the span).
fn find_edge(
    accepts: &dyn Fn(f64) -> bool,
    inside: f64,
    far: f64,
    whole: bool,
    steps: usize,
) -> Option<f64> {
    if accepts(far) {
        return None;
    }
    let (mut good, mut bad) = (inside, far);
    for _ in 0..steps {
        let mut mid = good + (bad - good) / 2.0;
        if whole {
            mid = if bad > good { mid.floor() } else { mid.ceil() };
        }
        if mid == good || mid == bad {
            break;
        }
        if accepts(mid) {
            good = mid;
        } else {
            bad = mid;
        }
    }
    Some(good)
}

/// What the reader of `kind` allows for the setting `entry` of the text `defaults`, found by
/// offering it numbers: it searches up and down from the value of the entry, by halving, at most
/// `steps` times on each side and no farther than `span`. A word is not measured.
pub fn probe(kind: &CfgKind, defaults: &str, entry: &Entry, steps: usize, span: f64) -> Range {
    let value_kind = kind_of(&entry.value);
    match value_kind {
        ValueKind::Flag => return Range::Flag,
        ValueKind::Word => return Range::NotProbed,
        ValueKind::Whole | ValueKind::Number => {}
    }
    let whole = value_kind == ValueKind::Whole;
    let accepts_text = |value: &str| {
        replace_value(defaults, entry.line, value).is_some_and(|t| kind.validate(&t).is_none())
    };
    let accepts = |v: f64| accepts_text(&number_text(v, whole));
    if !accepts_text(&entry.value) {
        return Range::NotProbed;
    }
    let Ok(base) = entry.value.parse::<f64>() else {
        return Range::NotProbed;
    };
    // the span reaches at least twice as far as the value itself
    let span = span.max(base.abs() * 2.0);
    let low = find_edge(&accepts, base, -span, whole, steps);
    let high = find_edge(&accepts, base, span, whole, steps);
    let below = |edge: f64| {
        if !whole && edge.abs() < 1e-9 {
            if accepts(0.0) {
                "0".to_owned()
            } else {
                ">0".to_owned()
            }
        } else {
            number_text(edge, whole)
        }
    };
    let above = |edge: f64| {
        if !whole && edge.abs() < 1e-9 {
            if accepts(0.0) {
                "0".to_owned()
            } else {
                "<0".to_owned()
            }
        } else {
            number_text(edge, whole)
        }
    };
    // a setting that takes only a few values (2 or 4, 8 or 16) is not found by halving, which
    // converges on the value it started from: whole numbers near zero and a few round larger
    // ones are offered as well, and the limits are widened to the smallest and the largest that
    // were accepted
    let (mut low, mut high) = (low, high);
    let mut points: Vec<f64> = Vec::new();
    if whole {
        points.extend((-8..=64).map(f64::from));
        let mut round = 100.0;
        while round <= span {
            points.push(round);
            round *= 10.0;
        }
        let mut power = 128.0;
        while power <= span {
            points.push(power);
            power *= 2.0;
        }
        let taken: Vec<f64> = points.iter().copied().filter(|p| accepts(*p)).collect();
        for p in taken {
            if low.is_some_and(|l| p < l) {
                low = Some(p);
            }
            if high.is_some_and(|h| p > h) {
                high = Some(p);
            }
        }
    }
    // a refused value between the limits means the limits are not the whole story
    let mut gaps = false;
    if low.is_some() || high.is_some() {
        let from = low.unwrap_or(base);
        let to = high.unwrap_or(base);
        for fraction in [0.25, 0.5, 0.75] {
            let mut point = from + (to - from) * fraction;
            if whole {
                point = point.round();
            }
            if !accepts(point) {
                gaps = true;
            }
        }
        for p in points.iter().filter(|p| **p >= from && **p <= to) {
            if !accepts(*p) {
                gaps = true;
            }
        }
    }
    Range::Numbers {
        low: low.map(below),
        high: high.map(above),
        gaps,
    }
}

/// What the reader of `kind` allows for each setting of its file of defaults, by key (the first
/// line of a key that is written twice).
pub fn probe_all(kind: &CfgKind, steps: usize, span: f64) -> BTreeMap<String, Range> {
    let mut out = BTreeMap::new();
    for entry in entries(kind.defaults) {
        if let std::collections::btree_map::Entry::Vacant(slot) = out.entry(entry.key.clone()) {
            slot.insert(probe(kind, kind.defaults, &entry, steps, span));
        }
    }
    out
}

/// A setting as the table of the window shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The line of the text.
    pub line: usize,
    /// The key.
    pub key: String,
    /// The value now.
    pub value: String,
    /// The value of the file of defaults, when it has the key.
    pub default: Option<String>,
    /// True when the value is not the default (or the default file has no such key).
    pub changed: bool,
    /// The sort of the value.
    pub kind: ValueKind,
    /// The comment above the setting.
    pub help: String,
    /// What the reader allows, when it was measured.
    pub range: Option<Range>,
    /// True when the reader refused the text at this line.
    pub refused: bool,
}

/// The text of a settings file being edited, with the kind of file it is and where it came from.
pub struct CfgDoc {
    kind: &'static CfgKind,
    text: String,
    original: String,
    input: Option<PathBuf>,
}

fn unusable(path: &Path, why: impl std::fmt::Display) -> ViewError {
    ViewError::new(FILE_UNUSABLE, format!("{}: {why}", path.display()))
}

impl CfgDoc {
    /// The file of defaults of a kind, as a document that came from no file.
    pub fn defaults(kind: &'static CfgKind) -> CfgDoc {
        CfgDoc {
            kind,
            text: kind.defaults.to_owned(),
            original: kind.defaults.to_owned(),
            input: None,
        }
    }

    /// Reads a file of a kind. A file larger than `max_bytes` or not in UTF-8 is refused.
    pub fn open(kind: &'static CfgKind, path: &Path, max_bytes: u64) -> Result<CfgDoc, ViewError> {
        let meta = fs::metadata(path).map_err(|e| unusable(path, e))?;
        if meta.len() > max_bytes {
            return Err(unusable(
                path,
                format!("{} bytes, the limit is {max_bytes}", meta.len()),
            ));
        }
        let bytes = fs::read(path).map_err(|e| unusable(path, e))?;
        let text = String::from_utf8(bytes).map_err(|_| unusable(path, "not UTF-8 text"))?;
        Ok(CfgDoc {
            kind,
            original: text.clone(),
            text,
            input: Some(path.to_path_buf()),
        })
    }

    /// The kind of file.
    pub fn kind(&self) -> &'static CfgKind {
        self.kind
    }

    /// The text as it is now.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The file that was read, when the document came from one.
    pub fn input(&self) -> Option<&Path> {
        self.input.as_deref()
    }

    /// True when the text is not the one that was read.
    pub fn is_modified(&self) -> bool {
        self.text != self.original
    }

    /// What the reader of the program refuses in the text as it is now.
    pub fn issue(&self) -> Option<Issue> {
        self.kind
            .validate(&self.text)
            .map(|issue| locate(&self.text, issue))
    }

    /// What the reader would say if the value on a line were another, without changing anything.
    pub fn issue_with(&self, line: usize, value: &str) -> Option<Issue> {
        match replace_value(&self.text, line, value) {
            Some(text) => self.kind.validate(&text).map(|issue| locate(&text, issue)),
            None => Some(make_issue("", format!("line {line}: not a setting"))),
        }
    }

    /// The settings of the text, in order, with their defaults and, when `ranges` has them, what
    /// the reader allows.
    pub fn rows(&self, ranges: &BTreeMap<String, Range>) -> Vec<Row> {
        let defaults = entries(self.kind.defaults);
        let refused_line = self.issue().and_then(|i| i.line);
        entries(&self.text)
            .into_iter()
            .map(|entry| {
                let default = defaults
                    .iter()
                    .find(|d| d.key == entry.key)
                    .map(|d| d.value.clone());
                let default_help = defaults
                    .iter()
                    .find(|d| d.key == entry.key)
                    .map(|d| d.help.clone())
                    .unwrap_or_default();
                Row {
                    line: entry.line,
                    changed: default.as_deref() != Some(entry.value.as_str()),
                    kind: kind_of(default.as_deref().unwrap_or(&entry.value)),
                    help: if entry.help.is_empty() {
                        default_help
                    } else {
                        entry.help
                    },
                    range: ranges.get(&entry.key).cloned(),
                    refused: refused_line == Some(entry.line),
                    key: entry.key,
                    value: entry.value,
                    default,
                }
            })
            .collect()
    }

    /// How many settings differ from the defaults.
    pub fn changed_count(&self, ranges: &BTreeMap<String, Range>) -> usize {
        self.rows(ranges).iter().filter(|r| r.changed).count()
    }

    /// The keys of the file of defaults that the text does not have.
    pub fn missing(&self) -> Vec<String> {
        let present: Vec<String> = entries(&self.text).into_iter().map(|e| e.key).collect();
        let mut out: Vec<String> = Vec::new();
        for entry in entries(self.kind.defaults) {
            if !present.contains(&entry.key) && !out.contains(&entry.key) {
                out.push(entry.key);
            }
        }
        out
    }

    /// Changes the value of the setting on a line. The text is changed even when the reader
    /// refuses it, so that what is typed stays on the screen with the reason; [`CfgDoc::issue`]
    /// says whether it can be saved.
    pub fn set_value(&mut self, line: usize, value: &str) -> Result<(), ViewError> {
        match replace_value(&self.text, line, value) {
            Some(text) => {
                self.text = text;
                Ok(())
            }
            None => Err(ViewError::new(
                EDIT_FAILED,
                format!("line {line} is not a setting or the value spans lines"),
            )),
        }
    }

    /// Puts the value of the default back on the setting of a line.
    pub fn reset_value(&mut self, line: usize) -> Result<(), ViewError> {
        let key = entries(&self.text)
            .into_iter()
            .find(|e| e.line == line)
            .map(|e| e.key);
        let default = key.and_then(|key| {
            entries(self.kind.defaults)
                .into_iter()
                .find(|d| d.key == key)
                .map(|d| d.value)
        });
        match default {
            Some(value) => self.set_value(line, &value),
            None => Err(ViewError::new(
                EDIT_FAILED,
                format!("line {line} has no default"),
            )),
        }
    }

    /// Adds a setting of the defaults that the text lacks, at the end, with its default value.
    pub fn add_missing(&mut self, key: &str) -> Result<(), ViewError> {
        let Some(default) = entries(self.kind.defaults)
            .into_iter()
            .find(|d| d.key == key)
        else {
            return Err(ViewError::new(
                EDIT_FAILED,
                format!("{key} is not a setting of this file"),
            ));
        };
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        self.text.push_str(&format!("{key} = {}\n", default.value));
        Ok(())
    }

    /// Takes back every change since the file was read.
    pub fn revert(&mut self) {
        self.text = self.original.clone();
    }

    /// Writes the text under a name. A text that the reader of the program refuses is not
    /// written. See [`write_atomic`] for what is never replaced.
    pub fn save_as(
        &self,
        target: &Path,
        overwrite: bool,
        backup_suffix: &str,
    ) -> Result<PathBuf, ViewError> {
        if let Some(issue) = self.issue() {
            return Err(ViewError::new(
                CHECK_FILE_UNUSABLE,
                format!("{}: {}", issue.code, issue.detail),
            ));
        }
        write_atomic(
            target,
            self.text.as_bytes(),
            self.input.as_deref(),
            overwrite,
            backup_suffix,
        )?;
        Ok(target.to_path_buf())
    }
}

/// Writes a file all at once: the bytes go to a file next to the target that is created new, and
/// that file is renamed to the target, so a reader never sees half a file. The file that was
/// opened and any file that exists are replaced only when `overwrite` says so (the order of the
/// person), and then a copy of the old file is kept with `backup_suffix` added to its name.
pub fn write_atomic(
    target: &Path,
    bytes: &[u8],
    input: Option<&Path>,
    overwrite: bool,
    backup_suffix: &str,
) -> Result<(), ViewError> {
    let failed =
        |e: std::io::Error| ViewError::new(SAVE_FAILED, format!("{}: {e}", target.display()));
    if !overwrite && input.is_some_and(|input| same_file(target, input)) {
        return Err(ViewError::new(
            SAVE_REFUSED,
            "the file that was opened is not written over unless the replacement is ordered: save under another name",
        ));
    }
    let exists = target.exists();
    if exists && !overwrite {
        return Err(ViewError::new(
            SAVE_REFUSED,
            format!(
                "{} exists; save under another name or order the replacement",
                target.display()
            ),
        ));
    }
    if exists && !backup_suffix.is_empty() {
        let mut name = target.as_os_str().to_os_string();
        name.push(backup_suffix);
        fs::copy(target, PathBuf::from(name)).map_err(failed)?;
    }
    let mut part_name = target.as_os_str().to_os_string();
    part_name.push(".part");
    let part = PathBuf::from(part_name);
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(failed)?;
        file.write_all(bytes).map_err(failed)?;
        file.sync_all().map_err(failed)?;
    }
    if let Err(e) = fs::rename(&part, target) {
        let _ = fs::remove_file(&part);
        return Err(failed(e));
    }
    Ok(())
}

/// The name a settings file is offered to be saved under: next to the file that was read, with
/// `.new` before the extension (`zones.cfg` becomes `zones.new.cfg`); without a file that was
/// read, in `dir` under the name of the kind.
pub fn default_output(kind: &CfgKind, input: Option<&Path>, dir: &Path) -> PathBuf {
    let (folder, name) = match input {
        Some(path) => (
            path.parent().map(Path::to_path_buf).unwrap_or_default(),
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| kind.base_name().to_owned()),
        ),
        None => (dir.to_path_buf(), kind.base_name().to_owned()),
    };
    let renamed = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => format!("{stem}.new.{extension}"),
        _ => format!("{name}.new"),
    };
    folder.join(renamed)
}

/// What the editor says about the last thing it did with a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfgNote {
    /// The file was read from this path.
    Read(PathBuf),
    /// There is no file at this path: the defaults are shown.
    NotFound(PathBuf),
    /// The file was written to this path.
    Saved(PathBuf),
    /// The file could not be read or written.
    Failed(ViewError),
}

/// The state of the window of the settings files.
pub struct CfgEditor {
    /// The folder the files are read from, as typed.
    pub dir: String,
    /// The index in [`KINDS`] of the file that is open.
    pub selected: usize,
    /// The file being edited.
    pub doc: CfgDoc,
    /// Whether the table lists only what differs from the defaults.
    pub only_changed: bool,
    /// The name the file will be saved under, as typed.
    pub save_text: String,
    /// Whether a file that exists may be replaced.
    pub overwrite: bool,
    /// What happened last.
    pub note: Option<CfgNote>,
    ranges: BTreeMap<usize, BTreeMap<String, Range>>,
    steps: usize,
    span: f64,
    max_bytes: u64,
    backup_suffix: String,
}

impl CfgEditor {
    /// The editor on the first file, with the folder, the limits and the suffix of the settings.
    pub fn new(settings: &ViewerSettings) -> CfgEditor {
        let mut editor = CfgEditor {
            dir: settings.rules_config_dir.clone(),
            selected: 0,
            doc: CfgDoc::defaults(&KINDS[0]),
            only_changed: false,
            save_text: String::new(),
            overwrite: false,
            note: None,
            ranges: BTreeMap::new(),
            steps: settings.rules_probe_steps,
            span: settings.rules_probe_span,
            max_bytes: settings.rules_max_file_bytes,
            backup_suffix: settings.backup_suffix.clone(),
        };
        editor.select(0);
        editor
    }

    /// Opens the file of a kind from the folder; when the file is not there, its defaults.
    pub fn select(&mut self, index: usize) {
        self.selected = index.min(KINDS.len() - 1);
        self.reload();
    }

    /// Reads the open kind of file again from the folder, dropping the changes.
    pub fn reload(&mut self) {
        let kind = &KINDS[self.selected];
        let path = Path::new(self.dir.trim()).join(kind.file);
        if path.is_file() {
            match CfgDoc::open(kind, &path, self.max_bytes) {
                Ok(doc) => {
                    self.doc = doc;
                    self.note = Some(CfgNote::Read(path));
                }
                Err(error) => {
                    self.doc = CfgDoc::defaults(kind);
                    self.note = Some(CfgNote::Failed(error));
                }
            }
        } else {
            self.doc = CfgDoc::defaults(kind);
            self.note = Some(CfgNote::NotFound(path));
        }
        if !self.ranges.contains_key(&self.selected) {
            let found = probe_all(kind, self.steps, self.span);
            self.ranges.insert(self.selected, found);
        }
        self.save_text = default_output(kind, self.doc.input(), Path::new(self.dir.trim()))
            .to_string_lossy()
            .into_owned();
    }

    /// What the reader allows for each key of the open file.
    pub fn ranges(&self) -> &BTreeMap<String, Range> {
        static NONE: BTreeMap<String, Range> = BTreeMap::new();
        self.ranges.get(&self.selected).unwrap_or(&NONE)
    }

    /// The rows the table shows: all of them, or only those that differ from the defaults.
    pub fn rows(&self) -> Vec<Row> {
        let rows = self.doc.rows(self.ranges());
        if self.only_changed {
            rows.into_iter().filter(|r| r.changed).collect()
        } else {
            rows
        }
    }

    /// Writes the file under the name that was typed.
    pub fn save(&mut self) {
        let target = PathBuf::from(self.save_text.trim());
        self.note = Some(
            match self
                .doc
                .save_as(&target, self.overwrite, &self.backup_suffix)
            {
                Ok(path) => CfgNote::Saved(path),
                Err(error) => CfgNote::Failed(error),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(file: &str) -> &'static CfgKind {
        KINDS.iter().find(|k| k.file == file).unwrap()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::mem::forget(dir);
        path
    }

    #[test]
    fn the_settings_of_a_text_come_with_their_line_and_the_comment_above() {
        let text = "# one\n# two\na = 1\nb = 2\n\n# three\nc = x\nnot a setting\nd=4\n";
        let found = entries(text);
        let keys: Vec<(&str, &str, usize)> = found
            .iter()
            .map(|e| (e.key.as_str(), e.value.as_str(), e.line))
            .collect();
        assert_eq!(
            keys,
            [("a", "1", 3), ("b", "2", 4), ("c", "x", 7), ("d", "4", 9)]
        );
        assert_eq!(found[0].help, "one two");
        assert_eq!(found[1].help, "one two");
        assert_eq!(found[2].help, "three");
        assert_eq!(found[3].help, "");
    }

    #[test]
    fn a_value_is_replaced_and_the_rest_of_the_text_stays() {
        let text = "# a = 1\r\n  key =  5 \r\nother = 2";
        assert_eq!(
            replace_value(text, 2, "9").unwrap(),
            "# a = 1\r\n  key = 9\r\nother = 2"
        );
        assert_eq!(
            replace_value(text, 3, "").unwrap(),
            "# a = 1\r\n  key =  5 \r\nother = "
        );
        assert_eq!(replace_value(text, 1, "9"), None);
        assert_eq!(replace_value(text, 4, "9"), None);
        assert_eq!(replace_value(text, 0, "9"), None);
        assert_eq!(replace_value(text, 2, "9\n10"), None);
        assert_eq!(replace_value("no equals\n", 1, "1"), None);
    }

    #[test]
    fn the_kind_of_a_value_is_read_from_how_it_is_written() {
        assert_eq!(kind_of("true"), ValueKind::Flag);
        assert_eq!(kind_of("16"), ValueKind::Whole);
        assert_eq!(kind_of("-3"), ValueKind::Whole);
        assert_eq!(kind_of("0.25"), ValueKind::Number);
        assert_eq!(kind_of("1e3"), ValueKind::Number);
        assert_eq!(kind_of("thermal"), ValueKind::Word);
        assert_eq!(kind_of("inf"), ValueKind::Word);
        assert_eq!(kind_of("nan"), ValueKind::Word);
        assert_eq!(kind_of(""), ValueKind::Word);
        assert_eq!(kind_of("#ff0000"), ValueKind::Word);
    }

    #[test]
    fn the_reader_of_the_file_decides_what_is_valid() {
        let zones = kind("zones.cfg");
        let mut doc = CfgDoc::defaults(zones);
        let rows = doc.rows(&BTreeMap::new());
        let sides = rows.iter().find(|r| r.key == "circle_sides").unwrap().line;
        assert_eq!(doc.issue(), None);
        // 12 is not one of the two values that the fill allows
        doc.set_value(sides, "12").unwrap();
        let issue = doc.issue().unwrap();
        assert_eq!(issue.code, "PHOENIX-ZONE-001");
        assert!(issue.detail.contains("circle_sides"), "{}", issue.detail);
        // the reader checks the range after reading the lines and names no line: the place is the
        // setting that its message names
        assert_eq!(issue.line, Some(sides));
        assert_eq!(doc.issue_with(sides, "8"), None);
        assert!(doc.issue_with(sides, "x").is_some());
        doc.set_value(sides, "8").unwrap();
        assert_eq!(doc.issue(), None);
        // a reader that names its line keeps it
        let wrong = rows
            .iter()
            .find(|r| r.key == "snap_tolerance_nm")
            .unwrap()
            .line;
        doc.set_value(wrong, "99999").unwrap();
        assert_eq!(doc.issue().unwrap().line, Some(wrong));
        doc.set_value(wrong, "abc").unwrap();
        assert_eq!(doc.issue().unwrap().line, Some(wrong));
        let marked = doc.rows(&BTreeMap::new());
        assert_eq!(marked.iter().filter(|r| r.refused).count(), 1);
        assert!(marked.iter().find(|r| r.line == wrong).unwrap().refused);
    }

    #[test]
    fn a_key_is_mentioned_only_as_a_word_of_its_own() {
        assert!(mentions("circle_sides must be 8 or 16", "circle_sides"));
        assert!(mentions("the value of a.b is not valid", "a.b"));
        assert!(!mentions("max_fragments_total must", "max_fragments"));
        assert!(!mentions("my_circle_sides", "circle_sides"));
        assert!(mentions(
            "x circle_sides_x and circle_sides,",
            "circle_sides"
        ));
        assert!(!mentions("anything", ""));
    }

    #[test]
    fn changes_against_the_default_are_marked_and_can_be_put_back() {
        let k = kind("zones.cfg");
        let mut doc = CfgDoc::defaults(k);
        let none = BTreeMap::new();
        assert_eq!(doc.changed_count(&none), 0);
        assert!(!doc.is_modified());
        let line = doc
            .rows(&none)
            .iter()
            .find(|r| r.key == "thermal_spokes")
            .unwrap()
            .line;
        doc.set_value(line, "2").unwrap();
        let row = doc
            .rows(&none)
            .into_iter()
            .find(|r| r.key == "thermal_spokes")
            .unwrap();
        assert!(row.changed);
        assert_eq!(row.default.as_deref(), Some("4"));
        assert_eq!(row.kind, ValueKind::Whole);
        assert!(!row.help.is_empty());
        assert_eq!(doc.changed_count(&none), 1);
        assert!(doc.is_modified());
        doc.reset_value(line).unwrap();
        assert_eq!(doc.changed_count(&none), 0);
        assert_eq!(doc.text(), k.defaults);
        doc.set_value(line, "2").unwrap();
        doc.revert();
        assert_eq!(doc.text(), k.defaults);
        assert!(doc.reset_value(9999).is_err());
        assert!(doc.set_value(9999, "1").is_err());
    }

    #[test]
    fn a_setting_the_file_lacks_can_be_added_with_its_default() {
        let k = kind("zones.cfg");
        let mut doc = CfgDoc::defaults(k);
        assert!(doc.missing().is_empty());
        let mut text = String::new();
        for e in entries(k.defaults) {
            if e.key != "circle_sides" {
                text.push_str(&format!("{} = {}\n", e.key, e.value));
            }
        }
        let path = scratch("zones.cfg");
        fs::write(&path, text.trim_end_matches('\n')).unwrap();
        doc = CfgDoc::open(k, &path, 1 << 20).unwrap();
        assert_eq!(doc.missing(), ["circle_sides"]);
        doc.add_missing("circle_sides").unwrap();
        assert!(doc.missing().is_empty());
        assert!(doc.text().ends_with("circle_sides = 16\n"));
        assert_eq!(doc.issue(), None);
        assert!(doc.add_missing("nothing").is_err());
    }

    #[test]
    fn a_flag_is_true_or_false_and_a_word_is_not_measured() {
        let k = kind("zones.cfg");
        let all = probe_all(k, 80, 1.0e15);
        assert_eq!(all["keep_unconnected_islands"], Range::Flag);
        assert_eq!(all["pad_connection"], Range::NotProbed);
        assert_eq!(Range::Flag.describe(), "true | false");
        assert_eq!(Range::NotProbed.describe(), "-");
    }

    #[test]
    fn the_range_of_a_number_is_what_the_reader_accepts() {
        let k = kind("zones.cfg");
        let all = probe_all(k, 80, 1.0e15);
        // the fill needs 1 to 90 degrees for the pieces of an arc
        assert_eq!(
            all["arc_piece_degrees"],
            Range::Numbers {
                low: Some("1".to_owned()),
                high: Some("90".to_owned()),
                gaps: false
            }
        );
        assert_eq!(all["arc_piece_degrees"].describe(), "1 .. 90");
        // 2 or 4 spokes: the limits are 2 and 4 and 3 is refused
        assert_eq!(
            all["thermal_spokes"],
            Range::Numbers {
                low: Some("2".to_owned()),
                high: Some("4".to_owned()),
                gaps: true
            }
        );
        assert!(all["thermal_spokes"].has_gaps());
        assert!(!all["arc_piece_degrees"].has_gaps());
        // 8 or 16 sides
        assert!(all["circle_sides"].has_gaps());
        // the width of a spoke: at least 1
        match &all["thermal_spoke_width_nm"] {
            Range::Numbers { low, high, .. } => {
                assert_eq!(low.as_deref(), Some("1"));
                assert!(high.is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_range_of_a_number_with_a_fraction_is_found_too() {
        let k = kind("viewer.cfg");
        let all = probe_all(k, 80, 1.0e15);
        // ui_scale_percent is a whole number with limits; font_size_pt has a fraction
        match &all["font_size_pt"] {
            Range::Numbers { low, high, gaps } => {
                assert!(low.is_some() && high.is_some());
                assert!(!gaps);
            }
            other => panic!("{other:?}"),
        }
        // a number that must be above zero has no smallest value: it says so
        match &all["zoom_wheel_rate"] {
            Range::Numbers { low, .. } => assert_eq!(low.as_deref(), Some(">0")),
            other => panic!("{other:?}"),
        }
        // a colour is a word
        assert_eq!(all["colour.wire"], Range::NotProbed);
    }

    #[test]
    fn a_limit_beyond_the_span_is_not_invented() {
        let k = kind("zones.cfg");
        let all = probe_all(k, 80, 100.0);
        // the area has no larger limit within twice its own value: the range says so with …
        match &all["min_island_area_nm2"] {
            Range::Numbers { high, .. } => assert_eq!(*high, None),
            other => panic!("{other:?}"),
        }
        assert!(all["min_island_area_nm2"].describe().ends_with('…'));
        assert_eq!(all["min_island_area_nm2"].describe(), "0 .. …");
    }

    #[test]
    fn a_text_that_the_reader_refuses_is_not_written() {
        let k = kind("zones.cfg");
        let mut doc = CfgDoc::defaults(k);
        let line = doc
            .rows(&BTreeMap::new())
            .iter()
            .find(|r| r.key == "circle_sides")
            .unwrap()
            .line;
        doc.set_value(line, "13").unwrap();
        let target = scratch("zones.new.cfg");
        let error = doc.save_as(&target, false, ".bak").unwrap_err();
        assert_eq!(error.code, CHECK_FILE_UNUSABLE);
        assert!(!target.exists());
    }

    #[test]
    fn the_file_is_written_whole_and_nothing_is_replaced_without_the_order() {
        let k = kind("zones.cfg");
        let mut doc = CfgDoc::defaults(k);
        let line = doc
            .rows(&BTreeMap::new())
            .iter()
            .find(|r| r.key == "circle_sides")
            .unwrap()
            .line;
        doc.set_value(line, "8").unwrap();
        let target = scratch("zones.new.cfg");
        let saved = doc.save_as(&target, false, ".bak").unwrap();
        assert_eq!(saved, target);
        assert_eq!(fs::read_to_string(&target).unwrap(), doc.text());
        let mut part = target.as_os_str().to_os_string();
        part.push(".part");
        assert!(!PathBuf::from(part).exists());
        // it exists now: a second save without the order is refused and leaves it as it was
        let before = fs::read(&target).unwrap();
        doc.set_value(line, "16").unwrap();
        let error = doc.save_as(&target, false, ".bak").unwrap_err();
        assert_eq!(error.code, SAVE_REFUSED);
        assert_eq!(fs::read(&target).unwrap(), before);
        // with the order it is replaced and the old one is kept next to it
        doc.save_as(&target, true, ".bak").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), doc.text());
        let mut copy = target.as_os_str().to_os_string();
        copy.push(".bak");
        assert_eq!(fs::read(PathBuf::from(copy)).unwrap(), before);
    }

    #[test]
    fn the_file_that_was_read_is_never_written_over_without_the_order() {
        let k = kind("zones.cfg");
        let path = scratch("zones.cfg");
        fs::write(&path, k.defaults).unwrap();
        let mut doc = CfgDoc::open(k, &path, 1 << 20).unwrap();
        assert_eq!(doc.input(), Some(path.as_path()));
        let line = doc
            .rows(&BTreeMap::new())
            .iter()
            .find(|r| r.key == "circle_sides")
            .unwrap()
            .line;
        doc.set_value(line, "8").unwrap();
        let error = doc.save_as(&path, false, ".bak").unwrap_err();
        assert_eq!(error.code, SAVE_REFUSED);
        assert_eq!(fs::read_to_string(&path).unwrap(), k.defaults);
        doc.save_as(&path, true, ".bak").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), doc.text());
    }

    #[test]
    fn a_file_that_is_too_large_or_not_text_is_not_opened() {
        let k = kind("zones.cfg");
        let path = scratch("zones.cfg");
        fs::write(&path, k.defaults).unwrap();
        let error = CfgDoc::open(k, &path, 10).err().unwrap();
        assert_eq!(error.code, FILE_UNUSABLE);
        fs::write(&path, [0xff_u8, 0xfe, 0x00]).unwrap();
        let error = CfgDoc::open(k, &path, 1 << 20).err().unwrap();
        assert_eq!(error.code, FILE_UNUSABLE);
        let missing = path.with_file_name("not-there.cfg");
        assert_eq!(
            CfgDoc::open(k, &missing, 1 << 20).err().unwrap().code,
            FILE_UNUSABLE
        );
    }

    #[test]
    fn the_name_offered_for_saving_is_never_the_name_that_was_read() {
        let k = kind("zones.cfg");
        let input = Path::new("some").join("folder").join("zones.cfg");
        let offered = default_output(k, Some(&input), Path::new("config"));
        assert_eq!(
            offered,
            Path::new("some").join("folder").join("zones.new.cfg")
        );
        assert_ne!(offered, input);
        assert_eq!(
            default_output(k, None, Path::new("config")),
            Path::new("config").join("zones.new.cfg")
        );
        let profile = kind("dfm/example.cfg");
        assert_eq!(
            default_output(profile, None, Path::new("cfg")),
            Path::new("cfg").join("example.new.cfg")
        );
        let bare = Path::new("noext");
        assert_eq!(
            default_output(k, Some(bare), Path::new("config")),
            PathBuf::from("noext.new")
        );
    }

    #[test]
    fn the_editor_starts_from_the_defaults_when_the_file_is_not_there() {
        let settings = ViewerSettings {
            rules_config_dir: scratch("none").to_string_lossy().into_owned(),
            ..ViewerSettings::default()
        };
        let mut editor = CfgEditor::new(&settings);
        assert!(matches!(editor.note, Some(CfgNote::NotFound(_))));
        assert_eq!(editor.doc.text(), KINDS[0].defaults);
        assert!(!editor.ranges().is_empty());
        assert_eq!(editor.rows().len(), entries(KINDS[0].defaults).len());
        editor.only_changed = true;
        assert!(editor.rows().is_empty());
        editor.select(4);
        assert_eq!(editor.doc.kind().file, "zones.cfg");
        assert!(editor.save_text.ends_with("zones.new.cfg"));
    }

    #[test]
    fn the_editor_reads_a_file_edits_it_and_saves_it_under_another_name() {
        let folder = scratch("config");
        fs::create_dir_all(&folder).unwrap();
        let k = kind("zones.cfg");
        fs::write(folder.join("zones.cfg"), k.defaults).unwrap();
        let settings = ViewerSettings {
            rules_config_dir: folder.to_string_lossy().into_owned(),
            ..ViewerSettings::default()
        };
        let mut editor = CfgEditor::new(&settings);
        editor.select(4);
        assert!(matches!(editor.note, Some(CfgNote::Read(_))));
        let line = editor
            .rows()
            .iter()
            .find(|r| r.key == "thermal_spokes")
            .unwrap()
            .line;
        editor.doc.set_value(line, "2").unwrap();
        editor.only_changed = true;
        assert_eq!(editor.rows().len(), 1);
        editor.save();
        assert!(
            matches!(editor.note, Some(CfgNote::Saved(_))),
            "{:?}",
            editor.note
        );
        let written = fs::read_to_string(folder.join("zones.new.cfg")).unwrap();
        assert!(written.contains("thermal_spokes = 2"));
        // the original is untouched
        assert_eq!(
            fs::read_to_string(folder.join("zones.cfg")).unwrap(),
            k.defaults
        );
        // saving again to the same name is refused and says why
        editor.save();
        match &editor.note {
            Some(CfgNote::Failed(error)) => assert_eq!(error.code, SAVE_REFUSED),
            other => panic!("{other:?}"),
        }
        editor.reload();
        assert!(!editor.doc.is_modified());
    }

    #[test]
    fn a_file_that_does_not_read_leaves_the_defaults_and_says_so() {
        let folder = scratch("config2");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("viewer.cfg"), [0xff_u8, 0xfe]).unwrap();
        let settings = ViewerSettings {
            rules_config_dir: folder.to_string_lossy().into_owned(),
            ..ViewerSettings::default()
        };
        let editor = CfgEditor::new(&settings);
        assert!(matches!(editor.note, Some(CfgNote::Failed(_))));
        assert_eq!(editor.doc.text(), KINDS[0].defaults);
    }
}
