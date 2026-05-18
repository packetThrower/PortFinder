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

    /// Filter level applied via `log::set_max_level` at boot
    /// and on every settings-popover slider change. The CLI's
    /// `-v` / `-vv` / `-q` flags still override at runtime —
    /// they call `set_max_level` after `apply_log_overrides`.
    #[serde(default)]
    pub log_level: LogLevel,

    /// When true, `history.json` is written alongside
    /// `settings.json` on each successful capture and hydrated
    /// back into the History popover on startup. Default
    /// false: opt-in keeps a fresh install from quietly
    /// accumulating a record of which switches the user
    /// probed. Flipping the toggle ON snapshots the current
    /// in-memory history to disk; flipping OFF deletes the
    /// file (in-memory stays for the rest of the session).
    #[serde(default)]
    pub persist_history: bool,
}

/// User-facing logging verbosity. Three options expose
/// `log::LevelFilter` to the GUI without surfacing the rarely-
/// useful `Off` (the Switch already covers that case) or
/// `Error` / `Warn` (those alone strip all the lifecycle
/// breadcrumbs that make a log file useful in the first place).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Info-level — lifecycle events only (boot, capture
    /// start/stop, update check, settings flips). The default.
    #[default]
    Normal,
    /// Debug-level — adds per-event capture diagnostics
    /// (interface picked, frame bytes, parser dispatch). Use
    /// when "PortFinder isn't capturing" is the bug.
    Verbose,
    /// Trace-level — adds the per-pcap-tick (~20 Hz per
    /// interface) timeout retries. Almost always overkill;
    /// useful when chasing a libpcap-side timing question.
    Trace,
}

impl LogLevel {
    /// Map to `log::LevelFilter` for `log::set_max_level`.
    pub fn to_max_level(self) -> log::LevelFilter {
        match self {
            Self::Normal => log::LevelFilter::Info,
            Self::Verbose => log::LevelFilter::Debug,
            Self::Trace => log::LevelFilter::Trace,
        }
    }

    /// Position on the popover's 3-stop slider (0=Normal,
    /// 1=Verbose, 2=Trace). The variant order matches the
    /// slider direction left-to-right — moving an option
    /// means updating both this and `from_stop_index`.
    pub fn stop_index(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Verbose => 1,
            Self::Trace => 2,
        }
    }

    /// Inverse of `stop_index`, for the slider's `Change`
    /// subscription. Returns `Normal` for any out-of-range
    /// index (shouldn't happen with `min=0, max=2, step=1`,
    /// but defensive against the slider snapping past an
    /// endpoint).
    pub fn from_stop_index(ix: usize) -> Self {
        match ix {
            1 => Self::Verbose,
            2 => Self::Trace,
            _ => Self::Normal,
        }
    }
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

/// `<config_dir>/PortFinder/history.json`. Same directory as
/// `settings.json` so the "Open settings folder" reveal shows
/// both in the file manager. Stored as JSON (rather than a
/// binary format) so a curious user can grep / `jq` past
/// captures without re-launching the app.
pub fn history_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("PortFinder").join("history.json"))
}

