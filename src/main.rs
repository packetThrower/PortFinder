//! PortFinder — network switch port discovery via CDP/LLDP/MNDP.
//!
//! Binary crate. Dispatches CLI vs GUI based on argv: a subcommand
//! (e.g. `portfinder capture --protocol lldp`) runs the headless CLI
//! and exits; no arguments (or only macOS bundle noise like `-psn_*`)
//! opens the gpui window.
//!
//! 3.x of this project was Tauri (Rust backend + Svelte frontend);
//! 4.x is pure Rust with Zed's gpui as the UI framework, the same
//! shape Baudrun uses. The CDP / LLDP / MNDP parsers and the
//! privilege / BPF helper modules port across unchanged — only the
//! UI layer and the build pipeline are different.

// The `windows` subsystem suppresses the console window that Rust's
// default `console` subsystem pops up alongside the GUI when a user
// double-clicks PortFinder.exe from File Explorer. Debug builds keep
// the console attached (the standard subsystem doesn't allocate a
// fresh one when launched from a terminal), so `cargo run` on
// Windows still surfaces stdout/stderr in the launching shell. The
// user-visible regression is only on installed Win + double-click,
// which this fixes. No-op on non-Windows targets.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app_view;
mod capture;
pub mod cli;
mod privilege;

use clap::Parser;
use serde::{Deserialize, Serialize};

/// Discovered network interface as surfaced to the UI / CLI. The
/// JSON / serde shape stays camelCase (Interface picker, JSON CLI
/// output) so external scripts that fed off the 3.x Tauri bindings
/// keep working without changes.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInfo {
    pub name: String,
    pub description: String,
    pub addresses: String,
    pub has_ip: bool,
}

/// Capture request payload. Matches the 3.x camelCase wire format.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub interface_name: String,
    pub protocol: String,
}

/// One CDP / LLDP / MNDP packet's worth of decoded switch info.
/// Parser modules fill missing fields with the literal `"N/A"`
/// sentinel; the GUI renders that as a faded "not advertised" cell.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub switch_name: String,
    pub switch_ip: String,
    pub switch_port: String,
    pub native_vlan: String,
    pub voice_vlan: String,
    pub mtu: String,
    pub switch_model: String,
}

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

    // Has args → CLI mode. clap handles --help, --version, unknown
    // subcommands, etc. and exits with the appropriate status itself.
    let cli = match cli::Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };
    std::process::exit(cli::run(cli));
}

/// Bring up env_logger with `info` as the default level and the
/// output piped to `~/Desktop/portfinder-debug.log` (created /
/// appended). The desktop target makes it trivial for testers to
/// find the log without spelunking through Application Support, and
/// the `RUST_LOG` env var still overrides everything for users who
/// want to crank it up or down.
///
/// Logging to a file rather than stderr matters in production: the
/// GUI subsystem on Windows detaches from the console, and the
/// .app bundle on macOS launches with no stderr stream you can read
/// after the fact. A desktop file is the friendliest "always there,
/// always inspectable" target.
fn init_logging() {
    use env_logger::{Builder, Target};
    use std::fs::OpenOptions;

    let mut builder = Builder::new();
    // Honour RUST_LOG if it's set; otherwise default to `info`. This
    // is what `Builder::from_env(Env::default().default_filter_or)`
    // does inline.
    if let Ok(filter) = std::env::var("RUST_LOG") {
        builder.parse_filters(&filter);
    } else {
        builder.filter_level(log::LevelFilter::Info);
    }

    if let Some(path) = desktop_log_path() {
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
            builder.target(Target::Pipe(Box::new(file)));
        }
    }
    builder.init();

    log::info!(
        "PortFinder v{} starting (target_os={})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
}

/// Resolve `~/Desktop/portfinder-debug.log` on macOS / Linux /
/// Windows without pulling in a dirs crate. Returns None if neither
/// `HOME` nor `USERPROFILE` is set — in that case the logger falls
/// back to stderr (which still works for `cargo run`).
fn desktop_log_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let home = std::env::var("USERPROFILE").ok()?;
    #[cfg(not(windows))]
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::PathBuf::from(home)
            .join("Desktop")
            .join("portfinder-debug.log"),
    )
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
