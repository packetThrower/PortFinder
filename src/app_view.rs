//! gpui window + view for PortFinder.
//!
//! Mirrors the 3.x Tauri/Svelte UI:
//!   - privilege-warning banner (BPF install on macOS, sudo hint on
//!     Linux, Npcap download link on Windows);
//!   - controls card: interface picker, "only with IPs" toggle,
//!     protocol picker (LLDP / CDP / MNDP), Start / Stop;
//!   - result card: 7 key/value rows with click-to-copy values;
//!   - status text + version footer.
//!
//! Bridges to the (sync, libpcap-backed) capture module via a tokio
//! runtime running on a background OS thread. `capture::run` is async
//! because internally it `spawn_blocking`s the pcap read and races
//! that against a CancellationToken — keeping that shape means the
//! parsers and the CLI path port across the 3.x → 4.x rewrite
//! unchanged. The gpui side talks to the runtime via flume channels.

use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flume::{Receiver, Sender};
use gpui::{
    actions, div, prelude::*, px, rgb, AnyView, App, AppContext, Bounds, ClipboardItem, Context,
    Entity, FocusHandle, Focusable, Hsla, IntoElement, KeyBinding, Menu, MenuItem, ParentElement,
    QuitMode, Render, SharedString, Styled, Window, WindowBounds, WindowDecorations, WindowOptions,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    popover::Popover,
    select::{Select, SelectEvent, SelectItem, SelectState},
    skeleton::Skeleton,
    slider::{Slider, SliderEvent, SliderState},
    switch::Switch,
    tooltip::Tooltip,
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Root, Sizable, Theme, ThemeMode, TitleBar,
};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle as TokioHandle;
use tokio_util::sync::CancellationToken;

// gpui's standard application-action pattern: declare actions via
// `actions!(namespace, [...])` which expands into one zero-sized
// struct per name, then wire them to keybindings + menu items in
// `run()` and to a handler via `cx.on_action`. Without the
// keybinding registration, Cmd+Q on macOS reaches the OS but the
// app has no handler — the menu item is also missing entirely
// (gpui doesn't install a default Application menu) so the user
// has no visible way to quit other than red-light-clicking the
// window.
//
// `Quit` is an app-level action — handled via `cx.on_action` in
// `run()` and bound globally. `StartOrStop` is view-level: the
// handler needs `&mut AppView` to flip `is_capturing` and kick
// the tokio task, so it's wired via `.on_action` on the AppView
// root div in `render` and the keybinding is scoped to the
// "AppView" key_context so it doesn't fire when a child Popover
// or modal is up.
actions!(portfinder, [Quit, StartOrStop]);

use crate::{
    capture, privilege, settings, updater, CaptureRequest, CaptureResult, InterfaceInfo,
};

/// Logical window width. Stays fixed — only the height grows or
/// shrinks based on the privilege banner and result-card state.
const BASE_WIDTH: f32 = 420.0;

/// Per-state content-height pieces, summed in `desired_height()` to
/// derive the right window height for the current state. Tuned to
/// match the actual rendered layout (gpui doesn't autosize windows
/// to content, so these have to track the per-element heights kept
/// in `render`). Adjust if you change widget sizes or padding.
///
/// Numbers cover what gpui calls "content" (the area below the
/// macOS title bar), so they don't include the ~28px title bar
/// itself — `window.resize` sets content size directly.
///
/// Breakdown for HEIGHT_BASE (no banner, no result card):
///   12 p_3 top padding
/// + ~180 controls card (3 form sections + Switch + button row + p_3)
/// + 12 gap_3
/// + (result card not included — see HEIGHT_RESULT_*)
/// + 12 gap_3
/// + ~17 status line (text_xs)
/// + 12 gap_3
/// + ~17 version line (text_xs)
/// + 12 p_3 bottom padding
/// + 34 gpui-component TitleBar (drawn by AppView so the chrome is
///   consistent across macOS / Linux / Windows; Linux Wayland in
///   particular doesn't get server-side decorations on Mutter, so
///   the app has to render its own)
///
/// ≈ 308 px. Set to 341 — visually-tuned so the version footer's
/// bottom inset reads as balanced with the 12 px top inset. Tuned
/// by eye because the per-piece calculation drifts a few px from
/// actual rendered heights (font metrics, hairline borders, gpui's
/// px rounding); the rule of thumb is "tweak this constant, not
/// the individual pieces, when the bottom dead-space looks wrong".
const HEIGHT_BASE: f32 = 341.0;

/// Extra vertical room added to `desired_height()` on Linux to
/// compensate for client-side-decoration overhead. On Wayland
/// compositors that don't support SSD (Mutter on GNOME, the
/// default on Ubuntu), gpui_linux reports `viewport_size()` as
/// the full window allocation but the compositor reserves some
/// of that area for shadow + resize handles — net effect is that
/// our `viewport_size` matches `desired_height` numerically while
/// the visually-rendered content area is ~24 px shorter, clipping
/// the version footer at the bottom edge. 24 px was measured from
/// the gap between requested 393 px and visibly-rendered ~370 px
/// on Ubuntu 24.04 + Mutter. Zero on macOS / Windows where the
/// compositor doesn't claw back any of our content area.
#[cfg(target_os = "linux")]
const HEIGHT_CSD_PADDING: f32 = 24.0;
#[cfg(not(target_os = "linux"))]
const HEIGHT_CSD_PADDING: f32 = 0.0;
/// Banner is conditionally rendered when capture privileges are
/// missing. Sized for the macOS path's three-line body text
/// ("PortFinder needs BPF access to capture packets. Installing
/// the helper grants /dev/bpf* read access to the access_bpf
/// group.") plus the Install BPF Helper button, plus the card's
/// own p_3 padding and the gap_2 between body and button. The
/// previous 110 px was tuned for the (incorrect) assumption of
/// two body lines; the real wrap pushes to three and the button
/// itself is fuller than estimated.
const HEIGHT_BANNER: f32 = 145.0;
/// Empty result card — a single italic line ("Run a capture to see
/// switch info here.") plus padding.
const HEIGHT_RESULT_EMPTY: f32 = 40.0;
/// Populated result card — seven key/value rows with the standard
/// row gap, plus the action-button footer row (small button +
/// `pt_2` / `mt_1` / 1 px top divider ≈ 38 px), plus card padding.
/// Same height whether or not a field is "absent"; the absent
/// rows still occupy their slot. Was 230 before the JSON footer
/// landed; bumped to 258 then to 268 when the divider + History
/// button row got the extra inset.
const HEIGHT_RESULT_FILLED: f32 = 268.0;
/// Capturing-state result card — same seven skeleton rows but at
/// their actual rendered content size (7 × 24 px rows + 6 × 4 px
/// gaps + 24 px card padding ≈ 216 px). `HEIGHT_RESULT_FILLED`
/// (230) was over-allocating because populated text rows pack
/// slightly tighter than the skeleton's `.h(px(24.0))` containers,
/// which left 14 px of dead space below the version footer in the
/// capturing state but not the populated state. Sizing this slot
/// to the skeleton's actual height collapses that gap.
const HEIGHT_RESULT_SKELETON: f32 = 206.0;
/// Extra vertical room consumed by the "Update available" footer
/// pill when it's shown. The pill is taller than a plain version-
/// text line (Button widgets carry their own padding); without
/// this allowance, the version row scrolls into the bottom edge
/// when an update notification is up.
const HEIGHT_UPDATE_PILL_EXTRA: f32 = 12.0;

/// Brand accent. macOS system blue — the same colour the 3.x design
/// system reached for whenever it needed to step outside the OS-
/// neutral palette (privilege-warning button, focused-toggle pill).
/// Wired into the gpui-component Theme as `primary`, which flows
/// through to the Start button's filled fill, the Switch's on-state
/// pill, and the Select's focus ring.
const BRAND_PRIMARY: u32 = 0x0078d4;
/// Hover state — lighter shift on hover. ~10% brighter than the
/// base; gpui-component derives the rest of the hover behaviour
/// internally.
const BRAND_PRIMARY_HOVER: u32 = 0x1f8fea;
/// Active (pressed) state — darker than the base by ~10%.
const BRAND_PRIMARY_ACTIVE: u32 = 0x005a9e;

/// Card surface — macOS 26's "secondary system grouped background"
/// (`UIColor.secondarySystemGroupedBackground` on iOS, the same
/// neutral gray Apple paints under elevated content in System
/// Settings, Mail, Notes, Messages). Cool gray, not warm — the
/// previous warm-cream tint read as 90s Aqua / OS 9 dialog
/// background, which clashes with the Liquid Glass aesthetic Tahoe
/// (macOS 26) drives toward.
const CARD_BG_LIGHT: u32 = 0xf2f2f7;
/// Dark-mode counterpart — Apple's `secondarySystemBackground` in
/// dark mode. One shade lighter than the window background so the
/// cards still read as elevated content.
const CARD_BG_DARK: u32 = 0x2c2c2e;

/// Skeleton-loader fill. gpui-component's default `theme.skeleton`
/// is a near-transparent gray that washes out against our
/// `CARD_BG_LIGHT` surface — the loading rows render but are barely
/// visible. Picking Apple's systemGray3 instead (the same neutral
/// gray macOS uses for inactive form chrome and loading bars) gives
/// the rows enough contrast against the card to read at a glance.
/// The Skeleton widget's built-in pulse drops opacity to 0.5 every
/// 2 s, so the resting colour needs to be solid enough that the
/// dim half of the pulse still shows.
const SKELETON_LIGHT: u32 = 0xc7c7cc;
/// Dark-mode counterpart — Apple's systemGray2 (a touch lighter
/// than systemGray3 here so the skeleton has visible contrast
/// against `CARD_BG_DARK`, which is already a dim surface).
const SKELETON_DARK: u32 = 0x48484a;

/// Process-lifetime tokio runtime handle. The runtime itself is
/// leaked (`mem::forget`) once at first access — gpui has its own
/// executor, but `capture::run` uses `tokio::task::spawn_blocking`
/// internally so a tokio runtime needs to be entered when futures
/// from that module execute.
static TOKIO: OnceLock<TokioHandle> = OnceLock::new();

fn tokio_handle() -> &'static TokioHandle {
    TOKIO.get_or_init(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("portfinder-tokio")
            .build()
            .expect("start tokio runtime");
        let handle = rt.handle().clone();
        // Keep the runtime alive for the process. Dropping the
        // Runtime tears down its worker threads, so we leak it —
        // the OS reclaims the memory at exit.
        std::mem::forget(rt);
        handle
    })
}

/// Discovery protocol the capture engine looks for. Local mirror of
/// the string-based wire protocol — easier to round-trip into
/// `CaptureRequest.protocol` on the way out.
///
/// `Serialize` / `Deserialize` are for the persistent-history
/// file (`history.json`) round-trip — each `HistoryEntry`
/// carries the protocol used. `lowercase` rename keeps the JSON
/// readable to a human poking at the file ("lldp" / "cdp" /
/// "mndp" rather than "Lldp" / "Cdp" / "Mndp").
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Protocol {
    Lldp,
    Cdp,
    Mndp,
}

impl Protocol {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lldp => "LLDP",
            Self::Cdp => "CDP",
            Self::Mndp => "MNDP",
        }
    }

    fn from_value(value: &str) -> Self {
        match value {
            "CDP" => Self::Cdp,
            "MNDP" => Self::Mndp,
            _ => Self::Lldp,
        }
    }
}

/// Select-widget option. Stores the user-facing title plus the
/// stable value that gets round-tripped on `SelectEvent::Confirm`.
#[derive(Clone)]
struct Opt {
    title: SharedString,
    value: SharedString,
}

impl Opt {
    fn new(title: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
        }
    }
}

impl SelectItem for Opt {
    type Value = SharedString;
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.value
    }
}

/// Posted by the background tokio task through a flume channel to
/// the gpui side when `capture::run` completes.
enum CaptureEvent {
    Done(Result<CaptureResult, String>),
}

