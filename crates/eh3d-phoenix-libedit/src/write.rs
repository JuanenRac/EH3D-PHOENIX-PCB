// SPDX-License-Identifier: GPL-3.0-or-later
//! Writing a library as an EAGLE file, proving that the file reads back the same, and saving it
//! under a new name. The input of an edit is never overwritten and no existing file is replaced;
//! a copy of the input, named by the first digits of its SHA-256, is kept next to the new file.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_eagle::{write_document, Element, Limits};
use sha2::{Digest, Sha256};

use crate::codes::{LibError, BACKUP_FAILED, ROUND_TRIP, WRITE_FAILED};
use crate::model::{DeviceSet, Item, Library, Package, Symbol};
use crate::read::read_library;

/// A length in millionths as the shortest exact decimal text: `1.27`, `-0.05`, `3`.
pub fn fmt_mm(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    let (int, frac) = (abs / 1_000_000, abs % 1_000_000);
    if frac == 0 {
        format!("{sign}{int}")
    } else {
        let text = format!("{frac:06}");
        format!("{sign}{int}.{}", text.trim_end_matches('0'))
    }
}

type Attrs = Vec<(String, String)>;

fn put(attrs: &mut Attrs, name: &str, value: impl Into<String>) {
    attrs.push((name.to_owned(), value.into()));
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn element(name: &str, attrs: Attrs, text: &str, children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attrs,
        text: text.to_owned(),
        children,
    }
}