/// Read the persisted capture history. Missing / malformed
/// files return an empty `Vec` — same forgiveness `Settings`
/// has, since a corrupt history file should not block the
/// app from booting. Callers convert to `VecDeque` at the use
/// site (gpui's `AppView::new`).
///
/// Generic over the deserialised type so the helper doesn't
/// have to know about `HistoryEntry`'s shape (which lives in
/// `app_view.rs` because it references `Protocol`, which is
/// also UI-side). The `T: DeserializeOwned + Default` bound
/// keeps the fallback safe even if `T` ever stops being
/// `Vec<...>`.
pub fn load_history<T: serde::de::DeserializeOwned + Default>() -> T {
    let Some(path) = history_path() else {
        return T::default();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return T::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Delete `history.json` if present. Called when the user
/// flips the "Save capture history" toggle OFF — "off" means
/// "no record on disk," so the file goes with the toggle.
/// Silent on missing file: deleting a file that isn't there
/// is the same end state as deleting one that was, and the
/// caller doesn't need to distinguish.
pub fn clear_history_file() {
    let Some(path) = history_path() else { return };
    match std::fs::remove_file(&path) {
        Ok(()) => log::info!("cleared history file {}", path.display()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("clear history failed: {e}"),
    }
}

/// Write the capture history JSON. Generic for the same
/// reason as `load_history`. Failures bubble up so the GUI
/// can log them; the in-memory deque stays authoritative
/// whether the write succeeds or not.
pub fn save_history<T: serde::Serialize>(history: &T) -> Result<(), String> {
    let path = history_path().ok_or("no config dir on this platform")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create config dir {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(history).map_err(|e| format!("serialise: {e}"))?;
    std::fs::write(&path, &json).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
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

/// One-shot path override for `set_logging_enabled`. When `Some`,
/// the next (and subsequent) `set_logging_enabled(true)` calls
/// open this path instead of the persisted-settings default.
/// Used by the CLI's `--log-file` flag: the override is
/// process-scoped, never written back to disk. `RwLock` rather
/// than `Mutex` because the read-hot path is `set_logging_enabled`
/// (called once per startup + once per UI toggle).
static LOG_FILE_OVERRIDE: RwLock<Option<PathBuf>> = RwLock::new(None);

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
    // CLI `--log-file <path>` override takes precedence over the
    // persisted-settings default. Held in a separate static so
    // the override is process-scoped and never written back to
    // disk.
    let path = LOG_FILE_OVERRIDE
        .read()
        .ok()
        .and_then(|g| g.clone())
        .or_else(log_file_path);
    let Some(path) = path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Size-based rotation: if the existing log is over 1 MiB,
    // move it aside before opening the append handle so the
    // file doesn't grow without bound. One backup generation
    // (`<name>.log.1`) is enough for the "I want to see what
    // happened in the last session" case; users who need more
    // history can chain `cp` via cron.
    //
    // Rotation happens here (at enable time) rather than per-
    // write to avoid the syscall cost on every log line. Enable
    // fires at process start + on every UI toggle flip, which
    // is the right cadence — a single run that fills 1 MiB
    // probably has bigger problems than its log size.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 1_048_576 {
            let backup = path.with_extension("log.1");
            let _ = std::fs::rename(&path, &backup);
        }
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
        *guard = Some(file);
        // Drop the lock before logging — log macros take the same
        // lock via `LogPipe::write`, and we'd self-deadlock if it
        // were held here.
        drop(guard);
        log::info!(
            "PortFinder v{} log session started (target_os={})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS
        );
    }
}

/// Process-scoped log path override. Used by the CLI's
/// `--log-file <path>` flag — the next `set_logging_enabled(true)`
/// (which the same flag call also triggers) opens this path
/// instead of the platform default from `log_file_path`. Doesn't
/// touch the on-disk settings.json.
pub fn override_log_path(path: PathBuf) {
    if let Ok(mut guard) = LOG_FILE_OVERRIDE.write() {
        *guard = Some(path);
    }
}

/// One-shot cleanup of the legacy `~/Desktop/portfinder-debug.log`
/// that alpha-era builds (every release up to and including
/// `4.0.0`) wrote unconditionally on every launch. The 4.0.1
/// opt-in logger never writes there, but the *file* persists for
/// anyone who installed an alpha — it shows up as a stray file on
/// their desktop until they delete it manually.
///
/// This deletes the file if:
///   - it exists at the expected path, AND
///   - it's a regular file (not a symlink — somebody might have
///     pointed it elsewhere on purpose), AND
///   - it's under 10 MiB (limits blast radius if a user happened
///     to drop a same-named but unrelated file there).
///
/// The check runs on every launch; the cost is one `stat()` per
/// startup once the file is gone, which is irrelevant. We don't
/// gate on a "did this once" flag because we don't want a settings
/// schema bump for a single-shot cleanup, and the operation is
/// idempotent.
pub fn try_remove_legacy_desktop_log() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let path = home.join("Desktop").join("portfinder-debug.log");
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return;
    };
    if !meta.file_type().is_file() {
        return;
    }
    if meta.len() > 10 * 1024 * 1024 {
        return;
    }
    if std::fs::remove_file(&path).is_ok() {
        log::info!(
            "removed legacy alpha-era debug log at {}",
            path.display()
        );
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
    reveal_dir(dir);
}

/// Sibling to `reveal_log_folder` for the config directory
/// (`settings.json` + `history.json` live here). Wired to the
/// settings popover's "Open settings folder" button. Same
/// best-effort policy: create the directory first so the
/// reveal lands somewhere even on a fresh install before any
/// setting has been saved.
pub fn reveal_settings_folder() {
    let Some(path) = settings_path() else { return };
    let Some(dir) = path.parent() else { return };
    reveal_dir(dir);
}

/// Shared "make sure dir exists, then ask the OS to open it"
/// implementation. macOS hands the path to `open`, Windows to
/// `explorer`, everything else to `xdg-open`. The spawn is
/// fire-and-forget — we don't await the child or surface its
/// exit code; the user noticing the file manager not opening
/// is feedback enough.
fn reveal_dir(dir: &std::path::Path) {
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
