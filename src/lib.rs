//! PortFinder library crate. Hosts the modules and shared
//! types that both binary entry points consume.
//!
//! ## Two binaries on Windows
//!
//! Windows assigns each `.exe` a single "subsystem" byte in
//! the PE header at link time. The kernel reads that byte
//! before `main()` runs and behaves accordingly:
//!
//! - **`IMAGE_SUBSYSTEM_WINDOWS_GUI`** ("windows"): no console
//!   allocated on launch; the shell fire-and-forgets the process
//!   the moment it spawns. Right for File Explorer double-click
//!   (no console window flashes up next to the GUI).
//! - **`IMAGE_SUBSYSTEM_WINDOWS_CUI`** ("console"): kernel
//!   inherits / allocates a console; cmd.exe / PowerShell *wait*
//!   for the process to exit before redrawing the prompt. Right
//!   for CLI usage (`PortFinder --help` actually prints help and
//!   returns control cleanly).
//!
//! A single binary has to pick one. There's no runtime override
//! — `AttachConsole` can reach stdio after the fact (modern Rust
//! stdlib doesn't cache the handles, [rust-lang/rust#40490]) but
//! the shell has already moved on by then, so `--help` output
//! prints *after* the next prompt has redrawn. The fix is two
//! binaries with two different PE subsystem bytes:
//!
//! - `src/main.rs` → `PortFinder.exe` (windows subsystem)
//! - `src/bin/portfinder-cli.rs` → `portfinder-cli.exe` (console
//!   subsystem)
//!
//! Both link to this library, so the CLI logic, capture
//! orchestration, parsers, and privilege detection are shared.
//! Each binary file is otherwise as thin as possible — its
//! whole reason to exist is the PE subsystem byte the linker
//! writes for it.
//!
//! On Linux and macOS the subsystem distinction doesn't exist,
//! so the GUI binary handles both CLI and GUI invocations via
//! argv dispatch. The `portfinder-cli` binary is built on those
//! platforms too (to keep the build pipeline single-shape) but
//! the .deb / .rpm / .pacman / .app bundles don't ship it.
//!
//! [rust-lang/rust#40490]: https://github.com/rust-lang/rust/pull/40490

pub mod app_view;
pub mod capture;
pub mod cli;
pub mod privilege;
pub mod updater;

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
pub fn init_logging() {
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
pub fn desktop_log_path() -> Option<std::path::PathBuf> {
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
