// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed errors of the inventory.

use std::path::PathBuf;

/// What can stop an inventory.
#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    /// A configuration line is not `key = value`.
    #[error("configuration line {line}: expected `key = value`")]
    ConfigSyntax {
        /// 1-based line number.
        line: usize,
    },
    /// A configuration key this version does not know.
    #[error("configuration line {line}: unknown key `{key}`")]
    ConfigUnknownKey {
        /// 1-based line number.
        line: usize,
        /// The key as written.
        key: String,
    },
    /// A configuration value that cannot be used.
    #[error("configuration line {line}: bad value for `{key}`")]
    ConfigBadValue {
        /// 1-based line number.
        line: usize,
        /// The key as written.
        key: String,
    },
    /// A bound of the configuration was exceeded; the inventory stops rather than guess.
    #[error("limit exceeded: {what}")]
    LimitExceeded {
        /// Which bound.
        what: &'static str,
    },
    /// A folder or file could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The path as the caller gave it, or below it.
        path: PathBuf,
        /// The operating system error.
        #[source]
        source: std::io::Error,
    },
}