/// One of the 7 result fields the GUI renders. The static string is
/// what the click-to-copy ✓ indicator keys off so we can show
/// "copied!" only on the row the user actually clicked.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ResultKey(&'static str);

/// Reserved `ResultKey` for the result card's "Copy as JSON"
/// footer button. Shares the `copied_key` state field with the
/// per-row value buttons, which is why we need a sentinel
/// distinct from the seven row keys (`"switch"`, `"ip"`, etc.)
/// — used by `render_result_card` to know when to flash
/// "Copied" next to the JSON button vs next to a row's value.
const RESULT_KEY_JSON: ResultKey = ResultKey("json");

/// One entry in the persisted capture history. Captured on
/// successful capture completion (`on_capture_done` with
/// `Ok`); failures, cancellations, and "Not advertised"-on-
/// every-field cases do still land here (a result with all
/// `"N/A"` fields can still be the answer to "is this port
/// even patched"). Stored in `AppView::history` as a
/// `VecDeque` with `HISTORY_MAX` cap, oldest evicted first;
/// the deque is serialized to `history.json` alongside
/// `settings.json` on each push and reloaded on startup.
///
/// `captured_at` is wall-clock seconds since UNIX_EPOCH (not
/// `Instant`, which is process-local and can't survive a
/// restart). `format_ago` does `now - captured_at` against
/// `SystemTime::now()` to render the relative timestamp. Wall
/// clock CAN move backwards (DST, NTP), but only the relative
/// display would briefly disagree — `saturating_sub` keeps
/// the format from underflowing in that case.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryEntry {
    result: CaptureResult,
    protocol: Protocol,
    interface_name: String,
    /// Seconds since UNIX_EPOCH at the moment the capture
    /// landed in `on_capture_done`. `u64` (not `i64`) — we
    /// never read history written before 1970-01-01.
    captured_at_epoch_secs: u64,
}

/// Cap on `AppView::history`. Past this many captures the
/// oldest entry gets evicted on `push_back`. 10 picked from
/// the "bouncing between switches / comparing two cables"
/// use case the TODO file describes — a tech walking a rack
/// is unlikely to be juggling more than ~10 ports in their
/// short-term-memory window.
const HISTORY_MAX: usize = 10;

/// Which page of the settings popover is currently visible.
/// `Main` is the regular settings list with the section rows;
/// the other two are drill-down sub-pages reachable via their
/// respective rows in `Main` and dismissable via a "← Back"
/// header on the sub-page itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SettingsView {
    Main,
    About,
    Language,
}

pub struct AppView {
    focus_handle: FocusHandle,

    // Discovered interfaces + UI state for the picker.
    interfaces: Vec<InterfaceInfo>,
    only_with_ips: bool,
    selected_interface_name: String,
    interface_select: Entity<SelectState<Vec<Opt>>>,

    // Protocol picker.
    protocol: Protocol,
    protocol_select: Entity<SelectState<Vec<Opt>>>,

    // Capture state.
    is_capturing: bool,
    capture_cancel: Option<CancellationToken>,
    capture_result_tx: Sender<CaptureEvent>,
    capture_result_rx: Receiver<CaptureEvent>,

    // Result + status.
    result: Option<CaptureResult>,
    error: String,
    status_text: SharedString,
    copied_key: Option<ResultKey>,

    // Last N capture results, newest at the back. Populated
    // by `on_capture_done` with `Ok`; rendered via the
    // History popover on the result card. In-memory only —
    // see `HistoryEntry` for the persistence-deferred note.
    history: VecDeque<HistoryEntry>,

    // Which page of the settings popover is showing. The
    // popover is drilled-into rather than long-scrolling
    // because gpui-component's Popover anchors to the
    // trigger's top-left and doesn't shift down on overflow
    // (its `resolved_corner` has no "below the trigger"
    // case), so a tall popover gets clipped at the window
    // bottom. Each sub-page renders a "← Back" header and
    // returns here when dismissed. Not persisted; reverts
    // to `Main` on relaunch.
    settings_view: SettingsView,

    // Dirty flag for the `window.resize` call in `Render::
    // render`. Set to `true` by every mutator that touches
    // an input to `desired_height()` (start_capture, on_
    // capture_done, restore_from_history, the BPF install
    // callback, the update listener, and the update-pill
    // dismiss). Cleared by `render` after applying the
    // resize.
    //
    // The earlier shape — checking `desired_height() !=
    // viewport_size().height` on every render frame — sent
    // an `xdg_surface.configure` round-trip through the
    // Wayland compositor on every animation tick under
    // fractional scaling (the 1 px slack guard isn't enough
    // when Hyprland reports logical sizes at 1.25× or
    // 1.5×). Strict wlroots compositors then raced popup
    // positioning against the in-flight configure, leaving
    // dropdowns rendering at stale coordinates. Mutter /
    // KWin smoothed over the configure storm; Hyprland
    // didn't. Initial value is `true` so the first render
    // sizes the window to fit the boot-time banner state
    // before the user sees it.
    resize_pending: bool,

    // Privileges + helper install.
    priv_status: Option<privilege::PrivilegeStatus>,
    is_installing: bool,

    // Update-available state — populated by the boot-time GitHub
    // Releases check (runs on a tokio-blocking task, posts back via
    // `update_rx`). `update_dismissed` is the user's per-session
    // "I saw it" flag; staying in-memory means the pill returns on
    // next launch if an upgrade still hasn't happened, which is the
    // right nag cadence for a tool that's used in 30-second bursts.
    update_available: Option<updater::UpdateInfo>,
    update_dismissed: bool,
    update_result_rx: Receiver<updater::UpdateInfo>,

    // User-toggled settings, loaded from the persisted JSON at
    // startup and re-saved whenever the title-bar hamburger
    // toggle flips a value. Currently just `debug_log`, but the
    // struct is shaped for additional opt-ins to land here
    // without per-field plumbing. The toggle takes effect live
    // via `settings::set_logging_enabled` — no "restart to
    // apply" dance needed.
    settings: settings::Settings,

    // Slider state for the title-bar settings popover's "Log
    // level" row. 3 discrete stops (0=Normal, 1=Verbose,
    // 2=Trace) matching `settings::LogLevel::stop_index`. The
    // Slider widget needs a long-lived `SliderState` entity
    // rather than a per-render value, so we keep it here and
    // subscribe to `SliderEvent::Change` via `_log_level_sub`
    // below — that listener mirrors the slider value back into
    // `settings.log_level`, applies via `log::set_max_level`,
    // and persists.
    log_level_slider: Entity<SliderState>,

    // Subscriptions kept alive for the entity's lifetime.
    iface_sub: gpui::Subscription,
    _proto_sub: gpui::Subscription,
    // Slider-change subscription. Held to keep the callback
    // alive; drop = unsubscribe.
    _log_level_sub: gpui::Subscription,
    // OS appearance observer. Fires on every system Light/Dark flip
    // (Control Center toggle, scheduled sunrise/sunset switch, etc.)
    // and reapplies the theme so the chrome tracks the OS without a
    // relaunch. Held to keep the callback alive for the view's
    // lifetime; drop = unsubscribe.
    _appearance_sub: gpui::Subscription,
}