/// The element of a drawing item.
pub fn item_element(item: &Item) -> Element {
    let mut a = Attrs::new();
    match item {
        Item::Pad(p) => {
            put(&mut a, "name", p.name.clone());
            put(&mut a, "x", fmt_mm(p.x));
            put(&mut a, "y", fmt_mm(p.y));
            put(&mut a, "drill", fmt_mm(p.drill));
            if p.diameter != 0 {
                put(&mut a, "diameter", fmt_mm(p.diameter));
            }
            if p.shape != crate::model::PadShape::Round {
                put(&mut a, "shape", p.shape.keyword());
            }
            if !p.rot.is_empty() {
                put(&mut a, "rot", p.rot.clone());
            }
            if !p.stop {
                put(&mut a, "stop", yes_no(false));
            }
            if !p.thermals {
                put(&mut a, "thermals", yes_no(false));
            }
            if p.first {
                put(&mut a, "first", yes_no(true));
            }
            a.extend(p.extra.iter().cloned());
            element("pad", a, "", Vec::new())
        }
        Item::Smd(p) => {
            put(&mut a, "name", p.name.clone());
            put(&mut a, "x", fmt_mm(p.x));
            put(&mut a, "y", fmt_mm(p.y));
            put(&mut a, "dx", fmt_mm(p.dx));
            put(&mut a, "dy", fmt_mm(p.dy));
            put(&mut a, "layer", p.layer.to_string());
            if p.roundness != 0 {
                put(&mut a, "roundness", p.roundness.to_string());
            }
            if !p.rot.is_empty() {
                put(&mut a, "rot", p.rot.clone());
            }
            if !p.stop {
                put(&mut a, "stop", yes_no(false));
            }
            if !p.thermals {
                put(&mut a, "thermals", yes_no(false));
            }
            if !p.cream {
                put(&mut a, "cream", yes_no(false));
            }
            a.extend(p.extra.iter().cloned());
            element("smd", a, "", Vec::new())
        }
        Item::Wire(w) => {
            put(&mut a, "x1", fmt_mm(w.x1));
            put(&mut a, "y1", fmt_mm(w.y1));
            put(&mut a, "x2", fmt_mm(w.x2));
            put(&mut a, "y2", fmt_mm(w.y2));
            put(&mut a, "width", fmt_mm(w.width));
            put(&mut a, "layer", w.layer.to_string());
            if w.curve != 0 {
                put(&mut a, "curve", fmt_mm(w.curve));
            }
            a.extend(w.extra.iter().cloned());
            element("wire", a, "", Vec::new())
        }
        Item::Rect(r) => {
            put(&mut a, "x1", fmt_mm(r.x1));
            put(&mut a, "y1", fmt_mm(r.y1));
            put(&mut a, "x2", fmt_mm(r.x2));
            put(&mut a, "y2", fmt_mm(r.y2));
            put(&mut a, "layer", r.layer.to_string());
            if !r.rot.is_empty() {
                put(&mut a, "rot", r.rot.clone());
            }
            a.extend(r.extra.iter().cloned());
            element("rectangle", a, "", Vec::new())
        }
        Item::Circle(c) => {
            put(&mut a, "x", fmt_mm(c.x));
            put(&mut a, "y", fmt_mm(c.y));
            put(&mut a, "radius", fmt_mm(c.radius));
            put(&mut a, "width", fmt_mm(c.width));
            put(&mut a, "layer", c.layer.to_string());
            a.extend(c.extra.iter().cloned());
            element("circle", a, "", Vec::new())
        }
        Item::Hole(h) => {
            put(&mut a, "x", fmt_mm(h.x));
            put(&mut a, "y", fmt_mm(h.y));
            put(&mut a, "drill", fmt_mm(h.drill));
            a.extend(h.extra.iter().cloned());
            element("hole", a, "", Vec::new())
        }
        Item::Pin(p) => {
            put(&mut a, "name", p.name.clone());
            put(&mut a, "x", fmt_mm(p.x));
            put(&mut a, "y", fmt_mm(p.y));
            if p.visible != "both" {
                put(&mut a, "visible", p.visible.clone());
            }
            if p.length != "long" {
                put(&mut a, "length", p.length.clone());
            }
            if p.direction != "io" {
                put(&mut a, "direction", p.direction.clone());
            }
            if p.function != "none" {
                put(&mut a, "function", p.function.clone());
            }
            if p.swaplevel != "0" {
                put(&mut a, "swaplevel", p.swaplevel.clone());
            }
            if !p.rot.is_empty() {
                put(&mut a, "rot", p.rot.clone());
            }
            a.extend(p.extra.iter().cloned());
            element("pin", a, "", Vec::new())
        }
        Item::Text(t) => {
            put(&mut a, "x", fmt_mm(t.x));
            put(&mut a, "y", fmt_mm(t.y));
            put(&mut a, "size", fmt_mm(t.size));
            put(&mut a, "layer", t.layer.to_string());
            if !t.rot.is_empty() {
                put(&mut a, "rot", t.rot.clone());
            }
            a.extend(t.extra.iter().cloned());
            element("text", a, &t.content, Vec::new())
        }
        Item::Other(e) => e.clone(),
    }
}

fn named(name: &str, extra: &Attrs) -> Attrs {
    let mut a = vec![("name".to_owned(), name.to_owned())];
    a.extend(extra.iter().cloned());
    a
}

fn package_element(p: &Package) -> Element {
    let mut children = p.description.clone();
    children.extend(p.items.iter().map(item_element));
    element("package", named(&p.name, &p.extra), "", children)
}

fn symbol_element(s: &Symbol) -> Element {
    let mut children = s.description.clone();
    children.extend(s.items.iter().map(item_element));
    element("symbol", named(&s.name, &s.extra), "", children)
}

