//! PortFinder GUI entry point — see `src/lib.rs` for the shared
//! crate code and `src/bin/portfinder-cli.rs` for the matching
//! console-subsystem CLI binary used on Windows.

// The `windows` subsystem suppresses the console window that Rust's
// default `console` subsystem pops up alongside the GUI when a user
// double-clicks PortFinder.exe from File Explorer. Debug builds keep
// the console attached (the standard subsystem doesn't allocate a
// fresh one when launched from a terminal), so `cargo run` on
// Windows still surfaces stdout/stderr in the launching shell. The
// trade-off is that a windows-subsystem binary launched from
// PowerShell can't cleanly print to the parent shell's stdio (the
// shell fire-and-forgets us), which is why CLI usage on Windows is
// routed through the matching `portfinder-cli.exe` binary instead.
// See `lib.rs` for the full dual-binary rationale. No-op on
// non-Windows targets.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use clap::Parser;
use portfinder::{app_view, cli, init_logging};

fn main() {
    init_logging();

    // Filter out macOS bundle launch noise (-psn_xxx). What's left
    // decides whether the user invoked us as a CLI or just double-
    // clicked the .app.
    let args: Vec<String> = std::env::args()
        .filter(|a| !a.starts_with("-psn_"))
        .collect();

    if args.len() <= 1 {
        // No CLI args → launch the GUI.
        app_view::run();
        return;
    }

    attach_parent_console();

    // Has args → CLI mode. Works as-expected on Linux / macOS where
    // there is no subsystem distinction. On Windows, the shell has
    // already fire-and-forgotten this process by the time output
    // reaches the attached console, so the output prints after the
    // prompt has redrawn — usable but ugly. The packaged Windows
    // installer ships a separate `portfinder-cli.exe` (console
    // subsystem) that doesn't have this UX gap; CLI users on
    // Windows should reach for that one instead of `PortFinder.exe`.
    // The fallback path here remains so that `cargo run --
    // capture …` still works during local development.
    let cli_args = match cli::Cli::try_parse_from(&args) {
        Ok(c) => c,
        Err(e) => e.exit(),
    };
    std::process::exit(cli::run(cli_args));
}

/// On Windows, the GUI subsystem detaches from the console. Without
/// this, CLI output written to stdout/stderr is invisible to the
/// parent shell. No-op on Linux/macOS.
fn attach_parent_console() {
    #[cfg(windows)]
    {
        const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
        unsafe extern "system" {
            fn AttachConsole(dwProcessId: u32) -> i32;
        }
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}
