// SPDX-License-Identifier: GPL-3.0-or-later
//! Writing the files to a place that is new: a folder that does not exist yet, or one file that
//! does not exist yet. Nothing that exists is ever replaced.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::codes::{PrintError, OUTPUT_REFUSED, WRITE_FAILED};
use crate::render::Rendered;

fn has_extension(path: &Path, wanted: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(wanted))
}

/// The paths that the files would be written to, after checking that the place is allowed: a path
/// that ends in `.svg` or `.pdf` is one file (the format and the number of files must agree);
/// any other path is a folder. Nothing is created and nothing that exists is accepted.
pub fn planned_paths(out: &Path, rendered: &Rendered) -> Result<Vec<PathBuf>, PrintError> {
    let refuse = |why: String| PrintError::new(OUTPUT_REFUSED, why);
    if out.exists() {
        return Err(refuse(format!("{} exists", out.display())));
    }
    let file_target = has_extension(out, "svg") || has_extension(out, "pdf");
    if file_target {
        let [_] = rendered.files.as_slice() else {
            return Err(refuse(format!(
                "{} holds {} files, so a folder is needed",
                out.display(),
                rendered.files.len()
            )));
        };
        let wanted = rendered.format.extension();
        if !has_extension(out, wanted) {
            return Err(refuse(format!(
                "{} does not end in .{wanted}, the format asked",
                out.display()
            )));
        }
        Ok(vec![out.to_path_buf()])
    } else {
        Ok(rendered.files.iter().map(|f| out.join(&f.name)).collect())
    }
}

/// Writes the files, each to a path that did not exist, and returns the paths.
pub fn write_new(out: &Path, rendered: &Rendered) -> Result<Vec<PathBuf>, PrintError> {
    let paths = planned_paths(out, rendered)?;
    let failed = |e: std::io::Error, path: &Path| {
        PrintError::new(WRITE_FAILED, format!("{}: {e}", path.display()))
    };
    let folder = !(has_extension(out, "svg") || has_extension(out, "pdf"));
    let parent = if folder { Some(out) } else { out.parent() };
    if let Some(dir) = parent.filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| failed(e, dir))?;
    }
    for (path, file) in paths.iter().zip(&rendered.files) {
        let mut handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| failed(e, path))?;
        handle.write_all(&file.bytes).map_err(|e| failed(e, path))?;
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Format, RenderedFile};

    fn rendered(format: Format, names: &[&str]) -> Rendered {
        Rendered {
            format,
            files: names
                .iter()
                .map(|n| RenderedFile {
                    name: (*n).to_owned(),
                    bytes: n.as_bytes().to_vec(),
                    pages: 1,
                })
                .collect(),
            pages: Vec::new(),
            boxed_chars: 0,
        }
    }

    #[test]
    fn a_folder_that_is_new_gets_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("new").join("prints");
        let r = rendered(Format::Svg, &["a.svg", "b.svg"]);
        let planned = planned_paths(&out, &r).unwrap();
        assert!(!out.exists());
        let written = write_new(&out, &r).unwrap();
        assert_eq!(planned, written);
        assert_eq!(std::fs::read(out.join("b.svg")).unwrap(), b"b.svg");
        // a second time it exists and is refused, and nothing changes
        let error = write_new(&out, &r).unwrap_err();
        assert_eq!(error.code, OUTPUT_REFUSED);
        assert_eq!(std::fs::read(out.join("a.svg")).unwrap(), b"a.svg");
    }

    #[test]
    fn one_file_needs_its_extension_and_one_file() {
        let dir = tempfile::tempdir().unwrap();
        let one = rendered(Format::Pdf, &["d.pdf"]);
        let out = dir.path().join("x.PDF");
        assert_eq!(write_new(&out, &one).unwrap(), vec![out.clone()]);
        assert_eq!(std::fs::read(&out).unwrap(), b"d.pdf");
        assert_eq!(
            write_new(&out, &one).unwrap_err().code,
            OUTPUT_REFUSED,
            "an existing file is never replaced"
        );
        let wrong = dir.path().join("y.svg");
        assert_eq!(
            planned_paths(&wrong, &one).unwrap_err().code,
            OUTPUT_REFUSED
        );
        let two = rendered(Format::Svg, &["a.svg", "b.svg"]);
        let single = dir.path().join("z.svg");
        assert_eq!(
            planned_paths(&single, &two).unwrap_err().code,
            OUTPUT_REFUSED
        );
        assert!(!single.exists());
    }
}
