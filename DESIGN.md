---
name: PortFinder
description: Network switch port discovery via CDP / LLDP / MNDP
colors:
  error: "#c0392b"
  card-bg-light: "#f7f7f7"
  card-bg-dark: "#2c2c2c"
  page-bg-light: "#ffffff"
  page-bg-dark: "#202020"
  thumb-on-pill: "#ffffff"
typography:
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
  label:
    fontFamily: "inherit"
    fontSize: "0.95em"
    fontWeight: 400
  value:
    fontFamily: "inherit"
    fontSize: "13px"
    fontWeight: 500
  status:
    fontFamily: "inherit"
    fontSize: "0.9em"
    fontWeight: 400
  footer:
    fontFamily: "inherit"
    fontSize: "0.8em"
    fontWeight: 400
rounded:
  pill-tiny: "4px"
  pill-small: "6px"
  card-sm: "8px"
  card-md: "10px"
  card-lg: "12px"
  switch-track: "9px"
  switch-thumb: "7px"
  progress-bar: "2px"
spacing:
  gap-row: "13px"
  card-padding: "14px 18px"
  app-padding: "10px 12px 8px"
  card-gap: "8px"
components:
  card:
    backgroundColor: "{colors.card-bg-light}"
    rounded: "{rounded.card-md}"
    padding: "{spacing.card-padding}"
  switch-track:
    rounded: "{rounded.switch-track}"
    width: "40px"
    height: "18px"
  switch-track-checked:
    rounded: "{rounded.switch-track}"
    width: "40px"
    height: "18px"
  switch-thumb:
    backgroundColor: "{colors.thumb-on-pill}"
    rounded: "{rounded.switch-thumb}"
    width: "22px"
    height: "14px"
  progress-fill:
    rounded: "{rounded.progress-bar}"
    height: "3px"
    width: "35%"
  privilege-warning:
    rounded: "{rounded.pill-small}"
    padding: "8px 12px"
  locale-picker:
    rounded: "{rounded.pill-tiny}"
    padding: "1px 4px"
---

# Design System: PortFinder

## 1. Overview

**Creative North Star: "Settings on Every Machine"**

PortFinder's in-app surface is, today, deliberately not a brand. It defers to whichever operating system you're running it under — the macOS build looks like Tahoe System Settings, the Windows build looks like Windows 11 Settings, the Linux build looks like a libadwaita boxed-list. Brand identity asserts itself at exactly one point: the dock icon. From the moment the window opens, the surface is whatever the OS provides — the system text color, the system accent color, the system focus ring, the platform's native widget rendering for `<select>`, `<button>`, `<input>`. The custom code is a thin layer of layout and chrome (cards, the iOS-style toggle, the capture progress bar) that sits on top of that native foundation.

This is a real design philosophy, not an unfinished one. The strategic case is that a small utility used in <30-second bursts under stress benefits from feeling like a built-in part of the machine — no cognitive cost of "what app's conventions are these," no mismatch between Settings-app focus styles and PortFinder focus styles. The trade-off is that PRODUCT.md's stated personality — *polished, branded, distinctive* — has nowhere to manifest in the current surface. The icon carries the entire identity load alone. Future iteration may move the surface in a more branded direction; this DESIGN.md captures what's actually rendered today, not what's wished for.

**Key Characteristics:**

- System colors as the primary palette — no hex brand color appears in the rendered surface.
- Per-OS font stack (SF, Segoe UI Variable, Inter) loaded by `data-os` attribute on `<html>`.
- Per-OS card chrome: radius, padding, border, shadow all vary to match each platform's Settings-app idiom.
- Light and dark via `color-scheme: light dark` plus `prefers-color-scheme` overrides per OS.
- Two-card layout (Controls / Result) plus an optional Privilege Warning banner above.
- One genuinely custom component: the iOS-style toggle switch, hand-built from `<input type="checkbox">` because no native checkbox renders the way the surrounding macOS surface needs.

## 2. Colors

The palette is system-derived. With one exception (error red), every color in the rendered surface is either a CSS system color, a per-OS hex pulled from that platform's Settings-app convention, or a transparent tint computed from a system color via `color-mix`. There is no brand-color section because the brand color isn't in the surface — it's in the icon.

### Primary

There is no primary brand color in the in-app surface. The system's accent color (`AccentColor`) carries any "this is interactive" weight: it tints the toggle switch when on, the progress-bar fill, the focus outline on the switch, and indirectly the platform's own native widget accents.

