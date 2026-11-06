// SPDX-License-Identifier: GPL-3.0-or-later
//! Applying the plan of the master library to a COPY of the libraries.
//!
//! Nothing here writes over a library: the libraries are given as bytes, the replacements are
//! made on a copy of those bytes, and the folder of the copy is written (by the caller) only to a
//! place that does not exist. The plan is applied only with its confirmation code, which is
//! derived from the plan itself, so a plan that was changed after it was read has another code.
//!
//! A step replaces a footprint or a symbol of one library by the master item, keeping the name it
//! has in that library, so the device sets, the schematics and the boards that use it by name keep
//! working and no part moves. A step is left out, and reported with its reason, when:
//!
//! * the group is a device set (only footprints and symbols are replaced);
//! * the replacement would turn, mirror or shift the part;
//! * the geometry is only close and not the same (unless the caller allows it);
//! * the pads (or the pins) of the master are not named the same way;
//! * a library or an item is missing, is named twice, or the library is not UTF-8 text;
//! * the library, read back after the replacement, is not exactly the library before it with the
//!   one item changed to the master.
//!
//! The steps that are made can be undone: [`Undo`] holds the bytes of every file that changed, and
//! giving them back restores the same bytes.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::codes::{
    SanError, FILE_TOO_LARGE, FILE_UNREADABLE, NOT_CONFIRMED, OUTPUT_FAILED, PLAN_INVALID,
};
use crate::model::{Item, Prim};
use crate::plan::{Plan, PlanGroup, Replacement, PLAN_FORMAT};
use crate::scan::{find_libraries, load_bytes, LoadedLibrary};
use crate::settings::SanitizerSettings;

/// The format tag of the report of an application.
pub const APPLY_FORMAT: &str = "phoenix-sanitize-apply/1";

/// A library file: its path relative to the folder, with `/`, and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibFile {
    /// The path relative to the folder.
    pub path: String,
    /// The content.
    pub bytes: Vec<u8>,
}

/// What the caller chooses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyOptions {
    /// The ids of the groups to apply; empty means all of them.
    pub groups: Vec<String>,
    /// Also replace items whose geometry is close and not the same.
    pub allow_near: bool,
}

/// Why a step was left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The group was not among the ones chosen.
    NotChosen,
    /// Device sets are not replaced.
    DeviceSet,
    /// The replacement would turn, mirror or shift the part.
    Turned,
    /// The geometry is only close.
    NotExact,
    /// The item is the master itself.
    SameItem,
    /// A library of the step is not among the files.
    LibraryMissing,
    /// A library of the step cannot be read.
    LibraryUnreadable,
    /// A library of the step is not UTF-8 text.
    NotUtf8,
    /// The item is not in its library.
    ItemMissing,
    /// The name is used twice in the library, so the element cannot be told apart.
    ItemAmbiguous,
    /// The pads or pins of the master are not named the same way.
    NamesDiffer,
    /// The library read back is not the library before with the item replaced.
    RereadDiffers,
}

impl Reason {
    /// The word used in the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::NotChosen => "not-chosen",
            Reason::DeviceSet => "device-set",
            Reason::Turned => "turned",
            Reason::NotExact => "not-exact",
            Reason::SameItem => "same-item",
            Reason::LibraryMissing => "library-missing",
            Reason::LibraryUnreadable => "library-unreadable",
            Reason::NotUtf8 => "not-utf8",
            Reason::ItemMissing => "item-missing",
            Reason::ItemAmbiguous => "item-ambiguous",
            Reason::NamesDiffer => "names-differ",
            Reason::RereadDiffers => "reread-differs",
        }
    }
}

/// One replacement of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The id of the group.
    pub group: String,
    /// `package`, `symbol` or `deviceset`.
    pub kind: String,
    /// The library file that holds the item to replace.
    pub library: String,
    /// The name of the item to replace.
    pub name: String,
    /// The library file of the master item.
    pub master_library: String,
    /// The name of the master item.
    pub master_name: String,
}

/// A step that was left out and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The step.
    pub step: Step,
    /// The reason.
    pub reason: Reason,
}

/// What undoes an application: the bytes of every file that changed, as they were.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Undo {
    /// The files as they were before.
    pub originals: Vec<LibFile>,
}

impl Undo {
    /// Gives the files their old bytes back. Returns how many files were restored.
    pub fn restore(&self, files: &mut [LibFile]) -> usize {
        let mut restored = 0;
        for original in &self.originals {
            if let Some(file) = files.iter_mut().find(|f| f.path == original.path) {
                file.bytes.clone_from(&original.bytes);
                restored += 1;
            }
        }
        restored
    }
}

/// The result of applying (or previewing) a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The libraries after the steps that were made, in the order they were given.
    pub files: Vec<LibFile>,
    /// The steps that were made.
    pub applied: Vec<Step>,
    /// The steps that were left out.
    pub skipped: Vec<Skipped>,
    /// What undoes the steps.
    pub undo: Undo,
}

