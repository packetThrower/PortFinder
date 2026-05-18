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

use std::sync::OnceLock;
use std::time::Duration;

use flume::{Receiver, Sender};
use gpui::{
    actions, div, prelude::*, px, rgb, App, AppContext, Bounds, ClipboardItem, Context, Entity,
    FocusHandle, Focusable, Hsla, IntoElement, KeyBinding, Menu, MenuItem, ParentElement, QuitMode,
    Render, SharedString, Styled, Window, WindowBounds, WindowDecorations, WindowOptions,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    popover::Popover,
    select::{Select, SelectEvent, SelectItem, SelectState},
    skeleton::Skeleton,
    switch::Switch,
    ActiveTheme, Disableable, IconName, IndexPath, Root, Sizable, Theme, ThemeMode, TitleBar,
};
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
// window. Quit is the priority; Cmd+W close-window can come
// later if anyone asks.
actions!(portfinder, [Quit]);

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
/// row gap, plus card padding. Same height whether or not a field
/// is "absent"; the absent rows still occupy their slot. 210 px
/// was too tight (clipped the version footer); 230 leaves the
/// version line visible with the same comfortable bottom inset
/// the empty-state baseline gives.
const HEIGHT_RESULT_FILLED: f32 = 230.0;
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    // Subscriptions kept alive for the entity's lifetime.
    iface_sub: gpui::Subscription,
    _proto_sub: gpui::Subscription,
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
            status_text: "Ready".into(),
            copied_key: None,
            priv_status,
            is_installing: false,
            update_available: None,
            update_dismissed: false,
            settings: settings::Settings::load_or_default(),
            update_result_rx: update_rx,
            iface_sub,
            _proto_sub: proto_sub,
            _appearance_sub: appearance_sub,
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
        self.status_text = format!("Capturing {}…", self.protocol.as_str()).into();

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
        self.status_text = "Stopping…".into();
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
        match res {
            Ok(r) => {
                self.result = Some(r);
                self.status_text = "Complete".into();
            }
            Err(msg) => {
                if msg.to_lowercase().contains("cancelled") {
                    self.status_text = "Stopped".into();
                } else {
                    self.error = msg;
                    self.status_text = "Error".into();
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
        self.status_text = "Installing helper…".into();
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
                        this.status_text = "BPF helper installed".into();
                        this.priv_status = Some(privilege::get_privilege_status());
                    }
                    Err(err) => {
                        this.error = err;
                        this.status_text = "Error".into();
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
        // state (banner present? result populated?) and apply it if
        // the window is currently the wrong height. `cx.notify()`
        // after every state change re-enters render, so the resize
        // re-evaluates on every flip. The 1px slack on the diff
        // guards against rounding noise from gpui's px → device-px
        // → px round-trip causing an infinite resize loop.
        let desired = px(self.desired_height());
        // Use `viewport_size()` (not `bounds().size`) because the two
        // gpui platform backends interpret bounds differently:
        //   - macOS: `bounds()` returns the FULL window frame (content
        //     + title bar). `resize()` sets just the CONTENT size.
        //   - Windows: `bounds()` returns the logical content area.
        //     `resize()` sets the CONTENT size (border offset added
        //     internally before SetWindowPos).
        // Comparing `bounds().size` against the desired content size
        // is therefore always ~28 px off on macOS — `resize()` fires
        // every render because they never converge (the loop is a
        // no-op visually but spams the log). `viewport_size()` is
        // the drawable area on both backends, so the comparison is
        // cross-platform-consistent.
        let current = window.viewport_size().height;
        if (current - desired).abs() > px(1.0) {
            // Log resize events to the debug log. Only fires on
            // actual mismatch — animation-driven re-renders (the
            // skeleton pulse) shouldn't spam the log because the
            // desired stays constant while pulsing. `debug` level
            // because resize is a per-frame render-tick concern,
            // not a lifecycle event — only useful when debugging
            // the Windows / Wayland "did the resize actually take
            // effect" question; not what you want filling the log
            // file on every banner state change.
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
                            .label(if installing { "Installing…" } else { "Install BPF Helper" })
                            .small()
                            .disabled(installing)
                            .tooltip("One-time install. Lets PortFinder capture without sudo.")
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
                    "PortFinder needs Npcap to capture packets on Windows.",
                ))
                .child(
                    Button::new("open-npcap")
                        .label("Download Npcap")
                        .small()
                        .tooltip("Opens npcap.com to download the installer.")
                        .on_click(|_, _window, cx| {
                            cx.open_url("https://npcap.com/#download");
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("After installing, relaunch PortFinder."),
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
                .child("PortFinder needs elevated privileges to capture packets.")
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
                            .child("Interface"),
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
                                    .tooltip("Refresh interface list")
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
                    .label("Only show interfaces with IPs")
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
                            .child("Protocol"),
                    )
                    .child(Select::new(&self.protocol_select).small()),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("start-capture")
                            .label("Start")
                            .primary()
                            .small()
                            .disabled(is_capturing)
                            .on_click(cx.listener(|this, _, _window, cx| this.start_capture(cx))),
                    )
                    .child(
                        Button::new("stop-capture")
                            .label("Stop")
                            .small()
                            .disabled(!is_capturing)
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
            return div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Run a capture to see switch info here.")
                .into_any_element();
        };

        let rows: [(ResultKey, &'static str, String); 7] = [
            (ResultKey("switch"), "Switch Name", result.switch_name),
            (ResultKey("ip"), "Switch IP", result.switch_ip),
            (ResultKey("port"), "Switch Port", result.switch_port),
            (ResultKey("vlan"), "VLAN", result.native_vlan),
            (ResultKey("voiceVlan"), "Voice VLAN", result.voice_vlan),
            (ResultKey("mtu"), "MTU", result.mtu),
            (ResultKey("model"), "Switch Model", result.switch_model),
        ];

        let mut col = div().flex().flex_col().gap_1();
        for (key, label, raw) in rows {
            col = col.child(self.render_result_row(key, label, raw, cx));
        }
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
        // Capture the AppView entity so the Switch's `on_click`
        // closure can mutate `self.settings` from inside the
        // popover's `.content(...)` callback. The popover renders
        // in an overlay layer that sits outside the AppView's
        // focus tree, so a `cx.dispatch_action` from in there
        // doesn't bubble up to a listener on the root div —
        // we use the explicit `entity.update(cx, ...)` form
        // instead. `cx.entity()` is a cheap Arc clone.
        let entity = cx.entity().clone();

        Popover::new("settings-popover")
            .trigger(
                Button::new("settings-trigger")
                    .icon(IconName::Menu)
                    .ghost()
                    .small(),
            )
            .content(move |_, _, _| {
                let entity = entity.clone();
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_3()
                    .w(px(260.0))
                    // Row 1: label on the left, Switch on the right
                    // — matches macOS System Settings' standard
                    // row layout (justify_between + items_center).
                    // Using a sibling label div rather than the
                    // Switch widget's own .label() so the chip
                    // stays right-aligned instead of riding next
                    // to its label on the left edge.
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_sm().child("Write debug log"))
                            .child(
                                Switch::new("settings-debug-log")
                                    .checked(debug_log)
                                    .small()
                                    .on_click(move |value: &bool, _window, cx| {
                                        let new = *value;
                                        entity.update(cx, |this, cx| {
                                            this.settings.debug_log = new;
                                            // Live on/off — drop or open
                                            // the log file immediately. No
                                            // restart needed.
                                            settings::set_logging_enabled(new);
                                            if let Err(e) = this.settings.save() {
                                                log::warn!("settings save failed: {e}");
                                            }
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    // Row 2: action button anchored bottom-right,
                    // also macOS-System-Settings convention
                    // ("Shortcuts…" / "Hot Corners…" on the
                    // Mission Control panel sit this way).
                    .child(
                        div().flex().justify_end().child(
                            Button::new("settings-open-log-folder")
                                .label("Open log folder")
                                .outline()
                                .small()
                                .on_click(|_, _window, _cx| {
                                    settings::reveal_log_folder();
                                }),
                        ),
                    )
                    .into_any_element()
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
                        .label(format!("Update v{} available", info.version))
                        .ghost()
                        .small()
                        .tooltip("Click to view this release on GitHub.")
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
                            cx.notify();
                        })),
                )
                .into_any_element(),
        )
    }

    fn render_result_row(
        &mut self,
        key: ResultKey,
        label: &'static str,
        raw: String,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let success = theme.success;
        let absent = raw.is_empty() || raw == "N/A";
        let copied = self.copied_key == Some(key);
        // ElementId is constructable from `(&'static str, u64)` —
        // hash the label pointer into a u64 so each row gets a
        // unique id without allocating a SharedString per render.
        let row_id = label.as_ptr() as u64;

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
                .child("Not advertised")
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
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "PortFinder".into(),
            items: vec![MenuItem::action("Quit PortFinder", Quit)],
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
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some("portfinder".into()),
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