### Neutral

- **Page background — light** (`#ffffff` on Windows / Linux; system Canvas / NSVisualEffectView vibrancy on macOS): the floor under the cards. On macOS the page is transparent so the window's vibrancy material shows through.
- **Page background — dark** (`#202020` Windows; `#242424` Linux; system Canvas on macOS): same role, dark variant.
- **Card surface — light** (`#f7f7f7` Windows / Linux; `rgba(255, 255, 255, 0.55)` macOS): both major sections (Controls, Result) sit on this surface. macOS uses a translucent fill so the underlying vibrancy material tints through; Windows / Linux use opaque grey because their respective Settings idioms don't use translucency on cards.
- **Card surface — dark** (`#2c2c2c` Windows; `#303030` Linux; `rgba(40, 40, 42, 0.65)` macOS): same role, dark variant.
- **Card border** (`color-mix` from CanvasText at 5–12% alpha, varying per OS / theme): the only line that gives the card edge any definition. Hairline thin; barely visible in dark mode by design.
- **Body text** (`CanvasText`): every text element. `dt` labels render at 70% opacity, status text at 75%, version text at 50%, empty-state at 55%. Hierarchy is opacity, not hue.

### Accent

- **System accent** (`AccentColor`): the toggle on-state, the progress-bar fill, the focus-visible outline. Whatever the user has set as their OS accent (blue by default, but can be red, orange, yellow, green, purple, pink, etc.) is what appears here. Honor user preference; don't override.

### State

- **Error** (`#c0392b`): the single hardcoded brand-style color in the surface. Used only for the status-text error variant. Chosen for sufficient contrast in both light and dark mode without leaning hot or pastel.
- **Highlight / HighlightText** (system): the privilege-warning banner uses these so it adopts whatever banner-style colors the OS prefers — typically a tinted blue background with light text on macOS, an orange-accent on some Windows themes, etc.

### Named Rules

**The System-First Rule.** Colors come from system CSS keywords (`Canvas`, `CanvasText`, `AccentColor`, `Highlight`, `HighlightText`, `ButtonBorder`) before they come from hex. If a value can be expressed as a system keyword, it is. This is the single most important rule for the current state — break it and the app starts feeling like a generic Electron utility instead of part of the OS.

**The Tint-Through-Function Rule.** Per-OS dividers and borders are computed from `CanvasText` via `color-mix(in srgb, CanvasText X%, transparent)`, not hardcoded greys. Same in light mode, automatically inverts in dark mode, automatically respects per-OS subtlety differences.

## 3. Typography

There is no chosen brand typeface. Each OS gets the platform's standard UI font; the in-app surface fits invisibly into whichever Settings app is alongside it.

**Per-OS body family** (loaded via `html[data-os="..."]`):

- **macOS**: `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif`
- **Windows**: `"Segoe UI Variable", "Segoe UI", system-ui, sans-serif` — at 14px instead of 13px to match Windows 11 Settings density.
- **Linux**: `"Inter", "Cantarell", system-ui, sans-serif`

**Character:** quiet and structural. Type is doing labelling work, not display work. There is no Display tier and no Headline tier — the largest text in the entire app is body-sized.

### Hierarchy

- **Body** (400 weight, 13px on macOS / Linux, 14px on Windows, 1.4 line-height): every label and dropdown text in the form, every result label.
- **Value** (500 weight, body size): the right-hand column of the result list — the actual switch / port / VLAN values the user came here to read. Heavier weight differentiates them from their `dt` labels at a glance.
- **Status** (400 weight, ~12px / 0.9em): the bottom-row status text ("Ready", "Capturing LLDP packets...", "Capture stopped").
- **Label** (400 weight, ~12.4px / 0.95em on result `dt`): subtler than body — 70% opacity de-emphasizes them so the values dominate.
- **Footer** (400 weight, ~10.4px / 0.8em): the version string. 50% opacity. Quiet bookkeeping.

### Named Rules

**The No-Display-Tier Rule.** PortFinder's window is 400×460. There is no real estate for a 32px headline and there is no marketing voice that wants one. Type tops out at body weight; emphasis comes from weight contrast (500 vs 400) and opacity differentiation, not size.