impl Outcome {
    /// The paths of the files that changed, sorted.
    pub fn changed(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.undo.originals.iter().map(|f| f.path.clone()).collect();
        paths.sort();
        paths
    }

    /// The summary as text: the steps that were made and the ones left out.
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "plan steps: {} applied, {} left out, {} libraries changed\n",
            self.applied.len(),
            self.skipped.len(),
            self.undo.originals.len()
        );
        for s in &self.applied {
            out.push_str(&format!(
                "applied {} {} {}:{} <- {}:{}\n",
                s.group, s.kind, s.library, s.name, s.master_library, s.master_name
            ));
        }
        for s in &self.skipped {
            out.push_str(&format!(
                "left out {} {} {}:{} ({})\n",
                s.step.group,
                s.step.kind,
                s.step.library,
                s.step.name,
                s.reason.as_str()
            ));
        }
        out
    }

    /// The summary as JSON, with a line feed at the end.
    pub fn to_json(&self) -> String {
        let step = |s: &Step| -> Value {
            json!({
                "group": s.group,
                "kind": s.kind,
                "library": s.library,
                "name": s.name,
                "master_library": s.master_library,
                "master_name": s.master_name,
            })
        };
        let applied: Vec<Value> = self.applied.iter().map(step).collect();
        let skipped: Vec<Value> = self
            .skipped
            .iter()
            .map(|s| {
                let mut value = step(&s.step);
                value["reason"] = json!(s.reason.as_str());
                value["code"] = json!(crate::codes::STEP_SKIPPED);
                value
            })
            .collect();
        let document = json!({
            "format": APPLY_FORMAT,
            "applied": applied,
            "skipped": skipped,
            "changed": self.changed(),
        });
        let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

/// The confirmation code of a plan: 12 hexadecimal digits, from the SHA-256 of the plan file, in
/// three groups (`1A2B-3C4D-5E6F`). The same plan gives the same code; any change to the plan
/// gives another one.
pub fn confirmation_code(plan: &Plan) -> String {
    let digest = hex(&Sha256::digest(plan.to_json().as_bytes()));
    format!("{}-{}-{}", &digest[0..4], &digest[4..8], &digest[8..12])
}

// ---------------------------------------------------------------------------------------------
// Finding an element in the text of a library
// ---------------------------------------------------------------------------------------------

/// Where an element is in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Found {
    None,
    One(Span),
    Many,
}

fn unescape(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The place of the `>` that closes an opening tag that starts at the beginning of `rest`,
/// skipping quoted values.
fn open_tag_end(rest: &str) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (i, b) in rest.bytes().enumerate() {
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => quote = Some(b),
                b'>' => return Some(i),
                _ => {}
            },
        }
    }
    None
}

/// The value of an attribute written in double quotes in an opening tag.
fn attribute(open_tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    for (at, _) in open_tag.match_indices(&needle) {
        let before = open_tag[..at].chars().next_back();
        if before.is_some_and(char::is_whitespace) {
            let rest = &open_tag[at + needle.len()..];
            return rest.find('"').map(|end| unescape(&rest[..end]));
        }
    }
    None
}

/// Finds the element `tag` whose attribute `name` is `wanted`. Comments are skipped.
fn find_element(text: &str, tag: &str, wanted: &str) -> Found {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut found: Vec<Span> = Vec::new();
    let mut pos = 0usize;
    while let Some(relative) = text[pos..].find('<') {
        let at = pos + relative;
        let rest = &text[at..];
        if rest.starts_with("<!--") {
            match rest.find("-->") {
                Some(end) => pos = at + end + 3,
                None => return Found::None,
            }
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            match rest.find("]]>") {
                Some(end) => pos = at + end + 3,
                None => return Found::None,
            }
            continue;
        }
        let tag_like = rest.starts_with(&open)
            && rest[open.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_whitespace() || c == '>' || c == '/');
        if !tag_like {
            pos = at + 1;
            continue;
        }
        let Some(tag_end) = open_tag_end(rest) else {
            return Found::None;
        };
        let open_tag = &rest[..=tag_end];
        let end = if open_tag.ends_with("/>") {
            at + tag_end + 1
        } else {
            match text[at + tag_end + 1..].find(&close) {
                Some(c) => at + tag_end + 1 + c + close.len(),
                None => return Found::None,
            }
        };
        if attribute(open_tag, "name").as_deref() == Some(wanted) {
            found.push(Span { start: at, end });
        }
        pos = end;
    }
    match found.as_slice() {
        [] => Found::None,
        [one] => Found::One(*one),
        _ => Found::Many,
    }
}

