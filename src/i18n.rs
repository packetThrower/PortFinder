//! Translation lookup + locale detection.
//!
//! `rust_i18n::i18n!("locales", fallback = "en")` is invoked from
//! `src/lib.rs` (the library crate's macro context). That macro
//! compiles every YAML file under `locales/` into a phf-style
//! lookup baked into the binary. `t!("path.to.key")` returns a
//! `Cow<'static, str>` (rust-i18n v4) and falls back to the
//! `en.yml` value when the active locale is missing the key.
//!
//! At startup `init()` resolves the active locale by precedence:
//!   1. `Settings.language` (if the user picked one in the popover)
//!   2. The OS locale via the `sys-locale` crate's `get_locale()`
//!      (e.g. `en-US`, `de-DE.UTF-8`); we strip everything after
//!      the language tag and match against `SUPPORTED`.
//!   3. The `fallback = "en"` registered with `i18n!()`.
//!
//! Switching locale at runtime is `rust_i18n::set_locale(code)`.
//! gpui's `Render::render` re-reads `t!()` on each frame, so a
//! `cx.notify()` after the swap is enough — no per-string
//! re-binding needed.

/// Locale codes the app ships translations for. The order here
/// is the order the language picker presents them in the settings
/// popover (English first, then European A→Z by endonym, then
/// non-Latin scripts). Each entry is a (code, display-name-in-
/// own-tongue) pair so the picker is readable to a user already
/// in the "wrong" locale.
pub const SUPPORTED: &[(&str, &str)] = &[
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("fr", "Français"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("ja", "日本語"),
];

/// Default locale code — also the `fallback = "en"` value passed
/// to `rust_i18n::i18n!()`. Centralised so callers don't hardcode
/// the string.
pub const DEFAULT: &str = "en";

/// Resolve the locale to use right now and install it via
/// `rust_i18n::set_locale`. Precedence:
///
/// 1. `override_code` — the persisted `Settings.language` if the
///    user picked one in the popover. `Some("")` is treated as
///    `None` (cleared / unset).
/// 2. The OS locale stripped to its language tag (`en-US` →
///    `en`), matched against `SUPPORTED`.
/// 3. `DEFAULT` (English).
///
/// Returns the code that was actually installed so callers can
/// log it / show it in the picker.
pub fn init(override_code: Option<&str>) -> &'static str {
    let resolved = resolve(override_code);
    rust_i18n::set_locale(resolved);
    log::info!("i18n: locale set to {resolved} (override={override_code:?})");
    resolved
}

/// Pure resolution helper — no side effects. Returns one of
/// `SUPPORTED`'s codes (or `DEFAULT` if nothing matched).
pub fn resolve(override_code: Option<&str>) -> &'static str {
    // Step 1: explicit override from settings.
    if let Some(code) = override_code.filter(|c| !c.is_empty()) {
        if let Some((registered, _)) = SUPPORTED.iter().find(|(c, _)| *c == code) {
            return registered;
        }
        log::warn!("i18n: settings override {code:?} not in SUPPORTED — ignoring");
    }

    // Step 2: OS locale, stripped to the language tag. `sys-locale`
    // returns the best guess at the user's preferred locale on
    // each platform: `defaultLocale` on macOS, `LANG`/`LC_ALL` on
    // Linux, `GetUserDefaultLocaleName` on Windows.
    if let Some(os) = sys_locale::get_locale() {
        // "en-US.UTF-8" → "en"; "zh-Hant-TW" → "zh"; "de" → "de".
        let lang = os
            .split(['-', '_', '.'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if let Some((registered, _)) = SUPPORTED.iter().find(|(c, _)| *c == lang) {
            return registered;
        }
    }

    // Step 3: fallback.
    DEFAULT
}

/// Lookup the display name for a code from `SUPPORTED`. Used by
/// the settings popover's language Select to render the option
/// labels. Falls back to the code itself if (somehow) not found.
pub fn display_name(code: &str) -> &'static str {
    SUPPORTED
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
        .unwrap_or("?")
}