**The Platform-Font Rule.** Body and value type both inherit from the per-OS family. Custom typeface choices are explicitly forbidden in the current system — picking "Inter on every platform" would break Settings-on-Every-Machine for the marginal gain of consistency.

## 4. Elevation

Effectively flat. No drop shadows on cards under macOS or Linux; Windows uses one extremely subtle shadow (`0 1px 3px rgba(0, 0, 0, 0.04)`) to match Fluent's Mica surface convention. Depth is conveyed by **tonal layering** — the page is one tone, cards are a sibling tone, and a hairline border separates them.

The macOS build adds one structural layer outside CSS: `NSVisualEffectView` vibrancy applied to the window via the `window-vibrancy` Rust crate. The vibrancy is what gives the page background its subtle depth and live color tint; CSS sees it as "transparent" and lets it through.

### Shadow Vocabulary

- **Card shadow — Windows** (`box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04)`): a tone-darkening hint at the card edge, almost invisible on dark mode (`none` is used there). Mimics Mica.
- **macOS / Linux**: no shadow. Tonal layering and a hairline border carry the edge.
- **Switch thumb** (`box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25)`): the only meaningful shadow in the surface — the thumb on the iOS-style toggle has a real drop shadow because it's the one element that has to read as a physical movable object.

### Named Rules

**The Tonal-Layer Rule.** Card depth = page-color one step + a hairline border. Don't reach for `box-shadow` to communicate "this is a card" — that's what the Bootstrap-shadow-on-everything aesthetic does, and it's the generic Electron trap.

## 5. Components

### Cards

The two main sections (Controls, Result) and an optional Privilege Warning above them.

- **Shape:** rounded by per-OS radius — macOS 10px, Windows 8px, Linux 12px (matching libadwaita boxed-list).
- **Background:** per-OS card-bg token, varies by theme.
- **Border:** 1px hairline computed from `CanvasText` at 5–12% alpha; per-OS / per-theme.
- **Shadow:** none on macOS / Linux; hint shadow on Windows. See Elevation.
- **Internal padding:** per-OS — macOS 14×18, Windows 16×18, Linux 14×16. The asymmetry is intentional and matches each platform's Settings-app convention.
- **Internal gap (`--gap-row`):** 13px macOS / 12px Windows / Linux. Vertical rhythm between form rows or list rows inside the card.

### Buttons

There are no styled buttons. `<button>` elements render with their platform's native chrome — system shape, system color, system focus ring, system pressed state. The only override is `min-width: 88px` on the Start / Stop buttons so they share width and read as a pair.

The "Install BPF Access" button on the macOS privilege banner is also unstyled native — it inherits whatever button look the privilege banner's `Highlight` background fights with.

### Form Group

A two-column grid: label on the left, control on the right. Label width is per-OS (macOS 70px, Windows / Linux 110px) reflecting each platform's Settings-app label proportions. The block centers horizontally — `justify-content: center` — so it doesn't sprawl to the right edge of its card.

- **Label:** body color at full opacity, right-aligned.
- **Control:** native widget; `width: 100%` of its column.

### Switch (Custom)

The one fully-custom component. The native `<input type="checkbox">` is hidden via `appearance: none` and rebuilt as an iOS-style pill with a sliding thumb. Used only for the "Only show interfaces with an IP" toggle.

- **Track:** 40×18px pill, radius 9px, background = `color-mix(in srgb, CanvasText 25%, transparent)` when off, `AccentColor` when on. Smooth 200ms transition between states.
- **Thumb:** 22×14px, radius 7px, white with a soft drop shadow. `transform: translateX(14px)` on checked, 200ms transition.
- **Focus:** 2px `AccentColor` outline at 2px offset. Honors keyboard navigation.

The switch is the one place where the surface willingly diverges from native — partly because no platform ships a checkbox that reads correctly inside an opaque card, partly because field techs respond faster to a switch (immediately readable as binary) than to a checkmark.

### Progress Bar

A capture-in-progress indicator. 100% width, 3px tall, radius 2px, background `ButtonBorder`. The fill is `AccentColor`, 35% wide, slid from -100% to +300% on a 1.2s `ease-in-out infinite` loop.

This is the only animated element in the current surface. It encodes "something is running but I don't know how long" — an indeterminate progress signal, not a percentage.

### Result List (definition list)