impl AppView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Initial interface list. Failures are non-fatal: an empty
        // list means the user just sees the "Sniff all" placeholder
        // until the refresh button gets clicked.
        let interfaces = capture::list_interfaces().unwrap_or_default();
        let initial_filtered = build_interface_opts(&interfaces, true);

        let interface_select =
            cx.new(|cx| SelectState::new(initial_filtered, Some(IndexPath::new(0)), window, cx));
        let protocol_select = cx.new(|cx| {
            SelectState::new(protocol_opts(), Some(IndexPath::new(0)), window, cx)
        });

        let iface_sub = cx.subscribe(
            &interface_select,
            |this, _state, event: &SelectEvent<Vec<Opt>>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.selected_interface_name = value.to_string();
                    cx.notify();
                }
            },
        );
        let proto_sub = cx.subscribe(
            &protocol_select,
            |this, _state, event: &SelectEvent<Vec<Opt>>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.protocol = Protocol::from_value(value.as_ref());
                    cx.notify();
                }
            },
        );

        let priv_status = Some(privilege::get_privilege_status());
        let (tx, rx) = flume::bounded(1);

        // Boot-time update check. Spawned on the tokio runtime via
        // `spawn_blocking` because `ureq` is synchronous; the gpui
        // side awaits the flume receiver in `spawn_update_listener`.
        // Bounded channel of size 1 — there's at most one result.
        let (update_tx, update_rx) = flume::bounded::<updater::UpdateInfo>(1);
        let current_version = env!("CARGO_PKG_VERSION");
        tokio_handle().spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                updater::check_for_update(current_version)
            })
            .await
            .ok()
            .flatten();
            if let Some(info) = result {
                log::info!("update check: {} available", info.version);
                let _ = update_tx.send_async(info).await;
            } else {
                log::info!("update check: no newer release");
            }
        });

        // OS appearance observer. Fires whenever the system flips
        // Light↔Dark (Control Center toggle, scheduled sunrise/
        // sunset switch, etc.). Each fire re-applies the gpui-
        // component Theme + our brand palette overrides; the
        // explicit `refresh_windows` ensures every open window
        // repaints with the new palette on the next frame.
        let appearance_sub = window.observe_window_appearance(|window, cx| {
            apply_system_theme(cx, window.appearance());
            cx.refresh_windows();
        });

        // Load settings once; cached in `view.settings` and used
        // both to seed the slider's initial position and to feed
        // `set_max_level` in the subscription handler below.
        let settings_loaded = settings::Settings::load_or_default();
        // Slider over 0..=2 (Normal / Verbose / Trace) with
        // step=1. The widget snaps the *value* to integer stops
        // internally, but its *thumb position* tracks the raw
        // mouse percentage (a quirk of the gpui-component slider
        // — see `update_value_by_position` in slider.rs: it
        // writes `self.percentage = raw_percentage` while
        // `self.value = snapped_value`, so the visual thumb
        // floats anywhere along the bar). To get the macOS-style
        // "snap to discrete stop" feel, the subscription below
        // calls `set_value` back on the SliderState every change
        // — that runs `update_thumb_pos`, which re-derives the
        // percentage *from* the snapped value, pulling the
        // thumb onto the nearest stop. `set_value` doesn't emit
        // a Change event, so there's no echo loop.
        let log_level_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(2.0)
                .step(1.0)
                .default_value(settings_loaded.log_level.stop_index() as f32)
        });
        // `subscribe_in` (not `subscribe`) so the callback gets
        // `&mut Window` — `SliderState::set_value`'s signature
        // wants one even though its body ignores it. The window
        // is captured from `AppView::new`'s arg.
        let log_level_sub = cx.subscribe_in(
            &log_level_slider,
            window,
            |this, slider, event: &SliderEvent, window, cx| {
                let SliderEvent::Change(value) = event;
                let ix = value.start().round() as usize;
                // Always snap the thumb, even if the log level
                // hasn't changed — the user may have dragged
                // mid-segment and released; we want the visual
                // to land on a stop.
                slider.update(cx, |state, cx| {
                    state.set_value(ix as f32, window, cx);
                });
                let new = settings::LogLevel::from_stop_index(ix);
                if this.settings.log_level == new {
                    return;
                }
                this.settings.log_level = new;
                log::set_max_level(new.to_max_level());
                if let Err(e) = this.settings.save() {
                    log::warn!("settings save failed: {e}");
                }
                cx.notify();
            },
        );

        let mut view = Self {
            focus_handle: cx.focus_handle(),
            interfaces,
            only_with_ips: true,
            selected_interface_name: String::new(),
            interface_select,
            protocol: Protocol::Lldp,
            protocol_select,
            is_capturing: false,
            capture_cancel: None,
            capture_result_tx: tx,
            capture_result_rx: rx,
            result: None,
            error: String::new(),
            status_text: t!("status.ready").into_owned().into(),
            copied_key: None,
            // Hydrate from `history.json` only when the
            // user has opted in via the popover toggle —
            // otherwise the file (which may exist from a
            // prior opted-in session) is ignored, not
            // deleted. Cap to `HISTORY_MAX` on load too, in
            // case a previous version wrote more entries or
            // the file was hand-edited.
            history: if settings_loaded.persist_history {
                let mut deque: VecDeque<HistoryEntry> =
                    settings::load_history::<Vec<HistoryEntry>>().into();
                while deque.len() > HISTORY_MAX {
                    deque.pop_front();
                }
                deque
            } else {
                VecDeque::with_capacity(HISTORY_MAX)
            },
            settings_view: SettingsView::Main,
            // `true` so the first `render` resizes the window
            // to fit the banner state detected at boot before
            // the user sees the initial frame.
            resize_pending: true,
            priv_status,
            is_installing: false,
            update_available: None,
            update_dismissed: false,
            settings: settings_loaded,
            log_level_slider,
            update_result_rx: update_rx,
            iface_sub,
            _proto_sub: proto_sub,
            _appearance_sub: appearance_sub,
            _log_level_sub: log_level_sub,
        };
        view.spawn_capture_listener(cx);
        view.spawn_update_listener(cx);
        view
    }

    /// gpui-side task: waits for the boot-time update check to
    /// resolve, then surfaces the result via `update_available` so
    /// the footer pill renders on the next paint. One-shot — the
    /// channel is bounded(1) and never re-used.
    fn spawn_update_listener(&mut self, cx: &mut Context<Self>) {
        let rx = self.update_result_rx.clone();
        cx.spawn(async move |this, cx| {
            if let Ok(info) = rx.recv_async().await {
                let _ = this.update(cx, |this, cx| {
                    this.update_available = Some(info);
                    // Footer pill appears → window grows by
                    // HEIGHT_UPDATE_PILL_EXTRA. Flag resize.
                    this.resize_pending = true;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// gpui-side task: drains the flume receiver and applies each
    /// capture result to the view. Detached for the view's lifetime.
    fn spawn_capture_listener(&mut self, cx: &mut Context<Self>) {
        let rx = self.capture_result_rx.clone();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv_async().await {
                let r = this.update(cx, |this, cx| match event {
                    CaptureEvent::Done(res) => this.on_capture_done(res, cx),
                });
                if r.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn refresh_interfaces(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match capture::list_interfaces() {
            Ok(ifaces) => {
                self.interfaces = ifaces;
                self.rebuild_interface_select(window, cx);
            }
            Err(err) => {
                self.error = format!("Failed to load interfaces: {err}");
                self.status_text = "Error".into();
            }
        }
        cx.notify();
    }

    fn rebuild_interface_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let opts = build_interface_opts(&self.interfaces, self.only_with_ips);
        // SelectState doesn't expose a set-delegate method publicly,
        // so swap the entity. The new subscription replaces the old
        // one held on `iface_sub` — dropping the old Subscription
        // unhooks the previous listener.
        let new_state =
            cx.new(|cx| SelectState::new(opts, Some(IndexPath::new(0)), window, cx));
        let sub = cx.subscribe(
            &new_state,
            |this, _state, event: &SelectEvent<Vec<Opt>>, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.selected_interface_name = value.to_string();
                    cx.notify();
                }
            },
        );
        self.interface_select = new_state;
        self.iface_sub = sub;
        self.selected_interface_name.clear();
    }

    fn start_capture(&mut self, cx: &mut Context<Self>) {
        if self.is_capturing {
            return;
        }
        log::info!(
            "start_capture: interface={:?} protocol={}",
            self.selected_interface_name,
            self.protocol.as_str()
        );
        self.is_capturing = true;
        self.error.clear();
        self.result = None;
        // Result card switches from `_FILLED` / `_EMPTY` to
        // `_SKELETON` height — flag the window resize.
        self.resize_pending = true;
        self.status_text =
            t!("status.capturing", protocol = self.protocol.as_str()).into_owned().into();

        let cancel = CancellationToken::new();
        if let Some(prev) = self.capture_cancel.take() {
            prev.cancel();
        }
        self.capture_cancel = Some(cancel.clone());

        let req = CaptureRequest {
            interface_name: self.selected_interface_name.clone(),
            protocol: self.protocol.as_str().to_string(),
        };
        let tx = self.capture_result_tx.clone();
        tokio_handle().spawn(async move {
            let res = capture::run(req, cancel).await;
            let _ = tx.send_async(CaptureEvent::Done(res)).await;
        });

        cx.notify();
    }

    fn stop_capture(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.capture_cancel.take() {
            token.cancel();
        }
        self.status_text = t!("status.stopping").into_owned().into();
        cx.notify();
    }

    fn on_capture_done(&mut self, res: Result<CaptureResult, String>, cx: &mut Context<Self>) {
        log::info!(
            "on_capture_done: {}",
            match &res {
                Ok(r) => format!("ok({})", r.switch_name),
                Err(e) => format!("err({e})"),
            }
        );
        self.is_capturing = false;
        self.capture_cancel = None;
        // `is_capturing` flipping off changes which result-card
        // height applies; result transitions also do. Flag the
        // resize for both Ok and Err branches.
        self.resize_pending = true;
        match res {
            Ok(r) => {
                // Push to history before claiming `r` for
                // `self.result` — keeps the entry's `result`
                // and the rendered card consistent without a
                // second clone. `HISTORY_MAX` cap is enforced
                // by `pop_front` before `push_back`, so the
                // deque never exceeds capacity.
                if self.history.len() >= HISTORY_MAX {
                    self.history.pop_front();
                }
                self.history.push_back(HistoryEntry {
                    result: r.clone(),
                    protocol: self.protocol,
                    interface_name: self.selected_interface_name.clone(),
                    captured_at_epoch_secs: now_epoch_secs(),
                });
                // Persist after every push, but only when the
                // user has opted in via the popover toggle.
                // The file is tiny (<5 KB for the max-10
                // deque), so syncing on each push is cheap;
                // failures are logged and ignored — in-memory
                // state is the source of truth, disk is a
                // best-effort mirror.
                if self.settings.persist_history {
                    if let Err(e) = settings::save_history(&self.history) {
                        log::warn!("save history failed: {e}");
                    }
                }
                self.result = Some(r);
                self.status_text = t!("status.complete").into_owned().into();
            }
            Err(msg) => {
                if msg.to_lowercase().contains("cancelled") {
                    self.status_text = t!("status.stopped").into_owned().into();
                } else {
                    self.error = msg;
                    self.status_text = t!("status.error").into_owned().into();
                }
            }
        }
        cx.notify();
    }

    fn install_bpf(&mut self, cx: &mut Context<Self>) {
        if self.is_installing {
            return;
        }
        self.is_installing = true;
        self.error.clear();
        self.status_text = t!("status.installing_helper").into_owned().into();
        cx.notify();

        // `privilege::install_bpf_helper` blocks on `osascript`; run
        // it on a background thread so the UI stays responsive while
        // the macOS auth dialog is up.
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async { privilege::install_bpf_helper() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.is_installing = false;
                match result {
                    Ok(()) => {
                        this.status_text = t!("status.bpf_helper_installed").into_owned().into();
                        this.priv_status = Some(privilege::get_privilege_status());
                        // Privilege banner disappears once
                        // has_access flips → window shrinks
                        // by HEIGHT_BANNER. Flag resize.
                        this.resize_pending = true;
                    }
                    Err(err) => {
                        this.error = err;
                        this.status_text = t!("status.error").into_owned().into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_only_with_ips(&mut self, value: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.only_with_ips == value {
            return;
        }
        self.only_with_ips = value;
        self.rebuild_interface_select(window, cx);
        cx.notify();
    }

    /// Content height the window should occupy given the current
    /// state. The render hook compares this against
    /// `window.bounds().size.height` and resizes when they
    /// diverge — `cx.notify()` after every state change is what
    /// re-triggers the comparison.
    fn desired_height(&self) -> f32 {
        let mut h = HEIGHT_BASE + HEIGHT_CSD_PADDING;
        if self
            .priv_status
            .as_ref()
            .map(|s| !s.has_access)
            .unwrap_or(false)
        {
            h += HEIGHT_BANNER;
        }
        // Three result-card states, three different card heights:
        //   - populated: HEIGHT_RESULT_FILLED (full text rows)
        //   - capturing: HEIGHT_RESULT_SKELETON (tighter skeleton
        //     rows, so the window stays compact while waiting)
        //   - idle: HEIGHT_RESULT_EMPTY (single-line placeholder)
        // The window resize from SKELETON to FILLED when a capture
        // lands gives the "got data" visual cue for free.
        h += if self.result.is_some() {
            HEIGHT_RESULT_FILLED
        } else if self.is_capturing {
            HEIGHT_RESULT_SKELETON
        } else if !self.history.is_empty() {
            // Empty state with a History button below the
            // "Run a capture…" placeholder gets ~30 px more
            // vertical than the bare placeholder. Idle state
            // never shows the History button until a capture
            // has landed, so the first-launch height is still
            // exactly HEIGHT_RESULT_EMPTY.
            HEIGHT_RESULT_EMPTY + 30.0
        } else {
            HEIGHT_RESULT_EMPTY
        };
        if self.update_available.is_some() && !self.update_dismissed {
            h += HEIGHT_UPDATE_PILL_EXTRA;
        }
        h
    }

    fn copy_value(&mut self, key: ResultKey, value: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.copied_key = Some(key);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1200))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.copied_key == Some(key) {
                    this.copied_key = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }
}

impl Focusable for AppView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Dynamic resize: pick the height that fits the current
        // state (banner present? result populated?) and apply it.
        // Gated on `resize_pending` so it only fires after a
        // mutator that actually changed a `desired_height()`
        // input — not on every `cx.notify()` (which can come
        // from theme observation, focus events, the copied-key
        // timeout, etc., none of which affect the desired size).
        //
        // The earlier shape — comparing `desired_height()` to
        // `viewport_size().height` on every render — sent an
        // `xdg_surface.configure` round-trip through the
        // Wayland compositor on every animation tick under
        // fractional scaling (the 1 px slack guard isn't
        // enough when Hyprland reports logical sizes at 1.25×
        // or 1.5×). Strict wlroots compositors then raced
        // popup positioning against the in-flight configure,
        // leaving dropdowns rendering at stale coordinates.
        // Each mutator that touches a `desired_height()` input
        // sets the flag explicitly, so the resize fires exactly
        // N times instead of every frame.
        //
        // `viewport_size()` is read here (not `bounds().size`)
        // because the two gpui platform backends interpret
        // bounds differently:
        //   - macOS: `bounds()` returns the FULL window frame
        //     (content + title bar). `resize()` sets just the
        //     CONTENT size.
        //   - Windows: `bounds()` returns the logical content
        //     area. `resize()` sets the CONTENT size (border
        //     offset added internally before SetWindowPos).
        // Comparing `bounds().size` against the desired content
        // size is therefore always ~28 px off on macOS;
        // `viewport_size()` is the drawable area on both
        // backends, so the comparison is cross-platform-
        // consistent. The 1 px slack guards against rounding
        // noise from the px → device-px → px round-trip when
        // the requested size already matches in practice.
        if self.resize_pending {
            self.resize_pending = false;
            let desired = px(self.desired_height());
            let current = window.viewport_size().height;
            if (current - desired).abs() > px(1.0) {
                log::debug!(
                    "render: resize viewport.h={} -> desired={} (scale={}, capturing={}, result={}, banner_visible={})",
                    current,
                    desired,
                    window.scale_factor(),
                    self.is_capturing,
                    self.result.is_some(),
                    self.priv_status
                        .as_ref()
                        .map(|s| !s.has_access)
                        .unwrap_or(false),
                );
                window.resize(gpui::size(px(BASE_WIDTH), desired));
            }
        }

        // Read all theme colors up front so the immutable cx borrow
        // is dropped before the render helpers (which take `&mut cx`)
        // run. Theme::theme() returns `&Theme`; without the block
        // the immutable borrow lives through to the closing `.child(...)`
        // call and the borrow checker rejects every helper invocation.
        let (bg, fg, card_bg, border, muted_fg, danger) = {
            let t = cx.theme();
            (
                t.background,
                t.foreground,
                t.popover,
                t.border,
                t.muted_foreground,
                t.danger,
            )
        };

        // Compute whether a banner is needed up front so we can
        // skip the banner child entirely when it isn't. The previous
        // shape always added a zero-size banner element, which made
        // `gap_3` fire twice between the top padding and the first
        // card (12 px padding + 12 px gap above empty banner + 12 px
        // gap below empty banner = 24 px above the controls card,
        // vs 12 px on the left/right edges). Conditional child gives
        // a truly 12 px top inset matching the sides.
        let needs_banner = self
            .priv_status
            .as_ref()
            .map(|s| !s.has_access)
            .unwrap_or(false);
        let banner = needs_banner.then(|| self.render_privilege_banner(cx));
        let controls = self.render_controls(cx);
        let result_card = self.render_result_card(cx);

        let status_color = if self.error.is_empty() {
            muted_fg
        } else {
            danger
        };
        let status_line = if !self.error.is_empty() {
            self.error.clone().into()
        } else {
            self.status_text.clone()
        };

        div()
            .id("portfinder-app")
            .track_focus(&self.focus_handle)
            // Matches `Some("AppView")` on the `StartOrStop`
            // keybinding in `run()`. gpui only routes that
            // action to the `on_action` handler below when an
            // element in the focus chain carries this context —
            // popovers / modals stacked over the AppView don't
            // (their own key_context wins), so the shortcut is
            // automatically inert while a popover is open.
            .key_context("AppView")
            .on_action(cx.listener(|this, _: &StartOrStop, _window, cx| {
                // Toggle: capturing → stop, idle → start. Mirrors
                // what the two Start / Stop buttons in
                // `render_controls` do; the buttons stay enabled
                // alongside the shortcut.
                if this.is_capturing {
                    this.stop_capture(cx);
                } else {
                    this.start_capture(cx);
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(bg)
            .text_color(fg)
            // Custom title bar — gpui-component's TitleBar widget
            // draws the "PortFinder" title + window controls. Lives
            // outside the `gap_3 + p_3` body so the title bar
            // touches the window edges; the body wrapper below
            // re-introduces the padding for the rest of the layout.
            // Required for Linux/Wayland (Mutter on GNOME doesn't
            // do server-side decorations, so without this the app
            // window has no chrome at all). On macOS the native
            // traffic lights overlay on top via
            // `traffic_light_position` in the WindowOptions.
            .child(
                TitleBar::new()
                    // Left: app name. Padded so it doesn't sit
                    // flush against the macOS traffic lights.
                    .child(div().pl_2().text_sm().child("PortFinder"))
                    // Right: stretches to fill, right-aligns the
                    // settings hamburger so it lands just inside
                    // the window controls (or just inside the
                    // right edge on macOS, where the traffic
                    // lights are on the left).
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_end()
                            .items_center()
                            .pr_1()
                            .child(self.render_settings_menu(cx)),
                    ),
            )
            .child(
                // Body wrapper — sibling to TitleBar, holds the
                // banner / cards / status / footer with the
                // standard p_3 + gap_3 spacing. Title bar lives
                // outside this so it stretches edge-to-edge.
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_3()
                    .when_some(banner, |this, el| this.child(el))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .bg(card_bg)
                            .child(controls),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .bg(card_bg)
                            .child(result_card),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(status_color)
                            .child(status_line),
                    )
                    .child({
                        // Footer: hairline top border separates the
                        // version line from the live status line
                        // above it. `pt_2` keeps the version text
                        // from sitting flush against the border.
                        // Right side carries the "Update available"
                        // pill when the boot-time check has surfaced
                        // a newer release and the user hasn't
                        // dismissed it.
                        let update_pill = self.render_update_pill(cx);
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .pt_2()
                            .border_t_1()
                            .border_color(border)
                            .text_xs()
                            .text_color(muted_fg)
                            .child(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .when_some(update_pill, |this, el| this.child(el))
                    }),
            )
    }
}

impl AppView {
    fn render_privilege_banner(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(status) = self.priv_status.clone() else {
            return div().w_0().h_0().into_any_element();
        };
        if status.has_access {
            return div().w_0().h_0().into_any_element();
        }

        let theme = cx.theme();
        let warning_bg = theme.warning.opacity(0.12);
        let warning_fg = theme.foreground;
        let border = theme.warning.opacity(0.4);

        let body: gpui::AnyElement = match status.platform.as_str() {
            "macos" if status.can_install => {
                let installing = self.is_installing;
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().child(
                        "PortFinder needs BPF access to capture packets. \
                         Installing the helper grants /dev/bpf* read access \
                         to the access_bpf group.",
                    ))
                    .child(
                        Button::new("install-bpf")
                            .label(if installing {
                                t!("button.install_bpf_helper_installing").into_owned()
                            } else {
                                t!("button.install_bpf_helper").into_owned()
                            })
                            .small()
                            .disabled(installing)
                            .tooltip(t!("button.install_bpf_helper_tooltip").into_owned())
                            .on_click(cx.listener(|this, _, _window, cx| this.install_bpf(cx))),
                    )
                    .into_any_element()
            }
            "linux" => div()
                .text_sm()
                .child(
                    "PortFinder needs CAP_NET_RAW or root to capture packets. \
                     Install the .deb / .rpm / .pkg.tar.zst package (it grants \
                     CAP_NET_RAW automatically) or relaunch with `sudo`.",
                )
                .into_any_element(),
            "windows" if !status.npcap_installed => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_sm().child(
                    t!("privilege.npcap_needed").into_owned(),
                ))
                .child(
                    Button::new("open-npcap")
                        .label(t!("button.download_npcap").into_owned())
                        .small()
                        .tooltip(t!("button.download_npcap_tooltip").into_owned())
                        .on_click(|_, _window, cx| {
                            cx.open_url("https://npcap.com/#download");
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(t!("privilege.npcap_relaunch").into_owned()),
                )
                .into_any_element(),
            "windows" if !status.npcap_non_admin => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_sm().child(
                    "Npcap was installed without the 'allow non-admin' option. \
                     Run PortFinder as Administrator or reinstall Npcap with \
                     non-admin support.",
                ))
                .into_any_element(),
            _ => div()
                .text_sm()
                .child(t!("privilege.needs_capture_privilege").into_owned())
                .into_any_element(),
        };

        div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(warning_bg)
            .text_color(warning_fg)
            .child(body)
            .into_any_element()
    }

    fn render_controls(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let is_capturing = self.is_capturing;
        let only_with_ips = self.only_with_ips;
        let theme = cx.theme();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(t!("controls.interface_label").into_owned()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .child(Select::new(&self.interface_select).small()),
                            )
                            .child(
                                Button::new("refresh-interfaces")
                                    .label("↻")
                                    .small()
                                    .tooltip(t!("controls.refresh_interfaces_tooltip").into_owned())
                                    .disabled(is_capturing)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.refresh_interfaces(window, cx)
                                    })),
                            ),
                    ),
            )
            .child(
                Switch::new("only-with-ips")
                    .checked(only_with_ips)
                    .label(t!("controls.only_with_ips").into_owned())
                    .small()
                    .disabled(is_capturing)
                    .on_click(cx.listener(|this, value: &bool, window, cx| {
                        this.toggle_only_with_ips(*value, window, cx)
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(t!("controls.protocol_label").into_owned()),
                    )
                    .child(Select::new(&self.protocol_select).small()),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        // `tooltip_with_action` renders the bound
                        // keystroke as a kbd glyph in the tooltip
                        // ("⌘R" on macOS, "Ctrl+R" elsewhere).
                        // The context must match the keymap scope
                        // we set in `run()` ("AppView") — that's
                        // the same context attached via
                        // `.key_context("AppView")` on the AppView
                        // root div, so gpui can resolve which
                        // binding to display.
                        Button::new("start-capture")
                            .label(t!("button.start").into_owned())
                            .primary()
                            .small()
                            .disabled(is_capturing)
                            .tooltip_with_action(
                                t!("button.start_tooltip").into_owned(),
                                &StartOrStop,
                                Some("AppView"),
                            )
                            .on_click(cx.listener(|this, _, _window, cx| this.start_capture(cx))),
                    )
                    .child(
                        Button::new("stop-capture")
                            .label(t!("button.stop").into_owned())
                            .small()
                            .disabled(!is_capturing)
                            .tooltip_with_action(
                                t!("button.stop_tooltip").into_owned(),
                                &StartOrStop,
                                Some("AppView"),
                            )
                            .on_click(cx.listener(|this, _, _window, cx| this.stop_capture(cx))),
                    ),
            )
            .into_any_element()
    }

    fn render_result_card(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        // While a capture is in flight, paint seven skeleton rows so
        // the user has a visual hint that work is happening — and so
        // the result card's footprint doesn't change shape when the
        // real values arrive (the skeletons match the eventual row
        // layout: short label box on the left, variable-width value
        // box on the right).
        if self.is_capturing {
            return Self::render_skeleton_rows();
        }

        let Some(result) = self.result.clone() else {
            // Idle / no-result state. Renders the standard
            // "Run a capture…" placeholder, plus a History
            // button below it when prior captures exist — gives
            // a path back to past data after the user has
            // started a fresh capture (which clears `result`
            // to None) but then cancelled / lost focus.
            let history_btn = self.render_history_popover(cx);
            let mut placeholder = div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("result.empty_placeholder").into_owned()),
                );
            if let Some(el) = history_btn {
                placeholder = placeholder.child(div().flex().justify_end().child(el));
            }
            return placeholder.into_any_element();
        };

        // Row labels are looked up via `t!()` so they translate
        // live with the active locale. `String` (not `&'static
        // str`) because t!() returns a Cow that may borrow the
        // active translation table at render time.
        let rows: [(ResultKey, String, String); 7] = [
            (
                ResultKey("switch"),
                t!("result.row.switch_name").into_owned(),
                result.switch_name.clone(),
            ),
            (
                ResultKey("ip"),
                t!("result.row.switch_ip").into_owned(),
                result.switch_ip.clone(),
            ),
            (
                ResultKey("port"),
                t!("result.row.switch_port").into_owned(),
                result.switch_port.clone(),
            ),
            (
                ResultKey("vlan"),
                t!("result.row.vlan").into_owned(),
                result.native_vlan.clone(),
            ),
            (
                ResultKey("voiceVlan"),
                t!("result.row.voice_vlan").into_owned(),
                result.voice_vlan.clone(),
            ),
            (
                ResultKey("mtu"),
                t!("result.row.mtu").into_owned(),
                result.mtu.clone(),
            ),
            (
                ResultKey("model"),
                t!("result.row.switch_model").into_owned(),
                result.switch_model.clone(),
            ),
        ];

        let json_copied = self.copied_key == Some(RESULT_KEY_JSON);
        let success = cx.theme().success;
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        // Built before the row loop so the borrow of `self`
        // ends before the loop's `self.render_result_row` —
        // `Option<AnyElement>` is `'static`, so storing it
        // ahead of time is free.
        let history_btn = self.render_history_popover(cx);
        let mut col = div().flex().flex_col().gap_1();
        for (key, label, raw) in rows {
            col = col.child(self.render_result_row(key, label, raw, cx));
        }
        // "Copy as JSON" footer. Right-aligned, below the seven
        // value rows. Mirrors the CLI's `--json` output format
        // (same `serde_json::to_string_pretty` on the same
        // `CaptureResult` struct), so a value pasted from the
        // GUI is byte-identical to what `portfinder-cli capture
        // --json` would have produced. Pressing the button puts
        // the JSON on the system clipboard and flashes "Copied"
        // for 1.2 s via the existing `copy_value` → `copied_key`
        // → timer pattern that the per-row value buttons use —
        // the row state and this state share the same field, so
        // copying a row value cancels the "Copied" indicator
        // here and vice versa. Acceptable: only one copy can
        // realistically be in flight at a time anyway.
        col = col.child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                // Top border + extra top padding separates the
                // action-button row from the seven value rows
                // above. Matches the divider style the About
                // section uses in the settings popover.
                .pt_2()
                .mt_1()
                .border_t_1()
                .border_color(border)
                .when(json_copied, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(success)
                            .child(t!("button.copied").into_owned()),
                    )
                })
                .when_some(history_btn, |this, el| this.child(el))
                .child(
                    Button::new("copy-result-json")
                        .label(t!("button.copy_as_json").into_owned())
                        .ghost()
                        .small()
                        .tooltip(t!("button.copy_as_json_tooltip").into_owned())
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            let Some(r) = this.result.as_ref() else { return };
                            let json = match serde_json::to_string_pretty(r) {
                                Ok(s) => s,
                                Err(e) => {
                                    log::warn!("copy result as json failed: {e}");
                                    return;
                                }
                            };
                            this.copy_value(RESULT_KEY_JSON, json, cx);
                        })),
                )
                .text_color(muted),
        );
        col.into_any_element()
    }

    /// Seven gpui-component `Skeleton` rows shown while a capture
    /// is in flight. Per-row value widths are hand-tuned to mimic
    /// the natural variability of the actual fields (Switch Model
    /// runs long, VLAN numbers run short) so the skeleton reads as
    /// a preview of the real card rather than a uniform bar chart.
    /// Skeleton's own pulse animation drives the loading affordance.
    fn render_skeleton_rows() -> gpui::AnyElement {
        // Per-row value widths, in px. Index aligns with the field
        // order in render_result_card: switch name, IP, port, VLAN,
        // voice VLAN, MTU, model.
        const VALUE_WIDTHS_PX: [f32; 7] = [160.0, 110.0, 70.0, 40.0, 40.0, 60.0, 180.0];

        let mut col = div().flex().flex_col().gap_1();
        for &w in &VALUE_WIDTHS_PX {
            col = col.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .h(px(24.0))
                    .child(Skeleton::new().w(px(72.0)).h_3())
                    .child(Skeleton::new().w(px(w)).h_3()),
            );
        }
        col.into_any_element()
    }

    /// "History (N)" button + popover, surfaced next to the
    /// "Copy as JSON" button on the result card. Returns
    /// `None` when the history is empty so the caller skips
    /// rendering altogether (a "History (0)" button with
    /// nothing to show would be just noise). Snapshots
    /// `self.history` into the popover closure at construct
    /// time — if a new capture lands while the popover is
    /// open, it shows the old list until the popover is
    /// reopened. Acceptable: the popover is short-lived and
    /// the new capture is also visible in the result card
    /// behind the popover.
    ///
    /// Each row renders a relative timestamp ("2m ago"),
    /// protocol + interface, and switch name + IP. Click →
    /// `restore_from_history` swaps `self.result` to that
    /// entry's data. The popover's own outside-click handler
    /// closes it on the next mouse-up.
    fn render_history_popover(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if self.history.is_empty() {
            return None;
        }

        // Snapshot history newest-first. `iter().rev()` over
        // a deque is cheap (it's a doubly-ended queue) so we
        // don't pay for a reversed `Vec` allocation.
        let entries: Vec<(usize, HistoryEntry)> = self
            .history
            .iter()
            .enumerate()
            .rev()
            .map(|(ix, e)| (ix, e.clone()))
            .collect();
        let count = entries.len();
        let entity = cx.entity().clone();
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let hover_bg = theme.muted;

        Some(
            Popover::new("history-popover")
                .trigger(
                    Button::new("history-trigger")
                        .label(t!("history.button", count = count).into_owned())
                        .ghost()
                        .small()
                        .tooltip(t!("history.button_tooltip").into_owned()),
                )
                .content(move |_, _, _| {
                    let mut col = div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .w(px(320.0))
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .pb_1()
                                .child(t!("history.header", count = count).into_owned()),
                        );
                    for (ix, entry) in entries.iter().cloned() {
                        let entity_for_click = entity.clone();
                        let entity_for_right_click = entity.clone();
                        let switch_name = if entry.result.switch_name.is_empty()
                            || entry.result.switch_name == "N/A"
                        {
                            t!("history.no_switch_name").into_owned()
                        } else {
                            entry.result.switch_name.clone()
                        };
                        let switch_ip = if entry.result.switch_ip.is_empty()
                            || entry.result.switch_ip == "N/A"
                        {
                            t!("history.ip_unknown").into_owned()
                        } else {
                            entry.result.switch_ip.clone()
                        };
                        let iface = if entry.interface_name.is_empty() {
                            t!("history.iface_all").into_owned()
                        } else {
                            entry.interface_name.clone()
                        };
                        // Port is the field most relevant to
                        // "which cable did I plug into" — surface
                        // it on the meta line when advertised.
                        // Drop the segment entirely on "N/A" /
                        // empty rather than rendering the literal
                        // sentinel.
                        let port_segment = if entry.result.switch_port.is_empty()
                            || entry.result.switch_port == "N/A"
                        {
                            String::new()
                        } else {
                            format!(" · {}", entry.result.switch_port)
                        };
                        let meta = format!(
                            "{} · {} on {}{}",
                            format_ago(entry.captured_at_epoch_secs),
                            entry.protocol.as_str(),
                            iface,
                            port_segment,
                        );
                        col = col.child(
                            div()
                                .id(("history-row", ix))
                                .cursor_pointer()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .p_2()
                                .border_t_1()
                                .border_color(border)
                                .hover(|this| this.bg(hover_bg))
                                .tooltip(|window, cx| -> AnyView {
                                    let text: SharedString =
                                        t!("history.row_tooltip").into_owned().into();
                                    Tooltip::element(move |_, _| {
                                        div().w(px(220.0)).text_sm().child(text.clone())
                                    })
                                    .build(window, cx)
                                })
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted)
                                        .child(meta),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .text_sm()
                                        .child(div().truncate().min_w_0().child(switch_name))
                                        .child(
                                            div()
                                                .flex_shrink_0()
                                                .text_color(muted)
                                                .child(switch_ip),
                                        ),
                                )
                                .on_click(move |_, _window, cx| {
                                    entity_for_click.update(cx, |this, cx| {
                                        this.restore_from_history(ix, cx);
                                    });
                                })
                                // Right-click → silently copy that
                                // entry's CaptureResult as JSON
                                // (same `serde_json::to_string_pretty`
                                // shape the CLI / "Copy as JSON"
                                // footer button produce). Silent:
                                // the popover stays open on right-
                                // click, no banner/toast feedback —
                                // the discoverable hint is the row
                                // tooltip above. Pragmatic: an
                                // explicit "Copied" indicator
                                // inside a popover that closes on
                                // outside-click is awkward.
                                .on_mouse_down(
                                    gpui::MouseButton::Right,
                                    move |_, _window, cx| {
                                        entity_for_right_click.update(cx, |this, cx| {
                                            this.copy_history_entry_json(ix, cx);
                                        });
                                    },
                                ),
                        );
                    }
                    col.into_any_element()
                })
                .into_any_element(),
        )
    }

    /// Copy a history entry's `CaptureResult` as JSON, same
    /// format the CLI's `--json` flag and the result-card's
    /// "Copy as JSON" footer button produce. Bound to right-
    /// click on each row in `render_history_popover`. Silent —
    /// the popover stays open, no toast feedback. The row's
    /// tooltip hints at the gesture's existence.
    fn copy_history_entry_json(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.history.get(ix) else { return };
        let json = match serde_json::to_string_pretty(&entry.result) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("copy history entry as json failed: {e}");
                return;
            }
        };
        log::info!("copy_history_entry_json: ix={ix}");
        cx.write_to_clipboard(ClipboardItem::new_string(json));
    }

    /// Reinstate the result + status text from a past
    /// capture. Doesn't touch the interface picker or
    /// protocol selector — those control the *next* capture
    /// and resetting them would be surprising. The status
    /// line lets the user see the entry didn't come from a
    /// live capture.
    fn restore_from_history(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.history.get(ix).cloned() else {
            return;
        };
        log::info!(
            "restore_from_history: ix={ix} protocol={} iface={}",
            entry.protocol.as_str(),
            entry.interface_name,
        );
        self.result = Some(entry.result);
        self.error.clear();
        self.copied_key = None;
        // Result card transitions to the populated height
        // (or stays there, if a prior result was already
        // showing). Flag resize either way — `resize_pending`
        // is cheap and the render-time guard skips no-ops.
        self.resize_pending = true;
        let iface_label = if entry.interface_name.is_empty() {
            t!("history.iface_all").into_owned()
        } else {
            entry.interface_name.clone()
        };
        self.status_text = t!(
            "status.restored_from_history",
            protocol = entry.protocol.as_str(),
            iface = iface_label,
            ago = format_ago(entry.captured_at_epoch_secs),
        )
        .into_owned()
        .into();
        cx.notify();
    }

    /// Footer "Update available" pill. Returns `None` when there's
    /// no update to surface (either the check came back empty or
    /// the user dismissed the pill for this session). The text half
    /// opens the GitHub release page in the user's browser on
    /// click; the trailing ✕ dismisses without navigating away.
    /// Title-bar hamburger button. Click → popover with the
    /// `debug_log` toggle + an "Open log folder" shortcut. Default
    /// state for the toggle is OFF — a fresh install never drops
    /// a log file anywhere until the user explicitly enables it
    /// from this menu. `init_logging` only reads the setting at
    /// process start, so flipping the toggle mid-session sets a
    /// `settings_dirty` flag that the popover surfaces as a
    /// "restart to apply" hint underneath the switch.
    fn render_settings_menu(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let debug_log = self.settings.debug_log;
        let persist_history = self.settings.persist_history;
        let view = self.settings_view;
        let language_code = self
            .settings
            .language
            .clone()
            .unwrap_or_else(|| crate::i18n::resolve(None).to_string());
        // Capture the AppView entity so the Switch's `on_click`
        // can mutate `self.settings` from inside the popover's
        // `.content(...)` callback. The popover renders in an
        // overlay layer that sits outside the AppView's focus
        // tree, so a `cx.dispatch_action` from in there doesn't
        // bubble up to a listener on the root div — we use the
        // explicit `entity.update(cx, ...)` form instead.
        // `cx.entity()` is a cheap Arc clone.
        //
        // The log-level Slider doesn't need a similar capture
        // because `SliderState::Change` is delivered via the
        // `cx.subscribe(&log_level_slider, ...)` we set up in
        // `AppView::new` — that handler already has `&mut self`
        // and writes through to settings + `log::set_max_level`
        // + disk.
        let entity = cx.entity().clone();
        let log_level_slider = self.log_level_slider.clone();
        // About-section snapshots. Captured here (not inside the
        // popover closure) because `priv_status` lives on
        // `AppView` and we want a stable value for the lifetime
        // of this popover render. `privilege_label` is the
        // platform-specific (label, status) tuple the BPF /
        // Npcap / CAP_NET_RAW row should show; `None` means we
        // haven't probed yet and the row gets skipped.
        let version = env!("CARGO_PKG_VERSION");
        let privilege_label = self.priv_status.as_ref().map(privilege_label_for);
        // 280 px is enough for the Switch row, the slider with
        // its endpoint labels, and the About rows below — the
        // slider stretches to fill, so we don't need to widen
        // for label fit.
        let panel_width = px(280.0);

        Popover::new("settings-popover")
            .trigger(
                Button::new("settings-trigger")
                    .icon(IconName::Menu)
                    .ghost()
                    .small(),
            )
            .content(move |_, _, cx| {
                let entity_for_history = entity.clone();
                let entity_for_switch = entity.clone();
                let entity_for_about = entity.clone();
                let muted = cx.theme().muted_foreground;
                let border = cx.theme().border;

                // -- Capture section --------------------------
                // Just the "Save capture history" switch for
                // now; sized as a labelled section so the
                // future is open (export-on-capture, max-
                // history-N, etc.).
                let capture_section = div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_section_header(
                        t!("settings.section.capture").into_owned(),
                        muted,
                    ))
                    .child(settings_switch_row(
                        "persist-history",
                        IconName::Inbox,
                        t!("settings.save_capture_history").into_owned(),
                        persist_history,
                        muted,
                        move |new, cx| {
                            entity_for_history.update(cx, |this, cx| {
                                this.settings.persist_history = new;
                                if new {
                                    if let Err(e) = settings::save_history(&this.history) {
                                        log::warn!("save history failed: {e}");
                                    }
                                } else {
                                    settings::clear_history_file();
                                }
                                if let Err(e) = this.settings.save() {
                                    log::warn!("settings save failed: {e}");
                                }
                                cx.notify();
                            });
                        },
                    ));

                // -- Logging section --------------------------
                // Log level comes before "Write debug log"
                // because the level controls verbosity for
                // EVERY logger output, not just the on-disk
                // file — a parallel `portfinder-cli` run
                // honours `settings.log_level` whether the GUI
                // toggle is on or off.
                let logging_section = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .pt_3()
                    .border_t_1()
                    .border_color(border)
                    .child(settings_section_header(
                        t!("settings.section.logging").into_owned(),
                        muted,
                    ))
                    // Log level: icon + label header row, then
                    // the slider, then ticks + labels. Slider
                    // change events are routed via the
                    // `_log_level_sub` subscription in
                    // `AppView::new` — that handler snaps the
                    // thumb to a stop and persists.
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(settings_row_header(
                                IconName::Settings2,
                                t!("settings.log_level").into_owned(),
                                muted,
                            ))
                            .child(Slider::new(&log_level_slider))
                            .child(
                                // Tick row: three 2x4 px
                                // notches at 0% / 50% / 100% of
                                // the bar, the only visual cue
                                // that the slider has discrete
                                // stops (the bar itself is
                                // continuous).
                                div()
                                    .flex()
                                    .justify_between()
                                    .child(div().w(px(2.0)).h(px(4.0)).bg(muted))
                                    .child(div().w(px(2.0)).h(px(4.0)).bg(muted))
                                    .child(div().w(px(2.0)).h(px(4.0)).bg(muted)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_xs()
                                    .text_color(muted)
                                    .child(log_level_label(
                                        "log-level-label-normal",
                                        t!("log_level.normal").into_owned(),
                                        t!("log_level.normal_description").into_owned(),
                                    ))
                                    .child(log_level_label(
                                        "log-level-label-verbose",
                                        t!("log_level.verbose").into_owned(),
                                        t!("log_level.verbose_description").into_owned(),
                                    ))
                                    .child(log_level_label(
                                        "log-level-label-trace",
                                        t!("log_level.trace").into_owned(),
                                        t!("log_level.trace_description").into_owned(),
                                    )),
                            ),
                    )
                    .child(settings_switch_row(
                        "debug-log",
                        IconName::SquareTerminal,
                        t!("settings.write_debug_log").into_owned(),
                        debug_log,
                        muted,
                        move |new, cx| {
                            entity_for_switch.update(cx, |this, cx| {
                                this.settings.debug_log = new;
                                // Live on/off — drop or open
                                // the log file immediately, no
                                // restart needed.
                                settings::set_logging_enabled(new);
                                if let Err(e) = this.settings.save() {
                                    log::warn!("settings save failed: {e}");
                                }
                                cx.notify();
                            });
                        },
                    ));

                // -- Language row -----------------------------
                // Drill-down to the language picker sub-page
                // (same pattern as About below). Displays the
                // current locale's endonym ("English",
                // "Español", "日本語", …) so the row text
                // tracks the active language. Click flips
                // `settings_view` to `Language`.
                let entity_for_lang_open = entity_for_about.clone();
                let lang_label = crate::i18n::display_name(&language_code).to_string();
                let language_row = div()
                    .id("settings-language-row")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(settings_row_header(
                        IconName::Globe,
                        t!("settings.language_row").into_owned(),
                        muted,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().text_color(muted).child(lang_label))
                            .child(Icon::new(IconName::ChevronRight).text_color(muted)),
                    )
                    .on_click(move |_, _window, cx| {
                        entity_for_lang_open.update(cx, |this, cx| {
                            this.settings_view = SettingsView::Language;
                            cx.notify();
                        });
                    });

                // -- About row --------------------------------
                // Drill-down entry to the About page rather
                // than an inline expandable section — gpui-
                // component's Popover doesn't open below the
                // trigger (its `resolved_corner` math has no
                // "below the trigger" anchor), so a tall
                // settings list gets clipped at the window
                // bottom. Splitting About into its own page
                // inside the same popover keeps both views
                // short. Click sets `settings_view =
                // About`, which the outer `match view`
                // below swaps in the About page.
                let entity_for_about_open = entity_for_about.clone();
                let about_row = div()
                    .id("settings-about-row")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pt_3()
                    .border_t_1()
                    .border_color(border)
                    .child(settings_row_header(
                        IconName::Info,
                        t!("settings.about_row").into_owned(),
                        muted,
                    ))
                    .child(Icon::new(IconName::ChevronRight).text_color(muted))
                    .on_click(move |_, _window, cx| {
                        entity_for_about_open.update(cx, |this, cx| {
                            this.settings_view = SettingsView::About;
                            cx.notify();
                        });
                    });

                // -- Folders row ------------------------------
                // Two outline buttons, gap_2, right-aligned —
                // macOS-System-Settings convention for the
                // trailing action(s) on a panel ("Shortcuts…"
                // / "Hot Corners…" sit this way). Order is
                // "config first, output second" — `settings.json`
                // + `history.json` live in the first dir;
                // `portfinder.log` in the second.
                let folders_row = div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .pt_3()
                    .border_t_1()
                    .border_color(border)
                    .child(
                        Button::new("settings-open-settings-folder")
                            .icon(IconName::FolderOpen)
                            .label(t!("button.settings_folder").into_owned())
                            .outline()
                            .small()
                            .on_click(|_, _window, _cx| {
                                settings::reveal_settings_folder();
                            }),
                    )
                    .child(
                        Button::new("settings-open-log-folder")
                            .icon(IconName::FolderOpen)
                            .label(t!("button.log_folder").into_owned())
                            .outline()
                            .small()
                            .on_click(|_, _window, _cx| {
                                settings::reveal_log_folder();
                            }),
                    );

                match view {
                    SettingsView::About => {
                        // Drill-down view. Same popover, "← Back"
                        // header flips `settings_view` back to
                        // `Main` and the next render returns to
                        // the settings list.
                        let entity_for_back = entity.clone();
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .w(panel_width)
                            .child(back_header(
                                "settings-about-back",
                                &t!("settings.about_back"),
                                muted,
                                entity_for_back,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .pt_2()
                                    .border_t_1()
                                    .border_color(border)
                                    .child(about_row_with_icon(
                                        IconName::Info,
                                        t!("about.version"),
                                        div()
                                            .text_sm()
                                            .text_color(muted)
                                            .child(format!("v{}", version))
                                            .into_any_element(),
                                        muted,
                                    ))
                                    .child(about_row_with_icon(
                                        IconName::Github,
                                        t!("about.repository"),
                                        Button::new("about-github")
                                            .label("GitHub ↗")
                                            .ghost()
                                            .small()
                                            .on_click(|_, _window, cx| {
                                                cx.open_url(
                                                    "https://github.com/packetThrower/PortFinder",
                                                );
                                            })
                                            .into_any_element(),
                                        muted,
                                    ))
                                    .child(about_row_with_icon(
                                        IconName::ExternalLink,
                                        t!("about.license"),
                                        Button::new("about-license")
                                            .label("GPL-3.0-or-later ↗")
                                            .ghost()
                                            .small()
                                            .on_click(|_, _window, cx| {
                                                cx.open_url(
                                                    "https://github.com/packetThrower/PortFinder/\
                                                     blob/main/LICENSE",
                                                );
                                            })
                                            .into_any_element(),
                                        muted,
                                    ))
                                    .when_some(privilege_label.clone(), |this, (label, status)| {
                                        this.child(about_row_with_icon(
                                            IconName::Network,
                                            label,
                                            div()
                                                .text_sm()
                                                .text_color(muted)
                                                .child(status)
                                                .into_any_element(),
                                            muted,
                                        ))
                                    }),
                            )
                            .into_any_element()
                    }
                    SettingsView::Language => {
                        // Drill-down language picker. Each
                        // `SUPPORTED` locale renders as a
                        // clickable row showing its endonym
                        // ("English", "Español", "日本語", …);
                        // the active one gets a leading check.
                        // Click sets `settings.language` +
                        // `rust_i18n::set_locale`, saves, and
                        // returns to the main view. Next render
                        // picks up the new locale on every t!()
                        // lookup — no relaunch.
                        let entity_for_back = entity.clone();
                        let active_code = language_code.clone();
                        let mut col = div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .w(panel_width)
                            .child(back_header(
                                "settings-language-back",
                                &t!("settings.section.language"),
                                muted,
                                entity_for_back,
                            ));
                        let mut rows = div()
                            .flex()
                            .flex_col()
                            .pt_2()
                            .border_t_1()
                            .border_color(border);
                        for (code, name) in crate::i18n::SUPPORTED.iter().copied() {
                            let entity_for_pick = entity.clone();
                            let selected = active_code == code;
                            rows = rows.child(
                                div()
                                    .id(SharedString::from(format!(
                                        "settings-language-pick-{code}"
                                    )))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .p_2()
                                    .hover(|this| this.bg(cx.theme().muted))
                                    .child(div().text_sm().child(name))
                                    .child(if selected {
                                        Icon::new(IconName::Check)
                                            .text_color(cx.theme().success)
                                            .into_any_element()
                                    } else {
                                        div().w(px(16.0)).h(px(16.0)).into_any_element()
                                    })
                                    .on_click(move |_, _window, cx| {
                                        entity_for_pick.update(cx, |this, cx| {
                                            this.settings.language = Some(code.to_string());
                                            rust_i18n::set_locale(code);
                                            if let Err(e) = this.settings.save() {
                                                log::warn!("settings save failed: {e}");
                                            }
                                            this.settings_view = SettingsView::Main;
                                            cx.notify();
                                        });
                                    }),
                            );
                        }
                        col = col.child(rows);
                        col.into_any_element()
                    }
                    SettingsView::Main => div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_3()
                        .w(panel_width)
                        .child(language_row)
                        .child(capture_section)
                        .child(logging_section)
                        .child(about_row)
                        .child(folders_row)
                        .into_any_element(),
                }
            })
            .into_any_element()
    }

    fn render_update_pill(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let info = self.update_available.clone()?;
        if self.update_dismissed {
            return None;
        }
        let url = info.html_url.clone();
        Some(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    Button::new("update-pill-open")
                        .label(t!("update.available", version = &info.version).into_owned())
                        .ghost()
                        .small()
                        .tooltip(t!("update.tooltip").into_owned())
                        .on_click(move |_, _window, cx| {
                            cx.open_url(&url);
                        }),
                )
                .child(
                    // No tooltip on the dismiss ✕ — gpui-component's
                    // Tooltip overlay doesn't track its trigger
                    // across removals, so when the pill is dismissed
                    // mid-hover the tooltip orphans and lingers on
                    // screen until the cursor moves over another
                    // hoverable region. The ✕ glyph is universally
                    // understood as "close" / "dismiss", so the
                    // tooltip wasn't carrying real informational
                    // weight anyway. The main pill's tooltip
                    // ("Click to view this release on GitHub") still
                    // tells the user what the pill itself is.
                    Button::new("update-pill-dismiss")
                        .label("✕")
                        .ghost()
                        .small()
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.update_dismissed = true;
                            // Footer pill disappears → window
                            // shrinks by HEIGHT_UPDATE_PILL_EXTRA.
                            // Flag resize.
                            this.resize_pending = true;
                            cx.notify();
                        })),
                )
                .into_any_element(),
        )
    }

    fn render_result_row(
        &mut self,
        key: ResultKey,
        label: String,
        raw: String,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let success = theme.success;
        let absent = raw.is_empty() || raw == "N/A";
        let copied = self.copied_key == Some(key);
        // ElementId is constructable from `(&'static str, u64)` —
        // hash the ResultKey's stable static string pointer into
        // a u64 so each row gets a unique id without allocating
        // a SharedString per render. Label is now dynamic (t!()
        // output) so we can't use its pointer; key.0 stays
        // 'static across locale switches.
        let row_id = key.0.as_ptr() as u64;

        // The right-hand cell. For long values (Switch Model on the
        // SG350 advertises ~80 chars) we truncate with an ellipsis
        // rather than letting the text run off the window edge.
        // `.truncate()` is gpui's shortcut for the three CSS bits
        // that make ellipsis actually appear: `overflow: hidden`,
        // `white-space: nowrap`, `text-overflow: ellipsis`. The
        // outer flex parent on the value cell also needs
        // `.min_w_0()` — without it the parent's `min-width: auto`
        // defaults keep the cell at its content width and the
        // ellipsis never fires.
        let value_el: gpui::AnyElement = if absent {
            div()
                .italic()
                .text_color(muted)
                .child(t!("result.not_advertised").into_owned())
                .into_any_element()
        } else {
            let copy_payload = raw.clone();
            div()
                .id(("copy-value", row_id))
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .min_w_0()
                .w_full()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_right()
                        .child(raw),
                )
                .child(if copied {
                    div()
                        .flex_shrink_0()
                        .text_color(success)
                        .child("✓")
                        .into_any_element()
                } else {
                    div().w_0().h_0().into_any_element()
                })
                .on_mouse_up(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _, _window, cx| {
                        this.copy_value(key, copy_payload.clone(), cx)
                    }),
                )
                .into_any_element()
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .w(px(96.0))
                    .flex_shrink_0()
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .justify_end()
                    .child(value_el),
            )
            .into_any_element()
    }
}

