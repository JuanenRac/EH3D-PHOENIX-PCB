// SPDX-License-Identifier: GPL-3.0-or-later
//! The `phoenix` program.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = eh3d_phoenix_cli::run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code);
}