/// The element with its name attribute replaced.
fn rename_element(element: &str, new_name: &str) -> Option<String> {
    let tag_end = open_tag_end(element)?;
    let open_tag = &element[..=tag_end];
    let needle = "name=\"";
    let at = open_tag.match_indices(needle).map(|(i, _)| i).find(|i| {
        open_tag[..*i]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    })?;
    let value_start = at + needle.len();
    let value_end = value_start + open_tag[value_start..].find('"')?;
    let mut out = String::with_capacity(element.len() + new_name.len());
    out.push_str(&element[..value_start]);
    out.push_str(&escape(new_name));
    out.push_str(&element[value_end..]);
    Some(out)
}

// ---------------------------------------------------------------------------------------------
// The steps
// ---------------------------------------------------------------------------------------------

fn load(file: &LibFile, s: &SanitizerSettings) -> LoadedLibrary {
    let mut library = LoadedLibrary {
        path: file.path.clone(),
        bytes: file.bytes.len() as u64,
        sha256: String::new(),
        version: String::new(),
        data: None,
        notes: Vec::new(),
    };
    load_bytes(&mut library, &file.bytes, s);
    library
}

/// The sorted names of the pads (footprints) or the pins (symbols) of an item.
fn names_of(item: &Item) -> Vec<String> {
    let mut names: Vec<String> = item
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Pad { name, .. } | Prim::Pin { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    names.sort();
    names
}

fn step_of(group: &PlanGroup, replacement: &Replacement) -> Step {
    Step {
        group: group.id.clone(),
        kind: group.kind.clone(),
        library: replacement.library.clone(),
        name: replacement.name.clone(),
        master_library: group.keep.library.clone(),
        master_name: group.keep.name.clone(),
    }
}

fn items_of<'a>(data: &'a crate::model::LibraryData, kind: &str) -> Option<&'a [Item]> {
    match kind {
        "package" => Some(data.packages.as_slice()),
        "symbol" => Some(data.symbols.as_slice()),
        _ => None,
    }
}

/// Makes one step on `work`; the files are left as they were when the step is left out.
fn make_step(
    work: &mut [LibFile],
    step: &Step,
    replacement: &Replacement,
    options: &ApplyOptions,
    s: &SanitizerSettings,
) -> Result<(), Reason> {
    if step.kind == "deviceset" {
        return Err(Reason::DeviceSet);
    }
    if replacement.turned {
        return Err(Reason::Turned);
    }
    if !replacement.exact && !options.allow_near {
        return Err(Reason::NotExact);
    }
    if step.library == step.master_library && step.name == step.master_name {
        return Err(Reason::SameItem);
    }
    let tag = match step.kind.as_str() {
        "package" => "package",
        "symbol" => "symbol",
        _ => return Err(Reason::ItemMissing),
    };
    let target_at = work
        .iter()
        .position(|f| f.path == step.library)
        .ok_or(Reason::LibraryMissing)?;
    let master_at = work
        .iter()
        .position(|f| f.path == step.master_library)
        .ok_or(Reason::LibraryMissing)?;
    let target_library = load(&work[target_at], s);
    let master_library = load(&work[master_at], s);
    let (Some(target_data), Some(master_data)) = (&target_library.data, &master_library.data)
    else {
        return Err(Reason::LibraryUnreadable);
    };
    let target_item = items_of(target_data, &step.kind)
        .and_then(|list| list.iter().find(|i| i.name == step.name))
        .ok_or(Reason::ItemMissing)?;
    let master_item = items_of(master_data, &step.kind)
        .and_then(|list| list.iter().find(|i| i.name == step.master_name))
        .ok_or(Reason::ItemMissing)?;
    if names_of(target_item) != names_of(master_item) {
        return Err(Reason::NamesDiffer);
    }
    let target_text = std::str::from_utf8(&work[target_at].bytes).map_err(|_| Reason::NotUtf8)?;
    let master_text = std::str::from_utf8(&work[master_at].bytes).map_err(|_| Reason::NotUtf8)?;
    let master_span = match find_element(master_text, tag, &step.master_name) {
        Found::One(span) => span,
        Found::None => return Err(Reason::ItemMissing),
        Found::Many => return Err(Reason::ItemAmbiguous),
    };
    let target_span = match find_element(target_text, tag, &step.name) {
        Found::One(span) => span,
        Found::None => return Err(Reason::ItemMissing),
        Found::Many => return Err(Reason::ItemAmbiguous),
    };
    let element = rename_element(&master_text[master_span.start..master_span.end], &step.name)
        .ok_or(Reason::ItemMissing)?;
    let mut new_text = String::with_capacity(target_text.len() + element.len());
    new_text.push_str(&target_text[..target_span.start]);
    new_text.push_str(&element);
    new_text.push_str(&target_text[target_span.end..]);

    // read back: the library must be the one before with this item drawn as the master
    let candidate = LibFile {
        path: step.library.clone(),
        bytes: new_text.into_bytes(),
    };
    let reread = load(&candidate, s);
    let mut expected = target_data.clone();
    let list = if tag == "package" {
        &mut expected.packages
    } else {
        &mut expected.symbols
    };
    if let Some(item) = list.iter_mut().find(|i| i.name == step.name) {
        item.prims.clone_from(&master_item.prims);
        item.has_description = master_item.has_description;
    }
    if reread.data.as_ref() != Some(&expected) {
        return Err(Reason::RereadDiffers);
    }
    work[target_at] = candidate;
    Ok(())
}