fn build_interface_opts(interfaces: &[InterfaceInfo], only_with_ips: bool) -> Vec<Opt> {
    let mut opts = Vec::new();
    for iface in interfaces {
        if !iface.name.is_empty() && only_with_ips && !iface.has_ip {
            continue;
        }
        let title = format_interface_title(iface);
        opts.push(Opt::new(title, iface.name.clone()));
    }
    if opts.is_empty() {
        opts.push(Opt::new("Sniff all interfaces", ""));
    }
    opts
}

fn format_interface_title(iface: &InterfaceInfo) -> String {
    if iface.name.is_empty() {
        return "Sniff all interfaces".into();
    }
    let display = if iface.description.is_empty() {
        iface.name.clone()
    } else {
        iface.description.clone()
    };
    let compact = compact_address(&iface.addresses);
    if compact.is_empty() {
        display
    } else {
        format!("{display} ({compact})")
    }
}

fn compact_address(addrs: &str) -> String {
    if addrs.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = addrs.split(", ").collect();
    let v4 = parts.iter().find(|p| {
        // Dotted-quad heuristic. `split('.').count() == 4` reads the
        // same as the Tauri-era `matches('.').count() == 3` did but
        // sidesteps an ambiguous-method-resolution headache with
        // `gpui_component::select::SelectItem::matches` (also takes
        // `&str`) that the compiler can't auto-pick between when
        // `&str` is in deref scope from the iterator's `&&str`.
        p.split('.').count() == 4 && p.chars().all(|c| c.is_ascii_digit() || c == '.')
    });
    match v4 {
        Some(s) => (*s).to_string(),
        None => parts.first().copied().unwrap_or("").to_string(),
    }
}

