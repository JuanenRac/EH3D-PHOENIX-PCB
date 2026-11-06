// SPDX-License-Identifier: GPL-3.0-or-later
//! The console of a program of the graphical subsystem of Windows, for EH3D-PHOENIX-PCB.
//!
//! The program `phoenix` is one program for the command line and for the window. On Windows it is
//! built for the graphical subsystem, so that no console window opens behind the window when it is
//! started with no arguments; a program of that subsystem has no console of its own, and
//! [`attach_to_parent`] joins it to the console of the process that started it, so that what the
//! commands print shows in a terminal.
//!
//! This is the only crate of the workspace that has `unsafe` code, in the module `windows` (three
//! calls of `kernel32` with constants and handles of the program, reviewed in
//! `docs/SECURITY_REVIEW.md`). On every other system the function does nothing.
#![deny(unsafe_code)]
#![warn(missing_docs)]

/// Joins the program to the console of the process that started it, when there is one. A standard
/// output or error that the starter gave (a redirection to a file or a pipe) is left as it is; one
/// that is missing is opened on the console. Without a console of the parent nothing happens.
pub fn attach_to_parent() {
    #[cfg(windows)]
    windows::attach_to_parent();
}

#[cfg(windows)]
mod windows {
    use std::os::windows::io::IntoRawHandle;

    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn GetStdHandle(which: u32) -> isize;
        fn SetStdHandle(which: u32, handle: isize) -> i32;
    }

    /// The process that started this one.
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    /// `STD_OUTPUT_HANDLE` is -11 and `STD_ERROR_HANDLE` is -12, as unsigned numbers.
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;

    /// Whether a handle given by `GetStdHandle` is missing (none, or the invalid value).
    pub(super) fn is_missing(handle: isize) -> bool {
        handle == 0 || handle == -1
    }

    #[allow(unsafe_code)]
    pub(super) fn attach_to_parent() {
        // SAFETY: `AttachConsole` takes a constant and does nothing without a console of the
        // parent; `GetStdHandle` and `SetStdHandle` take the constants of the two standard
        // streams, and the handle given to `SetStdHandle` is one that this function opened and
        // keeps open for the life of the program.
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                return;
            }
            for which in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                if !is_missing(GetStdHandle(which)) {
                    continue;
                }
                if let Ok(console) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
                    SetStdHandle(which, console.into_raw_handle() as isize);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn attaching_twice_is_harmless() {
        super::attach_to_parent();
        super::attach_to_parent();
    }

    #[cfg(windows)]
    #[test]
    fn a_missing_handle_is_none_or_invalid() {
        assert!(super::windows::is_missing(0));
        assert!(super::windows::is_missing(-1));
        assert!(!super::windows::is_missing(4));
    }
}