fn deviceset_element(d: &DeviceSet) -> Element {
    let mut attrs = Attrs::new();
    put(&mut attrs, "name", d.name.clone());
    put(&mut attrs, "prefix", d.prefix.clone());
    if d.uservalue {
        put(&mut attrs, "uservalue", yes_no(true));
    }
    attrs.extend(d.extra.iter().cloned());
    let mut children = d.description.clone();
    let gates = d
        .gates
        .iter()
        .map(|g| {
            let mut a = Attrs::new();
            put(&mut a, "name", g.name.clone());
            put(&mut a, "symbol", g.symbol.clone());
            put(&mut a, "x", fmt_mm(g.x));
            put(&mut a, "y", fmt_mm(g.y));
            a.extend(g.extra.iter().cloned());
            element("gate", a, "", Vec::new())
        })
        .collect();
    children.push(element("gates", Attrs::new(), "", gates));
    let devices = d
        .devices
        .iter()
        .map(|dev| {
            let mut a = Attrs::new();
            put(&mut a, "name", dev.name.clone());
            if let Some(package) = &dev.package {
                put(&mut a, "package", package.clone());
            }
            a.extend(dev.extra.iter().cloned());
            let mut kids = Vec::new();
            if !dev.connects.is_empty() {
                let connects = dev
                    .connects
                    .iter()
                    .map(|c| {
                        let mut b = Attrs::new();
                        put(&mut b, "gate", c.gate.clone());
                        put(&mut b, "pin", c.pin.clone());
                        put(&mut b, "pad", c.pad.clone());
                        b.extend(c.extra.iter().cloned());
                        element("connect", b, "", Vec::new())
                    })
                    .collect();
                kids.push(element("connects", Attrs::new(), "", connects));
            }
            kids.extend(dev.rest.iter().cloned());
            element("device", a, "", kids)
        })
        .collect();
    children.push(element("devices", Attrs::new(), "", devices));
    children.extend(d.rest.iter().cloned());
    element("deviceset", attrs, "", children)
}

/// Puts the children of a section of the library in its element, making the element when the
/// file had none and there is something to put in it.
fn place_section(library: &mut Element, name: &str, children: Vec<Element>) {
    if let Some(section) = library.children.iter_mut().find(|c| c.name == name) {
        section.children = children;
        return;
    }
    if children.is_empty() {
        return;
    }
    const ORDER: [&str; 3] = ["packages", "symbols", "devicesets"];
    let rank = ORDER.iter().position(|n| *n == name).unwrap_or(0);
    let at = library
        .children
        .iter()
        .position(|c| {
            ORDER
                .iter()
                .position(|n| *n == c.name)
                .is_some_and(|r| r > rank)
        })
        .unwrap_or(library.children.len());
    library
        .children
        .insert(at, element(name, Attrs::new(), "", children));
}

/// The tree of the file of a library.
pub fn to_tree(lib: &Library) -> Element {
    let mut root = lib.skeleton.clone();
    let packages: Vec<Element> = lib.packages.iter().map(|p| package_element(p)).collect();
    let symbols: Vec<Element> = lib.symbols.iter().map(|s| symbol_element(s)).collect();
    let sets: Vec<Element> = lib
        .devicesets
        .iter()
        .map(|d| deviceset_element(d))
        .collect();
    if let Some(drawing) = root.children.iter_mut().find(|c| c.name == "drawing") {
        if let Some(library) = drawing.children.iter_mut().find(|c| c.name == "library") {
            place_section(library, "packages", packages);
            place_section(library, "symbols", symbols);
            place_section(library, "devicesets", sets);
        }
    }
    root
}

/// The text of the file of a library. The same library always gives the same text.
pub fn to_text(lib: &Library) -> String {
    write_document(&to_tree(lib))
}

/// Writes the library, reads the text back and proves that the library read is the library
/// written and that writing it again gives the same text. Returns the text.
pub fn verify_round_trip(lib: &Library) -> Result<String, LibError> {
    let text = to_text(lib);
    let back = read_library(text.as_bytes(), &Limits::default())
        .map_err(|e| LibError::new(ROUND_TRIP, e.to_string()))?;
    if let Some(difference) = first_difference(lib, &back) {
        return Err(LibError::new(ROUND_TRIP, difference));
    }
    if to_text(&back) != text {
        return Err(LibError::new(
            ROUND_TRIP,
            "writing the library read back gives another text",
        ));
    }
    Ok(text)
}