fn protocol_opts() -> Vec<Opt> {
    vec![
        Opt::new("LLDP", "LLDP"),
        Opt::new("CDP", "CDP"),
        Opt::new("MNDP", "MNDP"),
    ]
}

/// One "Normal" / "Verbose" / "Trace" label under the log-level
/// slider, paired with a hover tooltip describing what that level
/// captures.
///
/// Tooltip content is built via `Tooltip::element` and wrapped in a
/// fixed-width `div` so the text wraps. `Tooltip::new(text)` puts
/// the string directly inside `h_flex`, where the intrinsic
/// single-line width wins over any `max_w` we set on the tooltip
/// itself — the box stretches and overflows the popover. A `div`
/// with `.w(220px)` pins the line-break boundary.
/// "2m ago" / "1h ago" / "3d ago" style relative timestamp
/// for the History popover. Resolution drops to whole units —
/// granularity isn't useful for "did I capture this before or
/// after that other port" reasoning, just "roughly when."
/// Day-level rollover is here for persisted entries that
/// survive past midnight; session-only history never makes
/// it that far.
///
/// Input is wall-clock seconds since UNIX_EPOCH (see
/// `HistoryEntry.captured_at_epoch_secs`). `saturating_sub`
/// guards against a system-clock jump that would put `now`
/// before the entry's stamp — we'd briefly render "just now"
/// rather than panic on an arithmetic underflow.
fn format_ago(captured_at_epoch_secs: u64) -> String {
    let secs = now_epoch_secs().saturating_sub(captured_at_epoch_secs);
    if secs < 5 {
        t!("format.just_now").into_owned()
    } else if secs < 60 {
        t!("format.seconds_ago", n = secs).into_owned()
    } else if secs < 3600 {
        t!("format.minutes_ago", n = secs / 60).into_owned()
    } else if secs < 86_400 {
        t!("format.hours_ago", n = secs / 3600).into_owned()
    } else {
        t!("format.days_ago", n = secs / 86_400).into_owned()
    }
}