fn run(
    plan: &Plan,
    files: &[LibFile],
    options: &ApplyOptions,
    s: &SanitizerSettings,
) -> Result<Outcome, SanError> {
    if plan.format != PLAN_FORMAT {
        return Err(SanError::new(
            PLAN_INVALID,
            format!("the format must be {PLAN_FORMAT}"),
        ));
    }
    for id in &options.groups {
        if !plan.groups.iter().any(|g| &g.id == id) {
            return Err(SanError::new(
                PLAN_INVALID,
                format!("the plan has no group {id}"),
            ));
        }
    }
    let mut work: Vec<LibFile> = files.to_vec();
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    for group in &plan.groups {
        let chosen = options.groups.is_empty() || options.groups.contains(&group.id);
        for replacement in &group.replace {
            let step = step_of(group, replacement);
            let result = if chosen {
                make_step(&mut work, &step, replacement, options, s)
            } else {
                Err(Reason::NotChosen)
            };
            match result {
                Ok(()) => applied.push(step),
                Err(reason) => skipped.push(Skipped { step, reason }),
            }
        }
    }
    let originals: Vec<LibFile> = files
        .iter()
        .zip(&work)
        .filter(|(before, after)| before.bytes != after.bytes)
        .map(|(before, _)| before.clone())
        .collect();
    Ok(Outcome {
        files: work,
        applied,
        skipped,
        undo: Undo { originals },
    })
}

/// Works out what applying the plan would do, on a copy, without any confirmation. Nothing is
/// written; this is the dry run.
pub fn preview_plan(
    plan: &Plan,
    files: &[LibFile],
    options: &ApplyOptions,
    s: &SanitizerSettings,
) -> Result<Outcome, SanError> {
    run(plan, files, options, s)
}

/// Applies the plan to a copy of the files, only when `confirm` is the confirmation code of the
/// plan (any case). The files that were given are not changed; the new bytes are in the outcome.
pub fn apply_plan(
    plan: &Plan,
    files: &[LibFile],
    confirm: &str,
    options: &ApplyOptions,
    s: &SanitizerSettings,
) -> Result<Outcome, SanError> {
    let code = confirmation_code(plan);
    if !confirm.trim().eq_ignore_ascii_case(&code) {
        return Err(SanError::new(
            NOT_CONFIRMED,
            "the code does not belong to this plan; read the plan and give its code",
        ));
    }
    run(plan, files, options, s)
}

// ---------------------------------------------------------------------------------------------
// Reading a folder and writing a new one
// ---------------------------------------------------------------------------------------------

/// Reads every `.lbr` file under a folder, in the order of their paths. A file that is larger
/// than the limit, or that cannot be read, is an error: a copy that left a library out would read
/// as a complete one.
pub fn read_folder(root: &Path, s: &SanitizerSettings) -> Result<Vec<LibFile>, SanError> {
    let (paths, _, _) = find_libraries(root, s)?;
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let name = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let length = fs::metadata(&path)
            .map_err(|e| SanError::new(FILE_UNREADABLE, format!("{name}: {e}")))?
            .len();
        if length > s.limits.max_file_bytes {
            return Err(SanError::new(FILE_TOO_LARGE, name));
        }
        let bytes =
            fs::read(&path).map_err(|e| SanError::new(FILE_UNREADABLE, format!("{name}: {e}")))?;
        files.push(LibFile { path: name, bytes });
    }
    Ok(files)
}

