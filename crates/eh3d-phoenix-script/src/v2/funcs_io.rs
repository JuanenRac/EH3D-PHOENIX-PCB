// SPDX-License-Identifier: GPL-3.0-or-later
//! The only way a script of the second version reaches a file: read a text file from the data
//! folder or ask to write a new one there. The data folder is `data_dir` of the settings, below the
//! folder of the script. A path must be relative, without a drive, a root or `..`, and without a
//! link on the way; a file that exists is never replaced; the number and the size of the files are
//! bounded; and a write is only noted, to be made when the run is applied.

use std::path::PathBuf;

use crate::codes::Diag;
use crate::files;
use crate::v2::codes::{DATA_LIMIT, DATA_PATH, MEMORY_LIMIT};
use crate::v2::funcs::{def, text, FuncDef};
use crate::v2::interp::{Interp, PendingFile};
use crate::v2::value::Value;

type R = Result<Value, Diag>;

fn data_path(i: &mut Interp<'_>, relative: &str) -> Result<PathBuf, Diag> {
    let s = i.ctx.settings;
    if s.data_dir.is_empty() {
        return Err(Diag::plain(
            DATA_PATH,
            "no data folder is set (data_dir in config/script.cfg)",
        ));
    }
    let Some(base) = i.ctx.base_dir else {
        return Err(Diag::plain(DATA_PATH, "there is no folder to start from"));
    };
    let plain = !relative.is_empty()
        && !relative.starts_with(['/', '\\'])
        && !relative.contains([':', '\0'])
        && relative.split(['/', '\\']).all(|part| part != "..");
    if !plain {
        return Err(Diag::plain(
            DATA_PATH,
            format!("{relative:?} is not a relative path inside the data folder"),
        ));
    }
    let full = format!("{}/{relative}", s.data_dir.trim_end_matches(['/', '\\']));
    let mut inside = s.clone();
    inside.allowed_dirs = vec![s.data_dir.clone()];
    let path = files::resolve(Some(base), &full, &inside)
        .map_err(|d| Diag::plain(DATA_PATH, format!("{relative}: {}", d.detail)))?;
    if !i.data_files.contains(&full) {
        if i.data_files.len() >= s.max_data_files {
            return Err(Diag::plain(
                DATA_LIMIT,
                format!("more than {} data files in one run", s.max_data_files),
            ));
        }
        i.data_files.push(full);
    }
    Ok(path)
}

fn f_read_text(i: &mut Interp<'_>, a: &[Value]) -> R {
    let relative = text(a, 0)?.to_owned();
    let path = data_path(i, &relative)?;
    let s = i.ctx.settings;
    let bytes = files::read_limited(&path, s.max_data_bytes)
        .map_err(|d| Diag::plain(DATA_LIMIT, d.detail))?;
    let content = String::from_utf8(bytes)
        .map_err(|_| Diag::plain(DATA_LIMIT, format!("{relative} is not UTF-8 text")))?;
    if content.len() > s.max_text_bytes {
        return Err(Diag::plain(
            MEMORY_LIMIT,
            format!("a text of more than {} bytes", s.max_text_bytes),
        ));
    }
    Ok(Value::Str(content))
}

fn f_write_text(i: &mut Interp<'_>, a: &[Value]) -> R {
    let relative = text(a, 0)?.to_owned();
    let content = text(a, 1)?.to_owned();
    let s = i.ctx.settings;
    if u64::try_from(content.len()).unwrap_or(u64::MAX) > s.max_data_bytes {
        return Err(Diag::plain(
            DATA_LIMIT,
            format!("{relative} would be larger than {} bytes", s.max_data_bytes),
        ));
    }
    let path = data_path(i, &relative)?;
    if std::fs::symlink_metadata(&path).is_ok() {
        return Err(Diag::plain(
            DATA_LIMIT,
            format!("{relative} exists and is never replaced"),
        ));
    }
    if i.writes.iter().any(|w| w.path == path) {
        return Err(Diag::plain(
            DATA_LIMIT,
            format!("{relative} was already written in this run"),
        ));
    }
    let size = content.len();
    i.writes.push(PendingFile {
        relative,
        path,
        bytes: content.into_bytes(),
    });
    i64::try_from(size)
        .map(Value::Int)
        .map_err(|_| Diag::plain(DATA_LIMIT, "the size does not fit 64 bits"))
}

fn f_data_enabled(i: &mut Interp<'_>, _a: &[Value]) -> R {
    Ok(Value::Bool(
        !i.ctx.settings.data_dir.is_empty() && i.ctx.base_dir.is_some(),
    ))
}

/// The functions on data files.
pub(crate) static TABLE: &[FuncDef] = &[
    def("data_enabled", 0, 0, 0, false, "data_enabled()", "Whether the settings give the script a data folder (data_dir) to read and write.", f_data_enabled),
    def("read_text", 1, 1, 0, false, "read_text(path)", "Reads a text file of the data folder (data_dir); an error when there is none, the path is not allowed or the file is too large.", f_read_text),
    def("write_text", 2, 2, 0, false, "write_text(path, text)", "Asks to write a new text file in the data folder; it is written only when the run is applied, and never over a file that exists.", f_write_text),
];
