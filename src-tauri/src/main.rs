// Prevents additional console window on Windows in release when launched
// as a GUI. For CLI mode we re-attach to the parent console explicitly.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;

fn main() {
    // Filter out macOS bundle launch noise (-psn_xxx). What's left tells us
    // whether the user invoked us as a CLI or just double-clicked the app.
    let args: Vec<String> = std::env::args()
        .filter(|a| !a.starts_with("-psn_"))
        .collect();

    if args.len() <= 1 {
        // No CLI args → launch the GUI.
        portfinder_lib::run();
        return;
    }

    attach_parent_console();

    // Has args → CLI mode. clap handles --help, --version, unknown
    // subcommands, etc. and exits with the appropriate status itself.
    let cli = match portfinder_lib::cli::Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };
    std::process::exit(portfinder_lib::cli::run(cli));
}

/// On Windows, the GUI subsystem detaches from the console. Without this,
/// CLI output written to stdout/stderr is invisible to the parent shell.
/// On Linux/macOS this is a no-op.
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
