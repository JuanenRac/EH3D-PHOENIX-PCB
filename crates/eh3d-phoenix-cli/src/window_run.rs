// SPDX-License-Identifier: GPL-3.0-or-later
//! The window of the program `phoenix`: it opens the viewer and editor, with the start screen when
//! no file is named. It is built with `--features gui`. On Windows the program is one of the
//! graphical subsystem, so no console window opens behind the window; when the program was started
//! from a terminal it writes there (see `main.rs`), and an error that keeps the window from opening
//! is also written to `last_error.txt` in the settings folder of the user, so it is found when there
//! was no terminal.
//!
//! With `--shot FILE.png` the program only takes a picture of the window and closes: it reads no
//! file of preferences, writes none and leaves no `last_error.txt`, so a reason for a failure is
//! written to the error stream only.

use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_viewer::launcher_recent::{config_dir, ERROR_FILE};
use eh3d_phoenix_viewer::launcher_session::lang_from_env;
use eh3d_phoenix_viewer::{gui, ViewerSettings};

use crate::window_args::parse_window_args;

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Writes the reason the window did not open where the owner of the computer can find it. In the
/// picture mode nothing is written to the settings folder.
fn report_error(message: &str, picture_mode: bool, err: &mut dyn Write) {
    let _ = writeln!(err, "error: {message}");
    if picture_mode {
        return;
    }
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
    let text = crate::bounded::read_config_text(&path)
        .map_err(|e| format!("cannot read the settings {}: {e}", path.display()))?;
    ViewerSettings::parse(&text).map_err(|e| format!("{}: {}", path.display(), e.message(Lang::En)))
}

/// Opens the window for the options in `args` (the arguments after the program name, without the
/// word `view` when it named the window). Returns the exit code: 0 when the window closed, 1 when
/// it could not run and 2 for a mistake in the options or the settings.
pub fn run_window(args: &[String], err: &mut dyn Write) -> i32 {
    let parsed = match parse_window_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            report_error(&message, args.iter().any(|a| a == "--shot"), err);
            return 2;
        }
    };
    let picture_mode = parsed.shot.is_some();
    let settings = match load_settings(parsed.settings.as_deref()) {
        Ok(settings) => settings,
        Err(message) => {
            report_error(&message, picture_mode, err);
            return 2;
        }
    };
    let lang = parsed
        .lang
        .or_else(|| lang_from_env(&env))
        .unwrap_or(Lang::En);
    let outcome = match parsed.shot {
        Some(request) => gui::run_shot(parsed.file, lang, settings, request),
        None => gui::run(parsed.file, lang, settings),
    };
    match outcome {
        Ok(()) => 0,
        Err(error) => {
            report_error(&error.message(lang), picture_mode, err);
            1
        }
    }
}
