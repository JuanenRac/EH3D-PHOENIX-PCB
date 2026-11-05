// SPDX-License-Identifier: GPL-3.0-or-later
//! `phoenix-gui` opens the window of the viewer and editor, with the start screen when no file
//! is named. It is built with `--features gui`. On Windows it is a program of the graphical
//! subsystem, so no console window opens behind it; for that reason it prints nothing there, and
//! an error that keeps the window from opening is written to `last_error.txt` in the settings
//! folder of the user. The command line (`phoenix`) keeps its console, so its output and its
//! exit codes work in a terminal and in a script.
#![forbid(unsafe_code)]
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use eh3d_phoenix_cli::window_args::parse_window_args;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_viewer::launcher_recent::{config_dir, ERROR_FILE};
use eh3d_phoenix_viewer::launcher_session::lang_from_env;
use eh3d_phoenix_viewer::{gui, ViewerSettings};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Writes the reason the window did not open where the owner of the computer can find it.
fn report_error(message: &str) {
    #[cfg(not(windows))]
    eprintln!("error: {message}");
    if let Some(dir) = config_dir(&env) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(ERROR_FILE), format!("{message}\n"));
    }
}

/// The settings file named on the command line, or `config/viewer.cfg` beside the program when
/// there is one, or the defaults.
fn load_settings(named: Option<&Path>) -> Result<ViewerSettings, String> {
    let path: Option<PathBuf> = match named {
        Some(path) => Some(path.to_path_buf()),
        None => std::env::current_exe()
            .ok()
            .and_then(|exe| {
                exe.parent()
                    .map(|dir| dir.join("config").join("viewer.cfg"))
            })
            .filter(|path| path.is_file()),
    };
    let Some(path) = path else {
        return Ok(ViewerSettings::default());
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read the settings {}: {e}", path.display()))?;
    ViewerSettings::parse(&text).map_err(|e| format!("{}: {}", path.display(), e.message(Lang::En)))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_window_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            report_error(&message);
            return ExitCode::from(2);
        }
    };
    let settings = match load_settings(parsed.settings.as_deref()) {
        Ok(settings) => settings,
        Err(message) => {
            report_error(&message);
            return ExitCode::from(2);
        }
    };
    let lang = parsed
        .lang
        .or_else(|| lang_from_env(&env))
        .unwrap_or(Lang::En);
    match gui::run(parsed.file, lang, settings) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report_error(&error.message(lang));
            ExitCode::from(1)
        }
    }
}