/// What differs first between two libraries, as a sentence; `None` when they are the same in
/// everything that the model holds.
pub fn first_difference(a: &Library, b: &Library) -> Option<String> {
    if a.packages.len() != b.packages.len() {
        return Some(format!(
            "{} footprints written, {} read",
            a.packages.len(),
            b.packages.len()
        ));
    }
    if a.symbols.len() != b.symbols.len() {
        return Some(format!(
            "{} symbols written, {} read",
            a.symbols.len(),
            b.symbols.len()
        ));
    }
    if a.devicesets.len() != b.devicesets.len() {
        return Some(format!(
            "{} device sets written, {} read",
            a.devicesets.len(),
            b.devicesets.len()
        ));
    }
    for (x, y) in a.packages.iter().zip(b.packages.iter()) {
        if x != y {
            return Some(format!("footprint {} differs", x.name));
        }
    }
    for (x, y) in a.symbols.iter().zip(b.symbols.iter()) {
        if x != y {
            return Some(format!("symbol {} differs", x.name));
        }
    }
    for (x, y) in a.devicesets.iter().zip(b.devicesets.iter()) {
        if x != y {
            return Some(format!("device set {} differs", x.name));
        }
    }
    None
}

/// The SHA-256 of some bytes as lower-case hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// What was done by [`save_new`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// The file written.
    pub path: PathBuf,
    /// Its size in bytes.
    pub bytes: usize,
    /// Its SHA-256.
    pub sha256: String,
    /// The copy of the original that was made, when there was an original.
    pub backup: Option<PathBuf>,
}

/// The name of the copy of an original: `<stem>.backup-<first 8 digits of the SHA-256>.lbr`.
pub fn backup_name(original: &Path, digest: &str) -> String {
    let stem = original
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "library".to_owned());
    let short: String = digest.chars().take(8).collect();
    format!("{stem}.backup-{short}.lbr")
}

/// Copies `original` into `folder` under the name of [`backup_name`], unless a file of that name
/// is there already (it holds the same bytes, because the name carries their digest).
pub fn make_backup(original: &Path, folder: &Path) -> Result<PathBuf, LibError> {
    let fail = |e: std::io::Error| LibError::new(BACKUP_FAILED, e.to_string());
    let bytes = fs::read(original).map_err(fail)?;
    let target = folder.join(backup_name(original, &sha256_hex(&bytes)));
    if !target.exists() {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(fail)?;
        file.write_all(&bytes).map_err(fail)?;
        file.sync_all().map_err(fail)?;
    }
    Ok(target)
}