/// Writes the files into a folder that does not exist yet, keeping their relative paths. Nothing
/// is written over an existing file, and nothing inside the folder that was read.
pub fn write_folder(
    files: &[LibFile],
    out: &Path,
    source: &Path,
) -> Result<Vec<PathBuf>, SanError> {
    let fail = |what: String| SanError::new(OUTPUT_FAILED, what);
    if out.exists() {
        return Err(fail(format!(
            "{} exists and is never replaced",
            out.display()
        )));
    }
    let source = fs::canonicalize(source).map_err(|e| fail(e.to_string()))?;
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent =
        fs::canonicalize(parent).map_err(|e| fail(format!("{}: {e}", parent.display())))?;
    if parent.starts_with(&source) {
        return Err(fail(
            "the new folder is inside the folder that was read".to_owned(),
        ));
    }
    for file in files {
        let safe = Path::new(&file.path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
        if !safe {
            return Err(fail(format!("{}: not a path inside the folder", file.path)));
        }
    }
    fs::create_dir_all(out).map_err(|e| fail(format!("{}: {e}", out.display())))?;
    let mut written = Vec::with_capacity(files.len());
    for file in files {
        let target = out.join(&file.path);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir).map_err(|e| fail(format!("{}: {e}", dir.display())))?;
        }
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| fail(format!("{}: {e}", target.display())))?;
        std::io::Write::write_all(&mut handle, &file.bytes)
            .map_err(|e| fail(format!("{}: {e}", target.display())))?;
        written.push(target);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{plan_of, ItemRef};
    use crate::report::analyse_libraries;
    use crate::testdata::{deviceset_xml, library_xml, package_xml, symbol_xml};
    use eh3d_phoenix_corpus::InventoryConfig;

    const TWO: [(f64, f64); 2] = [(-0.9, 0.0), (0.9, 0.0)];

    fn file(path: &str, text: &str) -> LibFile {
        LibFile {
            path: path.into(),
            bytes: text.as_bytes().to_vec(),
        }
    }

    fn text(f: &LibFile) -> &str {
        std::str::from_utf8(&f.bytes).unwrap()
    }

    fn plan_of_files(files: &[LibFile], s: &SanitizerSettings) -> Plan {
        let libs: Vec<LoadedLibrary> = files.iter().map(|f| load(f, s)).collect();
        plan_of(&analyse_libraries(&libs, Vec::new(), 0, s))
    }

    /// a.lbr holds the master, b.lbr the same footprint under another name, c.lbr a close one.
    fn three() -> Vec<LibFile> {
        vec![
            file(
                "a.lbr",
                &library_xml(&[package_xml("A", &TWO, 1.2, 1.4)], &[], &[]),
            ),
            file(
                "b.lbr",
                &library_xml(&[package_xml("B", &TWO, 1.2, 1.4)], &[], &[]),
            ),
            file(
                "c.lbr",
                &library_xml(
                    &[package_xml("C", &[(-0.92, 0.0), (0.9, 0.0)], 1.2, 1.4)],
                    &[],
                    &[],
                ),
            ),
        ]
    }

    fn step_group(kind: &str, replacement: Replacement, keep: (&str, &str)) -> Plan {
        Plan {
            format: PLAN_FORMAT.into(),
            applied: false,
            threshold: 90,
            preconditions: Vec::new(),
            groups: vec![PlanGroup {
                id: "X001".into(),
                kind: kind.into(),
                keep: ItemRef {
                    library: keep.0.into(),
                    name: keep.1.into(),
                },
                replace: vec![replacement],
                review: Vec::new(),
            }],
        }
    }

    fn replacement(library: &str, name: &str) -> Replacement {
        Replacement {
            library: library.into(),
            name: name.into(),
            score: 100,
            exact: true,
            turned: false,
        }
    }

    #[test]
    fn the_confirmation_code_is_derived_from_the_plan() {
        let s = SanitizerSettings::default();
        let plan = plan_of_files(&three(), &s);
        let code = confirmation_code(&plan);
        assert_eq!(code.len(), 14);
        assert!(code
            .chars()
            .all(|c| c == '-' || c.is_ascii_digit() || ('A'..='F').contains(&c)));
        assert_eq!(code.matches('-').count(), 2);
        assert_eq!(confirmation_code(&plan), code);
        let mut other = plan.clone();
        other.threshold = 80;
        assert_ne!(confirmation_code(&other), code);
        other = plan.clone();
        other.groups[0].replace.pop();
        assert_ne!(confirmation_code(&other), code);
    }

    #[test]
    fn nothing_is_applied_without_the_code_of_the_plan() {
        let s = SanitizerSettings::default();
        let files = three();
        let plan = plan_of_files(&files, &s);
        let e = apply_plan(
            &plan,
            &files,
            "0000-0000-0000",
            &ApplyOptions::default(),
            &s,
        )
        .unwrap_err();
        assert_eq!(e.code, NOT_CONFIRMED);
        let e = apply_plan(&plan, &files, "", &ApplyOptions::default(), &s).unwrap_err();
        assert_eq!(e.code, NOT_CONFIRMED);
        let code = confirmation_code(&plan).to_lowercase();
        let outcome = apply_plan(
            &plan,
            &files,
            &format!(" {code} "),
            &ApplyOptions::default(),
            &s,
        )
        .unwrap();
        assert_eq!(outcome.applied.len(), 1);
        // the files that were given are not touched
        assert_eq!(files, three());
    }

    #[test]
    fn an_exact_step_is_made_and_a_close_one_waits_for_permission() {
        let s = SanitizerSettings::default();
        let files = three();
        let plan = plan_of_files(&files, &s);
        let strict = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
        assert_eq!(strict.applied.len(), 1);
        assert_eq!(strict.applied[0].name, "B");
        assert_eq!(strict.skipped.len(), 1);
        assert_eq!(strict.skipped[0].step.name, "C");
        assert_eq!(strict.skipped[0].reason, Reason::NotExact);
        // the master drawn under the old name is the same text, so no file changed
        assert!(strict.changed().is_empty());
        assert_eq!(strict.files, files);

        let near = ApplyOptions {
            allow_near: true,
            ..ApplyOptions::default()
        };
        let loose = preview_plan(&plan, &files, &near, &s).unwrap();
        assert_eq!(loose.applied.len(), 2);
        assert_eq!(loose.changed(), vec!["c.lbr".to_owned()]);
        let c = &loose.files[2];
        assert!(text(c).contains("<package name=\"C\">"));
        assert!(text(c).contains("x=\"-0.9\""));
        assert!(!text(c).contains("x=\"-0.92\""));
        assert_eq!(loose.files[0], files[0]);
        assert_eq!(loose.files[1], files[1]);
        // the library read back holds the master footprint under the name it had
        let reread = load(c, &s);
        let data = reread.data.unwrap();
        let master = load(&files[0], &s).data.unwrap();
        assert_eq!(data.packages[0].name, "C");
        assert_eq!(data.packages[0].prims, master.packages[0].prims);
    }

    #[test]
    fn undo_gives_back_the_same_bytes() {
        let s = SanitizerSettings::default();
        let files = three();
        let plan = plan_of_files(&files, &s);
        let near = ApplyOptions {
            allow_near: true,
            ..ApplyOptions::default()
        };
        let outcome = apply_plan(&plan, &files, &confirmation_code(&plan), &near, &s).unwrap();
        assert_ne!(outcome.files, files);
        let mut back = outcome.files.clone();
        assert_eq!(outcome.undo.restore(&mut back), 1);
        assert_eq!(back, files);
        // and again: the same step can be made once more
        let again = preview_plan(&plan, &back, &near, &s).unwrap();
        assert_eq!(again.files, outcome.files);
    }

    #[test]
    fn a_step_that_turns_a_part_or_renames_pads_or_is_a_device_set_is_left_out() {
        let s = SanitizerSettings::default();
        let three_pads = [(-1.0, 0.0), (0.0, 0.0), (1.0, 0.0)];
        let files = vec![
            file(
                "m.lbr",
                &library_xml(&[package_xml("M", &TWO, 1.2, 1.4)], &[], &[]),
            ),
            file(
                "t.lbr",
                &library_xml(&[package_xml("T", &TWO, 1.2, 1.4)], &[], &[]),
            ),
            file(
                "w.lbr",
                &library_xml(&[package_xml("W", &three_pads, 1.2, 1.4)], &[], &[]),
            ),
        ];
        let turned = Replacement {
            turned: true,
            ..replacement("t.lbr", "T")
        };
        let cases = [
            (
                step_group("package", turned, ("m.lbr", "M")),
                Reason::Turned,
            ),
            (
                step_group("package", replacement("w.lbr", "W"), ("m.lbr", "M")),
                Reason::NamesDiffer,
            ),
            (
                step_group("deviceset", replacement("t.lbr", "T"), ("m.lbr", "M")),
                Reason::DeviceSet,
            ),
            (
                step_group("package", replacement("nowhere.lbr", "T"), ("m.lbr", "M")),
                Reason::LibraryMissing,
            ),
            (
                step_group("package", replacement("t.lbr", "GONE"), ("m.lbr", "M")),
                Reason::ItemMissing,
            ),
            (
                step_group("package", replacement("t.lbr", "T"), ("m.lbr", "GONE")),
                Reason::ItemMissing,
            ),
            (
                step_group("package", replacement("m.lbr", "M"), ("m.lbr", "M")),
                Reason::SameItem,
            ),
        ];
        for (plan, reason) in cases {
            let outcome = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
            assert!(outcome.applied.is_empty(), "{reason:?}");
            assert_eq!(outcome.skipped.len(), 1, "{reason:?}");
            assert_eq!(outcome.skipped[0].reason, reason);
            assert_eq!(outcome.files, files);
            assert!(outcome.undo.originals.is_empty());
        }
        let text = preview_plan(
            &step_group("package", replacement("w.lbr", "W"), ("m.lbr", "M")),
            &files,
            &ApplyOptions::default(),
            &s,
        )
        .unwrap()
        .to_text();
        assert!(text.contains("left out X001 package w.lbr:W (names-differ)"));
    }

    #[test]
    fn a_symbol_is_replaced_when_its_pins_are_named_the_same_way() {
        let s = SanitizerSettings::default();
        let pins = [("1", 0.0, 0.0), ("2", 5.08, 0.0)];
        let files = vec![
            file(
                "m.lbr",
                &library_xml(&[], &[symbol_xml("SM", &pins, 1.0)], &[]),
            ),
            file(
                "t.lbr",
                &library_xml(&[], &[symbol_xml("ST", &pins, 1.001)], &[]),
            ),
            file(
                "p.lbr",
                &library_xml(
                    &[],
                    &[symbol_xml("SP", &[("A", 0.0, 0.0), ("B", 5.08, 0.0)], 1.0)],
                    &[],
                ),
            ),
        ];
        let plan = step_group("symbol", replacement("t.lbr", "ST"), ("m.lbr", "SM"));
        let outcome = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
        assert_eq!(outcome.applied.len(), 1);
        assert_eq!(outcome.changed(), vec!["t.lbr".to_owned()]);
        assert!(text(&outcome.files[1]).contains("<symbol name=\"ST\">"));
        assert!(!text(&outcome.files[1]).contains("1.001"));
        let named = step_group("symbol", replacement("p.lbr", "SP"), ("m.lbr", "SM"));
        let refused = preview_plan(&named, &files, &ApplyOptions::default(), &s).unwrap();
        assert_eq!(refused.skipped[0].reason, Reason::NamesDiffer);
    }

    #[test]
    fn the_device_sets_that_use_the_footprint_by_name_are_left_as_they_are() {
        let s = SanitizerSettings::default();
        let symbol = symbol_xml("S", &[("1", 0.0, 0.0), ("2", 5.08, 0.0)], 1.0);
        let set = deviceset_xml("R", "R", "S", "B", &[("1", "1"), ("2", "2")]);
        let files = vec![
            file(
                "a.lbr",
                &library_xml(&[package_xml("A", &TWO, 1.2, 1.4)], &[], &[]),
            ),
            file(
                "b.lbr",
                &library_xml(
                    &[package_xml("B", &[(-0.92, 0.0), (0.9, 0.0)], 1.2, 1.4)],
                    &[symbol],
                    &[set],
                ),
            ),
        ];
        let close = Replacement {
            exact: false,
            ..replacement("b.lbr", "B")
        };
        let plan = step_group("package", close, ("a.lbr", "A"));
        let near = ApplyOptions {
            allow_near: true,
            ..ApplyOptions::default()
        };
        let outcome = preview_plan(&plan, &files, &near, &s).unwrap();
        assert_eq!(outcome.applied.len(), 1);
        let after = text(&outcome.files[1]);
        assert!(after.contains("<deviceset name=\"R\""));
        assert!(after.contains("package=\"B\""));
        assert!(after.contains("<symbol name=\"S\">"));
        let before = text(&files[1]);
        let cut = |t: &str| t[t.find("</packages>").unwrap()..].to_owned();
        assert_eq!(cut(before), cut(after));
    }

    #[test]
    fn a_library_that_is_not_text_or_not_a_library_is_left_out() {
        let s = SanitizerSettings::default();
        let mut latin = library_xml(&[package_xml("T", &TWO, 1.2, 1.4)], &[], &[]).into_bytes();
        latin.extend_from_slice(&[0xe9, 0xe9]);
        let files = vec![
            file(
                "m.lbr",
                &library_xml(&[package_xml("M", &TWO, 1.2, 1.4)], &[], &[]),
            ),
            LibFile {
                path: "t.lbr".into(),
                bytes: latin,
            },
            file("broken.lbr", "<eagle"),
        ];
        let plan = step_group("package", replacement("t.lbr", "T"), ("m.lbr", "M"));
        let outcome = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
        assert!(matches!(
            outcome.skipped[0].reason,
            Reason::NotUtf8 | Reason::LibraryUnreadable
        ));
        let plan = step_group("package", replacement("broken.lbr", "T"), ("m.lbr", "M"));
        let outcome = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
        assert_eq!(outcome.skipped[0].reason, Reason::LibraryUnreadable);
    }

    #[test]
    fn only_the_groups_that_are_chosen_are_applied_and_an_unknown_group_is_refused() {
        let s = SanitizerSettings::default();
        let files = three();
        let plan = plan_of_files(&files, &s);
        let id = plan.groups[0].id.clone();
        let chosen = ApplyOptions {
            groups: vec![id],
            allow_near: true,
        };
        assert_eq!(
            preview_plan(&plan, &files, &chosen, &s)
                .unwrap()
                .applied
                .len(),
            2
        );
        let none = ApplyOptions {
            groups: vec!["Z999".into()],
            ..ApplyOptions::default()
        };
        assert_eq!(
            preview_plan(&plan, &files, &none, &s).unwrap_err().code,
            PLAN_INVALID
        );
        let mut wrong = plan.clone();
        wrong.format = "x".into();
        assert_eq!(
            preview_plan(&wrong, &files, &ApplyOptions::default(), &s)
                .unwrap_err()
                .code,
            PLAN_INVALID
        );
        // a group that is not chosen is reported as left out
        let mut two = plan.clone();
        let mut extra = two.groups[0].clone();
        extra.id = "P002".into();
        two.groups.push(extra);
        let only_first = ApplyOptions {
            groups: vec![two.groups[0].id.clone()],
            allow_near: true,
        };
        let outcome = preview_plan(&two, &files, &only_first, &s).unwrap();
        assert_eq!(outcome.applied.len(), 2);
        assert_eq!(
            outcome
                .skipped
                .iter()
                .filter(|k| k.reason == Reason::NotChosen)
                .count(),
            2
        );
    }

    #[test]
    fn an_element_is_found_by_its_name_and_comments_and_look_alikes_are_skipped() {
        let xml = "<packages>\n<!-- <package name=\"A\"> -->\n<package3d name=\"A\"/>\n<package name=\"A\" x=\"1\"><smd name=\"1\"/></package>\n<package name=\"Q&quot;1\"/>\n<package name=\"D\"></package><package name=\"D\"></package>\n</packages>";
        match find_element(xml, "package", "A") {
            Found::One(span) => {
                assert!(xml[span.start..span.end].starts_with("<package name=\"A\" x=\"1\">"));
                assert!(xml[span.start..span.end].ends_with("</package>"));
            }
            other => panic!("{other:?}"),
        }
        match find_element(xml, "package", "Q\"1") {
            Found::One(span) => {
                assert_eq!(&xml[span.start..span.end], "<package name=\"Q&quot;1\"/>")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(find_element(xml, "package", "D"), Found::Many);
        assert_eq!(find_element(xml, "package", "Z"), Found::None);
        assert_eq!(
            find_element("<package name=\"A\">", "package", "A"),
            Found::None
        );
        assert_eq!(
            rename_element(
                "<package name=\"A\" x=\"1\"><smd name=\"1\"/></package>",
                "R&D"
            )
            .unwrap(),
            "<package name=\"R&amp;D\" x=\"1\"><smd name=\"1\"/></package>"
        );
        assert_eq!(rename_element("<package x=\"1\"/>", "B"), None);
        assert_eq!(
            attribute("<a data-name=\"x\" name=\"y\">", "name").as_deref(),
            Some("y")
        );
    }

    #[test]
    fn a_folder_is_read_and_a_copy_is_written_only_to_a_new_place() {
        let s = SanitizerSettings::default();
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("libs");
        fs::create_dir_all(source.join("sub")).unwrap();
        for f in three() {
            let target = if f.path == "c.lbr" {
                source.join("sub").join("c.lbr")
            } else {
                source.join(&f.path)
            };
            fs::write(target, &f.bytes).unwrap();
        }
        fs::write(source.join("notes.txt"), "x").unwrap();
        let files = read_folder(&source, &s).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["a.lbr", "b.lbr", "sub/c.lbr"]);

        let out = dir.path().join("copy");
        let written = write_folder(&files, &out, &source).unwrap();
        assert_eq!(written.len(), 3);
        assert_eq!(
            fs::read(out.join("sub").join("c.lbr")).unwrap(),
            files[2].bytes
        );
        // a folder that exists, or that is inside the source, is refused
        assert_eq!(
            write_folder(&files, &out, &source).unwrap_err().code,
            OUTPUT_FAILED
        );
        let inside = source.join("again");
        assert_eq!(
            write_folder(&files, &inside, &source).unwrap_err().code,
            OUTPUT_FAILED
        );
        assert!(!inside.exists());
        let evil = vec![LibFile {
            path: "../x.lbr".into(),
            bytes: Vec::new(),
        }];
        let elsewhere = dir.path().join("evil");
        assert_eq!(
            write_folder(&evil, &elsewhere, &source).unwrap_err().code,
            OUTPUT_FAILED
        );
        assert!(!elsewhere.exists());
        let tiny = SanitizerSettings {
            limits: InventoryConfig {
                max_file_bytes: 10,
                ..InventoryConfig::default()
            },
            ..SanitizerSettings::default()
        };
        assert_eq!(
            read_folder(&source, &tiny).unwrap_err().code,
            FILE_TOO_LARGE
        );
    }

    #[test]
    fn the_report_of_an_application_is_the_same_every_time() {
        let s = SanitizerSettings::default();
        let files = three();
        let plan = plan_of_files(&files, &s);
        let a = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
        let b = preview_plan(&plan, &files, &ApplyOptions::default(), &s).unwrap();
        assert_eq!(a.to_json(), b.to_json());
        let json: Value = serde_json::from_str(&a.to_json()).unwrap();
        assert_eq!(json["format"], APPLY_FORMAT);
        assert_eq!(json["applied"][0]["name"], "B");
        assert_eq!(json["skipped"][0]["reason"], "not-exact");
        assert_eq!(json["skipped"][0]["code"], "PHOENIX-SAN-016");
        assert!(a
            .to_text()
            .starts_with("plan steps: 1 applied, 1 left out, 0 libraries changed\n"));
    }
}