/// Seconds since UNIX_EPOCH right now, as a `u64`. Defaults
/// to 0 (i.e., 1970-01-01) if the system clock somehow
/// pre-dates UNIX_EPOCH; the relative-time math gracefully
/// degrades to "just now" via `saturating_sub` rather than
/// blowing up.
fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn log_level_label(
    id: &'static str,
    label: impl Into<SharedString>,
    description: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    let description = description.into();
    div().id(id).child(label).tooltip(move |window, cx| -> AnyView {
        let description = description.clone();
        Tooltip::element(move |_, _| {
            div().w(px(220.0)).text_sm().child(description.clone())
        })
        .build(window, cx)
    })
}

/// Small "section heading" label for the settings popover —
/// a single `text_xs` muted-tone line above each group of
/// rows ("Capture", "Logging", "About"). Lowercase letter-
/// spacing isn't macOS-System-Settings (those use Bold caps);
/// staying with plain xs muted keeps the popover legible
/// against gpui-component's default theme without bringing
/// in a custom font weight.
fn settings_section_header(label: impl Into<SharedString>, muted: Hsla) -> gpui::Div {
    div().text_xs().text_color(muted).child(label.into())
}

/// Leading icon + label for a settings row that doesn't have
/// a `justify_between` shape — currently just the Log level
/// slider's header line, which sits above a full-width
/// Slider. Icon is rendered in the muted-tone since the
/// label colour drives the row's hierarchy, not the icon.
fn settings_row_header(
    icon: IconName,
    label: impl Into<SharedString>,
    muted: Hsla,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(Icon::new(icon).text_color(muted))
        .child(div().text_sm().child(label.into()))
}