/// Saves the library as a new file. The round trip is proved before anything is written, a file
/// that exists is never replaced (so the input is never overwritten) and, when there is an
/// original, a copy of it is kept next to the new file. After writing, the file is read from the
/// disk and compared once more.
pub fn save_new(lib: &Library, original: Option<&Path>, out: &Path) -> Result<Saved, LibError> {
    let fail = |e: std::io::Error| LibError::new(WRITE_FAILED, e.to_string());
    let text = verify_round_trip(lib)?;
    if out.exists() {
        return Err(LibError::new(
            WRITE_FAILED,
            "the target exists and is never replaced",
        ));
    }
    let folder = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let backup = match original {
        Some(original) if original.exists() => Some(make_backup(original, &folder)?),
        _ => None,
    };
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out)
            .map_err(fail)?;
        file.write_all(text.as_bytes()).map_err(fail)?;
        file.sync_all().map_err(fail)?;
    }
    let on_disk = fs::read(out).map_err(fail)?;
    if on_disk != text.as_bytes() {
        return Err(LibError::new(
            ROUND_TRIP,
            "the file on the disk is not the text that was written",
        ));
    }
    let back = read_library(&on_disk, &Limits::default())
        .map_err(|e| LibError::new(ROUND_TRIP, e.to_string()))?;
    if let Some(difference) = first_difference(lib, &back) {
        return Err(LibError::new(ROUND_TRIP, difference));
    }
    Ok(Saved {
        path: out.to_path_buf(),
        bytes: on_disk.len(),
        sha256: sha256_hex(&on_disk),
        backup,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::tests::{sample, SAMPLE};

    #[test]
    fn lengths_are_written_as_the_shortest_exact_decimal() {
        assert_eq!(fmt_mm(0), "0");
        assert_eq!(fmt_mm(1_270_000), "1.27");
        assert_eq!(fmt_mm(-50_000), "-0.05");
        assert_eq!(fmt_mm(3_000_000), "3");
        assert_eq!(fmt_mm(1), "0.000001");
        for v in [0, 1, -1, 999_999, 1_234_567, -7_000_000, 1 << 29] {
            assert_eq!(crate::read::exact_millionths(&fmt_mm(v)), Some(v));
        }
    }

    #[test]
    fn the_sample_reads_back_the_same_after_writing() {
        let lib = sample();
        let text = verify_round_trip(&lib).unwrap();
        assert!(text.starts_with("<?xml"));
        assert!(text.contains("<polygon"));
        assert!(text.contains("<description>Test library</description>"));
        let again = read_library(text.as_bytes(), &Limits::default()).unwrap();
        assert_eq!(first_difference(&lib, &again), None);
        assert_eq!(to_text(&again), text);
    }

    #[test]
    fn the_same_library_always_gives_the_same_text() {
        assert_eq!(to_text(&sample()), to_text(&sample()));
        assert!(SAMPLE.contains("R0603"));
    }

    #[test]
    fn a_section_that_the_file_did_not_have_is_made_in_its_place() {
        let mut lib = Library::empty();
        lib.symbols.push(std::sync::Arc::new(Symbol {
            name: "S".into(),
            ..Symbol::default()
        }));
        lib.packages.push(std::sync::Arc::new(Package {
            name: "P".into(),
            ..Package::default()
        }));
        let tree = to_tree(&lib);
        let library = tree
            .child("drawing")
            .and_then(|d| d.child("library"))
            .unwrap();
        let names: Vec<&str> = library.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["packages", "symbols"]);
        assert!(verify_round_trip(&lib).is_ok());
    }

    #[test]
    fn saving_makes_a_new_file_and_a_copy_of_the_original_and_never_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("in.lbr");
        fs::write(&original, SAMPLE).unwrap();
        let lib = sample();
        let out = dir.path().join("out.lbr");
        let saved = save_new(&lib, Some(&original), &out).unwrap();
        assert_eq!(saved.bytes, fs::read(&out).unwrap().len());
        assert_eq!(saved.sha256.len(), 64);
        let backup = saved.backup.clone().unwrap();
        assert_eq!(fs::read(&backup).unwrap(), SAMPLE.as_bytes());
        assert!(backup
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("in.backup-"));
        // the second time the target exists, and so does the input
        assert_eq!(
            save_new(&lib, Some(&original), &out).unwrap_err().code,
            WRITE_FAILED
        );
        assert_eq!(
            save_new(&lib, None, &original).unwrap_err().code,
            WRITE_FAILED
        );
        assert_eq!(fs::read(&original).unwrap(), SAMPLE.as_bytes());
    }

    #[test]
    fn a_library_that_does_not_read_back_is_not_written() {
        let mut lib = sample();
        // a coordinate beyond what the file can say exactly: the XML writer is exact, so break
        // the tree instead, with an element name that the reader refuses to take as a package
        let mut skeleton = lib.skeleton.clone();
        skeleton.name = "other".to_owned();
        lib.skeleton = skeleton;
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("x.lbr");
        let e = save_new(&lib, None, &out).unwrap_err();
        assert_eq!(e.code, ROUND_TRIP);
        assert!(!out.exists());
    }

    #[test]
    fn the_backup_is_made_once() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("a.lbr");
        fs::write(&original, b"abc").unwrap();
        let first = make_backup(&original, dir.path()).unwrap();
        let second = make_backup(&original, dir.path()).unwrap();
        assert_eq!(first, second);
        assert!(first.exists());
        assert_eq!(
            first.file_name().unwrap().to_string_lossy(),
            "a.backup-ba7816bf.lbr"
        );
    }
}