A `<dl>` with two-column grid, mirroring the Form Group label width. `<dt>` is body-color at 70% opacity, right-aligned, ~12.4px. `<dd>` is body-color at full opacity, weight 500, body size, with `text-overflow: ellipsis` for long values. Row gap 4px, column gap 10px.

When no capture has happened, the list is replaced by a centered `.empty-state` paragraph at 55% opacity.

### Privilege Warning Banner

Sits above the Controls card when the app can't capture (no BPF helper on macOS, no Npcap on Windows, no `setcap` on Linux).

- **Background:** `Highlight` (system).
- **Text:** `HighlightText` (system).
- **Radius:** 6px.
- **Padding:** 8×12.
- **Body:** 0.92em — slightly smaller than body so it reads as secondary chrome rather than primary content.

When present, the banner pushes the window taller via `setSize` from `tauri::WebviewWindow` so the layout doesn't compress the cards.

### Locale Picker (footer dropdown)

Small native `<select>` placed in the footer alongside the version string. 0.78em font, 70% opacity (1.0 on hover/focus), transparent background, 1px border at 25% alpha CanvasText, 4px radius, 1×4 padding. Lists the four supported locales by their native names.

### Refresh Button

Tiny native button next to the interface dropdown showing `↻` (U+21BB). 28px min-width, 14px font, 1×6 padding. Otherwise platform-native.

## 6. Do's and Don'ts

### Do:

- **Do** rely on system colors first. `Canvas`, `CanvasText`, `AccentColor`, `Highlight`, `HighlightText`, `ButtonBorder` carry the bulk of the surface. If a value can be expressed as one, express it that way.
- **Do** respect the user's system accent color. Don't hardcode blue. `AccentColor` is the contract.
- **Do** vary per-OS chrome via `html[data-os="..."]` — radius, padding, font stack, surface color all change deliberately. The variation is the point.
- **Do** use `color-mix(in srgb, CanvasText X%, transparent)` for borders, dividers, and tints. This auto-inverts in dark mode and respects per-OS subtlety.
- **Do** carry state via text and icon, not color alone. Capture state ("Ready" / "Capturing LLDP packets…" / "Capture stopped") is a textual status, not a colored dot. Error state is a red status text *and* the word "Error" — not red text alone. Color-blind safety is a hard constraint, not a polish item.
- **Do** keep type one tier deep. There is no Display, no Headline. Body and label, weight contrast and opacity contrast carry hierarchy.
- **Do** keep motion to one place. The capture progress bar is the only animation in the system. Add more only when motion makes a state change clearer, never decoratively. Respect `prefers-reduced-motion` as a calm fallback.

### Don't:

- **Don't** ship a generic Electron aesthetic. PRODUCT.md names this directly: same Bootstrap-ish components as fifty other utilities, every widget rounded the same way, no rooting in any visual tradition. The current design exists in opposition to that — preserve the "this looks like Settings" reading, don't drift into "looks like a cross-platform tool that picked a Material theme."
- **Don't** add decorative gradients, gradient text, or `background-clip: text`. PRODUCT.md rejects flashy SaaS-dashboard chrome by name.
- **Don't** add side-stripe borders (`border-left: 4px solid color`) on the privilege warning, the result rows, or anything else. Use full borders, background tints, leading icons, or nothing.
- **Don't** add a hero metric / hero number. There's no marketing surface inside the app.
- **Don't** glassmorphism the cards. The macOS vibrancy is a system feature delivered by `NSVisualEffectView`, applied once at the window level — that's the only "glass" in the system. Don't paint additional `backdrop-filter: blur()` on cards, banners, or the locale picker.
- **Don't** override native widget chrome with custom CSS. `<button>`, `<select>`, `<input>` keep their platform appearance. The toggle switch is the *only* exception, and it's hand-built specifically to read correctly inside an opaque card.
- **Don't** introduce a brand color into the surface unless the design direction explicitly changes. Today the brand color (icon navy / amber) lives only in the icon. Adding it to a button or accent in the app would break Settings-on-Every-Machine without replacing it with an articulated alternative.
- **Don't** use modals. The privilege warning is a banner, not a dialog. Inline > modal in this app.
- **Don't** stack the Controls and Result cards with extra padding between them. The 8px `gap` on `.app` is the single source of vertical rhythm — adding `margin-top` on a card breaks it.
- **Don't** introduce em dashes in copy. Commas, colons, periods, parentheses do the work. (PRODUCT.md voice rule.)