/// Standard "icon + label on the left, Switch on the right"
/// settings-popover row. Used by Capture/Save-history and
/// Logging/Write-debug-log; the `on_click` closure carries
/// the row's persistence + side-effect logic so this helper
/// stays generic.
fn settings_switch_row(
    id: &'static str,
    icon: IconName,
    label: impl Into<SharedString>,
    checked: bool,
    muted: Hsla,
    on_click: impl Fn(bool, &mut App) + 'static,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(settings_row_header(icon, label, muted))
        .child(
            Switch::new(SharedString::from(format!("settings-switch-{id}")))
                .checked(checked)
                .small()
                .on_click(move |value: &bool, _window, cx| on_click(*value, cx)),
        )
}

/// "← Back" header that sits at the top of each settings sub-
/// page (About, Language). Clicking it sets `settings_view`
/// back to `Main` via the captured AppView entity. The drill-
/// down pattern matches macOS System Settings panels.
fn back_header(
    id: &'static str,
    label: &str,
    muted: Hsla,
    entity: Entity<AppView>,
) -> gpui::Stateful<gpui::Div> {
    let owned: SharedString = label.to_string().into();
    div()
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .gap_2()
        .pb_1()
        .child(Icon::new(IconName::ChevronLeft).text_color(muted))
        .child(div().text_sm().child(owned))
        .on_click(move |_, _window, cx| {
            entity.update(cx, |this, cx| {
                this.settings_view = SettingsView::Main;
                cx.notify();
            });
        })
}

/// One row in the settings-popover "About" section. Leading
/// icon, label, then a value-or-control column flush right.
/// macOS System-Settings convention: full-width
/// `justify_between` so the value/control hangs against the
/// trailing edge regardless of label length. The value
/// column accepts any `AnyElement` so it can be a muted text
/// label (version, status) or a `Button` (link).
fn about_row_with_icon(
    icon: IconName,
    label: impl Into<SharedString>,
    value: gpui::AnyElement,
    muted: Hsla,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(settings_row_header(icon, label, muted))
        .child(value)
}

/// Maps `PrivilegeStatus` to the (row label, status text) the
/// "About" section's capture-privilege row should display. The
/// label is platform-specific so it matches the terminology the
/// privilege banner already uses for the same condition — macOS
/// users hear "BPF helper", Windows users "Npcap", and Linux
/// users get the generic "Capture access" (no single named
/// component to point at — it's `setcap` on the binary, set by
/// the .deb / .rpm postinstall).
fn privilege_label_for(status: &privilege::PrivilegeStatus) -> (String, String) {
    if cfg!(target_os = "macos") {
        (
            t!("about.bpf_helper").into_owned(),
            if status.helper_installed {
                t!("about.installed").into_owned()
            } else {
                t!("about.not_installed").into_owned()
            },
        )
    } else if cfg!(target_os = "windows") {
        (
            t!("about.npcap").into_owned(),
            if status.npcap_installed {
                t!("about.installed").into_owned()
            } else {
                t!("about.not_installed").into_owned()
            },
        )
    } else {
        (
            t!("about.capture_access").into_owned(),
            if status.has_access {
                t!("about.available").into_owned()
            } else {
                t!("about.unavailable").into_owned()
            },
        )
    }
}

