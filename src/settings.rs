//! User-toggled persistent settings + live logger control.
//!
//! Settings are a tiny JSON file at the platform-conventional
//! config path:
//!
//! - macOS: `~/Library/Application Support/PortFinder/settings.json`
//! - Linux: `$XDG_CONFIG_HOME/portfinder/settings.json`
//!   (defaulting to `~/.config/portfinder/settings.json` per XDG)
//! - Windows: `%APPDATA%\PortFinder\settings.json`
//!
//! Loaded once at startup (via `Settings::load_or_default`) and
//! written on user-driven change (via `Settings::save`). Missing /
//! malformed files silently fall back to `Settings::default()` so
//! a corrupt settings file never blocks the app from launching.
//!
//! `LogPipe` + `set_logging_enabled` give the logger live on/off
//! without a process restart: `env_logger` is initialised once at
//! startup with `LogPipe` as its target, and the pipe consults a
//! `RwLock<Option<File>>` on every write. Toggling the in-app
//! Switch swaps the file handle in or out — next log line from
//! any thread observes the change.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::RwLock;

/// What's persisted. Defaults to "off / unset for everything" so a
/// fresh install (or a deleted settings file) gives the user the
/// quietest possible baseline.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// When true, `init_logging` writes log lines to the platform
    /// log path (see `log_file_path`). Default false: a stable
    /// install never drops a log file anywhere until the user
    /// explicitly enables it via the in-app toggle.
    #[serde(default)]
    pub debug_log: bool,
}

impl Settings {
    pub fn load_or_default() -> Self {
        let Some(path) = settings_path() else {
            return Self::default();
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return Self::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_path().ok_or("no config dir on this platform")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create config dir {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(|e| format!("serialise: {e}"))?;
        std::fs::write(&path, &json).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(())
    }
}

/// `<config_dir>/PortFinder/settings.json`. `dirs::config_dir()`
/// returns the per-OS conventional config path (Application Support
/// on macOS, %APPDATA% on Windows, XDG_CONFIG_HOME on Linux).
fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("PortFinder").join("settings.json"))
}

/// Where the debug log lives when `debug_log` is true. Logs go to
/// the OS-conventional *log* directory, not the config directory:
/// `~/Library/Logs/PortFinder/` on macOS (Console.app indexes this
/// automatically), `$XDG_STATE_HOME/portfinder/` on Linux,
/// `%LOCALAPPDATA%\PortFinder\Logs\` on Windows. Local (not Roaming)
/// on Windows is the convention for logs — they're machine-specific
/// and shouldn't sync across the user's other Windows hosts.
pub fn log_file_path() -> Option<PathBuf> {
    let dir = if cfg!(target_os = "windows") {
        // `dirs::data_local_dir` is %LOCALAPPDATA% on Windows.
        dirs::data_local_dir()?.join("PortFinder").join("Logs")
    } else if cfg!(target_os = "macos") {
        // Apple's documented spot for app logs. `dirs::home_dir` +
        // `Library/Logs` is the cleanest way to land there without
        // pulling a macOS-specific crate; `dirs` doesn't have a
        // dedicated log_dir() helper on macOS.
        dirs::home_dir()?.join("Library").join("Logs").join("PortFinder")
    } else {
        // `dirs::state_dir` is $XDG_STATE_HOME (defaulting to
        // ~/.local/state) — the XDG-spec'd location for logs and
        // other persistent-but-regenerable user state.
        dirs::state_dir()?.join("portfinder")
    };
    Some(dir.join("portfinder.log"))
}

/// The `File` env_logger pipes into. `None` means logging is off
/// — `LogPipe::write` short-circuits and discards. Wrapped in an
/// `RwLock` so the in-app toggle can swap the handle on/off from
/// the GUI thread while log calls from other threads see the
/// change on their next write.
static LOG_FILE: RwLock<Option<File>> = RwLock::new(None);

/// `io::Write` implementation handed to `env_logger` at startup
/// via `Target::Pipe`. Forwards each write to the currently
/// installed file in `LOG_FILE`, or silently discards when no
/// file is installed. Zero-sized — all the state lives in the
/// `LOG_FILE` static.
///
/// The lock-per-write cost is irrelevant for our volume (one or
/// two lines per minute at info level, none in steady state) and
/// keeps the file swap atomic with respect to ongoing writes.
pub struct LogPipe;

impl Write for LogPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = LOG_FILE
            .write()
            .map_err(|_| io::Error::other("LOG_FILE lock poisoned"))?;
        match guard.as_mut() {
            Some(file) => file.write(buf),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = LOG_FILE
            .write()
            .map_err(|_| io::Error::other("LOG_FILE lock poisoned"))?;
        match guard.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

/// Flip the global logging file on/off. Called from
/// `init_logging` at startup (driven by the persisted setting)
/// and from the in-app Switch's on_click (driven by the user
/// toggle). When `enabled=true`, opens / appends to the log
/// file at `log_file_path` and stores the handle in `LOG_FILE`;
/// when `enabled=false`, drops the stored handle (closes the fd)
/// so subsequent writes silently discard.
///
/// Open failures (missing parent dir, read-only filesystem, etc.)
/// are swallowed — the worst case is that "enabled" silently
/// produces no log file, which is the same UX as "disabled".
/// The user can verify via "Open log folder" whether the
/// directory exists and is populated.
pub fn set_logging_enabled(enabled: bool) {
    let Ok(mut guard) = LOG_FILE.write() else {
        return;
    };
    if !enabled {
        *guard = None;
        return;
    }
    let Some(path) = log_file_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        *guard = Some(file);
    }
}

/// Cross-platform "open this directory in the user's file manager"
/// helper. Used by the title-bar settings popover's "Open log
/// folder" button. Silently best-effort — failures (no file
/// manager, missing dir, missing OS-level open helper) just don't
/// open anything. Creates the directory first so the open
/// succeeds even before any log line has been written.
pub fn reveal_log_folder() {
    let Some(log_path) = log_file_path() else { return };
    let Some(dir) = log_path.parent() else { return };
    let _ = std::fs::create_dir_all(dir);

    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![])
    } else if cfg!(target_os = "windows") {
        ("explorer", vec![])
    } else {
        ("xdg-open", vec![])
    };
    let _ = std::process::Command::new(cmd)
        .args(args)
        .arg(dir)
        .spawn();
}
