// SPDX-License-Identifier: GPL-3.0-or-later
//! The `phoenix` program: the command line and, in a build with the `gui` feature, the window.
//!
//! With no arguments the window opens (the help is printed in a build without it). A command of the
//! command line runs on the console with the same output and exit codes as always. The options of
//! the window (`--open`, `--shot`, `--window`, ...) and a file name start the window, and
//! `phoenix view FILE` opens it on a file; `launch.rs` holds the decision and `window_run.rs` the
//! window.
//!
//! On Windows a build with the window is a program of the graphical subsystem, so no console window
//! opens behind the window. For the commands the program attaches itself to the console of the
//! process that started it, so the output shows in a terminal and, when the output is redirected or
//! piped, in the file or the pipe, as it always did.
#![forbid(unsafe_code)]
#![cfg_attr(all(windows, feature = "gui"), windows_subsystem = "windows")]

use eh3d_phoenix_cli::launch::{choose_mode, Mode};

#[cfg(feature = "gui")]
fn run_window(args: &[String]) -> i32 {
    eh3d_phoenix_cli::window_run::run_window(args, &mut std::io::stderr())
}

#[cfg(not(feature = "gui"))]
fn run_window(_args: &[String]) -> i32 {
    // `choose_mode` never asks for the window in a build without it
    2
}

fn main() {
    // a no-op unless this is a program of the graphical subsystem of Windows that has a parent console
    eh3d_phoenix_console::attach_to_parent();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let is_file = |word: &str| std::path::Path::new(word).is_file();
    let code = match choose_mode(&args, cfg!(feature = "gui"), &is_file) {
        Mode::Window { skip } => run_window(&args[skip..]),
        Mode::Help => eh3d_phoenix_cli::run(
            &["--help".to_owned()],
            &mut std::io::stdout(),
            &mut std::io::stderr(),
        ),
        Mode::Cli => eh3d_phoenix_cli::run(&args, &mut std::io::stdout(), &mut std::io::stderr()),
    };
    std::process::exit(code);
}