/// Main GUI entrypoint. Starts gpui, opens the PortFinder window,
/// and runs until the user closes it. Spawned by `main.rs` when no
/// CLI args were passed.
pub fn run() {
    // Touch the tokio runtime so it spins up before the first
    // capture click. Cheap; the runtime itself lives on its own
    // thread pool.
    let _ = tokio_handle();

    // `gpui_platform::application()` is the canonical entry point
    // for the Zed-git gpui line; crates.io's `Application::new()`
    // would compile but doesn't exist on this fork. `with_assets`
    // registers gpui-component's bundled icon SVGs as the app's
    // asset source so any `IconName::*` referenced by widget code
    // resolves to a real glyph.
    // `QuitMode::LastWindowClosed` makes the red-traffic-light close
    // (or any other "last window dismissed" path) terminate the
    // process. gpui's default on macOS is `Explicit` — modelled
    // after Zed and other multi-window apps where the menu bar stays
    // alive after the last window closes. PortFinder is single-
    // window: nobody expects it to linger in the Dock after they
    // close it, and the matching dock-icon + ⌘+Q wiring would feel
    // pointless if the close button didn't also exit. Behaviour on
    // Linux / Windows is unchanged (`LastWindowClosed` was already
    // the default there).
    let app = gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);
    app.run(move |cx: &mut App| {
        // gpui-component widgets (Input, Form, Select, Switch, …)
        // require the Theme global to be installed before the first
        // render — without this, the very first `Select::new` panics
        // looking for `Theme`.
        gpui_component::init(cx);

        // Wire Cmd+Q. Three pieces:
        //   1. The keybinding (`cmd-q` works across macOS / Linux /
        //      Windows — gpui maps `cmd` to the platform's primary
        //      modifier).
        //   2. The Application menu so macOS shows "Quit PortFinder
        //      ⌘Q" under the app's menu-bar entry (matches every
        //      Mac user's muscle memory; without a `cx.set_menus`
        //      call gpui doesn't install any menu at all and the
        //      menu bar shows nothing app-specific).
        //   3. The handler that actually exits — `cx.quit()` runs
        //      any `on_app_quit` callbacks before tearing down.
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            // `secondary-r` is Cmd+R on macOS and Ctrl+R on Linux
            // / Windows — the standard "primary modifier" alias
            // gpui exposes for cross-platform shortcuts. Scoped
            // to `Some("AppView")` so the binding only fires when
            // the AppView root div is in the focus chain;
            // popovers and modals that don't carry the AppView
            // key_context get to keep their own key handling
            // (notably the settings popover's slider, which uses
            // arrow keys).
            KeyBinding::new("secondary-r", StartOrStop, Some("AppView")),
        ]);
        cx.set_menus(vec![Menu {
            name: "PortFinder".into(),
            items: vec![
                // gpui derives the keystroke shown next to the
                // menu label from the bound keybinding above —
                // we don't pass the keystroke string here.
                MenuItem::action(t!("menu.start_or_stop").into_owned(), StartOrStop),
                MenuItem::separator(),
                MenuItem::action(t!("menu.quit").into_owned(), Quit),
            ],
            disabled: false,
        }]);
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

        // Boot palette follows the system appearance. AppView
        // installs an `observe_window_appearance` subscription
        // when its window opens, so subsequent OS theme flips
        // re-apply this helper live — no relaunch needed.
        apply_system_theme(cx, cx.window_appearance());

        // Dev-mode dock icon override on macOS. Production .app
        // bundles get the icon from the Info.plist + Resources/
        // path; `cargo run` without a bundle would otherwise drop
        // to the default Cargo glyph in the Dock.
        #[cfg(target_os = "macos")]
        install_macos_dock_icon();

        // Initial bounds — sized for the "no banner, no result"
        // state so the window doesn't visibly snap-resize on the
        // first paint. If the privileges check at boot turns out
        // to need a banner, the render hook resizes up on the
        // first frame; visually that's a single 60–110 px grow,
        // not the jarring full-window jump a wrong-direction
        // initial size would cause.
        let initial_h = HEIGHT_BASE + HEIGHT_RESULT_EMPTY;
        let bounds = Bounds::centered(None, gpui::size(px(BASE_WIDTH), px(initial_h)), cx);
        // `TitleBar::title_bar_options()` returns the
        // `TitlebarOptions` shape gpui-component's TitleBar widget
        // expects: `title = None` (the widget draws the title text
        // itself, so the OS doesn't double up), `appears_transparent
        // = true` (the OS-provided chrome is hidden; the widget
        // takes its place), and `traffic_light_position` correctly
        // offset so the native macOS traffic lights overlay sits
        // inside the widget's title bar instead of clipping the
        // content area. Using this matters most on Linux/Wayland
        // where the compositor (Mutter on GNOME) doesn't provide
        // server-side decorations — without the widget the app
        // window has zero chrome.
        // `app_id` lets Wayland compositors (Mutter on GNOME, KWin
        // on KDE) match the running window to the installed
        // `portfinder.desktop` file and use its `Icon=portfinder`
        // entry for the dock / overview / window-list. Without this,
        // the compositor has no link from the window back to the
        // .desktop and shows a generic placeholder icon while the
        // app is open (the installed-app icon in System Settings
        // looks correct because the .desktop file is found via the
        // hicolor theme directly; only the *running* window needs
        // the back-link). String must match the .desktop filename
        // without the `.desktop` extension.
        // `window_decorations: Some(Client)` is what makes gpui
        // send `set_mode(MODE_CLIENT_SIDE)` over xdg-decoration to
        // the Wayland compositor on window construction. Without
        // it, gpui defaults to `Server` (per
        // `crates/gpui/src/window.rs:1319` on Zed commit
        // 3bd9d13b — `unwrap_or(WindowDecorations::Server)`),
        // which KWin (KDE Plasma) honours by drawing its own
        // server-side title bar ON TOP of our gpui-component
        // TitleBar widget — dual title bars on every KDE install.
        // Mutter (GNOME) ignores the protocol and always does
        // CSD so the bug isn't visible there. `title_bar_options()`
        // only styles the in-window widget; it doesn't touch
        // `window_decorations` (gpui-component's own example apps
        // pair the two settings explicitly).
        // `is_resizable: false`: AppView drives its own size via
        // `window.resize` whenever the privilege banner or result
        // card appears / collapses (see `desired_height` +
        // `apply_size`), and gpui snaps the user-dragged bounds
        // back to those programmatic values on the next render —
        // visually the corner just snaps back as if it bounced.
        // The window is the wrong shape for free resizing
        // anyway: a single-column form with no horizontal
        // overflow and explicit per-state heights. Lock it so
        // the resize handles don't appear in the first place
        // and the snap-back can't happen. The window stays
        // movable / minimisable / closeable as normal.
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some("portfinder".into()),
            is_resizable: false,
            ..Default::default()
        };

        let _ = cx.open_window(opts, |window, cx| {
            let app_view = cx.new(|cx| AppView::new(window, cx));
            // Root::new takes `impl Into<AnyView>`; the explicit
            // `.into()` confuses the inference (E0283) because the
            // turbofish-less form can match too many `Into` impls.
            // Pass the entity directly — `Entity<AppView>: Into<AnyView>`
            // resolves trivially when AppView: Render.
            cx.new(|cx| Root::new(app_view, window, cx))
        });

        cx.activate(true);
    });
}

/// Map an OS appearance to a gpui-component ThemeMode and install
/// it, then re-apply our brand palette. Always paired: every
/// `Theme::change` resets the theme's slots back to gpui-component
/// defaults for the new mode, so the brand override must come after.
/// Called once at boot in `run()`, and again from the per-window
/// `observe_window_appearance` callback on every system Light/Dark
/// flip.
fn apply_system_theme(cx: &mut App, appearance: gpui::WindowAppearance) {
    let mode = if matches!(
        appearance,
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
    ) {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    Theme::change(mode, None, cx);
    apply_brand_palette(cx, mode);
}

/// Overrides the gpui-component theme's primary slot with our brand
/// blue. Called by `apply_system_theme` immediately after every
/// `Theme::change` so the override survives both the initial boot
/// and any future light/dark flip.
///
/// gpui-component splits its accent surface between two slot
/// families: `primary` / `primary_*` (used by Switch's on-state
/// pill, Select's focus ring, list-row highlight, etc.) and
/// `button_primary` / `button_primary_*` (used **only** by Button's
/// `.primary()` variant — the field is named separately so a host
/// app can give the Start-button-style filled buttons their own
/// fill colour without dragging every other accent surface with
/// them). Both slot families need to be patched here or the Switch
/// turns blue but the Start button stays the gpui-component default
/// (near-black on light mode).
fn apply_brand_palette(cx: &mut App, mode: ThemeMode) {
    let theme = Theme::global_mut(cx);
    let primary: Hsla = rgb(BRAND_PRIMARY).into();
    let primary_hover: Hsla = rgb(BRAND_PRIMARY_HOVER).into();
    let primary_active: Hsla = rgb(BRAND_PRIMARY_ACTIVE).into();
    let white: Hsla = rgb(0xffffff).into();

    // Accent surface — Switch on-state, Select focus ring, etc.
    theme.primary = primary;
    theme.primary_hover = primary_hover;
    theme.primary_active = primary_active;
    theme.primary_foreground = white;

    // Filled-button surface — Start button's `.primary()` variant.
    theme.button_primary = primary;
    theme.button_primary_hover = primary_hover;
    theme.button_primary_active = primary_active;
    theme.button_primary_foreground = white;

    // Card surface — Apple's secondary-system-fill grays so the
    // cards read as native macOS 26 (Tahoe) elevated content rather
    // than a custom palette. Overrides `theme.popover` because
    // that's the slot `render` reads for `card_bg`. Side effect:
    // Select's open-dropdown popover and any gpui-component Tooltip
    // use the same slot, so they pick up the same neutral gray —
    // desirable consistency (every elevated surface in the app
    // speaks the same colour, matching System Settings' behaviour).
    let card_bg: Hsla = match mode {
        ThemeMode::Dark => rgb(CARD_BG_DARK).into(),
        ThemeMode::Light => rgb(CARD_BG_LIGHT).into(),
    };
    theme.popover = card_bg;

    // Skeleton fill — set per-mode so the result-card loading rows
    // have visible contrast against the (deliberately subtle) card
    // surface. Without this override, the gpui-component default
    // skeleton colour is barely darker than `theme.popover` and
    // the pulsing rows look like an empty card.
    theme.skeleton = match mode {
        ThemeMode::Dark => rgb(SKELETON_DARK).into(),
        ThemeMode::Light => rgb(SKELETON_LIGHT).into(),
    };
}

/// macOS-only: set the dock icon at runtime so `cargo run` shows the
/// real PortFinder glyph instead of the default Cargo / terminal
/// binary icon. Production .app bundles ship the icon via the
/// Info.plist + Resources path; this runtime override is dev-only.
///
/// Source PNG (1024×1024) gets composited onto a fresh 1024×1024
/// canvas with Apple's "live area" inset (~80% of canvas), matching
/// the inset cargo-packager's bundler applies when it generates the
/// .icns. Without the inset, a `cargo run` dock icon would look
/// noticeably larger than the same app's bundled icon.
#[cfg(target_os = "macos")]
fn install_macos_dock_icon() {
    use objc2::AnyThread;
    use objc2_app_kit::{NSApplication, NSCompositingOperation, NSImage};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    const CANVAS_PX: f64 = 1024.0;
    const CONTENT_PX: f64 = 824.0;
    const INSET_PX: f64 = (CANVAS_PX - CONTENT_PX) / 2.0;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let icon_path = format!("{manifest_dir}/resources/icons/icon.icns");
    // SAFETY: gpui_platform::application runs its callback on the
    // macOS main thread, which is what NSApplication /
    // NSGraphicsContext (lockFocus / unlockFocus) require.
    // lockFocus / unlockFocus are deprecated in favour of the
    // resolution-independent block-based
    // `imageWithSize:flipped:drawingHandler:` API; the simpler form
    // is fine for a one-shot icon override that never redraws.
    #[allow(deprecated)]
    unsafe {
        let path = NSString::from_str(&icon_path);
        let Some(source) = NSImage::initWithContentsOfFile(NSImage::alloc(), &path)
        else {
            log::warn!("dock icon: could not load {icon_path}");
            return;
        };
        let canvas =
            NSImage::initWithSize(NSImage::alloc(), NSSize::new(CANVAS_PX, CANVAS_PX));
        canvas.lockFocus();
        source.drawInRect_fromRect_operation_fraction(
            NSRect::new(
                NSPoint::new(INSET_PX, INSET_PX),
                NSSize::new(CONTENT_PX, CONTENT_PX),
            ),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            NSCompositingOperation::Copy,
            1.0,
        );
        canvas.unlockFocus();

        let mtm = objc2::MainThreadMarker::new_unchecked();
        let app = NSApplication::sharedApplication(mtm);
        app.setApplicationIconImage(Some(&canvas));
    }
}
