//! Console-subsystem CLI entry point. See `src/lib.rs` for the
//! dual-binary rationale on Windows.
//!
//! **No `windows_subsystem` attribute on this file.** That's the
//! whole point — the linker writes `IMAGE_SUBSYSTEM_WINDOWS_CUI`
//! into the PE header, the kernel keeps PowerShell / cmd.exe
//! attached and waiting on the process, and stdio routes through
//! to the parent terminal the way users expect for a CLI. The
//! companion `PortFinder.exe` (built from `src/main.rs`) uses
//! `windows_subsystem = "windows"` so File Explorer double-clicks
//! don't pop up a black console window next to the GUI window.
//!
//! On Linux and macOS the subsystem byte doesn't exist; this
//! binary still builds and runs but is functionally equivalent to
//! `PortFinder capture …` on those platforms. The .deb / .rpm /
//! .pacman / .app bundles don't ship it — only the Windows
//! installer does.

use clap::Parser;
use portfinder::{cli, init_logging};

fn main() {
    init_logging();

    let cli_args = match cli::Cli::try_parse() {
        Ok(c) => c,
        Err(e) => e.exit(),
    };
    std::process::exit(cli::run(cli_args));
}
