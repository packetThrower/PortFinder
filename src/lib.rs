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
pub mod settings;
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

/// Wire up `env_logger` once for the process. The actual on/off
/// decision lives in the `settings::LogPipe` pipe + the
/// `settings::set_logging_enabled` swap, not in this
/// initialisation — env_logger can only be initialised once per
/// process, so we always install the pipe and let the pipe drop
/// or forward bytes based on whether a `File` is currently
/// installed in the global `LOG_FILE`.
///
/// At startup we read `Settings::debug_log` and call
/// `set_logging_enabled` with the persisted value. The in-app
/// title-bar Switch then calls the same function on every flip
/// for live on/off without a restart.
///
/// `RUST_LOG` is honoured for the filter level only (default
/// `info`); the env var being set does NOT override the
/// persisted on/off decision, since the pipe still discards when
/// no file is installed.
pub fn init_logging() {
    use env_logger::{Builder, Target};

    let mut builder = Builder::new();
    // env_logger applies an internal filter *before* writes
    // reach our `LogPipe`, and that filter is locked in at
    // `init()` time. Setting it to `Trace` here means env_logger
    // never gates on level — the real gate is `log::set_max_level`,
    // which we can adjust at runtime from the CLI's `-v` / `-q`
    // flags. `RUST_LOG` still wins if set (advanced users get the
    // standard env-var control over both level and per-module
    // filters).
    if let Ok(filter) = std::env::var("RUST_LOG") {
        builder.parse_filters(&filter);
    } else {
        builder.filter_level(log::LevelFilter::Trace);
    }
    builder.target(Target::Pipe(Box::new(settings::LogPipe)));
    builder.init();

    // `RUST_LOG` already set our level. Otherwise, default cap is
    // `Info` — the GUI's "Write debug log" toggle alone writes
    // info-level lifecycle events; `cli -v` lowers to Debug,
    // `cli -vv` to Trace, `cli -q` to Warn.
    if std::env::var("RUST_LOG").is_err() {
        log::set_max_level(log::LevelFilter::Info);
    }

    if settings::Settings::load_or_default().debug_log {
        settings::set_logging_enabled(true);
    }

    // One-shot cleanup of `~/Desktop/portfinder-debug.log` from
    // alpha builds (every release through 4.0.0 wrote there
    // unconditionally; 4.0.1+ doesn't, but the existing file
    // persists). Runs after `set_logging_enabled` so the
    // `log::info!` inside the helper actually lands in the file
    // when debug_log is on.
    settings::try_remove_legacy_desktop_log();

    // The startup banner used to live here, but it would drop on
    // the floor for the CLI `--log-file` path (file isn't open
    // yet — that happens later in `cli::apply_log_overrides`).
    // `set_logging_enabled` now emits the banner whenever it
    // successfully opens a log file, so every enable path
    // (settings, CLI flag, UI toggle) gets the same first line.
}
